#!/usr/bin/env bash
set -euo pipefail

# Live-platform verification helper.
# Runs the offscreen/fleet suite (proven, windowless) and attempts a bounded
# native window launch. It never claims hardware (display/gamepad) results it
# cannot observe.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Verification must not play engine/tire audio over the host's speakers.
export CAR_GAME_MUTE=1

EVIDENCE_DIR="$ROOT/artifacts/live-platform"
mkdir -p "$EVIDENCE_DIR"

echo "== cargo fmt --check =="
cargo fmt --check

echo "== cargo test --all-targets =="
cargo test --all-targets

echo "== cargo clippy --all-targets -- -D warnings =="
cargo clippy --all-targets -- -D warnings

echo "== offscreen fleet check =="
cargo run --offline -- --offscreen --fleet-check

echo "== run evidence =="
python3 tools/verify_fleet.py
python3 tools/verify_run.py

echo "== native window attempt (bounded) =="
# Native app has no auto-exit; bound it so it cannot hang. Success means the
# process started and created its window (see log). We do not assert hardware.
if command -v timeout >/dev/null 2>&1; then
  run_bounded() { timeout 30 "$@"; }
elif command -v gtimeout >/dev/null 2>&1; then
  run_bounded() { gtimeout 30 "$@"; }
else
  # Stock macOS has neither; bound with a background killer instead.
  run_bounded() {
    "$@" &
    local pid=$!
    ( sleep 30; pkill -TERM -P "$pid" 2>/dev/null; kill -TERM "$pid" 2>/dev/null ) &
    local killer=$!
    local status=0
    wait "$pid" || status=$?
    kill "$killer" 2>/dev/null || true
    wait "$killer" 2>/dev/null || true
    case "$status" in
      143|137|101) return 124 ;;
    esac
    return "$status"
  }
fi

if run_bounded cargo run --offline -- >"$EVIDENCE_DIR/native-window.log" 2>&1; then
  echo "native window exited cleanly within 30s"
else
  status=$?
  # 124 = timeout (expected: app stays open); treat as started-ok if window created
  if [ "$status" -eq 124 ] && grep -q "Creating new window" "$EVIDENCE_DIR/native-window.log"; then
    echo "native window started (created window); timed out as expected (no auto-exit). Evidence: $EVIDENCE_DIR/native-window.log"
  else
    echo "native window failed to start (exit=$status). Evidence: $EVIDENCE_DIR/native-window.log" >&2
    cat "$EVIDENCE_DIR/native-window.log" >&2
    exit "$status"
  fi
fi

echo "== verification evidence =="
echo "Fleet report and captures: see artifacts/latest (or artifacts/runs/<run-id>/)"
echo "Native window log: $EVIDENCE_DIR/native-window.log"
echo "Structural/image verification was run above by tools/verify_fleet.py."
