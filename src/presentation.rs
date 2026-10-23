use crate::{
    config::{RenderBackend, RuntimeConfig},
    roster::{CarSpec, Fleet},
    runtime::{CommandBus, ControllerOwner, ProductionCommand},
    vehicle::{DriveInput, Vehicle, spawn_pose},
};
use bevy::{
    audio::{AudioSinkPlayback, Volume},
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    light::CascadeShadowConfigBuilder,
    prelude::*,
    render::render_resource::TextureFormat,
};
use bevy_rapier3d::prelude::*;

#[derive(Component)]
pub struct VisualCar;
#[derive(Component)]
struct AdditionalVisual(Entity);
#[derive(Component)]
struct WheelVisual(usize);
#[derive(Component)]
pub struct ChaseCamera;
#[derive(Resource, Default)]
pub struct InspectionView(pub Option<Transform>);
#[derive(Resource, Default)]
pub struct QaRenderTarget(pub Option<Handle<Image>>);
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresentationUpdate;
#[derive(Component)]
struct SpeedText;
#[derive(Component)]
struct DetailText;
#[derive(Component)]
struct CarText;
#[derive(Component)]
struct PauseText;
#[derive(Component)]
struct RpmBar;
#[derive(Component)]
struct EngineSound;
#[derive(Component)]
struct TireSound;
pub use crate::runtime::Session;

pub struct PresentationPlugin;
impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InspectionView>()
            .init_resource::<QaRenderTarget>()
            .insert_resource(ClearColor(Color::srgb(0.58, 0.72, 0.81)))
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.80, 0.88, 1.0),
                brightness: 2500.0,
                ..default()
            })
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    read_controls,
                    select_car,
                    crate::runtime::dispatch_update,
                    refresh_car_visual,
                    tag_wheels,
                    follow_vehicle,
                    animate_wheels,
                    additional_visuals,
                    camera_follow,
                    update_hud,
                    update_car_label,
                    update_audio,
                )
                    .chain()
                    .in_set(PresentationUpdate),
            );
    }
}

fn text_style(size: f32, color: Color) -> (TextFont, TextColor) {
    (
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn spawn_visual(commands: &mut Commands, assets: &AssetServer, spec: &CarSpec, pose: Transform) {
    commands.spawn((
        Name::new(format!("{} visual", spec.id)),
        VisualCar,
        Visibility::default(),
        pose,
        children![(
            WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(spec.path.clone()))),
            Transform::from_xyz(0.0, spec.visual_offset, 0.0)
        )],
    ));
}

#[allow(clippy::too_many_lines)]
fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    spec: Res<CarSpec>,
    session: Res<Session>,
    mut images: ResMut<Assets<Image>>,
    config: Res<RuntimeConfig>,
) {
    spawn_visual(&mut commands, &assets, &spec, spawn_pose());
    let camera = commands
        .spawn((
            ChaseCamera,
            IsDefaultUiCamera,
            Camera3d::default(),
            Msaa::Sample4,
            Tonemapping::AgX,
            Projection::Perspective(PerspectiveProjection {
                fov: 65_f32.to_radians(),
                far: 1200.0,
                ..default()
            }),
            Transform::from_xyz(3.2, 3.8, 198.0).looking_at(Vec3::new(3.2, 0.9, 188.0), Vec3::Y),
            DistanceFog {
                color: Color::srgb(0.58, 0.72, 0.81),
                falloff: FogFalloff::Linear {
                    start: 220.0,
                    end: 780.0,
                },
                ..default()
            },
        ))
        .id();
    if config.backend == RenderBackend::Offscreen {
        let target = images.add(Image::new_target_texture(
            1920,
            1080,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
        commands
            .entity(camera)
            .insert(RenderTarget::Image(target.clone().into()));
        commands.insert_resource(QaRenderTarget(Some(target)));
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 18000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.85, -0.55, 0.0)),
        CascadeShadowConfigBuilder {
            maximum_distance: 160.0,
            ..default()
        }
        .build(),
    ));

    let ink = Color::srgb(0.91, 0.93, 0.91);
    let muted = Color::srgb(0.62, 0.69, 0.70);
    let accent = Color::srgb(1.0, 0.54, 0.23);
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(28),
            left: px(32),
            padding: UiRect::all(px(12)),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.055, 0.07, 0.82)),
        children![
            (Text::new("GRID / FREE DRIVE"), text_style(25.0, ink)),
            (Text::new("A CITY TO YOURSELF"), text_style(12.0, accent)),
        ],
    ));
    commands.spawn((
        Text::new(""),
        CarText,
        text_style(15.0, ink),
        Node {
            position_type: PositionType::Absolute,
            top: px(30),
            right: px(34),
            padding: UiRect::all(px(12)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.055, 0.07, 0.82)),
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(30),
            left: px(32),
            padding: UiRect::all(px(16)),
            flex_direction: FlexDirection::Column,
            row_gap: px(7),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.055, 0.07, 0.82)),
        children![
            (
                Text::new("WASD / ARROWS  DRIVE    SPACE  HANDBRAKE"),
                text_style(13.0, ink)
            ),
            (
                Text::new("RT / LT  DRIVE / BRAKE    LEFT STICK  STEER    A  HANDBRAKE"),
                text_style(12.0, muted)
            ),
            (
                Text::new("R / X  RESET    ESC / START  PAUSE    M  SOUND"),
                text_style(12.0, muted)
            ),
            (
                Text::new("[ / ]  CAR    LB / RB  CAR"),
                text_style(12.0, accent)
            ),
        ],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(30),
            right: px(32),
            width: px(235),
            padding: UiRect::all(px(20)),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.055, 0.07, 0.88)),
        children![
            (Text::new("000"), text_style(76.0, ink), SpeedText),
            (
                Text::new("KM/H    GEAR 1"),
                text_style(14.0, accent),
                DetailText
            ),
            (
                Node {
                    height: px(4),
                    width: percent(100),
                    margin: UiRect::top(px(12)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.22, 0.24)),
                children![(
                    Node {
                        height: percent(100),
                        width: percent(10),
                        ..default()
                    },
                    BackgroundColor(accent),
                    RpmBar
                )]
            ),
        ],
    ));
    commands.spawn((
        PauseText,
        Text::new(""),
        text_style(28.0, ink),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: percent(38),
            width: percent(100),
            padding: UiRect::all(px(28)),
            ..default()
        },
        Visibility::Hidden,
        BackgroundColor(Color::srgba(0.025, 0.04, 0.055, 0.92)),
    ));
    commands.spawn((
        EngineSound,
        AudioPlayer::new(assets.load("audio/engine.wav")),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(if session.muted { 0.0 } else { 0.15 })),
    ));
    commands.spawn((
        TireSound,
        AudioPlayer::new(assets.load("audio/tires.wav")),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
    ));
}

pub(crate) fn select_car(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut bus: ResMut<CommandBus>,
) {
    let mut previous = keys.just_pressed(KeyCode::BracketLeft);
    let mut next = keys.just_pressed(KeyCode::BracketRight);
    for pad in &pads {
        previous |= pad.just_pressed(GamepadButton::LeftTrigger);
        next |= pad.just_pressed(GamepadButton::RightTrigger);
    }
    if previous || next {
        bus.submit(
            ControllerOwner::LegacyComposite,
            ProductionCommand::CycleCar {
                direction: i32::from(next) - i32::from(previous),
            },
        );
    }
}

fn refresh_car_visual(
    mut commands: Commands,
    assets: Res<AssetServer>,
    spec: Res<CarSpec>,
    cars: Query<&Transform, With<crate::controls::PlayerVehicle>>,
    visuals: Query<Entity, (With<VisualCar>, Without<AdditionalVisual>)>,
) {
    if !spec.is_changed() || spec.is_added() {
        return;
    }
    let Ok(pose) = cars.single() else {
        return;
    };
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    spawn_visual(&mut commands, &assets, &spec, *pose);
}

fn update_car_label(
    spec: Res<CarSpec>,
    fleet: Res<Fleet>,
    mut labels: Query<&mut Text, With<CarText>>,
) {
    for mut text in &mut labels {
        text.0 = format!(
            "{}\n{}{}  /  {:02} OF {:02}",
            spec.name,
            spec.game,
            if spec.variant == "add-on" {
                " (ADD-ON)"
            } else {
                ""
            },
            fleet.selected + 1,
            fleet.cars.len()
        );
    }
}

pub fn deadzone(value: f32) -> f32 {
    if value.abs() <= 0.12 {
        0.0
    } else {
        value.signum() * ((value.abs() - 0.12) / 0.88).min(1.0)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn read_controls(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    session: Res<Session>,
    mut bus: ResMut<CommandBus>,
) {
    let mut pause = keys.just_pressed(KeyCode::Escape);
    let mut reset = keys.just_pressed(KeyCode::KeyR);
    let mut quit = keys.just_pressed(KeyCode::KeyQ);
    let pressed = |a, b| keys.pressed(a) || keys.pressed(b);
    let mut throttle = if pressed(KeyCode::KeyW, KeyCode::ArrowUp) {
        1.0_f32
    } else {
        0.0
    };
    let mut brake = if pressed(KeyCode::KeyS, KeyCode::ArrowDown) {
        1.0_f32
    } else {
        0.0
    };
    let mut steer = f32::from(pressed(KeyCode::KeyD, KeyCode::ArrowRight))
        - f32::from(pressed(KeyCode::KeyA, KeyCode::ArrowLeft));
    let mut handbrake = keys.pressed(KeyCode::Space);
    for pad in &gamepads {
        pause |= pad.just_pressed(GamepadButton::Start);
        reset |= pad.just_pressed(GamepadButton::West);
        quit |= pad.just_pressed(GamepadButton::Select);
        throttle = throttle.max(pad.get(GamepadButton::RightTrigger2).unwrap_or(0.0));
        brake = brake.max(pad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0));
        let axis = deadzone(pad.get(GamepadAxis::LeftStickX).unwrap_or(0.0));
        if axis.abs() > steer.abs() {
            steer = axis;
        }
        handbrake |= pad.pressed(GamepadButton::South);
    }
    let source = ControllerOwner::LegacyComposite;
    if pause {
        bus.submit(
            source,
            ProductionCommand::SetPaused {
                paused: !session.paused,
            },
        );
    }
    if keys.just_pressed(KeyCode::KeyM) {
        bus.submit(source, ProductionCommand::ToggleMute);
    }
    if quit {
        bus.submit(source, ProductionCommand::Quit);
    }
    bus.submit(
        source,
        ProductionCommand::Drive {
            throttle: throttle.clamp(0.0, 1.0),
            brake: brake.clamp(0.0, 1.0),
            steer: steer.clamp(-1.0, 1.0),
            handbrake,
        },
    );
    if reset {
        bus.submit(source, ProductionCommand::Reset);
    }
}

fn tag_wheels(mut commands: Commands, names: Query<(Entity, &Name), Added<Name>>) {
    for (entity, name) in &names {
        let s = name.as_str();
        if s.starts_with("wheel_") || s.starts_with("tire_") {
            let i = match s.trim_start_matches("wheel_").trim_start_matches("tire_") {
                "front_left" => 0,
                "front_right" => 1,
                "rear_left" => 2,
                "rear_right" => 3,
                _ => continue,
            };
            commands.entity(entity).insert(WheelVisual(i));
        }
    }
}

// The visual and chassis transforms must be disjoint ECS queries.
#[allow(clippy::type_complexity)]
fn follow_vehicle(
    time: Res<Time<Fixed>>,
    cars: Query<(&Transform, &Vehicle), With<crate::controls::PlayerVehicle>>,
    mut visuals: Query<
        &mut Transform,
        (With<VisualCar>, Without<Vehicle>, Without<AdditionalVisual>),
    >,
) {
    let Ok((pose, car)) = cars.single() else {
        return;
    };
    let t = time.overstep_fraction();
    for mut visual in &mut visuals {
        visual.translation = car.previous.translation.lerp(pose.translation, t);
        visual.rotation = car.previous.rotation.slerp(pose.rotation, t);
    }
}

fn animate_wheels(
    spec: Res<CarSpec>,
    cars: Query<&Vehicle, With<crate::controls::PlayerVehicle>>,
    mut wheels: Query<(&WheelVisual, &mut Transform)>,
) {
    let Ok(car) = cars.single() else {
        return;
    };
    for (wheel, mut transform) in &mut wheels {
        let i = wheel.0;
        transform.translation.y = spec.mounts()[i].y - car.wheel_lengths[i] - spec.visual_offset;
        transform.rotation = Quat::from_rotation_y(if i < 2 { car.steering } else { 0.0 })
            * Quat::from_rotation_x(car.wheel_spin[i]);
    }
}

// These filters make mutable camera transforms explicitly disjoint from car transforms.
#[allow(clippy::type_complexity)]
fn camera_follow(
    time: Res<Time>,
    inspection: Res<InspectionView>,
    context: ReadRapierContext,
    cars: Query<(Entity, &Vehicle), With<crate::controls::PlayerVehicle>>,
    visuals: Query<&Transform, (With<VisualCar>, Without<AdditionalVisual>)>,
    mut cameras: Query<(&mut Transform, &mut Projection), (With<ChaseCamera>, Without<VisualCar>)>,
) {
    let (Ok((entity, car)), Ok(visual)) = (cars.single(), visuals.single()) else {
        return;
    };
    let forward = visual.rotation * Vec3::NEG_Z;
    let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let focus = visual.translation + Vec3::Y * 0.8 + flat_forward * 1.6;
    let desired =
        visual.translation - flat_forward * (7.5 + car.speed.abs() * 0.045) + Vec3::Y * 3.0;
    for (mut camera, mut projection) in &mut cameras {
        if let Some(pose) = inspection.0 {
            *camera = pose;
            continue;
        }
        let mut desired = desired;
        if let Ok(context) = context.single() {
            let delta = desired - focus;
            if let Some((_, distance)) = context.cast_ray(
                focus,
                delta.normalize_or_zero(),
                delta.length(),
                true,
                QueryFilter::default().exclude_rigid_body(entity),
            ) {
                desired = focus + delta.normalize_or_zero() * (distance - 0.6).max(0.7);
            }
        }
        let alpha = 1.0 - (-7.0 * time.delta_secs()).exp();
        camera.translation = camera.translation.lerp(desired, alpha);
        let target = camera.looking_at(focus, Vec3::Y).rotation;
        camera.rotation = camera.rotation.slerp(target, alpha);
        if let Projection::Perspective(p) = &mut *projection {
            let target = (65.0 + car.speed.abs() * 0.16).min(77.0).to_radians();
            p.fov += (target - p.fov) * alpha;
        }
    }
}

// Disjoint marker filters are required for Bevy's mutable text queries.
#[allow(clippy::type_complexity)]
fn update_hud(
    cars: Query<&Vehicle, With<crate::controls::PlayerVehicle>>,
    session: Res<Session>,
    mut speed: Query<&mut Text, (With<SpeedText>, Without<DetailText>, Without<PauseText>)>,
    mut detail: Query<&mut Text, (With<DetailText>, Without<SpeedText>, Without<PauseText>)>,
    mut pause: Query<
        (&mut Text, &mut Visibility),
        (With<PauseText>, Without<DetailText>, Without<SpeedText>),
    >,
    mut bars: Query<&mut Node, With<RpmBar>>,
) {
    let Ok(car) = cars.single() else {
        return;
    };
    for mut text in &mut speed {
        text.0 = format!("{:03}", (car.speed.abs() * 3.6).round() as u32);
    }
    for mut text in &mut detail {
        let gear = if car.gear < 0 {
            "R".to_string()
        } else {
            car.gear.to_string()
        };
        text.0 = format!(
            "KM/H    GEAR {gear}{}",
            if session.muted { "    MUTED" } else { "" }
        );
    }
    for (mut text, mut visible) in &mut pause {
        *visible = if session.paused {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        text.0 = "PAUSED\n\nESC / START  RESUME\nQ / BACK  QUIT".to_string();
    }
    for mut node in &mut bars {
        node.width = percent((car.rpm / 7000.0 * 100.0).min(100.0));
    }
}

// Bevy injects these resources and disjoint sink queries as system parameters.
#[allow(clippy::too_many_arguments)]
fn update_audio(
    mut commands: Commands,
    assets: Res<AssetServer>,
    time: Res<Time>,
    session: Res<Session>,
    input: Res<DriveInput>,
    cars: Query<(&Vehicle, &Velocity), With<crate::controls::PlayerVehicle>>,
    mut engines: Query<&mut AudioSink, (With<EngineSound>, Without<TireSound>)>,
    mut tires: Query<&mut AudioSink, (With<TireSound>, Without<EngineSound>)>,
    mut previous_velocity: Local<Vec3>,
    mut impact_cooldown: Local<f32>,
    mut previous_reset: Local<u32>,
) {
    let Ok((car, velocity)) = cars.single() else {
        return;
    };
    let active = !session.paused && !session.muted;
    for mut sink in &mut engines {
        sink.set_speed((car.rpm / 1800.0).clamp(0.5, 3.8));
        sink.set_volume(Volume::Linear(if active {
            0.12 + input.throttle * 0.16
        } else {
            0.0
        }));
    }
    for mut sink in &mut tires {
        let volume = ((car.slip - 0.12) * 1.6).clamp(0.0, 0.5);
        sink.set_volume(Volume::Linear(if active { volume } else { 0.0 }));
    }
    *impact_cooldown -= time.delta_secs();
    let delta = (velocity.linear - *previous_velocity).length();
    let resetting = car.reset_count != *previous_reset;
    *previous_reset = car.reset_count;
    if active && delta > 3.2 && !resetting && *impact_cooldown <= 0.0 {
        commands.spawn((
            AudioPlayer::new(assets.load("audio/impact.wav")),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear((delta / 18.0).clamp(0.1, 0.65))),
        ));
        *impact_cooldown = 0.3;
    }
    *previous_velocity = velocity.linear;
}

// Each extra visual follows its own chassis; the selected player's HUD/camera remain scoped.
#[allow(clippy::type_complexity)]
fn additional_visuals(
    mut commands: Commands,
    assets: Res<AssetServer>,
    time: Res<Time<Fixed>>,
    cars: Query<
        (
            Entity,
            &Transform,
            &Vehicle,
            &crate::controls::VehicleControl,
        ),
        Without<crate::controls::PlayerVehicle>,
    >,
    mut visuals: Query<(&AdditionalVisual, &mut Transform), (With<VisualCar>, Without<Vehicle>)>,
    parents: Query<&ChildOf>,
    mut wheels: Query<
        (Entity, &WheelVisual, &mut Transform),
        (Without<VisualCar>, Without<Vehicle>),
    >,
) {
    for (body, pose, car, control) in &cars {
        if !visuals.iter().any(|(v, _)| v.0 == body) {
            commands.spawn((
                VisualCar,
                AdditionalVisual(body),
                Visibility::default(),
                *pose,
                children![(
                    WorldAssetRoot(
                        assets.load(GltfAssetLabel::Scene(0).from_asset(control.spec.path.clone()))
                    ),
                    Transform::from_xyz(0.0, control.spec.visual_offset, 0.0)
                )],
            ));
        }
        for (visual, mut transform) in &mut visuals {
            if visual.0 != body {
                continue;
            }
            let t = time.overstep_fraction();
            transform.translation = car.previous.translation.lerp(pose.translation, t);
            transform.rotation = car.previous.rotation.slerp(pose.rotation, t);
        }
    }
    for (entity, wheel, mut transform) in &mut wheels {
        let Some(body) = parents
            .iter_ancestors(entity)
            .find_map(|p| visuals.get(p).ok().map(|(v, _)| v.0))
        else {
            continue;
        };
        let Ok((_, _, car, control)) = cars.get(body) else {
            continue;
        };
        let i = wheel.0;
        transform.translation.y =
            control.spec.mounts()[i].y - car.wheel_lengths[i] - control.spec.visual_offset;
        transform.rotation = Quat::from_rotation_y(if i < 2 { car.steering } else { 0.0 })
            * Quat::from_rotation_x(car.wheel_spin[i]);
    }
}
