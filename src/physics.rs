//! One synchronized query/force boundary around the existing Rapier integration.
pub mod commands;
mod hierarchy;
pub(crate) mod prepare;
mod query;
pub use commands::PhysicsEdit;
pub(crate) mod recording;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
pub use query::CurrentQueries;
pub use recording::{CompletedSteps, ForceSample, PhaseDurations, StepEvidence, WheelSample};
use std::time::Instant;

/// Authored collider binding, independent of disposable ECS and Rapier handles.
#[derive(Component, Clone, Debug)]
pub struct ColliderIdentity(pub String);
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum QueryMode {
    LegacyPreSync,
    #[default]
    Synchronized,
}
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PhysicsPhase {
    Prepare,
    Query,
    Mechanics,
    Bridge,
    Observe,
}
#[derive(Resource, Default)]
pub(crate) struct PhaseClock {
    start: Option<Instant>,
    durations: PhaseDurations,
}
pub struct PhysicsAdapterPlugin;
impl Plugin for PhysicsAdapterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QueryMode>()
            .init_resource::<CurrentQueries>()
            .init_resource::<CompletedSteps>()
            .init_resource::<PhaseClock>()
            .init_resource::<recording::PublishedAssembly>()
            .configure_sets(
                FixedUpdate,
                PhysicsPhase::Prepare.before(PhysicsSet::SyncBackend),
            )
            .configure_sets(
                FixedUpdate,
                (
                    PhysicsSet::SyncBackend,
                    PhysicsPhase::Query,
                    PhysicsPhase::Mechanics,
                    PhysicsPhase::Bridge,
                    PhysicsSet::StepSimulation,
                    PhysicsSet::Writeback,
                    PhysicsPhase::Observe,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (prepare::prepare, hierarchy::reconcile)
                    .chain()
                    .in_set(PhysicsPhase::Prepare),
            )
            .add_systems(
                FixedUpdate,
                begin_sync
                    .after(PhysicsPhase::Prepare)
                    .before(PhysicsSet::SyncBackend),
            )
            .add_systems(
                FixedUpdate,
                (finish_sync, query::publish, recording::domain_checkpoint)
                    .chain()
                    .in_set(PhysicsPhase::Query),
            )
            .add_systems(
                FixedUpdate,
                begin_mechanics
                    .after(PhysicsPhase::Query)
                    .before(PhysicsPhase::Mechanics),
            )
            .add_systems(FixedUpdate, bridge.in_set(PhysicsPhase::Bridge))
            .add_systems(
                FixedUpdate,
                begin_step
                    .after(PhysicsPhase::Bridge)
                    .before(PhysicsSet::StepSimulation),
            )
            .add_systems(
                FixedUpdate,
                finish_step
                    .after(PhysicsSet::StepSimulation)
                    .before(PhysicsSet::Writeback),
            )
            .add_systems(
                FixedUpdate,
                recording::observe.in_set(PhysicsPhase::Observe),
            );
    }
}
fn begin_sync(mut clock: ResMut<PhaseClock>) {
    clock.durations = PhaseDurations::default();
    clock.start = Some(Instant::now());
}
fn finish_sync(mut clock: ResMut<PhaseClock>) {
    clock.durations.sync_ns = clock
        .start
        .replace(Instant::now())
        .map(|s| s.elapsed().as_nanos() as u64);
}
fn begin_mechanics(mut clock: ResMut<PhaseClock>) {
    clock.durations.query_ns = clock
        .start
        .replace(Instant::now())
        .map(|s| s.elapsed().as_nanos() as u64);
}
fn begin_step(mut clock: ResMut<PhaseClock>) {
    clock.start = Some(Instant::now());
}
fn finish_step(mut clock: ResMut<PhaseClock>) {
    clock.durations.step_ns = clock
        .start
        .replace(Instant::now())
        .map(|s| s.elapsed().as_nanos() as u64);
}
fn bridge(
    mode: Res<QueryMode>,
    mut clock: ResMut<PhaseClock>,
    cars: Query<
        (
            &RapierRigidBodyHandle,
            &RapierContextEntityLink,
            &ExternalForce,
        ),
        With<crate::vehicle::Vehicle>,
    >,
    mut contexts: Query<&mut RapierRigidBodySet>,
) {
    clock.durations.mechanics_ns = clock
        .start
        .replace(Instant::now())
        .map(|s| s.elapsed().as_nanos() as u64);
    if *mode == QueryMode::Synchronized {
        for (handle, link, force) in &cars {
            let mut bodies = contexts.get_mut(link.0).expect("vehicle context");
            let body = bodies
                .bodies
                .get_mut(handle.0)
                .expect("vehicle backend body");
            body.reset_forces(true);
            body.reset_torques(true);
            body.add_force(force.force, true);
            body.add_torque(force.torque, true);
        }
    }
    clock.durations.bridge_ns = clock.start.take().map(|s| s.elapsed().as_nanos() as u64);
}
