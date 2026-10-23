//! Current-pose queries own their index; the production pipeline's queues are untouched.
use super::ColliderIdentity;
use bevy::prelude::*;
use bevy_rapier3d::{prelude::*, rapier::prelude as raw};
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct CurrentQueries {
    colliders: raw::ColliderSet,
    bodies: raw::RigidBodySet,
    broad: raw::BroadPhaseBvh,
    bindings: HashMap<raw::ColliderHandle, Binding>,
    pub tick: u64,
    pub revision: u64,
}
struct Binding {
    entity: Entity,
    parent: Option<Entity>,
    identity: Option<String>,
}
pub struct WheelHit {
    pub entity: Entity,
    pub identity: Option<String>,
    pub distance: f32,
    pub normal: Vec3,
}
impl CurrentQueries {
    pub fn cast(
        &self,
        origin: Vec3,
        direction: Vec3,
        range: f32,
        exclude: Entity,
    ) -> Option<WheelHit> {
        let predicate = |handle, _: &raw::Collider| {
            let binding = &self.bindings[&handle];
            binding.entity != exclude && binding.parent != Some(exclude)
        };
        let pipeline = self.broad.as_query_pipeline(
            &bevy_rapier3d::rapier::parry::query::DefaultQueryDispatcher,
            &self.bodies,
            &self.colliders,
            raw::QueryFilter::default()
                .exclude_sensors()
                .predicate(&predicate),
        );
        let (handle, hit) =
            pipeline.cast_ray_and_get_normal(&raw::Ray::new(origin, direction), range, true)?;
        let binding = &self.bindings[&handle];
        Some(WheelHit {
            entity: binding.entity,
            identity: binding.identity.clone(),
            distance: hit.time_of_impact,
            normal: hit.normal,
        })
    }
}
pub(super) fn publish(
    clock: Option<Res<crate::runtime::SimulationClock>>,
    contexts: Query<(&RapierContextColliders, &RapierRigidBodySet)>,
    identities: Query<&ColliderIdentity>,
    mut current: ResMut<CurrentQueries>,
) {
    let revision = current.revision + 1;
    let tick = clock.map_or(revision, |clock| clock.current);
    // Rebuild only a disposable query database. No take_modified/take_removed calls
    // on production sets and no collision solver or integration is run here.
    *current = CurrentQueries {
        tick,
        revision,
        ..default()
    };
    for (colliders, bodies) in &contexts {
        for (handle, collider) in colliders.colliders.iter_enabled() {
            let entity = colliders.collider_entity(handle).expect("backend entity");
            let parent = collider.parent().and_then(|h| bodies.bodies.get(h));
            if parent.is_some_and(|body| !body.is_enabled()) {
                continue;
            }
            let pose = parent.map_or(*collider.position(), |body| {
                body.position() * collider.position_wrt_parent().expect("parent pose")
            });
            let parent_entity = collider.parent().and_then(|h| bodies.rigid_body_entity(h));
            let mut snapshot = collider.clone();
            snapshot.set_position(pose);
            let snapshot_handle = current.colliders.insert(snapshot);
            current.bindings.insert(
                snapshot_handle,
                Binding {
                    entity,
                    parent: parent_entity,
                    identity: identities.get(entity).ok().map(|id| id.0.clone()),
                },
            );
        }
    }
    let params = raw::IntegrationParameters::default();
    let current = &mut *current;
    for (handle, collider) in current.colliders.iter_enabled() {
        current
            .broad
            .set_aabb(&params, handle, collider.compute_aabb());
    }
}
