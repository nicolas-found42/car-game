use car_game::diagnostics::RecorderConfig;

#[test]
fn expanded_fleet_finalization_budget_is_explicit_and_bounded() {
    let default = RecorderConfig::default();
    assert_eq!(default.finalize_max_bytes, 512 * 1024 * 1024);
    let mut expanded = RecorderConfig {
        finalize_max_bytes: 8 * 1024 * 1024 * 1024,
        ..default
    };
    assert!(expanded.validate().is_ok());
    expanded.finalize_max_bytes += 1;
    assert!(expanded.validate().is_err());
}

#[test]
fn large_aggregate_report_survives_publication_and_provenance_export() {
    use bevy::app::AppExit;
    use car_game::diagnostics::RunRecorder;
    use serde_json::{Value, json};
    let root = std::env::temp_dir().join(format!("grid-large-report-{}", std::process::id()));
    let run = RunRecorder::create_in(&root, "test", "350z");
    let report = json!(
        (0..300)
            .map(|id| json!({"id": format!("probe-{id}"), "frame_times_ms": vec![10.25; 4800]}))
            .collect::<Vec<_>>()
    );
    let bytes = serde_json::to_vec_pretty(&report).unwrap().len();
    assert!(bytes > 2 * 1024 * 1024 && bytes < 32 * 1024 * 1024);
    run.record_report(&report);
    run.finish(AppExit::Success);
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(run.dir().join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["exit"]["code"], 0);
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(run.dir().join("report.json")).unwrap())
            .unwrap(),
        report
    );
    std::fs::remove_dir_all(root).unwrap();
}
