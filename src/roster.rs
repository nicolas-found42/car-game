//! Prepared geometry metadata is shared by rendering, chassis setup and checks.
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Resource, Clone, Deserialize, Debug)]
pub struct CarSpec {
    pub id: String,
    pub name: String,
    pub game: String,
    pub game_id: String,
    pub variant: String,
    pub path: String,
    pub visual_offset: f32,
    pub wheels: [[f32; 3]; 4],
    pub radii: [f32; 4],
    pub half_width: f32,
    pub half_length: f32,
    pub body_center_z: f32,
    pub roof_height: f32,
    pub wheel_nodes: usize,
}

impl Default for CarSpec {
    fn default() -> Self {
        Fleet::default().cars.remove(0)
    }
}

impl CarSpec {
    pub fn mounts(&self) -> [Vec3; 4] {
        self.wheels.map(|p| Vec3::new(p[0], 0.16, p[2]))
    }

    pub fn wheelbase(&self) -> f32 {
        (self.wheels[2][2] + self.wheels[3][2] - self.wheels[0][2] - self.wheels[1][2]) * 0.5
    }
}

#[derive(Resource)]
pub struct Fleet {
    pub cars: Vec<CarSpec>,
    pub selected: usize,
    pub pending: Option<usize>,
}

impl Default for Fleet {
    fn default() -> Self {
        Self {
            cars: serde_json::from_str(include_str!("../assets/cars/roster.json"))
                .expect("validated bundled car roster"),
            selected: 0,
            pending: None,
        }
    }
}

impl Fleet {
    pub fn from_args() -> Self {
        let mut fleet = Self::default();
        let args: Vec<String> = std::env::args().collect();
        if let Some(index) = args.iter().position(|arg| arg == "--car") {
            let id = args.get(index + 1).expect("--car requires a roster id");
            fleet.selected = fleet
                .cars
                .iter()
                .position(|car| &car.id == id)
                .unwrap_or_else(|| panic!("Unknown car {id}; see assets/cars/roster.json"));
        }
        fleet
    }

    pub fn cycle(&mut self, direction: isize) {
        let current = self.pending.unwrap_or(self.selected);
        self.pending =
            Some((current as isize + direction).rem_euclid(self.cars.len() as isize) as usize);
    }
}
