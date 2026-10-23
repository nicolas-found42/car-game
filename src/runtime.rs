//! Production command ingress and dispatch, shared by live input and scenarios.
use crate::{
    config::AccessContext,
    diagnostics::{ReplayInput, RunHandle, RunRecorder},
    roster::{CarSpec, Fleet},
    telemetry::{Envelope, EvidenceClock, SpanContext},
    vehicle::{DriveInput, Vehicle, chassis_collider, recovery_pose},
};
use bevy::{app::AppExit, prelude::*};
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::VecDeque, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerOwner {
    #[default]
    LegacyComposite,
    Replay,
    Scenario,
}
impl ControllerOwner {
    pub fn name(self) -> &'static str {
        match self {
            Self::LegacyComposite => "legacy_composite",
            Self::Replay => "replay",
            Self::Scenario => "scenario",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductionCommand {
    Drive {
        throttle: f32,
        brake: f32,
        steer: f32,
        #[serde(default)]
        handbrake: bool,
    },
    Vehicle {
        target: crate::assembly::VehicleId,
        command: crate::controls::VehicleCommand,
    },
    SpawnVehicle {
        id: crate::assembly::VehicleId,
        car: String,
        position: [f32; 3],
        controller: ControllerOwner,
    },
    InspectAssembly,
    SnapshotAssembly {
        key: String,
    },
    RestoreAssembly {
        expected_revision: u64,
        source: crate::assembly::SnapshotSource,
    },
    AssemblyTransaction {
        transaction: crate::assembly::Transaction,
    },
    Reset,
    SelectCar {
        car: String,
        #[serde(default)]
        reset_to_spawn: bool,
    },
    CycleCar {
        direction: i32,
    },
    SetPaused {
        paused: bool,
    },
    AssignController {
        controller: ControllerOwner,
    },
    ToggleMute,
    Quit,
}
impl ProductionCommand {
    pub fn validate(&self) -> Result<(), String> {
        if let Self::Vehicle { target, command } = self {
            if target.0.is_empty() {
                return Err("empty-vehicle-id".into());
            }
            command.validate()?;
        }
        if let Self::SpawnVehicle { id, position, .. } = self
            && (id.0.is_empty() || position.iter().any(|v| !v.is_finite()))
        {
            return Err("invalid-vehicle-spawn".into());
        }
        if let Self::Drive {
            throttle,
            brake,
            steer,
            ..
        } = self
            && (!throttle.is_finite()
                || !brake.is_finite()
                || !steer.is_finite()
                || !(0.0..=1.0).contains(throttle)
                || !(0.0..=1.0).contains(brake)
                || !(-1.0..=1.0).contains(steer))
        {
            return Err("invalid controls: expected finite normalized throttle/brake/steer".into());
        }
        if let Self::CycleCar { direction } = self
            && ![-1, 1].contains(direction)
        {
            return Err("cycle direction must be -1 or 1".into());
        }
        Ok(())
    }
}
#[derive(Resource, Default)]
pub struct SimulationClock {
    pub current: u64,
    pub completed: u64,
    pub render_frame: u64,
}
#[derive(Resource, Default)]
pub struct ControllerAssignment(pub ControllerOwner);
#[derive(Clone)]
pub struct QueuedCommand {
    /// Original ingress when a targeted command is normalized for legacy dispatch.
    pub intent: Option<ProductionCommand>,
    pub command: ProductionCommand,
    pub source: ControllerOwner,
    pub envelope: Envelope,
}
#[derive(Resource, Default)]
pub struct CommandBus {
    pub recorder: Option<Arc<RunRecorder>>,
    clock: EvidenceClock,
    ingress: VecDeque<QueuedCommand>,
    fixed: VecDeque<QueuedCommand>,
}
impl CommandBus {
    pub(crate) fn has_pending(&self) -> bool {
        !self.ingress.is_empty() || !self.fixed.is_empty()
    }
    pub fn submit(&mut self, source: ControllerOwner, command: ProductionCommand) -> String {
        self.submit_with_span(source, command, SpanContext::default())
    }
    pub fn submit_with_span(
        &mut self,
        source: ControllerOwner,
        command: ProductionCommand,
        span: SpanContext,
    ) -> String {
        let mut envelope = self
            .recorder
            .as_ref()
            .map_or_else(|| self.clock.envelope("unrecorded"), |run| run.envelope());
        let id = self.clock.next_command(&envelope.run_id);
        envelope.actor_id = Some(match &command {
            ProductionCommand::Vehicle { target, .. } => target.0.clone(),
            ProductionCommand::SpawnVehicle { id, .. } => id.0.clone(),
            _ => "player-vehicle".into(),
        });
        envelope.command_id = Some(id.clone());
        envelope.correlation_id = Some(id.clone());
        envelope.span = span;
        if let Some(run) = &self.recorder {
            run.command(
                &envelope,
                &serde_json::to_value(&command).expect("command"),
                source.name(),
            );
        }
        self.ingress.push_back(QueuedCommand {
            intent: None,
            command,
            source,
            envelope,
        });
        id
    }
}
#[derive(Resource, Default)]
pub struct PendingReset(pub Option<QueuedCommand>);
#[derive(Resource, Default)]
struct ReplayReleasePending(bool);
#[derive(Resource, Default)]
pub struct Session {
    pub paused: bool,
    pub muted: bool,
}
impl Session {
    pub fn from_env() -> Self {
        Self {
            muted: std::env::var("CAR_GAME_MUTE")
                .map(|s| matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false),
            paused: false,
        }
    }
}
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum CommandSet {
    BeginTick,
    Dispatch,
    CompleteTick,
}
#[derive(Message)]
pub struct CommandOutcome {
    pub disposition: String,
    pub reason: String,
}
pub struct RuntimePlugin;
impl Plugin for RuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CommandOutcome>()
            .init_resource::<SimulationClock>()
            .init_resource::<CommandBus>()
            .init_resource::<ControllerAssignment>()
            .init_resource::<PendingReset>()
            .init_resource::<ReplayReleasePending>()
            .init_resource::<AccessContext>()
            .init_resource::<Session>()
            .add_systems(Startup, assign_initial)
            .add_systems(First, begin_frame)
            .add_systems(
                Last,
                (crate::diagnostics::poll_lifecycle, flush_on_exit).chain(),
            )
            .configure_sets(
                FixedFirst,
                (CommandSet::BeginTick, CommandSet::Dispatch).chain(),
            )
            .add_systems(FixedFirst, begin_tick.in_set(CommandSet::BeginTick))
            .add_systems(
                FixedFirst,
                (replay_commands, dispatch_fixed)
                    .chain()
                    .in_set(CommandSet::Dispatch),
            )
            .add_systems(FixedLast, complete_tick.in_set(CommandSet::CompleteTick))
            .add_systems(FixedLast, release_replay.after(CommandSet::CompleteTick));
    }
}
fn begin_frame(
    mut clock: ResMut<SimulationClock>,
    bus: Res<CommandBus>,
    run: Option<Res<RunHandle>>,
) {
    clock.render_frame += 1;
    bus.clock.frame(clock.render_frame);
    if let Some(run) = run {
        run.evidence_clock.frame(clock.render_frame);
    }
}
fn begin_tick(mut clock: ResMut<SimulationClock>) {
    clock.current = clock.completed + 1;
}
fn complete_tick(
    mut clock: ResMut<SimulationClock>,
    bus: Res<CommandBus>,
    run: Option<Res<RunHandle>>,
    spec: Res<CarSpec>,
    cars: Query<(
        &Transform,
        &Velocity,
        &Vehicle,
        &crate::controls::VehicleControl,
        Option<&crate::controls::PlayerVehicle>,
    )>,
) {
    clock.completed = clock.current;
    bus.clock.completed(clock.completed);
    if let Some(run) = run {
        run.evidence_clock.completed(clock.completed);
        let mut completed = run.envelope();
        completed.boundary = "completed_tick".into();
        run.event_envelope(
            "simulation.tick_completed",
            json!({"vehicle_count":cars.iter().count()}),
            completed,
        );
        for (pose, velocity, car, control, player) in &cars {
            let mut envelope = run.envelope();
            envelope.actor_id = Some(if player.is_some() {
                "player-vehicle".into()
            } else {
                control.id.0.clone()
            });
            envelope.boundary = "completed_tick".into();
            run.event_envelope("simulation.completed", json!({"car":if player.is_some() { &spec.id } else { &control.spec.id },"vehicle_id":control.id,"position":pose.translation.to_array(),"linear_velocity":velocity.linear.to_array(),"reset_count":car.reset_count}), envelope);
        }
        run.completed_tick(clock.completed);
    }
}
/// Update ingress stays live while virtual time is paused; physical actions wait for FixedFirst.
pub fn dispatch_update(world: &mut World) {
    let commands: Vec<_> = world
        .resource_mut::<CommandBus>()
        .ingress
        .drain(..)
        .collect();
    for command in commands {
        let physical = matches!(
            command.command,
            ProductionCommand::Reset
                | ProductionCommand::SnapshotAssembly { .. }
                | ProductionCommand::RestoreAssembly { .. }
                | ProductionCommand::Vehicle { .. }
                | ProductionCommand::SpawnVehicle { .. }
                | ProductionCommand::InspectAssembly
                | ProductionCommand::AssemblyTransaction { .. }
                | ProductionCommand::SelectCar { .. }
                | ProductionCommand::CycleCar { .. }
        );
        let ordered = !world.resource::<CommandBus>().fixed.is_empty()
            && matches!(
                command.command,
                ProductionCommand::Drive { .. } | ProductionCommand::AssignController { .. }
            );
        if physical || ordered {
            world.resource_mut::<CommandBus>().fixed.push_back(command);
        } else {
            apply_immediate(world, command);
        }
    }
}
fn dispatch_fixed(world: &mut World) {
    dispatch_update(world);
    let commands: Vec<_> = world.resource_mut::<CommandBus>().fixed.drain(..).collect();
    for command in commands {
        if let Some(reason) = rejection(world, &command) {
            outcome(world, &command, "rejected", reason);
            continue;
        }
        match &command.command {
            ProductionCommand::SnapshotAssembly { key } => {
                crate::assembly::snapshot_command(world, &command, key)
            }
            ProductionCommand::RestoreAssembly {
                source,
                expected_revision,
            } => crate::assembly::restore_command(world, &command, source, *expected_revision),
            ProductionCommand::Vehicle {
                target,
                command: action,
            } => crate::controls::command(world, &command, target, action),
            ProductionCommand::SpawnVehicle {
                id,
                car,
                position,
                controller,
            } => {
                let result = crate::controls::spawn(world, id.clone(), car, *position, *controller);
                let (disposition, reason) = match result {
                    Ok(()) => ("applied", "vehicle-spawned"),
                    Err(reason) => ("rejected", reason),
                };
                outcome(world, &command, disposition, reason);
            }
            ProductionCommand::InspectAssembly => crate::assembly::command(world, &command, None),
            ProductionCommand::AssemblyTransaction { transaction } => {
                crate::assembly::command(world, &command, Some(transaction))
            }
            ProductionCommand::Reset => {
                let has_target = world
                    .query_filtered::<Entity, With<crate::controls::PlayerVehicle>>()
                    .iter(world)
                    .count()
                    == 1;
                if !has_target {
                    outcome(world, &command, "rejected", "no-target");
                } else if world.resource::<PendingReset>().0.is_some() {
                    outcome(world, &command, "skipped", "reset-already-pending");
                } else {
                    world.resource_mut::<DriveInput>().reset = true;
                    world.resource_mut::<PendingReset>().0 = Some(command);
                }
            }
            ProductionCommand::SelectCar {
                car,
                reset_to_spawn,
            } => select(world, &command, car.clone(), *reset_to_spawn),
            ProductionCommand::CycleCar { direction } => {
                let fleet = world.resource::<Fleet>();
                let index = (fleet.selected as i32 + direction).rem_euclid(fleet.cars.len() as i32)
                    as usize;
                select(world, &command, fleet.cars[index].id.clone(), false);
            }
            _ => apply_immediate(world, command),
        }
    }
}
fn rejection(world: &World, command: &QueuedCommand) -> Option<&'static str> {
    if command.command.validate().is_err() {
        return Some("invalid-command");
    }
    if matches!(
        command.command,
        ProductionCommand::Drive { .. } | ProductionCommand::Reset
    ) && world.resource::<ControllerAssignment>().0 != command.source
    {
        return Some("controller-owner-active");
    }
    if command.source != ControllerOwner::Replay
        && !matches!(
            command.command,
            ProductionCommand::Vehicle { .. } | ProductionCommand::SpawnVehicle { .. }
        )
        && world
            .get_resource::<ReplayInput>()
            .is_some_and(|r| r.active)
        && !matches!(
            command.command,
            ProductionCommand::SetPaused { .. }
                | ProductionCommand::Quit
                | ProductionCommand::ToggleMute
        )
    {
        return Some("replay-owner-active");
    }
    None
}
fn apply_immediate(world: &mut World, command: QueuedCommand) {
    if let Some(reason) = rejection(world, &command) {
        outcome(world, &command, "rejected", reason);
        return;
    }
    if matches!(command.command, ProductionCommand::AssignController { .. })
        && world.resource::<PendingReset>().0.is_some()
    {
        world.resource_mut::<CommandBus>().fixed.push_back(command);
        return;
    }
    let reason = match command.command {
        ProductionCommand::Drive {
            throttle,
            brake,
            steer,
            handbrake,
        } => {
            let paused = world.resource::<Session>().paused;
            let mut input = world.resource_mut::<DriveInput>();
            input.throttle = if paused { 0.0 } else { throttle };
            input.brake = if paused { 0.0 } else { brake };
            input.steer = if paused { 0.0 } else { steer };
            input.handbrake = !paused && handbrake;
            if paused {
                "paused-controls-cleared"
            } else {
                "held-controls-updated"
            }
        }
        ProductionCommand::SetPaused { paused } => {
            world.resource_mut::<Session>().paused = paused;
            if paused {
                let reset = world.resource::<DriveInput>().reset;
                *world.resource_mut::<DriveInput>() = DriveInput {
                    reset,
                    ..Default::default()
                };
                for mut control in world
                    .query::<&mut crate::controls::VehicleControl>()
                    .iter_mut(world)
                {
                    control.input = DriveInput {
                        reset: control.input.reset,
                        ..Default::default()
                    };
                }
                world.resource_mut::<Time<Virtual>>().pause();
            } else {
                world.resource_mut::<Time<Virtual>>().unpause();
            }
            "pause-state-updated"
        }
        ProductionCommand::AssignController { controller } => {
            world.resource_mut::<ControllerAssignment>().0 = controller;
            *world.resource_mut::<DriveInput>() = DriveInput::default();
            "controller-assigned"
        }
        ProductionCommand::ToggleMute => {
            let mut session = world.resource_mut::<Session>();
            session.muted = !session.muted;
            "mute-state-updated"
        }
        ProductionCommand::Quit if world.resource::<Session>().paused => {
            world.write_message(AppExit::Success);
            "quit"
        }
        ProductionCommand::Quit => {
            outcome(world, &command, "rejected", "quit-requires-pause");
            return;
        }
        _ => unreachable!("immediate commands"),
    };
    outcome(world, &command, "applied", reason);
}
fn select(world: &mut World, command: &QueuedCommand, id: String, reset_to_spawn: bool) {
    if world.resource::<PendingReset>().0.is_some() {
        outcome(world, command, "rejected", "conflicting-selection");
        return;
    }
    let Some(index) = world
        .resource::<Fleet>()
        .cars
        .iter()
        .position(|car| car.id == id)
    else {
        outcome(world, command, "rejected", "unknown-car");
        return;
    };
    if !world.resource::<AccessContext>().permits(&id) {
        outcome(world, command, "rejected", "not-owned");
        return;
    }
    let spec = world.resource::<Fleet>().cars[index].clone();
    let mut query = world.query_filtered::<(
        &mut Transform,
        &mut Velocity,
        &mut ExternalForce,
        &mut Collider,
        &mut AdditionalMassProperties,
        &mut Vehicle,
    ), With<crate::controls::PlayerVehicle>>();
    let Ok((mut pose, mut velocity, mut force, mut collider, mut mass, mut car)) =
        query.single_mut(world)
    else {
        outcome(world, command, "rejected", "no-target");
        return;
    };
    *pose = recovery_pose(if reset_to_spawn {
        crate::vehicle::spawn_pose().translation
    } else {
        pose.translation
    });
    *velocity = Velocity::zero();
    *force = ExternalForce::default();
    *collider = chassis_collider(&spec);
    *mass = crate::vehicle::baseline_mass_properties();
    let count = car.reset_count + 1;
    *car = Vehicle::default();
    car.previous = *pose;
    car.reset_count = count;
    world.insert_resource(spec);
    world
        .resource_mut::<crate::assembly::AssemblyRuntime>()
        .applied_revision = None;
    world.resource_mut::<Fleet>().selected = index;
    *world.resource_mut::<DriveInput>() = DriveInput::default();
    outcome(world, command, "applied", "car-switched");
}
pub fn outcome(world: &mut World, command: &QueuedCommand, disposition: &str, reason: &str) {
    world.write_message(CommandOutcome {
        disposition: disposition.into(),
        reason: reason.into(),
    });
    if let Some(run) = world.get_resource::<RunHandle>() {
        let mut envelope = run.envelope();
        envelope.actor_id = command.envelope.actor_id.clone();
        envelope.command_id = command.envelope.command_id.clone();
        envelope.correlation_id = command.envelope.correlation_id.clone();
        envelope.span = command.envelope.span.clone();
        envelope.tick = world.resource::<SimulationClock>().current;
        envelope.boundary = "command_effect".into();
        run.event_envelope(
            "command.outcome",
            json!({
                "command": command.intent.as_ref().unwrap_or(&command.command),
                "normalized_command": command.intent.as_ref().map(|_| &command.command),
                "source": command.source,
                "disposition": disposition,
                "reason": reason,
                "ingress": command.envelope,
                "selected_car": world.get_resource::<CarSpec>().map(|s| &s.id),
                "paused": world.resource::<Session>().paused,
                "controller": world.resource::<ControllerAssignment>().0
            }),
            envelope.clone(),
        );
        if disposition == "applied" {
            legacy_effect(world, run, command, envelope);
        }
    }
}

fn legacy_effect(world: &World, run: &RunRecorder, command: &QueuedCommand, envelope: Envelope) {
    let emit = |kind: &str, fields| run.event_envelope(kind, fields, envelope.clone());
    match command.command {
        ProductionCommand::CycleCar { direction } => {
            let fleet = world.resource::<Fleet>();
            emit(
                "car.select",
                json!({"direction":direction,"pending":fleet.selected}),
            );
            emit(
                "car.switched",
                json!({"car":world.resource::<CarSpec>().id,"index":fleet.selected,"total":fleet.cars.len()}),
            );
        }
        ProductionCommand::SelectCar { .. } => {
            let fleet = world.resource::<Fleet>();
            emit(
                "car.switched",
                json!({"car":world.resource::<CarSpec>().id,"index":fleet.selected,"total":fleet.cars.len()}),
            );
        }
        ProductionCommand::SetPaused { paused } => emit("session.pause", json!({"paused":paused})),
        ProductionCommand::ToggleMute => emit(
            "session.mute",
            json!({"muted":world.resource::<Session>().muted}),
        ),
        ProductionCommand::Quit => emit("session.quit", json!({"paused":true})),
        _ => {}
    }
}

fn replay_commands(
    mut replay: Option<ResMut<ReplayInput>>,
    mut bus: ResMut<CommandBus>,
    mut release: ResMut<ReplayReleasePending>,
) {
    let Some(replay) = replay.as_mut().filter(|r| r.active) else {
        return;
    };
    if let Some(sample) = replay.next_sample() {
        bus.submit(
            ControllerOwner::Replay,
            ProductionCommand::Drive {
                throttle: sample.throttle,
                brake: sample.brake,
                steer: sample.steer,
                handbrake: sample.handbrake,
            },
        );
        if sample.reset {
            bus.submit(ControllerOwner::Replay, ProductionCommand::Reset);
        }
    }
    if !replay.active {
        release.0 = true;
        if let Some(run) = &bus.recorder {
            run.event("replay.finished", json!({"path":replay.path}));
        }
    }
}

fn assign_initial(
    mut bus: ResMut<CommandBus>,
    config: Option<Res<crate::RuntimeConfig>>,
    replay: Option<Res<ReplayInput>>,
) {
    let controller = config.map_or(ControllerOwner::LegacyComposite, |c| {
        if c.scenario.is_some() {
            ControllerOwner::Scenario
        } else if replay.as_ref().is_some_and(|r| r.active) {
            ControllerOwner::Replay
        } else {
            ControllerOwner::LegacyComposite
        }
    });
    bus.submit(
        controller,
        ProductionCommand::AssignController { controller },
    );
}

/// Terminating bounded runs account for commands that never reached a fixed boundary.
pub fn discard_pending(world: &mut World, reason: &str) {
    let mut pending: Vec<_> = world
        .resource_mut::<CommandBus>()
        .ingress
        .drain(..)
        .collect();
    pending.extend(world.resource_mut::<CommandBus>().fixed.drain(..));
    if let Some(command) = world.resource_mut::<PendingReset>().0.take() {
        pending.push(command);
    }
    for command in pending {
        outcome(world, &command, "skipped", reason);
    }
}

fn release_replay(world: &mut World) {
    if !world.resource::<ReplayReleasePending>().0 || world.resource::<PendingReset>().0.is_some() {
        return;
    }
    world.resource_mut::<ReplayReleasePending>().0 = false;
    world.resource_mut::<CommandBus>().submit(
        ControllerOwner::Replay,
        ProductionCommand::AssignController {
            controller: ControllerOwner::LegacyComposite,
        },
    );
    dispatch_update(world);
}

// App::run transfers the app into its runner; flush while this world still exists.
fn flush_on_exit(world: &mut World) {
    if world
        .get_resource::<Messages<AppExit>>()
        .is_none_or(|messages| messages.is_empty())
    {
        return;
    }
    discard_pending(world, "run-ended");
    if let Some(mut trace) = world.get_resource_mut::<crate::diagnostics::PhysicsTrace>() {
        trace.flush();
    }
}

/// Targeted player commands enter the same dispatch/reset ordering as legacy ingress.
pub(crate) fn apply_targeted_player(world: &mut World, command: QueuedCommand) {
    if matches!(command.command, ProductionCommand::Reset) {
        if let Some(reason) = rejection(world, &command) {
            outcome(world, &command, "rejected", reason);
        } else if world.resource::<PendingReset>().0.is_some() {
            outcome(world, &command, "skipped", "reset-already-pending");
        } else {
            world.resource_mut::<DriveInput>().reset = true;
            world.resource_mut::<PendingReset>().0 = Some(command);
        }
    } else {
        apply_immediate(world, command);
    }
}
