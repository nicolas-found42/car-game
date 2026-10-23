static FIXTURE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
use car_game::{RuntimeConfig, scenario::ScenarioDefinition};

#[test]
fn scenario_requires_an_observable_outcome() {
    let result = ScenarioDefinition::from_json(
        r#"{
        "schema_version":1,"name":"empty","scene":"driving",
        "initial":{"car":"350z"},"commands":[],"assertions":[],
        "max_ticks":600,"max_wall_ms":10000,"readiness_timeout_ms":1000
    }"#,
    );
    assert!(result.unwrap_err().contains("assertion"));
    assert!(RuntimeConfig::from_args(["--scenario-filter", "nothing"]).is_err());
}

fn run_fixture(value: serde_json::Value) -> (serde_json::Value, std::path::PathBuf) {
    run_fixture_with_frame_duration(value, std::time::Duration::from_secs_f64(1.0 / 120.0))
}
fn run_fixture_with_frame_duration(
    value: serde_json::Value,
    duration: std::time::Duration,
) -> (serde_json::Value, std::path::PathBuf) {
    use bevy::{prelude::*, time::TimeUpdateStrategy};
    let scenario = ScenarioDefinition::from_json(&value.to_string()).unwrap();
    let root = std::env::temp_dir().join(format!(
        "grid-scenario-{}-{}-{}",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = RuntimeConfig {
        backend: car_game::RenderBackend::Disabled,
        mode: "scenario".into(),
        car: scenario.initial.car.clone(),
        seed: scenario.seed,
        access: scenario.context.clone(),
        scenario: Some(scenario),
        ..Default::default()
    };
    let run = std::sync::Arc::new(car_game::diagnostics::RunRecorder::create_in(
        &root,
        "scenario",
        &config.car,
    ));
    let dir = run.dir().to_path_buf();
    let mut app = car_game::build_app(config, run.clone()).unwrap();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(duration));
    let guard = std::time::Instant::now();
    for _ in 0..100000 {
        app.update();
        if app.should_exit().is_some() || guard.elapsed().as_secs() >= 5 {
            break;
        }
    }
    assert!(
        app.should_exit().is_some(),
        "scenario must exit within its simulation bound"
    );
    run.finish(app.should_exit().unwrap());
    drop(app);
    let summary =
        serde_json::from_slice(&std::fs::read(dir.join("scenario-summary.json")).unwrap()).unwrap();
    (summary, dir)
}

fn driving_fixture() -> serde_json::Value {
    serde_json::json!({"schema_version":1,"name":"driving-reset","scene":"driving","seed":17,
        "initial":{"car":"350z"},"max_ticks":700,"max_wall_ms":10000,"readiness_timeout_ms":1000,
        "commands":[{"tick":0,"command":{"action":"drive","throttle":1.0,"brake":0.0,"steer":0.0}},
                    {"tick":500,"command":{"action":"reset"}}],
        "assertions":[{"tick":480,"kind":"displacement_at_least","meters":10.0},
                      {"tick":501,"kind":"reset_count","count":1}]})
}

#[test]
fn real_driving_and_one_shot_reset_use_the_production_application() {
    let (summary, dir) = run_fixture(driving_fixture());
    assert_eq!(summary["passed"], true, "{summary}");
    assert_eq!(summary["assertions_executed"], 2);
    let commands: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("commands.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let resets: Vec<_> = commands
        .iter()
        .filter(|line| line["command"]["action"] == "reset")
        .collect();
    assert_eq!(resets.len(), 1);
    let events: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("events.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let transitions: Vec<_> = events
        .iter()
        .filter(|e| e["kind"] == "vehicle.reset")
        .collect();
    assert_eq!(transitions.len(), 1);
    assert_eq!(transitions[0]["command_id"], resets[0]["command_id"]);
    let tick = transitions[0]["tick"].as_u64().unwrap();
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "simulation.completed" && e["tick"] == tick)
    );
    std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn pause_resume_and_selection_keep_real_ingress_timing() {
    let value = serde_json::json!({"schema_version":1,"name":"pause-selection","scene":"driving",
        "initial":{"car":"350z"},"max_ticks":100,"max_wall_ms":2000,"readiness_timeout_ms":1000,
        "commands":[{"elapsed_ms":100,"command":{"action":"set_paused","paused":true}},
                    {"elapsed_ms":200,"command":{"action":"set_paused","paused":false}},
                    {"tick":40,"command":{"action":"select_car","car":"redline-911"}}],
        "assertions":[{"tick":70,"kind":"selected_car","car":"redline-911"}]});
    let (summary, dir) = run_fixture(value);
    assert_eq!(summary["passed"], true, "{summary}");
    let commands: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("commands.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let pause: Vec<_> = commands
        .iter()
        .filter(|c| c["command"]["action"] == "set_paused")
        .collect();
    assert_eq!(pause.len(), 2);
    assert_eq!(pause[0]["tick"], pause[1]["tick"]);
    assert!(
        pause[0]["render_frame"].as_u64().unwrap() < pause[1]["render_frame"].as_u64().unwrap()
    );
    assert!(pause[0]["elapsed_ns"].as_u64().unwrap() < pause[1]["elapsed_ns"].as_u64().unwrap());
    assert!(
        commands
            .iter()
            .any(|c| c["command"]["action"] == "select_car")
    );
    std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn explicit_career_selection_enforces_ownership_with_rendering_disabled() {
    let value = serde_json::json!({"schema_version":1,"name":"career-ownership","scene":"driving",
        "context":{"kind":"career","owner_id":"fixture-owner","owned_cars":["350z"]},
        "initial":{"car":"350z"},"max_ticks":100,"max_wall_ms":2000,"readiness_timeout_ms":1000,
        "commands":[{"tick":5,"command":{"action":"select_car","car":"redline-911"}},
                    {"tick":10,"command":{"action":"select_car","car":"not-a-car"}}],
        "assertions":[{"tick":20,"kind":"selected_car","car":"350z"},
                      {"tick":20,"kind":"command_rejected","reason":"not-owned","count":1},
                      {"tick":20,"kind":"command_rejected","reason":"unknown-car","count":1}]});
    let (summary, dir) = run_fixture(value);
    assert_eq!(summary["passed"], true, "{summary}");
    std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn required_capture_cannot_pass_without_a_renderer() {
    let mut fixture = driving_fixture();
    fixture["required_capture"] = true.into();
    let (summary, dir) = run_fixture(fixture);
    assert_eq!(summary["passed"], false);
    assert_eq!(summary["failure"], "required-capture-unavailable");
    assert_eq!(summary["assertions_executed"], 0);
    assert!(dir.join("failure.json").is_file());
    std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

fn sandbox_app() -> (
    bevy::prelude::App,
    std::sync::Arc<car_game::diagnostics::RunRecorder>,
) {
    use bevy::time::TimeUpdateStrategy;
    let root = std::env::temp_dir().join(format!(
        "grid-controls-{}-{}-{}",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let run = std::sync::Arc::new(car_game::diagnostics::RunRecorder::create_in(
        &root, "controls", "350z",
    ));
    let mut app = car_game::build_app(
        RuntimeConfig {
            backend: car_game::RenderBackend::Disabled,
            ..Default::default()
        },
        run.clone(),
    )
    .unwrap();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 120.0),
    ));
    app.update();
    app.update();
    (app, run)
}
fn key_event(
    app: &mut bevy::prelude::App,
    code: bevy::prelude::KeyCode,
    state: bevy::input::ButtonState,
) {
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: code,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            state,
            text: None,
            repeat: false,
            window: bevy::prelude::Entity::PLACEHOLDER,
        });
}
#[test]
fn keyboard_car_selection_wraps_the_roster_through_production_ingress() {
    use bevy::{input::ButtonState, prelude::*};
    let (mut app, run) = sandbox_app();
    key_event(&mut app, KeyCode::BracketLeft, ButtonState::Pressed);
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<car_game::roster::CarSpec>().id,
        "redline-gt500"
    );
    key_event(&mut app, KeyCode::BracketLeft, ButtonState::Released);
    key_event(&mut app, KeyCode::BracketRight, ButtonState::Pressed);
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<car_game::roster::CarSpec>().id,
        "350z"
    );
    drop(app);
    assert!(matches!(
        run.finish(bevy::app::AppExit::Success),
        bevy::app::AppExit::Success
    ));
    std::fs::remove_dir_all(run.dir().parent().unwrap()).unwrap();
}

#[test]
fn final_replay_sample_reset_is_consumed_before_live_ownership_returns() {
    let (mut app, run) = sandbox_app();
    app.insert_resource(car_game::diagnostics::ReplayInput {
        active: true,
        samples: vec![car_game::diagnostics::InputSample {
            throttle: 1.0,
            reset: true,
            ..Default::default()
        }],
        ..Default::default()
    });
    app.world_mut()
        .resource_mut::<car_game::runtime::CommandBus>()
        .submit(
            car_game::runtime::ControllerOwner::Replay,
            car_game::runtime::ProductionCommand::AssignController {
                controller: car_game::runtime::ControllerOwner::Replay,
            },
        );
    app.update();
    app.update();
    let mut query = app.world_mut().query::<&car_game::vehicle::Vehicle>();
    assert_eq!(query.single(app.world()).unwrap().reset_count, 1);
    assert_eq!(
        app.world()
            .resource::<car_game::runtime::ControllerAssignment>()
            .0,
        car_game::runtime::ControllerOwner::LegacyComposite
    );
    drop(app);
    assert!(matches!(
        run.finish(bevy::app::AppExit::Success),
        bevy::app::AppExit::Success
    ));
    std::fs::remove_dir_all(run.dir().parent().unwrap()).unwrap();
}

#[test]
fn gamepad_bumper_selects_the_next_car_through_production_ingress() {
    use bevy::{
        input::gamepad::{RawGamepadButtonChangedEvent, RawGamepadEvent},
        prelude::*,
    };
    let (mut app, run) = sandbox_app();
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    app.world_mut()
        .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            pad,
            GamepadButton::RightTrigger,
            1.0,
        )));
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<car_game::roster::CarSpec>().id,
        "fr2-cobra"
    );
    drop(app);
    assert!(matches!(
        run.finish(bevy::app::AppExit::Success),
        bevy::app::AppExit::Success
    ));
    std::fs::remove_dir_all(run.dir().parent().unwrap()).unwrap();
}

#[test]
fn readiness_timeouts_execute_no_assertions_and_retain_failure_evidence() {
    let mut fixture = driving_fixture();
    fixture["initial"]["position"] = serde_json::json!([3.2, 100000.0, 190.0]);
    fixture["readiness_timeout_ms"] = 5.into();
    let (summary, dir) = run_fixture(fixture);
    assert_eq!(summary["passed"], false);
    assert_eq!(summary["failure"], "readiness-timeout");
    assert_eq!(summary["assertions_executed"], 0);
    assert!(dir.join("failure.json").is_file());
    std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn a_declared_motion_assertion_can_fail_on_the_actual_car() {
    let fixture = serde_json::json!({"schema_version":1,"name":"stationary-failure","scene":"driving",
        "initial":{"car":"350z"},"max_ticks":50,"max_wall_ms":2000,"readiness_timeout_ms":1000,
        "commands":[],"assertions":[{"tick":20,"kind":"displacement_at_least","meters":10.0}]});
    let (summary, dir) = run_fixture(fixture);
    assert_eq!(summary["passed"], false);
    assert_eq!(summary["assertions_executed"], 1);
    assert_eq!(summary["assertions_passed"], 0);
    std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn a_paused_scenario_still_fails_at_its_real_wall_deadline() {
    let fixture = serde_json::json!({"schema_version":1,"name":"paused-deadline","scene":"driving",
        "initial":{"car":"350z"},"max_ticks":100,"max_wall_ms":3000,"readiness_timeout_ms":2000,
        "commands":[{"elapsed_ms":0,"command":{"action":"set_paused","paused":true}}],
        "assertions":[{"tick":100,"kind":"reset_count","count":0}]});
    let (summary, dir) = run_fixture(fixture);
    assert_eq!(summary["passed"], false);
    assert_eq!(summary["failure"], "wall-time-limit");
    let commands = std::fs::read_to_string(dir.join("commands.jsonl")).unwrap();
    assert!(commands.lines().any(
        |line| serde_json::from_str::<serde_json::Value>(line).unwrap()["command"]["action"]
            == "set_paused"
    ));
    assert_eq!(summary["assertions_executed"], 0);
    std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn an_empty_legacy_replay_keeps_live_driving_available() {
    use bevy::{input::ButtonState, prelude::*};
    let root = std::env::temp_dir().join(format!(
        "grid-empty-replay-{}-{}",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("empty-input.jsonl");
    std::fs::write(&path, "").unwrap();
    let run = std::sync::Arc::new(car_game::diagnostics::RunRecorder::create_in(
        &root, "replay", "350z",
    ));
    let mut app = car_game::build_app(
        RuntimeConfig {
            backend: car_game::RenderBackend::Disabled,
            mode: "replay".into(),
            replay_path: Some(path),
            ..Default::default()
        },
        run.clone(),
    )
    .unwrap();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 120.0),
    ));
    app.update();
    app.update();
    key_event(&mut app, KeyCode::KeyW, ButtonState::Pressed);
    for _ in 0..240 {
        app.update();
    }
    let mut cars = app.world_mut().query::<&bevy_rapier3d::prelude::Velocity>();
    assert!(-cars.single(app.world()).unwrap().linear.z > 4.0);
    drop(app);
    assert!(matches!(
        run.finish(bevy::app::AppExit::Success),
        bevy::app::AppExit::Success
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_late_selection_is_applied_before_a_scenario_can_finish_successfully() {
    let fixture = serde_json::json!({"schema_version":1,"name":"late-selection","scene":"driving",
        "initial":{"car":"350z"},"max_ticks":1000,"max_wall_ms":2000,"readiness_timeout_ms":1000,
        "commands":[{"elapsed_ms":200,"command":{"action":"select_car","car":"redline-911"}}],
        "assertions":[{"tick":5,"kind":"reset_count","count":0}]});
    let (summary, dir) = run_fixture(fixture);
    assert_eq!(summary["passed"], true, "{summary}");
    let events: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("events.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let selection = events
        .iter()
        .find(|e| {
            e["kind"] == "command.outcome" && e["fields"]["command"]["action"] == "select_car"
        })
        .unwrap();
    assert_eq!(selection["fields"]["disposition"], "applied");
    std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn the_tick_bound_stops_catch_up_work_at_the_declared_completed_boundary() {
    let fixture = serde_json::json!({"schema_version":1,"name":"bounded-catch-up","scene":"driving",
        "initial":{"car":"350z"},"max_ticks":5,"max_wall_ms":2000,"readiness_timeout_ms":1000,
        "commands":[],"assertions":[{"tick":5,"kind":"reset_count","count":0}]});
    let (summary, dir) =
        run_fixture_with_frame_duration(fixture, std::time::Duration::from_millis(100));
    assert_eq!(summary["passed"], true, "{summary}");
    assert_eq!(summary["ticks"], 5);
    let events: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("events.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let last_tick = events
        .iter()
        .rfind(|e| e["kind"] == "simulation.tick_completed")
        .unwrap();
    assert_eq!(last_tick["tick"], summary["completed_tick"]);
    std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn the_public_builder_rejects_a_sandbox_override_of_an_explicit_career_scenario() {
    let fixture = serde_json::json!({"schema_version":1,"name":"career-conflict","scene":"driving",
        "context":{"kind":"career","owner_id":"fixture-owner","owned_cars":["350z"]},
        "initial":{"car":"350z"},"max_ticks":100,"max_wall_ms":2000,"readiness_timeout_ms":1000,
        "commands":[],"assertions":[{"tick":5,"kind":"selected_car","car":"350z"}]});
    let scenario = ScenarioDefinition::from_json(&fixture.to_string()).unwrap();
    let root = std::env::temp_dir().join(format!(
        "grid-career-conflict-{}-{}",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let run = std::sync::Arc::new(car_game::diagnostics::RunRecorder::create_in(
        &root, "scenario", "350z",
    ));
    let config = RuntimeConfig {
        backend: car_game::RenderBackend::Disabled,
        mode: "scenario".into(),
        scenario: Some(scenario),
        ..Default::default()
    };
    let Err(error) = car_game::build_app(config, run.clone()) else {
        panic!("an explicit career scenario cannot use sandbox access");
    };
    assert!(error.contains("context"));
    run.finish(bevy::app::AppExit::error());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn assembly_transactions_use_production_commands_and_completed_tick_inspection() {
    let fixture = serde_json::json!({"schema_version":1,"name":"assembly-wheel","scene":"driving","initial":{"car":"350z"},"max_ticks":30,"max_wall_ms":10000,"readiness_timeout_ms":1000,
    "commands":[
      {"tick":0,"command":{"action":"inspect_assembly"}},
      {"tick":2,"command":{"action":"assembly_transaction","transaction":{"id":"remove","expected_revision":0,"part":"350z-wheel-0","destination":{"kind":"root","location":{"kind":"inventory","owner":"sandbox","slot":"rack-0"}}}}},
      {"tick":4,"command":{"action":"assembly_transaction","transaction":{"id":"stale","expected_revision":0,"part":"350z-wheel-1","destination":{"kind":"root","location":{"kind":"inventory","owner":"sandbox","slot":"rack-1"}}}}},
      {"tick":6,"command":{"action":"assembly_transaction","transaction":{"id":"restore","expected_revision":1,"part":"350z-wheel-0","destination":{"kind":"mounted","parent":"350z-chassis","mount":"wheel-0"}}}}
    ],"assertions":[
      {"tick":1,"kind":"assembly_state","revision":0,"mass":1320.,"readiness":"ready"},
      {"tick":3,"kind":"assembly_state","revision":1,"mass":1295.,"readiness":"incomplete-inoperable"},
      {"tick":5,"kind":"command_rejected","reason":"stale-revision","count":1},
      {"tick":7,"kind":"assembly_state","revision":2,"mass":1320.,"readiness":"ready"}
    ]});
    let (summary, dir) = run_fixture(fixture);
    assert_eq!(summary["passed"], true, "{summary}");
    assert!(
        std::fs::read_to_string(dir.join("events.jsonl"))
            .unwrap()
            .contains("assembly.inspection")
    );
}

#[test]
fn simultaneous_cars_use_independent_geometry_inputs_and_exclusive_controllers() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../examples/scenarios/independent-vehicles.json"
    ))
    .unwrap();
    let (summary, dir) = run_fixture(fixture);
    assert_eq!(summary["passed"], true, "{summary}");
    assert_eq!(summary["assertions_executed"], 5);
    let events: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("events.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let physics: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("physics.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let player = physics
        .iter()
        .find(|v| v["actor_id"] == "player-vehicle" && v["input"]["throttle"] == 1.0)
        .unwrap();
    let second = physics
        .iter()
        .find(|v| v["actor_id"] == "second-car" && v["tick"] == player["tick"])
        .unwrap();
    assert_eq!(second["input"]["throttle"], 0.0);
    assert_eq!(second["input"]["brake"], 1.0);
    // A target has exactly one effective input event at every completed solver tick.
    let mut seen = std::collections::HashSet::new();
    for event in events.iter().filter(|v| v["kind"] == "controls.effective") {
        assert!(
            seen.insert((event["actor_id"].to_string(), event["tick"].to_string())),
            "duplicate consumption: {event}"
        );
    }
    let rejected = events
        .iter()
        .filter(|event| {
            event["kind"] == "command.outcome"
                && event["actor_id"] == "second-car"
                && event["fields"]["reason"] == "controller-owner-active"
        })
        .count();
    assert_eq!(rejected, 2);
    let targeted_reset = events
        .iter()
        .find(|event| {
            event["kind"] == "vehicle.reset"
                && event["fields"]["vehicle_id"] == "pilot-350z"
                && event["command_id"].is_string()
        })
        .unwrap();
    let ingress: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("commands.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let original = ingress
        .iter()
        .find(|command| command["command_id"] == targeted_reset["command_id"])
        .unwrap();
    assert_eq!(original["command"]["target"], "pilot-350z");
    let first_geometry = &summary["results"][0]["observed"];
    let second_geometry = &summary["results"][1]["observed"];
    assert_ne!(first_geometry["radii"], second_geometry["radii"]);
    assert_ne!(first_geometry["wheelbase"], second_geometry["wheelbase"]);
}

#[test]
fn live_secondary_controller_resets_once_while_player_replay_keeps_ownership() {
    use bevy::prelude::*;
    use car_game::{
        assembly::VehicleId,
        controls::{VehicleCommand, VehicleControl},
        diagnostics::{InputSample, ReplayInput},
        runtime::{CommandBus, ControllerAssignment, ControllerOwner, ProductionCommand},
        vehicle::Vehicle,
    };
    let (mut app, run) = sandbox_app();
    app.world_mut().insert_resource(ReplayInput {
        samples: vec![
            InputSample {
                throttle: 1.0,
                ..Default::default()
            };
            100
        ],
        active: true,
        ..Default::default()
    });
    let mut bus = app.world_mut().resource_mut::<CommandBus>();
    bus.submit(
        ControllerOwner::Replay,
        ProductionCommand::AssignController {
            controller: ControllerOwner::Replay,
        },
    );
    bus.submit(
        ControllerOwner::LegacyComposite,
        ProductionCommand::SpawnVehicle {
            id: VehicleId::from("live-car"),
            car: "gt4-nsx".into(),
            position: [-3.2, 0.62, 190.0],
            controller: ControllerOwner::LegacyComposite,
        },
    );
    bus.submit(
        ControllerOwner::LegacyComposite,
        ProductionCommand::Vehicle {
            target: VehicleId::from("live-car"),
            command: VehicleCommand::Drive {
                throttle: 0.0,
                brake: 1.0,
                steer: 0.0,
                handbrake: false,
            },
        },
    );
    for _ in 0..40 {
        app.update();
    }
    let secondary = app
        .world_mut()
        .query::<(Entity, &VehicleControl)>()
        .iter(app.world())
        .find(|(_, c)| c.id.0 == "live-car")
        .unwrap()
        .0;
    assert!(
        app.world()
            .get::<bevy_rapier3d::prelude::Velocity>(secondary)
            .unwrap()
            .linear
            .z
            > 0.1
    );
    app.world_mut().resource_mut::<CommandBus>().submit(
        ControllerOwner::LegacyComposite,
        ProductionCommand::Vehicle {
            target: VehicleId::from("live-car"),
            command: VehicleCommand::Reset,
        },
    );
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        app.world().get::<Vehicle>(secondary).unwrap().reset_count,
        1
    );
    let primary_resets = app
        .world_mut()
        .query_filtered::<&Vehicle, With<car_game::controls::PlayerVehicle>>()
        .single(app.world())
        .unwrap()
        .reset_count;
    assert_eq!(primary_resets, 0);
    assert_eq!(
        app.world().resource::<ControllerAssignment>().0,
        ControllerOwner::Replay
    );
    let held = app.world().get::<VehicleControl>(secondary).unwrap();
    assert_eq!(held.owner, ControllerOwner::LegacyComposite);
    assert_eq!(held.input.brake, 1.0);
    assert!(!held.input.reset);
    run.finish(bevy::app::AppExit::Success);
}
