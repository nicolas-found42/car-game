//! Closed-loop scenario driver over completed-step state and production commands.
use crate::{
    controls::{PlayerVehicle, VehicleCommand, VehicleControl},
    diagnostics::PhysicsSample,
    physics::CompletedSteps,
    roster::CarSpec,
    runtime::{CommandBus, ControllerOwner, ProductionCommand, SimulationClock},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePlan {
    pub points: Vec<[f32; 2]>,
    pub speed_mps: f32,
}
impl RoutePlan {
    pub fn validate(&self) -> Result<(), String> {
        if self.points.len() < 2
            || !self.speed_mps.is_finite()
            || !(2.0..=25.0).contains(&self.speed_mps)
            || self.points.iter().flatten().any(|v| !v.is_finite())
            || self
                .points
                .windows(2)
                .any(|p| Vec2::from(p[0]).distance(Vec2::from(p[1])) < 0.1)
        {
            return Err("route needs distinct finite points and speed in 2..25 m/s".into());
        }
        Ok(())
    }
    fn total(&self) -> f32 {
        self.points
            .windows(2)
            .map(|p| Vec2::from(p[0]).distance(Vec2::from(p[1])))
            .sum()
    }
    fn at(&self, distance: f32) -> Vec2 {
        let mut remaining = distance;
        for p in self.points.windows(2) {
            let (a, b) = (Vec2::from(p[0]), Vec2::from(p[1]));
            let length = a.distance(b);
            if remaining <= length {
                return a.lerp(b, remaining / length);
            }
            remaining -= length;
        }
        Vec2::from(*self.points.last().expect("validated route"))
    }
}

#[derive(Resource)]
pub(super) struct RouteTracker {
    plan: RoutePlan,
    pub start_tick: Option<u64>,
    segment: usize,
    entered_segment: bool,
    progress_m: f32,
    max_cross_track_m: f32,
    active_ticks: u64,
    four_contact_ticks: u64,
    completed: bool,
}
impl RouteTracker {
    pub fn new(plan: RoutePlan) -> Self {
        Self {
            plan,
            start_tick: None,
            segment: 0,
            entered_segment: false,
            progress_m: 0.,
            max_cross_track_m: 0.,
            active_ticks: 0,
            four_contact_ticks: 0,
            completed: false,
        }
    }
    fn record(&mut self, sample: &PhysicsSample) {
        let position = Vec2::new(sample.position[0], sample.position[2]);
        let a = Vec2::from(self.plan.points[self.segment]);
        let b = Vec2::from(self.plan.points[self.segment + 1]);
        let delta = b - a;
        let t = ((position - a).dot(delta) / delta.length_squared()).clamp(0., 1.);
        let offset: f32 = self.plan.points[..=self.segment]
            .windows(2)
            .map(|p| Vec2::from(p[0]).distance(Vec2::from(p[1])))
            .sum();
        self.entered_segment |= t <= 0.2 && position.distance(a) < 2.;
        if self.entered_segment {
            self.progress_m = self.progress_m.max(offset + delta.length() * t);
        }
        self.max_cross_track_m = self.max_cross_track_m.max(position.distance(a + delta * t));
        self.active_ticks += 1;
        self.four_contact_ticks += u64::from(sample.grounded == 4);
        if self.entered_segment && t >= 0.8 && position.distance(b) < 2. {
            if self.segment + 2 == self.plan.points.len() {
                self.completed = true;
            } else {
                self.segment += 1;
            }
        }
    }
    pub fn outcome(&self, tolerance: f32) -> serde_json::Value {
        serde_json::json!({"passed":self.completed && self.max_cross_track_m <= tolerance,
            "progress_m":self.progress_m,"total_m":self.plan.total(),"completed":self.completed,
            "maximum_cross_track_m":self.max_cross_track_m,"active_ticks":self.active_ticks,
            "four_contact_ticks":self.four_contact_ticks,"corridor_half_width_m":tolerance,
            "boundary":"completed_tick"})
    }
    fn drive(&self, sample: &PhysicsSample, spec: &CarSpec, reference: bool) -> VehicleCommand {
        let position = Vec2::new(sample.position[0], sample.position[2]);
        let speed = Vec2::new(sample.linear_velocity[0], sample.linear_velocity[2]).length();
        let lookahead = 6. + speed * 0.55;
        let target = self.plan.at(self.progress_m + lookahead);
        let direction =
            Vec3::new(target.x - position.x, 0., target.y - position.y).normalize_or_zero();
        let forward = Quat::from_array(sample.rotation) * Vec3::NEG_Z;
        let angle = forward.cross(direction).y.atan2(forward.dot(direction));
        let profile = crate::vehicle::profile::for_car(&sample.car);
        let (max_steer, falloff) = if reference {
            (profile.max_steer_rad, profile.steer_speed_falloff)
        } else {
            (0.56, 0.06)
        };
        let wheel_angle = (2. * spec.wheelbase() * angle.sin() / lookahead).atan();
        let steer = (-wheel_angle / (max_steer / (1. + speed * falloff))).clamp(-1., 1.);
        let remaining = self.plan.total() - self.progress_m;
        let target_speed = self.plan.speed_mps * (remaining / 12.).clamp(0.15, 1.).sqrt();
        let error = target_speed - speed;
        let (throttle, brake) = if self.completed {
            (0., if speed > 1. { 1. } else { 0. })
        } else {
            ((error * 0.5).clamp(0., 1.), (-error * 0.3).clamp(0., 1.))
        };
        VehicleCommand::Drive {
            throttle,
            brake,
            steer,
            handbrake: self.completed,
        }
    }
}

pub(super) fn observe(
    tracker: Option<ResMut<RouteTracker>>,
    clock: Res<SimulationClock>,
    completed: Res<CompletedSteps>,
    player: Query<&VehicleControl, With<PlayerVehicle>>,
) {
    let Some(mut track) = tracker else {
        return;
    };
    if track
        .start_tick
        .is_none_or(|start| clock.completed < start + 240)
    {
        return;
    }
    if let Some(sample) = player.single().ok().and_then(|c| completed.0.get(&c.id)) {
        track.record(sample);
    }
}

pub(super) fn controls(
    tracker: Option<Res<RouteTracker>>,
    clock: Res<SimulationClock>,
    config: Res<crate::RuntimeConfig>,
    spec: Res<CarSpec>,
    completed: Res<CompletedSteps>,
    player: Query<&VehicleControl, With<PlayerVehicle>>,
    mut bus: ResMut<CommandBus>,
) {
    let Some(track) = tracker else {
        return;
    };
    if track
        .start_tick
        .is_none_or(|start| clock.current < start + 240)
    {
        return;
    }
    let Ok(control) = player.single() else {
        return;
    };
    if let Some(sample) = completed.0.get(&control.id) {
        bus.submit(
            ControllerOwner::Scenario,
            ProductionCommand::Vehicle {
                target: control.id.clone(),
                command: track.drive(sample, &spec, config.reference_physics),
            },
        );
    }
}
