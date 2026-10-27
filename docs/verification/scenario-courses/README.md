# Opt-in scenario courses

Recovery of checkpoint `d75f950` adds `course: northeast_curve` and an optional
closed-loop route to the existing production scenario runner. Omitted `course`
means the original city; all original city collider identities/transforms and
all 13 active editions remain unchanged. The connector adds 96 identified curbs
only when explicitly selected. No fleet-only scene omissions are restored.

The route driver observes only the player's completed-step state and submits
targeted production commands. Sequential segment tracking prevents a car placed
at the endpoint from claiming it traversed the route. A parked secondary actor
remains independently controlled. This is an authored scenario controller, not
recovered original-game driving AI. Route support counts cover the declared
active observation interval, including stopping after completion.

`--fast-scenario` advances disabled-render, tick-only scenarios at manual 120Hz
without sleeping. Visual modes and elapsed-time triggers are rejected. Normal
execution and default driving clocks are unchanged. Fast-mode host frame timing
is not real-time performance evidence; real wall-clock deadlines remain bounded.
The flag and selected course are retained in provenance. The existing recorder,
current assertions, capture linkage and per-actor pipeline stay authoritative.

```sh
CAR_GAME_MUTE=1 cargo run --release -- --render-disabled --fast-scenario --scenario examples/scenarios/curve-forward.json
CAR_GAME_MUTE=1 cargo run --release -- --offscreen --scenario examples/scenarios/curve-preview.json
cargo run --quiet --example fleet_courses
```

Source-pinned recorded binary
`f573fb892c68d8e412554551026a80cdb7f3cd2955381f1e786cb18cee490f30`
passes the full ladder (111 Rust tests and all Python/asset controls), 13 default
fleet drive scripts, wheel animations, 52 captured views and the strict verifier.
Both approximately 200m route directions pass through the actual fast CLI with
maximum cross-track distance below 2m; both traces strictly verify. An endpoint
teleport control fails completion and strict verification, while the recorder
remains healthy. The baseline binary rejects the new course field. The offscreen
preview passes capture/hash/frame linkage and was visually inspected. Original
runs and logs remain under primary `artifacts/recovery-courses/` and
`artifacts/recovery-courses-recorded-verification.log`.

`courses.json` exports the exact Rust geometry and its source SHA; route results,
preview validation and the unmodified screenshot are retained beside this file.
The two original diagnostic inputs request staged-only NSX/Mini editions. Their
exact bytes are preserved as historical inputs with hashes; they are deliberately
not runnable examples that silently enable inactive cars.

The initial endpoint control had many segments; bot review exposed a two-point
shortcut. A new short-route control reproduced false success before the admission
fix. Observed first-segment entry is now required before accepting progress and
an endpoint; each shared endpoint carries that entry into the following segment.
The fresh 112-test headless ladder passes. Recorded build
`3e68fe08ce688e0cc138316805a9c172e07dbc6995150fe7180cad4140d7cc46`
passes both real fast route directions again; 0.5m and 100m two-point runs starting
at their endpoint fail completion and strict validation. `fixed-route-results.json`
retains those current results, separately from initial acceptance.

Sol-loop: two rounds with gpt-6.1-sol on medium, approved after the endpoint
admission fix. The behavior diff grew by 13 lines for that fix and the additional
regression adds 20 lines. Usage was not exposed.
