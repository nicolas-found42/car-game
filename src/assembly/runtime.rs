use super::*;
use crate::{diagnostics::RunHandle, roster::CarSpec};
use bevy_rapier3d::prelude::AdditionalMassProperties;
/// Derived runtime mappings are deliberately excluded from serialized Assembly.
#[derive(Resource, Default, Clone)]
pub struct AssemblyRuntime {
    pub parts: BTreeMap<PartId, Entity>,
    pub generation: u64,
    pub bodies: BTreeMap<VehicleId, Entity>,
    pub applied_revision: Option<u64>,
    pub wheel_enabled: [bool; 4],
}
pub struct AssemblyPlugin;
impl Plugin for AssemblyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssemblyRuntime>()
            .init_resource::<SnapshotStore>()
            .init_resource::<super::snapshot::SnapshotOrigins>()
            .add_systems(
                PostStartup,
                (initialize, super::snapshot::initialize).chain(),
            )
            .add_systems(
                FixedUpdate,
                prepare
                    .in_set(crate::physics::PhysicsPhase::Prepare)
                    .after(crate::physics::prepare::prepare),
            );
    }
}
fn initialize(mut commands: Commands) {
    let assembly = Assembly::pilot(&crate::roster::CarSpec::default());
    assembly.validate().expect("pilot manifest");
    // Pilot authority is explicit; other fleet cars keep the permanent regression path.
    commands.insert_resource(assembly);
}
fn prepare(
    assembly: Res<Assembly>,
    mut runtime: ResMut<AssemblyRuntime>,
    mut cars: Query<(
        Entity,
        &crate::controls::VehicleControl,
        &mut AdditionalMassProperties,
    )>,
    identities: Query<&PartIdentity>,
) {
    let Some((entity, _, mut properties)) = cars
        .iter_mut()
        .find(|(_, control, _)| control.id == assembly.vehicle && control.spec.id == "350z")
    else {
        runtime.applied_revision = None;
        runtime.bodies.remove(&assembly.vehicle);
        return;
    };
    if runtime.applied_revision == Some(assembly.revision)
        && runtime.bodies.get(&assembly.vehicle) == Some(&entity)
    {
        return;
    }
    // Resolve persistent parts through disposable runtime anchors. Rendering and
    // the physical adapter share this map; restore replaces every anchor.
    assert_eq!(runtime.parts.len(), assembly.parts.len());
    for (id, entity) in &runtime.parts {
        assert_eq!(
            &identities.get(*entity).expect("prepared part mapping").0,
            id
        );
        assembly.part(id).expect("mapped ledger part");
    }
    let mass = assembly.aggregate().expect("validated assembly").rapier();
    runtime.wheel_enabled = assembly.wheel_mask().expect("validated capability");
    *properties = mass;
    runtime.bodies.insert(assembly.vehicle.clone(), entity);
    runtime.applied_revision = Some(assembly.revision);
}
pub fn command(
    world: &mut World,
    queued: &crate::runtime::QueuedCommand,
    transaction: Option<&Transaction>,
) {
    if world.resource::<CarSpec>().id != "350z" {
        crate::runtime::outcome(world, queued, "rejected", "pilot-350z-required");
        return;
    }
    let result = transaction.map_or(Ok("assembly-inspected"), |t| {
        world.resource_mut::<Assembly>().transact(t)
    });
    let (disposition, reason) = match result {
        Ok(reason) => ("applied", reason.to_string()),
        Err(reason) => ("rejected", reason),
    };
    if let Some(run) = world.get_resource::<RunHandle>() {
        let mut envelope = queued.envelope.clone();
        envelope.tick = world.resource::<crate::runtime::SimulationClock>().current;
        envelope.boundary = "assembly_commit".into();
        run.event_envelope("assembly.inspection",serde_json::json!({"inspection":world.resource::<Assembly>().inspect().expect("validated ledger"),"disposition":disposition,"blocker":reason,"transaction":transaction}),envelope);
    }
    crate::runtime::outcome(world, queued, disposition, &reason);
}
