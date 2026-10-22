#!/usr/bin/env bash
# Multi-tiered code limit & complexity gates for car-game.
#
# Three complementary checks for maximum peace of mind:
#   1. codelimit (https://github.com/getcodelimit/codelimit):
#      Zero-config Python refactoring alarm for tools/*.py (> 60 LOC fails).
#   2. lizard-analyzer (https://github.com/terryyin/lizard):
#      Cross-language structural & complexity gate for both Rust (src/) and
#      Python (tools/). Checks NLOC, CCN (cyclomatic complexity), and parameter counts.
#      Grandfathers legacy debt via whitelizard.txt.
#   3. Clippy too_many_lines (rust-lang/rust-clippy):
#      First-party compiler-native AST gate for Rust (src/), configured via clippy.toml.
#
# All gates operate on changed files vs base-ref (ratchet style).
#
# Usage: scripts/check_codelimit.sh [base-ref]   (default base: origin/main, falls back to main)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

base="${1:-}"
if [ -z "$base" ]; then
  if git rev-parse --verify origin/main >/dev/null 2>&1; then
    base="origin/main"
  else
    base="main"
  fi
fi

# Changed files: staged + unstaged + untracked, vs base (covers PRs, local
# commits, and the working tree).
changed_files() {
  {
    git diff --name-only --diff-filter=ACM "$base" --cached
    git diff --name-only --diff-filter=ACM "$base"
    git ls-files --others --exclude-standard
  } | sort -u
}

# Bash 3.2 (macOS /bin/bash) has no mapfile; read the lists portably.
py_files=()
while IFS= read -r file; do
  [ -n "$file" ] && py_files+=("$file")
done < <(changed_files | grep '\.py$' || true)
rs_files=()
while IFS= read -r file; do
  [ -n "$file" ] && rs_files+=("$file")
done < <(changed_files | grep '\.rs$' || true)

# ============================================================================
# Tier 1: codelimit (Python refactoring alarm)
# ============================================================================
if command -v codelimit >/dev/null 2>&1; then
  if [ "${#py_files[@]}" -gt 0 ]; then
    echo "== [1/3] codelimit check (changed Python vs $base) =="
    codelimit check "${py_files[@]}"
  else
    echo "== [1/3] codelimit: no changed Python files vs $base, skipping =="
  fi
else
  echo "SKIP: codelimit not installed (brew install codelimit or pipx install codelimit)" >&2
fi

# ============================================================================
# Tier 2: lizard (Cross-language NLOC, CCN, and parameter limits)
# ============================================================================
if command -v lizard >/dev/null 2>&1; then
  if [ "${#py_files[@]}" -gt 0 ]; then
    echo "== [2/3] lizard check (Python: NLOC<=60, CCN<=15, PARAM<=7 vs $base) =="
    lizard -l python -T nloc=60 -C 15 -a 7 -W whitelizard.txt -w "${py_files[@]}"
  fi
  if [ "${#rs_files[@]}" -gt 0 ]; then
    echo "== [2/3] lizard check (Rust: NLOC<=100, CCN<=20, PARAM<=7 vs $base) =="
    lizard -l rust -T nloc=100 -C 20 -a 7 -W whitelizard.txt -w "${rs_files[@]}"
  fi
  if [ "${#py_files[@]}" -eq 0 ] && [ "${#rs_files[@]}" -eq 0 ]; then
    echo "== [2/3] lizard: no changed Rust or Python files vs $base, skipping =="
  fi
else
  echo "SKIP: lizard not installed (brew install lizard-analyzer or pip install lizard)" >&2
fi

# ============================================================================
# Tier 3: Clippy too_many_lines (Rust-native compiler AST gate)
# ============================================================================
if [ "${#rs_files[@]}" -gt 0 ]; then
  echo "== [3/3] clippy too_many_lines (changed Rust vs $base) =="
  violations="$(cargo clippy --all-targets --message-format=short -- -W clippy::too_many_lines 2>&1 \
    | grep "warning: this function has too many lines" || true)"
  if [ -n "$violations" ]; then
    changed_violations="$(echo "$violations" | while IFS= read -r line; do
      loc="$(echo "$line" | cut -d: -f1)"
      case " ${rs_files[*]} " in
        *" $loc "*) echo "$line" ;;
      esac
    done || true)"
    if [ -n "$changed_violations" ]; then
      echo "FAIL: too_many_lines violation in a changed Rust file:" >&2
      echo "$changed_violations" >&2
      echo "Refactor the function below the threshold in clippy.toml." >&2
      exit 1
    fi
  fi
  echo "clippy too_many_lines: no violations in changed Rust files"
else
  echo "== [3/3] clippy: no changed Rust files vs $base, skipping =="
fi

echo "== All 3 code limit & complexity gates passed =="
