//! Rounded northeast connector between the two perimeter avenues.
use super::Markings;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub const CENTER: Vec2 = Vec2::new(420., 420.);
pub const RADIUS: f32 = 38.;
pub const WIDTH: f32 = 24.;
pub const SEGMENTS: usize = 48;

pub fn point(angle: f32, radius: f32) -> Vec2 {
    CENTER + Vec2::new(angle.cos(), angle.sin()) * radius
}

pub fn route(reverse: bool) -> Vec<[f32; 2]> {
    let mut points = vec![[458., 350.], [458., 390.]];
    points.extend(
        (0..=24).map(|i| point(i as f32 * std::f32::consts::FRAC_PI_2 / 24., RADIUS).to_array()),
    );
    points.extend([[390., 458.], [350., 458.]]);
    if reverse {
        points.reverse();
    }
    points
}

pub(super) fn build(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let cube = meshes.add(Cuboid::default());
    let curb = materials.add(Color::srgb(0.65, 0.65, 0.58));
    let white = materials.add(Color::srgb(0.9, 0.9, 0.8));
    let yellow = materials.add(Color::srgb(0.91, 0.63, 0.25));
    let mut edges = Markings::default();
    let mut center = Markings::default();
    for i in 0..SEGMENTS {
        let a = i as f32 * std::f32::consts::FRAC_PI_2 / SEGMENTS as f32;
        let b = (i + 1) as f32 * std::f32::consts::FRAC_PI_2 / SEGMENTS as f32;
        for radius in [RADIUS - WIDTH / 2., RADIUS + WIDTH / 2.] {
            edges.arc_strip(a, b, radius, 0.15);
            let p = point(
                (a + b) / 2.,
                radius + if radius > RADIUS { 1.5 } else { -1.5 },
            );
            let length = radius * (b - a) + 0.1;
            commands.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(curb.clone()),
                Transform::from_xyz(p.x, 0.11, p.y)
                    .with_rotation(Quat::from_rotation_y(-(a + b) / 2.))
                    .with_scale(Vec3::new(0.35, 0.22, length)),
                crate::physics::ColliderIdentity(format!("course:curve:{i}:{radius}")),
                RigidBody::Fixed,
                Collider::cuboid(0.5, 0.5, 0.5),
            ));
        }
        for offset in [-0.17, 0.17] {
            center.arc_strip(a, b, RADIUS + offset, 0.12);
        }
    }
    commands.spawn((Mesh3d(meshes.add(edges.mesh())), MeshMaterial3d(white)));
    commands.spawn((Mesh3d(meshes.add(center.mesh())), MeshMaterial3d(yellow)));
    // The existing continuous asphalt collider supports the connector without seams.
}

impl Markings {
    fn arc_strip(&mut self, a: f32, b: f32, radius: f32, width: f32) {
        let index = self.positions.len() as u32;
        for (angle, r) in [
            (a, radius - width / 2.),
            (b, radius - width / 2.),
            (b, radius + width / 2.),
            (a, radius + width / 2.),
        ] {
            let p = point(angle, r);
            self.positions.push([p.x, 0.016, p.y]);
        }
        self.indices
            .extend([index, index + 1, index + 2, index, index + 2, index + 3]);
    }
}
