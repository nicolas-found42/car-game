//! Parsed execution settings: rendering never grants career access.
use crate::{roster::Fleet, scenario::ScenarioDefinition};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderBackend {
    #[default]
    Window,
    Offscreen,
    Disabled,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AccessContext {
    #[default]
    Sandbox,
    Career {
        owner_id: String,
        owned_cars: Vec<String>,
    },
}
impl AccessContext {
    pub fn permits(&self, car: &str) -> bool {
        match self {
            Self::Sandbox => true,
            Self::Career {
                owner_id,
                owned_cars,
            } => !owner_id.trim().is_empty() && owned_cars.iter().any(|id| id == car),
        }
    }
}

#[derive(Resource, Clone, Debug)]
pub struct RuntimeConfig {
    pub reference_physics: bool,
    pub fast_scenario: bool,
    pub mode: String,
    pub visual_only: bool,
    pub backend: RenderBackend,
    pub car: String,
    pub asset_root: PathBuf,
    pub asset_root_kind: String,
    pub artifact_dir: PathBuf,
    pub seed: u64,
    pub access: AccessContext,
    pub scenario: Option<ScenarioDefinition>,
    pub replay_path: Option<PathBuf>,
    pub log_filter: String,
}
impl Default for RuntimeConfig {
    fn default() -> Self {
        let bundle = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent()?.parent().map(|p| p.join("Resources/assets")))
            .filter(|p| p.is_dir());
        let bundled = bundle.is_some();
        Self {
            reference_physics: false,
            fast_scenario: false,
            mode: "interactive".into(),
            visual_only: false,
            backend: RenderBackend::Window,
            car: "350z".into(),
            asset_root: bundle
                .unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))),
            asset_root_kind: if bundled {
                "bundle_resources"
            } else {
                "checkout_assets"
            }
            .into(),
            artifact_dir: "artifacts/runs".into(),
            seed: 0,
            access: AccessContext::Sandbox,
            scenario: None,
            replay_path: None,
            log_filter: std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "car_game=debug,bevy_asset=warn,wgpu=error,naga=warn".into()),
        }
    }
}
impl RuntimeConfig {
    pub fn from_args(args: impl IntoIterator<Item = impl Into<String>>) -> Result<Self, String> {
        let args: Vec<String> = args.into_iter().map(Into::into).collect();
        let backend = if args.iter().any(|a| a == "--render-disabled") {
            RenderBackend::Disabled
        } else if args.iter().any(|a| a == "--offscreen") {
            RenderBackend::Offscreen
        } else {
            RenderBackend::Window
        };
        let mut config = Self {
            fast_scenario: args.iter().any(|a| a == "--fast-scenario"),
            reference_physics: args.iter().any(|a| a == "--reference-physics"),
            backend,
            visual_only: args.iter().any(|a| a == "--visual-check"),
            ..Self::default()
        };
        for (flag, mode) in [
            ("--replay-input", "replay"),
            ("--smoke", "smoke"),
            ("--visual-check", "visual"),
            ("--fleet-check", "fleet"),
        ] {
            if args.iter().any(|a| a == flag) {
                config.mode = mode.into();
            }
        }
        if let Some(value) = argument(&args, "--car")? {
            config.car = value.into();
        }
        if let Some(value) = argument(&args, "--artifact-dir")? {
            config.artifact_dir = value.into();
        }
        config.replay_path = argument(&args, "--replay-input")?.map(PathBuf::from);
        config.load_scenario(&args)?;
        if let Some(value) = argument(&args, "--seed")? {
            config.seed = value
                .parse()
                .map_err(|_| "--seed requires an unsigned integer")?;
        }
        if let Some(filter) = argument(&args, "--scenario-filter")?
            && (filter.trim().is_empty()
                || config
                    .scenario
                    .as_ref()
                    .is_none_or(|s| !s.name.contains(filter)))
        {
            return Err("scenario filter selected zero scenarios".into());
        }
        if !Fleet::default().cars.iter().any(|car| car.id == config.car) {
            return Err(format!("unknown car {}", config.car));
        }
        config.validate_execution()?;
        Ok(config)
    }
    pub fn validate_execution(&self) -> Result<(), String> {
        if self.fast_scenario
            && (self.backend != RenderBackend::Disabled
                || self.mode != "scenario"
                || self
                    .scenario
                    .as_ref()
                    .is_none_or(|s| s.commands.iter().any(|c| c.elapsed_ms.is_some())))
        {
            return Err("--fast-scenario requires a render-disabled tick-only scenario".into());
        }
        Ok(())
    }
    fn load_scenario(&mut self, args: &[String]) -> Result<(), String> {
        if let Some(path) = argument(args, "--scenario")? {
            if self.mode != "interactive" {
                return Err(
                    "--scenario cannot be combined with smoke/fleet/visual/replay modes".into(),
                );
            }
            let text =
                std::fs::read_to_string(path).map_err(|e| format!("read scenario {path}: {e}"))?;
            let scenario = ScenarioDefinition::from_json(&text)?;
            self.seed = scenario.seed;
            self.car = scenario.initial.car.clone();
            self.access = scenario.context.clone();
            self.scenario = Some(scenario);
            self.mode = "scenario".into();
        }
        if self.backend == RenderBackend::Disabled
            && matches!(self.mode.as_str(), "smoke" | "fleet" | "visual")
        {
            return Err("visual verification requires a rendering backend".into());
        }
        Ok(())
    }
}
fn argument<'a>(args: &'a [String], flag: &str) -> Result<Option<&'a str>, String> {
    let Some(i) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    args.get(i + 1)
        .filter(|v| !v.starts_with("--"))
        .map(|v| Some(v.as_str()))
        .ok_or_else(|| format!("{flag} requires a value"))
}
