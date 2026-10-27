use bevy::prelude::*;
use car_game::{
    RenderBackend, RuntimeConfig, diagnostics::RunRecorder, scenario::ScenarioDefinition,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

fn root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("course-{}-{label}", std::process::id()))
}
fn curve(reverse: bool) -> Value {
    serde_json::from_str(if reverse {
        include_str!("../examples/scenarios/curve-reverse.json")
    } else {
        include_str!("../examples/scenarios/curve-forward.json")
    })
    .unwrap()
}
fn execute(value: Value, label: &str) -> (Value, PathBuf) {
    let scenario = ScenarioDefinition::from_json(&value.to_string()).unwrap();
    let config = RuntimeConfig {
        backend: RenderBackend::Disabled,
        fast_scenario: true,
        mode: "scenario".into(),
        car: scenario.initial.car.clone(),
        seed: scenario.seed,
        scenario: Some(scenario),
        ..default()
    };
    let run = Arc::new(
        RunRecorder::try_create_with_config(
            &root(label),
            "scenario",
            "350z",
            car_game::diagnostics::RecorderConfig {
                queue_bytes: 32 * 1024 * 1024,
                queue_records: 32768,
                ..default()
            },
        )
        .unwrap(),
    );
    let mut app = car_game::build_app(config, run.clone()).unwrap();
    for _ in 0..6000 {
        app.update();
        if app.should_exit().is_some() {
            break;
        }
    }
    let exit = app.should_exit().expect("scenario exits within tick bound");
    drop(app);
    let finalized = run.finish(exit);
    let summary: Value =
        serde_json::from_slice(&std::fs::read(run.dir().join("scenario-summary.json")).unwrap())
            .unwrap();
    assert_eq!(finalized.is_success(), summary["passed"] == true);
    (summary, run.dir().to_path_buf())
}

#[test]
fn course_selection_adds_only_opt_in_connector_colliders() {
    let mut cities = Vec::new();
    for course in ["city", "northeast_curve"] {
        let mut value = curve(false);
        value["course"] = course.into();
        value["initial"]["position"] = json!([3.2, 0.62, 190.]);
        let scenario = ScenarioDefinition::from_json(&value.to_string()).unwrap();
        let config = RuntimeConfig {
            backend: RenderBackend::Disabled,
            mode: "scenario".into(),
            scenario: Some(scenario),
            ..default()
        };
        let run = Arc::new(RunRecorder::create_in(&root(course), "scenario", "350z"));
        let mut app = car_game::build_app(config, run.clone()).unwrap();
        app.update();
        let mut geometry = BTreeMap::new();
        let mut connector = 0;
        for (id, pose) in app
            .world_mut()
            .query::<(&car_game::physics::ColliderIdentity, &Transform)>()
            .iter(app.world())
        {
            if id.0.starts_with("city:") {
                geometry.insert(
                    id.0.clone(),
                    (pose.translation.to_array(), pose.scale.to_array()),
                );
            }
            connector += usize::from(id.0.starts_with("course:curve:"));
        }
        assert_eq!(connector, if course == "city" { 0 } else { 96 });
        cities.push(geometry);
        drop(app);
        run.finish(bevy::app::AppExit::Success);
        std::fs::remove_dir_all(root(course)).unwrap();
    }
    assert!(!cities[0].is_empty());
    assert_eq!(cities[0], cities[1]);
    assert_eq!(car_game::roster::Fleet::default().cars.len(), 13);
    let mut omitted = curve(false);
    omitted.as_object_mut().unwrap().remove("course");
    assert_eq!(
        ScenarioDefinition::from_json(&omitted.to_string())
            .unwrap()
            .course,
        car_game::scenario::Course::City
    );
}

#[test]
fn accelerated_routes_use_completed_steps_and_player_command_identity() {
    for reverse in [false, true] {
        let label = if reverse { "reverse" } else { "forward" };
        let mut fixture = curve(reverse);
        fixture["commands"] = json!([{"tick":0,"command":{"action":"spawn_vehicle","id":"route-observer","car":"gt4-nsx","position":[3.2,0.62,190.],"controller":"scenario"}}]);
        fixture["assertions"].as_array_mut().unwrap().push(json!({"tick":5400,"kind":"vehicle_state","vehicle":"route-observer","car":"gt4-nsx","controller":"scenario","reset_count":0,"min_forward_mps":-0.01,"max_forward_mps":0.01}));
        let (summary, dir) = execute(fixture, label);
        assert_eq!(summary["passed"], true, "{summary}");
        let observed = &summary["results"][0]["observed"];
        assert_eq!(observed["boundary"], "completed_tick");
        assert!(observed["active_ticks"].as_u64().unwrap() > 240);
        let commands = std::fs::read_to_string(dir.join("commands.jsonl")).unwrap();
        assert!(
            commands
                .lines()
                .map(|s| serde_json::from_str::<Value>(s).unwrap())
                .any(|r| r["command"]["action"] == "vehicle"
                    && r["command"]["target"] == "pilot-350z")
        );
        std::fs::remove_dir_all(root(label)).unwrap();
    }
}

#[test]
fn unfinished_route_cannot_pass_on_car_identity_alone() {
    let mut value = curve(false);
    value["max_ticks"] = 31.into();
    value["assertions"][0]["tick"] = 30.into();
    let (summary, _) = execute(value, "unfinished");
    assert_eq!(summary["passed"], false);
    assert_eq!(summary["results"][0]["observed"]["completed"], false);
    std::fs::remove_dir_all(root("unfinished")).unwrap();
}

#[test]
fn fast_execution_rejects_wall_triggers_and_visual_or_non_scenario_modes() {
    assert!(RuntimeConfig::from_args(["--fast-scenario"]).is_err());
    let mut scenario = ScenarioDefinition::from_json(&curve(false).to_string()).unwrap();
    let mut config = RuntimeConfig {
        backend: RenderBackend::Disabled,
        mode: "scenario".into(),
        fast_scenario: true,
        scenario: Some(scenario.clone()),
        ..default()
    };
    assert!(config.validate_execution().is_ok());
    config.backend = RenderBackend::Offscreen;
    assert!(config.validate_execution().is_err());
    config.backend = RenderBackend::Disabled;
    scenario
        .commands
        .push(car_game::scenario::ScheduledCommand {
            tick: None,
            elapsed_ms: Some(1),
            command: car_game::runtime::ProductionCommand::Reset,
        });
    config.scenario = Some(scenario);
    assert!(config.validate_execution().is_err());
    let mut invalid = curve(false);
    invalid["route"]["points"] = json!([[0., 0.], [0., 0.]]);
    assert!(ScenarioDefinition::from_json(&invalid.to_string()).is_err());
}
