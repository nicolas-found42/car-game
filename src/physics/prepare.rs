//! Controls and pose mutations are committed before Rapier synchronizes transforms.
use super::{ColliderIdentity, CurrentQueries, StepEvidence, WheelSample};
use crate::{
    controls::{PlayerVehicle, VehicleControl},
    diagnostics::{InputSample, PhysicsTrace, RunHandle},
    runtime::{ControllerAssignment, PendingReset, QueuedCommand, SimulationClock},
    vehicle::{DriveInput, Vehicle, recovery_pose},
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use serde_json::json;

pub(crate) fn prepare(world: &mut World) {
    let mut legacy = *world.resource::<DriveInput>();
    world.resource_mut::<DriveInput>().reset = false;
    let spec = world.resource::<crate::roster::CarSpec>().clone();
    let owner = world.get_resource::<ControllerAssignment>().map(|a| a.0);
    let tick = world.get_resource::<SimulationClock>().map_or_else(
        || world.resource::<CurrentQueries>().revision + 1,
        |c| c.current,
    );
    let run = world.get_resource::<RunHandle>().map(|r| r.0.clone());
    let mut pending = world
        .get_resource_mut::<PendingReset>()
        .and_then(|mut p| p.0.take());
    let mut input_record = None;
    let mut query = world.query::<(
        &mut Transform,
        &mut Velocity,
        &mut ExternalForce,
        &mut Vehicle,
        &mut VehicleControl,
        &mut StepEvidence,
        Option<&PlayerVehicle>,
        Option<&mut ColliderIdentity>,
    )>();
    for (mut pose, mut velocity, mut force, mut car, mut control, mut evidence, player, identity) in
        query.iter_mut(world)
    {
        if player.is_some() {
            control.spec = spec.clone();
            control.input = legacy;
            control.owner = owner.unwrap_or(control.owner);
        }
        if let Some(mut identity) = identity {
            identity.0 = format!("{}:chassis", control.id.0);
        }
        let reset = std::mem::take(&mut control.input.reset);
        let input = InputSample {
            step: tick.saturating_sub(1),
            tick,
            controller: Some(control.owner),
            throttle: control.input.throttle,
            brake: control.input.brake,
            steer: control.input.steer,
            handbrake: control.input.handbrake,
            reset,
        };
        if player.is_some() {
            input_record = Some(input);
            legacy.reset = false;
        }
        *evidence = StepEvidence {
            input,
            outcome: "integrated".into(),
            ..default()
        };
        if let Some(run) = &run {
            record_controls(run, &control, input, player.is_some());
        }
        if reset || pose.translation.y < -12.0 {
            let transition = reset_pose(
                &mut pose,
                &mut velocity,
                &mut force,
                &mut car,
                &mut evidence,
            );
            let request = if player.is_some() {
                pending.take()
            } else {
                control.pending_reset.take()
            };
            if let Some(run) = &run {
                record_reset(
                    run,
                    &control,
                    tick,
                    request,
                    reset,
                    transition,
                    if player.is_some() {
                        "player-vehicle"
                    } else {
                        &control.id.0
                    },
                );
            }
        }
        car.previous = *pose;
        evidence.pre_step_position = pose.translation.to_array();
        evidence.pre_step_linear_velocity = velocity.linear.to_array();
    }
    if let Some(input) = input_record
        && let Some(mut trace) = world.get_resource_mut::<PhysicsTrace>()
    {
        trace.record_input(input);
    }
    if let Some(mut resource) = world.get_resource_mut::<PendingReset>() {
        resource.0 = pending;
    }
}
fn record_reset(
    run: &crate::diagnostics::RunRecorder,
    control: &VehicleControl,
    tick: u64,
    request: Option<QueuedCommand>,
    requested: bool,
    mut fields: serde_json::Value,
    actor_id: &str,
) {
    let mut envelope = run.envelope();
    envelope.tick = tick;
    envelope.boundary = "applied_tick".into();
    envelope.actor_id = Some(actor_id.into());
    fields["vehicle_id"] = json!(control.id);
    fields["disposition"] = json!("applied");
    fields["reason"] = json!(if requested {
        "requested-reset"
    } else {
        "fell-below-world"
    });
    fields["requested_command"] = json!(
        request
            .as_ref()
            .map(|r| r.intent.as_ref().unwrap_or(&r.command))
    );
    if let Some(request) = request {
        envelope.command_id = request.envelope.command_id;
        envelope.correlation_id = request.envelope.correlation_id;
        envelope.span = request.envelope.span;
    }
    run.event_envelope("vehicle.reset", fields, envelope);
}

fn reset_pose(
    pose: &mut Transform,
    velocity: &mut Velocity,
    force: &mut ExternalForce,
    car: &mut Vehicle,
    evidence: &mut StepEvidence,
) -> serde_json::Value {
    let before = json!({"position":pose.translation.to_array(),"rotation":pose.rotation.to_array(),"linear_velocity":velocity.linear.to_array(),"angular_velocity":velocity.angular.to_array(),"reset_count":car.reset_count});
    *pose = recovery_pose(pose.translation);
    *velocity = Velocity::zero();
    *force = ExternalForce::default();
    let count = car.reset_count + 1;
    *car = Vehicle::default();
    car.reset_count = count;
    evidence.outcome = "reset".into();
    evidence.ray_misses = 4;
    evidence.wheels = std::array::from_fn(|corner| WheelSample {
        corner,
        availability: "reset_no_mechanics".into(),
        ..default()
    });
    json!({"before":before,"after":{"position":pose.translation.to_array(),"rotation":pose.rotation.to_array(),"linear_velocity":velocity.linear.to_array(),"angular_velocity":velocity.angular.to_array(),"reset_count":count}})
}

fn record_controls(
    run: &crate::diagnostics::RunRecorder,
    control: &VehicleControl,
    input: InputSample,
    player: bool,
) {
    let mut envelope = run.envelope();
    envelope.tick = input.tick;
    envelope.boundary = "fixed_consumption".into();
    envelope.actor_id = Some(if player {
        "player-vehicle".into()
    } else {
        control.id.0.clone()
    });
    run.event_envelope("controls.effective",json!({"vehicle_id":control.id,"throttle":input.throttle,"brake":input.brake,"steer":input.steer,"handbrake":input.handbrake,"reset":input.reset,"controller":control.owner}),envelope);
}
