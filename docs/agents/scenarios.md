# Shared production runtime and bounded scenarios

`car_game::build_app(RuntimeConfig, Arc<RunRecorder>)` constructs the production
City, Vehicle, input/command and recorder plugins. `src/main.rs` only parses CLI,
handles diagnosis, launches that app and flushes/finalizes the run. Existing
`--offscreen`, `--smoke`, `--fleet-check`, `--visual-check`, `--car`,
`--replay-input` and `--diagnose` invocations retain their sandbox defaults.

Run a bounded native Metal scenario:

```sh
CAR_GAME_MUTE=1 cargo run -- --offscreen --scenario examples/scenarios/driving-reset.json --seed 17 --artifact-dir artifacts/scenarios
```

`--scenario-filter <substring>` must match the selected scenario. Zero selections
are an error. `--render-disabled` explicitly disables visual/audio presentation
while retaining the same authored City colliders, Vehicle, Rapier and production
command handlers. It validates prepared asset node metadata and actual physics
support. It never satisfies required image captures; those runs fail with a
recorded `required-capture-unavailable` summary.

A scenario declares version/name, `scene: driving`, an independent sandbox or
career context, seed, initial car/pose, readiness/tick/wall bounds, timed commands,
nonempty outcome assertions and optional required screenshot. Career fixtures
provide an owner and owned catalog IDs; offscreen rendering grants no ownership.
This fixture policy is deliberately temporary until durable vehicle/owner IDs
land. The stable run actor is `player-vehicle`; catalog ID is recorded separately.

Commands use exactly one `tick` or `elapsed_ms` trigger, sorted independently per
clock. Tick triggers are relative to readiness and dispatch in FixedFirst before
the existing pre-SyncBackend vehicle system. Millisecond triggers use real frame
time after readiness and remain active while virtual time is paused. Use them
for pause/resume. Actual wall deadlines use monotonic `Instant` outside fixed
schedules, so a paused or asset-stalled run cannot wait forever. Assertions run
against completed physics ticks; all declared assertions must execute and pass.

The implemented command API is `runtime::CommandBus::submit` (or
`submit_with_span`). Keyboard and the existing composite gamepad policy, replay,
smoke and scenarios use that same dispatch. Held controls and one-shot reset,
selection, pause/resume, assignment, mute and quit remain distinct commands.
Physical reset/selection effects happen at the next fixed boundary. Legacy live
input is collected in Update, preserving its effect on the next frame's fixed
work. The current custom force computation remains before Rapier SyncBackend.

Every command is appended once at ingress to `commands.jsonl`, before consumption.
Its versioned envelope retains last completed canonical tick, run/actor identity,
command/correlation IDs, frame, RFC3339 timestamp and monotonic elapsed time.
`command.outcome` records applied/rejected/skipped dispositions; `vehicle.reset`
records the actual once-only correlated before/after transition. `controls.effective`
records the controller and held input at fixed consumption, including reset before
it is taken. Part/transaction/result span fields are optional correlation hooks.
They do not create future mechanical or economy operations.

The canonical clock advances once per outer fixed tick, including reset and
trace-disabled ticks. `simulation.tick_completed` is the global post-writeback
boundary; `simulation.completed` contains actor state there. Existing physics
samples retain their pre-integration meaning and sample `step`; additive `tick`
provides canonical identity. Legacy flat input traces remain replayable. Window
focus/resize and gamepad connect/disconnect are captured at actual ingress without
waiting for GPU initialization.

Each run retains `scenario.json`, `scenario-summary.json`, command/input/physics/
event streams, normal manifest/log/failure evidence and any verified image.
Scenario summaries report seed, bounds/workload, per-assertion observed outcomes,
execution/pass counts, readiness boundary and capture metrics. Seed is preserved
with an explicit no-random-draws state for current deterministic authored fixtures;
no artificial random behavior is added to baseline dynamics. Build/dependency,
asset/catalog and artifact hashes are supplied by the common provenance manifest.

A scenario completes only after every timed command has reached production dispatch
and every assertion has executed. A final physical action cannot be silently skipped
because earlier assertions finished. The runner discards remaining fixed overstep at
its final boundary, so a slow render frame cannot advance beyond the declared tick
cap. The legacy smoke fixture requests `select_car` with `reset_to_spawn: true`;
that production handler owns the common initial pose for cross-car captures.
