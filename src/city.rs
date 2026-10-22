use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};
use bevy_rapier3d::prelude::*;

pub const SPACING: f32 = 112.0;
pub const ROAD: f32 = 24.0;
pub const EDGE: f32 = 480.0;

pub struct CityPlugin;
impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, build_city);
    }
}

fn block(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    solid: bool,
) {
    let mut entity = commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(position).with_scale(size),
    ));
    if solid {
        entity.insert((RigidBody::Fixed, Collider::cuboid(0.5, 0.5, 0.5)));
    }
}

#[derive(Default)]
struct Markings {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
}
impl Markings {
    fn rectangle(&mut self, x: f32, z: f32, width: f32, length: f32) {
        let i = self.positions.len() as u32;
        self.positions.extend([
            [x - width / 2.0, 0.014, z - length / 2.0],
            [x - width / 2.0, 0.014, z + length / 2.0],
            [x + width / 2.0, 0.014, z + length / 2.0],
            [x + width / 2.0, 0.014, z - length / 2.0],
        ]);
        self.indices.extend([i, i + 1, i + 2, i, i + 2, i + 3]);
    }
    fn mesh(self) -> Mesh {
        let n = self.positions.len();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n])
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

#[allow(clippy::too_many_lines)]
fn build_city(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    let cube = meshes.add(Cuboid::default());
    let asphalt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.13, 0.15, 0.17),
        perceptual_roughness: 0.96,
        ..default()
    });
    let pavement = materials.add(Color::srgb(0.48, 0.49, 0.46));
    let roof = materials.add(Color::srgb(0.26, 0.30, 0.31));
    let grass = materials.add(Color::srgb(0.25, 0.35, 0.22));
    let foliage = materials.add(Color::srgb(0.22, 0.32, 0.20));
    let metal = materials.add(Color::srgb(0.16, 0.21, 0.22));
    let white = materials.add(Color::srgb(0.81, 0.81, 0.72));
    let yellow = materials.add(Color::srgb(0.91, 0.63, 0.25));
    let facades: Vec<_> = (0..4)
        .map(|i| {
            materials.add(StandardMaterial {
                base_color_texture: Some(assets.load(format!("textures/facade-{i}.png"))),
                perceptual_roughness: 0.82,
                ..default()
            })
        })
        .collect();
    block(
        &mut commands,
        &cube,
        &asphalt,
        Vec3::new(0.0, -0.2, 0.0),
        Vec3::new(1040.0, 0.4, 1040.0),
        true,
    );
    // Raised block-sized sidewalks leave every street and intersection connected.
    for x in -4_i32..4 {
        for z in -4_i32..4 {
            let center = Vec3::new((x as f32 + 0.5) * SPACING, 0.0, (z as f32 + 0.5) * SPACING);
            block(
                &mut commands,
                &cube,
                &pavement,
                center + Vec3::Y * 0.09,
                Vec3::new(SPACING - ROAD, 0.18, SPACING - ROAD),
                true,
            );
            let park = (x * 13 + z * 7).rem_euclid(11) == 0;
            if park {
                block(
                    &mut commands,
                    &cube,
                    &grass,
                    center + Vec3::Y * 0.2,
                    Vec3::new(72.0, 0.05, 72.0),
                    false,
                );
                for t in 0..8 {
                    let angle = t as f32 * std::f32::consts::TAU / 8.0;
                    let p = center + Vec3::new(angle.cos() * 26.0, 0.0, angle.sin() * 26.0);
                    block(
                        &mut commands,
                        &cube,
                        &metal,
                        p + Vec3::Y * 2.2,
                        Vec3::new(0.5, 4.0, 0.5),
                        false,
                    );
                    block(
                        &mut commands,
                        &cube,
                        &foliage,
                        p + Vec3::Y * 5.4,
                        Vec3::new(6.0, 5.0, 6.0),
                        false,
                    );
                }
                continue;
            }
            for bx in [-1, 1] {
                for bz in [-1, 1] {
                    let seed = (x * 31 + z * 17 + bx * 7 + bz * 3).unsigned_abs();
                    let downtown =
                        (4.0 - (x as f32 + 0.5).abs().max((z as f32 + 0.5).abs())).max(0.0);
                    let height = 9.0 + (seed % 5) as f32 * 4.5 + downtown * 6.0;
                    let width = 26.0 + (seed % 3) as f32 * 2.0;
                    let depth = 26.0 + (seed % 4) as f32 * 2.0;
                    let p = center + Vec3::new(bx as f32 * 20.0, 0.0, bz as f32 * 20.0);
                    block(
                        &mut commands,
                        &cube,
                        &facades[(seed % 4) as usize],
                        p + Vec3::Y * (height / 2.0 + 0.18),
                        Vec3::new(width, height, depth),
                        true,
                    );
                    block(
                        &mut commands,
                        &cube,
                        &roof,
                        p + Vec3::Y * (height + 0.35),
                        Vec3::new(width + 0.4, 0.35, depth + 0.4),
                        false,
                    );
                    block(
                        &mut commands,
                        &cube,
                        &roof,
                        p + Vec3::Y * (height + 1.0),
                        Vec3::new(5.0, 1.4, 4.0),
                        false,
                    );
                }
            }
        }
    }
    let mut lines = Markings::default();
    let mut centers = Markings::default();
    for lane in -4..=4 {
        let a = lane as f32 * SPACING;
        for segment in -4..4 {
            let start = segment as f32 * SPACING + ROAD / 2.0 + 3.0;
            let end = (segment + 1) as f32 * SPACING - ROAD / 2.0 - 3.0;
            for offset in [-0.17, 0.17] {
                centers.rectangle(a + offset, (start + end) / 2.0, 0.12, end - start);
                centers.rectangle((start + end) / 2.0, a + offset, end - start, 0.12);
            }
            for side in [-1.0, 1.0] {
                lines.rectangle(a + side * 11.6, (start + end) / 2.0, 0.13, end - start);
                lines.rectangle((start + end) / 2.0, a + side * 11.6, end - start, 0.13);
                for dash in 0..7 {
                    let b = start + 4.0 + dash as f32 * 11.0;
                    lines.rectangle(a + side * 6.0, b, 0.13, 4.5);
                    lines.rectangle(b, a + side * 6.0, 4.5, 0.13);
                }
            }
        }
        for cross in -4..=4 {
            let b = cross as f32 * SPACING;
            for side in [-1.0, 1.0] {
                for stripe in -5..=5 {
                    lines.rectangle(a + stripe as f32 * 1.8, b + side * 14.0, 0.8, 2.8);
                    lines.rectangle(a + side * 14.0, b + stripe as f32 * 1.8, 2.8, 0.8);
                }
                // Simple street lamps: visual-only props on the sidewalks.
                let p = Vec3::new(a + side * 16.0, 0.0, b + side * 18.0);
                block(
                    &mut commands,
                    &cube,
                    &metal,
                    p + Vec3::Y * 3.6,
                    Vec3::new(0.18, 7.2, 0.18),
                    false,
                );
                block(
                    &mut commands,
                    &cube,
                    &white,
                    p + Vec3::new(-side * 1.0, 7.25, 0.0),
                    Vec3::new(2.3, 0.18, 0.55),
                    false,
                );
            }
        }
    }
    commands.spawn((
        Mesh3d(meshes.add(lines.mesh())),
        MeshMaterial3d(white.clone()),
    ));
    commands.spawn((
        Mesh3d(meshes.add(centers.mesh())),
        MeshMaterial3d(yellow.clone()),
    ));
    for sign in [-1.0, 1.0] {
        block(
            &mut commands,
            &cube,
            &roof,
            Vec3::new(sign * EDGE, 1.0, 0.0),
            Vec3::new(1.0, 2.0, EDGE * 2.0),
            true,
        );
        block(
            &mut commands,
            &cube,
            &roof,
            Vec3::new(0.0, 1.0, sign * EDGE),
            Vec3::new(EDGE * 2.0, 2.0, 1.0),
            true,
        );
        for i in -47..=47 {
            let a = i as f32 * 10.0;
            block(
                &mut commands,
                &cube,
                &yellow,
                Vec3::new(sign * (EDGE - 0.55), 0.9, a),
                Vec3::new(0.06, 0.45, 3.0),
                false,
            );
            block(
                &mut commands,
                &cube,
                &yellow,
                Vec3::new(a, 0.9, sign * (EDGE - 0.55)),
                Vec3::new(3.0, 0.45, 0.06),
                false,
            );
        }
    }
}
