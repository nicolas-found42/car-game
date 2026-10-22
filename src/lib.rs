//! The single production application used by the binary and bounded scenarios.
use bevy::{
    app::ScheduleRunnerPlugin,
    asset::AssetApp,
    log::{Level, LogPlugin},
    prelude::*,
    window::{ExitCondition, PresentMode, WindowResolution},
    winit::WinitPlugin,
};
use std::{sync::Arc, time::Duration};
pub mod city;
pub mod config;
pub mod diagnostics;
mod ingress;
pub mod presentation;
pub mod provenance;
pub mod roster;
pub mod runtime;
pub mod scenario;
pub mod smoke;
pub mod telemetry;
pub mod vehicle;
pub use config::{RenderBackend, RuntimeConfig};

pub fn build_app(config: RuntimeConfig, run: Arc<diagnostics::RunRecorder>) -> Result<App, String> {
    let mut fleet = roster::Fleet::default();
    fleet.selected = fleet
        .cars
        .iter()
        .position(|c| c.id == config.car)
        .ok_or("unknown initial car")?;
    if !config.access.permits(&config.car) {
        return Err("initial career car is not-owned".into());
    }
    if let Some(scenario) = &config.scenario {
        scenario.validate()?;
        if config.mode != "scenario"
            || config.car != scenario.initial.car
            || config.access != scenario.context
        {
            return Err(
                "scenario context or initial state conflicts with runtime configuration".into(),
            );
        }
    }
    run.capture_provenance(
        &config.asset_root,
        &config.asset_root_kind,
        effective_config(&config),
    );
    let replay = config
        .replay_path
        .as_ref()
        .map(|p| diagnostics::ReplayInput::load(p))
        .transpose()?;
    let mut app = App::new();
    add_platform(&mut app, &config);
    app.insert_resource(config.clone())
        .insert_resource(fleet.cars[fleet.selected].clone())
        .insert_resource(fleet)
        .insert_resource(config.access.clone())
        .insert_resource(diagnostics::RunHandle(run.clone()))
        .insert_resource(diagnostics::PhysicsTrace::new(run.dir()))
        .insert_resource(runtime::Session::from_env())
        .add_plugins((
            runtime::RuntimePlugin,
            vehicle::VehiclePlugin,
            city::CityPlugin,
        ));
    app.world_mut()
        .resource_mut::<runtime::CommandBus>()
        .recorder = Some(run.clone());
    if let Some(replay) = replay {
        app.insert_resource(replay);
    }
    if config.backend != RenderBackend::Disabled {
        app.add_systems(Update, ingress::observe_windows);
        app.add_plugins((
            diagnostics::DiagnosticsPlugin,
            presentation::PresentationPlugin,
            smoke::SmokePlugin::new(run.dir().into()),
        ));
    } else {
        app.add_systems(
            Update,
            (
                presentation::read_controls,
                presentation::select_car,
                runtime::dispatch_update,
            )
                .chain(),
        );
    }
    app.add_systems(Update, ingress::observe_gamepads);
    app.add_plugins(scenario::ScenarioPlugin);
    if config.backend == RenderBackend::Offscreen {
        app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 120.0,
        )));
    }
    Ok(app)
}
fn add_platform(app: &mut App, config: &RuntimeConfig) {
    let assets = AssetPlugin {
        file_path: config.asset_root.to_string_lossy().into_owned(),
        ..default()
    };
    if config.backend == RenderBackend::Disabled {
        app.add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 120.0,
            ))),
            TransformPlugin,
            bevy::input::InputPlugin,
            assets,
        ))
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>();
        return;
    }
    let offscreen = config.backend == RenderBackend::Offscreen;
    let mut plugins = DefaultPlugins
        .set(LogPlugin {
            filter: config.log_filter.clone(),
            level: Level::INFO,
            custom_layer: diagnostics::file_log_layer,
            ..default()
        })
        .set(WindowPlugin {
            primary_window: (!offscreen).then(|| Window {
                title: "GRID — Free Drive".into(),
                resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            exit_condition: if offscreen {
                ExitCondition::DontExit
            } else {
                ExitCondition::OnAllClosed
            },
            ..default()
        })
        .set(assets);
    if offscreen {
        plugins = plugins.disable::<WinitPlugin>();
    }
    app.add_plugins(plugins);
}

fn effective_config(config: &RuntimeConfig) -> serde_json::Value {
    serde_json::json!({
        "mode": config.mode,
        "car": config.car,
        "visual_only": config.visual_only,
        "offscreen": config.backend == RenderBackend::Offscreen,
        "render_disabled": config.backend == RenderBackend::Disabled,
        "seed": config.seed,
        "fixed_hz": 120,
        "log_filter": config.log_filter,
        "scene": config.scenario.as_ref().map_or("driving", |s| s.scene.as_str()),
        "context": config.access,
        "scenario": config.scenario,
        "max_ticks": config.scenario.as_ref().map(|s| s.max_ticks),
        "capture_required": config.scenario.as_ref().is_some_and(|s| s.required_capture),
        "replay_input": config.replay_path
    })
}
