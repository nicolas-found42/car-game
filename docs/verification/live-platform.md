# Live-Platform Verification

## Proven (offscreen, this environment)

- `cargo fmt --check`: passed.
- `cargo test --all-targets`: 16 tests passed (including `every_car_meets_the_baseline_driving_contract`).
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo run --offline -- --offscreen --fleet-check`: completed; 13/13 cars produced reports with `passed: true`, 4 nonblank captures each, ~102 average FPS, p95 frame ~10.4-10.6 ms. Evidence in `artifacts/fleet/*-report.json` and `artifacts/fleet/*-*.png`.
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
cargo run --offline --   # native window; no auto-exit
python3 tools/verify_fleet.py
```

## Automation

Run `scripts/verify_live.sh`. It runs the proven checks, attempts a bounded native launch, records `artifacts/live-platform/native-window.log`, and fails clearly if the native window cannot start. It does not claim hardware results it cannot observe.
