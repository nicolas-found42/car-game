//! GT500 acceleration checkpoints through the unchanged production VehiclePlugin.
//! Quarter-mile speed is instantaneous at the crossing, not a timing-trap average.
use bevy::{prelude::*, time::TimeUpdateStrategy};
use bevy_rapier3d::prelude::*;
use car_game::{
    roster::Fleet,
    vehicle::{DriveInput, STEP, Vehicle, VehiclePlugin},
};
use serde_json::json;
fn main() {
    let spec = Fleet::default()
        .cars
        .into_iter()
        .find(|c| c.id == "redline-gt500")
        .unwrap();
    let mut app = App::new();
    app.insert_resource(spec)
        .add_plugins((MinimalPlugins, TransformPlugin, VehiclePlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(STEP as f64),
        ));
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
    let start_z = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Transform, With<Vehicle>>();
        q.single(app.world()).unwrap().translation.z
    };
    app.world_mut().resource_mut::<DriveInput>().throttle = 1.;
    let mut checkpoints = Vec::new();
    let mut trace = Vec::new();
    let mut next_mph = 10.;
    let mut previous_gear = 1;
    let mut shifts = Vec::new();
    let mut quarter = None;
    for tick in 1..=18000 {
        app.update();
        let mut q = app.world_mut().query::<(&Transform, &Velocity, &Vehicle)>();
        let (p, v, c) = q.single(app.world()).unwrap();
        let speed = Vec2::new(v.linear.x, v.linear.z).length();
        let seconds = tick as f32 * STEP;
        assert_eq!(c.grounded, 4);
        if speed >= next_mph * 0.44704 && next_mph <= 150. {
            checkpoints.push(json!({"mph":next_mph,"seconds":seconds,"rpm":c.rpm,"gear":c.gear}));
            next_mph += 10.;
        }
        if c.gear != previous_gear {
            shifts.push(
                json!({"seconds":seconds,"mph":speed/0.44704,"from":previous_gear,"to":c.gear}),
            );
            previous_gear = c.gear;
        }
        if quarter.is_none() && start_z - p.translation.z >= 402.336 {
            quarter = Some(json!({"seconds":seconds,"mph":speed/0.44704}));
        }
        if tick % 12 == 0 {
            trace.push(json!({"seconds":seconds,"speed_mps":speed,"gear":c.gear,"rpm":c.rpm,"demand_n":c.drive_demands.iter().sum::<f32>(),"applied_n":c.longitudinal_forces.iter().sum::<f32>()}));
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"parameters":car_game::vehicle::profile::report("redline-gt500"),"checkpoints":checkpoints,"shifts":shifts,"quarter_mile":quarter,"trace":trace})).unwrap());
}
