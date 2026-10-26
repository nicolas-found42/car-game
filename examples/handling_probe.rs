//! Geometry ablation fixture (same car profile retained across its variants): production VehiclePlugin on a flat road, no tuning overrides.
use bevy::{prelude::*, time::TimeUpdateStrategy};
use bevy_rapier3d::prelude::*;
use car_game::{
    roster::{CarSpec, Fleet},
    vehicle::{DriveInput, STEP, Vehicle, VehiclePlugin},
};
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct ResultRow {
    car: String,
    variant: String,
    entry_mps: f32,
    braking_m: f32,
    braking_s: f32,
    fast_tilt_deg: f32,
}
fn fixture(spec: CarSpec) -> App {
    let mut app = App::new();
    app.insert_resource(spec)
        .add_plugins((MinimalPlugins, TransformPlugin, VehiclePlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            STEP as f64,
        )));
    app.world_mut().spawn((
        RigidBody::Fixed,
        Collider::cuboid(1000., 0.2, 1000.),
        Transform::from_xyz(0., -0.2, 0.),
    ));
    app
}
fn observe(app: &mut App) -> (Vec3, f32, f32) {
    let mut query = app
        .world_mut()
        .query_filtered::<(&Transform, &Velocity), With<Vehicle>>();
    let (pose, velocity) = query.single(app.world()).expect("dynamic chassis");
    (
        pose.translation,
        Vec2::new(velocity.linear.x, velocity.linear.z).length(),
        (pose.rotation * Vec3::Y)
            .y
            .clamp(-1., 1.)
            .acos()
            .to_degrees(),
    )
}
fn run(spec: CarSpec, variant: &str) -> ResultRow {
    let mut app = fixture(spec.clone());
    for _ in 0..240 {
        app.update();
    }
    app.world_mut().resource_mut::<DriveInput>().throttle = 1.;
    for _ in 0..720 {
        app.update();
    }
    let (mut previous, entry, _) = observe(&mut app);
    *app.world_mut().resource_mut::<DriveInput>() = DriveInput {
        brake: 1.,
        ..default()
    };
    let mut distance = 0.;
    let mut time = f32::NAN;
    for tick in 1..=480 {
        app.update();
        let (position, speed, _) = observe(&mut app);
        distance += Vec2::new(position.x - previous.x, position.z - previous.z).length();
        previous = position;
        if speed <= 1. {
            time = tick as f32 * STEP;
            break;
        }
    }
    let mut turn = fixture(spec.clone());
    for _ in 0..240 {
        turn.update();
    }
    turn.world_mut().resource_mut::<DriveInput>().throttle = 1.;
    for _ in 0..720 {
        turn.update();
    }
    turn.world_mut().resource_mut::<DriveInput>().steer = 0.5;
    let mut tilt: f32 = 0.;
    for _ in 0..180 {
        turn.update();
        tilt = tilt.max(observe(&mut turn).2);
    }
    ResultRow {
        car: spec.id,
        variant: variant.into(),
        entry_mps: entry,
        braking_m: distance,
        braking_s: time,
        fast_tilt_deg: tilt,
    }
}
fn main() {
    let fleet = Fleet::default();
    let baseline = fleet.cars[0].clone();
    let original = fleet
        .cars
        .iter()
        .find(|car| car.id == "redline-911")
        .unwrap()
        .clone();
    let mut results = vec![
        run(baseline.clone(), "stock"),
        run(original.clone(), "stock"),
    ];
    if std::env::args().any(|arg| arg == "--variants") {
        let mut wheelbase = original.clone();
        for wheel in &mut wheelbase.wheels {
            wheel[2] *= baseline.wheelbase() / original.wheelbase();
        }
        results.push(run(wheelbase, "baseline-wheelbase"));
        let mut track = original.clone();
        for (wheel, reference) in track.wheels.iter_mut().zip(baseline.wheels) {
            wheel[0] = reference[0];
        }
        results.push(run(track, "baseline-track"));
        let mut radii = original.clone();
        radii.radii = baseline.radii;
        results.push(run(radii, "baseline-radii"));
        let mut body = original;
        body.half_width = baseline.half_width;
        body.half_length = baseline.half_length;
        body.body_center_z = baseline.body_center_z;
        body.roof_height = baseline.roof_height;
        results.push(run(body, "baseline-body"));
    }
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
}
