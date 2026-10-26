//! Common proving-ground matrix, executing production VehiclePlugin on fresh worlds.
#[path = "stress/case.rs"]
mod case;
#[path = "stress/fixture.rs"]
mod fixture;
#[path = "stress/metrics.rs"]
mod metrics;
use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use car_game::{
    roster::{CarSpec, Fleet},
    vehicle::{DriveInput, Vehicle},
};
use case::Case;
use sha2::{Digest, Sha256};
use std::{
    io::{BufWriter, Write},
    path::PathBuf,
};

fn execute(spec: CarSpec, case: &Case) -> (metrics::Metrics, String, Vec<[f32; 41]>) {
    let mut app = fixture::flat(spec, case);
    let mut q = app.world_mut().query::<(&Transform, &Velocity, &Vehicle)>();
    let start = q.single(app.world()).unwrap().0.translation;
    let mut metrics = metrics::Metrics::new(start);
    let mut hash = Sha256::new();
    let mut trace = Vec::with_capacity(case.ticks);
    for tick in 0..case.ticks {
        let input = case.input(tick);
        *app.world_mut().resource_mut::<DriveInput>() = input;
        app.update();
        let (p, v, c) = q.single(app.world()).unwrap();
        let frame = metrics::sample(p, v, c, input);
        metrics.observe(&frame, case.entry_mps);
        metrics::hash_frame(&mut hash, &frame);
        trace.push(frame);
    }
    (metrics, format!("{:x}", hash.finalize()), trace)
}
fn option(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .map(|i| args.get(i + 1).expect("option value").clone())
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|a| a == "--manifest") {
        println!(
            "{}",
            serde_json::to_string_pretty(&(0..5000).map(Case::at).collect::<Vec<_>>()).unwrap()
        );
        return;
    }
    let car = option(&args, "--car").expect("--car required");
    let start = option(&args, "--start").map_or(0, |v| v.parse::<usize>().unwrap());
    let end = option(&args, "--end").map_or(5000, |v| v.parse::<usize>().unwrap());
    assert!(start < end && end <= 5000);
    let root = PathBuf::from(
        option(&args, "--failures").unwrap_or_else(|| "artifacts/stress-failures".into()),
    );
    let spec = Fleet::default()
        .cars
        .into_iter()
        .find(|s| s.id == car)
        .expect("known car");
    let mut output = BufWriter::new(std::io::stdout().lock());
    let selected = option(&args, "--cases").map(|s| {
        s.split(',')
            .map(|v| v.parse::<usize>().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
    });
    for id in (start..end).filter(|id| selected.as_ref().is_none_or(|s| s.contains(id))) {
        let case = Case::at(id);
        let (first, h1, t1) = execute(spec.clone(), &case);
        let (second, h2, t2) = execute(spec.clone(), &case);
        let failures = first.failures(&case.family, case.entry_mps);
        let second_failures = second.failures(&case.family, case.entry_mps);
        let mut traces = Vec::new();
        if !failures.is_empty() || !second_failures.is_empty() || h1 != h2 {
            traces.push(metrics::trace_file(
                &root,
                &car,
                &format!("{id:04}"),
                1,
                &t1,
            ));
            if h1 != h2 {
                traces.push(metrics::trace_file(
                    &root,
                    &car,
                    &format!("{id:04}"),
                    2,
                    &t2,
                ));
            }
        }
        let row = serde_json::json!({"car":car,"case":id,"family":case.family,"repeats":2,"repeat_hashes":[h1,h2],"metrics":first,"second_metrics":if h1!=h2 {Some(second)}else{None},"failures":[failures,second_failures],"traces":traces});
        writeln!(output, "{row}").unwrap();
        output.flush().unwrap();
    }
}
