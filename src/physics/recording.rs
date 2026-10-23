use super::{PhaseClock, QueryMode};
use crate::{
    assembly::{Assembly, AssemblyRuntime, VehicleId},
    controls::{PlayerVehicle, VehicleControl},
    diagnostics::{InputSample, PhysicsSample, PhysicsTrace, RunHandle},
    vehicle::Vehicle,
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize)]
pub struct PhaseDurations {
    pub sync_ns: Option<u64>,
    pub query_ns: Option<u64>,
    pub mechanics_ns: Option<u64>,
    pub bridge_ns: Option<u64>,
    pub step_ns: Option<u64>,
    pub writeback_ns: Option<u64>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ForceSample {
    pub force: [f32; 3],
    pub torque: [f32; 3],
}
impl From<ExternalForce> for ForceSample {
    fn from(value: ExternalForce) -> Self {
        Self {
            force: value.force.to_array(),
            torque: value.torque.to_array(),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct WheelSample {
    pub corner: usize,
    pub part_id: Option<crate::assembly::PartId>,
    pub availability: String,
    pub collider_id: Option<String>,
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub range: f32,
    pub distance: Option<f32>,
    pub point: Option<[f32; 3]>,
    pub normal: Option<[f32; 3]>,
    pub suspension: ForceSample,
    pub tire: ForceSample,
    pub antiroll: ForceSample,
}
#[derive(Component, Clone, Debug, Default, Serialize)]
pub struct StepEvidence {
    pub query_tick: u64,
    pub query_boundary: String,
    pub geometry_revision: u64,
    pub assembly_revision: Option<u64>,
    pub assembly_binding: String,
    pub mass: Option<f32>,
    pub center_of_mass: [f32; 3],
    pub assembly_generation: Option<u64>,
    pub input: InputSample,
    pub outcome: String,
    pub wheels: [WheelSample; 4],
    pub wheel_loads: [f32; 4],
    pub ray_misses: usize,
    pub chassis: ForceSample,
    pub total: ForceSample,
    pub pre_step_position: [f32; 3],
    pub pre_step_linear_velocity: [f32; 3],
}
#[derive(Resource, Default)]
pub struct CompletedSteps(pub BTreeMap<VehicleId, PhysicsSample>);

pub(super) fn observe(world: &mut World) {
    let mut durations = world.resource::<PhaseClock>().durations.clone();
    durations.writeback_ns = world
        .resource::<PhaseClock>()
        .start
        .map(|s| s.elapsed().as_nanos() as u64);
    let tick = world
        .get_resource::<crate::runtime::SimulationClock>()
        .map_or_else(
            || world.resource::<super::CurrentQueries>().tick,
            |c| c.current,
        );
    let time = world.resource::<Time<Fixed>>().elapsed_secs();
    let mode = *world.resource::<QueryMode>();
    let recorder = world.get_resource::<RunHandle>().map(|r| r.0.clone());
    let applied = backend_forces(world);
    let execution = execution_status(world);
    let mut query = world.query::<(
        Entity,
        &Transform,
        &Velocity,
        &Vehicle,
        &VehicleControl,
        &StepEvidence,
        Option<&PlayerVehicle>,
    )>();
    let samples: Vec<_> = query
        .iter(world)
        .map(|(entity, pose, velocity, car, control, evidence, player)| {
            let mut sample = completed_sample(
                (pose, velocity, car, control, evidence, player),
                (tick, time, mode),
                &durations,
            );
            sample.envelope = recorder.as_ref().map(|run| {
                let mut envelope = run.envelope();
                envelope.tick = tick;
                envelope.boundary = "completed_tick".into();
                envelope.actor_id = Some(sample.actor_id.clone());
                envelope
            });
            sample.backend_applied = applied.get(&entity).cloned();
            sample.body_part_id = body_part(world, evidence);
            if let Some(reason) = execution.get(&entity) {
                sample.integration_steps = 0;
                sample.evidence.outcome = (*reason).into();
            }
            (control.id.clone(), player.is_some(), sample)
        })
        .collect();
    world.resource_mut::<CompletedSteps>().0.clear();
    for (id, player, sample) in samples {
        if let Some(reason) = crate::vehicle::physics_invariant(&sample) {
            if let Some(run) = &recorder {
                run.fail(
                    "physics_invariant",
                    serde_json::json!({"reason":reason,"sample":sample}),
                );
            }
            world.write_message(bevy::app::AppExit::error());
        }
        if player && let Some(mut trace) = world.get_resource_mut::<PhysicsTrace>() {
            trace.record(sample.clone());
        } else if let Some(run) = &recorder {
            run.record_evidence(crate::diagnostics::EvidenceStream::Physics, tick, &sample);
        }
        world.resource_mut::<CompletedSteps>().0.insert(id, sample);
    }
}
/// Resolve only the graph actually bound to this chassis; never borrow the player's mask.
pub(crate) fn assembly_revision(
    entity: Entity,
    id: &VehicleId,
    assembly: Option<&Assembly>,
    runtime: Option<&AssemblyRuntime>,
) -> Option<(u64, u64)> {
    let assembly = assembly?;
    let runtime = runtime?;
    (assembly.vehicle == *id
        && runtime.bodies.get(id) == Some(&entity)
        && runtime.applied_revision == Some(assembly.revision))
    .then_some((assembly.revision, runtime.generation))
}

fn backend_forces(world: &mut World) -> BTreeMap<Entity, ForceSample> {
    world
        .query::<&RapierRigidBodySet>()
        .iter(world)
        .flat_map(|set| {
            set.bodies.iter().filter_map(|(handle, body)| {
                Some((
                    set.rigid_body_entity(handle)?,
                    ExternalForce {
                        force: body.user_force(),
                        torque: body.user_torque(),
                    }
                    .into(),
                ))
            })
        })
        .collect()
}
fn body_part(world: &World, evidence: &StepEvidence) -> Option<crate::assembly::PartId> {
    evidence.assembly_revision?;
    let assembly = world.get_resource::<Assembly>()?;
    assembly
        .parts
        .iter()
        .find(|part| {
            matches!(
                assembly.definitions[&part.definition].collision,
                crate::assembly::CollisionBinding::ResidualChassis
            )
        })
        .map(|p| p.id.clone())
}
#[derive(Resource, Default)]
pub(super) struct PublishedAssembly(Option<(VehicleId, u64, u64)>);
pub(super) fn domain_checkpoint(
    assembly: Option<Res<Assembly>>,
    runtime: Option<Res<AssemblyRuntime>>,
    run: Option<Res<RunHandle>>,
    query: Res<super::CurrentQueries>,
    mut published: ResMut<PublishedAssembly>,
) {
    let (Some(assembly), Some(runtime), Some(run)) = (assembly, runtime, run) else {
        return;
    };
    let Some(_) = runtime.bodies.get(&assembly.vehicle) else {
        return;
    };
    let key = (
        assembly.vehicle.clone(),
        assembly.revision,
        runtime.generation,
    );
    if runtime.applied_revision != Some(assembly.revision) || published.0.as_ref() == Some(&key) {
        return;
    }
    let mut envelope = run.envelope();
    envelope.tick = query.tick;
    envelope.boundary = "physics_prepared".into();
    envelope.actor_id = Some(assembly.vehicle.0.clone());
    run.event_envelope(
        "physics.assembly_checkpoint",
        serde_json::json!({
            "snapshot": crate::assembly::AssemblySnapshot::capture(&assembly, query.tick),
            "runtime_generation": runtime.generation,
        }),
        envelope,
    );
    published.0 = Some(key);
}

type CompletedState<'a> = (
    &'a Transform,
    &'a Velocity,
    &'a Vehicle,
    &'a VehicleControl,
    &'a StepEvidence,
    Option<&'a PlayerVehicle>,
);
fn completed_sample(
    state: CompletedState<'_>,
    clock: (u64, f32, QueryMode),
    durations: &PhaseDurations,
) -> PhysicsSample {
    let (pose, velocity, car, control, evidence, player) = state;
    let (tick, time, mode) = clock;
    let actor_id: String = if player.is_some() {
        "player-vehicle".into()
    } else {
        control.id.0.clone()
    };
    PhysicsSample {
        envelope: None,
        backend_applied: None,
        body_part_id: None,
        step: tick.saturating_sub(1),
        tick,
        actor_id,
        vehicle_id: control.id.clone(),
        body_id: format!("{}:chassis", control.id.0),
        boundary: "completed_tick",
        integration_steps: 1,
        query_mode: if mode == QueryMode::Synchronized {
            "synchronized"
        } else {
            "legacy_pre_sync"
        },
        time,
        car: control.spec.id.clone(),
        position: pose.translation.to_array(),
        rotation: pose.rotation.to_array(),
        linear_velocity: velocity.linear.to_array(),
        angular_velocity: velocity.angular.to_array(),
        grounded: car.grounded,
        wheel_lengths: car.wheel_lengths,
        wheel_loads: evidence.wheel_loads,
        steering: car.steering,
        speed: velocity.linear.dot(pose.rotation * Vec3::NEG_Z),
        slip: car.slip,
        rpm: car.rpm,
        gear: car.gear,
        reset_count: car.reset_count,
        input: evidence.input,
        ray_misses: evidence.ray_misses,
        evidence: evidence.clone(),
        phase_durations: durations.clone(),
    }
}

pub(crate) fn assert_completed(
    world: &World,
    vehicle: &VehicleId,
    grounded: usize,
    collider: Option<&str>,
    outcome: &str,
    min_forward_mps: f32,
) -> serde_json::Value {
    let Some(sample) = world.resource::<CompletedSteps>().0.get(vehicle) else {
        return serde_json::json!({"passed":false,"reason":"completed-state-unavailable"});
    };
    let clock = world.resource::<crate::runtime::SimulationClock>();
    let contacts = collider.is_none_or(|id| {
        sample
            .evidence
            .wheels
            .iter()
            .all(|w| w.collider_id.as_deref() == Some(id))
    });
    let force_applied = sample.backend_applied.as_ref().is_some_and(|backend| {
        backend.force == sample.evidence.total.force
            && backend.torque == sample.evidence.total.torque
    });
    let passed = sample.tick == clock.completed
        && sample.evidence.query_tick == clock.completed
        && sample.boundary == "completed_tick"
        && sample.integration_steps == 1
        && sample.grounded == grounded
        && sample.evidence.outcome == outcome
        && sample.speed >= min_forward_mps
        && contacts
        && force_applied;
    serde_json::json!({"passed":passed,"sample":sample,"force_applied":force_applied})
}

fn execution_status(world: &mut World) -> BTreeMap<Entity, &'static str> {
    let active: BTreeMap<_, _> = world
        .query::<(Entity, &RapierConfiguration)>()
        .iter(world)
        .map(|(entity, config)| (entity, config.physics_pipeline_active))
        .collect();
    world
        .query_filtered::<(Entity, Option<&RapierContextEntityLink>), With<Vehicle>>()
        .iter(world)
        .filter_map(
            |(entity, link)| match link.and_then(|link| active.get(&link.0)) {
                Some(true) => None,
                Some(false) => Some((entity, "physics_pipeline_inactive")),
                None => Some((entity, "unavailable_context")),
            },
        )
        .collect()
}
