//! Bounded anomaly detection, independent of simulation control and acceptance.
use super::{PhysicsSample, RunRecorder};
use serde_json::json;
use std::collections::VecDeque;

pub(crate) fn observe_clocks(
    run: bevy::prelude::Res<super::RunHandle>,
    real: bevy::prelude::Res<bevy::prelude::Time<bevy::prelude::Real>>,
    virtual_time: bevy::prelude::Res<bevy::prelude::Time<bevy::prelude::Virtual>>,
) {
    run.observe_clock(real.delta_secs_f64(), virtual_time.delta_secs_f64());
}

#[derive(Default)]
pub(super) struct Monitor {
    car: String,
    gear: i32,
    changes: VecDeque<u64>,
    unsupported: u64,
    zero_drive: u64,
    supported: bool,
    contacts: [&'static str; 4],
    resets: u32,
}
impl Monitor {
    pub fn observe(&mut self, sample: &PhysicsSample) -> Vec<&'static str> {
        if self.car != sample.car || self.resets != sample.reset_count {
            *self = Self {
                car: sample.car.clone(),
                gear: sample.gear,
                resets: sample.reset_count,
                ..Default::default()
            };
        }
        if self.gear > 0 && sample.gear > 0 && self.gear != sample.gear {
            self.changes.push_back(sample.tick);
        }
        self.gear = sample.gear;
        while self
            .changes
            .front()
            .is_some_and(|tick| sample.tick.saturating_sub(*tick) >= 120)
        {
            self.changes.pop_front();
        }
        self.unsupported = if sample.grounded == 0 && self.supported {
            self.unsupported + 1
        } else {
            0
        };
        self.supported |= sample.grounded == 4;
        self.zero_drive = if sample.input.throttle > 0.5
            && sample.input.brake < 0.01
            && sample.grounded > 0
            && sample.drive_demands.iter().all(|d| d.abs() < 0.001)
        {
            self.zero_drive + 1
        } else {
            0
        };
        let mut reasons = Vec::new();
        if self.changes.len() >= 8 {
            reasons.push("gear_hunting");
        }
        if self.unsupported == 12 {
            reasons.push("contact_loss");
        }
        if self.zero_drive == 120 {
            reasons.push("zero_drive_demand");
        }
        reasons
    }
    fn contact_changed(&mut self, sample: &PhysicsSample) -> bool {
        let next =
            sample.decisions.as_ref().map_or_else(
                || {
                    sample.evidence.wheels.each_ref().map(|wheel| {
                        match wheel.availability.as_str() {
                            "hit" => "hit",
                            "inverted" => "body_inverted",
                            "wheel_absent" => "wheel_absent",
                            "rejected_normal" => "normal_rejected",
                            _ => "ray_miss",
                        }
                    })
                },
                |decision| decision.wheels.each_ref().map(|w| w.contact),
            );
        let changed = self.contacts != next;
        self.contacts = next;
        changed
    }
}
impl RunRecorder {
    pub fn observe_physics(&self, sample: &PhysicsSample) {
        let mut monitors = self.monitor.lock().unwrap_or_else(|e| e.into_inner());
        let monitor = monitors.entry(sample.actor_id.clone()).or_default();
        let reasons = monitor.observe(sample);
        let contacts_changed = monitor.contact_changed(sample);
        drop(monitors);
        if sample
            .decisions
            .as_ref()
            .is_some_and(|d| d.shift_reason != "unchanged")
        {
            self.decision_event("vehicle.shift_decision", sample);
        }
        if contacts_changed {
            self.decision_event("vehicle.contact_decision", sample);
        }
        for reason in reasons {
            self.trigger_window(reason, sample.tick, &sample.car, Some(&sample.actor_id));
        }
    }
    fn decision_event(&self, kind: &str, sample: &PhysicsSample) {
        let mut envelope = self.envelope();
        envelope.tick = sample.tick;
        envelope.actor_id = Some(sample.actor_id.clone());
        envelope.boundary = sample.boundary.into();
        self.event_envelope(
            kind,
            json!({"car":sample.car,"decisions":sample.decisions,"completed_evidence":sample.evidence,"decision_calculation_boundary":"force_calculation_before_integration"}),
            envelope,
        );
    }
    pub fn observe_clock(&self, real_delta: f64, virtual_delta: f64) {
        if real_delta > 1. && real_delta - virtual_delta > 0.5 {
            self.event(
                "diagnostic.clock_gap",
                json!({"real_delta_s":real_delta,"virtual_delta_s":virtual_delta}),
            );
            self.trigger_window("clock_gap", self.envelope().tick, "session", None);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{prelude::*, time::TimeUpdateStrategy};
    #[test]
    fn a_real_virtual_clock_gap_is_observed_without_advancing_physics() {
        let root = std::env::temp_dir().join(format!("grid-clock-observer-{}", std::process::id()));
        let run = std::sync::Arc::new(super::RunRecorder::create_in(&root, "test", "350z"));
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(crate::diagnostics::RunHandle(run.clone()))
            .insert_resource(TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs(30),
            ))
            .add_systems(Update, super::observe_clocks);
        app.update();
        app.update();
        assert_eq!(run.envelope().tick, 0);
        assert_eq!(run.finish(AppExit::Success), AppExit::Success);
        let incident: serde_json::Value = serde_json::from_slice(
            &std::fs::read(run.dir().join("incidents/0001/incident.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(incident["reason"], "clock_gap");
        assert_eq!(incident["coverage"]["post_complete"], false);
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }
}
