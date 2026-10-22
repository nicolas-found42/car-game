use super::{AssertionKind, ScenarioDefinition};
use crate::{
    RenderBackend, RuntimeConfig,
    diagnostics::{PhysicsTrace, RunHandle, capture_metrics},
    presentation::{PresentationUpdate, QaRenderTarget},
    roster::CarSpec,
    runtime::{CommandBus, CommandSet, ControllerOwner, SimulationClock},
    vehicle::Vehicle,
};
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};
use bevy_rapier3d::prelude::Velocity;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

#[derive(Resource)]
struct ScenarioRun {
    definition: ScenarioDefinition,
    start: Instant,
    ready: Option<(u64, f64, Vec3)>,
    submitted: HashSet<usize>,
    results: Vec<Value>,
    finished: bool,
    capture_requested: bool,
    capture: Option<Value>,
    capture_error: Option<String>,
    rejections: HashMap<String, usize>,
}
#[derive(Component)]
struct ScenarioCapture;
pub struct ScenarioPlugin;
impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        let Some(definition) = app.world().resource::<RuntimeConfig>().scenario.clone() else {
            return;
        };
        app.insert_resource(ScenarioRun {
            definition,
            start: Instant::now(),
            ready: None,
            submitted: HashSet::new(),
            results: Vec::new(),
            finished: false,
            capture_requested: false,
            capture: None,
            capture_error: None,
            rejections: HashMap::new(),
        })
        .add_systems(PostStartup, initialize)
        .add_systems(
            FixedFirst,
            tick_commands
                .after(CommandSet::BeginTick)
                .before(CommandSet::Dispatch),
        )
        .add_systems(
            FixedLast,
            (collect_outcomes, evaluate)
                .chain()
                .after(CommandSet::CompleteTick),
        )
        .add_systems(
            Update,
            (update, crate::runtime::dispatch_update)
                .chain()
                .after(PresentationUpdate),
        );
    }
}
fn initialize(world: &mut World) {
    let initial = world.resource::<ScenarioRun>().definition.initial.clone();
    for (mut pose, mut car) in world
        .query::<(&mut Transform, &mut Vehicle)>()
        .iter_mut(world)
    {
        *pose = Transform::from_translation(Vec3::from_array(initial.position))
            .with_rotation(Quat::from_rotation_y(initial.rotation_y));
        car.previous = *pose;
    }
    crate::runtime::dispatch_update(world);
    let config = world.resource::<RuntimeConfig>();
    let run = world.resource::<RunHandle>();
    write_json(
        &run.dir().join("scenario.json"),
        &json!({"definition":world.resource::<ScenarioRun>().definition,"effective_seed":config.seed,"render_backend":config.backend,"context":config.access,"rng":{"algorithm":"no-random-draws","state":config.seed,"draws":0}}),
    );
}
fn tick_commands(
    mut state: ResMut<ScenarioRun>,
    clock: Res<SimulationClock>,
    mut bus: ResMut<CommandBus>,
) {
    if state.finished {
        return;
    }
    let Some((start, _, _)) = state.ready else {
        return;
    };
    let tick = clock.current - start;
    submit_due(&mut state, &mut bus, Some(tick), None);
}
fn submit_due(state: &mut ScenarioRun, bus: &mut CommandBus, tick: Option<u64>, ms: Option<u64>) {
    for (index, item) in state.definition.commands.iter().enumerate() {
        let due = item.tick.zip(tick).is_some_and(|(at, now)| now >= at)
            || item.elapsed_ms.zip(ms).is_some_and(|(at, now)| now >= at);
        if due && state.submitted.insert(index) {
            bus.submit(ControllerOwner::Scenario, item.command.clone());
        }
    }
}
fn update(world: &mut World) {
    world.resource_scope(|world, mut state: Mut<ScenarioRun>| {
        if state.finished {
            return;
        }
        let elapsed = state.start.elapsed().as_millis();
        let backend = world.resource::<RuntimeConfig>().backend;
        if state.definition.required_capture && backend == RenderBackend::Disabled {
            finish(world, &mut state, Some("required-capture-unavailable"));
            return;
        }
        if elapsed >= state.definition.max_wall_ms as u128 {
            finish(world, &mut state, Some("wall-time-limit"));
            return;
        }
        if state.ready.is_none() {
            if let Some(position) = ready(world) {
                let tick = world.resource::<SimulationClock>().completed;
                let real_time = world.resource::<Time<Real>>().elapsed_secs_f64();
                state.ready = Some((tick, real_time, position));
                world
                    .resource::<RunHandle>()
                    .event("scenario.ready", json!({"tick":tick,"backend":backend}));
            } else if elapsed >= state.definition.readiness_timeout_ms as u128 {
                finish(world, &mut state, Some("readiness-timeout"));
            }
            return;
        }
        let elapsed_ms = ((world.resource::<Time<Real>>().elapsed_secs_f64()
            - state.ready.unwrap().1)
            * 1000.0) as u64;
        submit_due(
            &mut state,
            &mut world.resource_mut::<CommandBus>(),
            None,
            Some(elapsed_ms),
        );
        crate::runtime::dispatch_update(world);
        if let Some(reason) = state.capture_error.clone() {
            finish(world, &mut state, Some(&reason));
            return;
        }
        if work_complete(world, &state) {
            if state.definition.required_capture && state.capture.is_none() {
                if !state.capture_requested {
                    request_capture(world);
                    state.capture_requested = true;
                }
            } else {
                finish(world, &mut state, None);
            }
        }
    });
}
fn ready(world: &mut World) -> Option<Vec3> {
    let (pose, grounded) = world
        .query::<(&Transform, &Vehicle)>()
        .single(world)
        .ok()
        .map(|(p, c)| (p.translation, c.grounded))?;
    if grounded == 0 {
        return None;
    }
    let config = world.resource::<RuntimeConfig>();
    let spec = world.resource::<CarSpec>().clone();
    let loaded = if config.backend == RenderBackend::Disabled {
        asset_nodes(&config.asset_root.join(&spec.path))
    } else {
        world
            .query::<&Name>()
            .iter(world)
            .filter(|name| crate::roster::WHEEL_NODE_NAMES.contains(&name.as_str()))
            .count()
    };
    (loaded == spec.wheel_nodes).then_some(pose)
}
fn asset_nodes(path: &std::path::Path) -> usize {
    let Ok(bytes) = std::fs::read(path) else {
        return 0;
    };
    if bytes.len() < 20 || &bytes[..4] != b"glTF" || &bytes[16..20] != b"JSON" {
        return 0;
    }
    let length = u32::from_le_bytes(bytes[12..16].try_into().expect("four bytes")) as usize;
    let Some(chunk) = bytes.get(20..20 + length) else {
        return 0;
    };
    let Ok(value) = serde_json::from_slice::<Value>(chunk) else {
        return 0;
    };
    value["nodes"].as_array().map_or(0, |nodes| {
        nodes
            .iter()
            .filter_map(|n| n["name"].as_str())
            .filter(|n| crate::roster::WHEEL_NODE_NAMES.contains(n))
            .count()
    })
}
fn evaluate(world: &mut World) {
    world.resource_scope(|world, mut state: Mut<ScenarioRun>| {
        let Some((start, _, position)) = state.ready else {
            return;
        };
        if state.finished {
            return;
        }
        let tick = world.resource::<SimulationClock>().completed - start;
        while let Some(assertion) = state
            .definition
            .assertions
            .get(state.results.len())
            .cloned()
            .filter(|a| tick >= a.tick)
        {
            let observed = observe(world, &assertion.condition, position, &state.rejections);
            let passed = observed["passed"] == true;
            world.resource::<RunHandle>().event(
                "scenario.assertion",
                json!({"tick":tick,"assertion":assertion,"observed":observed,"passed":passed}),
            );
            state
                .results
                .push(json!({"assertion":assertion,"observed":observed,"passed":passed}));
        }
        if work_complete(world, &state)
            && (!state.definition.required_capture || state.capture.is_some())
        {
            finish(world, &mut state, None);
        } else if tick >= state.definition.max_ticks {
            finish(world, &mut state, Some("tick-limit"));
        }
    });
}
fn observe(
    world: &mut World,
    condition: &AssertionKind,
    start: Vec3,
    rejections: &HashMap<String, usize>,
) -> Value {
    let Some((pose, velocity, resets)) = world
        .query::<(&Transform, &Velocity, &Vehicle)>()
        .iter(world)
        .next()
        .map(|(p, v, c)| (*p, *v, c.reset_count))
    else {
        return json!({"passed":false,"reason":"no-target"});
    };
    match condition {
        AssertionKind::DisplacementAtLeast { meters } => {
            let actual =
                Vec2::new(pose.translation.x - start.x, pose.translation.z - start.z).length();
            json!({"passed":actual >= *meters,"meters":actual})
        }
        AssertionKind::SpeedAtLeast { mps } => {
            let actual = velocity.linear.length();
            json!({"passed":actual >= *mps,"mps":actual})
        }
        AssertionKind::ResetCount { count } => json!({"passed":resets == *count,"count":resets}),
        AssertionKind::SelectedCar { car } => {
            let actual = &world.resource::<CarSpec>().id;
            json!({"passed":actual == car,"car":actual})
        }
        AssertionKind::CommandRejected { reason, count } => {
            let actual = rejections.get(reason).copied().unwrap_or(0);
            json!({"passed":actual == *count,"count":actual})
        }
    }
}
fn request_capture(world: &mut World) {
    let run = world.resource::<RunHandle>();
    let path = run.dir().join("captures/scenario.png");
    let capture = world
        .get_resource::<QaRenderTarget>()
        .and_then(|t| t.0.clone())
        .map_or_else(Screenshot::primary_window, Screenshot::image);
    world
        .spawn((capture, ScenarioCapture))
        .observe(save_to_disk(path))
        .observe(validate_capture);
}
fn validate_capture(
    capture: On<ScreenshotCaptured>,
    requests: Query<(), With<ScenarioCapture>>,
    mut state: ResMut<ScenarioRun>,
) {
    if requests.get(capture.entity).is_err() {
        return;
    }
    match capture_metrics(&capture.image) {
        Some(metrics) if metrics.nonblank => state.capture = serde_json::to_value(metrics).ok(),
        _ => state.capture_error = Some("required-capture-invalid".into()),
    }
}
fn finish(world: &mut World, state: &mut ScenarioRun, failure: Option<&str>) {
    if state.finished {
        return;
    }
    state.finished = true;
    world
        .resource_mut::<Time<Fixed>>()
        .discard_overstep(Duration::MAX);
    crate::runtime::discard_pending(world, "scenario-ended");
    let all_asserted = state.results.len() == state.definition.assertions.len();
    let passed_count = state.results.iter().filter(|r| r["passed"] == true).count();
    let passed =
        failure.is_none() && all_asserted && passed_count == state.definition.assertions.len();
    let clock = world.resource::<SimulationClock>();
    let ticks = state
        .ready
        .map_or(0, |(start, _, _)| clock.completed - start);
    let seed = world.resource::<RuntimeConfig>().seed;
    if let Some(mut trace) = world.get_resource_mut::<PhysicsTrace>() {
        trace.flush();
    }
    let run = world.resource::<RunHandle>();
    let capture_path = run.dir().join("captures/scenario.png");
    let capture = state.capture.as_ref().map(|metrics| json!({"path":"captures/scenario.png","metrics":metrics,"sha256":crate::diagnostics::sha256_file(&capture_path)}));
    let capture_missing = state.definition.required_capture
        && !capture.as_ref().is_some_and(|c| c["sha256"].is_string());
    let failure = failure.or(if capture_missing {
        Some("required-capture-missing")
    } else {
        None
    });
    let passed = passed && !capture_missing;
    let summary = json!({
        "schema_version": 1,
        "name": state.definition.name,
        "seed": seed,
        "passed": passed,
        "failure": failure,
        "ticks": ticks,
        "completed_tick": world.resource::<SimulationClock>().completed,
        "max_ticks": state.definition.max_ticks,
        "max_wall_ms": state.definition.max_wall_ms,
        "readiness_timeout_ms": state.definition.readiness_timeout_ms,
        "elapsed_ms": state.start.elapsed().as_millis(),
        "assertions_declared": state.definition.assertions.len(),
        "assertions_executed": state.results.len(),
        "assertions_passed": passed_count,
        "results": state.results,
        "capture": capture,
        "readiness": state.ready.map(|(tick, _, _)| json!({"tick": tick})),
        "artifacts": ["manifest.json", "scenario.json", "commands.jsonl", "input.jsonl", "physics.jsonl", "events.jsonl"]
    });
    run.event("scenario.finished", summary.clone());
    if !passed {
        run.fail(
            failure.unwrap_or("scenario-assertion-failed"),
            summary.clone(),
        );
    }
    write_json(&run.dir().join("scenario-summary.json"), &summary);
    world.write_message(if passed {
        AppExit::Success
    } else {
        AppExit::error()
    });
}
fn write_json(path: &std::path::Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec_pretty(value).expect("json"))
        .unwrap_or_else(|error| panic!("write scenario evidence {}: {error}", path.display()));
}

fn collect_outcomes(
    mut messages: MessageReader<crate::runtime::CommandOutcome>,
    mut state: ResMut<ScenarioRun>,
) {
    for outcome in messages.read() {
        if outcome.disposition == "rejected" {
            *state.rejections.entry(outcome.reason.clone()).or_default() += 1;
        }
    }
}

fn work_complete(world: &World, state: &ScenarioRun) -> bool {
    state.results.len() == state.definition.assertions.len()
        && state.submitted.len() == state.definition.commands.len()
        && !world.resource::<CommandBus>().has_pending()
        && world.resource::<crate::runtime::PendingReset>().0.is_none()
}
