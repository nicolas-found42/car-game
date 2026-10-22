use bevy::app::AppExit;
use car_game::diagnostics::RunRecorder;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(1);
fn temporary() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "grid-recorder-health-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}
fn read(path: impl AsRef<std::path::Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn failed_report_publication_cannot_finalize_as_success() {
    let root = temporary();
    let run = RunRecorder::create_in(&root, "test", "350z");
    fs::create_dir(run.dir().join("report.json")).unwrap();
    run.record_report(&json!([{"id":"350z","passed":true}]));
    run.finish(AppExit::Success);
    let manifest = read(run.dir().join("manifest.json"));
    assert_ne!(
        manifest["exit"]["code"], 0,
        "a lost report must fail evidence finalization"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unwritable_run_destination_returns_a_structured_startup_error() {
    let root = temporary();
    fs::write(root.join("not-a-directory"), b"occupied").unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_car-game"))
        .args([
            "--scenario",
            "examples/scenarios/driving-reset.json",
            "--render-disabled",
            "--artifact-dir",
        ])
        .arg(root.join("not-a-directory/runs"))
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("recorder_error"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn serialization_errors_are_latched_and_survive_later_success() {
    struct Invalid;
    impl serde::Serialize for Invalid {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom(
                "controlled serialization failure",
            ))
        }
    }
    let root = temporary();
    let run = RunRecorder::create_in(&root, "test", "350z");
    assert!(!run.record_evidence(car_game::diagnostics::EvidenceStream::Events, 10, &Invalid));
    run.event("valid.control", json!({"passed":true}));
    run.finish(AppExit::Success);
    let health = read(run.dir().join("recorder-health.json"));
    assert_eq!(health["failed"], true);
    assert!(
        health["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["stage"] == "serialize")
    );
    assert_ne!(read(run.dir().join("manifest.json"))["exit"]["code"], 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn saturated_recording_has_bounded_backpressure_and_cannot_recover_to_healthy() {
    use car_game::diagnostics::{EvidenceStream, RecorderConfig};
    let root = temporary();
    let config = RecorderConfig {
        queue_records: 2,
        queue_bytes: 2048,
        max_record_bytes: 1024,
        writer_delay_ms: 10,
        ..Default::default()
    };
    let run = RunRecorder::try_create_with_config(&root, "test", "350z", config).unwrap();
    let start = std::time::Instant::now();
    for tick in 1..100 {
        run.record_evidence(
            EvidenceStream::Events,
            tick,
            &json!({"tick":tick,"payload":"x".repeat(400)}),
        );
    }
    assert!(
        start.elapsed() < std::time::Duration::from_millis(500),
        "producers must not wait for a slow writer"
    );
    run.flush();
    run.event("valid.control", json!({"passed":true}));
    run.finish(AppExit::Success);
    let health = read(run.dir().join("recorder-health.json"));
    assert_eq!(health["failed"], true);
    assert!(health["dropped"].as_u64().unwrap() > 0);
    assert!(health["queue_high_bytes"].as_u64().unwrap() <= 2048);
    assert!(health["queue_high_records"].as_u64().unwrap() <= 2);
    assert!(!health["losses"].as_array().unwrap().is_empty());
    assert_ne!(read(run.dir().join("manifest.json"))["exit"]["code"], 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn write_flush_sync_and_drop_failures_preserve_the_last_durable_boundary() {
    use car_game::diagnostics::{
        EvidenceStream, RecorderConfig, RecorderFault, RecorderFaultStage,
    };
    for stage in [
        RecorderFaultStage::Write,
        RecorderFaultStage::Flush,
        RecorderFaultStage::Sync,
        RecorderFaultStage::Drop,
    ] {
        let root = temporary();
        let config = RecorderConfig {
            fault: Some(RecorderFault {
                stage,
                after_records: 12,
            }),
            ..Default::default()
        };
        let run = RunRecorder::try_create_with_config(&root, "test", "350z", config).unwrap();
        for tick in 1..=5 {
            run.record_evidence(EvidenceStream::Events, tick, &json!({"tick":tick}));
        }
        run.completed_tick(5);
        assert!(run.flush());
        assert_eq!(run.health()["last_durable_tick"], 5);
        for tick in 6..=20 {
            run.record_evidence(EvidenceStream::Events, tick, &json!({"tick":tick}));
        }
        run.completed_tick(20);
        let exit = run.finish(AppExit::Success);
        assert!(matches!(exit, AppExit::Error(_)));
        let health = read(run.dir().join("recorder-health.json"));
        assert_eq!(health["failed"], true, "{stage:?}: {health}");
        assert_eq!(health["last_durable_tick"], 5, "{stage:?}: {health}");
        assert!(
            health["errors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["stage"] == stage.name())
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn incidents_keep_thirty_seconds_of_commands_inputs_and_completed_state() {
    use car_game::diagnostics::{EvidenceStream, RecorderConfig};
    let root = temporary();
    let run = RunRecorder::try_create_with_config(
        &root,
        "test",
        "350z",
        RecorderConfig {
            queue_records: 32768,
            ..Default::default()
        },
    )
    .unwrap();
    for tick in 1..=4000 {
        for stream in [
            EvidenceStream::Commands,
            EvidenceStream::Input,
            EvidenceStream::Events,
        ] {
            run.record_evidence(
                stream,
                tick,
                &json!({"tick":tick,"kind":"simulation.completed"}),
            );
        }
        run.completed_tick(tick);
    }
    run.fail("controlled_incident", json!({"tick":4000}));
    run.finish(AppExit::error());
    let history = fs::read_to_string(run.dir().join("incident-history.jsonl"))
        .expect("incident must preserve the in-memory history");
    let rows: Vec<Value> = history
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(rows.iter().all(|row| row["tick"].as_u64().unwrap() >= 401));
    for stream in ["commands", "input", "events"] {
        assert!(
            rows.iter().filter(|row| row["stream"] == stream).count() >= 3600,
            "{stream}"
        );
    }
    let incident = read(run.dir().join("incident.json"));
    assert_eq!(incident["history"]["first_tick"], 401);
    assert_eq!(incident["history"]["last_tick"], 4000);
    assert_eq!(incident["history"]["shortened"], false);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_producers_do_not_lose_evidence_when_capacity_is_available() {
    use car_game::diagnostics::{EvidenceStream, RecorderConfig};
    let root = temporary();
    let run = std::sync::Arc::new(
        RunRecorder::try_create_with_config(
            &root,
            "test",
            "350z",
            RecorderConfig {
                queue_records: 32768,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    std::thread::scope(|scope| {
        for producer in 0..4 {
            let run = run.clone();
            scope.spawn(move || {
                for tick in 1..=200 {
                    assert!(run.record_evidence(
                        EvidenceStream::Events,
                        tick,
                        &json!({"producer":producer,"tick":tick})
                    ));
                }
            });
        }
    });
    assert!(matches!(run.finish(AppExit::Success), AppExit::Success));
    assert_eq!(run.health()["dropped"], 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn incident_windows_handle_delayed_records_without_retaining_old_ticks() {
    use car_game::diagnostics::{EvidenceStream, RecorderConfig};
    let root = temporary();
    let run = RunRecorder::try_create_with_config(
        &root,
        "test",
        "350z",
        RecorderConfig {
            history_ticks: 3,
            ..Default::default()
        },
    )
    .unwrap();
    for tick in [3, 2, 5] {
        run.record_evidence(EvidenceStream::Events, tick, &json!({"tick":tick}));
    }
    run.completed_tick(5);
    run.fail("controlled_incident", json!({}));
    run.finish(AppExit::error());
    let history = fs::read_to_string(run.dir().join("incident-history.jsonl")).unwrap();
    let ticks: Vec<u64> = history
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).unwrap()["tick"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert!(ticks.iter().all(|tick| *tick >= 3), "{ticks:?}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn capacity_truncation_is_explicit_even_when_every_record_has_the_same_tick() {
    use car_game::diagnostics::{EvidenceStream, RecorderConfig};
    let root = temporary();
    let run = RunRecorder::try_create_with_config(
        &root,
        "test",
        "350z",
        RecorderConfig {
            history_ticks: 1,
            history_bytes: 1024,
            history_records: 2,
            ..Default::default()
        },
    )
    .unwrap();
    for _ in 0..100 {
        run.record_evidence(
            EvidenceStream::Input,
            1,
            &json!({"tick":1,"payload":"x".repeat(200)}),
        );
    }
    run.completed_tick(1);
    run.fail("controlled_incident", json!({}));
    run.finish(AppExit::error());
    let incident = read(run.dir().join("incident.json"));
    assert!(incident["history"]["retained_records"].as_u64().unwrap() <= 2);
    assert!(incident["history"]["retained_bytes"].as_u64().unwrap() <= 1024);
    assert_eq!(incident["history"]["shortened"], true);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_durability_follows_the_completed_tick_after_actor_records() {
    use bevy::{prelude::*, time::TimeUpdateStrategy};
    use car_game::{RenderBackend, RuntimeConfig, scenario::ScenarioDefinition};
    let root = temporary();
    let scenario=ScenarioDefinition::from_json(&json!({"schema_version":1,"name":"durability","scene":"driving","initial":{"car":"350z"},"commands":[],"assertions":[{"tick":30,"kind":"selected_car","car":"350z"}],"max_ticks":90,"max_wall_ms":10000,"readiness_timeout_ms":2000}).to_string()).unwrap();
    let config = RuntimeConfig {
        backend: RenderBackend::Disabled,
        mode: "scenario".into(),
        scenario: Some(scenario),
        ..Default::default()
    };
    let run = std::sync::Arc::new(RunRecorder::create_in(&root, "scenario", "350z"));
    let mut app = car_game::build_app(config, run.clone()).unwrap();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 120.0),
    ));
    for _ in 0..200 {
        app.update();
        if app.should_exit().is_some() {
            break;
        }
    }
    let tick = app
        .world()
        .resource::<car_game::runtime::SimulationClock>()
        .completed;
    assert!(tick >= 30);
    assert!(matches!(
        run.finish(app.should_exit().unwrap()),
        AppExit::Success
    ));
    let health = read(run.dir().join("recorder-health.json"));
    assert_eq!(health["last_durable_tick"], tick);
    let events = fs::read_to_string(run.dir().join("events.jsonl")).unwrap();
    assert!(
        events
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .any(|event| event["kind"] == "simulation.completed" && event["tick"] == tick)
    );
    drop(app);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_cli_faults_fail_the_run_and_are_captured_as_effective_settings() {
    let root = temporary();
    let scenario = root.join("scenario.json");
    fs::write(&scenario,json!({"schema_version":1,"name":"fault","scene":"driving","initial":{"car":"350z"},"commands":[],"assertions":[{"tick":30,"kind":"selected_car","car":"350z"}],"max_ticks":90,"max_wall_ms":5000,"readiness_timeout_ms":2000}).to_string()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_car-game"))
        .args(["--render-disabled", "--scenario"])
        .arg(&scenario)
        .arg("--artifact-dir")
        .arg(root.join("runs"))
        .args(["--recorder-fault", "write:20"])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "a requested recording fault must be exercised"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("recorder_error"));
    let dir = fs::read_dir(root.join("runs"))
        .unwrap()
        .filter_map(Result::ok)
        .find(|p| p.path().is_dir())
        .unwrap()
        .path();
    let capture = read(dir.join("provenance/capture.json"));
    let effective = capture["inventory"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["role"] == "effective_config")
        .unwrap();
    let effective = read(dir.join(effective["object"].as_str().unwrap()));
    let settings = &effective["settings"];
    assert_eq!(settings["recorder"]["fault"]["stage"], "write");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn dropping_an_unfinished_recorder_reports_loss_before_returning() {
    use car_game::diagnostics::RecorderConfig;
    let root = temporary();
    let run = RunRecorder::try_create_with_config(
        &root,
        "test",
        "350z",
        RecorderConfig {
            writer_delay_ms: 100,
            ..Default::default()
        },
    )
    .unwrap();
    let dir = run.dir().to_path_buf();
    drop(run);
    let health = read(dir.join("recorder-health.json"));
    assert_eq!(health["failed"], true);
    assert_eq!(health["phase"], "incomplete");
    assert!(
        health["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["stage"] == "drop")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_failure_tails_cannot_expand_an_incident_beyond_the_declared_read_bound() {
    let root = temporary();
    let run = RunRecorder::create_in(&root, "test", "350z");
    fs::write(run.dir().join("run.log"), vec![b'x'; 1024 * 1024]).unwrap();
    run.fail("controlled_incident", json!({}));
    run.finish(AppExit::error());
    let failure = read(run.dir().join("failure.json"));
    let bytes: usize = failure["log_tail"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().len())
        .sum();
    assert!(bytes <= 256 * 1024, "tail copied {bytes} bytes");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_inventory_publication_cannot_leave_a_frozen_success_index() {
    let root = temporary();
    let run = RunRecorder::create_in(&root, "test", "350z");
    run.capture_provenance(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets")),
        "checkout_assets",
        json!({"mode":"test"}),
    );
    fs::create_dir(run.dir().join("evidence-manifest.sha256")).unwrap();
    assert!(matches!(run.finish(AppExit::Success), AppExit::Error(_)));
    assert!(
        !run.dir().join("evidence-manifest.json").exists(),
        "a failed freeze must not publish a successful immutable index"
    );
    assert_eq!(read(run.dir().join("recorder-health.json"))["failed"], true);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn oversized_events_do_not_escape_the_bounded_history_through_legacy_failure_metadata() {
    use car_game::diagnostics::RecorderConfig;
    let root = temporary();
    let run = RunRecorder::try_create_with_config(
        &root,
        "test",
        "350z",
        RecorderConfig {
            max_record_bytes: 1024,
            ..Default::default()
        },
    )
    .unwrap();
    run.event("oversized", json!({"payload":"x".repeat(1024*1024)}));
    run.fail("controlled_incident", json!({}));
    run.finish(AppExit::error());
    let size = fs::metadata(run.dir().join("failure.json")).unwrap().len();
    assert!(
        size < 512 * 1024,
        "legacy failure metadata retained {size} bytes"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_drain_deadline_does_not_leave_the_writer_processing_an_abandoned_queue() {
    use car_game::diagnostics::{EvidenceStream, RecorderConfig};
    let root = temporary();
    let run = RunRecorder::try_create_with_config(
        &root,
        "test",
        "350z",
        RecorderConfig {
            writer_delay_ms: 50,
            drain_timeout_ms: 10,
            ..Default::default()
        },
    )
    .unwrap();
    for tick in 1..100 {
        run.record_evidence(EvidenceStream::Events, tick, &json!({"tick":tick}));
    }
    let start = std::time::Instant::now();
    assert!(matches!(run.finish(AppExit::Success), AppExit::Error(_)));
    assert!(start.elapsed() < std::time::Duration::from_millis(500));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let written = run.health()["written"].clone();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(
        run.health()["written"],
        written,
        "timed-out shutdown must cancel pending work"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn finalization_respects_an_explicit_export_byte_budget() {
    use car_game::diagnostics::RecorderConfig;
    let root = temporary();
    let run = RunRecorder::try_create_with_config(
        &root,
        "test",
        "350z",
        RecorderConfig {
            finalize_max_bytes: 1024,
            ..Default::default()
        },
    )
    .unwrap();
    run.capture_provenance(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets")),
        "checkout_assets",
        json!({"mode":"test"}),
    );
    assert!(matches!(run.finish(AppExit::Success), AppExit::Error(_)));
    assert!(!run.dir().join("evidence-manifest.json").exists());
    let health = read(run.dir().join("recorder-health.json"));
    assert!(
        health["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["stage"] == "finalize")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_trace_or_metadata_cannot_be_silently_omitted_from_finalization() {
    for filename in ["corrupt.jsonl", "corrupt.json"] {
        let root = temporary();
        let run = RunRecorder::create_in(&root, "test", "350z");
        run.capture_provenance(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets")),
            "checkout_assets",
            json!({"mode":"test"}),
        );
        fs::write(run.dir().join(filename), b"{truncated").unwrap();
        assert!(matches!(run.finish(AppExit::Success), AppExit::Error(_)));
        assert!(!run.dir().join("evidence-manifest.json").exists());
        assert_eq!(read(run.dir().join("recorder-health.json"))["failed"], true);
        fs::remove_dir_all(root).unwrap();
    }
}
