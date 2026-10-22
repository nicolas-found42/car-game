//! Native per-car driving and multi-angle visual checks, enabled only by CLI.
use crate::{
    diagnostics::{RunHandle, capture_metrics, sha256_file},
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
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

#[derive(Component)]
struct CaptureRequest {
    view: String,
    path: PathBuf,
}

#[derive(Resource)]
struct Smoke {
    run_dir: PathBuf,
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
    captures: BTreeMap<String, Value>,
    pending_capture: Option<(String, f64)>,
    current_car: String,
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
        self.captures.clear();
        self.pending_capture = None;
        self.current_car.clear();
    }
}

pub struct SmokePlugin {
    run_dir: PathBuf,
}

impl SmokePlugin {
    pub fn new(run_dir: PathBuf) -> Self {
        Self { run_dir }
    }
}

impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        let visual_only = std::env::args().any(|a| a == "--visual-check");
        let fleet_check = visual_only || std::env::args().any(|a| a == "--fleet-check");
        if fleet_check || std::env::args().any(|a| a == "--smoke") {
            std::fs::create_dir_all(self.run_dir.join("captures"))
                .expect("create smoke captures directory");
            app.insert_resource(Smoke {
                run_dir: self.run_dir.clone(),
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
                captures: BTreeMap::new(),
                pending_capture: None,
                current_car: String::new(),
            })
            .add_systems(
                Update,
                (scripted_drive, check_capture_timeout)
                    .chain()
                    .after(PresentationUpdate),
            );
        }
    }
}

fn validate_capture(
    capture: On<ScreenshotCaptured>,
    requests: Query<&CaptureRequest>,
    mut smoke: ResMut<Smoke>,
    run: Res<RunHandle>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(request) = requests.get(capture.entity) else {
        error!("FLEET: screenshot arrived without a capture request");
        return;
    };
    let metrics = capture_metrics(&capture.image);
    let value = metrics
        .as_ref()
        .map(|metrics| serde_json::to_value(metrics).unwrap_or(Value::Null))
        .unwrap_or(Value::Null);
    let path = request.path.display().to_string();
    match metrics {
        Some(metrics) if metrics.nonblank => {
            smoke.valid_captures += 1;
            smoke.captures.insert(request.view.clone(), value.clone());
            run.record_capture_metrics(&smoke.current_car, &request.view, &value);
            run.event(
                "screenshot.validated",
                json!({
                    "car": smoke.current_car,
                    "view": request.view,
                    "path": path,
                    "metrics": value,
                }),
            );
            smoke.pending_capture = None;
        }
        Some(metrics) => {
            run.fail(
                "blank_capture",
                json!({
                    "car": smoke.current_car,
                    "view": request.view,
                    "path": path,
                    "metrics": metrics,
                }),
            );
            smoke.pending_capture = None;
            exit.write(AppExit::error());
        }
        None => {
            run.fail(
                "unreadable_capture",
                json!({
                    "car": smoke.current_car,
                    "view": request.view,
                    "path": path,
                }),
            );
            smoke.pending_capture = None;
            exit.write(AppExit::error());
        }
    }
}

fn check_capture_timeout(
    time: Res<Time<Real>>,
    mut smoke: ResMut<Smoke>,
    run: Res<RunHandle>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some((view, requested_at)) = smoke.pending_capture.clone() else {
        return;
    };
    let age = time.elapsed_secs_f64() - requested_at;
    if age > 5.0 {
        run.fail(
            "capture_timeout",
            json!({
                "car": smoke.current_car,
                "view": view,
                "age_seconds": age,
                "stage": smoke.stage,
            }),
        );
        smoke.pending_capture = None;
        exit.write(AppExit::error());
    }
}

// Bevy system inputs: checked vehicle, timed script, screenshots and report output.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn scripted_drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut smoke: ResMut<Smoke>,
    mut input: ResMut<DriveInput>,
    spec: Res<CarSpec>,
    mut fleet: ResMut<Fleet>,
    mut inspection: ResMut<InspectionView>,
    render_target: Res<QaRenderTarget>,
    run: Res<RunHandle>,
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
    if smoke.current_car != spec.id {
        smoke.current_car = spec.id.clone();
        smoke.captures.clear();
        smoke.pending_capture = None;
        run.event(
            "car.start",
            json!({"car": spec.id, "game": spec.game_id, "path": spec.path}),
        );
    }
    let expected_names = &wheel_names[..spec.wheel_nodes.min(wheel_names.len())];
    let loaded_names: Vec<String> = names
        .iter()
        .filter(|(name, _)| expected_names.contains(&name.as_str()))
        .map(|(name, _)| name.to_string())
        .collect();
    let loaded = loaded_names.len();
    let missing_names: Vec<String> = expected_names
        .iter()
        .filter(|name| !loaded_names.iter().any(|loaded| loaded == *name))
        .map(|name| (*name).to_string())
        .collect();
    if smoke.started.is_none() {
        if loaded == spec.wheel_nodes {
            smoke.started = Some(time.elapsed_secs_f64());
            smoke.starting_reset_count = cars
                .single_mut()
                .map(|(_, car, _)| car.reset_count)
                .unwrap_or(0);
            info!(
                "FLEET: {} loaded; {} wheel/tire nodes ready: {:?}",
                spec.id, loaded, loaded_names
            );
            run.event(
                "asset.loaded",
                json!({
                    "car": spec.id,
                    "loaded_nodes": loaded,
                    "expected_nodes": spec.wheel_nodes,
                    "loaded_node_names": loaded_names,
                    "missing_node_names": missing_names,
                }),
            );
        } else if time.elapsed_secs_f64() - smoke.waiting_since > 60.0 {
            error!(
                "FLEET: {} model load timeout: {} nodes; missing {:?}",
                spec.id, loaded, missing_names
            );
            run.record_report(&json!(smoke.reports));
            run.fail(
                "model_load_timeout",
                json!({
                    "car": spec.id,
                    "loaded_nodes": loaded,
                    "expected_nodes": spec.wheel_nodes,
                    "loaded_node_names": loaded_names,
                    "missing_node_names": missing_names,
                }),
            );
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
            let path = smoke
                .run_dir
                .join("captures")
                .join(format!("{}-{view}.png", spec.id));
            let requested_at = time.elapsed_secs_f64();
            smoke.pending_capture = Some((view.to_string(), requested_at));
            run.event(
                "screenshot.requested",
                json!({
                    "car": spec.id,
                    "view": view,
                    "path": path.display().to_string(),
                    "render_target": if render_target.0.is_some() { "offscreen" } else { "window" },
                }),
            );
            commands
                .spawn((
                    if let Some(target) = &render_target.0 {
                        Screenshot::image(target.clone())
                    } else {
                        Screenshot::primary_window()
                    },
                    CaptureRequest {
                        view: view.to_string(),
                        path: path.clone(),
                    },
                ))
                .observe(save_to_disk(path))
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
            let capture_metrics = serde_json::to_value(&smoke.captures).unwrap_or(Value::Null);
            let mut captures = serde_json::Map::new();
            for view in smoke.captures.keys() {
                let relative = format!("captures/{}-{view}.png", spec.id);
                let path = smoke.run_dir.join(&relative);
                captures.insert(
                    view.clone(),
                    json!({
                        "path": relative,
                        "sha256": sha256_file(&path),
                        "metrics": smoke.captures.get(view).cloned().unwrap_or(Value::Null),
                    }),
                );
            }
            let report = json!({
                "id": spec.id, "name": spec.name, "game": spec.game_id, "passed": passed,
                "visual_capture_only": smoke.visual_only,
                "nonblank_captures": smoke.valid_captures,
                "model_nodes": loaded, "expected_nodes": spec.wheel_nodes,
                "loaded_node_names": loaded_names, "missing_node_names": missing_names,
                "max_speed_kmh": smoke.max_speed, "reset_count": car.reset_count - smoke.starting_reset_count,
                "animated_nodes": smoke.animated_nodes.len(),
                "animated_node_names": smoke.animated_nodes.iter().cloned().collect::<Vec<_>>(),
                "grounded_after_reset": car.grounded, "turned": smoke.turned,
                "brake_start_kmh": before * 3.6, "brake_end_kmh": after * 3.6,
                "average_fps": 1.0 / average, "p95_frame_ms": p95 * 1000.0,
                "frame_times_ms": smoke.frames.iter().map(|seconds| seconds * 1000.0).collect::<Vec<_>>(),
                "frames": samples.len(), "resolution": [1920,1080],
                "render_target": if render_target.0.is_some() { "offscreen" } else { "window" },
                "capture_metrics": capture_metrics,
                "captures": captures,
                "asset_path": format!("assets/{}", spec.path),
                "asset_sha256": sha256_file(&std::path::Path::new("assets").join(&spec.path)),
                "run_id": run.run_id(),
            });
            run.record_car_report(&spec.id, &report);
            info!("FLEET REPORT {}", report);
            smoke.reports.push(report.clone());
            run.record_report(&json!(smoke.reports));
            if !passed {
                error!("FLEET: {} driving/recovery validation failed", spec.id);
                run.fail("driving_validation_failed", report);
                exit.write(AppExit::error());
            } else if smoke.fleet_check && fleet.selected + 1 < fleet.cars.len() {
                // All static comparisons start on the same road under the same light.
                *pose = spawn_pose();
                fleet.pending = Some(fleet.selected + 1);
                run.event(
                    "car.next",
                    json!({"from": spec.id, "to": fleet.cars[fleet.selected + 1].id}),
                );
                smoke.next(time.elapsed_secs_f64());
            } else {
                run.event("run.complete", json!({"cars": smoke.reports.len()}));
                exit.write(AppExit::Success);
            }
        }
    }
}
