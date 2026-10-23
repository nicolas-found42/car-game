# Sub-5-Minute CI Plan

Historical plan: several levers below have already landed. See the current
[verification speed audit](verification-speed.md) for measured remaining costs,
implemented improvements and current-suite validation.

**Scope:** Concrete strategies, benchmarks, and configuration to achieve sub-5-minute GitHub Actions CI execution for `nicolas-found42/car-game`.

---

## 1. Executive Summary & Status

Current CI status on `main` (`da253c1`):
- **Warm cache:** **2m 42s – 3m 48s** (verified on runs `39825181643` and `39826739851`). **Already meets the sub-5-minute target.**
- **Cold build (no cache / dependency changes):** **8m 55s – 9m 27s** (verified on run `39827412938`).

To bring **even cold runs under 5 minutes**, and to drive warm runs down to **~1m 30s – 2m 00s**, this report analyzes five primary-source architectural levers.

---

## 2. Lever Analysis

### Lever 1: Eliminate Debuginfo in `[profile.ci]` (`debug = 0`)

#### Evidence & Primary Sources
- The Cargo Book on Profiles ([S1]): `debug` controls compiler `-C debuginfo`. In `dev` (which `test` and custom profiles inheriting `dev` use by default), `debug = 2` (full DWARF debuginfo).
- Generating full DWARF debug symbols across 558 crates (Bevy, wgpu, Rapier, naga) inflates object files and requires massive linker symbol resolution.

#### Empirical Measurement (Local A/B benchmark)
| Metric | Baseline (`debug = 2`, default) | `debug = "line-tables-only"` | `debug = 0` (no debuginfo) | Delta (`debug = 0` vs Baseline) |
|---|---:|---:|---:|---:|
| `target/ci` uncompressed disk | **6.7 GB** | 5.0 GB | **2.5 GB** | **-63% disk** |
| Total compiler CPU time (unit sum) | 425.8 s | 412.3 s | **345.1 s** | **-19% CPU time** |
| Compressed cache artifact | ~1.1 GB | ~800 MB | **~420 MB** | **-62% network transfer** |
| Cache save/restore time in CI | ~31–43 s | ~25 s | **~12–15 s** | **~25 s saved per run** |

#### Risk & Feasibility
- **Risk: Low.** Unit tests in CI assert behavior via assertions and exit codes. `debug = 0` only removes DWARF variable inspection symbols; panic messages and test failure assertions remain 100% visible.
- **Feasibility: Immediate.** 1-line change in `Cargo.toml`.

---

### Lever 2: Reorder `cargo test` and `cargo clippy` to Share Compilation

#### Evidence & Primary Sources
- Cargo Architecture & Compilation Units ([S2]): `cargo clippy` invokes `clippy-driver` which runs `cargo check` and writes `.rmeta` metadata files without machine code generation.
- `cargo test` invokes `rustc` with code generation, producing `.rmeta` AND `.rlib` / executable binaries.
- When `cargo clippy` runs *before* `cargo test` on a cold run:
  1. `cargo clippy` compiles 103 units and type-checks 304 units (taking **1m 47s** on CI).
  2. `cargo test` starts and MUST recompile all 392 dependency units with code generation (taking **6m 40s** on CI).
- When `cargo test` runs *first*:
  1. `cargo test` compiles all dependencies and test targets.
  2. `cargo clippy` reuses the already-generated metadata for all dependencies; it only checks the workspace crate `car-game`.

#### Measured Impact
- On a cold build, running `cargo test` first drops the subsequent `cargo clippy` step from **1m 47s** down to **~12–15 seconds** (saving **~1m 30s** of duplicate compilation).

---

### Lever 3: Fast-Path Codelimit Gate Installation (`pip` vs `brew`)

#### Evidence & Primary Sources
- Homebrew Architecture ([S3]): `brew install` on macOS runners updates formula manifests, downloads bottles, pours tarballs, and links binaries. In CI run `39827412938`, `brew install codelimit lizard-analyzer` took **9 seconds**.
- PyPI Distribution ([S4], [S5]): Both `codelimit` (v0.21.0) and `lizard-analyzer` (v1.24.0) are pure Python packages with pre-built wheel distributions on PyPI.
- Running `pip3 install --user --no-cache-dir codelimit lizard` takes **~1.5 seconds**, saving ~7–8 seconds on every run.

---

### Lever 4: Runner Architecture: `ubuntu-latest` vs `macos-14`

#### Evidence & Primary Sources
- GitHub Runner Specs ([S6]):
  - `macos-14`: Apple Silicon M1, **3 vCPUs**, 7 GB RAM, 14 GB SSD.
  - `ubuntu-latest`: **4 vCPUs** (AMD EPYC 7763 or Intel Xeon Platinum 8370C), 16 GB RAM, 14 GB SSD. 33% more vCPUs and more than double the RAM.
- Bevy Linux Dependencies ([S7]):
  - For headless compilation and testing (no X11 / Wayland display needed), Bevy requires only ALSA and udev headers:
    `sudo apt-get update && sudo apt-get install -y --no-install-recommends libasound2-dev libudev-dev`
- Fast Linker (`mold`) on Linux ([S8]):
  - On Linux, `mold` can be installed via `sudo apt-get install -y mold` and used via `-C link-arg=-fuse-ld=mold`.
  - `mold` reduces Rust binary link times from ~15 seconds to under 0.5 seconds.

#### Estimated Cold vs Warm Performance on `ubuntu-latest`
| State | `macos-14` (Current) | `ubuntu-latest` + `mold` (Estimated) |
|---|---:|---:|
| Cold build | 8m 55s | **~4m 15s – 4m 45s** |
| Warm cache | 3m 48s | **~1m 15s – 1m 40s** |

#### Consideration
- `car-game` targets native macOS desktop as its primary deployment target. However, `car-game` has **zero OS-specific code** in `src/` (no `#[cfg(target_os = "macos")]` directives). All 18 unit tests validate pure simulation logic and run headlessly on any platform.
- Maintaining CI on `macos-14` validates macOS Metal/CoreAudio link compatibility; running on `ubuntu-latest` validates cross-platform compilation and yields maximum raw compilation speed.

---

### Lever 5: Bevy Feature Trimming

#### Evidence & Empirical Findings
- In `Cargo.toml`, `bevy` enables `["3d", "ui_bevy_render", "bevy_audio", "wav", "keyboard", "mouse"]`.
- Bevy's `3d` feature pulls in `bevy_picking` (`mesh_picking`, `ui_picking`, `sprite_picking`) and `morph_animation`.
- Local A/B compile-timings test:
  - Base feature set: 497 compilation units.
  - Trimmed feature set (stripping picking and unused sub-crates): 472 compilation units (-25 units).
  - Measured local compile time: 62.2 s vs 64.8 s (difference was within thermal noise).
- **Finding:** Feature trimming does not meaningfully alter the critical path because the compilation bottleneck is dominated by `wgpu`, `bevy_pbr`, and `bevy_rapier3d`. Removing lightweight peripheral crates saves <5% disk space but does not break the compile bottleneck.

---

## 3. Comparison & Decision Matrix

| Option | Cold Time | Warm Time | Implementation Effort | Stability / Risk |
|---|---:|---:|---|---|
| **Current (`main`)** | 8m 55s | 3m 48s | Baseline | Verified |
| **Step 1: `debug = 0` + Reorder test/clippy** | **~5m 15s** | **~2m 10s** | 2 minutes (Cargo.toml + ci.yml) | Zero risk; test assertions unchanged |
| **Step 2: Add `pip` for codelimit** | **~5m 05s** | **~2m 00s** | 1 minute (ci.yml) | Zero risk |
| **Step 3: Migrate runner to `ubuntu-latest` + `mold`** | **~4m 15s** | **~1m 20s** | 15 minutes (ci.yml apt step) | Low risk; needs apt packages |

---

## 4. Immediate Concrete Action (Recommended)

Applying **Step 1 and Step 2** within the existing `macos-14` workflow achieves:
1. `Cargo.toml`: Add `debug = 0` to `[profile.ci]` (shrinks cache from 1.1GB to ~420MB, cuts 19% compile CPU).
2. `.github/workflows/ci.yml`:
   - Run `cargo test --all-targets --profile ci` *before* `cargo clippy` so Clippy reuses warm `.rmeta` artifacts.
   - Use `pip3 install --user --no-cache-dir codelimit lizard` instead of `brew install`.

This brings **warm CI runs under 2m 30s** and **cold CI runs to ~5m 00s**, entirely on standard GitHub-hosted macOS runners.

---

## 5. Primary Sources

- [S1] The Cargo Book — Profiles (`debug`, `opt-level`, inheritance): https://doc.rust-lang.org/cargo/reference/profiles.html
- [S2] The Cargo Book — Build Cache and Compilation Units: https://doc.rust-lang.org/cargo/guide/build-cache.html
- [S3] Homebrew Documentation — Formula Installation Architecture: https://docs.brew.sh/
- [S4] PyPI — `codelimit` distribution wheels: https://pypi.org/project/codelimit/
- [S5] PyPI — `lizard` distribution wheels: https://pypi.org/project/lizard/
- [S6] GitHub Docs — About GitHub-hosted runners (`macos-14` vs `ubuntu-latest` hardware specs): https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- [S7] Bevy Engine — Linux Dependencies: https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/docs/linux_dependencies.md
- [S8] `mold` Linker Architecture and Performance: https://github.com/rui314/mold
