# GRID — Free Drive

A native Rust driving prototype for macOS with **13 selectable cars**, an empty city grid roughly one kilometre across, grip-focused arcade handling and a chase camera. The fleet keeps the original GT4 Nissan 350Z and adds two cars from each of six recovered racing games.

## Play

Install Rust and the macOS command-line developer tools, then run from this checkout:

```sh
cargo run
```

The prepared cars, textures and placeholder audio are included in this repository. Running the game does not require the sibling source-model collection or Python. Start on another car with `cargo run -- --car redline-911`.

To build a standalone macOS app with its assets:

```sh
sh tools/bundle_macos.sh
```

Open the resulting `dist/GRID.app` to play. The first build compiles Bevy and Rapier; later builds reuse the local Cargo cache. `sh tools/bundle_macos.sh debug` builds a faster-to-compile development edition. Rust 1.97.1 or newer is required by Bevy 0.20.

## Controls

| Action | Keyboard | Gamepad |
| --- | --- | --- |
| Accelerate | W / Up | Right trigger |
| Brake, then reverse near a stop | S / Down | Left trigger |
| Steer | A/D / Left/Right | Left stick |
| Handbrake | Space | South button (A / Cross) |
| Reset upright on a nearby road | R | West button (X / Square) |
| Pause / resume | Escape | Start |
| Quit while paused | Q | Select / Back |
| Previous / next car | [ / ] | Left / right bumper |
| Mute / unmute | M | — |

Switching cars recovers the new car upright and stopped on a nearby road. The HUD names its source game and position in the fleet.

The city has solid buildings and sidewalks, a connected perimeter road, and barriers at unfinished exits. Impacts affect motion without a damage system. Audio is synthesized placeholder engine, tire and impact sound. The HUD shows speed in km/h and a simple automatic gear/RPM estimate.

## Check

```sh
python3 tools/check_assets.py
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- --smoke
```

The headless tests use the same Rapier simulation as the game. They cover suspension, parked stability, acceleration/braking, wheel rotation, turning, collision, reverse limiting, reset, pause/resume/quit, synthetic analog gamepad input and render-cadence independence.

`--smoke` checks one car; `--car <id>` chooses it at startup (for example `cargo run -- --car mcla-evo --smoke`). `--fleet-check` cycles every car through the native script. Each run captures matched front/side/rear images and a driving image, checks loaded wheel assemblies, acceleration, turning, braking and reset, and writes a self-contained evidence directory under `artifacts/runs/<run-id>/`. `--visual-check` captures the static comparison without claiming driving success. Blank captures fail. Add `--offscreen` to render the same native scene and HUD to a 1920×1080 GPU image when no usable window surface is available; reports distinguish that target from window performance.

Every run directory contains `manifest.json` (git state, binary hash, arguments, environment, GPU, window, exit), `run.log`, `events.jsonl`, per-car reports under `cars/`, capture files and metrics under `captures/`, and `physics.jsonl` plus `input.jsonl`. Failures additionally write `failure.json` with the last stage, recent events, log tail, physics tail, and any panic backtrace. A run never overwrites another run; `artifacts/latest` points at the newest one.

Offscreen checks use a windowless native runner capped at 120 updates/second, with actual wall-clock frame samples. After a full run, `python3 tools/verify_fleet.py` checks the latest run’s reports, hashes and image metrics, and builds six matched comparison sheets plus a fleet overview inside that run directory. This reporting tool requires Pillow; visual review is still required after the structural/capture checks.

```sh
python3 tools/test_fleet_assets.py
cargo run -- --offscreen --fleet-check
python3 tools/verify_fleet.py
python3 tools/verify_run.py
python3 tools/evidence_manifest.py
```

`tools/verify_fleet.py` and `tools/verify_run.py` default to `artifacts/latest`; pass `--run artifacts/runs/<id>` to inspect an older run. They write only inside the selected run directory, so an earlier run and its manual notes are never replaced. `tools/check_assets.py` writes timestamped integrity evidence and a failure artifact instead of overwriting a single file. `sh scripts/verify.sh` runs the whole local chain and tees every command’s output into `artifacts/logs/`.

Source-integrity checks and the Python asset tests require `../carmodels`; the comparison-sheet tool also requires Pillow. Native reports, screenshots and comparison sheets are generated locally under `artifacts/`. Build outputs and verification artifacts are ignored by Git.

The fleet physics test applies the same parked stability, four-wheel support, acceleration, braking, turning, handbrake, reset, reverse limiting and wall-collision contract to all 13 cars. Keyboard selection and synthetic gamepad bumper/analog mappings are tested. Physical gamepad compatibility still needs testing with the intended controller.

## Verified state

The current prototype has passed:

- **18 Rust tests**, formatting and strict Clippy checks.
- **Seven asset tests**, including corruption controls and a regression that keeps the GT500's complete rear window under one glass material.
- Source geometry, texture and wheel-assembly checks, plus glTF validation with zero errors or warnings across all 13 prepared cars.
- Native driving scripts for all 13 cars and **52 nonblank 1920×1080 captures**, with six matched front/side/rear comparison sheets against the original 350Z.
- Manual native-window checks for keyboard acceleration, reset, previous/next selection and wraparound in both directions.

The automated visual captures use the native Bevy renderer's offscreen target. Their frame timings are renderer measurements, not a display-refresh benchmark. Visual review corrected wheel backplates, lamp covers and the GT500 rear-window material boundary while preserving source geometry and image bytes. The original 350Z GLB and receipt remain unchanged. GT1 cars retain visibly simpler geometry and textures; this verification establishes functional parity and coherent source integration, not equal artistic detail across eras.

## Fleet

The original 350Z stays available, plus **two additions from each source game**:

| Source game | Additions | IDs |
| --- | --- | --- |
| Ford Racing 2 | SVT Cobra; Mustang FR500 | `fr2-cobra`, `fr2-fr500` |
| Gran Turismo | RX-7 FD A spec; Viper GTS | `gt-rx7`, `gt-viper` |
| Gran Turismo 4 | BMW M3 CSL; Acura NSX | `gt4-m3`, `gt4-nsx` |
| Midnight Club 3 Remix | Skyline; Supra | `mc3-skyline`, `mc3-supra` |
| Midnight Club: Los Angeles | Lancer Evolution IX; Skyline GT-R V-Spec | `mcla-evo`, `mcla-skyline` |
| Redline add-ons | Porsche 911 SC 3.0; Mustang GT500 (2008) | `redline-911`, `redline-gt500` |

The roster uses recovered source-era art. GT1 cars retain their simpler PS1 geometry; fidelity to those assets does not imply GT4-level geometric detail. All editions use GRID's lighting and complete animated wheel assemblies. The shared arcade tuning is not a claim about each real car's performance.

## Car asset

`assets/cars/350z.glb` is a self-contained game edition prepared from the user-supplied collection at `../carmodels`. Its receipt records the source and prepared SHA-256 hashes. Wheel transforms, geometry and embedded PNG bytes are preserved. Default-paint materials are retained, source paint-alpha preview repairs are applied, and the opaque cabin backing uses an explicitly artistic dark-glass material. These are display choices, not a reconstruction of native GT4 shading.

To regenerate assets with Python's standard library:

```sh
python3 tools/prepare_assets.py --source-root ../carmodels
python3 tools/prepare_fleet.py --source-root ../carmodels
python3 tools/check_assets.py
```

This also regenerates deterministic facade textures and placeholder WAV sounds. The sibling collection is needed for regeneration and full source-integrity checks; playing the prepared game or bundled app does not access it.

The initial handling is a tunable interpretation of the approved Forza Horizon-inspired feel, not a reproduction of that game's physics. The first build focuses on free drive; racing events, traffic, pedestrians and progression are future scope.

## Diagnostics

Every run writes an append-only evidence directory. Start with the newest run:

```sh
cargo run -- --diagnose artifacts/latest
python3 tools/verify_run.py
```

A failure bundle includes `failure.json` (reason, stage, details, recent events, log tail, physics tail), `panic.json` for panics, and partial `report.json` where a car had already completed. Physics telemetry is recorded at the fixed 120 Hz step with inputs, pose, velocity, wheel lengths/loads and contact misses; `input.jsonl` can be replayed exactly:

```sh
cargo run -- --replay-input artifacts/runs/<run-id>/input.jsonl
```

The app records its git commit, dirty state, binary SHA-256, arguments, OS, Rust/Bevy/Rapier versions, asset/source roots, GPU adapter and backend, window resolution/focus/present mode, gamepad identity, and every car start/switch, screenshot request/validation and exit. Use `RUST_LOG` to increase module logging; the same lines are written to `run.log`.

## Project notes

- [First-build specification](https://github.com/nicolas-found42/car-game/issues/2)
- [Fleet expansion and verification](https://github.com/nicolas-found42/car-game/issues/3)
- [Domain glossary](GLOSSARY.md)
- [Engine and vehicle dynamics decision](docs/adr/0001-bevy-rapier-vehicle-dynamics.md)
- [Prepared car edition decision](docs/adr/0002-prepare-a-local-car-edition.md)

The fleet preparation records exact source geometry/image preservation and intentional material/assembly changes in per-car JSON receipts. See [fleet assembly decision](docs/adr/0003-prepare-a-selectable-fleet.md).
