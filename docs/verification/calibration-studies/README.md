# Production calibration studies

This selectively recovers the study tooling from preservation checkpoint
`d75f9504144bbbc0f4b25d210fb0bf87bbffd3cf`, after the opt-in course recovery
in [#115](https://github.com/nicolas-found42/car-game/pull/115). Default driving,
arcade physics and the 13-car active roster are unchanged. Studies explicitly
select reference physics and existing active cars; staged cars are rejected.

## Run a study

Build a recorder-bound executable using `python3 tools/record_build.py --profile
release` (serialize Cargo with other work). Then, from the repo root:

```sh
python3 tools/run_calibration_study.py artifacts/my-study \
  --binary target/release/car-game.recorded --cars 350z mcla-evo \
  --cases launch-partial brake curve-forward-slow --repeats 2
python3 tools/analyze_calibration_study.py artifacts/my-study
python3 tools/compare_calibration_studies.py artifacts/my-study artifacts/control-study \
  --output artifacts/comparison.json
```

The destination must be new. Absolute destinations outside the checkout work.
The runner is sequential, bounded to 180 seconds per child, and retains logs,
exit codes, partial runs and every failed row. A failed child makes the command
fail; analysis requires the entire declared matrix. It never retries or selects
only successful repeats. Recorder queue limits are explicit for this fast batch;
global recorder defaults remain unchanged. Large cohorts copy immutable run
materials per run, so budget disk space before increasing the cohort.

Choose any of the 24 cases listed by `--help`: the original five braking/turning
cases, eleven launch/coast/brake/handbrake/slalom cases, and eight curve/urban/lane
routes. Only curve cases select `northeast_curve`. All other cases retain `city`.
Route following uses the same player-scoped production driver as normal scenarios.

`study.json` declares car/case/repeat identities, producer hash, selected asset
hashes, policy and frozen study-input paths/hashes. Each raw run must pass both
`verify_run.py` and immutable bundle verification. Analysis binds scenario,
roster, profile, compiled driver/course sources, selected GLB and effective
reference/fast/fixed-clock policy to the plan. Applied profiles and mass must
match the frozen corpus, and undriven axles must have zero engine demand.
Measurements, applied-profile events and readiness come from verified frozen objects,
never mutable top-level traces or current workspace profiles. Raw production
verification remains a separate acceptance gate; exports legitimately canonicalize
JSON and redact host paths, so raw/object byte equality is not assumed. Player physics
must contain each completed tick exactly once, including requested-reset ticks.
Open-loop held inputs are independently checked against the scenario timeline.

`analysis.json` and `metrics.csv` contain per-run measurements. Generic metrics
cover relative tick 240 through the final assertion, including stopped tails;
the five original handling cases add their explicit action windows and first
near-stop metrics. Contact counts and load-bearing wheel counts are distinct.
Repeated simulations show repeatability, not real-world variance. These reports
neither certify original-game/manufacturer fidelity nor establish exact replay
(`runtime_replay_complete` is currently false).

Comparisons re-audit both cohorts, require the same roster, visual assets,
protocol, route/driver, policy, matrix and scenario stimuli, and require identical
effective open-loop inputs. Closed-loop cases match the route/controller rather
than identical feedback-driven inputs. Binary/profile hashes may differ as
reported study factors; differences alone do not establish causation. Same-binary
matched controls are valid. Missing metrics stay null rather than becoming zero.

## Optional plot

In a virtual environment, install `matplotlib>=3.8,<4`, then run:

```sh
python tools/plot_calibration_study.py artifacts/my-study --output artifacts/study.png
```

The plot re-audits raw evidence. Bars show repeat means and whiskers observed
min/max, with explicit simulation qualifications. Core tools and CI use the
Python standard library and do not require Matplotlib.

## Recovery map

| Original checkpoint tooling | Current recovery |
| --- | --- |
| `handling_protocol.py`, `fleet40_protocol.py` | `handling_protocol.py`, `calibration_protocol.py`: 24 cases with current opt-in course geometry. |
| `handling_metrics.py` | Explicit tick-window, first-stop, support and unwrapped-yaw math; legacy clearance helper is an estimated two-cuboid envelope. |
| `run_handling_study.py`, `run_fleet40_study.py`, read-only portions of `run_large_fleet.py` | `calibration_plan.py`, `run_calibration_study.py`: explicit active selection and production CLI. |
| `analyze_*study.py`, `analyze_large_fleet.py`, `audit_fleet40_evidence.py` | `calibration_evidence.py`, `analyze_calibration_study.py`: current strict verifiers and actor-scoped frozen evidence. |
| `compare_handling_studies.py` | `compare_calibration_studies.py`: dynamic matrices and strict stimulus matching. |
| `plot_fleet40_results.py`, `plot_large_fleet.py` | `plot_calibration_study.py`: dynamic observed cohorts. |
| `prepare_large_fleet.py`, `publish_large_fleet.py`, profile builders | Remain archived: restoring their roster/profile publication behavior would violate the unchanged-roster requirement. |
| Old minimal-app Rust probes, stress/batch/skidpad harnesses | Remain archived: their old control/simulation owners bypass the current production recorder. Current studies use the accepted production pipeline. |
| 314 historical result paths and old fixed-size reference comparisons | Byte-pinned at the preservation checkpoint; historical observations are not current-main acceptance. |

No bulk historical matrices or staged-only experiments are relabeled as current
results. See `proof.json` and `control.png` for the observed recovery checks.
