//! Explicit reference values and declared estimates; metadata is retained in physics.json.
use crate::roster::CarSpec;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Clone, Debug, Deserialize)]
pub struct PhysicsProfile {
    pub mass_kg: f32,
    pub power_kw: f32,
    pub torque_nm: f32,
    pub power_rpm: f32,
    pub torque_rpm: f32,
    pub redline_rpm: f32,
    pub gears: Vec<f32>,
    pub final_drive: f32,
    pub front_weight: f32,
    pub front_drive: f32,
    pub cg_height_m: f32,
    pub ride_frequency_hz: [f32; 2],
    pub damping_ratio: f32,
    pub anti_roll_fraction: [f32; 2],
    pub tire_mu: f32,
    pub lateral_mu: f32,
    pub cornering_per_load: [f32; 2],
    pub drag_area_m2: f32,
    pub rolling_coefficient: f32,
    pub driveline_efficiency: f32,
    pub shift_seconds: f32,
    pub brake_decel_mps2: f32,
    pub max_steer_rad: f32,
    pub steer_speed_falloff: f32,
    pub speed_governor_mps: Option<f32>,
}

static PROFILES: LazyLock<BTreeMap<String, (PhysicsProfile, serde_json::Value)>> =
    LazyLock::new(|| {
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../assets/cars/physics.json"))
                .expect("bundled physics profile JSON");
        source["cars"]
            .as_array()
            .expect("profile list")
            .iter()
            .map(|row| {
                let id = row["id"].as_str().expect("profile id").to_string();
                let profile =
                    serde_json::from_value(row["parameters"].clone()).expect("typed profile");
                (id, (profile, row.clone()))
            })
            .collect()
    });

pub fn for_car(id: &str) -> &'static PhysicsProfile {
    &PROFILES
        .get(id)
        .expect("every fleet car has an explicit physics profile")
        .0
}

pub fn report(id: &str) -> serde_json::Value {
    PROFILES.get(id).expect("profile metadata").1.clone()
}

impl PhysicsProfile {
    pub fn corner_mass(&self, axle: usize) -> f32 {
        self.mass_kg
            * if axle == 0 {
                self.front_weight
            } else {
                1. - self.front_weight
            }
            / 2.
    }
    pub fn spring(&self, axle: usize) -> f32 {
        self.corner_mass(axle) * (std::f32::consts::TAU * self.ride_frequency_hz[axle]).powi(2)
    }
    pub fn damper(&self, axle: usize) -> f32 {
        2. * self.damping_ratio * (self.spring(axle) * self.corner_mass(axle)).sqrt()
    }
    pub fn properties(&self, spec: &CarSpec) -> MassProperties {
        let front = (spec.wheels[0][2] + spec.wheels[1][2]) * 0.5;
        let rear = (spec.wheels[2][2] + spec.wheels[3][2]) * 0.5;
        let radius = spec.radii.iter().sum::<f32>() / 4.;
        let sag = (0..2)
            .map(|a| self.corner_mass(a) * 9.81 / self.spring(a))
            .sum::<f32>()
            / 2.;
        let rest_origin = radius + super::REST - 0.16 - sag;
        let (w, l, h) = (
            spec.half_width * 1.8,
            spec.half_length * 1.6,
            self.cg_height_m * 1.6,
        );
        MassProperties {
            mass: self.mass_kg,
            local_center_of_mass: Vec3::new(
                0.,
                self.cg_height_m - rest_origin,
                rear - self.front_weight * (rear - front),
            ),
            principal_inertia: Vec3::new(h * h + l * l, w * w + l * l, w * w + h * h)
                * (self.mass_kg / 12.),
            ..default()
        }
    }
    pub fn mass_properties(&self, spec: &CarSpec) -> AdditionalMassProperties {
        AdditionalMassProperties::MassProperties(self.properties(spec))
    }
    /// Estimated torque shape constrained by published peak torque and peak power.
    pub fn torque(&self, rpm: f32) -> f32 {
        let peak_power_torque = self.power_kw * 9549.297 / self.power_rpm;
        let (x0, y0, x1, y1) = if rpm <= self.torque_rpm {
            (900., self.torque_nm * 0.55, self.torque_rpm, self.torque_nm)
        } else if rpm <= self.power_rpm {
            (
                self.torque_rpm,
                self.torque_nm,
                self.power_rpm,
                peak_power_torque,
            )
        } else {
            (
                self.power_rpm,
                peak_power_torque,
                self.redline_rpm,
                peak_power_torque * 0.85,
            )
        };
        let fraction = ((rpm - x0) / (x1 - x0).max(1.)).clamp(0., 1.);
        (y0 + (y1 - y0) * fraction)
            .min(self.torque_nm)
            .min(self.power_kw * 9549.297 / rpm.max(900.))
    }
    /// Kinematic mean wheel speed at the nominal driven-axle split. A harmonic
    /// radius averages wheel RPM, excludes undriven wheels, and preserves power
    /// equivalence. AWD remains a nominal center-coupling approximation; the
    /// load-dependent force allocation does not change engine gearing.
    pub fn drive_radius(&self, radii: [f32; 4]) -> f32 {
        let front = (1. / radii[0] + 1. / radii[1]) * 0.5;
        let rear = (1. / radii[2] + 1. / radii[3]) * 0.5;
        1. / (self.front_drive * front + (1. - self.front_drive) * rear)
    }

    /// Keep the lower gear at least 5% below its upshift threshold on a
    /// downshift. Wide ratio gaps must not make adjacent shift rules overlap.
    pub fn downshift_rpm(&self, index: usize) -> f32 {
        let nominal = self.torque_rpm * 0.65;
        if index == 0 {
            nominal
        } else {
            nominal
                .min(self.redline_rpm * 0.95 * (self.gears[index] / self.gears[index - 1]) * 0.95)
        }
    }

    pub fn drive_force(
        &self,
        car: &mut super::Vehicle,
        speed: f32,
        radius: f32,
        throttle: f32,
    ) -> f32 {
        let index = (car.gear.max(1) as usize - 1).min(self.gears.len() - 1);
        let wheel_rpm = speed.abs() / radius * 60. / std::f32::consts::TAU;
        let rpm = wheel_rpm * self.gears[index] * self.final_drive;
        let next = if rpm > self.redline_rpm * 0.95 && index + 1 < self.gears.len() {
            index + 1
        } else if index > 0 && rpm < self.downshift_rpm(index) {
            index - 1
        } else {
            index
        };
        if next != index {
            car.shift_remaining = self.shift_seconds;
        }
        car.gear = next as i32 + 1;
        car.shift_remaining = (car.shift_remaining - super::STEP).max(0.);
        let launch = (self.torque_rpm * 0.65).clamp(1800., 3200.);
        car.rpm = (wheel_rpm * self.gears[next] * self.final_drive).max(if throttle > 0.01 {
            launch
        } else {
            900.
        });
        if car.shift_remaining > 0. || car.rpm > self.redline_rpm {
            return 0.;
        }
        throttle
            * self
                .speed_governor_mps
                .map_or(1., |limit| ((limit - speed) / 0.5).clamp(0., 1.))
            * self.torque(car.rpm)
            * self.gears[next]
            * self.final_drive
            * self.driveline_efficiency
            / radius
    }
    pub fn drive_shares(&self, demand: f32, loads: [f32; 4]) -> [f32; 4] {
        let rear_grip = (loads[2] + loads[3]) * self.tire_mu;
        let front = if self.front_drive >= 1. {
            1.
        } else if self.front_drive > 0. {
            ((demand.abs() - rear_grip) / demand.abs().max(1.)).clamp(self.front_drive, 0.5)
        } else {
            0.
        };
        [
            front * 0.5,
            front * 0.5,
            (1. - front) * 0.5,
            (1. - front) * 0.5,
        ]
    }
}
