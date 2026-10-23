//! Scenario and world-streaming edits use the same fixed-boundary command dispatch.
use super::ColliderIdentity;
use crate::{assembly::VehicleId, controls::VehicleControl};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum PhysicsEdit {
    InsertRoad {
        id: String,
        position: [f32; 3],
        half_extents: [f32; 3],
        #[serde(default)]
        parent: Option<String>,
    },
    RemoveCollider {
        id: String,
    },
    RelocateVehicle {
        vehicle: VehicleId,
        position: [f32; 3],
        rotation_y: f32,
    },
    ReparentCollider {
        id: String,
        parent: String,
        position: [f32; 3],
    },
}
impl PhysicsEdit {
    pub fn validate(&self) -> Result<(), String> {
        let valid = match self {
            Self::InsertRoad {
                id,
                position,
                half_extents,
                parent: _,
            } => {
                !id.is_empty()
                    && finite(position)
                    && finite(half_extents)
                    && half_extents.iter().all(|v| *v > 0.)
            }
            Self::RemoveCollider { id } => !id.is_empty(),
            Self::RelocateVehicle {
                vehicle,
                position,
                rotation_y,
            } => !vehicle.0.is_empty() && finite(position) && rotation_y.is_finite(),
            Self::ReparentCollider {
                id,
                parent,
                position,
            } => !id.is_empty() && !parent.is_empty() && id != parent && finite(position),
        };
        valid.then_some(()).ok_or("invalid-physics-edit".into())
    }
}
fn finite(position: &[f32; 3]) -> bool {
    position.iter().all(|v| v.is_finite())
}
fn collider(world: &mut World, id: &str) -> Result<Entity, &'static str> {
    world
        .query::<(Entity, &ColliderIdentity)>()
        .iter(world)
        .find(|(_, binding)| binding.0 == id)
        .map(|(entity, _)| entity)
        .ok_or("unknown-collider")
}
pub(crate) fn apply(world: &mut World, edit: &PhysicsEdit) -> Result<&'static str, &'static str> {
    match edit {
        PhysicsEdit::InsertRoad {
            id,
            position,
            half_extents,
            parent,
        } => {
            if collider(world, id).is_ok() {
                return Err("duplicate-collider-id");
            }
            let parent = parent.as_ref().map(|id| collider(world, id)).transpose()?;
            if parent.is_some_and(|entity| world.get::<RigidBody>(entity).is_none()) {
                return Err("parent-requires-body");
            }
            let mut entity = world.spawn((
                ColliderIdentity(id.clone()),
                Collider::cuboid(half_extents[0], half_extents[1], half_extents[2]),
                Transform::from_translation(Vec3::from_array(*position)),
            ));
            if let Some(parent) = parent {
                entity.insert(ChildOf(parent));
            } else {
                entity.insert(RigidBody::Fixed);
            }
            Ok("road-inserted")
        }
        PhysicsEdit::RemoveCollider { id } => {
            let entity = collider(world, id)?;
            // Removing geometry must not recursively destroy a vehicle or durable parts.
            world.entity_mut(entity).remove::<Collider>();
            world.entity_mut(entity).remove::<ColliderIdentity>();
            Ok("collider-removed")
        }
        PhysicsEdit::RelocateVehicle {
            vehicle,
            position,
            rotation_y,
        } => {
            let entity = world
                .query::<(Entity, &VehicleControl)>()
                .iter(world)
                .find(|(_, control)| control.id == *vehicle)
                .map(|(e, _)| e)
                .ok_or("unknown-vehicle")?;
            let pose = Transform::from_translation(Vec3::from_array(*position))
                .with_rotation(Quat::from_rotation_y(*rotation_y));
            world
                .entity_mut(entity)
                .insert((pose, Velocity::zero(), ExternalForce::default()));
            Ok("vehicle-relocated")
        }
        PhysicsEdit::ReparentCollider {
            id,
            parent,
            position,
        } => {
            let child = collider(world, id)?;
            let parent = collider(world, parent)?;
            if world.get::<RigidBody>(parent).is_none() {
                return Err("parent-requires-body");
            }
            if world.get::<VehicleControl>(child).is_some() {
                return Err("vehicle-root-cannot-reparent");
            }
            if world.get::<RigidBody>(child).is_some() {
                return Err("collider-root-requires-body-removal");
            }
            world.entity_mut(child).insert((
                ChildOf(parent),
                Transform::from_translation(Vec3::from_array(*position)),
            ));
            Ok("collider-reparented")
        }
    }
}
