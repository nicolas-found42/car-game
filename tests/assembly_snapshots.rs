use bevy::{prelude::*, time::TimeUpdateStrategy};
use car_game::{
    RenderBackend, RuntimeConfig,
    assembly::*,
    diagnostics::RunRecorder,
    runtime::{CommandBus, ControllerOwner, ProductionCommand},
    scenario::ScenarioDefinition,
};
use std::{path::PathBuf, sync::Arc, time::Duration};

fn application(
    name: &str,
    scenario: Option<ScenarioDefinition>,
) -> (App, Arc<RunRecorder>, PathBuf) {
    let evidence = std::env::var_os("ASSEMBLY_SNAPSHOT_EVIDENCE").map(PathBuf::from);
    let root = evidence.unwrap_or_else(|| {
        std::env::temp_dir().join(format!("grid-snapshots-{}-{name}", std::process::id()))
    });
    let config = RuntimeConfig {
        backend: RenderBackend::Disabled,
        mode: if scenario.is_some() {
            "scenario"
        } else {
            "snapshot-test"
        }
        .into(),
        scenario,
        ..default()
    };
    let run = Arc::new(RunRecorder::create_in(&root, name, "350z"));
    let dir = run.dir().to_path_buf();
    let mut app = car_game::build_app(config, run.clone()).unwrap();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1. / 120.,
    )));
    if name == "snapshot-target" {
        for _ in 0..1024 {
            app.world_mut().spawn_empty();
        }
    }
    app.update();
    (app, run, dir)
}
fn submit(app: &mut App, command: ProductionCommand) {
    app.world_mut()
        .resource_mut::<CommandBus>()
        .submit(ControllerOwner::Scenario, command);
    app.update();
    app.update();
}
fn finish(app: App, run: Arc<RunRecorder>, dir: PathBuf) {
    drop(app);
    run.finish(bevy::app::AppExit::Success);
    if std::env::var_os("ASSEMBLY_SNAPSHOT_EVIDENCE").is_none() {
        std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
    }
}
/// Explicit non-stock fixture: partition existing front-left wheel/tire nodes and
/// its 25kg / diagonal 2kg m² into a 20kg wheel plus centered 5kg tire child.
/// Intact and removed totals stay identical to the permanent pilot manifest.
fn author_nested_fixture(app: &mut App) {
    let world = app.world_mut();
    let mut assembly = world.resource_mut::<Assembly>();
    let wheel = DefinitionId::from("350z-test-wheel-0");
    let tire = DefinitionId::from("snapshot-fixture-tire");
    let parent = assembly.definitions.get_mut(&wheel).unwrap();
    let mut child = parent.clone();
    child.mass = 5.;
    child.inertia = [[0.4, 0., 0.], [0., 0.4, 0.], [0., 0., 0.4]];
    child.mounts.clear();
    child
        .visual
        .nodes
        .retain(|(_, name)| name.starts_with("tire_"));
    child.provenance =
        "#13 authored snapshot fixture; existing tire node, approximate 5kg centered child".into();
    parent.mass = 20.;
    parent.inertia = [[1.6, 0., 0.], [0., 1.6, 0.], [0., 0., 1.6]];
    parent
        .visual
        .nodes
        .retain(|(_, name)| name.starts_with("wheel_"));
    parent.mounts.insert(
        "fixture-tire".into(),
        Mount {
            wheel_corner: None,
            interface: child.interface.clone(),
            frame: default(),
            required: true,
        },
    );
    assembly.definitions.insert(tire.clone(), child);
    assembly.parts.push(Part {
        id: PartId::from("snapshot-fixture-tire"),
        definition: tire,
        attachment: Attachment::Mounted {
            parent: PartId::from("350z-wheel-0"),
            mount: "fixture-tire".into(),
        },
        condition: 0.37,
        secured: false,
    });
    assembly.validate().unwrap();
    assert!((assembly.aggregate().unwrap().mass - 1320.).abs() < 1e-9);
    let id = PartId::from("snapshot-fixture-tire");
    let entity = world.spawn(PartIdentity(id.clone())).id();
    world
        .resource_mut::<AssemblyRuntime>()
        .parts
        .insert(id, entity);
}
#[test]
fn nested_subtree_round_trip_runs_production_scenario_and_preserves_descendant_state() {
    let mut definition =
        ScenarioDefinition::from_json(include_str!("../examples/scenarios/assembly-snapshot.json"))
            .unwrap();
    // Authored worn/unsecured child remains unsafe when reattached, not magically repaired.
    if let car_game::scenario::AssertionKind::AssemblyState { readiness, .. } =
        &mut definition.assertions[3].condition
    {
        *readiness = "unsafe".into();
    }
    let (mut app, run, dir) = application("nested-snapshot", Some(definition));
    author_nested_fixture(&mut app);
    for _ in 0..500 {
        app.update();
        if app.should_exit().is_some() {
            break;
        }
    }
    assert_eq!(app.should_exit(), Some(bevy::app::AppExit::Success));
    let assembly = app.world().resource::<Assembly>();
    let child = assembly
        .part(&PartId::from("snapshot-fixture-tire"))
        .unwrap();
    assert_eq!((child.condition, child.secured), (0.37, false));
    assert!(assembly.installed(&child.id).unwrap());
    assert_eq!(assembly.committed.len(), 2);
    let summary: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("scenario-summary.json")).unwrap()).unwrap();
    assert_eq!(summary["passed"], true);
    assert_eq!(summary["assertions_executed"], 4);
    assert_eq!(summary["results"][2]["observed"]["part_count"], 6);
    finish(app, run, dir);
}
#[test]
fn restore_rejections_leave_domain_and_entity_mappings_unchanged() {
    let (mut app, run, dir) = application("invalid-snapshot", None);
    for _ in 0..5 {
        app.update();
    }
    let snapshot = AssemblySnapshot::capture(app.world().resource::<Assembly>(), 0);
    let invalid = invalid_snapshots(&snapshot);
    let count = invalid.len();
    for snapshot in invalid {
        let before = app.world().resource::<Assembly>().clone();
        let entities = app.world().resource::<AssemblyRuntime>().clone();
        submit(
            &mut app,
            ProductionCommand::RestoreAssembly {
                expected_revision: before.revision,
                source: SnapshotSource::Inline {
                    snapshot: Box::new(snapshot),
                },
            },
        );
        assert_eq!(*app.world().resource::<Assembly>(), before);
        let after = app.world().resource::<AssemblyRuntime>();
        assert_eq!(after.parts, entities.parts);
        assert_eq!(after.bodies, entities.bodies);
        assert_eq!(after.applied_revision, entities.applied_revision);
        assert_eq!(after.wheel_enabled, entities.wheel_enabled);
        assert_eq!(after.generation, entities.generation);
    }
    assert!(run.flush());
    let events = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
    let outcomes: Vec<serde_json::Value> = events
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|event: &serde_json::Value| {
            event["kind"] == "command.outcome"
                && event["fields"]["command"]["action"] == "restore_assembly"
        })
        .collect();
    assert_eq!(outcomes.len(), count);
    assert!(
        outcomes
            .iter()
            .all(|event| event["fields"]["disposition"] == "rejected")
    );
    finish(app, run, dir);
}
fn invalid_snapshots(snapshot: &AssemblySnapshot) -> Vec<AssemblySnapshot> {
    let mut invalid = Vec::new();
    let mut next = snapshot.clone();
    next.assembly_version = 900;
    invalid.push(next);
    let mut next = snapshot.clone();
    next.root_locations.push(next.root_locations[0].clone());
    invalid.push(next);
    let mut next = snapshot.clone();
    next.attachments[0].parent = PartId::from("dangling");
    invalid.push(next);
    let mut next = snapshot.clone();
    next.parts.push(next.parts[0].clone());
    invalid.push(next);
    let mut next = snapshot.clone();
    next.attachments.push(next.attachments[0].clone());
    invalid.push(next);
    let mut next = snapshot.clone();
    next.root_locations[0].root = PartId::from("dangling");
    invalid.push(next);
    let mut next = snapshot.clone();
    next.catalog_sha256 = "wrong-catalog".into();
    invalid.push(next);
    let mut next = snapshot.clone();
    next.parts[0].definition = DefinitionId::from("unknown-definition");
    invalid.push(next);
    let mut next = snapshot.clone();
    for part in ["350z-wheel-0", "350z-wheel-1"] {
        next.attachments.retain(|edge| edge.part.0 != part);
        next.root_locations
            .push(car_game::assembly::snapshot::RootLocationDto {
                root: PartId::from(part),
                location: Location::Inventory {
                    owner: OwnerId::from("sandbox"),
                    slot: "duplicate-slot".into(),
                },
            });
    }
    invalid.push(next);
    let mut next = snapshot.clone();
    next.networks.push(NetworkEdge {
        from: PartId::from("dangling"),
        to: PartId::from("350z-chassis"),
        network: "fixture".into(),
    });
    invalid.push(next);
    let mut next = snapshot.clone();
    next.prerequisites
        .push((PartId::from("350z-chassis"), PartId::from("dangling")));
    invalid.push(next);
    invalid
}

#[test]
fn serialized_snapshot_restores_in_fresh_application_without_runtime_handles() {
    let (mut original, run, dir) = application("snapshot-source", None);
    author_nested_fixture(&mut original);
    submit(
        &mut original,
        ProductionCommand::AssemblyTransaction {
            transaction: Transaction {
                id: TransactionId::from("detach-nested"),
                expected_revision: 0,
                part: PartId::from("350z-wheel-0"),
                destination: Attachment::Root {
                    location: Location::Inventory {
                        owner: OwnerId::from("sandbox"),
                        slot: "nested-rack".into(),
                    },
                },
            },
        },
    );
    submit(
        &mut original,
        ProductionCommand::SnapshotAssembly {
            key: "portable".into(),
        },
    );
    let snapshot = original.world().resource::<SnapshotStore>().0["portable"].clone();
    let original_mappings = original.world().resource::<AssemblyRuntime>().clone();
    let decoded = serialized_dto(&snapshot);
    let (mut fresh, fresh_run, fresh_dir) = application("snapshot-target", None);
    author_nested_fixture(&mut fresh);
    submit(
        &mut fresh,
        ProductionCommand::RestoreAssembly {
            expected_revision: 0,
            source: SnapshotSource::Inline {
                snapshot: Box::new(decoded),
            },
        },
    );
    let restored = fresh.world().resource::<Assembly>();
    assert_eq!(
        restored.parts,
        original.world().resource::<Assembly>().parts
    );
    assert_eq!(restored.revision, 2);
    assert_eq!(
        restored
            .root(&PartId::from("snapshot-fixture-tire"))
            .unwrap(),
        &Location::Inventory {
            owner: OwnerId::from("sandbox"),
            slot: "nested-rack".into()
        }
    );
    let mapping = fresh.world().resource::<AssemblyRuntime>();
    assert_eq!(mapping.wheel_enabled, [false, true, true, true]);
    assert_ne!(mapping.bodies, original_mappings.bodies);
    for (id, entity) in &mapping.parts {
        assert_ne!(original_mappings.parts.get(id), Some(entity));
    }
    let detached_mass = assert_body_mass(&mut fresh, 1295.);
    submit(
        &mut fresh,
        ProductionCommand::AssemblyTransaction {
            transaction: Transaction {
                id: TransactionId::from("reattach-nested"),
                expected_revision: 2,
                part: PartId::from("350z-wheel-0"),
                destination: Attachment::Mounted {
                    parent: PartId::from("350z-chassis"),
                    mount: "wheel-0".into(),
                },
            },
        },
    );
    let reattached_mass = assert_body_mass(&mut fresh, 1320.);
    assert_eq!(
        fresh.world().resource::<AssemblyRuntime>().wheel_enabled,
        [true; 4]
    );
    assert!(
        fresh
            .world()
            .resource::<Assembly>()
            .installed(&PartId::from("snapshot-fixture-tire"))
            .unwrap()
    );
    assert_eq!(fresh.world().resource::<Assembly>().committed.len(), 2);
    std::fs::write(fresh_dir.join("fresh-runtime-proof.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "source_runtime_bodies":format!("{:?}",original_mappings.bodies),
        "restored_runtime_bodies":format!("{:?}",fresh.world().resource::<AssemblyRuntime>().bodies),
        "source_parts":format!("{:?}",original_mappings.parts),
        "restored_parts":format!("{:?}",fresh.world().resource::<AssemblyRuntime>().parts),
        "detached_mass":detached_mass,"reattached_mass":reattached_mass,
        "descendant":fresh.world().resource::<Assembly>().part(&PartId::from("snapshot-fixture-tire")).unwrap(),
        "inspection":fresh.world().resource::<Assembly>().inspect().unwrap()
    })).unwrap()).unwrap();
    finish(original, run, dir);
    finish(fresh, fresh_run, fresh_dir);
}

fn serialized_dto(snapshot: &AssemblySnapshot) -> AssemblySnapshot {
    let bytes = serde_json::to_vec(snapshot).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for forbidden in [
        "definitions",
        "bodies",
        "entities",
        "aggregate",
        "applied_revision",
        "wheel_enabled",
    ] {
        assert!(value.get(forbidden).is_none());
    }
    serde_json::from_slice(&bytes).unwrap()
}

fn assert_body_mass(app: &mut App, expected: f32) -> f32 {
    use bevy_rapier3d::prelude::ReadMassProperties;
    let world = app.world_mut();
    let mass = world
        .query_filtered::<&ReadMassProperties, With<car_game::vehicle::Vehicle>>()
        .single(world)
        .unwrap()
        .mass;
    assert!((mass - expected).abs() < 0.001, "{mass} != {expected}");
    mass
}
