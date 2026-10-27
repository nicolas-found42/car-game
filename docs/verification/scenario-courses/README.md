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

Endpoint admission also has short-route and return-leg controls. The new
0.5m two-point test reproduced false success before observed entry was required;
a point-sequence test reproduced a later short return leg being skipped before
entry state reset at every segment. Both now reject the shortcut and retain a
valid entry/traversal continuation. The fresh headless ladder passes 113 Rust
tests and all Python/asset gates.

Final recorded build
`b0f5ff012124523f937212edbb0447d7ccfb6166171388f15a0c4205a8fd23c4`
reverifies both actual fast route directions and rejects 0.5m/100m two-point
endpoint starts. `route-results.json` holds these final controls; the initial
full default-fleet gate and unchanged geometry preview above remain separate
acceptance evidence. Original failing and intermediate runs stay in primary
artifacts, without duplicating their reports in this directory.

Sol-loop: three rounds with gpt-6.1-sol on medium, approved after endpoint and
per-segment admission fixes. The behavior fix adds one entry flag and resets it
at segment boundaries; the additional tests cover first and later segments.
Usage was not exposed.
