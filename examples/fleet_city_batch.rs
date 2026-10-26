//! Compact original-scenario execution using the production ScenarioPlugin.
// Shared modules also provide the standalone proving-ground runner.
#[allow(dead_code)]
#[path = "stress/case.rs"]
mod case;
// Shared modules also provide the standalone proving-ground runner.
#[allow(dead_code)]
#[path = "stress/city.rs"]
mod city;
// Shared modules also provide the standalone proving-ground runner.
#[allow(dead_code)]
#[path = "stress/fixture.rs"]
mod fixture;
// Shared modules also provide the standalone proving-ground runner.
#[allow(dead_code)]
#[path = "stress/metrics.rs"]
mod metrics;
use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use car_game::{
    RenderBackend, RuntimeConfig,
    roster::Fleet,
    runtime::SimulationClock,
    scenario::{ScenarioDefinition, ScenarioOutcome},
    vehicle::Vehicle,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::Write;

fn execute(definition: Value) -> (Value, metrics::Metrics, String, Vec<[f32; 41]>) {
    let definition = ScenarioDefinition::from_json(&definition.to_string()).unwrap();
    let spec = Fleet::default()
        .cars
        .into_iter()
        .find(|s| s.id == definition.initial.car)
        .unwrap();
    let config = RuntimeConfig {
        backend: RenderBackend::Disabled,
        mode: "scenario".into(),
        car: spec.id.clone(),
        seed: definition.seed,
        access: definition.context.clone(),
        scenario: Some(definition),
        fast_scenario: true,
        ..default()
    };
    let mut app = city::app(config, spec);
    let mut frames = Vec::new();
    let mut last = 0;
    for _ in 0..20000 {
        app.update();
        let completed = app.world().resource::<SimulationClock>().completed;
        if completed > last {
            let input = app.world().resource::<city::EffectiveInput>().0;
            let mut q = app.world_mut().query::<(&Transform, &Velocity, &Vehicle)>();
            let (p, v, c) = q.single(app.world()).unwrap();
            frames.push((completed, metrics::sample(p, v, c, input)));
            last = completed;
        }
        if app.world().resource::<ScenarioOutcome>().0.is_some() {
            break;
        }
    }
    let summary = app
        .world()
        .resource::<ScenarioOutcome>()
        .0
        .clone()
        .expect("bounded scenario completed");
    let ready = summary["readiness"]["tick"].as_u64().unwrap();
    let ticks = summary["ticks"].as_u64().unwrap();
    let start = frames.iter().find(|(tick, _)| *tick == ready).unwrap().1;
    let frames: Vec<_> = frames
        .into_iter()
        .filter(|(tick, _)| *tick > ready && *tick <= ready + ticks)
        .map(|(_, f)| f)
        .collect();
    assert_eq!(frames.len(), ticks as usize);
    let mut metrics = metrics::Metrics::new(Vec3::from_slice(&start[..3]));
    let mut hash = Sha256::new();
    for frame in &frames {
        metrics.observe(frame, 0.);
        metrics::hash_frame(&mut hash, frame);
    }
    (summary, metrics, format!("{:x}", hash.finalize()), frames)
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let car = args.get(1).expect("car");
    let file = args.get(2).expect("definition manifest");
    let definitions: Vec<Value> = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let mut output = std::io::stdout().lock();
    for row in definitions {
        let name = row["case"].as_str().unwrap();
        let mut def = row["definition"].clone();
        def["initial"]["car"] = json!(car);
        def["name"] = json!(format!("fleet40-{name}-{car}"));
        for a in def["assertions"].as_array_mut().unwrap() {
            if a["kind"] == "selected_car" {
                a["car"] = json!(car);
            }
        }
        let (a, metrics, h1, t1) = execute(def.clone());
        let (b, other, h2, t2) = execute(def);
        let mut a_compare = a.clone();
        let mut b_compare = b.clone();
        a_compare.as_object_mut().unwrap().remove("elapsed_ms");
        b_compare.as_object_mut().unwrap().remove("elapsed_ms");
        let mut traces = Vec::new();
        if a["passed"] != true || b["passed"] != true || h1 != h2 || a_compare != b_compare {
            let root = std::path::PathBuf::from(args.get(3).expect("failure trace root"));
            traces.push(metrics::trace_file(&root, car, name, 1, &t1));
            if h1 != h2 {
                traces.push(metrics::trace_file(&root, car, name, 2, &t2));
            }
        }
        let row = json!({"car":car,"case":name,"traces":traces,"repeats":2,"repeat_hashes":[h1,h2],"summary":a,"repeat_summary_equal":a_compare==b_compare,"metrics":metrics,"second_metrics":if h1!=h2 {Some(other)} else {None}});
        writeln!(output, "{row}").unwrap();
        output.flush().unwrap();
    }
}
