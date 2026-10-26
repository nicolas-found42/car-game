//! Flat-road production physics measurements. No fitted target values are injected.
use bevy::{prelude::*, time::TimeUpdateStrategy};
use bevy_rapier3d::prelude::*;
use car_game::{
    roster::{CarSpec, Fleet},
    vehicle::{DriveInput, STEP, Vehicle, VehiclePlugin},
};
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct Measurement {
    car: String,
    zero_60_mph_s: Option<f32>,
    zero_100_kmh_s: Option<f32>,
    zero_100_mph_s: Option<f32>,
    zero_200_kmh_s: Option<f32>,
    zero_125_mph_s: Option<f32>,
    top_speed_kmh: f32,
    top_speed_last10_gain_mps: f32,
    top_speed_four_contact_pct: f32,
    brake_60_mph_0_m: Option<f32>,
    brake_70_mph_0_m: Option<f32>,
    brake_100_0_m: Option<f32>,
    supported_fraction: f32,
    static_front_fraction: f32,
    static_total_load_n: f32,
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
        Collider::cuboid(100000., 0.2, 100000.),
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
fn state(app: &mut App) -> (Vec3, f32, usize) {
    let mut q = app.world_mut().query::<(&Transform, &Velocity, &Vehicle)>();
    let (p, v, c) = q.single(app.world()).unwrap();
    (
        p.translation,
        Vec2::new(v.linear.x, v.linear.z).length(),
        c.grounded,
    )
}
fn measure(spec: CarSpec) -> Measurement {
    let mut app = fixture(spec.clone());
    let mut q = app.world_mut().query::<(&Transform, &Vehicle)>();
    let (pose, car) = q.single(app.world()).unwrap();
    let profile = car_game::vehicle::profile::for_car(&spec.id);
    let load: [f32; 4] =
        std::array::from_fn(|i| (0.44 - car.wheel_lengths[i]) * profile.spring(i / 2));
    let _ = pose;
    let mut row = Measurement {
        car: spec.id.clone(),
        zero_60_mph_s: None,
        zero_100_kmh_s: None,
        zero_100_mph_s: None,
        zero_200_kmh_s: None,
        zero_125_mph_s: None,
        top_speed_kmh: 0.,
        top_speed_last10_gain_mps: 0.,
        top_speed_four_contact_pct: 0.,
        brake_60_mph_0_m: None,
        brake_70_mph_0_m: None,
        brake_100_0_m: None,
        supported_fraction: 0.,
        static_front_fraction: (load[0] + load[1]) / load.iter().sum::<f32>(),
        static_total_load_n: load.iter().sum(),
    };
    app.world_mut().resource_mut::<DriveInput>().throttle = 1.;
    let mut supported = 0;
    for tick in 1..=2400 {
        app.update();
        let (_, speed, n) = state(&mut app);
        supported += usize::from(n == 4);
        if speed >= 200. / 3.6 && row.zero_200_kmh_s.is_none() {row.zero_200_kmh_s=Some(tick as f32 * STEP);}
        if speed >= 125. * 0.44704 && row.zero_125_mph_s.is_none() {row.zero_125_mph_s=Some(tick as f32 * STEP);}
        if speed >= 26.8224 && row.zero_60_mph_s.is_none() {
            row.zero_60_mph_s = Some(tick as f32 * STEP);
        }
        if speed >= 44.704 && row.zero_100_mph_s.is_none() {
            row.zero_100_mph_s = Some(tick as f32 * STEP);
        }
        if speed >= 100. / 3.6 && row.zero_100_kmh_s.is_none() {
            row.zero_100_kmh_s = Some(tick as f32 * STEP);
        }
    }
    let mut previous_speed=0.; let mut contacts=0;
    for tick in 0..14400 {
        app.update(); let (_, speed, grounded)=state(&mut app);
        if tick==13200 {previous_speed=speed;}
        if tick==14399 {row.top_speed_last10_gain_mps=speed-previous_speed;}
        contacts+=usize::from(grounded==4);
        row.top_speed_kmh=row.top_speed_kmh.max(speed*3.6);
    }
    row.top_speed_four_contact_pct=100.*contacts as f32/14400.;
    row.supported_fraction = supported as f32 / 2400.;
    row.brake_100_0_m = brake_distance(spec.clone(), 100. / 3.6);
    row.brake_60_mph_0_m = brake_distance(spec.clone(), 26.8224);
    row.brake_70_mph_0_m = brake_distance(spec, 31.2928);
    row
}
fn brake_distance(spec: CarSpec, entry: f32) -> Option<f32> {
    let mut brake = fixture(spec);
    let mut q = brake
        .world_mut()
        .query::<(&mut Transform, &mut Velocity, &mut Vehicle)>();
    let (p, mut v, mut car) = q.single_mut(brake.world_mut()).unwrap();
    let start = *p;
    v.linear = Vec3::new(0., 0., -entry);
    v.angular = Vec3::ZERO;
    car.gear = 3;
    *brake.world_mut().resource_mut::<DriveInput>() = DriveInput {
        brake: 1.,
        ..default()
    };
    let mut previous = start.translation;
    let mut distance = 0.;
    for _ in 0..1200 {
        brake.update();
        let (p, speed, _) = state(&mut brake);
        distance += Vec2::new(p.x - previous.x, p.z - previous.z).length();
        previous = p;
        if speed <= 0.1 {
            return Some(distance);
        }
    }
    None
}

fn main() {
    let frozen: serde_json::Value=serde_json::from_str(include_str!("../assets/cars/physics.json")).unwrap();
    for row in frozen["cars"].as_array().unwrap() {assert_eq!(car_game::vehicle::profile::report(row["id"].as_str().unwrap()),*row,"Frozen profile/library mismatch");}
    let rows: Vec<_> = Fleet::default().cars.into_iter().map(measure).collect();
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
