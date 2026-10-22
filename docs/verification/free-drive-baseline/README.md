# Free-drive baseline before mechanical migration (#10)

This captures the executable baseline at `b5b59a8105d729295734e4efc3ee86b6e0dec177` for parent #9. No Rust, assets, roster, physics tuning or gameplay changed. The added inventory command reads the exact receipt-linked render assets; its test exercises that CLI against the unchanged 350Z edition. Recording limitations below are baseline findings, not repaired telemetry.

Raw evidence is retained outside the disposable implementation worktree at `/Users/nicolas/Documents/github/car-game/artifacts/issue-9-baseline/`. `evidence-index.json` records every retained file's SHA-256. `measurements.json` contains the machine-readable run identities and per-car measurements; `mechanical-inventory.json` is the checked-in asset inventory. These artifacts are local, ignored evidence, not repository blobs or CI-hosted downloads. A missing local artifact is unavailable evidence and must not be counted as passing.

## Identity and reproduction

The target is an Apple M5 Pro MacBook Pro (Mac17,9), 18 CPU cores, 20 GPU cores, 48 GB RAM, native `aarch64-apple-darwin`, Metal 4, with a connected 3024×1964 built-in display. `build-identity.json` retains the OS, exact rustc/Cargo versions, lockfile hash and direct dependency tree; each run's manifest retains its own executable SHA-256, source revision, arguments, adapter, target and window state. The original executable and prepared assets are retained under `baseline-application/`, alongside production source and lockfile snapshots. The roster SHA-256 is `d13d57b3f666860932e80db383d938be072f70058d3acf2326b5df879623e64a`. `production-diff.patch` is empty; dirty run manifests reflect only this evidence documentation/tooling. Hardware serial numbers are intentionally excluded. Rust edition 2024, Bevy 0.20.0, Rapier integration 0.37.0 and the 120-Hz simulation remain unchanged.

Run from the repository root. Source integrity and the inventory require the exact sibling `../carmodels` collection; an isolated worktree can link that sibling to `/Users/nicolas/Documents/github/carmodels`. `CARGO_TARGET_DIR` only shares compiled objects. Each run still creates its own new directory, with the worktree's assets and manifest context.

```sh
export CAR_GAME_MUTE=1
export RUST_LOG=warn
export CARGO_TARGET_DIR=/Users/nicolas/Documents/github/car-game/target
cargo fmt --check
cargo clippy --all-targets -- -D warnings
scripts/check_codelimit.sh HEAD
cargo test --all-targets
python3 -m compileall -q tools
python3 tools/check_assets.py
python3 tools/test_fleet_assets.py
python3 tools/test_mechanical_inventory.py
python3 tools/inventory_mechanical_assets.py --source-root ../carmodels > mechanical-inventory.json
bash scripts/verify.sh
# After preserving the fleet run path, exercise the actual native window:
cargo run --release -- --smoke
# Diagnose a chosen preserved run instead of whichever run is latest:
cargo run --release -- --diagnose /absolute/path/to/preserved/run
```

`tools/verify_fleet.py --run <fleet-dir>` and `tools/verify_run.py <fleet-dir>` recheck a selected run. The latter takes a positional path; the README's earlier suggestion that both tools use `--run` is inaccurate. Keep `artifacts/latest` only as a convenience pointer, not as a baseline identity. Baseline scripts use explicit preserved paths after run selection.

## Preserved fleet and commands

Stable IDs in roster order: `350z`, `fr2-cobra`, `fr2-fr500`, `gt-rx7`, `gt-viper`, `gt4-m3`, `gt4-nsx`, `mc3-skyline`, `mc3-supra`, `mcla-evo`, `mcla-skyline`, `redline-911`, `redline-gt500`. The unchanged 350Z GLB SHA-256 is `e79630558ed16c54942afc552838fd465a73bf507754947342bd65a805d37874`.

| Action | Keyboard | Gamepad mapping |
| --- | --- | --- |
| Throttle | W / Up | Right trigger |
| Brake / reverse near rest | S / Down | Left trigger |
| Steering | A/D / Left/Right | Left stick |
| Handbrake | Space | South |
| Reset upright on nearby road | R | West |
| Pause/resume | Escape | Start |
| Quit while paused | Q | Select/Back |
| Previous/next fleet car | [ / ] | Left/right bumper |
| Mute | M | None |

Existing arguments are `--car <id>`, `--smoke`, `--fleet-check`, `--visual-check`, `--offscreen`, `--replay-input <path>` and `--diagnose [path]`. Bare invocation is sandbox free drive; visual mode does not certify driving. Offscreen is the same native GPU scene rendered at 1920×1080, not a display-refresh benchmark. No scenario/seed/artifact-dir contract exists yet. Controls and cadence have fresh production-physics tests; physical gamepad compatibility and audible quality remain unverified. Every GPU run here is muted.

## Workload and comparison policy

The existing fleet script runs each car with the same city, one active car, the prepared edition, default arcade tuning and 1920×1080 target. Relative to model readiness it captures front/side/rear at approximately 2/3/4 seconds, drives full throttle during 5–13 s, adds steer 0.45 during 11–13 s, handbrake during 12–12.5 s, brake during 13–17 s (released below 1 m/s), captures driving after 12 s, resets after 18 s and evaluates after 22 s. These controls follow wall time; they are reproducible workload instructions, not a claim of bit-identical tick inputs. Retain `input.jsonl` and timestamps to distinguish scheduling differences.

The headless fleet test exercises four contacts and parked speed <0.1 m/s; 600 parked ticks with displacement <0.15 m; 720 throttle ticks reaching >15 m/s; 240 brake ticks ending below 60% of pre-brake speed; steering/handbrake; upright stopped reset; 1200 reverse ticks between 5 and 8.1 m/s; and physical wall blocking. Other existing tests cover render cadence, pause, keyboard and synthetic gamepad controls. They use the same production raycast dynamics. The script's actual control records and its reports are stronger numerical evidence than reconstructing a result from a prose description.

Proposed migration budgets remain **unratified**: 1% longitudinal response, 0.10 m straight endpoint error, fixed-update CPU p99 <8.333 ms on this declared Mac/workload. Do not silently use offscreen frame p99 as fixed-update CPU p99: the current recorder exposes frame durations, not a scoped CPU timer for the fixed schedule. This capture establishes measured fleet speed/braking/frame distributions and reset-trace gaps; it does not certify those three future budgets or substitute wall-timed fleet paths for a fixed-tick straight-line reference. For handling comparisons, use the existing headless fixture and the future shared scenario seam to bind identical completed ticks, controls, initial state and toleranced endpoints before ratifying the budgets. Changed mechanical behavior requires explicitly versioned expectations.

## Mechanical content boundary

`mechanical-inventory.json` identifies every prepared/source GLB and receipt by hash and includes raw source/prepared node names, mesh/primitive counts, UV0 primitive coverage, wheel mount/radius values and current collider dimensions. The inventory deliberately does not infer piston/rotary/V-layout topology or operating curves from the car's marketed identity or a mesh name.

| Car ID | Meshes | Primitives with UV0 / all | Scene triangles |
| --- | ---: | ---: | ---: |
| 350z | 5 | 52 / 53 | 9,310 |
| fr2-cobra | 11 | 103 / 105 | 4,040 |
| fr2-fr500 | 11 | 111 / 113 | 4,583 |
| gt-rx7 | 5 | 13 / 14 | 528 |
| gt-viper | 5 | 15 / 16 | 622 |
| gt4-m3 | 5 | 54 / 54 | 7,772 |
| gt4-nsx | 5 | 48 / 48 | 7,652 |
| mc3-skyline | 25 | 87 / 87 | 8,058 |
| mc3-supra | 26 | 90 / 90 | 8,100 |
| mcla-evo | 58 | 168 / 188 | 106,110 |
| mcla-skyline | 58 | 173 / 189 | 115,637 |
| redline-911 | 6 | 18 / 18 | 8,748 |
| redline-gt500 | 10 | 18 / 18 | 10,122 |

For **each** of these 13 cars: selected source geometry/image preservation and prepared hashes are verified by the existing integrity suite; complete animated wheel assemblies/pivots are visual integration assets. The 350Z has eight wheel/tire nodes; the additions have four grouped wheel pivots. Source render hierarchy is not an owning attachment tree or assembly-prerequisite graph. Mesh-level UV presence does not establish individually paintable panel/decal charts or deformation quality. The chassis uses two code-generated cuboids and four suspension raycasts; those are authored arcade fixtures, not source per-part collision assets. Mass is shared at 1320 kg and engine RPM/automatic gear are arcade estimates, not recovered engine/drivetrain topology.

No complete operating/mechanical manifest, instance part identity, internal fastener/access model, verified component colliders, per-part calibrated mass/inertia or complete UV/deformation coverage is currently supplied for any car. Raw mesh names can point authors toward visual candidates, but cannot certify that content. The 35-GB sibling collection contains additional unselected source material; this inventory covers only the exact selected GLBs, and makes no claim that a broader content-authoring audit would find nothing useful there. Provenance verifies local derivation and explicit preview repairs, not licensing or real stock performance. Missing mechanical content blocks the corresponding P2/P3/P4 gates.

## Observability controls

The isolated empty-trace probe starts from a **completed** real fleet run, copies its manifest/events/reports/log and references the unchanged capture files, then creates empty `physics.jsonl` and `input.jsonl`. The real run remains untouched. `probe.json` retains the copy procedure, original run and output; the unmodified existing `tools/verify_run.py` exit/output is retained. A verifier success on this deliberately invalid fixture documents a false pass; it is not a valid gameplay run or a telemetry-quality pass.

The reset mismatch probe reads the real traces without modifying them. It counts recorded `input.reset=true` and reports reset-count transitions per car, including adjacent sample times/steps. The source consumes `input.reset` before recording and skips the reset tick, so trace step numbers alone cannot prove that every completed fixed tick was recorded. Measurements below state what actually appeared; the source ordering explains it without upgrading a hypothesis into evidence. Immutable diagnostic provenance/export belongs to #88; validation, recorder health and replay changes belong to the supplemental observability tickets.

## Fresh results and retained controls

The first fleet run is `20261022T20274547116Z-b5b59a8-fleet-342-1792700865489841000`, binary SHA-256 `efe4d9aa55af49ffcb3efbc7a5626614ebca519285d5d3c8e99bbb558eb8fe6c`. It completed all 13 drive scripts and 52 nonblank 1920×1080 captures on the Apple M5 Pro Metal adapter. Four contact sheets (`visual-review-1.jpg` through `visual-review-4.jpg`) were inspected for all front, side, rear and driving views: each contains the expected assembled car and wheels, road and city context; the GT1 editions retain their visibly simpler source-era geometry. The native driving capture additionally shows the readable speed/fleet/control HUD. These are actual inspected pixels, not an inference from nonblank sky or hash checks.

The separate native 350Z run is `20261022T203232998023Z-b5b59a8-smoke-3486-1792701153014196000`: process exit 0, a focused 1920×1080 window, AutoVsync, Metal M5 Pro, four captures and a passing drive report. `native-invocation.json`, `native-smoke.log` and the preserved run retain the actual window result. It was allowed to finish its own script, rather than treating a process timeout as gameplay acceptance. Physical keyboard/gamepad interaction was not manually exercised in this capture.

Both explicit run-evidence checks passed. The initial `verify.sh` invocation itself exited 1: the native run was started after the fleet executable exited but before the remaining default-path validator ran; it changed `artifacts/latest` to the still-running native run, so that validator reported a missing `run.exit`. The already-completed fleet has its terminal event. This was a measurement-orchestration race introduced by this capture, not evidence of a fleet-recording failure. The first run also overlapped some warm headless checks, so its renderer timing distribution includes that host workload. The original failure is retained in `verify.log`; `fleet-explicit-verify.log` and `native-explicit-verify.log` select their completed run paths. An additional isolated full-chain run avoids both overlaps; its final result and identity are recorded below. Never "repair" a real run by inserting a terminal event.

The real first fleet trace has **34,319** recorded pre-integration physics samples and the same number of input records, with **zero** recorded `reset=true` inputs despite **one reset per car** in the reports and actual reset-count increases. For the 350Z, adjacent trace steps 2099→2100 span time 17.500002→17.516668 s while reset_count changes 0→1; both input records say reset=false. `measurements.json` records the corresponding before/after samples for all 13 cars. This is measured lost reset provenance, not merely a suspected code path.

The copied negative control has **zero-byte** physics/input files. The unchanged verifier still returned **exit 0** and `RUN EVIDENCE PASS` for that deliberately invalid copy. Its retained `probe.json` records the source, operation, exact verifier hash, invocation and output. The original run's 34,319 records were not altered. No empty-trace fixture is included in the list of valid runs.

A matched 350Z straight segment was also read from the actual offscreen and native traces: 600 uninterrupted recorded full-throttle intervals at 120 Hz, no steering/braking/handbrake, five simulated seconds from the first full-throttle sample. These legacy physics samples are taken before Rapier integration; they are not coherent completed-step records. Both end at **86.705568 km/h** with planar displacement **66.47365 m**. Observed speed and planar-displacement differences are zero at the JSON's recorded precision. `native-handling-comparison.json` records both start/end steps, vectors and identities. This supports reproducibility for that exact one-host segment; it does not ratify universal stock handling or the proposed migration budgets.

The baseline inspection/probe can be repeated with retained `analyze.py`:

```sh
python3 /absolute/evidence-root/analyze.py /absolute/repository /absolute/completed-fleet-run /absolute/new-output-directory
```

Use the baseline revision and a new output directory; the script refuses to overwrite an existing probe. The preserved `baseline-tools/verify_run.py` can also recheck the retained negative control directly after the main repository evolves; later validators should reject it. It reads the selected real trace, writes its measured summary, copies that completed run into `empty-trace-probe/` without the traces, creates empty trace files there and invokes the repository's existing positional-path verifier. That intentional verifier false pass remains a recorded failure of a negative control even when the subprocess returns success.

## Isolated full-chain result

The sequential repeat **passed `bash scripts/verify.sh` with exit 0**. Run `20261022T203335624405Z-b5b59a8-fleet-3963-1792701215649553000` used the same preserved executable and Metal adapter, completed 13/13 drive reports, 52/52 captures, and both fleet/run validators. `verify-isolated.log`, `fleet-isolated/` and `isolated-results.json` retain the full output and identity. Its 185 events include the actual terminal `run.exit`. No native run changed `artifacts/latest` during that chain.

The documented ladder passed formatting, all-target strict Clippy, the changed-Python code-limit/complexity gates and all 18 Rust tests. Separate checks passed Python compilation, all seven original asset tests (including their deliberate corruption controls), the new inventory CLI test, and byte-for-byte reproduction of the checked-in inventory. The inventory test has retained red/green outputs. The full offline chain additionally reverified all source/edition assets. Metal/native evidence is described above; audio listening and physical controller coverage remain explicitly unavailable.

The isolated repeat's measured results are below. Frame p99 is explicitly a **frame** statistic; it cannot be compared to the proposed fixed-update CPU budget. Both runs contain the same stock-independent shared arcade tuning.

| ID | Peak km/h | Brake start → end km/h | Frame p99 ms |
| --- | ---: | ---: | ---: |
| 350z | 101.534 | 71.780 → 2.958 | 12.557 |
| fr2-cobra | 101.497 | 72.033 → 3.248 | 12.564 |
| fr2-fr500 | 101.516 | 70.836 → 3.181 | 12.248 |
| gt-rx7 | 101.672 | 72.804 → 2.865 | 10.715 |
| gt-viper | 101.504 | 72.549 → 0.997 | 11.036 |
| gt4-m3 | 101.523 | 71.523 → 3.203 | 10.819 |
| gt4-nsx | 101.621 | 73.217 → 2.679 | 10.684 |
| mc3-skyline | 101.525 | 70.509 → 3.241 | 10.738 |
| mc3-supra | 101.590 | 70.791 → 3.331 | 10.676 |
| mcla-evo | 101.552 | 71.438 → 3.303 | 11.128 |
| mcla-skyline | 101.514 | 72.030 → 3.182 | 10.727 |
| redline-911 | 101.773 | 73.227 → 0.643 | 10.868 |
| redline-gt500 | 101.452 | 70.028 → 3.348 | 10.807 |
