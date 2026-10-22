# Verification & Code Quality Gates

The standard verification ladder for agents working in `car-game`. Run these checks before submitting pull requests, changing mechanics, or marking tasks complete.

## Verification Ladder

Execute checks from top to bottom. Fix any failure at the current rung before proceeding down the ladder.

```
1. cargo fmt --check                     Format (Rust)
2. cargo clippy --all-targets -- -D warnings
                                         Compiler lints & AST bounds (Rust)
3. scripts/check_codelimit.sh HEAD       Multi-tier code limits & complexity (Rust + Python)
4. cargo test --all-targets              Headless physics, controls & production scenarios
5. python3 -m compileall -q tools        Syntax verification for Python tooling
6. python3 tools/check_assets.py         Asset integrity (textures, cars, audio receipts)
7. scripts/verify.sh                     Full offline chain (includes offscreen fleet check)
```

---

## 1. Fast Local Checks (Every Code Edit)

Run after modifying any Rust or Python file:

```sh
# Format Rust code
cargo fmt

# Compiler lints with all warnings treated as errors
cargo clippy --all-targets -- -D warnings

# Multi-tiered code limits & complexity gate against current working tree / HEAD
scripts/check_codelimit.sh HEAD

# Fast headless tests
cargo test --all-targets
```

Completion criterion: all commands exit with code `0`, zero Clippy warnings, zero test failures.

---

## 2. Multi-Tier Code Limit & Complexity Gate

`scripts/check_codelimit.sh [base-ref]` (default: `origin/main`, falls back to `main` or explicit ref like `HEAD`) enforces a three-tier quality policy on changed files:

| Tier | Tool | Target | Thresholds | How to Grandfather Existing Debt |
|---|---|---|---|---|
| **Tier 1** | `codelimit` | Changed `tools/*.py` | LOC > 60 fails | Hard-coded. Use `# nocl` on function def line if strictly necessary. |
| **Tier 2** | `lizard` | Changed `src/*.rs` & `tools/*.py` | **Rust**: NLOC ≤ 100, CCN ≤ 20, PARAM ≤ 7<br>**Python**: NLOC ≤ 60, CCN ≤ 15, PARAM ≤ 7 | Add `file_path:function_name` to `whitelizard.txt`. |
| **Tier 3** | `clippy` | Changed `src/*.rs` | `too-many-lines-threshold = 100` (`clippy.toml`) | Add `#[allow(clippy::too_many_lines)]` with a descriptive comment. |

### Grandfathering Rules
- **Do not raise global thresholds** to accommodate a single oversized function.
- **Bevy systems** with heavy query signatures legitimately exceed 7 parameters: annotate with `#[allow(clippy::too_many_arguments)]` and record in `whitelizard.txt`.
- **Refactoring rule**: When refactoring an existing grandfathered function below threshold, remove it from `whitelizard.txt` to lock in the improvement (ratchet).

---

## 3. Asset & Tooling Checks

When touching `tools/*.py`, `assets/`, or receipts in `assets/cars/*.json`:

```sh
# Syntax check
python3 -m compileall -q tools

# Asset hash and receipt integrity (runs offline; requires ../carmodels for full re-check)
python3 tools/check_assets.py
```

---

## 4. Full & Live Verification Chains

When touching vehicle physics (`src/vehicle.rs`), simulation loop, rendering/camera, or roster:

### Offline Verification (`scripts/verify.sh`)
Runs the full local chain, offscreen fleet rendering across all 13 cars, and evidence generation:

```sh
sh scripts/verify.sh
```
Logs and captures are written to `artifacts/logs/` and `artifacts/latest/`.

### Live Platform Verification (`scripts/verify_live.sh`)
Runs offscreen checks with audio muted (`CAR_GAME_MUTE=1`) and attempts a bounded 30-second native window launch on macOS Metal hardware.

```sh
sh scripts/verify_live.sh
```
Evidence is written to `artifacts/live-platform/native-window.log`.
