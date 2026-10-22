#!/usr/bin/env bash
# Run the full local verification chain and retain every command's output.
set -euo pipefail
cd "$(dirname "$0")/.."

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
log="artifacts/logs/verify-${stamp}.log"
mkdir -p "$(dirname "$log")"

{
  echo "== verify $(date -u +%Y-%m-%dT%H:%M:%SZ) =="
  echo "== git $(git rev-parse HEAD 2>/dev/null || echo unknown) =="
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo test
  python3 tools/check_assets.py
  cargo run --release -- --offscreen --fleet-check
  python3 tools/verify_fleet.py
  python3 tools/verify_run.py
} 2>&1 | tee "$log"

echo "verification log: $log"
