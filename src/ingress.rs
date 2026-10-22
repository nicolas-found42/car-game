//! Observations at the device/window ingress, independent of renderer readiness.
use crate::diagnostics::RunHandle;
use bevy::{
    input::gamepad::{GamepadConnection, GamepadConnectionEvent},
    prelude::*,
    window::{WindowFocused, WindowResized},
};
use serde_json::json;

pub fn observe_gamepads(mut messages: MessageReader<GamepadConnectionEvent>, run: Res<RunHandle>) {
    for event in messages.read() {
        let (kind, device) = match &event.connection {
            GamepadConnection::Connected {
                name,
                vendor_id,
                product_id,
            } => (
                "gamepad.connected",
                json!({"name":name,"vendor_id":vendor_id,"product_id":product_id}),
            ),
            GamepadConnection::Disconnected => ("gamepad.disconnected", json!(null)),
        };
        run.event(kind, json!({"runtime_entity":event.gamepad.to_bits(),"controller_id":format!("{}:gamepad:{}",run.run_id(),event.gamepad.to_bits()),"assignment_policy":"legacy_composite","device":device}));
    }
}
pub fn observe_windows(
    mut focused: MessageReader<WindowFocused>,
    mut resized: MessageReader<WindowResized>,
    run: Res<RunHandle>,
) {
    for event in focused.read() {
        run.event(
            "window.focus",
            json!({"runtime_entity":event.window.to_bits(),"focused":event.focused}),
        );
    }
    for event in resized.read() {
        run.event("window.resize", json!({"runtime_entity":event.window.to_bits(),"width":event.width,"height":event.height}));
    }
}
