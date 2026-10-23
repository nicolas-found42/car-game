use car_game::{RuntimeConfig, build_app, diagnostics};
fn main() {
    if let Some(target) = diagnostics::diagnose_target() {
        diagnostics::print_diagnosis(target.as_deref());
        return;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let recorder_config = diagnostics::RecorderConfig::from_args(&args).unwrap_or_else(|error| {
        eprintln!("recorder_error: {error}");
        std::process::exit(2)
    });
    let config = RuntimeConfig::from_args(args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let run = std::sync::Arc::new(
        diagnostics::RunRecorder::try_create_with_config(
            &config.artifact_dir,
            &config.mode,
            &config.car,
            recorder_config,
        )
        .unwrap_or_else(|error| {
            eprintln!("recorder_error: {error}");
            std::process::exit(2)
        }),
    );
    diagnostics::set_run_recorder(run.clone());
    diagnostics::install_panic_hook();
    if let Err(error) = diagnostics::install_termination_handler(&run) {
        eprintln!("recorder_error: {error}");
        run.fail("signal_handler_unavailable", serde_json::json!({}));
        run.finish(bevy::app::AppExit::error());
        std::process::exit(2);
    }
    let exit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match build_app(config, run.clone()) {
            Ok(mut app) => app.run(),
            Err(error) => {
                run.fail("configuration", serde_json::json!({"error":error}));
                bevy::app::AppExit::error()
            }
        }
    }))
    .unwrap_or_else(|_| {
        run.fail(
            "panic",
            serde_json::json!({"context":"panic.json","backtrace_availability":"unavailable"}),
        );
        bevy::app::AppExit::error()
    });
    // The production Last schedule flushes the live world before runner teardown.
    let exit = run.finish(exit);
    if let bevy::app::AppExit::Error(code) = exit {
        std::process::exit(code.get().into());
    }
}
