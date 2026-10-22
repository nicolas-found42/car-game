use super::{RecorderError, RunHandle, RunRecorder};
use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

static TERMINATION_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn install_termination_handler(run: &RunRecorder) -> Result<(), RecorderError> {
    ctrlc::set_handler(|| {
        TERMINATION_REQUESTED.store(true, Ordering::Release);
        bevy::app::TerminalCtrlCHandlerPlugin::gracefully_exit();
    })
    .map_err(|_| RecorderError::new("signal_handler", "termination", "registration_failed"))?;
    run.signal_support();
    Ok(())
}
pub fn poll_lifecycle(world: &mut World) {
    let Some(run) = world.get_resource::<RunHandle>().cloned() else {
        return;
    };
    if TERMINATION_REQUESTED.load(Ordering::Acquire) {
        run.terminate();
        world.write_message(AppExit::Success);
    } else if run.recording_failed() {
        run.fail(
            "recorder_failure",
            serde_json::json!({"health":"recorder-health.json"}),
        );
        world.write_message(AppExit::error());
    }
}
