//! Bounded scenarios submit production commands and observe completed physics ticks.
mod route;
mod runner;
use crate::{config::AccessContext, runtime::ProductionCommand};
pub use route::RoutePlan;
pub use runner::ScenarioPlugin;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Course {
    #[default]
    City,
    NortheastCurve,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InitialState {
    pub car: String,
    #[serde(default = "initial_position")]
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation_y: f32,
}
fn initial_position() -> [f32; 3] {
    [3.2, 0.62, 190.0]
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledCommand {
    pub tick: Option<u64>,
    pub elapsed_ms: Option<u64>,
    pub command: ProductionCommand,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssertionKind {
    RouteCompleted {
        max_cross_track_m: f32,
    },
    DisplacementAtLeast {
        meters: f32,
    },
    SpeedAtLeast {
        mps: f32,
    },
    ResetCount {
        count: u32,
    },
    SnapshotMatches {
        key: String,
        fresh_entities: bool,
    },
    PhysicsState {
        vehicle: crate::assembly::VehicleId,
        grounded: usize,
        collider: Option<String>,
        outcome: String,
        min_forward_mps: f32,
    },
    VehicleState {
        vehicle: crate::assembly::VehicleId,
        car: String,
        controller: crate::runtime::ControllerOwner,
        reset_count: u32,
        min_forward_mps: f32,
        max_forward_mps: f32,
    },
    AssemblyState {
        revision: u64,
        mass: f64,
        readiness: String,
    },
    SelectedCar {
        car: String,
    },
    CommandRejected {
        reason: String,
        count: usize,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScenarioAssertion {
    pub tick: u64,
    #[serde(flatten)]
    pub condition: AssertionKind,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDefinition {
    #[serde(default)]
    pub course: Course,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<RoutePlan>,
    pub schema_version: u32,
    pub name: String,
    pub scene: String,
    #[serde(default)]
    pub context: AccessContext,
    #[serde(default)]
    pub seed: u64,
    pub initial: InitialState,
    pub commands: Vec<ScheduledCommand>,
    pub assertions: Vec<ScenarioAssertion>,
    pub max_ticks: u64,
    pub max_wall_ms: u64,
    pub readiness_timeout_ms: u64,
    #[serde(default)]
    pub required_capture: bool,
}
impl ScenarioDefinition {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let scenario: Self = serde_json::from_str(text).map_err(|e| format!("scenario: {e}"))?;
        scenario.validate()?;
        Ok(scenario)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.scene != "driving" || self.name.trim().is_empty() {
            return Err("unsupported scenario schema, scene or empty name".into());
        }
        if self.assertions.is_empty() {
            return Err("scenario requires at least one assertion".into());
        }
        if self.max_ticks == 0
            || self.max_wall_ms == 0
            || self.readiness_timeout_ms == 0
            || self.readiness_timeout_ms > self.max_wall_ms
        {
            return Err("invalid scenario bounds".into());
        }
        if self.initial.position.iter().any(|v| !v.is_finite())
            || !self.initial.rotation_y.is_finite()
        {
            return Err("nonfinite initial pose".into());
        }
        if !self.context.permits(&self.initial.car) {
            return Err("initial career car is not-owned".into());
        }
        self.validate_route()?;
        self.validate_timeline()?;
        self.validate_assertions()
    }
    fn validate_route(&self) -> Result<(), String> {
        if let Some(route) = &self.route {
            route.validate()?;
            for assertion in &self.assertions {
                if let AssertionKind::RouteCompleted { max_cross_track_m } = assertion.condition
                    && (!max_cross_track_m.is_finite() || max_cross_track_m <= 0.)
                {
                    return Err("route corridor tolerance must be positive and finite".into());
                }
            }
        }
        if self.route.is_none()
            && self
                .assertions
                .iter()
                .any(|a| matches!(a.condition, AssertionKind::RouteCompleted { .. }))
        {
            return Err("route assertion requires a route".into());
        }
        Ok(())
    }
    fn validate_timeline(&self) -> Result<(), String> {
        let mut last_tick = 0;
        let mut last_ms = 0;
        for item in &self.commands {
            item.command.validate()?;
            match (item.tick, item.elapsed_ms) {
                (Some(tick), None) if tick >= last_tick && tick < self.max_ticks => {
                    last_tick = tick
                }
                (None, Some(ms)) if ms >= last_ms && ms < self.max_wall_ms => last_ms = ms,
                _ => {
                    return Err("commands require one sorted trigger within scenario bounds".into());
                }
            }
        }
        Ok(())
    }
    fn validate_assertions(&self) -> Result<(), String> {
        let mut last_tick = 0;
        for assertion in &self.assertions {
            if assertion.tick < last_tick || assertion.tick > self.max_ticks {
                return Err("assertions require sorted ticks within scenario bounds".into());
            }
            last_tick = assertion.tick;
            match &assertion.condition {
                AssertionKind::PhysicsState {
                    vehicle,
                    grounded,
                    min_forward_mps,
                    outcome,
                    ..
                } if vehicle.0.is_empty()
                    || *grounded > 4
                    || !min_forward_mps.is_finite()
                    || outcome.is_empty() =>
                {
                    return Err("physics assertion requires valid vehicle, grounded count, outcome and speed".into());
                }
                AssertionKind::VehicleState {
                    min_forward_mps,
                    max_forward_mps,
                    vehicle,
                    ..
                } if vehicle.0.is_empty()
                    || !min_forward_mps.is_finite()
                    || !max_forward_mps.is_finite()
                    || min_forward_mps > max_forward_mps =>
                {
                    return Err(
                        "vehicle assertion requires valid target and finite ordered speed bounds"
                            .into(),
                    );
                }
                AssertionKind::DisplacementAtLeast { meters }
                | AssertionKind::SpeedAtLeast { mps: meters }
                    if !meters.is_finite() || *meters <= 0.0 =>
                {
                    return Err("motion assertions require positive finite thresholds".into());
                }
                AssertionKind::CommandRejected { reason, count }
                    if reason.is_empty() || *count == 0 =>
                {
                    return Err("rejection assertions require reason and positive count".into());
                }
                _ => {}
            }
        }
        Ok(())
    }
}
