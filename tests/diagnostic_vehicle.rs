//! Check production decisions against historical failure reproductions.
#[allow(dead_code)]
#[path = "../examples/stress/case.rs"]
mod case;
#[allow(dead_code)]
#[path = "../examples/stress/city.rs"]
mod city;
#[allow(dead_code)]
#[path = "../examples/stress/fixture.rs"]
mod fixture;
use bevy::prelude::*;
use car_game::{
    RuntimeConfig,
    diagnostics::{PhysicsTrace, RecorderConfig, RunHandle, RunRecorder},
    vehicle::DriveInput,
};
use serde_json::Value;
use std::{fs, sync::Arc};

fn recorder(name: &str, car: &str) -> Arc<RunRecorder> {
    let root = std::env::temp_dir().join(format!("grid-decision-{name}-{}", std::process::id()));
    Arc::new(
        RunRecorder::try_create_with_config(
            &root,
            "test",
            car,
            RecorderConfig {
                queue_records: 32768,
                queue_bytes: 32 * 1024 * 1024,
                ..default()
            },
        )
        .unwrap(),
    )
}
fn rows(run: &RunRecorder, name: &str) -> Vec<Value> {
    fs::read_to_string(run.dir().join(name))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
#[test]
fn historical_nsx_acceleration_does_not_hunt_or_starve_engine_demand() {
    let spec = car_game::roster::Fleet::default()
        .cars
        .into_iter()
        .find(|c| c.id == "exp-231")
        .unwrap();
    let case = case::Case::at(49);
    let mut app = fixture::flat(spec, &case);
    let run = recorder("nsx", "exp-231");
    app.insert_resource(RunHandle(run.clone()))
        .insert_resource(PhysicsTrace::for_recorder(run.clone()));
    for tick in 0..case.ticks {
        *app.world_mut().resource_mut::<DriveInput>() = case.input(tick);
        app.update();
    }
    assert_eq!(run.finish(AppExit::Success), AppExit::Success);
    let samples = rows(&run, "physics.jsonl");
    let changes = samples
        .windows(2)
        .filter(|w| w[0]["gear"] != w[1]["gear"])
        .count();
    assert!(
        changes <= 10,
        "gear hunting: {changes} changes; {}",
        run.dir().display()
    );
    assert!(samples.iter().any(|s| s["gear"].as_i64().unwrap() >= 3));
    let tail = &samples[samples.len() - 1200..];
    let interrupted = tail
        .iter()
        .filter(|s| s["decisions"]["engine_demand_n"].as_f64().unwrap().abs() < 0.001)
        .count();
    assert!(
        interrupted < 180,
        "engine demand interrupted for {interrupted}/1200 ticks"
    );
    let events = rows(&run, "events.jsonl");
    assert!(
        !events
            .iter()
            .any(|e| e.to_string().contains("gear_hunting"))
    );
    assert!(samples.iter().any(|s| {
        s["decisions"]["shift_reason"] == "upshift_rpm_threshold"
            && s["decisions"]["drive_cutoff"] == "shift_in_progress"
    }));
    drop(app);
    fs::remove_dir_all(run.dir().parent().unwrap()).unwrap();
}

#[test]
fn original_mini_curb_collision_exposes_contact_rejection_and_freezes_history() {
    let spec = car_game::roster::Fleet::default()
        .cars
        .into_iter()
        .find(|c| c.id == "exp-037")
        .unwrap();
    let config = RuntimeConfig {
        car: spec.id.clone(),
        backend: car_game::RenderBackend::Disabled,
        ..default()
    };
    let mut app = city::app(config, spec);
    let run = recorder("mini", "exp-037");
    app.insert_resource(RunHandle(run.clone()))
        .insert_resource(PhysicsTrace::for_recorder(run.clone()));
    app.world_mut()
        .resource_mut::<car_game::runtime::CommandBus>()
        .recorder = Some(run.clone());
    for tick in 0..1800 {
        *app.world_mut().resource_mut::<DriveInput>() = DriveInput {
            throttle: if tick >= 600 { 1. } else { 0. },
            steer: if tick >= 1320 { 0.45 } else { 0. },
            handbrake: (1440..1500).contains(&tick),
            ..default()
        };
        app.update();
    }
    assert_eq!(run.finish(AppExit::Success), AppExit::Success);
    let samples = rows(&run, "physics.jsonl");
    assert!(samples.iter().any(|s| {
        s["decisions"]["wheels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["contact"] == "ray_miss")
    }));
    let incident: Value =
        serde_json::from_slice(&fs::read(run.dir().join("incidents/0001/incident.json")).unwrap())
            .unwrap();
    assert_eq!(
        incident["reason"],
        "contact_loss",
        "{}",
        run.dir().display()
    );
    assert_eq!(incident["coverage"]["post_complete"], true);
    drop(app);
    fs::remove_dir_all(run.dir().parent().unwrap()).unwrap();
}
