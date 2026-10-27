use bevy::app::AppExit;
use car_game::diagnostics::{EvidenceStream, RunRecorder};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn tick(run: &RunRecorder, at: u64) {
    run.record_evidence(
        EvidenceStream::Events,
        at,
        &json!({"tick":at,"kind":"control"}),
    );
    run.completed_tick(at);
}
#[test]
fn recoverable_windows_include_post_history_and_do_not_replace_fatal_incidents() {
    let root = std::env::temp_dir().join(format!("grid-windows-{}", std::process::id()));
    let run = RunRecorder::create_in(&root, "test", "350z");
    for at in 0..=200 {
        tick(&run, at);
    }
    run.observe_clock(30., 0.25);
    for at in 201..=1000 {
        tick(&run, at);
    }
    run.flush();
    let first = read(run.dir().join("incidents/0001/incident.json"));
    assert_eq!(first["reason"], "clock_gap");
    assert_eq!(first["coverage"]["post_complete"], true);
    assert_eq!(first["behavioral_failure"], false);
    let records =
        fs::read_to_string(run.dir().join("incidents/0001/incident-history.jsonl")).unwrap();
    let ticks: Vec<_> = records
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).unwrap()["tick"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert!(ticks.contains(&100) && ticks.contains(&440));
    run.observe_clock(30., 0.25);
    tick(&run, 1001);
    run.fail("fatal_control", json!({}));
    run.finish(AppExit::error());
    assert_eq!(
        read(run.dir().join("incident.json"))["reason"],
        "fatal_control"
    );
    assert_eq!(
        read(run.dir().join("incidents/0002/incident.json"))["coverage"]["post_complete"],
        false
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recoverable_diagnostics_remain_nonfatal_and_exports_are_bounded() {
    let root = std::env::temp_dir().join(format!("grid-window-caps-{}", std::process::id()));
    let run = RunRecorder::create_in(&root, "test", "350z");
    for at in 0..9000 {
        tick(&run, at);
        if at % 900 == 0 {
            run.flush();
            run.observe_clock(30., 0.25);
        }
    }
    assert_eq!(run.finish(AppExit::Success), AppExit::Success);
    let status = read(run.dir().join("diagnostic-windows.json"));
    assert_eq!(status["triggered"], 8);
    assert!(status["suppressed"].as_u64().unwrap() >= 2);
    assert_eq!(
        fs::read_dir(run.dir().join("incidents")).unwrap().count(),
        8
    );
    assert_eq!(
        read(run.dir().join("recorder-health.json"))["failed"],
        false
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn shortened_history_is_explicit_in_automatic_window_coverage() {
    let root = std::env::temp_dir().join(format!("grid-window-short-{}", std::process::id()));
    let run = RunRecorder::try_create_with_config(
        &root,
        "test",
        "350z",
        car_game::diagnostics::RecorderConfig {
            history_records: 2,
            ..Default::default()
        },
    )
    .unwrap();
    for at in 0..=200 {
        tick(&run, at);
    }
    run.observe_clock(30., 0.25);
    for at in 201..=440 {
        tick(&run, at);
    }
    run.finish(AppExit::Success);
    let incident = read(run.dir().join("incidents/0001/incident.json"));
    assert_eq!(incident["coverage"]["pre_complete"], false);
    assert_eq!(incident["coverage"]["post_complete"], false);
    assert_eq!(incident["coverage"]["history"]["shortened"], true);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn partial_first_post_tick_eviction_cannot_claim_complete_coverage() {
    let root = std::env::temp_dir().join(format!("grid-window-partial-{}", std::process::id()));
    let run = RunRecorder::try_create_with_config(
        &root,
        "test",
        "350z",
        car_game::diagnostics::RecorderConfig {
            history_records: 479,
            ..Default::default()
        },
    )
    .unwrap();
    tick(&run, 200);
    run.observe_clock(30., 0.25);
    for at in 201..=440 {
        run.record_evidence(
            EvidenceStream::Input,
            at,
            &json!({"tick":at,"control":true}),
        );
        tick(&run, at);
    }
    run.finish(AppExit::Success);
    let incident = read(run.dir().join("incidents/0001/incident.json"));
    assert_eq!(incident["coverage"]["first_tick"], 201);
    assert_eq!(incident["coverage"]["post_complete"], false);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_window_publication_cannot_be_reported_as_healthy() {
    let root = std::env::temp_dir().join(format!("grid-window-storage-{}", std::process::id()));
    let run = RunRecorder::create_in(&root, "test", "350z");
    fs::write(run.dir().join("incidents"), b"occupied").unwrap();
    run.observe_clock(30., 0.25);
    for at in 0..=240 {
        tick(&run, at);
    }
    assert_ne!(run.finish(AppExit::Success), AppExit::Success);
    assert_eq!(read(run.dir().join("recorder-health.json"))["failed"], true);
    assert_eq!(
        read(run.dir().join("diagnostic-windows.json"))["exported"],
        0
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prior_recording_loss_prevents_complete_window_claims() {
    struct Lost;
    impl serde::Serialize for Lost {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("lost evidence control"))
        }
    }
    let root = std::env::temp_dir().join(format!("grid-window-loss-{}", std::process::id()));
    let run = RunRecorder::create_in(&root, "test", "350z");
    assert!(!run.record_evidence(EvidenceStream::Events, 0, &Lost));
    run.observe_clock(30., 0.25);
    for at in 0..=240 {
        tick(&run, at);
    }
    run.finish(AppExit::error());
    let incident = read(run.dir().join("incidents/0001/incident.json"));
    assert_eq!(incident["coverage"]["pre_complete"], false);
    assert_eq!(incident["coverage"]["post_complete"], false);
    assert_eq!(incident["coverage"]["recorder_healthy_at_export"], false);
    fs::remove_dir_all(root).unwrap();
}

// A controlled telemetry fault keeps the positive detector covered after fixing
// the production NSX. These samples are detector inputs, not a physics fixture.
fn reversing_sample(tick: u64) -> car_game::diagnostics::PhysicsSample {
    use car_game::diagnostics::{InputSample, decision::VehicleDecisions};
    let gear = if tick <= 12 { 1 + (tick % 2) as i32 } else { 2 };
    let mut decisions = VehicleDecisions::new(gear);
    for wheel in &mut decisions.wheels {
        wheel.contact = "accepted";
    }
    car_game::diagnostics::PhysicsSample {
        decisions: Some(decisions),
        envelope: None,
        vehicle_id: car_game::assembly::VehicleId::from("control"),
        body_id: "control:chassis".into(),
        body_part_id: None,
        backend_applied: None,
        boundary: "completed_tick",
        integration_steps: 1,
        query_mode: "synchronized",
        evidence: car_game::physics::StepEvidence::default(),
        phase_durations: Default::default(),
        step: tick,
        tick,
        actor_id: "controlled-gear-reversals".into(),
        time: tick as f32 / 120.,
        car: "control".into(),
        position: [0.; 3],
        rotation: [0., 0., 0., 1.],
        linear_velocity: [0.; 3],
        angular_velocity: [0.; 3],
        grounded: 4,
        wheel_lengths: [0.34; 4],
        wheel_loads: [2500.; 4],
        steering: 0.,
        speed: 0.,
        slip: 0.,
        rpm: 900.,
        gear,
        drive_demands: [0.; 4],
        longitudinal_forces: [0.; 4],
        reset_count: 0,
        input: InputSample::default(),
        ray_misses: 0,
    }
}

#[test]
fn controlled_shift_reversals_still_export_a_hunting_window() {
    let root = std::env::temp_dir().join(format!("grid-controlled-hunting-{}", std::process::id()));
    let run = RunRecorder::create_in(&root, "test", "control");
    for at in 0..=300 {
        let sample = reversing_sample(at);
        run.record_evidence(EvidenceStream::Physics, at, &sample);
        run.observe_physics(&sample);
        run.completed_tick(at);
    }
    assert_eq!(run.finish(AppExit::Success), AppExit::Success);
    let incident = read(run.dir().join("incidents/0001/incident.json"));
    assert_eq!(incident["reason"], "gear_hunting");
    assert_eq!(incident["actor_id"], "controlled-gear-reversals");
    assert_eq!(incident["coverage"]["post_complete"], true);
    assert_eq!(
        read(run.dir().join("diagnostic-windows.json"))["triggered"],
        1
    );
    assert!(!run.dir().join("incident.json").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn independent_same_model_actors_do_not_invent_gear_hunting() {
    let root =
        std::env::temp_dir().join(format!("grid-independent-monitor-{}", std::process::id()));
    let run = RunRecorder::create_in(&root, "test", "control");
    for tick in 1..=300 {
        for (actor, gear) in [("first", 1), ("second", 2)] {
            let mut sample = reversing_sample(tick);
            sample.actor_id = actor.into();
            sample.gear = gear;
            run.observe_physics(&sample);
        }
        run.completed_tick(tick);
    }
    assert_eq!(run.finish(AppExit::Success), AppExit::Success);
    assert_eq!(
        read(run.dir().join("diagnostic-windows.json"))["triggered"],
        0
    );
    fs::remove_dir_all(root).unwrap();
}
