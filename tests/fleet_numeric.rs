//! Batch execution must retain production scheduling and scenario outcomes.
#[allow(dead_code)]
#[path = "../examples/stress/case.rs"]
mod case;
#[allow(dead_code)]
#[path = "../examples/stress/city.rs"]
mod city;
#[allow(dead_code)]
#[path = "../examples/stress/fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "../examples/stress/metrics.rs"]
mod metrics;
use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use car_game::{
    RenderBackend, RuntimeConfig,
    roster::Fleet,
    scenario::{ScenarioDefinition, ScenarioOutcome},
    vehicle::Vehicle,
};
use serde_json::{Value, json};

fn config(car: &str) -> RuntimeConfig {
    let value = json!({"schema_version":1,"name":"numeric-equivalence","scene":"driving","seed":17,
        "initial":{"car":car},"max_ticks":601,"max_wall_ms":60000,"readiness_timeout_ms":10000,
        "commands":[{"tick":0,"command":{"action":"drive","throttle":1.0,"brake":0.0,"steer":0.0}},
                    {"tick":240,"command":{"action":"drive","throttle":0.0,"brake":0.8,"steer":0.25}},
                    {"tick":480,"command":{"action":"reset"}}],
        "assertions":[{"tick":400,"kind":"displacement_at_least","meters":1.0},
                      {"tick":600,"kind":"reset_count","count":1}]});
    let definition = ScenarioDefinition::from_json(&value.to_string()).unwrap();
    RuntimeConfig {
        backend: RenderBackend::Disabled,
        mode: "scenario".into(),
        car: car.into(),
        seed: definition.seed,
        access: definition.context.clone(),
        scenario: Some(definition),
        fast_scenario: true,
        ..default()
    }
}
fn sample(app: &mut App) -> [f32; 41] {
    let input = app
        .world()
        .resource::<car_game::vehicle::DriveInput>()
        .to_owned();
    let mut q = app.world_mut().query::<(&Transform, &Velocity, &Vehicle)>();
    let (p, v, c) = q.single(app.world()).unwrap();
    metrics::sample(p, v, c, input)
}
fn comparable(mut value: Value) -> Value {
    value.as_object_mut().unwrap().remove("elapsed_ms");
    value.as_object_mut().unwrap().remove("artifacts");
    value
}
#[test]
fn recorderless_batch_matches_production_ticks_and_assertions() {
    for car in ["350z", "redline-gt500", "ref-a08", "ref-e12", "ref-a14"] {
        let spec = Fleet::default()
            .cars
            .into_iter()
            .find(|s| s.id == car)
            .unwrap();
        let root = std::env::temp_dir().join(format!("numeric-{}-{car}", std::process::id()));
        let recorder = std::sync::Arc::new(
            car_game::diagnostics::RunRecorder::try_create_with_config(
                &root,
                "scenario",
                car,
                car_game::diagnostics::RecorderConfig {
                    queue_records: 32768,
                    queue_bytes: 32 * 1024 * 1024,
                    ..default()
                },
            )
            .unwrap(),
        );
        let mut production = car_game::build_app(config(car), recorder.clone()).unwrap();
        // Construct after provenance I/O so preparation cannot consume batch readiness time.
        let mut batch = city::app(config(car), spec);
        for tick in 0..2000 {
            batch.update();
            production.update();
            assert_eq!(
                sample(&mut batch),
                sample(&mut production),
                "{car} tick {tick}"
            );
            if batch.world().resource::<ScenarioOutcome>().0.is_some() {
                break;
            }
        }
        let a = batch
            .world()
            .resource::<ScenarioOutcome>()
            .0
            .clone()
            .unwrap();
        let b = production
            .world()
            .resource::<ScenarioOutcome>()
            .0
            .clone()
            .unwrap();
        assert_eq!(a["passed"], true, "{car}: {a}");
        assert_eq!(comparable(a), comparable(b), "{car}");
        recorder.finish(production.should_exit().unwrap());
        drop(production);
        drop(recorder);
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn expanded_cases_are_unique_bounded_and_have_no_car_specific_inputs() {
    let mut unique = std::collections::BTreeSet::new();
    for id in 0..5000 {
        let case = case::Case::at(id);
        let mut value = serde_json::to_value(&case).unwrap();
        value.as_object_mut().unwrap().remove("id");
        assert!(unique.insert(value.to_string()), "duplicate {id}");
        assert!((120..=2400).contains(&case.ticks));
        for tick in [0, 24, case.ticks - 1] {
            let input = case.input(tick);
            assert!((0.0..=1.0).contains(&input.throttle));
            assert!((0.0..=1.0).contains(&input.brake));
            assert!(input.steer.abs() <= 1.0);
        }
    }
    assert_eq!(unique.len(), 5000);
}

fn proving_case_resets(id: &str, case: &case::Case) -> u32 {
    let spec = Fleet::default()
        .cars
        .into_iter()
        .find(|s| s.id == id)
        .unwrap();
    let mut app = fixture::flat(spec, case);
    for tick in 0..case.ticks {
        *app.world_mut()
            .resource_mut::<car_game::vehicle::DriveInput>() = case.input(tick);
        app.update();
    }
    let mut q = app.world_mut().query::<&Vehicle>();
    q.single(app.world()).unwrap().reset_count
}

#[test]
fn downhill_proving_ground_does_not_cross_the_city_fall_boundary() {
    let raised = case::Case::at(4409);
    let mut legacy = raised.clone();
    legacy.base_height_m = 0.;
    assert!(
        proving_case_resets("gt-viper", &legacy) > 0,
        "control must reproduce the supported-car reset"
    );
    for id in [
        "gt-viper",
        "ref-e17",
        "redline-gt500",
        "ref-a08",
        "ref-e12",
        "ref-a14",
    ] {
        assert_eq!(
            proving_case_resets(id, &raised),
            0,
            "{id}: intentional downhill track is valid ground"
        );
    }
}
