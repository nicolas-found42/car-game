//! Flat-road production physics measurements. No fitted target values are injected.
use bevy::{prelude::*, time::TimeUpdateStrategy};
use bevy_rapier3d::prelude::*;
use car_game::{
    roster::{CarSpec, Fleet},
    vehicle::{DriveInput, STEP, Vehicle, VehiclePlugin},
};
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

fn run(case: &str, urban_surfaces: bool) -> serde_json::Value {
    let spec=Fleet::default().cars.into_iter().find(|s|s.id=="redline-gt500").unwrap();
    let mut app=fixture(spec);
    if urban_surfaces {
        app.world_mut().spawn((RigidBody::Fixed,Collider::cuboid(44.,0.09,44.),Transform::from_xyz(-280.,0.09,392.)));
        app.world_mut().spawn((RigidBody::Fixed,Collider::cuboid(480.,1.,0.5),Transform::from_xyz(0.,1.,480.)));
    }
    let root=std::path::Path::new("artifacts/fleet40-calibrated-v2-20261024/scenarios");
    let scenario:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join(format!("{case}-redline-gt500.json"))).unwrap()).unwrap();
    let mut q=app.world_mut().query::<(&mut Transform,&mut Velocity,&mut Vehicle)>();
    let (mut pose,mut velocity,mut car)=q.single_mut(app.world_mut()).unwrap();
    *pose=Transform::from_xyz(-300.,0.62,458.).with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2));
    *velocity=Velocity::zero(); *car=Vehicle::default();
    let mut samples=Vec::new();
    for tick in 0..=1080 {
        for c in scenario["commands"].as_array().unwrap().iter().filter(|c|c["tick"].as_u64()==Some(tick)) {
            let c=&c["command"];
            *app.world_mut().resource_mut::<DriveInput>()=DriveInput {throttle:c["throttle"].as_f64().unwrap_or(0.) as f32,brake:c["brake"].as_f64().unwrap_or(0.) as f32,steer:c["steer"].as_f64().unwrap_or(0.) as f32,..default()};
        }
        app.update();
        let mut q=app.world_mut().query::<(&Transform,&Velocity,&Vehicle)>();
        let (p,v,c)=q.single(app.world()).unwrap();
        samples.push(serde_json::json!({"tick":tick,"position":p.translation.to_array(),"grounded":c.grounded,"speed_mps":Vec2::new(v.linear.x,v.linear.z).length(),"tilt_deg":(p.rotation*Vec3::Y).y.clamp(-1.,1.).acos().to_degrees()}));
    }
    let moving:Vec<_>=samples.iter().filter(|r|r["tick"].as_u64().unwrap()>=240).collect();
    let support=100.*moving.iter().filter(|r|r["grounded"]==4).count() as f64/moving.len() as f64;
    serde_json::json!({"case":case,"urban_surfaces":urban_surfaces,"four_contact_pct":support,"peak_tilt_deg":moving.iter().map(|r|r["tilt_deg"].as_f64().unwrap()).fold(0.,f64::max),"samples":samples})
}
fn main() {
    let frozen:serde_json::Value=serde_json::from_str(include_str!("../assets/cars/physics.json")).unwrap();
    for p in frozen["cars"].as_array().unwrap() {assert_eq!(car_game::vehicle::profile::report(p["id"].as_str().unwrap()),*p);}
    let mut rows=Vec::new();
    for case in ["trail-brake-left","trail-brake-right"] {for urban in [false,true] {rows.push(run(case,urban));}}
    println!("{}",serde_json::to_string_pretty(&rows).unwrap());
}
