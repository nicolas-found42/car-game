use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use car_game::{roster::Fleet, vehicle::VehiclePlugin};

fn spawned_mass(id: &str) -> f32 {
    let spec = Fleet::default()
        .cars
        .into_iter()
        .find(|s| s.id == id)
        .unwrap();
    let mut app = App::new();
    app.insert_resource(spec)
        .add_plugins((MinimalPlugins, TransformPlugin, VehiclePlugin));
    app.update();
    let mut q = app.world_mut().query::<&AdditionalMassProperties>();
    match q.single(app.world()).unwrap() {
        AdditionalMassProperties::MassProperties(properties) => properties.mass,
        _ => panic!("Explicit profile mass properties required"),
    }
}

#[test]
fn estimated_torque_curves_respect_both_reference_output_limits() {
    use car_game::vehicle::profile;
    for spec in Fleet::default().cars {
        let p = profile::for_car(&spec.id);
        for rpm in (900..=p.redline_rpm as u32).step_by(25) {
            let torque = p.torque(rpm as f32);
            assert!(torque <= p.torque_nm + 0.01, "{}", spec.id);
            assert!(
                torque * rpm as f32 / 9549.297 <= p.power_kw + 0.01,
                "{}",
                spec.id
            );
        }
    }
}

#[test]
fn selected_fleet_cars_spawn_with_distinct_reference_masses() {
    let porsche = spawned_mass("redline-911");
    let mustang = spawned_mass("redline-gt500");
    assert!(
        mustang - porsche > 500.,
        "Shared arcade mass: Porsche={porsche}, Mustang={mustang}"
    );
    assert!((porsche - 1160.).abs() < 0.1);
    assert!((mustang - 1778.).abs() < 2.);
}

fn settled(id: &str) -> App {
    use bevy::time::TimeUpdateStrategy;
    use car_game::vehicle::STEP;
    let spec = Fleet::default()
        .cars
        .into_iter()
        .find(|s| s.id == id)
        .unwrap();
    let mut app = App::new();
    app.insert_resource(spec)
        .add_plugins((MinimalPlugins, TransformPlugin, VehiclePlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(STEP as f64),
        ));
    app.world_mut().spawn((
        RigidBody::Fixed,
        Collider::cuboid(1000., 0.2, 1000.),
        Transform::from_xyz(0., -0.2, 0.),
    ));
    for _ in 0..360 {
        app.update();
    }
    app
}

#[test]
fn physical_center_of_mass_produces_the_declared_static_axle_loads() {
    use car_game::vehicle::{Vehicle, profile};
    for id in [
        "redline-911",
        "mcla-evo",
        "gt4-nsx",
        "ref-a13",
        "ref-e09",
        "ref-j12",
    ] {
        let app = &mut settled(id);
        let mut q = app.world_mut().query::<&Vehicle>();
        let car = q.single(app.world()).unwrap();
        let p = profile::for_car(id);
        let loads: [f32; 4] =
            std::array::from_fn(|i| (0.44 - car.wheel_lengths[i]) * p.spring(i / 2));
        let total = loads.iter().sum::<f32>();
        assert_eq!(car.grounded, 4, "{id}");
        assert!(
            ((loads[0] + loads[1]) / total - p.front_weight).abs() < 0.015,
            "{id}"
        );
        assert!((total / (p.mass_kg * 9.81) - 1.).abs() < 0.02, "{id}");
    }
}

#[test]
fn production_traction_matches_each_declared_driven_axle_layout() {
    use car_game::vehicle::{DriveInput, Vehicle};
    for (id, front, rear) in [
        ("350z", false, true),
        ("mcla-evo", true, true),
        ("mcla-skyline", true, true),
        ("ref-a06", true, false),
        ("ref-e09", true, false),
    ] {
        let app = &mut settled(id);
        app.world_mut().resource_mut::<DriveInput>().throttle = 1.;
        for _ in 0..120 {
            app.update();
        }
        let mut q = app.world_mut().query::<&Vehicle>();
        let car = q.single(app.world()).unwrap();
        let requested = car.drive_demands[0] + car.drive_demands[1];
        let applied = car.longitudinal_forces[0] + car.longitudinal_forces[1];
        if front {
            assert!(requested > 0. && applied > 0., "{id}");
        } else {
            assert_eq!(requested, 0., "{id}");
        }
        let rear_applied = car.longitudinal_forces[2] + car.longitudinal_forces[3];
        assert_eq!(rear_applied > 0., rear, "{id}");
    }
}

fn sprint_seconds(id: &str, target_mps: f32) -> Option<f32> {
    use bevy_rapier3d::prelude::Velocity;
    use car_game::vehicle::{DriveInput, STEP, Vehicle};
    let mut app = settled(id);
    app.world_mut().resource_mut::<DriveInput>().throttle = 1.;
    for tick in 1..=2400 {
        app.update();
        let mut q = app.world_mut().query::<(&Velocity, &Vehicle)>();
        let (v, car) = q.single(app.world()).unwrap();
        assert_eq!(car.grounded, 4, "{id}");
        if Vec2::new(v.linear.x, v.linear.z).length() >= target_mps {
            return Some(tick as f32 * STEP);
        }
    }
    None
}

#[test]
fn calibration_launches_match_broad_same_model_test_envelopes() {
    let murcielago = sprint_seconds("ref-e04", 100. / 3.6).unwrap();
    let gt500 = sprint_seconds("redline-gt500", 26.8224).unwrap();
    assert!((3.4..4.3).contains(&murcielago), "Murcielago {murcielago}");
    assert!((4.0..4.9).contains(&gt500), "GT500 {gt500}");
}

#[test]
fn gt500_midrange_does_not_hide_behind_a_slow_launch() {
    // 2008 MotorTrend test: 4.3 s to 60 mph, 9.7 s to 100 mph.
    // The interval avoids attributing an unknown rollout to engine delivery.
    let sixty = sprint_seconds("redline-gt500", 26.8224).unwrap();
    let hundred = sprint_seconds("redline-gt500", 44.704).unwrap();
    let interval = hundred - sixty;
    assert!((4.0..4.7).contains(&sixty), "launch {sixty}");
    assert!((5.1..5.8).contains(&interval), "60–100 mph {interval}");
}

fn stop_metres(id: &str, entry: f32) -> f32 {
    use bevy_rapier3d::prelude::Velocity;
    use car_game::vehicle::DriveInput;
    let mut app = settled(id);
    let mut q = app.world_mut().query::<(&Transform, &mut Velocity)>();
    let (pose, mut v) = q.single_mut(app.world_mut()).unwrap();
    let mut previous = pose.translation;
    v.linear = Vec3::new(0., 0., -entry);
    app.world_mut().resource_mut::<DriveInput>().brake = 1.;
    let mut distance = 0.;
    for _ in 0..1200 {
        app.update();
        let mut q = app.world_mut().query::<(&Transform, &Velocity)>();
        let (pose, v) = q.single(app.world()).unwrap();
        distance += Vec2::new(
            pose.translation.x - previous.x,
            pose.translation.z - previous.z,
        )
        .length();
        previous = pose.translation;
        if Vec2::new(v.linear.x, v.linear.z).length() <= 0.1 {
            return distance;
        }
    }
    panic!("{id} never stopped");
}

#[test]
fn calibration_service_brakes_do_not_outperform_period_test_envelopes() {
    for (id, speed, low, high) in [
        ("gt-viper", 31.2928, 51., 57.),
        ("redline-gt500", 26.8224, 34., 38.),
        ("350z", 31.2928, 47., 53.),
        ("ref-a14", 26.8224, 35., 39.),
    ] {
        let distance = stop_metres(id, speed);
        assert!((low..high).contains(&distance), "{id} {distance}");
    }
}

#[test]
fn documented_speed_governor_cuts_drive_without_limiting_other_cars() {
    use car_game::vehicle::{Vehicle, profile};
    let mut car = Vehicle::default();
    car.gear = 6;
    let limited = profile::for_car("gt4-m3").drive_force(&mut car, 75., 0.34, 1.);
    assert_eq!(limited, 0., "M3 must stop delivering power above 250 km/h");
    let below_limit = profile::for_car("gt4-m3").drive_force(&mut car, 55., 0.34, 1.);
    assert!(below_limit > 0.);
    let unrestricted = profile::for_car("ref-e03").drive_force(&mut car, 75., 0.34, 1.);
    assert!(unrestricted > 0., "No universal fleet speed cap");
}

fn demand_with_radii(id: &str, radii: [f32; 4]) -> (f32, f32) {
    use car_game::vehicle::{DriveInput, Vehicle};
    let mut app = settled(id);
    app.world_mut()
        .resource_mut::<car_game::roster::CarSpec>()
        .radii = radii;
    app.world_mut().resource_mut::<DriveInput>().throttle = 1.;
    let mut q = app.world_mut().query::<(&mut Velocity, &mut Vehicle)>();
    let (mut velocity, mut car) = q.single_mut(app.world_mut()).unwrap();
    velocity.linear = Vec3::new(0., 0., -12.);
    car.gear = 1;
    app.update();
    let mut q = app.world_mut().query::<&Vehicle>();
    let car = q.single(app.world()).unwrap();
    (car.drive_demands.iter().sum(), car.rpm)
}

#[test]
fn undriven_tires_cannot_change_production_engine_rpm_or_drive_demand() {
    for (id, original, changed) in [
        ("350z", [0.32, 0.32, 0.36, 0.36], [0.4, 0.4, 0.36, 0.36]),
        ("ref-a06", [0.32, 0.32, 0.36, 0.36], [0.32, 0.32, 0.4, 0.4]),
    ] {
        let before = demand_with_radii(id, original);
        let after = demand_with_radii(id, changed);
        assert!(before.0 > 0.);
        assert_eq!(
            before, after,
            "{id}: undriven tires changed engine coupling"
        );
    }
}

#[test]
fn driven_radius_averages_driven_wheel_rpm_at_nominal_axle_split() {
    use car_game::vehicle::profile;
    let radii = [0.30, 0.34, 0.36, 0.40];
    for id in ["350z", "ref-a06", "mcla-evo", "mcla-skyline"] {
        let p = profile::for_car(id);
        let expected = p.front_drive * (1. / radii[0] + 1. / radii[1]) / 2.
            + (1. - p.front_drive) * (1. / radii[2] + 1. / radii[3]) / 2.;
        assert!((1. / p.drive_radius(radii) - expected).abs() < 1e-6, "{id}");
        assert!((p.drive_radius([0.35; 4]) - 0.35).abs() < 1e-7);
        let mut car = car_game::vehicle::Vehicle::default();
        let force = p.drive_force(&mut car, 12., p.drive_radius(radii), 1.);
        assert!(force > 0.);
        let rpm = (12. * expected * 60. / std::f32::consts::TAU * p.gears[0] * p.final_drive)
            .max((p.torque_rpm * 0.65).clamp(1800., 3200.));
        assert!((car.rpm - rpm).abs() < 0.002);
    }
}

// Reuse the production benchmark fixture; its CLI main is unused in this test.
#[allow(dead_code)]
#[path = "../examples/fleet_skidpad.rs"]
mod skidpad_fixture;

#[test]
fn period_us_sedan_acceleration_and_brakes_match_qualified_envelopes() {
    // US2007 RS4 test is related-year evidence for the2008 edition.
    // CTS-V2004 source is exact-year/engine; rollout is not converted.
    for (id, launch, hundred, interval) in
        [("ref-e12", 4.6, 11.1, 6.5), ("ref-a14", 5.1, 12.3, 7.2)]
    {
        let sixty = sprint_seconds(id, 26.8224).unwrap();
        let actual = sprint_seconds(id, 44.704).unwrap();
        assert!((sixty / launch - 1.).abs() < 0.06, "{id} launch {sixty}");
        assert!(
            (actual / hundred - 1.).abs() < 0.05,
            "{id} 100 mph {actual}"
        );
        assert!(
            ((actual - sixty) / interval - 1.).abs() < 0.08,
            "{id} midrange {}",
            actual - sixty
        );
    }
    let rs4_stop = stop_metres("ref-e12", 31.2928);
    assert!(
        (rs4_stop / (165. * 0.3048) - 1.).abs() < 0.05,
        "RS4 stop {rs4_stop}"
    );
}

#[test]
fn period_us_sedan_lateral_grip_uses_the_matched_circle_diameter() {
    for (id, diameter_ft, reference_g) in [("ref-a14", 200., 0.88), ("ref-e12", 300., 0.91)] {
        let spec = Fleet::default()
            .cars
            .into_iter()
            .find(|c| c.id == id)
            .unwrap();
        let measured = skidpad_fixture::sweep(spec, diameter_ft).peak_sustained_g;
        assert!(
            (measured - reference_g).abs() < 0.03,
            "{id} {diameter_ft} ft grip {measured}"
        );
    }
}

#[test]
fn track_350z_midrange_matches_the_qualified_original_test_envelope() {
    // C/D 2003 US Track 6MT: 5.4/13.9 s, hence 8.5 s from 60 to 100 mph.
    // Unknown rollout is not converted; AW's 5.66 s launch provides spread.
    let sixty = sprint_seconds("350z", 26.8224).unwrap();
    let hundred = sprint_seconds("350z", 44.704).unwrap();
    assert!((5.1..5.95).contains(&sixty), "350Z launch {sixty}");
    assert!((13.3..14.7).contains(&hundred), "350Z 100 mph {hundred}");
    assert!(
        (8.1..9.0).contains(&(hundred - sixty)),
        "350Z midrange {}",
        hundred - sixty
    );
}

#[test]
fn early_nsx_grip_respects_a_qualified_period_envelope() {
    // C/D .86 g on 300 ft; independent R&T .93 g on a makeshift circle.
    // R&T diameter/build status are unresolved: this is a broad guard, not
    // a claim of exact production-car or matched-circle validation.
    let spec = Fleet::default()
        .cars
        .into_iter()
        .find(|c| c.id == "gt4-nsx")
        .unwrap();
    let grip = skidpad_fixture::sweep(spec, 300.).peak_sustained_g;
    assert!((0.84..0.95).contains(&grip), "Early NSX grip {grip}");
}
