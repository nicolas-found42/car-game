//! Rapier 0.37 does not update same-context collider parents from ChildOf changes.
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
#[derive(Component, Clone, Copy)]
struct ParentBinding(Option<Entity>);
pub(super) fn reconcile(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        Option<&ChildOf>,
        Option<&RapierColliderHandle>,
        Option<&ParentBinding>,
    ), (With<Collider>, Without<RigidBody>)>();
    let changes: Vec<_> = query
        .iter(world)
        .filter_map(|(entity, child_of, handle, old)| {
            let mut parent = child_of.map(ChildOf::parent);
            while let Some(candidate) = parent {
                if world.get::<RigidBody>(candidate).is_some() {
                    break;
                }
                parent = world.get::<ChildOf>(candidate).map(ChildOf::parent);
            }
            (old.is_none_or(|binding| binding.0 != parent)).then_some((
                entity,
                parent,
                old.is_some() && handle.is_some(),
            ))
        })
        .collect();
    for (entity, parent, invalidate) in changes {
        let mut target = world.entity_mut(entity);
        if invalidate {
            // Normal SyncBackend owns removal, initialization and mass-event queues.
            // Invalidating this derived handle preserves the entity and authored ID.
            target.remove::<RapierColliderHandle>();
        }
        target.insert(ParentBinding(parent));
    }
}
