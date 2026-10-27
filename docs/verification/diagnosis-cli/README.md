# Searchable recorded-run diagnosis

Recovered the standalone JSON interface from checkpoint `d75f950`. It searches
explicit run roots, filters actor/tick windows, identifies shift hunting and
contact/clock anomalies, and compares recorded samples. It reports existing
recorded facts and inferences separately. It does not replay a simulation or
replace `tools/verify_run.py` as the integrity acceptance entry point.

This recovery slice adds the analysis interface only. New automatic incident
recording and richer force/shift decision production remain a separate runtime
recovery slice; old traces cannot gain missing decisions by analysis.

Run `python3 tools/diagnose.py --help` for the available subcommands. Focused
controls are `python3 tools/test_diagnosis.py`. Historical failures remain in
their original bundles; diagnosis reads them without mutation.

## Incident timeline

`python3 tools/diagnose.py search --root artifacts/runs` reads the recorder's
`index.jsonl` and recovers directories missing from it without writing either.
Commands accepting a run accept a directory or a run ID under `--root`.
`python3 tools/diagnose.py incident RUN_ID --root artifacts/runs` emits bounded
JSON with recorder health, failure, correlated lifecycle boundaries, supporting
records, reproduction when available, and explicit unknowns. Add `--other
RUN_ID` to include the first physics divergence and its sample references.
`timeline` supports tick, car, actor, vehicle, entity and correlation filters.
Exit 0 indicates a complete healthy incident query; exit 1 indicates a failed,
degraded, incomplete or ongoing/stale run or an evidence-read error; exit 2
indicates invalid input. An unfinished run is not asserted dead: it may be
ongoing or stale. GPU internal submission, execution and readback remain
unobserved; an absent completion does not identify their cause.

The production scenario and smoke paths expose opt-in fault injection for
verification: `--diagnostic-fault capture-missing` suppresses the capture
completion observer for a scenario, `readiness-stall` holds scenario readiness,
and `asset-readiness-stall` holds smoke/fleet asset readiness. These are recorded
in the run. Scenario runs save their definition for reproduction. Faults are
rejected outside their intended modes.

The retained `incident-evidence.json` is an excerpt of three production runs
on macOS Metal: capture completion suppressed (failure), the same capture
scenario without injection (control), and smoke asset readiness held (failure).
It retains the actual correlated records, recorder health, phase samples and
unknowns. Reproduce from the repository root with:

```sh
CAR_GAME_MUTE=1 cargo run -- --offscreen --scenario docs/verification/diagnosis-cli/capture-scenario.json --diagnostic-fault capture-missing
CAR_GAME_MUTE=1 cargo run -- --offscreen --scenario docs/verification/diagnosis-cli/capture-scenario.json
CAR_GAME_MUTE=1 cargo run -- --offscreen --smoke --diagnostic-fault asset-readiness-stall
python3 tools/diagnose.py incident RUN_ID
```

The injection holds the application readiness or completion observation; it
does not claim a GPU-internal fault. The capture fault ended with `tick-limit`
after all assertions executed, and its final observed capture boundary was
`scenario.capture_queued`. The control saved and validated its capture. The
asset fault observed scene instantiation, then withheld the fleet wheel-readiness
boundary and failed with `model_load_timeout`.
