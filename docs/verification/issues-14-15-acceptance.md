# Issues 14 and 15: recorded and native acceptance

Accepted on October 26, 2026 against source
`c66fa539fb2c396cfe81cd83860a19b95919060a`. This closes the outstanding
verification gaps after implementations merged in PRs #98 and #99, including
the road-entity lifecycle correction in #105. No production code changed.

## Repository and fleet proof

The whole-command Cargo lease ran:

```sh
CAR_GAME_MUTE=1 python3 artifacts/p1-coordination/cargo_serial.py bash scripts/verify.sh --recorded --base origin/main
```

Exit status was zero: format, warning-free all-target Clippy, complexity,
84 Rust tests, doc tests, Python compilation, asset integrity, provenance,
recorder-health, complexity controls and fleet negative controls passed.
`origin/main` equaled HEAD during this run. The single recorded fleet run
passed 13 driving scripts, wheel animation and 52 nonblank captured views;
`verify_fleet.py` and `verify_run.py` passed with 153405 events.

The exact recorded executable SHA-256 was
`b6b4922468b64089d4999e8579517e2bba6b6d23c6e3ecb874c433d5a485a8f3`.
Retained build and runtime material passed inventory integrity verification,
with `rebuild_complete=true` and `runtime_material_complete=true`.

## Production scenario proof

The same recorded executable ran the existing independent-vehicles and
synchronized-physics fixtures with rendering disabled, offscreen Metal and a
native Metal window. All six processes exited zero; every required assertion
and rendered capture passed. Both native captures were visually inspected.

| Fixture | Rendering disabled | Offscreen Metal | Native Metal |
| --- | --- | --- | --- |
| Independent vehicles (#14) | 5/5 assertions | 2/2 assertions | 2/2 assertions |
| Synchronized physics (#15) | 8/8 assertions | 8/8 assertions | 8/8 assertions |

#14 additionally retains production regressions for opposite simultaneous
inputs, independent geometry, controller rejection/transfer, held input versus
one-shot resets, and keyboard/gamepad/replay ingress. #15 retains first-tick
road insertion/removal, relocation, collider reparenting, composed forces,
reset, legacy-reference and single-step gravity regressions.

All six scenario streams passed recorder-health and portable material checks.
Their completed physics records have unique actor/tick keys, current query
ticks, exactly one integration step, and backend-applied forces/torques equal
to the composed totals. Effective-control actor/tick keys are unique.

A bounded interactive native-window run lasted 30.72 seconds, created its
window and terminated with exit zero. This directly executed the additional
native-startup stage of `verify_live.sh` using the recorded binary; the entire
script was not repeated because its ladder/fleet stages were already proved.

## Retained evidence and limits

Local evidence is retained under `artifacts/issue-14-15-current-main/`:
`AUDIT.md` contains the full acceptance mapping and exact run paths;
`verify-recorded.log`, `build.json`, `binary.sha256`, per-backend scenario
bundles, `native-window-result.json` and `evidence-validation.json` preserve
the observed results. Revalidate those existing bundles from the repo root:

```sh
python3 artifacts/issue-14-15-current-main/validate_evidence.py
```

The fleet run is
`artifacts/runs/20261026T205957197898Z-c66fa53-fleet-83410-1-1793048397218081000`.
These ignored artifacts are retained local evidence, not files distributed
with this documentation. The full run verifier is fleet-specific; scenarios
used its explicit `--health-only` mode plus assertion, exit, capture,
completed-step and inventory checks, not a fabricated fleet report.

No physical gamepad, live keyboard interaction or audible-audio certification
is claimed. Bundles explicitly retain `runtime_replay_complete=false`; complete
rebuild material is not a claim of portable exact runtime replay. Independent
assembly graphs for additional editions remain #16 scope. This acceptance does
not close the wider P1 phase gate or later observability issues.
