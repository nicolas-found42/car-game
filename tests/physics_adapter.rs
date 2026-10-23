use bevy::{prelude::*, time::TimeUpdateStrategy};
use car_game::{
    RenderBackend, RuntimeConfig,
    assembly::VehicleId,
    diagnostics::{PhysicsSample, RunRecorder},
    physics::{CompletedSteps, PhysicsEdit, QueryMode},
    runtime::{CommandBus, ControllerOwner, ProductionCommand, SimulationClock},
};
use std::{path::PathBuf, sync::Arc, time::Duration};

fn application(name: &str) -> (App, Arc<RunRecorder>, PathBuf) {
    configured_application(name, None)
}
fn configured_application(
    name: &str,
    scenario: Option<car_game::scenario::ScenarioDefinition>,
) -> (App, Arc<RunRecorder>, PathBuf) {
    let root = std::env::var_os("PHYSICS_ADAPTER_EVIDENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("grid-physics-{}-{name}", std::process::id()))
        });
    let config = RuntimeConfig {
        backend: RenderBackend::Disabled,
        mode: if scenario.is_some() {
            "scenario"
        } else {
            "physics-adapter-test"
        }
        .into(),
        seed: scenario.as_ref().map_or(0, |s| s.seed),
        scenario,
        ..default()
    };
    let run = Arc::new(RunRecorder::create_in(&root, name, "350z"));
    let dir = run.dir().to_path_buf();
    let mut app = car_game::build_app(config, run.clone()).unwrap();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        car_game::vehicle::STEP as f64,
    )));
    app.update();
    submit(
        &mut app,
        [ProductionCommand::AssignController {
            controller: ControllerOwner::Scenario,
        }],
    );
    (app, run, dir)
}
fn submit(app: &mut App, commands: impl IntoIterator<Item = ProductionCommand>) {
    let before = app.world().resource::<SimulationClock>().completed;
    for command in commands {
        app.world_mut()
            .resource_mut::<CommandBus>()
            .submit(ControllerOwner::Scenario, command);
    }
    app.update();
    assert_eq!(
        app.world().resource::<SimulationClock>().completed,
        before + 1
    );
}
fn edit(edit: PhysicsEdit) -> ProductionCommand {
    ProductionCommand::PhysicsEdit { edit }
}
fn relocate(position: [f32; 3]) -> ProductionCommand {
    edit(PhysicsEdit::RelocateVehicle {
        vehicle: VehicleId::from("pilot-350z"),
        position,
        rotation_y: 0.,
    })
}
fn road(id: &str, position: [f32; 3]) -> ProductionCommand {
    edit(PhysicsEdit::InsertRoad {
        id: id.into(),
        position,
        half_extents: [8., 0.2, 8.],
        parent: None,
    })
}
fn drive(throttle: f32) -> ProductionCommand {
    ProductionCommand::Drive {
        throttle,
        brake: 0.,
        steer: 0.,
        handbrake: false,
    }
}
fn sample(app: &App) -> PhysicsSample {
    app.world().resource::<CompletedSteps>().0[&VehicleId::from("pilot-350z")].clone()
}
fn finish(app: App, run: Arc<RunRecorder>, dir: PathBuf) {
    drop(app);
    run.finish(bevy::app::AppExit::Success);
    if std::env::var_os("PHYSICS_ADAPTER_EVIDENCE").is_none() {
        std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
    }
}
#[test]
fn inserted_removed_and_relocated_road_queries_and_forces_are_current_on_the_first_tick() {
    let (mut app, run, dir) = application("current-road");
    submit(
        &mut app,
        [
            road("fixture:road", [1000., 9.8, 0.]),
            relocate([1000., 10.55, 0.]),
            drive(1.),
        ],
    );
    let loaded = sample(&app);
    assert_eq!(loaded.grounded, 4);
    assert_eq!(loaded.evidence.query_tick, loaded.tick);
    assert!(
        loaded
            .evidence
            .wheels
            .iter()
            .all(|w| w.collider_id.as_deref() == Some("fixture:road"))
    );
    assert!(
        loaded.linear_velocity[2] < -0.001,
        "new drive force must affect this step: {loaded:?}"
    );
    assert_eq!(loaded.evidence.pre_step_linear_velocity, [0.; 3]);
    assert_eq!(loaded.integration_steps, 1);
    assert!((loaded.time - loaded.tick as f32 / 120.).abs() < 1e-5);
    submit(
        &mut app,
        [
            edit(PhysicsEdit::RemoveCollider {
                id: "fixture:road".into(),
            }),
            drive(0.),
        ],
    );
    let removed = sample(&app);
    assert_eq!(removed.grounded, 0);
    assert!(
        removed
            .evidence
            .wheels
            .iter()
            .all(|w| w.availability == "miss")
    );
    assert!(removed.evidence.total.force[1] < 1.);
    submit(
        &mut app,
        [
            road("fixture:second-road", [1100., 9.8, 0.]),
            relocate([1100., 10.55, 0.]),
        ],
    );
    assert!(
        sample(&app)
            .evidence
            .wheels
            .iter()
            .all(|w| w.collider_id.as_deref() == Some("fixture:second-road"))
    );
    finish(app, run, dir);
}
#[test]
fn reset_ticks_publish_actual_writeback_with_zero_composed_force_and_no_repeated_one_shot() {
    let (mut app, run, dir) = application("completed-reset");
    submit(&mut app, [drive(1.)]);
    submit(&mut app, [ProductionCommand::Reset]);
    let reset = sample(&app);
    assert_eq!(reset.reset_count, 1);
    assert_eq!(reset.evidence.outcome, "reset");
    assert_eq!(reset.evidence.total.force, [0.; 3]);
    assert_eq!(reset.evidence.total.torque, [0.; 3]);
    assert!(reset.input.reset);
    assert_eq!(reset.evidence.query_tick, reset.tick);
    assert_eq!(reset.evidence.assembly_revision, Some(0));
    let mut query = app.world_mut().query_filtered::<(&Transform, &bevy_rapier3d::prelude::Velocity), With<car_game::controls::PlayerVehicle>>();
    let (pose, velocity) = query.single(app.world()).unwrap();
    assert_eq!(reset.position, pose.translation.to_array());
    assert_eq!(reset.linear_velocity, velocity.linear.to_array());
    assert!(
        reset.linear_velocity[1] < 0.,
        "the normal gravity step follows reset preparation"
    );
    submit(&mut app, []);
    let next = sample(&app);
    assert_eq!(next.tick, reset.tick + 1);
    assert_eq!(next.reset_count, 1);
    assert!(!next.input.reset);
    assert!(next.phase_durations.sync_ns.is_some());
    assert!(next.phase_durations.query_ns.is_some());
    assert!(next.phase_durations.bridge_ns.is_some());
    run.flush();
    let records: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("physics.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(
        records
            .iter()
            .any(|r| r["tick"] == reset.tick && r["evidence"]["outcome"] == "reset")
    );
    assert!(
        records
            .windows(2)
            .all(|w| w[1]["tick"].as_u64().unwrap() == w[0]["tick"].as_u64().unwrap() + 1)
    );
    finish(app, run, dir);
}
#[test]
fn reparented_child_collider_uses_new_parent_pose_without_consuming_solver_updates() {
    let (mut app, run, dir) = application("reparented-collider");
    submit(
        &mut app,
        [
            road("fixture:a", [1000., 9.8, 0.]),
            road("fixture:b", [1100., 9.8, 0.]),
            edit(PhysicsEdit::InsertRoad {
                id: "fixture:deck".into(),
                position: [0., 2., 0.],
                half_extents: [8., 0.2, 8.],
                parent: Some("fixture:a".into()),
            }),
            relocate([1000., 12.55, 0.]),
        ],
    );
    assert!(
        sample(&app)
            .evidence
            .wheels
            .iter()
            .all(|w| w.collider_id.as_deref() == Some("fixture:deck"))
    );
    submit(
        &mut app,
        [
            edit(PhysicsEdit::ReparentCollider {
                id: "fixture:deck".into(),
                parent: "fixture:b".into(),
                position: [0., 2., 0.],
            }),
            relocate([1100., 12.55, 0.]),
        ],
    );
    assert!(
        sample(&app)
            .evidence
            .wheels
            .iter()
            .all(|w| w.collider_id.as_deref() == Some("fixture:deck"))
    );
    // Query reads did not steal initialization, removal or reparent notifications:
    // the regular solver must still stop a genuinely colliding falling chassis.
    submit(&mut app, [relocate([1100., 11.9, 0.])]);
    for _ in 0..120 {
        submit(&mut app, []);
    }
    assert!(sample(&app).position[1] > 11.5);
    finish(app, run, dir);
}
#[test]
fn legacy_pre_sync_path_remains_selectable_as_a_regression_reference() {
    let (mut app, run, dir) = application("legacy-reference");
    app.insert_resource(QueryMode::LegacyPreSync);
    submit(
        &mut app,
        [
            road("fixture:legacy", [1000., 9.8, 0.]),
            relocate([1000., 10.55, 0.]),
        ],
    );
    assert_eq!(sample(&app).query_mode, "legacy_pre_sync");
    assert_eq!(
        sample(&app).grounded,
        0,
        "legacy queries see previous backend geometry"
    );
    submit(&mut app, []);
    assert_eq!(sample(&app).grounded, 4);
    finish(app, run, dir);
}

#[test]
fn composed_forces_match_the_backend_and_removed_wheel_capability_stays_with_its_vehicle() {
    let (mut app, run, dir) = application("force-composition");
    let remove: ProductionCommand = serde_json::from_value(serde_json::json!({
        "action":"assembly_transaction","transaction":{"id":"remove-first-wheel","expected_revision":0,
        "part":"350z-wheel-0","destination":{"kind":"root","location":{"kind":"inventory","owner":"sandbox","slot":"workbench"}}}
    })).unwrap();
    submit(
        &mut app,
        [
            ProductionCommand::SpawnVehicle {
                id: VehicleId::from("independent"),
                car: "gt4-nsx".into(),
                position: [-3.2, 0.62, 190.],
                controller: ControllerOwner::Scenario,
            },
            remove,
            drive(1.),
        ],
    );
    let player = sample(&app);
    assert_eq!(player.evidence.assembly_revision, Some(1));
    assert_eq!(player.evidence.mass, Some(1295.));
    assert_eq!(player.body_part_id.unwrap().0, "350z-chassis");
    assert_eq!(player.evidence.wheels[0].availability, "wheel_absent");
    assert_eq!(
        player.evidence.wheels[0].part_id.as_ref().unwrap().0,
        "350z-wheel-0"
    );
    let other = &app.world().resource::<CompletedSteps>().0[&VehicleId::from("independent")];
    assert_eq!(other.evidence.assembly_binding, "no_assembly_binding");
    assert_eq!(other.evidence.assembly_revision, None);
    assert_eq!(other.grounded, 4);
    assert_eq!(other.evidence.mass, Some(1320.));
    for state in app.world().resource::<CompletedSteps>().0.values() {
        let mut force = Vec3::from_array(state.evidence.chassis.force);
        let mut torque = Vec3::from_array(state.evidence.chassis.torque);
        for wheel in &state.evidence.wheels {
            for part in [&wheel.suspension, &wheel.tire, &wheel.antiroll] {
                force += Vec3::from_array(part.force);
                torque += Vec3::from_array(part.torque);
            }
        }
        assert!((force - Vec3::from_array(state.evidence.total.force)).length() < 0.01);
        assert!((torque - Vec3::from_array(state.evidence.total.torque)).length() < 0.01);
        assert_eq!(
            state.evidence.total.force,
            state.backend_applied.as_ref().unwrap().force
        );
        assert_eq!(
            state.evidence.total.torque,
            state.backend_applied.as_ref().unwrap().torque
        );
    }
    finish(app, run, dir);
}

#[test]
fn authored_query_force_and_reset_scenario_executes_all_eight_first_tick_assertions() {
    let definition = car_game::scenario::ScenarioDefinition::from_json(include_str!(
        "../examples/scenarios/synchronized-physics.json"
    ))
    .unwrap();
    let (mut app, run, dir) = configured_application("synchronized-scenario", Some(definition));
    for _ in 0..500 {
        app.update();
        if app.should_exit().is_some() {
            break;
        }
    }
    let summary: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("scenario-summary.json")).unwrap()).unwrap();
    assert_eq!(summary["passed"], true, "{summary}");
    assert_eq!(summary["assertions_executed"], 8);
    assert_eq!(app.should_exit(), Some(bevy::app::AppExit::Success));
    finish(app, run, dir);
}

#[test]
fn airborne_gravity_advances_exactly_one_outer_step_without_stale_drive_force() {
    let (mut app, run, dir) = application("one-step");
    submit(&mut app, [drive(1.)]);
    submit(&mut app, [relocate([1000., 100., 0.]), drive(0.)]);
    let state = sample(&app);
    let expected = -9.81 * car_game::vehicle::STEP / (1. + 0.02 * car_game::vehicle::STEP);
    assert!(
        (state.linear_velocity[1] - expected).abs() < 1e-5,
        "{state:?}"
    );
    assert!(state.linear_velocity[0].abs() < 1e-6 && state.linear_velocity[2].abs() < 1e-6);
    assert_eq!(state.evidence.total.force, [0.; 3]);
    assert_eq!(state.backend_applied.unwrap().force, [0.; 3]);
    assert_eq!(state.integration_steps, 1);
    finish(app, run, dir);
}
