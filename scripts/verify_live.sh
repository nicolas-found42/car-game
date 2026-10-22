#!/usr/bin/env bash
set -euo pipefail

# Live-platform verification helper.
# Runs the offscreen/fleet suite (proven, windowless) and attempts a bounded
# native window launch. It never claims hardware (display/gamepad) results it
# cannot observe.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

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

echo "== native window attempt (bounded) =="
# Native app has no auto-exit; bound it so it cannot hang. Success means the
# process started and created its window (see log). We do not assert hardware.
if timeout 30 cargo run --offline -- >"$EVIDENCE_DIR/native-window.log" 2>&1; then
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
echo "Fleet reports/images: $ROOT/artifacts/fleet/"
echo "Native window log: $EVIDENCE_DIR/native-window.log"
echo "Run 'python3 tools/verify_fleet.py' for structural/image verification."
