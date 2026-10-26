//! Observations only: these values never feed back into force calculation.
use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
pub struct WheelDecision {
    pub contact: &'static str,
    pub point: Option<[f32; 3]>,
    pub normal: Option<[f32; 3]>,
    pub requested_tire_n: [f32; 2],
    pub applied_tire_n: [f32; 2],
    pub brake_n: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct VehicleDecisions {
    pub schema_version: u32,
    pub boundary: &'static str,
    pub gear_before: i32,
    pub gear_after: i32,
    pub shift_reason: &'static str,
    pub shift_remaining_s: f32,
    pub rpm_before_shift: f32,
    pub upshift_rpm: f32,
    pub downshift_rpm: f32,
    pub nominal_downshift_rpm: f32,
    pub drive_cutoff: &'static str,
    pub engine_demand_n: f32,
    pub driven_radius_m: f32,
    pub wheels: [WheelDecision; 4],
}
impl VehicleDecisions {
    pub fn new(gear: i32) -> Self {
        Self {
            schema_version: 1,
            boundary: "force_calculation_before_integration",
            gear_before: gear,
            gear_after: gear,
            shift_reason: "unchanged",
            shift_remaining_s: 0.,
            rpm_before_shift: 0.,
            upshift_rpm: 0.,
            downshift_rpm: 0.,
            nominal_downshift_rpm: 0.,
            drive_cutoff: "none",
            engine_demand_n: 0.,
            driven_radius_m: 0.,
            wheels: std::array::from_fn(|_| WheelDecision {
                contact: "ray_miss",
                ..Default::default()
            }),
        }
    }
    pub fn drive(
        &mut self,
        car: &crate::vehicle::Vehicle,
        demand: f32,
        radius: f32,
        cutoff: &'static str,
        profile: &crate::vehicle::profile::PhysicsProfile,
        speed: f32,
    ) {
        self.gear_after = car.gear;
        self.shift_reason = match car.gear.cmp(&self.gear_before.max(1)) {
            std::cmp::Ordering::Greater => "upshift_rpm_threshold",
            std::cmp::Ordering::Less => "downshift_rpm_threshold",
            std::cmp::Ordering::Equal => "unchanged",
        };
        self.shift_remaining_s = car.shift_remaining;
        self.engine_demand_n = demand;
        self.driven_radius_m = radius;
        self.drive_cutoff = cutoff;
        let index = (self.gear_before.max(1) as usize - 1).min(profile.gears.len() - 1);
        self.rpm_before_shift = speed.abs() / radius * 60. / std::f32::consts::TAU
            * profile.gears[index]
            * profile.final_drive;
        self.upshift_rpm = profile.redline_rpm * 0.95;
        self.downshift_rpm = profile.downshift_rpm(index);
        self.nominal_downshift_rpm = profile.torque_rpm * 0.65;
    }
}
