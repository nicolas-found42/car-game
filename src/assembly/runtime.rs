use super::*;
use crate::{diagnostics::RunHandle, roster::CarSpec};
use bevy_rapier3d::prelude::AdditionalMassProperties;
/// Derived runtime mappings are deliberately excluded from serialized Assembly.
#[derive(Resource, Default)]
pub struct AssemblyRuntime {
    pub bodies: BTreeMap<VehicleId, Entity>,
    pub applied_revision: Option<u64>,
    pub wheel_enabled: [bool; 4],
}
pub struct AssemblyPlugin;
impl Plugin for AssemblyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssemblyRuntime>()
            .add_systems(PostStartup, initialize)
            .add_systems(FixedUpdate, prepare.before(crate::vehicle::simulate));
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
    spec: Res<CarSpec>,
    mut runtime: ResMut<AssemblyRuntime>,
    mut cars: Query<(Entity, &mut AdditionalMassProperties), With<crate::controls::PlayerVehicle>>,
) {
    if spec.id != "350z" {
        runtime.applied_revision = None;
        runtime.bodies.remove(&assembly.vehicle);
        return;
    }
    if runtime.applied_revision == Some(assembly.revision) {
        return;
    }
    let mass = assembly.aggregate().expect("validated assembly").rapier();
    runtime.wheel_enabled = assembly.wheel_mask().expect("validated capability");
    for (entity, mut properties) in &mut cars {
        *properties = mass;
        runtime.bodies.insert(assembly.vehicle.clone(), entity);
    }
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
