use bevy::prelude::*;
use car_game::vehicle::DriveInput;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Case {
    pub id: usize,
    pub family: String,
    pub ticks: usize,
    pub entry_mps: f32,
    pub heading_rad: f32,
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    pub frequency_hz: f32,
    pub slope_deg: f32,
    pub base_height_m: f32,
    pub bump_height_m: f32,
    pub bump_width_m: f32,
    pub handbrake_ticks: usize,
}
impl Case {
    pub fn at(id: usize) -> Self {
        assert!(id < 5000);
        let family = [
            "launch",
            "brake",
            "coast",
            "steer-step",
            "slalom",
            "trail-brake",
            "handbrake",
            "reverse",
            "slope",
            "bumps",
        ][id / 500];
        let a = (id % 10) as f32;
        let b = ((id / 10) % 5) as f32;
        let c = ((id % 500) / 50) as f32;
        let mut v = Self {
            id,
            family: family.into(),
            ticks: 600,
            entry_mps: 0.,
            heading_rad: 0.,
            throttle: 0.,
            brake: 0.,
            steer: 0.,
            frequency_hz: 0.,
            slope_deg: 0.,
            base_height_m: 0.,
            bump_height_m: 0.,
            bump_width_m: 0.,
            handbrake_ticks: 0,
        };
        match family {
            "launch" => {
                v.throttle = (b + 1.) / 5.;
                v.ticks = 240 + (a as usize) * 240;
                v.heading_rad = c * std::f32::consts::TAU / 10.;
            }
            "brake" => {
                v.entry_mps = (a + 1.) * 5.;
                v.brake = (b + 1.) / 5.;
                v.heading_rad = c * std::f32::consts::TAU / 10.;
            }
            "coast" => {
                v.entry_mps = (a + 1.) * 3.;
                v.ticks = 120 + (b as usize) * 120;
                v.heading_rad = c * std::f32::consts::TAU / 10.;
            }
            "steer-step" => {
                v.entry_mps = (a + 1.) * 2.;
                v.steer = (b + 1.) * 0.1;
                v.heading_rad = c * std::f32::consts::TAU / 10.;
                v.ticks = 360;
            }
            "slalom" => {
                v.entry_mps = (a + 1.) * 2.;
                v.frequency_hz = (b + 1.) * 0.2;
                v.steer = (c + 1.) * 0.05;
                v.ticks = 960;
            }
            "trail-brake" => {
                v.entry_mps = (a + 1.) * 5.;
                v.brake = (b + 1.) / 5.;
                v.steer = (c - 4.5) / 9.;
                v.ticks = 480;
            }
            "handbrake" => {
                v.entry_mps = 5. + a * 2.5;
                v.steer = (b + 1.) * 0.1;
                v.handbrake_ticks = ((c + 1.) * 30.) as usize;
                v.ticks = 480;
            }
            "reverse" => {
                v.brake = (a + 1.) / 10.;
                v.steer = (c - 4.5) / 9.;
                v.ticks = 240 + (b as usize) * 120;
            }
            "slope" => {
                v.base_height_m = 128.;
                v.entry_mps = a * 1.5;
                v.slope_deg = (b - 2.) * 2.5;
                v.throttle = c / 9.;
            }
            "bumps" => {
                v.entry_mps = (a + 1.) * 2.;
                v.bump_height_m = (b + 1.) * 0.01;
                v.bump_width_m = 0.4 + c * 0.2;
            }
            _ => unreachable!(),
        }
        v
    }
    pub fn input(&self, tick: usize) -> DriveInput {
        let steer = if self.family == "slalom" {
            self.steer * (tick as f32 / 120. * self.frequency_hz * std::f32::consts::TAU).sin()
        } else if tick >= 24 {
            self.steer
        } else {
            0.
        };
        DriveInput {
            throttle: self.throttle,
            brake: self.brake,
            steer,
            handbrake: self.family == "handbrake" && tick >= 24 && tick < 24 + self.handbrake_ticks,
            reset: false,
        }
    }
    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_y(self.heading_rad)
    }
}
