use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use car_game::vehicle::{DriveInput, Vehicle};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn sample(p: &Transform, v: &Velocity, c: &Vehicle, i: DriveInput) -> [f32; 41] {
    let mut values = [0.; 41];
    let mut at = 0;
    for part in [
        p.translation.to_array().as_slice(),
        p.rotation.to_array().as_slice(),
        v.linear.to_array().as_slice(),
        v.angular.to_array().as_slice(),
        &[
            c.rpm,
            c.gear as f32,
            c.steering,
            c.speed,
            c.slip,
            c.reset_count as f32,
        ],
        &c.wheel_lengths,
        &c.wheel_spin,
        &c.drive_demands,
        &c.longitudinal_forces,
        &[
            i.throttle,
            i.brake,
            i.steer,
            u8::from(i.handbrake) as f32,
            u8::from(i.reset) as f32,
        ],
        &[c.grounded as f32],
    ] {
        values[at..at + part.len()].copy_from_slice(part);
        at += part.len();
    }
    values
}
#[derive(Serialize, Clone)]
pub struct Metrics {
    pub ticks: usize,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub final_speed_mps: f32,
    pub maximum_speed_mps: f32,
    pub minimum_speed_mps: f32,
    pub peak_tilt_deg: f32,
    pub peak_slip_mps: f32,
    pub four_contact_ticks: usize,
    pub any_contact_ticks: usize,
    pub reset_count: u32,
    pub distance_m: f32,
    pub first_stop_tick: Option<usize>,
    pub first_stop_distance_m: Option<f32>,
    pub finite: bool,
}
impl Metrics {
    pub fn new(start: Vec3) -> Self {
        Self {
            ticks: 0,
            start: start.to_array(),
            end: start.to_array(),
            final_speed_mps: 0.,
            maximum_speed_mps: 0.,
            minimum_speed_mps: f32::MAX,
            peak_tilt_deg: 0.,
            peak_slip_mps: 0.,
            four_contact_ticks: 0,
            any_contact_ticks: 0,
            reset_count: 0,
            distance_m: 0.,
            first_stop_tick: None,
            first_stop_distance_m: None,
            finite: true,
        }
    }
    pub fn observe(&mut self, s: &[f32; 41], entry: f32) {
        self.ticks += 1;
        self.finite &= s.iter().all(|v| v.is_finite());
        let speed = Vec3::from_slice(&s[7..10]).length();
        self.final_speed_mps = speed;
        self.maximum_speed_mps = self.maximum_speed_mps.max(speed);
        self.minimum_speed_mps = self.minimum_speed_mps.min(speed);
        let tilt = (1. - 2. * (s[3] * s[3] + s[5] * s[5]))
            .clamp(-1., 1.)
            .acos()
            .to_degrees();
        self.peak_tilt_deg = self.peak_tilt_deg.max(tilt);
        self.peak_slip_mps = self.peak_slip_mps.max(s[17].abs());
        self.four_contact_ticks += usize::from(s[40] >= 4.);
        self.any_contact_ticks += usize::from(s[40] > 0.);
        self.reset_count = self.reset_count.max(s[18] as u32);
        self.distance_m += Vec2::new(s[0] - self.end[0], s[2] - self.end[2]).length();
        self.end.copy_from_slice(&s[..3]);
        if entry > 1. && speed < 0.2 && self.first_stop_tick.is_none() {
            self.first_stop_tick = Some(self.ticks);
            self.first_stop_distance_m = Some(self.distance_m);
        }
    }
    pub fn failures(&self, family: &str, entry: f32) -> Vec<&'static str> {
        let mut reasons = Vec::new();
        if !self.finite {
            reasons.push("nonfinite-state");
        }
        if self.reset_count > 0 {
            reasons.push("unexpected-reset");
        }
        if self.peak_tilt_deg > 80. {
            reasons.push("rollover");
        }
        if self.maximum_speed_mps > 150. {
            reasons.push("unbounded-speed");
        }
        if self.any_contact_ticks == 0 {
            reasons.push("no-wheel-support");
        }
        if matches!(family, "launch" | "reverse") && self.distance_m < 0.05 {
            reasons.push("no-propulsion-response");
        }
        if family == "coast" && self.final_speed_mps > entry + 0.5 {
            reasons.push("coasting-energy-increase");
        }
        if matches!(family, "brake" | "trail-brake") && self.minimum_speed_mps > entry - 0.05 {
            reasons.push("no-braking-response");
        }
        reasons
    }
}
pub fn hash_frame(hash: &mut Sha256, frame: &[f32; 41]) {
    for value in frame {
        hash.update(value.to_bits().to_le_bytes());
    }
}

pub fn trace_file(
    root: &std::path::Path,
    car: &str,
    id: &str,
    repeat: usize,
    trace: &[[f32; 41]],
) -> String {
    use std::io::Write;
    std::fs::create_dir_all(root).unwrap();
    let path = root.join(format!("{car}-{id}-r{repeat}.f32le"));
    let mut file = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    for frame in trace {
        for value in frame {
            file.write_all(&value.to_le_bytes()).unwrap();
        }
    }
    file.flush().unwrap();
    path.display().to_string()
}
