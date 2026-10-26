//! Shared closed-loop driver: commands traverse the same production command bus.
use crate::{
    roster::CarSpec,
    runtime::{CommandBus, ControllerOwner, ProductionCommand, SimulationClock},
    vehicle::{Vehicle, profile},
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
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
    fn project(&self, position: Vec2) -> (f32, f32) {
        let (mut best, mut progress, mut offset) = (f32::MAX, 0., 0.);
        for p in self.points.windows(2) {
            let (a, b) = (Vec2::from(p[0]), Vec2::from(p[1]));
            let delta = b - a;
            let t = ((position - a).dot(delta) / delta.length_squared()).clamp(0., 1.);
            let error = position.distance(a + delta * t);
            if error < best {
                best = error;
                progress = offset + delta.length() * t;
            }
            offset += delta.length();
        }
        (progress, best)
    }
}

#[derive(Resource)]
pub(super) struct RouteTracker {
    pub plan: RoutePlan,
    pub start_tick: Option<u64>,
    pub progress_m: f32,
    pub max_cross_track_m: f32,
    pub active_ticks: u64,
    pub four_contact_ticks: u64,
    pub completed: bool,
}
impl RouteTracker {
    pub fn new(plan: RoutePlan) -> Self {
        Self {
            plan,
            start_tick: None,
            progress_m: 0.,
            max_cross_track_m: 0.,
            active_ticks: 0,
            four_contact_ticks: 0,
            completed: false,
        }
    }
    pub fn outcome(&self, tolerance: f32) -> serde_json::Value {
        serde_json::json!({"passed":self.completed && self.max_cross_track_m <= tolerance,
            "progress_m":self.progress_m,"total_m":self.plan.total(),"completed":self.completed,
            "maximum_cross_track_m":self.max_cross_track_m,"active_ticks":self.active_ticks,
            "four_contact_ticks":self.four_contact_ticks,"corridor_half_width_m":tolerance})
    }
}

pub(super) fn controls(
    tracker: Option<ResMut<RouteTracker>>,
    clock: Res<SimulationClock>,
    spec: Res<CarSpec>,
    car: Query<(&Transform, &Velocity, &Vehicle)>,
    mut bus: ResMut<CommandBus>,
) {
    let Some(mut track) = tracker else {
        return;
    };
    if track
        .start_tick
        .is_none_or(|start| clock.current < start + 240)
    {
        return;
    }
    let Ok((pose, velocity, car)) = car.single() else {
        return;
    };
    let position = Vec2::new(pose.translation.x, pose.translation.z);
    let speed = Vec2::new(velocity.linear.x, velocity.linear.z).length();
    let (distance, error) = track.plan.project(position);
    track.progress_m = track.progress_m.max(distance);
    track.max_cross_track_m = track.max_cross_track_m.max(error);
    track.active_ticks += 1;
    track.four_contact_ticks += u64::from(car.grounded == 4);
    let remaining = track.plan.total() - track.progress_m;
    track.completed |= remaining < 2.;
    let lookahead = 6. + speed * 0.55;
    let target = track.plan.at(track.progress_m + lookahead);
    let direction = Vec3::new(target.x - position.x, 0., target.y - position.y).normalize_or_zero();
    let forward = pose.rotation * Vec3::NEG_Z;
    let angle = forward.cross(direction).y.atan2(forward.dot(direction));
    let p = profile::for_car(&spec.id);
    let wheel_angle = (2. * spec.wheelbase() * angle.sin() / lookahead).atan();
    let steer =
        (-wheel_angle / (p.max_steer_rad / (1. + speed * p.steer_speed_falloff))).clamp(-1., 1.);
    let target_speed = track.plan.speed_mps * (remaining / 12.).clamp(0.15, 1.).sqrt();
    let error_speed = target_speed - speed;
    let (throttle, brake) = if track.completed {
        (0., if speed > 1. { 1. } else { 0. })
    } else {
        (
            (error_speed * 0.5).clamp(0., 1.),
            (-error_speed * 0.3).clamp(0., 1.),
        )
    };
    bus.submit(
        ControllerOwner::Scenario,
        ProductionCommand::Drive {
            throttle,
            brake,
            steer,
            handbrake: track.completed,
        },
    );
}
