use super::case::Case;
use bevy::{prelude::*, time::TimeUpdateStrategy};
use bevy_rapier3d::prelude::*;
use car_game::{
    roster::{CarSpec, Fleet},
    vehicle::{STEP, Vehicle, VehiclePlugin},
};
use std::time::Duration;

pub fn flat(spec: CarSpec, case: &Case) -> App {
    let mut app = App::new();
    // These fixtures have no scene hierarchy worth asynchronous dirty-tree tracking.
    app.insert_resource(bevy::transform::systems::StaticTransformOptimizations::Disabled);
    app.insert_resource(Fleet {
        cars: vec![spec.clone()],
        selected: 0,
        pending: None,
    })
    .insert_resource(spec)
    .add_plugins((
        MinimalPlugins.set(task_pool()),
        TransformPlugin,
        VehiclePlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        STEP as f64,
    )));
    app.world_mut().spawn((
        RigidBody::Fixed,
        Collider::cuboid(4000., 0.2, 4000.),
        Transform::from_xyz(0., case.base_height_m - 0.2, 0.)
            .with_rotation(Quat::from_rotation_x(case.slope_deg.to_radians())),
    ));
    if case.family == "bumps" {
        for distance in [8., 16., 24., 32., 40.] {
            app.world_mut().spawn((
                RigidBody::Fixed,
                Collider::cuboid(100., case.bump_height_m / 2., case.bump_width_m / 2.),
                Transform::from_xyz(0., case.bump_height_m / 2., -distance),
            ));
        }
    }
    single_thread(&mut app);
    app.update();
    let mut q = app.world_mut().query::<(&mut Transform, &mut Vehicle)>();
    let (mut pose, mut car) = q.single_mut(app.world_mut()).unwrap();
    *pose = Transform::from_xyz(0., case.base_height_m + 0.62, 0.).with_rotation(case.rotation());
    car.previous = *pose;
    for _ in 0..360 {
        app.update();
    }
    let mut q = app.world_mut().query::<(&Transform, &mut Velocity)>();
    let (pose, mut velocity) = q.single_mut(app.world_mut()).unwrap();
    velocity.linear = pose.rotation * Vec3::NEG_Z * case.entry_mps;
    velocity.angular = Vec3::ZERO;
    app
}

pub fn single_thread(app: &mut App) {
    for (_, schedule) in app
        .world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .iter_mut()
    {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    }
}

/// Each process already owns a worker; avoid parallel-query coordination for one car.
pub fn task_pool() -> bevy::app::TaskPoolPlugin {
    bevy::app::TaskPoolPlugin {
        task_pool_options: bevy::app::TaskPoolOptions::with_num_threads(1),
    }
}
