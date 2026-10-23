//! Vehicle dynamics live here, independent of rendering and input devices.
use crate::{
    controls::{PlayerVehicle, VehicleControl},
    diagnostics::PhysicsSample,
    roster::{CarSpec, Fleet},
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub const STEP: f32 = 1.0 / 120.0;
pub const MASS: f32 = 1320.0;
pub const BASELINE_INERTIA: [f32; 3] = [1800., 2300., 650.];
pub fn baseline_mass_properties() -> AdditionalMassProperties {
    AdditionalMassProperties::MassProperties(MassProperties {
        mass: MASS,
        local_center_of_mass: Vec3::ZERO,
        principal_inertia: Vec3::from_array(BASELINE_INERTIA),
        ..default()
    })
}
const REST: f32 = 0.44;

#[derive(Resource, Default, Clone, Copy)]
pub struct DriveInput {
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    pub handbrake: bool,
    pub reset: bool,
}

#[derive(Component)]
pub struct Vehicle {
    pub steering: f32,
    pub wheel_lengths: [f32; 4],
    pub wheel_spin: [f32; 4],
    pub grounded: usize,
    pub speed: f32,
    pub slip: f32,
    pub rpm: f32,
    pub gear: i32,
    pub previous: Transform,
    pub reset_count: u32,
    reverse_engaged: bool,
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            steering: 0.0,
            wheel_lengths: [REST; 4],
            wheel_spin: [0.0; 4],
            grounded: 0,
            speed: 0.0,
            slip: 0.0,
            rpm: 900.0,
            gear: 1,
            previous: spawn_pose(),
            reset_count: 0,
            reverse_engaged: false,
        }
    }
}

pub struct VehiclePlugin;
impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DriveInput>()
            .init_resource::<CarSpec>()
            .init_resource::<Fleet>()
            .insert_resource(Time::<Fixed>::from_seconds(STEP as f64))
            .insert_resource(TimestepMode::Fixed {
                dt: STEP,
                substeps: 1,
            })
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule())
            .add_systems(Startup, spawn_vehicle)
            .add_plugins(crate::physics::PhysicsAdapterPlugin)
            .add_systems(
                FixedUpdate,
                simulate
                    .after(crate::physics::PhysicsPhase::Prepare)
                    .before(PhysicsSet::SyncBackend)
                    .run_if(|mode: Res<crate::physics::QueryMode>| {
                        *mode == crate::physics::QueryMode::LegacyPreSync
                    }),
            )
            .add_systems(
                FixedUpdate,
                simulate
                    .in_set(crate::physics::PhysicsPhase::Mechanics)
                    .run_if(|mode: Res<crate::physics::QueryMode>| {
                        *mode == crate::physics::QueryMode::Synchronized
                    }),
            );
    }
}

pub fn spawn_pose() -> Transform {
    Transform::from_xyz(3.2, 0.62, 190.0)
}

pub fn chassis_collider(spec: &CarSpec) -> Collider {
    Collider::compound(vec![
        (
            Vec3::new(0.0, 0.0, spec.body_center_z),
            Quat::IDENTITY,
            Collider::cuboid(spec.half_width, 0.27, spec.half_length),
        ),
        (
            Vec3::new(
                0.0,
                (spec.roof_height - 0.25).clamp(0.32, 0.6),
                spec.body_center_z + 0.13,
            ),
            Quat::IDENTITY,
            Collider::cuboid(spec.half_width * 0.8, 0.25, spec.half_length * 0.5),
        ),
    ])
}

fn spawn_vehicle(mut commands: Commands, spec: Res<CarSpec>) {
    let spec = spec.clone();
    commands.spawn(chassis_bundle(&spec, spawn_pose())).insert((
        PlayerVehicle,
        crate::physics::ColliderIdentity("pilot-350z:chassis".into()),
        VehicleControl::new(
            crate::assembly::VehicleId::from("pilot-350z"),
            spec,
            crate::runtime::ControllerOwner::default(),
        ),
    ));
}

pub fn chassis_bundle(spec: &CarSpec, pose: Transform) -> impl Bundle {
    (
        Name::new("Vehicle chassis"),
        Vehicle::default(),
        crate::physics::StepEvidence::default(),
        pose,
        RigidBody::Dynamic,
        chassis_collider(spec),
        ColliderMassProperties::Density(0.0),
        baseline_mass_properties(),
        Velocity::zero(),
        ExternalForce::default(),
        Damping {
            linear_damping: 0.02,
            angular_damping: 0.25,
        },
        Friction::coefficient(0.3),
        Restitution::coefficient(0.08),
        Ccd::enabled(),
        Sleeping::disabled(),
    )
}

/// Snap to the nearest street, inside the perimeter, facing its longitudinal axis.
/// A lane offset keeps a reset out of intersection centers and off sidewalks.
pub fn recovery_pose(position: Vec3) -> Transform {
    let x = (position.x / crate::city::SPACING).round().clamp(-4.0, 4.0) * crate::city::SPACING;
    let z = (position.z / crate::city::SPACING).round().clamp(-4.0, 4.0) * crate::city::SPACING;
    if (position.x - x).abs() <= (position.z - z).abs() {
        Transform::from_xyz(x + 3.2, 0.65, position.z.clamp(-430.0, 430.0))
    } else {
        Transform::from_xyz(position.x.clamp(-430.0, 430.0), 0.65, z - 3.2)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
    }
}

// The same force solver is selected on either side of synchronization for baseline comparison.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::type_complexity
)]
pub(crate) fn simulate(
    context: ReadRapierContext,
    mode: Res<crate::physics::QueryMode>,
    current: Res<crate::physics::CurrentQueries>,
    assembly: Option<Res<crate::assembly::AssemblyRuntime>>,
    ledger: Option<Res<crate::assembly::Assembly>>,
    mut cars: Query<(
        Entity,
        &Transform,
        &Velocity,
        &mut ExternalForce,
        &mut Vehicle,
        &VehicleControl,
        &mut crate::physics::StepEvidence,
    )>,
) {
    let context = context.single().ok();
    for (entity, pose, velocity, mut total, mut car, control, mut evidence) in &mut cars {
        let spec = &control.spec;
        let input = control.input;
        let revision = crate::physics::recording::assembly_revision(
            entity,
            &control.id,
            ledger.as_deref(),
            assembly.as_deref(),
        );
        evidence.assembly_revision = revision.map(|r| r.0);
        evidence.assembly_generation = revision.map(|r| r.1);
        evidence.assembly_binding = if revision.is_some() {
            "bound"
        } else {
            "no_assembly_binding"
        }
        .into();
        evidence.query_tick = current.tick;
        evidence.query_boundary = if *mode == crate::physics::QueryMode::Synchronized {
            "synchronized_pre_step"
        } else {
            "previous_step_queries"
        }
        .into();
        evidence.geometry_revision = current.revision;
        evidence.mass = context.as_ref().and_then(|c| c.rigidbody_set.mass(entity));
        let force_center = context
            .as_ref()
            .and_then(|c| c.rigidbody_set.center_of_mass(entity))
            .unwrap_or(pose.translation);
        evidence.center_of_mass = force_center.to_array();
        if evidence.outcome == "reset" {
            continue;
        }
        *total = ExternalForce::default();
        if *mode == crate::physics::QueryMode::LegacyPreSync && context.is_none() {
            evidence.outcome = "unavailable_context".into();
            continue;
        }
        let center = pose.translation;
        let up = pose.rotation * Vec3::Y;
        let forward = pose.rotation * Vec3::NEG_Z;
        let speed = velocity.linear.dot(forward);
        car.speed = speed;
        let max_steer = 0.56 / (1.0 + speed.abs() * 0.06);
        let target = -input.steer.clamp(-1.0, 1.0) * max_steer;
        car.steering += (target - car.steering) * (1.0 - (-9.0 * STEP).exp());
        let mut lengths = [REST; 4];
        let mut loads = [0.0; 4];
        let mut contacts = [None; 4];
        let mut ray_misses = 0;
        car.grounded = 0;
        car.slip = 0.0;
        *total = ExternalForce::default();

        for (i, mount) in spec.mounts().iter().enumerate() {
            let origin = center + pose.rotation * *mount;
            evidence.wheels[i] = crate::physics::WheelSample {
                corner: i,
                part_id: revision.and_then(|_| {
                    ledger.as_ref().and_then(|a| {
                        a.parts
                            .iter()
                            .find(|p| {
                                matches!(a.definitions[&p.definition].collision,
                        crate::assembly::CollisionBinding::RaycastWheel { corner } if corner == i)
                            })
                            .map(|p| p.id.clone())
                    })
                }),
                availability: "miss".into(),
                origin: origin.to_array(),
                direction: (-up).to_array(),
                range: REST + spec.radii[i],
                ..default()
            };
            if revision.is_some() && assembly.as_ref().is_some_and(|a| !a.wheel_enabled[i]) {
                evidence.wheels[i].availability = "wheel_absent".into();
                ray_misses += 1;
                continue;
            }
            if up.y < 0.25 {
                evidence.wheels[i].availability = "inverted".into();
                ray_misses += 1;
                continue;
            }
            let hit = if *mode == crate::physics::QueryMode::Synchronized {
                current
                    .cast(origin, -up, REST + spec.radii[i], entity)
                    .map(|hit| (hit.identity, hit.distance, hit.normal))
            } else {
                context
                    .as_ref()
                    .and_then(|context| {
                        context.cast_ray_and_get_normal(
                            origin,
                            -up,
                            REST + spec.radii[i],
                            true,
                            QueryFilter::default()
                                .exclude_rigid_body(entity)
                                .exclude_sensors(),
                        )
                    })
                    .map(|(_, hit)| (None, hit.time_of_impact, hit.normal))
            };
            let Some((identity, distance, normal)) = hit else {
                ray_misses += 1;
                continue;
            };
            evidence.wheels[i].collider_id = identity;
            evidence.wheels[i].distance = Some(distance);
            evidence.wheels[i].normal = Some(normal.to_array());
            evidence.wheels[i].point = Some((origin - up * distance).to_array());
            if normal.dot(up) < 0.45 {
                evidence.wheels[i].availability = "rejected_normal".into();
                ray_misses += 1;
                continue;
            }
            evidence.wheels[i].availability = "hit".into();
            lengths[i] = (distance - spec.radii[i]).clamp(0.02, REST);
            let point = origin - up * distance;
            let point_velocity = velocity.linear + velocity.angular.cross(point - force_center);
            let compression = REST - lengths[i];
            let load = (compression * 38000.0 - point_velocity.dot(normal) * 4100.0)
                .clamp(0.0, MASS * 9.81 * 1.5);
            loads[i] = load;
            contacts[i] = Some((point, normal, point_velocity));
            let force = ExternalForce::at_point(normal * load, point, force_center);
            evidence.wheels[i].suspension = force.into();
            *total += force;
            car.grounded += 1;
        }
        let planar_speed = Vec2::new(velocity.linear.x, velocity.linear.z).length();
        if input.throttle > 0.01 {
            car.reverse_engaged = false;
        } else if input.brake > 0.01 && planar_speed < 0.8 {
            car.reverse_engaged = true;
        }
        let (drive, brake, reverse) =
            if input.brake > 0.01 && input.throttle < 0.01 && speed < 0.8 && car.reverse_engaged {
                (
                    -input.brake * 4800.0 * (1.0 - speed.abs() / 8.0).max(0.0),
                    0.0,
                    true,
                )
            } else if input.throttle > 0.01 && speed < -0.8 {
                (0.0, input.throttle, false)
            } else {
                (
                    input.throttle * 8500.0 * (1.0 - speed.max(0.0) / 77.0).max(0.06),
                    input.brake,
                    false,
                )
            };
        for i in 0..4 {
            if let Some((point, normal, point_velocity)) = contacts[i] {
                let steer = if i < 2 { car.steering } else { 0.0 };
                let wheel_forward = pose.rotation * (Quat::from_rotation_y(steer) * Vec3::NEG_Z);
                let wheel_forward =
                    (wheel_forward - normal * wheel_forward.dot(normal)).normalize_or_zero();
                let side = wheel_forward.cross(normal).normalize_or_zero();
                let longitudinal = point_velocity.dot(wheel_forward);
                let lateral = point_velocity.dot(side);
                let angle = lateral.atan2(longitudinal.abs().max(2.0));
                let rear_slide = input.handbrake && i >= 2;
                let grip = loads[i] * if rear_slide { 0.60 } else { 1.22 };
                let braking = (brake * MASS * 10.0 / 4.0 + if rear_slide { 6500.0 } else { 0.0 })
                    .min(longitudinal.abs() * MASS / (4.0 * STEP));
                let engine = if i >= 2 && !rear_slide {
                    drive / 2.0
                } else {
                    0.0
                };
                let fx = engine - longitudinal.signum() * braking;
                let fy = -angle * if i < 2 { 47000.0 } else { 45000.0 };
                let force2 = Vec2::new(fx, fy).clamp_length_max(grip);
                let force = wheel_forward * force2.x + side * force2.y;
                let force = ExternalForce::at_point(force, point, force_center);
                evidence.wheels[i].tire = force.into();
                *total += force;
                // Forward is -Z, so rolling forward requires rotation about -X.
                car.wheel_spin[i] = (car.wheel_spin[i] - longitudinal * STEP / spec.radii[i])
                    % std::f32::consts::TAU;
                car.slip = car
                    .slip
                    .max(angle.abs() * (longitudinal.abs() / 8.0).clamp(0.0, 1.0));
            }
        }
        for axle in [0, 2] {
            if contacts[axle].is_some() && contacts[axle + 1].is_some() {
                let roll = (lengths[axle + 1] - lengths[axle]) * 6500.0;
                for (i, sign) in [(axle, 1.0), (axle + 1, -1.0)] {
                    let force = ExternalForce::at_point(
                        up * roll * sign,
                        center + pose.rotation * spec.mounts()[i],
                        force_center,
                    );
                    evidence.wheels[i].antiroll = force.into();
                    *total += force;
                }
            }
        }
        let wheel_total = *total;
        total.force -= velocity.linear * velocity.linear.length() * 0.45;
        total.force -= Vec3::new(velocity.linear.x, 0.0, velocity.linear.z) * 17.0;
        if car.grounded >= 3 {
            total.force -= up * speed * speed * 1.3;
            let target_yaw = (car.steering.tan() * speed / spec.wheelbase())
                .clamp(-11.0 / speed.abs().max(4.0), 11.0 / speed.abs().max(4.0));
            let assist = if input.handbrake { 350.0 } else { 1700.0 };
            total.torque.y += (target_yaw - velocity.angular.y) * assist;
        }
        car.wheel_lengths = lengths;
        car.gear = if reverse || speed < -0.8 {
            -1
        } else {
            (1 + (speed.max(0.0) / 12.0) as i32).min(6)
        };
        car.rpm =
            900.0 + ((speed.abs() % 12.0) / 12.0 * 4900.0 + input.throttle * 900.0).min(6000.0);

        evidence.chassis = ExternalForce {
            force: total.force - wheel_total.force,
            torque: total.torque - wheel_total.torque,
        }
        .into();
        evidence.total = (*total).into();
        evidence.wheel_loads = loads;
        evidence.ray_misses = ray_misses;
    }
}

pub(crate) fn physics_invariant(sample: &PhysicsSample) -> Option<&'static str> {
    if !sample.position.iter().all(|value| value.is_finite()) {
        return Some("non_finite_position");
    }
    if !sample.rotation.iter().all(|value| value.is_finite()) {
        return Some("non_finite_rotation");
    }
    if !sample
        .linear_velocity
        .iter()
        .chain(sample.angular_velocity.iter())
        .all(|value| value.is_finite())
    {
        return Some("non_finite_velocity");
    }
    if sample.grounded > 4 {
        return Some("grounded_more_than_four_wheels");
    }
    if sample
        .wheel_lengths
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0 || *value > REST + 0.05)
    {
        return Some("wheel_length_out_of_range");
    }
    if sample
        .wheel_loads
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Some("wheel_load_out_of_range");
    }
    if !sample.speed.is_finite() || !sample.rpm.is_finite() || !sample.steering.is_finite() {
        return Some("non_finite_vehicle_state");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn world(wall: bool) -> App {
        world_for(CarSpec::default(), wall)
    }
    fn world_for(spec: CarSpec, wall: bool) -> App {
        world_for_scheduling(spec, wall, true)
    }
    fn world_for_scheduling(spec: CarSpec, wall: bool, single_threaded: bool) -> App {
        let mut app = App::new();
        app.insert_resource(spec);
        app.add_plugins((MinimalPlugins, TransformPlugin, VehiclePlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                STEP as f64,
            )));
        // Tiny physics fixtures benefit from avoiding task-pool handoffs. The
        // differential test below checks them against production's default executor.
        if single_threaded {
            for (_, schedule) in app
                .world_mut()
                .resource_mut::<bevy::ecs::schedule::Schedules>()
                .iter_mut()
            {
                schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
            }
        }
        app.world_mut().spawn((
            RigidBody::Fixed,
            Collider::cuboid(1000.0, 0.2, 1000.0),
            Transform::from_xyz(0.0, -0.2, 0.0),
        ));
        if wall {
            app.world_mut().spawn((
                RigidBody::Fixed,
                Collider::cuboid(30.0, 3.0, 0.5),
                Transform::from_xyz(0.0, 3.0, 120.0),
            ));
        }
        tick(&mut app, 240);
        app
    }
    fn tick(app: &mut App, count: usize) {
        for _ in 0..count {
            app.update();
        }
    }
    fn state(app: &mut App) -> (Transform, Velocity, usize) {
        let mut q = app.world_mut().query::<(&Transform, &Velocity, &Vehicle)>();
        let (pose, vel, car) = q.single(app.world()).unwrap();
        (*pose, *vel, car.grounded)
    }
    #[test]
    fn every_car_meets_the_baseline_driving_contract() {
        let cars = Fleet::default().cars;
        let workers = std::thread::available_parallelism()
            .map_or(1, usize::from)
            .min(4);
        let chunk_size = cars.len().div_ceil(workers).max(1);
        std::thread::scope(|scope| {
            for chunk in cars.chunks(chunk_size) {
                scope.spawn(move || {
                    for spec in chunk {
                        assert_driving_contract(spec.clone());
                    }
                });
            }
        });
    }
    fn assert_driving_contract(spec: CarSpec) {
        let id = spec.id.clone();
        let mut app = world_for(spec.clone(), false);
        let (parked, velocity, grounded) = state(&mut app);
        assert_eq!(grounded, 4, "{id}: suspension support");
        assert!(
            velocity.linear.length() < 0.1,
            "{id}: parked velocity {velocity:?}"
        );
        tick(&mut app, 600);
        let (still, _, _) = state(&mut app);
        assert!(
            (still.translation - parked.translation).length() < 0.15,
            "{id}: parked drift"
        );

        app.world_mut().resource_mut::<DriveInput>().throttle = 1.0;
        tick(&mut app, 720);
        let (_, velocity, _) = state(&mut app);
        let before = velocity.linear.length();
        assert!(before > 15.0, "{id}: acceleration {before}");
        let mut wheels = app.world_mut().query::<&Vehicle>();
        assert!(
            wheels
                .single(app.world())
                .unwrap()
                .wheel_spin
                .iter()
                .all(|x| x.is_finite()),
            "{id}: wheel animation"
        );
        *app.world_mut().resource_mut::<DriveInput>() = DriveInput {
            brake: 1.0,
            ..default()
        };
        tick(&mut app, 240);
        assert!(
            state(&mut app).1.linear.length() < before * 0.6,
            "{id}: braking"
        );

        *app.world_mut().resource_mut::<DriveInput>() = DriveInput {
            reset: true,
            ..default()
        };
        tick(&mut app, 240);
        app.world_mut().resource_mut::<DriveInput>().throttle = 1.0;
        tick(&mut app, 360);
        app.world_mut().resource_mut::<DriveInput>().steer = 0.5;
        tick(&mut app, 120);
        let (turned, _, grounded) = state(&mut app);
        assert!(
            (turned.rotation * Vec3::NEG_Z).x > 0.05 && grounded >= 2,
            "{id}: turning {turned:?}"
        );
        app.world_mut().resource_mut::<DriveInput>().handbrake = true;
        tick(&mut app, 60);
        assert!(
            state(&mut app).0.translation.is_finite(),
            "{id}: handbrake stability"
        );

        *app.world_mut().resource_mut::<DriveInput>() = DriveInput {
            reset: true,
            ..default()
        };
        tick(&mut app, 240);
        assert!(
            state(&mut app).1.linear.length() < 0.1,
            "{id}: recovery stopped"
        );
        app.world_mut().resource_mut::<DriveInput>().brake = 1.0;
        tick(&mut app, 1200);
        let (_, velocity, grounded) = state(&mut app);
        assert!(
            velocity.linear.length() > 5.0 && velocity.linear.length() < 8.1 && grounded == 4,
            "{id}: reverse {velocity:?}"
        );

        let mut wall = world_for(spec, true);
        wall.world_mut().resource_mut::<DriveInput>().throttle = 1.0;
        tick(&mut wall, 1200);
        assert!(
            state(&mut wall).0.translation.z > 121.0,
            "{id}: collision penetration"
        );
        println!(
            "FLEET PHYSICS PASS {id}: park, four contacts, accelerate, brake, turn, handbrake, reset, reverse, collision"
        );
    }
    #[test]
    fn physics_fixture_matches_default_executor_at_every_tick() {
        let mut reference = world_for_scheduling(CarSpec::default(), true, false);
        let mut fixture = world_for(CarSpec::default(), true);
        let observe = |app: &mut App| {
            let (pose, velocity, grounded) = state(app);
            let mut cars = app.world_mut().query::<&Vehicle>();
            let car = cars.single(app.world()).unwrap();
            (
                pose.translation,
                pose.rotation,
                velocity.linear,
                velocity.angular,
                grounded,
                car.wheel_spin,
                car.wheel_lengths,
            )
        };
        for (input, ticks) in [
            (
                DriveInput {
                    throttle: 1.0,
                    ..default()
                },
                720,
            ),
            (
                DriveInput {
                    throttle: 1.0,
                    steer: 0.5,
                    ..default()
                },
                120,
            ),
            (
                DriveInput {
                    throttle: 1.0,
                    steer: 0.5,
                    handbrake: true,
                    ..default()
                },
                60,
            ),
            (
                DriveInput {
                    brake: 1.0,
                    ..default()
                },
                240,
            ),
            (
                DriveInput {
                    reset: true,
                    ..default()
                },
                1,
            ),
            (
                DriveInput {
                    brake: 1.0,
                    ..default()
                },
                240,
            ),
        ] {
            *reference.world_mut().resource_mut::<DriveInput>() = input;
            *fixture.world_mut().resource_mut::<DriveInput>() = input;
            for tick_index in 0..ticks {
                tick(&mut reference, 1);
                tick(&mut fixture, 1);
                assert_eq!(
                    observe(&mut fixture),
                    observe(&mut reference),
                    "tick {tick_index}"
                );
            }
        }
    }
    #[test]
    fn suspension_settles_on_four_wheels() {
        let mut app = world(false);
        let (pose, vel, grounded) = state(&mut app);
        assert_eq!(grounded, 4);
        assert!((0.4..0.7).contains(&pose.translation.y), "{pose:?}");
        assert!(vel.linear.length() < 0.1, "{vel:?}");
    }
    #[test]
    fn parked_car_does_not_creep_on_level_ground() {
        let mut app = world(false);
        let (before, _, _) = state(&mut app);
        tick(&mut app, 1200);
        let (after, velocity, _) = state(&mut app);
        let movement = Vec2::new(
            after.translation.x - before.translation.x,
            after.translation.z - before.translation.z,
        )
        .length();
        assert!(
            movement < 0.15,
            "parked movement={movement}, velocity={velocity:?}"
        );
    }
    #[test]
    fn acceleration_and_brakes_change_real_chassis_velocity() {
        let mut app = world(false);
        app.world_mut().resource_mut::<DriveInput>().throttle = 1.0;
        tick(&mut app, 720);
        let (pose, vel, _) = state(&mut app);
        let before = -vel.linear.z;
        assert!(before > 15.0, "{vel:?}");
        assert!(pose.translation.z < 140.0, "{pose:?}");
        *app.world_mut().resource_mut::<DriveInput>() = DriveInput {
            brake: 1.0,
            ..default()
        };
        tick(&mut app, 240);
        let (_, after, _) = state(&mut app);
        assert!(
            after.linear.length() < before * 0.6,
            "before={before} after={after:?}"
        );
    }
    #[test]
    fn wheel_rotation_matches_forward_rolling_direction() {
        let mut app = world(false);
        app.world_mut().resource_mut::<DriveInput>().throttle = 1.0;
        tick(&mut app, 60);
        let mut query = app.world_mut().query::<&Vehicle>();
        let car = query.single(app.world()).unwrap();
        // Forward=-Z requires omega=-X: the bottom surface then travels +Z.
        assert!(car.speed > 0.5);
        assert!(
            car.wheel_spin.iter().all(|spin| *spin < 0.0),
            "{:?}",
            car.wheel_spin
        );
    }
    #[test]
    fn steering_right_turns_and_remains_supported() {
        let mut app = world(false);
        app.world_mut().resource_mut::<DriveInput>().throttle = 1.0;
        tick(&mut app, 360);
        app.world_mut().resource_mut::<DriveInput>().steer = 0.5;
        tick(&mut app, 120);
        let (pose, _, grounded) = state(&mut app);
        assert!(pose.translation.x > 4.2, "{pose:?}");
        assert!((pose.rotation * Vec3::NEG_Z).x > 0.05, "{pose:?}");
        assert!(grounded >= 2, "{grounded}");
    }
    #[test]
    fn solid_wall_stops_the_vehicle() {
        let mut app = world(true);
        app.world_mut().resource_mut::<DriveInput>().throttle = 1.0;
        tick(&mut app, 1200);
        let (pose, _, _) = state(&mut app);
        assert!(pose.translation.z > 121.0, "wall penetration: {pose:?}");
        assert!(pose.translation.z < 125.0, "did not reach wall: {pose:?}");
    }
    #[test]
    fn reset_recovers_inverted_car_on_a_road_and_clears_motion() {
        let mut app = world(false);
        let mut q = app.world_mut().query::<(&mut Transform, &mut Velocity)>();
        for (mut pose, mut vel) in q.iter_mut(app.world_mut()) {
            pose.translation = Vec3::new(47.0, 3.0, 71.0);
            pose.rotation = Quat::from_rotation_z(std::f32::consts::PI);
            vel.linear = Vec3::new(12.0, 0.0, 4.0);
        }
        app.world_mut().resource_mut::<DriveInput>().reset = true;
        tick(&mut app, 2);
        let (pose, vel, _) = state(&mut app);
        assert!((pose.rotation * Vec3::Y).y > 0.99);
        assert!((pose.translation.z - 108.8).abs() < 0.1, "{pose:?}");
        assert!(
            Vec2::new(vel.linear.x, vel.linear.z).length() < 0.1,
            "{vel:?}"
        );
    }
    #[test]
    fn low_speed_brake_becomes_reverse() {
        let mut app = world(false);
        app.world_mut().resource_mut::<DriveInput>().brake = 1.0;
        tick(&mut app, 360);
        let (_, vel, grounded) = state(&mut app);
        assert!(vel.linear.z > 5.0, "{vel:?}");
        assert_eq!(grounded, 4);
        tick(&mut app, 1200);
        let (_, vel, _) = state(&mut app);
        assert!(vel.linear.length() < 8.1, "reverse limiter: {vel:?}");
    }
    #[test]
    fn brake_does_not_engage_reverse_while_sliding_sideways() {
        let mut app = world(false);
        let mut query = app.world_mut().query::<&mut Velocity>();
        query.single_mut(app.world_mut()).unwrap().linear = Vec3::X * 8.0;
        app.world_mut().resource_mut::<DriveInput>().brake = 1.0;
        app.update();
        let mut query = app.world_mut().query::<&Vehicle>();
        assert_eq!(query.single(app.world()).unwrap().gear, 1);

        // Backward velocity may display R but must not select reverse drive.
        let mut query = app.world_mut().query::<&mut Velocity>();
        query.single_mut(app.world_mut()).unwrap().linear = Vec3::X * 8.0 + Vec3::Z * 2.0;
        for _ in 0..4 {
            app.update();
            let mut query = app.world_mut().query::<(&Vehicle, &ExternalForce)>();
            let (car, force) = query.single(app.world()).unwrap();
            assert!(!car.reverse_engaged);
            assert!(
                force.force.z < 0.0,
                "braking must oppose backward motion: {force:?}"
            );
        }
    }

    #[test]
    fn pause_freezes_physics_then_resume_and_quit_work() {
        use crate::presentation::{Session, read_controls};
        let mut app = world(false);
        app.add_plugins(crate::runtime::RuntimePlugin)
            .init_resource::<Session>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(
                Update,
                (read_controls, crate::runtime::dispatch_update).chain(),
            );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        let (before, _, _) = state(&mut app);
        tick(&mut app, 120);
        let (after, _, _) = state(&mut app);
        assert_eq!(before.translation, after.translation);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        tick(&mut app, 120);
        let (_, velocity, _) = state(&mut app);
        assert!(-velocity.linear.z > 2.0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyQ);
        app.update();
        assert!(app.should_exit().is_some());
    }

    #[test]
    fn analog_gamepad_input_drives_the_same_controller() {
        use crate::presentation::{Session, read_controls};
        let mut app = world(false);
        app.add_plugins(crate::runtime::RuntimePlugin)
            .init_resource::<Session>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(
                Update,
                (read_controls, crate::runtime::dispatch_update).chain(),
            );
        let mut pad = Gamepad::default();
        pad.analog_mut().set(GamepadButton::RightTrigger2, 0.65);
        pad.analog_mut().set(GamepadAxis::LeftStickX, 0.5);
        let pad_id = app.world_mut().spawn(pad).id();
        tick(&mut app, 240);
        let input = app.world().resource::<DriveInput>();
        assert!((input.throttle - 0.65).abs() < 0.001);
        assert!(input.steer > 0.3 && input.steer < 0.6);
        let (pose, velocity, _) = state(&mut app);
        assert!(velocity.linear.length() > 4.0 && pose.translation.x > 3.3);
        app.world_mut().despawn(pad_id);
        app.update();
        assert_eq!(app.world().resource::<DriveInput>().throttle, 0.0);
    }
    #[test]
    fn simulation_does_not_depend_on_render_cadence() {
        let mut fast = world(false);
        let mut slow = world(false);
        fast.world_mut().resource_mut::<DriveInput>().throttle = 1.0;
        slow.world_mut().resource_mut::<DriveInput>().throttle = 1.0;
        slow.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            STEP as f64 * 4.0,
        )));
        tick(&mut fast, 960);
        tick(&mut slow, 240);
        let (a, va, _) = state(&mut fast);
        let (b, vb, _) = state(&mut slow);
        assert!(a.translation.distance(b.translation) < 0.5, "{a:?} {b:?}");
        assert!(va.linear.distance(vb.linear) < 0.2, "{va:?} {vb:?}");
    }
}
