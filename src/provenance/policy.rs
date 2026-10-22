use serde_json::{Value, json};
use std::{collections::BTreeMap, env, path::PathBuf};

pub const ID: &str = "grid-portable-allowlist-v1";

pub fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-.,=:".contains(c))
}

pub fn arguments() -> (Vec<Value>, Vec<(String, PathBuf)>, usize) {
    let mut args = env::args().skip(1);
    let mut exported = Vec::new();
    let mut content = Vec::new();
    let mut omitted = 0;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke" | "--fleet-check" | "--visual-check" | "--offscreen"
            | "--render-disabled" => exported.push(json!(arg)),
            "--car" | "--seed" => {
                if let Some(value) = args.next().filter(|v| safe_token(v)) {
                    exported.extend([json!(arg), json!(value)]);
                } else {
                    omitted += 1;
                }
            }
            "--scenario" | "--replay-input" => {
                if let Some(path) = args.next() {
                    let role = if arg == "--scenario" {
                        "scenario"
                    } else {
                        "replay_input"
                    };
                    content.push((role.into(), path.into()));
                    exported.extend([json!(arg), json!(format!("inputs/{role}"))]);
                } else {
                    omitted += 1;
                }
            }
            "--artifact-dir" => {
                let _ = args.next();
                exported.push(json!({"output_destination": "redacted"}));
            }
            _ => omitted += 1,
        }
    }
    (exported, content, omitted)
}

pub fn environment() -> Value {
    let mute = env::var("CAR_GAME_MUTE").unwrap_or_else(|_| "0".into());
    let log = env::var("RUST_LOG")
        .unwrap_or_else(|_| "car_game=debug,bevy_asset=warn,wgpu=error,naga=warn".into());
    let mut values = BTreeMap::from([
        (
            "CAR_GAME_MUTE",
            json!(if ["0", "1"].contains(&mute.as_str()) {
                mute.as_str()
            } else {
                "redacted"
            }),
        ),
        (
            "RUST_LOG",
            json!(if safe_token(&log) {
                log.as_str()
            } else {
                "redacted"
            }),
        ),
    ]);
    for key in ["WGPU_BACKEND", "WGPU_POWER_PREF"] {
        if let Ok(value) = env::var(key) {
            values.insert(
                key,
                json!(if safe_token(&value) {
                    value
                } else {
                    "redacted".into()
                }),
            );
        }
    }
    json!(values)
}

/// Exported observations are derived metadata, never byte-exact original logs.
pub fn redact(value: &Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| {
                    let value = if [
                        "args",
                        "env",
                        "cwd",
                        "source_root",
                        "assets_root",
                        "dir",
                        "log_tail",
                        "backtrace",
                        "message",
                        "location",
                        "rustc",
                        "cargo",
                        "branch",
                    ]
                    .contains(&key.as_str())
                    {
                        json!("redacted_by_policy")
                    } else {
                        redact(value)
                    };
                    (key.clone(), value)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        Value::String(text)
            if text.starts_with('/')
                || text.contains(":\\")
                || text.contains("/Users/")
                || text.contains("/home/") =>
        {
            json!("redacted_by_policy")
        }
        _ => value.clone(),
    }
}

pub fn config(value: &Value) -> Value {
    let keys = [
        "mode",
        "visual_only",
        "car",
        "offscreen",
        "render_disabled",
        "seed",
        "fixed_hz",
        "log_filter",
        "max_ticks",
        "tick_rate",
        "capture_required",
        "scene",
        "scenario",
        "context",
        "replay_input",
    ];
    let mut result = serde_json::Map::new();
    if let Some(fields) = value.as_object() {
        for (key, value) in fields {
            if keys.contains(&key.as_str()) {
                let sanitized = if key == "replay_input" && !value.is_null() {
                    json!("inputs/replay_input")
                } else {
                    match value {
                        Value::String(text) if !safe_token(text) => json!("redacted_by_policy"),
                        _ => redact(value),
                    }
                };
                result.insert(key.clone(), sanitized);
            }
        }
    }
    Value::Object(result)
}
