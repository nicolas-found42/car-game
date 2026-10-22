# Live-Platform Verification

## Proven (offscreen, this environment)

- `cargo fmt --check`: passed.
- `cargo test --all-targets`: 18 tests passed (including `every_car_meets_the_baseline_driving_contract`).
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo run --offline -- --offscreen --fleet-check`: completed; 13/13 cars produced reports with `passed: true`, 4 nonblank captures each, ~102 average FPS, p95 frame ~10.4-10.6 ms. Evidence is written per run under `artifacts/runs/<run-id>/` (`report.json`, `cars/<car-id>.json`, `captures/`), and `artifacts/latest` names the newest run.
- `python3 tools/verify_fleet.py` and `python3 tools/verify_run.py`: passed against that fleet run (`FLEET NATIVE EVIDENCE PASS`, `RUN EVIDENCE PASS`).
- Native window build: `cargo run --offline --` started and created window "GRID — Free Drive" (Metal/Apple M5 Pro). It has no auto-exit; bounded run timed out as expected. Evidence: `artifacts/live-platform/native-window.log`.

## Unverified (physical display / gamepad)

- Display-refresh timing under a real compositor/window server is not measured; offscreen timings are renderer measurements only.
- No physical gamepad was available in this environment; gamepad input is covered by unit tests (`analog_gamepad_input_drives_the_same_controller`, `gamepad_bumper_selects_the_next_car`) but not by hardware.
- Visual review of captures remains manual; `python3 tools/verify_fleet.py` provides structural/image checks only.

## Commands

```sh
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run --offline -- --offscreen --fleet-check
python3 tools/verify_fleet.py
python3 tools/verify_run.py
cargo run --offline --   # native window; no auto-exit
```

`scripts/verify.sh` runs the full local chain, including the Python asset tests. It requires the sibling `../carmodels` source collection.

## Automation

Run `scripts/verify_live.sh`. It sets `CAR_GAME_MUTE=1`, which starts the game in its existing muted state (the same state as pressing `M`): engine/tire volumes are zero and impact sounds are suppressed, so verification never plays audio over the host's speakers. Launching the binary without that variable keeps interactive sound. It runs the proven checks and both run verifiers, attempts a bounded native launch, records `artifacts/live-platform/native-window.log`, and fails clearly if the native window cannot start. The bound uses `timeout`, then `gtimeout`, and otherwise kills the process tree after 30 seconds, so it works on a stock macOS host without GNU coreutils. It does not claim hardware results it cannot observe.
