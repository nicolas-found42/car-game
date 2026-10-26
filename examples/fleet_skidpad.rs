//! Quasi-steady dry skidpad sweep through production VehiclePlugin forces.
use bevy::{prelude::*, time::TimeUpdateStrategy};
use bevy_rapier3d::prelude::*;
use car_game::{
    roster::{CarSpec, Fleet},
    vehicle::{DriveInput, STEP, Vehicle, VehiclePlugin, profile},
};
use serde::Serialize;
use std::time::Duration;

fn fixture(spec: CarSpec) -> App {
    let mut app = App::new();
    app.insert_resource(spec)
        .add_plugins((MinimalPlugins, TransformPlugin, VehiclePlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            STEP as f64,
        )));
    app.world_mut().spawn((
        RigidBody::Fixed,
        Collider::cuboid(4000., 0.2, 4000.),
        Transform::from_xyz(0., -0.2, 0.),
    ));
    for (_, schedule) in app
        .world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .iter_mut()
    {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    }
    for _ in 0..360 {
        app.update();
    }
    app
}
#[derive(Serialize)]
pub(crate) struct Sweep {
    car: String,
    diameter_ft: f32,
    pub(crate) peak_sustained_g: f32,
    speed_at_peak_mps: f32,
    radial_error_at_peak_m: f32,
    peak_tilt_deg: f32,
    accepted_windows: usize,
}
fn command(
    spec: &CarSpec,
    pose: &Transform,
    velocity: &Velocity,
    radius: f32,
    target: f32,
) -> DriveInput {
    let p = profile::for_car(&spec.id);
    let speed = Vec2::new(velocity.linear.x, velocity.linear.z).length();
    let lookahead = 4. + speed * 0.35;
    let theta = (-pose.translation.z).atan2(pose.translation.x + radius) + lookahead / radius;
    let point = Vec3::new(
        radius * (theta.cos() - 1.),
        pose.translation.y,
        -radius * theta.sin(),
    );
    let direction = (point - pose.translation).normalize_or_zero();
    let forward = pose.rotation * Vec3::NEG_Z;
    let angle = forward.cross(direction).y.atan2(forward.dot(direction));
    let wheel_angle = (2. * spec.wheelbase() * angle.sin() / lookahead).atan();
    DriveInput {
        throttle: ((target - speed) * 0.8).clamp(0., 1.),
        brake: ((speed - target) * 0.4).clamp(0., 1.),
        steer: (-wheel_angle / (p.max_steer_rad / (1. + speed * p.steer_speed_falloff)))
            .clamp(-1., 1.),
        ..default()
    }
}
pub(crate) fn sweep(spec: CarSpec, diameter_ft: f32) -> Sweep {
    let radius = diameter_ft * 0.3048 / 2.;
    let mut app = fixture(spec.clone());
    let mut result = Sweep {
        car: spec.id.clone(),
        diameter_ft,
        peak_sustained_g: 0.,
        speed_at_peak_mps: 0.,
        radial_error_at_peak_m: 0.,
        peak_tilt_deg: 0.,
        accepted_windows: 0,
    };
    let (mut accel, mut speed_sum, mut error_sum, mut accepted) = (0., 0., 0., true);
    for tick in 0..16800 {
        let target_g = 0.25 + (tick as f32 / 120. - 10.).max(0.) * 0.008;
        let target = (target_g * 9.80665 * radius).sqrt();
        let mut q = app.world_mut().query::<(&Transform, &Velocity, &Vehicle)>();
        let (pose, velocity, car) = q.single(app.world()).unwrap();
        let speed = Vec2::new(velocity.linear.x, velocity.linear.z).length();
        let error =
            (Vec2::new(pose.translation.x + radius, pose.translation.z).length() - radius).abs();
        let tilt = (pose.rotation * Vec3::Y)
            .y
            .clamp(-1., 1.)
            .acos()
            .to_degrees();
        let controls = command(&spec, pose, velocity, radius, target);
        accel += speed * velocity.angular.y.abs() / 9.80665;
        speed_sum += speed;
        error_sum += error;
        accepted &= error < 1. && (speed - target).abs() < 0.5 && car.grounded == 4;
        result.peak_tilt_deg = result.peak_tilt_deg.max(tilt);
        if (tick + 1) % 120 == 0 {
            if tick > 1200 && accepted {
                result.accepted_windows += 1;
                if accel / 120. > result.peak_sustained_g {
                    result.peak_sustained_g = accel / 120.;
                    result.speed_at_peak_mps = speed_sum / 120.;
                    result.radial_error_at_peak_m = error_sum / 120.;
                }
            }
            (accel, speed_sum, error_sum, accepted) = (0., 0., 0., true);
        }
        *app.world_mut().resource_mut::<DriveInput>() = controls;
        app.update();
    }
    result
}
fn main() {
    let filter = std::env::var("FLEET_IDS").unwrap_or_default();
    let mut rows = Vec::new();
    for spec in Fleet::default().cars {
        if !filter.is_empty() && !filter.split(',').any(|id| id == spec.id) {
            continue;
        }
        for diameter in [200., 300.] {
            rows.push(sweep(spec.clone(), diameter));
        }
    }
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
