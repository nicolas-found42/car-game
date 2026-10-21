//! Native per-car driving and multi-angle visual checks, enabled only by CLI.
use crate::{
    presentation::{InspectionView, PresentationUpdate, QaRenderTarget},
    roster::{CarSpec, Fleet},
    vehicle::{DriveInput, Vehicle, spawn_pose},
};
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};
use bevy_rapier3d::prelude::Velocity;
use std::collections::HashSet;

#[derive(Resource)]
struct Smoke {
    fleet_check: bool,
    visual_only: bool,
    started: Option<f64>,
    waiting_since: f64,
    stage: u32,
    frames: Vec<f64>,
    max_speed: f32,
    braking_start: Option<f32>,
    braking_end: Option<f32>,
    turned: bool,
    reports: Vec<serde_json::Value>,
    valid_captures: usize,
    starting_reset_count: u32,
    animated_nodes: HashSet<String>,
}

impl Smoke {
    fn next(&mut self, now: f64) {
        self.started = None;
        self.waiting_since = now;
        self.stage = 0;
        self.frames.clear();
        self.max_speed = 0.0;
        self.braking_start = None;
        self.braking_end = None;
        self.turned = false;
        self.valid_captures = 0;
        self.animated_nodes.clear();
    }
}

pub struct SmokePlugin;
impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        let visual_only = std::env::args().any(|a| a == "--visual-check");
        let fleet_check = visual_only || std::env::args().any(|a| a == "--fleet-check");
        if fleet_check || std::env::args().any(|a| a == "--smoke") {
            std::fs::create_dir_all("artifacts/fleet").expect("create smoke directory");
            app.insert_resource(Smoke {
                fleet_check,
                visual_only,
                started: None,
                waiting_since: 0.0,
                stage: 0,
                frames: Vec::new(),
                max_speed: 0.0,
                braking_start: None,
                braking_end: None,
                turned: false,
                reports: Vec::new(),
                valid_captures: 0,
                starting_reset_count: 0,
                animated_nodes: HashSet::new(),
            })
            .add_systems(Update, scripted_drive.after(PresentationUpdate));
        }
    }
}

fn pixels_have_content(bytes: &[u8]) -> bool {
    let mut pixels = bytes.as_chunks::<4>().0.iter();
    let Some(first) = pixels.next() else {
        return false;
    };
    pixels.any(|pixel| pixel[..3] != first[..3])
}

fn validate_capture(
    capture: On<ScreenshotCaptured>,
    mut smoke: ResMut<Smoke>,
    mut exit: MessageWriter<AppExit>,
) {
    if capture
        .image
        .data
        .as_ref()
        .is_some_and(|bytes| pixels_have_content(bytes))
    {
        smoke.valid_captures += 1;
    } else {
        error!(
            "FLEET: blank capture; native visual verification requires an unlocked, rendering desktop"
        );
        exit.write(AppExit::error());
    }
}

// Bevy system inputs: checked vehicle, timed script, screenshots and report output.
#[allow(clippy::too_many_arguments)]
fn scripted_drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut smoke: ResMut<Smoke>,
    mut input: ResMut<DriveInput>,
    spec: Res<CarSpec>,
    mut fleet: ResMut<Fleet>,
    mut inspection: ResMut<InspectionView>,
    render_target: Res<QaRenderTarget>,
    names: Query<(&Name, &Transform), Without<Vehicle>>,
    mut cars: Query<(&mut Transform, &Vehicle, &Velocity)>,
    mut exit: MessageWriter<AppExit>,
) {
    let wheel_names = [
        "wheel_front_left",
        "wheel_front_right",
        "wheel_rear_left",
        "wheel_rear_right",
        "tire_front_left",
        "tire_front_right",
        "tire_rear_left",
        "tire_rear_right",
    ];
    let loaded = names
        .iter()
        .filter(|(n, _)| wheel_names.contains(&n.as_str()))
        .count();
    if smoke.started.is_none() {
        if loaded == spec.wheel_nodes {
            smoke.started = Some(time.elapsed_secs_f64());
            smoke.starting_reset_count = cars
                .single_mut()
                .map(|(_, car, _)| car.reset_count)
                .unwrap_or(0);
            info!(
                "FLEET: {} loaded; {} wheel/tire nodes ready",
                spec.id, loaded
            );
        } else if time.elapsed_secs_f64() - smoke.waiting_since > 60.0 {
            error!("FLEET: {} model load timeout: {} nodes", spec.id, loaded);
            exit.write(AppExit::error());
        }
        return;
    }
    let t = time.elapsed_secs_f64() - smoke.started.unwrap();
    if (5.0..13.0).contains(&t) {
        for (name, pose) in &names {
            if wheel_names.contains(&name.as_str()) && pose.rotation.x.abs() > 0.01 {
                smoke.animated_nodes.insert(name.to_string());
            }
        }
    }
    if (5.0..22.0).contains(&t) {
        smoke.frames.push(time.delta_secs_f64());
    }
    input.throttle = if (5.0..13.0).contains(&t) { 1.0 } else { 0.0 };
    input.steer = if (11.0..13.0).contains(&t) { 0.45 } else { 0.0 };
    input.brake = if (13.0..17.0).contains(&t) { 1.0 } else { 0.0 };
    input.handbrake = (12.0..12.5).contains(&t);
    if let Ok((mut pose, car, velocity)) = cars.single_mut() {
        let planar_speed = Vec2::new(velocity.linear.x, velocity.linear.z).length();
        if (13.0..17.0).contains(&t) && planar_speed < 1.0 {
            input.brake = 0.0;
        }
        smoke.max_speed = smoke.max_speed.max(car.speed.abs() * 3.6);
        if (11.0..14.0).contains(&t) && (pose.rotation * Vec3::NEG_Z).x > 0.05 {
            smoke.turned = true;
        }
        if t >= 13.0 && smoke.braking_start.is_none() {
            smoke.braking_start = Some(planar_speed);
        }
        if t >= 16.5 && smoke.braking_end.is_none() {
            smoke.braking_end = Some(planar_speed);
        }
        let focus = pose.translation + Vec3::Y * 0.25;
        if (1.5..2.5).contains(&t) {
            inspection.0 = Some(
                Transform::from_translation(pose.translation + Vec3::new(4.0, 2.0, -6.0))
                    .looking_at(focus, Vec3::Y),
            );
        } else if (2.5..3.5).contains(&t) {
            inspection.0 = Some(
                Transform::from_translation(pose.translation + Vec3::new(6.3, 1.3, 0.0))
                    .looking_at(focus, Vec3::Y),
            );
        } else if (3.5..4.5).contains(&t) {
            inspection.0 = Some(
                Transform::from_translation(pose.translation + Vec3::new(4.0, 2.0, 6.0))
                    .looking_at(focus, Vec3::Y),
            );
        } else {
            inspection.0 = None;
        }
        let capture = match smoke.stage {
            0 if t > 2.0 => Some("front"),
            1 if t > 3.0 => Some("side"),
            2 if t > 4.0 => Some("rear"),
            3 if t > 12.0 => Some("driving"),
            _ => None,
        };
        if let Some(view) = capture {
            commands
                .spawn(if let Some(target) = &render_target.0 {
                    Screenshot::image(target.clone())
                } else {
                    Screenshot::primary_window()
                })
                .observe(save_to_disk(format!(
                    "artifacts/fleet/{}-{view}.png",
                    spec.id
                )))
                .observe(validate_capture);
            smoke.stage += 1;
        }
        if t > 18.0 && smoke.stage == 4 {
            input.reset = true;
            smoke.stage = 5;
        }
        if t > if smoke.visual_only { 5.0 } else { 22.0 } {
            let mut samples = smoke.frames.clone();
            samples.sort_by(f64::total_cmp);
            let average = samples.iter().sum::<f64>() / samples.len().max(1) as f64;
            let p95 = samples
                .get((samples.len() as f64 * 0.95) as usize)
                .copied()
                .unwrap_or(0.0);
            let before = smoke.braking_start.unwrap_or(0.0);
            let after = smoke.braking_end.unwrap_or(f32::INFINITY);
            let motion_passed = if smoke.visual_only {
                loaded == spec.wheel_nodes && car.grounded == 4
            } else {
                smoke.max_speed > 30.0
                    && car.reset_count > smoke.starting_reset_count
                    && smoke.animated_nodes.len() == spec.wheel_nodes
                    && pose.translation.is_finite()
                    && smoke.turned
                    && before > 5.0
                    && after < before * 0.5
                    && car.grounded == 4
            };
            let passed =
                motion_passed && smoke.valid_captures == if smoke.visual_only { 3 } else { 4 };
            let report = serde_json::json!({
                "id": spec.id, "name": spec.name, "game": spec.game_id, "passed": passed,
                "visual_capture_only": smoke.visual_only,
                "nonblank_captures": smoke.valid_captures,
                "model_nodes": loaded, "expected_nodes": spec.wheel_nodes,
                "max_speed_kmh": smoke.max_speed, "reset_count": car.reset_count - smoke.starting_reset_count,
                "animated_nodes": smoke.animated_nodes.len(),
                "grounded_after_reset": car.grounded, "turned": smoke.turned,
                "brake_start_kmh": before * 3.6, "brake_end_kmh": after * 3.6,
                "average_fps": 1.0 / average, "p95_frame_ms": p95 * 1000.0,
                "frames": samples.len(), "resolution": [1920,1080],
                "render_target": if render_target.0.is_some() { "offscreen" } else { "window" },
            });
            std::fs::write(
                format!("artifacts/fleet/{}-report.json", spec.id),
                serde_json::to_string_pretty(&report).unwrap(),
            )
            .expect("write car report");
            info!("FLEET REPORT {}", report);
            smoke.reports.push(report);
            std::fs::write(
                if smoke.visual_only {
                    "artifacts/fleet-visual-capture.json"
                } else {
                    "artifacts/fleet-report.json"
                },
                serde_json::to_string_pretty(&smoke.reports).unwrap(),
            )
            .expect("write fleet report");
            if !passed {
                error!("FLEET: {} driving/recovery validation failed", spec.id);
                exit.write(AppExit::error());
            } else if smoke.fleet_check && fleet.selected + 1 < fleet.cars.len() {
                // All static comparisons start on the same road under the same light.
                *pose = spawn_pose();
                fleet.pending = Some(fleet.selected + 1);
                smoke.next(time.elapsed_secs_f64());
            } else {
                exit.write(AppExit::Success);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::pixels_have_content;

    #[test]
    fn blank_captures_cannot_satisfy_visual_verification() {
        assert!(!pixels_have_content(&[]));
        assert!(!pixels_have_content(&[0, 0, 0, 255, 0, 0, 0, 255]));
        assert!(!pixels_have_content(&[255, 0, 0, 255, 255, 0, 0, 255]));
        assert!(pixels_have_content(&[20, 30, 40, 255, 80, 90, 100, 255]));
    }
}
