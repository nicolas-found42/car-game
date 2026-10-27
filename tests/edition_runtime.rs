use bevy::{prelude::*, time::TimeUpdateStrategy};
use car_game::{
    assembly::*,
    diagnostics::RunRecorder,
    physics::CompletedSteps,
    runtime::{CommandBus, ControllerOwner, ProductionCommand, SimulationClock},
};
use std::{sync::Arc, time::Duration};

fn submit(app: &mut App, command: ProductionCommand) {
    app.world_mut()
        .resource_mut::<CommandBus>()
        .submit(ControllerOwner::Scenario, command);
    app.update();
}

fn transaction(assembly: &Assembly, corner: usize, reinstall: bool) -> Transaction {
    Transaction {
        id: TransactionId(format!("{}-{corner}-{reinstall}", assembly.vehicle.0)),
        expected_revision: assembly.revision,
        part: PartId(format!(
            "{}-wheel-{corner}",
            assembly.parts[0].id.0.trim_end_matches("-chassis")
        )),
        destination: if reinstall {
            Attachment::Mounted {
                parent: assembly.parts[0].id.clone(),
                mount: format!("wheel-{corner}"),
            }
        } else {
            Attachment::Root {
                location: Location::Inventory {
                    owner: assembly.owner.clone(),
                    slot: format!("rack-{corner}"),
                },
            }
        },
    }
}

fn application() -> (App, Arc<RunRecorder>, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("edition-runtime-{}", std::process::id()));
    let config = car_game::RuntimeConfig {
        backend: car_game::RenderBackend::Disabled,
        car: "gt4-nsx".into(),
        ..default()
    };
    let run = Arc::new(RunRecorder::create_in(&root, "edition-runtime", "gt4-nsx"));
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
    (app, run, root)
}

#[test]
fn production_selection_retains_ledgers_runtime_anchors_and_snapshot_authority() {
    let (mut app, run, root) = application();
    let specs: Vec<_> = car_game::roster::Fleet::default()
        .cars
        .into_iter()
        .filter(|spec| editions::prepared_hash(&spec.id).is_some())
        .collect();
    let mut retained = Vec::new();
    for spec in &specs {
        submit(
            &mut app,
            ProductionCommand::SelectCar {
                car: spec.id.clone(),
                reset_to_spawn: true,
            },
        );
        assert_eq!(
            app.world().resource::<Assembly>().vehicle,
            editions::vehicle_id(&spec.id)
        );
        submit(
            &mut app,
            ProductionCommand::SnapshotAssembly {
                key: spec.id.clone(),
            },
        );
        for corner in 0..4 {
            remove_and_check(&mut app, spec, corner);
            if corner != 3 {
                let reinstall = transaction(app.world().resource::<Assembly>(), corner, true);
                submit(
                    &mut app,
                    ProductionCommand::AssemblyTransaction {
                        transaction: reinstall,
                    },
                );
            }
        }
        let ledger = app.world().resource::<Assembly>().clone();
        let runtime = app.world().resource::<AssemblyRuntime>().clone();
        submit(&mut app, ProductionCommand::Reset);
        assert_eq!(*app.world().resource::<Assembly>(), ledger);
        submit(&mut app, ProductionCommand::CycleCar { direction: 1 });
        submit(&mut app, ProductionCommand::CycleCar { direction: -1 });
        assert_eq!(*app.world().resource::<Assembly>(), ledger);
        retained.push((spec.id.clone(), ledger, runtime));
    }
    for (id, ledger, runtime) in retained {
        submit(
            &mut app,
            ProductionCommand::SelectCar {
                car: id.clone(),
                reset_to_spawn: true,
            },
        );
        assert_eq!(*app.world().resource::<Assembly>(), ledger);
        assert_eq!(
            app.world().resource::<AssemblyRuntime>().parts,
            runtime.parts
        );
        assert_eq!(
            app.world().resource::<AssemblyRuntime>().generation,
            runtime.generation
        );
        submit(
            &mut app,
            ProductionCommand::RestoreAssembly {
                expected_revision: ledger.revision,
                source: SnapshotSource::Saved { key: id },
            },
        );
        assert_eq!(
            app.world().resource::<Assembly>().aggregate().unwrap().mass,
            1320.
        );
        assert!(app.world().resource::<AssemblyRuntime>().generation > runtime.generation);
        drive_and_check(&mut app);
    }
    check_unsupported(&mut app);
    drop(app);
    run.finish(bevy::app::AppExit::Success);
    std::fs::remove_dir_all(root).unwrap();
}

fn check_unsupported(app: &mut App) {
    let before = app.world().resource::<Assembly>().clone();
    submit(
        app,
        ProductionCommand::SelectCar {
            car: "fr2-cobra".into(),
            reset_to_spawn: true,
        },
    );
    submit(
        app,
        ProductionCommand::SnapshotAssembly {
            key: "unsupported".into(),
        },
    );
    assert!(
        !app.world()
            .resource::<SnapshotStore>()
            .0
            .contains_key("unsupported")
    );
    assert_eq!(*app.world().resource::<Assembly>(), before);
    assert_eq!(
        app.world().resource::<AssemblyRuntime>().applied_revision,
        None
    );
    submit(
        app,
        ProductionCommand::SpawnVehicle {
            id: before.vehicle.clone(),
            car: "gt4-nsx".into(),
            position: [30., 0.62, 190.],
            controller: ControllerOwner::Scenario,
        },
    );
    assert_eq!(
        app.world().resource::<CompletedSteps>().0[&before.vehicle]
            .evidence
            .assembly_revision,
        None
    );
    submit(
        app,
        ProductionCommand::SelectCar {
            car: "gt4-nsx".into(),
            reset_to_spawn: true,
        },
    );
    assert_eq!(
        app.world().resource::<car_game::roster::CarSpec>().id,
        "fr2-cobra"
    );
    assert_eq!(*app.world().resource::<Assembly>(), before);
}

fn remove_and_check(app: &mut App, spec: &car_game::roster::CarSpec, corner: usize) {
    let removal = transaction(app.world().resource::<Assembly>(), corner, false);
    submit(
        app,
        ProductionCommand::AssemblyTransaction {
            transaction: removal,
        },
    );
    let sample = &app.world().resource::<CompletedSteps>().0[&editions::vehicle_id(&spec.id)];
    assert_eq!(
        sample.tick,
        app.world().resource::<SimulationClock>().completed
    );
    assert_eq!(sample.evidence.mass, Some(1295.));
    assert_eq!(sample.evidence.wheels[corner].part_id, None);
    for index in (0..4).filter(|index| *index != corner) {
        assert_eq!(
            sample.evidence.wheels[index].part_id.as_ref().unwrap().0,
            format!("{}-wheel-{index}", spec.id)
        );
    }
    assert!(!app.world().resource::<AssemblyRuntime>().wheel_enabled[corner]);
}

fn drive_and_check(app: &mut App) {
    submit(
        app,
        ProductionCommand::Drive {
            throttle: 1.,
            brake: 0.,
            steer: 0.,
            handbrake: false,
        },
    );
    for _ in 0..180 {
        app.update();
    }
    let assembly = app.world().resource::<Assembly>();
    let sample = &app.world().resource::<CompletedSteps>().0[&assembly.vehicle];
    assert_eq!(sample.evidence.assembly_revision, Some(assembly.revision));
    assert_eq!(sample.evidence.mass, Some(1320.));
    assert!(sample.speed > 1.);
}
