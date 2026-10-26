//! Per-chassis configuration and exclusive controller state behind production ingress.
use crate::{
    assembly::VehicleId,
    roster::{CarSpec, Fleet},
    runtime::{ControllerOwner, ProductionCommand, QueuedCommand},
    vehicle::DriveInput,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component)]
pub struct PlayerVehicle;
#[derive(Component)]
pub struct VehicleControl {
    pub id: VehicleId,
    pub spec: CarSpec,
    pub owner: ControllerOwner,
    pub input: DriveInput,
    pub pending_reset: Option<QueuedCommand>,
}
impl VehicleControl {
    pub fn new(id: VehicleId, spec: CarSpec, owner: ControllerOwner) -> Self {
        Self {
            id,
            spec,
            owner,
            input: DriveInput::default(),
            pending_reset: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum VehicleCommand {
    Drive {
        throttle: f32,
        brake: f32,
        steer: f32,
        #[serde(default)]
        handbrake: bool,
    },
    Reset,
    AssignController {
        controller: ControllerOwner,
    },
}
impl VehicleCommand {
    pub fn validate(&self) -> Result<(), String> {
        if let Self::Drive {
            throttle,
            brake,
            steer,
            handbrake,
        } = self
        {
            ProductionCommand::Drive {
                throttle: *throttle,
                brake: *brake,
                steer: *steer,
                handbrake: *handbrake,
            }
            .validate()?;
        }
        Ok(())
    }
}

pub fn spawn(
    world: &mut World,
    id: VehicleId,
    car: &str,
    position: [f32; 3],
    owner: ControllerOwner,
) -> Result<(), &'static str> {
    // The selected player's legacy evidence actor must remain globally unique.
    if id.0 == "player-vehicle" {
        return Err("reserved-vehicle-id");
    }
    if id.0.is_empty()
        || world
            .query::<&VehicleControl>()
            .iter(world)
            .any(|c| c.id == id)
    {
        return Err("duplicate-or-empty-vehicle-id");
    }
    if !world
        .resource::<crate::config::AccessContext>()
        .permits(car)
    {
        return Err("not-owned");
    }
    let spec = world
        .resource::<Fleet>()
        .cars
        .iter()
        .find(|s| s.id == car)
        .cloned()
        .ok_or("unknown-car")?;
    // Authored roads share the collider namespace; a clash would misroute physics edits.
    let collider_id = format!("{}:chassis", id.0);
    if crate::physics::commands::collider(world, &collider_id).is_ok() {
        return Err("duplicate-collider-id");
    }
    let pose = Transform::from_translation(Vec3::from_array(position));
    world
        .spawn(crate::vehicle::chassis_bundle(&spec, pose))
        .insert(crate::physics::ColliderIdentity(collider_id))
        .insert(VehicleControl::new(id, spec, owner));
    Ok(())
}

pub fn command(
    world: &mut World,
    queued: &QueuedCommand,
    target: &VehicleId,
    action: &VehicleCommand,
) {
    let target_entity = world
        .query::<(Entity, &VehicleControl)>()
        .iter(world)
        .find(|(_, c)| &c.id == target)
        .map(|(e, _)| e);
    let Some(entity) = target_entity else {
        crate::runtime::outcome(world, queued, "rejected", "unknown-vehicle");
        return;
    };
    // The existing player ingress remains the compatibility bridge, including replay release.
    if world.get::<PlayerVehicle>(entity).is_some() {
        if matches!(action, VehicleCommand::AssignController { .. })
            && world.resource::<crate::runtime::ControllerAssignment>().0 != queued.source
        {
            crate::runtime::outcome(world, queued, "rejected", "controller-owner-active");
            return;
        }
        let command = match action {
            VehicleCommand::Drive {
                throttle,
                brake,
                steer,
                handbrake,
            } => ProductionCommand::Drive {
                throttle: *throttle,
                brake: *brake,
                steer: *steer,
                handbrake: *handbrake,
            },
            VehicleCommand::Reset => ProductionCommand::Reset,
            VehicleCommand::AssignController { controller } => {
                ProductionCommand::AssignController {
                    controller: *controller,
                }
            }
        };
        crate::runtime::apply_targeted_player(
            world,
            QueuedCommand {
                intent: Some(queued.command.clone()),
                command,
                ..queued.clone()
            },
        );
        return;
    }
    let paused = world.resource::<crate::runtime::Session>().paused;
    let mut control = world
        .get_mut::<VehicleControl>(entity)
        .expect("target checked");
    let (disposition, reason) = if queued.source != control.owner {
        ("rejected", "controller-owner-active")
    } else {
        match action {
            VehicleCommand::Drive {
                throttle,
                brake,
                steer,
                handbrake,
            } => {
                let reset = control.input.reset;
                control.input = if paused {
                    DriveInput::default()
                } else {
                    DriveInput {
                        throttle: *throttle,
                        brake: *brake,
                        steer: *steer,
                        handbrake: *handbrake,
                        reset,
                    }
                };
                ("applied", "held-controls-updated")
            }
            VehicleCommand::Reset if control.pending_reset.is_some() => {
                ("skipped", "reset-already-pending")
            }
            VehicleCommand::Reset => {
                control.input.reset = true;
                control.pending_reset = Some(queued.clone());
                ("applied", "reset-queued")
            }
            VehicleCommand::AssignController { .. } if control.pending_reset.is_some() => {
                ("rejected", "reset-pending")
            }
            VehicleCommand::AssignController { controller } => {
                control.owner = *controller;
                control.input = DriveInput::default();
                ("applied", "controller-assigned")
            }
        }
    };
    crate::runtime::outcome(world, queued, disposition, reason);
}

/// Observe the same chassis and actuator state consumed by the production solver.
pub fn observe(
    world: &mut World,
    target: &VehicleId,
    car: &str,
    owner: ControllerOwner,
    resets: u32,
    speed_bounds: [f32; 2],
) -> serde_json::Value {
    use bevy_rapier3d::prelude::Velocity;
    let found = world
        .query::<(
            &VehicleControl,
            &Transform,
            &Velocity,
            &crate::vehicle::Vehicle,
        )>()
        .iter(world)
        .find(|(control, _, _, _)| &control.id == target);
    let Some((control, pose, velocity, vehicle)) = found else {
        return serde_json::json!({"passed":false,"reason":"unknown-vehicle"});
    };
    let speed = velocity.linear.dot(pose.rotation * Vec3::NEG_Z);
    serde_json::json!({"passed":control.spec.id == car && control.owner == owner && vehicle.reset_count == resets && speed >= speed_bounds[0] && speed <= speed_bounds[1],"vehicle_id":target,"car":control.spec.id,"controller":control.owner,"reset_count":vehicle.reset_count,"forward_mps":speed,"grounded":vehicle.grounded,"wheelbase":control.spec.wheelbase(),"radii":control.spec.radii,"held":{"throttle":control.input.throttle,"brake":control.input.brake,"steer":control.input.steer}})
}
