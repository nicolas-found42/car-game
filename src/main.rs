use bevy::prelude::*;
use bevy::window::{ExitCondition, PresentMode, WindowResolution};
use bevy::{app::ScheduleRunnerPlugin, winit::WinitPlugin};
use std::time::Duration;

mod city;
mod presentation;
mod roster;
mod smoke;
mod vehicle;

fn main() {
    let bundled_assets = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent()?.parent().map(|p| p.join("Resources/assets")))
        .filter(|p| p.is_dir());
    let fleet = roster::Fleet::from_args();
    let offscreen = std::env::args().any(|a| a == "--offscreen");
    let mut plugins = DefaultPlugins
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
            file_path: bundled_assets
                .unwrap_or_else(|| {
                    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
                })
                .to_string_lossy()
                .into_owned(),
            ..default()
        });
    if offscreen {
        plugins = plugins.disable::<WinitPlugin>();
    }
    let mut app = App::new();
    app.insert_resource(fleet.cars[fleet.selected].clone())
        .insert_resource(fleet)
        .add_plugins(plugins)
        .add_plugins((
            vehicle::VehiclePlugin,
            city::CityPlugin,
            presentation::PresentationPlugin,
            smoke::SmokePlugin,
        ));
    if offscreen {
        app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 120.0,
        )));
    }
    app.run();
}
