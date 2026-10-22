use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::window::{ExitCondition, PresentMode, WindowResolution};
use bevy::{app::ScheduleRunnerPlugin, winit::WinitPlugin};
use std::time::Duration;

mod city;
mod diagnostics;
mod presentation;
mod provenance;
mod roster;
mod smoke;
mod vehicle;

fn main() {
    if let Some(target) = diagnostics::diagnose_target() {
        diagnostics::print_diagnosis(target.as_deref());
        return;
    }

    let bundled_assets = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent()?.parent().map(|p| p.join("Resources/assets")))
        .filter(|p| p.is_dir());
    let root_kind = if bundled_assets.is_some() {
        "bundle_resources"
    } else {
        "checkout_assets"
    };
    let resolved_assets = bundled_assets.unwrap_or_else(|| {
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
    });
    let fleet = roster::Fleet::from_args();
    let car_id = fleet.cars[fleet.selected].id.clone();
    let mode = run_mode();
    let run = std::sync::Arc::new(diagnostics::RunRecorder::create(mode, &car_id));
    run.capture_provenance(
        &resolved_assets,
        root_kind,
        serde_json::json!({"mode": mode, "car": car_id, "fixed_hz": 120}),
    );
    diagnostics::set_run_recorder(run.clone());
    let run_dir = run.dir().to_path_buf();
    diagnostics::set_log_file(run_dir.join("run.log"));
    diagnostics::install_panic_hook();
    let replay = diagnostics::replay_from_args();

    let offscreen = std::env::args().any(|a| a == "--offscreen");
    let filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "car_game=debug,bevy_asset=warn,wgpu=error,naga=warn".to_string());
    let mut plugins = DefaultPlugins
        .set(LogPlugin {
            filter,
            level: Level::INFO,
            custom_layer: diagnostics::file_log_layer,
            ..default()
        })
        .set(WindowPlugin {
            primary_window: if offscreen {
                None
            } else {
                Some(Window {
                    title: "GRID — Free Drive".into(),
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                })
            },
            exit_condition: if offscreen {
                ExitCondition::DontExit
            } else {
                ExitCondition::OnAllClosed
            },
            ..default()
        })
        .set(AssetPlugin {
            file_path: resolved_assets.to_string_lossy().into_owned(),
            ..default()
        });
    if offscreen {
        plugins = plugins.disable::<WinitPlugin>();
    }
    let mut app = App::new();
    app.insert_resource(fleet.cars[fleet.selected].clone())
        .insert_resource(fleet)
        .insert_resource(diagnostics::RunHandle(run.clone()))
        .insert_resource(diagnostics::PhysicsTrace::new(&run_dir))
        .add_plugins(plugins)
        .add_plugins(diagnostics::DiagnosticsPlugin)
        .add_plugins((
            vehicle::VehiclePlugin,
            city::CityPlugin,
            presentation::PresentationPlugin,
            smoke::SmokePlugin::new(run_dir),
        ));
    if let Some(replay) = replay {
        app.insert_resource(replay);
    }
    if offscreen {
        app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 120.0,
        )));
    }
    let exit = app.run();
    if let Some(mut trace) = app
        .world_mut()
        .get_resource_mut::<diagnostics::PhysicsTrace>()
    {
        trace.flush();
    }
    if let Some(run) = diagnostics::run_recorder() {
        run.finish(exit);
    }
}

fn run_mode() -> &'static str {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--fleet-check") {
        "fleet"
    } else if args.iter().any(|arg| arg == "--visual-check") {
        "visual"
    } else if args.iter().any(|arg| arg == "--smoke") {
        "smoke"
    } else if args.iter().any(|arg| arg == "--replay-input") {
        "replay"
    } else {
        "interactive"
    }
}
