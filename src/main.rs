use car_game::{RuntimeConfig, build_app, diagnostics};
fn main() {
    if let Some(target) = diagnostics::diagnose_target() {
        diagnostics::print_diagnosis(target.as_deref());
        return;
    }
    let config = RuntimeConfig::from_args(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let run = std::sync::Arc::new(diagnostics::RunRecorder::create_in(
        &config.artifact_dir,
        &config.mode,
        &config.car,
    ));
    diagnostics::set_run_recorder(run.clone());
    diagnostics::set_log_file(run.dir().join("run.log"));
    diagnostics::install_panic_hook();
    let mut app = build_app(config, run.clone()).unwrap_or_else(|e| {
        run.fail("configuration", serde_json::json!({"error":e}));
        run.finish(bevy::app::AppExit::Error(
            std::num::NonZeroU8::new(2).expect("nonzero exit code"),
        ));
        eprintln!("{e}");
        std::process::exit(2)
    });
    let exit = app.run();
    // The production Last schedule flushes the live world before runner teardown.
    run.finish(exit.clone());
    if let bevy::app::AppExit::Error(code) = exit {
        std::process::exit(code.get().into());
    }
}
