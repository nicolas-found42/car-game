//! Headless assembly of the production scenario, command, city and vehicle plugins.
use bevy::{asset::AssetApp, prelude::*, time::TimeUpdateStrategy};
use car_game::{
    RuntimeConfig,
    assembly::AssemblyPlugin,
    city::CityPlugin,
    roster::{CarSpec, Fleet},
    runtime::{CommandSet, RuntimePlugin, Session},
    scenario::ScenarioPlugin,
    vehicle::{DriveInput, VehiclePlugin},
};
use std::time::Duration;

#[derive(Resource, Default)]
pub struct EffectiveInput(pub DriveInput);
fn consume(input: Res<DriveInput>, mut observed: ResMut<EffectiveInput>) {
    observed.0 = *input;
}
pub fn app(config: RuntimeConfig, spec: CarSpec) -> App {
    let mut app = App::new();
    // These fixtures have no scene hierarchy worth asynchronous dirty-tree tracking.
    app.insert_resource(bevy::transform::systems::StaticTransformOptimizations::Disabled);
    app.add_plugins((
        MinimalPlugins.set(super::fixture::task_pool()),
        TransformPlugin,
        bevy::input::InputPlugin,
        AssetPlugin {
            file_path: config.asset_root.to_string_lossy().into_owned(),
            ..default()
        },
    ))
    .init_asset::<Image>()
    .init_asset::<Mesh>()
    .init_asset::<StandardMaterial>()
    .insert_resource(Fleet {
        cars: vec![spec.clone()],
        selected: 0,
        pending: None,
    })
    .insert_resource(spec)
    .insert_resource(config.access.clone())
    .insert_resource(Session::from_env())
    .insert_resource(config)
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1. / 120.,
    )))
    .init_resource::<EffectiveInput>()
    .add_plugins((
        RuntimePlugin,
        AssemblyPlugin,
        VehiclePlugin,
        CityPlugin,
        ScenarioPlugin,
    ))
    .add_systems(FixedFirst, consume.after(CommandSet::Dispatch))
    .add_systems(Update, car_game::runtime::dispatch_update);
    super::fixture::single_thread(&mut app);
    app
}
