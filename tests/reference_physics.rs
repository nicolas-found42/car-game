use bevy::{prelude::*, time::TimeUpdateStrategy};
use car_game::{assembly::*, diagnostics::RunRecorder, physics::CompletedSteps, runtime::*};
use std::{sync::Arc, time::Duration};

fn submit(app: &mut App, command: ProductionCommand) {
    app.world_mut()
        .resource_mut::<CommandBus>()
        .submit(ControllerOwner::Scenario, command);
    app.update();
}

#[test]
fn reference_mode_drives_every_active_edition_with_its_own_mass_and_controls() {
    let root = std::env::temp_dir().join(format!("reference-fleet-{}", std::process::id()));
    let run = Arc::new(RunRecorder::create_in(&root, "reference-test", "350z"));
    let config = car_game::RuntimeConfig {
        backend: car_game::RenderBackend::Disabled,
        reference_physics: true,
        ..default()
    };
    let mut app = car_game::build_app(config, run.clone()).unwrap();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        car_game::vehicle::STEP as f64,
    )));
    app.update();
    submit(
        &mut app,
        ProductionCommand::AssignController {
            controller: ControllerOwner::Scenario,
        },
    );
    for spec in car_game::roster::Fleet::default().cars {
        submit(
            &mut app,
            ProductionCommand::SelectCar {
                car: spec.id.clone(),
                reset_to_spawn: true,
            },
        );
        submit(
            &mut app,
            ProductionCommand::Drive {
                throttle: 0.5,
                brake: 0.,
                steer: 0.,
                handbrake: false,
            },
        );
        for _ in 0..240 {
            app.update();
        }
        let id = editions::vehicle_id(&spec.id);
        let sample = &app.world().resource::<CompletedSteps>().0[&id];
        let profile = car_game::vehicle::profile::for_car(&spec.id);
        assert!(
            (sample.evidence.mass.unwrap() - profile.mass_kg).abs() < 0.02,
            "{}",
            spec.id
        );
        assert!(
            sample.speed > 0.1 && sample.grounded >= 3,
            "{}: {:?}",
            spec.id,
            sample
        );
        assert_eq!(sample.input.throttle, 0.5);
        assert!(sample.decisions.is_some());
        if profile.front_drive == 0. {
            assert_eq!(&sample.drive_demands[..2], &[0., 0.]);
        }
        submit(&mut app, ProductionCommand::Reset);
        submit(
            &mut app,
            ProductionCommand::Drive {
                throttle: 0.,
                brake: 1.,
                steer: 0.,
                handbrake: false,
            },
        );
        for _ in 0..20 {
            app.update();
        }
        let reverse = &app.world().resource::<CompletedSteps>().0[&id];
        assert_eq!(reverse.gear, -1);
        assert_eq!(reverse.decisions.as_ref().unwrap().gear_after, reverse.gear);
    }
    drop(app);
    run.finish(bevy::app::AppExit::Success);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reference_major_parts_decompose_declared_properties_without_changing_default_fixture() {
    for spec in car_game::roster::Fleet::default().cars {
        let Some(original) = Assembly::major_parts(&spec) else {
            continue;
        };
        let configured = Assembly::major_parts_configured(&spec, true).unwrap();
        configured.validate().unwrap();
        let properties = car_game::vehicle::profile::for_car(&spec.id).properties(&spec);
        let aggregate = configured.aggregate().unwrap();
        assert_eq!(original.aggregate().unwrap().mass, 1320.);
        assert!((aggregate.mass - properties.mass as f64).abs() < 1e-6);
        for axis in 0..3 {
            assert!(
                (aggregate.center[axis] - properties.local_center_of_mass[axis] as f64).abs()
                    < 1e-6
            );
            assert!(
                (aggregate.inertia[axis][axis] - properties.principal_inertia[axis] as f64).abs()
                    < 1e-5
            );
        }
        assert_eq!(configured.parts, original.parts);
        assert_eq!(configured.vehicle, original.vehicle);
    }
}

#[test]
fn ratio_aware_hysteresis_prevents_the_preserved_nsx_shift_reversal() {
    let profile = car_game::vehicle::profile::for_car("exp-231");
    for index in 1..profile.gears.len() {
        let landing = profile.redline_rpm * 0.95 * profile.gears[index] / profile.gears[index - 1];
        assert!(profile.downshift_rpm(index) < landing);
    }
    let mut car = car_game::vehicle::Vehicle::default();
    let radius = 0.32;
    let speed = profile.redline_rpm * 0.951 / (profile.gears[0] * profile.final_drive)
        * std::f32::consts::TAU
        / 60.
        * radius;
    profile.drive_force(&mut car, speed, radius, 1.);
    assert_eq!(car.gear, 2);
    for _ in 0..120 {
        profile.drive_force(&mut car, speed, radius, 1.);
    }
    assert_eq!(car.gear, 2);
}
