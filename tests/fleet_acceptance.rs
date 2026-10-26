//! Native acceptance needs a common obstacle-free maneuver envelope.
#[allow(dead_code)]
#[path = "../examples/stress/case.rs"]
mod case;
#[allow(dead_code)]
#[path = "../examples/stress/city.rs"]
mod city;
#[allow(dead_code)]
#[path = "../examples/stress/fixture.rs"]
mod fixture;
use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use car_game::{RenderBackend, RuntimeConfig, roster::CarSpec, vehicle::DriveInput};

fn brake_entry(spec: CarSpec, mode: &str) -> (f32, f32) {
    let config = RuntimeConfig {
        backend: RenderBackend::Disabled,
        mode: mode.into(),
        car: spec.id.clone(),
        ..default()
    };
    let mut app = city::app(config, spec);
    for _ in 0..600 {
        app.update();
    }
    let mut poses = app
        .world_mut()
        .query_filtered::<&Transform, With<Velocity>>();
    let settled_height = poses.single(app.world()).unwrap().translation.y;
    for tick in 600..1560 {
        *app.world_mut().resource_mut::<DriveInput>() = DriveInput {
            throttle: 1.,
            steer: if tick >= 1320 { 0.45 } else { 0. },
            handbrake: (1440..1500).contains(&tick),
            ..default()
        };
        app.update();
    }
    let mut query = app.world_mut().query::<(&Transform, &Velocity)>();
    let (pose, velocity) = query.single(app.world()).unwrap();
    (
        Vec2::new(velocity.linear.x, velocity.linear.z).length(),
        pose.translation.y - settled_height,
    )
}

#[test]
fn fleet_course_preserves_the_original_city_and_enters_braking_at_speed_for_all_cars() {
    let cars = car_game::roster::Fleet::default().cars;
    let mini = cars.iter().find(|c| c.id == "exp-037").unwrap().clone();
    let legacy = brake_entry(mini, "interactive");
    assert!(
        legacy.0 < 5. && legacy.1 > 0.5,
        "legacy collision must reproduce: {legacy:?}"
    );
    let mut failures = Vec::new();
    for car in cars {
        let id = car.id.clone();
        let result = brake_entry(car, "fleet");
        if result.0 <= 5. || !result.0.is_finite() || result.1 > 0.4 {
            failures.push((id, result));
        }
    }
    assert!(failures.is_empty(), "native course failures: {failures:?}");
}
