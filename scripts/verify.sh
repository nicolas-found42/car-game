#!/usr/bin/env bash
# Run each repository gate once; optionally retain compiler-bound fleet evidence.
set -euo pipefail
cd "$(dirname "$0")/.."

headless=false
profile="${CODELIMIT_CARGO_PROFILE:-dev}"
recorded=false
base=HEAD
while [ "$#" -gt 0 ]; do
  case "$1" in
    --headless) headless=true ;;
    --recorded) recorded=true ;;
    --base)
      [ "$#" -ge 2 ] || { echo '--base needs a Git ref' >&2; exit 2; }
      base="$2"; shift ;;
    *) echo "usage: scripts/verify.sh [--headless] [--recorded] [--base REF]" >&2; exit 2 ;;
  esac
  shift
done
if "$headless" && "$recorded"; then
  echo '--recorded requires the full fleet stage' >&2
  exit 2
fi
# Reject an invalid base before starting expensive commands.
git rev-parse --verify "$base^{commit}" >/dev/null
# Keep long native capture runs awake without changing saved power preferences.
if command -v caffeinate >/dev/null 2>&1; then
  caffeinate -i -w "$$" >/dev/null 2>&1 &
fi
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
log="artifacts/logs/verify-${stamp}.log"
mkdir -p "$(dirname "$log")"
run() {
  local started=$SECONDS
  printf '\n== %s ==\n' "$*"
  "$@"
  printf '== PASS (%ss): %s ==\n' "$((SECONDS - started))" "$*"
}

{
  echo "== verify $(date -u +%Y-%m-%dT%H:%M:%SZ) =="
  echo "== git $(git rev-parse HEAD) =="
  run cargo fmt --check
  run cargo clippy --locked --all-targets --profile "$profile" -- -D warnings
  run scripts/check_codelimit.sh HEAD
  if [ "$(git rev-parse "$base")" != "$(git rev-parse HEAD)" ]; then
    run scripts/check_codelimit.sh "$base"
  fi
  run cargo test --locked --all-targets --profile "$profile"
  run cargo test --locked --doc --profile "$profile"
  run python3 -m compileall -q tools
  run python3 tools/check_assets.py
  run python3 -m unittest discover -s tools -p 'test_*provenance*.py'
  run python3 tools/test_recorder_health.py
  run python3 tools/test_codelimit.py
  run python3 tools/test_fleet_assets.py
  run python3 tools/test_physics_profiles.py
  run python3 tools/test_handling_metrics.py
  run python3 tools/test_fleet40_protocol.py
  run python3 tools/test_large_fleet.py
  run python3 tools/test_diagnosis.py
  if ! "$headless"; then
    # Scale the finalization allowance by roster size, bounded at 8GiB for the 300-car run.
    fleet_budget=$(python3 -c 'import json; print(min(8, max(1, (len(json.load(open("assets/cars/roster.json"))) + 39) // 40)) * 1073741824)')
    if "$recorded"; then
      run python3 tools/record_build.py --profile release
      run target/release/car-game.recorded --offscreen --fleet-check --recorder-finalize-max-bytes "$fleet_budget"
    else
      run cargo run --locked --release -- --offscreen --fleet-check --recorder-finalize-max-bytes "$fleet_budget"
    fi
    run python3 tools/verify_fleet.py
    run python3 tools/verify_run.py
  fi
} 2>&1 | tee "$log"

echo "verification log: $log"
