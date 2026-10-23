# CI Performance Analysis

Current follow-up: [verification speed audit](verification-speed.md). The 18-test
measurements below describe an earlier suite and workflow; use the follow-up's
66-test baseline and current PR results for today's timings.

**Scope:** GitHub Actions wall time for `nicolas-found42/car-game`. This document is analysis only; no workflow, `Cargo.toml`, or source changes were made as part of this analysis.

**Baseline revision:** PR head `4d83ad2` (the revision exercised by the runs below); workflow [`ci.yml`](https://github.com/nicolas-found42/car-game/blob/main/.github/workflows/ci.yml) [S16], manifest [`Cargo.toml`](https://github.com/nicolas-found42/car-game/blob/main/Cargo.toml) [S17]. Sources are listed at the end and cited as `[S#]`.

---

## 1. Measured baseline

The workflow defines one job, `rust`, on `runs-on: macos-14`, triggered by bare `on: push` and `on: pull_request`, with no cache step [S16]. The repository is public [S20], and the Actions cache usage API reports **0 active caches / 0 bytes** [S18], so every run in this baseline was cold.

### 1.1 Runs and step times

All three runs below target the same commit `4d83ad2`; times are from each run's public jobs API [S13b, S14b, S15b].

| Run | Event | Job wall | Clippy step | Test step | `Finished test profile` | Test execution |
|---|---|---:|---:|---:|---:|---:|
| [39817393080](https://github.com/nicolas-found42/car-game/actions/runs/39817393080) [S13] | `pull_request` | **17:01** | 2:47 | 13:58 | 13m 42s | 18 tests, 15.49 s |
| [39817389082](https://github.com/nicolas-found42/car-game/actions/runs/39817389082) [S14] | `push` | **26:21** | 4:02 | 21:55 | 21m 19s | 18 tests, 34.20 s |
| [39817383646](https://github.com/nicolas-found42/car-game/actions/runs/39817383646) [S15] | `push` | **26:54** | 4:45 | 21:48 | 21m 22s | 18 tests, 25.41 s |

Supporting step times from the PR run: checkout 1 s, Rust toolchain install 7 s, format <1 s, Python steps 2 s total, artifact upload 1 s [S13b].

### 1.2 Where the time goes

From the PR run log [S13]:

- The job contains **495 `Compiling` lines** total: 103 before the Clippy step finished and 392 during the test step. The Clippy step additionally contains 304 `Checking` lines.
- Cargo reports ``Finished `dev` profile [optimized + debuginfo] target(s) in 2m 47s`` for Clippy, then ``Finished `test` profile [optimized + debuginfo] target(s) in 13m 42s`` for tests [S13].
- The test-profile build alone is **822 s / 1021 s = 80.5%** of the 17:01 job; Clippy is **167 s / 1021 s = 16.4%**.
- Only 18 tests run, in **15.49 s** [S13]. Test execution is not the cost; dependency code generation is.

The same 495 compile lines appear in both push logs [S14, S15], so the work is identical across runs. Yet the test-profile build took 13m 42s versus 21m 19s and 21m 22s: a **+7m 37s (+55.6%)** spread for the same commit. Test execution also varied 15.49 s to 34.20 s. The cause of this run-to-run variance is **UNVERIFIED**; it is consistent with different/contended macOS runner instances, but the logs do not identify the hardware or contention.

### 1.3 Duplicate runs

For `4d83ad2`, the repository produced **two `push` runs and one `pull_request` run** (39817383646, 39817389082, 39817393080), all on branch `codex/live-platform-verification` [S13, S14, S15]. Aggregate job time was 17:01 + 26:21 + 26:54 = **70m 16s** across three macOS VMs; the two redundant push runs account for **53m 15s** of that. The public API does not expose why the push event produced two runs; that cause is **UNVERIFIED**.

`concurrency` alone cannot merge the push and PR runs: `push` uses `refs/heads/<branch>` while `pull_request` uses `refs/pull/<number>/merge` [S7], and the recommended group key includes `github.ref` [S6, S7]. Removing branch `push` triggers is the mechanism that removes the duplicates (§4).

The 22m baseline figure corresponds to the longer push-run test step (21:48–21:55), not the shortest run; the verified full-job range is **17:01 to 26:54** [S13, S14, S15].

---

## 2. Runner options

### 2.1 Standard GitHub-hosted runners

For both public and private repositories, the `macos-14` label is a **standard** runner with:

| Property | Value |
|---|---|
| Architecture | arm64 (Apple M1) |
| CPU | **3 cores** |
| RAM | 7 GB |
| SSD | 14 GB |
| Runner class | Standard GitHub-hosted |

Source: GitHub-hosted runners reference [S1]. Standard runners are **free and unlimited on public repositories** [S1]; where billed, the macOS 3/4-core rate is **$0.062/minute** [S4]. `macos-14` is not a larger runner.

### 2.2 Larger macOS runners

Larger runners are managed VMs with more resources, available **only to organizations and enterprises on GitHub Team or GitHub Enterprise Cloud** [S2, S3]. This repository is owned by a personal account (`nicolas-found42`) [S20], so larger runners are not currently selectable without moving the repository to an eligible organization. **This is an availability blocker, not just a cost decision.**

| Label | Architecture | Cores | RAM | SSD | Rate |
|---|---|---:|---:|---:|---:|
| `macos-14-xlarge` | arm64 (M2; pricing SKU says M2 Pro) | **5** (+8 GPU hardware acceleration) | 14 GB | 14 GB | $0.102/min |
| `macos-14-large` | x64 (Intel) | **12** | 30 GB | 14 GB | $0.077/min |

Specs: [S2]. Rates: [S4]. Standard macos-14 for comparison: 3 cores, 7 GB, $0.062/min [S1, S4].

### 2.3 Billing model: standard vs larger

- **Standard:** free/unlimited for public repositories [S1]. For private repositories, standard macOS consumes included minutes and is then billed at the per-minute rate [S1, S4].
- **Larger:** never eligible for included minutes; billed per minute for both public and private repositories, and **not free for public repositories** [S3, S4]. There is no charge while a larger runner is idle [S3].

Arithmetic at the measured 17:01 duration: 17.0 × $0.062 = **$1.05** (standard, if billed), 17.0 × $0.102 = **$1.73** (`macos-14-xlarge`), 17.0 × $0.077 = **$1.31** (`macos-14-large`). On this public repository the standard run is $0.

### 2.4 What the larger runner would and would not buy

- 5 cores vs 3 is 1.67× the cores at 1.65× the per-minute rate; 12 cores vs 3 is 4× the cores at 1.24× the rate [S2, S4]. Compile speedup is **not** proportional to core count (linking, memory bandwidth, and serial phases remain), so the expected wall-time reduction is **UNVERIFIED**.
- `macos-14-xlarge` is arm64, so the rustc host triple is unchanged and a rust-cache entry can still restore (rust-cache keys include the rustc host) [S9]. `macos-14-large` is x64, so it produces a different host key and cannot reuse the arm64 target cache [S9]; expect a cold cache after switching.
- Larger-runner pools are smaller; GitHub warns that a warm VM may not be ready on first request, that once a runner is in use VMs are ready for subsequent runs within 5 minutes, and that a subset stays warm afterward to reduce future assignment time [S2].

---

## 3. Cache strategy

### 3.1 Platform limits, eviction, and scope

GitHub's dependency-caching reference [S5] states:

| Topic | Behavior |
|---|---|
| Size limit | **10 GB per repository** by default; can be increased by owners/administrators; usage beyond 10 GB is billed. |
| Age eviction | Entries not accessed for **more than 7 days** are removed. |
| Pressure eviction | When at the limit, GitHub saves the new cache and evicts entries **oldest last-access date first**. |
| Mutability | An existing cache cannot be changed; a new key is required. |
| Branch scope | A run can restore caches from its **current branch and the default branch**. |
| PR scope | A PR run can restore the **base branch** (including fork base branches) but not child or sibling branches. |
| PR cache destination | A cache created by a PR run is scoped to the **merge ref** (`refs/pull/.../merge`) and can only be restored by re-runs of that PR. |
| Cache version | Key matching also includes a version stamped from the cached `path` and compression tool. |

The 10 GB/7-day/eviction rules are also summarized by rust-cache's README [S9].

### 3.2 How `actions/cache` restore keys behave

The `cache` action restores in this order [S5]:

1. **Exact match** for `key` (plus cache version).
2. **Partial/prefix match** for `key`.
3. Each `restore-keys` entry **sequentially**, also accepting prefix matches; among multiple partial matches, the **most recently created** cache wins.
4. If nothing matches, the job continues; on successful completion, a new cache is created with `key` [S5].

Search starts in the current branch, then retries the same steps on the default branch, subject to the scope rules above [S5]. A cache hit means an exact `key` match; a prefix restore is a miss for the `cache-hit` output [S5].

### 3.3 `Swatinem/rust-cache@v2`: what it caches and how it keys

`rust-cache` caches [S9]:

- `~/.cargo` — installed binaries, registry, registry cache, and git dependencies. `~/.cargo/registry/src` is deliberately **not** cached because Cargo can recreate it from the compressed archives in `~/.cargo/registry/cache`.
- `./target` — build artifacts of **dependencies**.

Before saving it removes pre-existing `~/.cargo/bin` files, unused dependencies, non-dependency artifacts, incremental artifacts, and artifacts with an mtime older than one week [S9]. It also sets **`CARGO_INCREMENTAL=0`** itself [S9].

The complete default key is composed from [S9, S10]:

- `job_id` (when `add-job-id-key` is `true`, the default);
- rustc release, host, and hash for installed toolchains;
- a hash of every `Cargo.lock` / `Cargo.toml`, `rust-toolchain` / `rust-toolchain.toml`, and `.cargo/config.toml` (when `add-rust-environment-hash-key` is `true`, the default);
- values of environment-variable **prefixes** listed in `env-vars` (default `CARGO CC CFLAGS CXX CMAKE RUST`).

It also attempts to restore from a previous `Cargo.lock` version, so a lockfile update should rebuild only changed dependencies [S9].

### 3.4 Requested inputs

Defaults are from `action.yml` and the README [S9, S10]:

| Input | Default | Semantics |
|---|---|---|
| `prefix-key` | `v0-rust` | Prefix for the whole key; change it to start a new cache namespace manually. |
| `shared-key` | empty | Used **instead of** the automatic job-based key, making the key stable across jobs. |
| `key` | empty | Additional key component alongside the automatic job-based key. |
| `add-job-id-key` | `true` | Includes the job id in the automatic key; this is what keeps different jobs' caches separate by default. |
| `workspaces` | `. -> target` | Newline-separated `$workspace -> $target` entries; target defaults to `target` under each workspace. |
| `cache-targets` | `true` | If `false`, only the Cargo registry is cached, not workspace `target` directories. |
| `cache-all-crates` | `false` | If `false`, only dependency crates are cached; workspace crates are excluded. If `true`, all crates are cached. |
| `cache-workspace-crates` | `false` | Caches workspace crates specifically. |
| `save-if` | `true` | If `false`, the cache is restored but never saved; intended for jobs that should not write caches. |
| `cache-on-failure` | `false` | If true, the post-step saves even when the job failed. |
| `cache-bin` | `true` | Caches `~/.cargo/bin` (with pre-existing binaries removed before save). |
| `lookup-only` | `false` | Checks the key and sets `cache-hit` without downloading. |
| `cache-provider` | `github` | Cache backend (`github` or `warpbuild`). |

### 3.5 macOS-specific caveats

- The action explicitly works around [cargo#8603](https://github.com/rust-lang/cargo/issues/8603) / [actions/cache#403](https://github.com/actions/cache/issues/403), which "would otherwise corrupt the cache on macOS builds" [S9]. This is the main platform-specific correctness caveat, and it is already handled by `@v2`.
- Cache keys include the rustc **host**, so arm64 and x64 caches are not interchangeable [S9]. Switching from `macos-14` to `macos-14-large` (Intel) invalidates the target cache; switching to `macos-14-xlarge` (arm64) does not.
- Artifacts older than one week are cleaned before save [S9]; rarely rebuilt dependencies can be dropped and rebuilt later.
- A target cache for this project is potentially large. Locally, `du -sh target` reports **13 GB**, with **11 GB** under `target/debug`, **9.8 GB** under `target/debug/deps`, and **1 GB** under `target/debug/incremental`. This is a **LOCAL observation, not a CI cache measurement**; the compressed CI cache size is **UNVERIFIED**. The relevant limits are 10 GB per repository [S5] and 14 GB of runner SSD [S1], so target caching could approach either limit. `rust-cache`'s removal of incremental and non-dependency artifacts reduces this, but the residual size is **UNVERIFIED**.
- The action's own README notes it is most effective for repositories with a high dependency/own-code ratio and a `Cargo.lock` [S9]. This repository has one binary/package and 495 `Compiling` lines per run (dependencies plus the workspace crate) [S13], so it matches that profile.

### 3.6 Suitability for a single Bevy binary

**Suitable, with cache-size monitoring.** The project has a `Cargo.lock` [S17], a small workspace (one package) and 495 `Compiling` lines per run (dependencies plus the workspace crate) [S13]; `rust-cache` caches dependency artifacts and excludes the frequently changing workspace crate by default (`cache-all-crates: false`) [S9, S10]. No `workspaces` configuration is needed because the default is `. -> target` [S9].

Recommended inputs for the current single-job workflow:

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    # Save only from the default branch; PR runs restore it. Manual dispatches
    # may seed or verify the cache on a branch.
    save-if: ${{ github.ref == 'refs/heads/main' || github.event_name == 'workflow_dispatch' }}
    # Leave shared-key unset: the default job-id key keeps this job's cache distinct.
    # Leave cache-all-crates false: the workspace crate changes every commit.
    # Leave workspaces at its default (". -> target").
```

Reasons:

- **Save on `main` only.** A PR-created cache is scoped to the PR merge ref and cannot seed `main` or another PR [S5]. Letting every PR save consumes the 10 GB budget with entries that only help that PR's re-runs. `main` caches are visible to PR runs [S5].
- **Do not set `shared-key` for one job.** It replaces the job-based key [S9]; with one job it provides no benefit and makes accidental cross-job sharing easier later.
- **Place the action after the toolchain install**, because the key uses the current rustc version [S9].
- **Expect one cold run** after adding the action or changing `Cargo.toml`/profile settings, because the key hashes those files [S9].
- **Monitor size** with the cache usage API [S18]. If the 10 GB limit causes thrash [S5], either increase the limit (billed [S5]), set `cache-targets: false` to cache only the registry [S9, S10], or reduce debug info in the CI profile (§5).

The cache removes dependency rebuild work, but the exact wall-time saved on a hit is **UNVERIFIED**; it is not claimed here as a specific number.

---

## 4. Workflow triggers and concurrency

### 4.1 Current behavior

`on: push` with no branch filter runs for every branch push, and `on: pull_request` runs for PR activity [S16]. For `4d83ad2` this produced two push runs plus one PR run [S13, S14, S15]. `pull_request` defaults to the `opened`, `synchronize`, and `reopened` activity types [S7], and its `GITHUB_REF` is the PR merge ref `refs/pull/<number>/merge`; the checked-out `GITHUB_SHA` is the last merge commit [S7].

### 4.2 Recommended trigger split

```yaml
on:
  push:
    branches: [main]
  pull_request:
```

- `push.branches` restricts the push event to `main` [S8], eliminating branch-push runs that duplicate the PR run.
- PRs still run for `opened`, `synchronize`, and `reopened` by default [S7].
- Optionally add `workflow_dispatch` for manual runs of a branch that has no PR.

This is a trigger-level deduplication: `concurrency` cannot do it because push and PR runs are in different refs and therefore different groups [S6, S7].

### 4.3 Concurrency

GitHub's concurrency rules [S6]:

- `concurrency.group` may be any string or expression; allowed contexts include `github`.
- At most one job/workflow per group runs at a time; by default a newly queued run replaces an existing **pending** run.
- `cancel-in-progress: true` also cancels the currently **running** member of the group.
- Group names are case-insensitive.
- Group names should include `github.workflow` when multiple workflows exist, otherwise a group collision can cancel another workflow's run.
- `cancel-in-progress` may be a conditional expression.

Recommended workflow-level block:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

Why this shape:

- `github.ref` is `refs/pull/<n>/merge` for a PR [S7], stable across `synchronize`; a new push to the PR therefore cancels the superseded run in the same group [S6].
- On `main`, `cancel-in-progress` is false, so an in-progress validation is not killed mid-run. Note that the default queue behavior still replaces a **pending** run when a newer one arrives [S6]; if every `main` commit must be validated, do not put `main` pushes in a cancelling/serializing group, or use a separate workflow for `main`.
- Including `github.workflow` prevents this workflow from cancelling unrelated workflows [S6].
- Push and PR runs still have different groups, so concurrency does not deduplicate them; the `push.branches: [main]` filter does.

### 4.4 Effect

For a PR commit, the observed pattern changes from three runs totaling 70m 16s of job time to one PR run (17m 01s in the best observed case), plus the eventual `main` push run after merge. On a public repository the standard minutes are free [S1], but this removes two redundant macOS VMs per PR commit and the associated capacity/queue pressure and duplicate status checks.

---

## 5. Profile options

### 5.1 Current profile and measured effect

```toml
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

[S17]

Cargo documents that `opt-level` controls `-C opt-level` [S11, S19] and that "higher optimization levels may produce faster runtime code at the expense of longer compiler times"; it also warns that level 3 can be slower than level 2 and recommends re-evaluating [S11]. The `"*"` package override applies to all dependencies but **not** workspace members [S11]. Build scripts, proc macros, and their dependencies keep the fast build-override defaults (`opt-level = 0`) unless overridden [S11].

The `test` profile **inherits from `dev`** [S11], so `cargo test` uses the dev package override. A local verbose build of a two-package fixture with these exact profile tables emitted `-C opt-level=3` for the dependency and `-C opt-level=1` for the binary under `cargo build --profile test -v`; the package override reaches the test profile through inheritance. This matches the CI log's ``Finished `test` profile [optimized + debuginfo]`` line [S13].

Measured consequence in CI: the test step performs **392 compiles** at this optimization level and takes 13m 42s in the fast run, 21m 19s/21m 22s in the slower runs [S13, S14, S15]. Clippy, which checks rather than code-generates, takes only 2m 47s–4m 45s.

### 5.2 A CI-only profile

Custom profiles must declare `inherits`; their output goes to `target/<profile-name>` [S11]. A CI profile can lower dependency optimization without changing local `dev` behavior:

```toml
[profile.ci]
inherits = "dev"
incremental = false

[profile.ci.package."*"]
opt-level = 0   # try 1 if test runtime becomes unacceptable
```

Use it consistently for both Rust checks:

```sh
cargo clippy --all-targets --profile ci -- -D warnings
cargo test --all-targets --profile ci
```

Notes and trade-offs:

- `--profile ci` is the documented way to select a custom profile and applies to all targets [S11]. Using the same profile for Clippy and tests keeps one `target/ci` directory, so the cache covers both.
- The profile changes the optimization level under which tests execute. Tests that depend on floating-point details (physics) could behave differently; that risk is **UNVERIFIED** and must be validated by running the suite under the new profile. Keep the runtime result visible in the log (the baseline test execution is only 15–34 s, so a slower optimized-at-0 run has headroom).
- Changing `Cargo.toml` changes the rust-cache key once [S9], so the first run after the change is cold.
- `opt-level = 0` for dependencies is the largest available compile-time lever because it targets the 392-codegen-compile phase; the actual wall-time reduction is **UNVERIFIED**. If Bevy/Rapier test runtime becomes too slow, use `opt-level = 1` for the dependency override.

Local implementation check (Apple M5 Pro, 18 cores; **not CI**): a cold `cargo test --profile ci --all-targets` finished in 95 s wall with all 18 tests passing (test execution 43.78 s, versus 15.49 s at the dev profile's dependency `opt-level = 3`). `cargo clippy --profile ci` took 25 s, and `target/ci` was 7.3 GB after both, below the 10 GB cache limit before registry contents. The CI wall-time reduction remains to be measured.

### 5.3 Incremental compilation

`dev` (and therefore `test` by inheritance) defaults to `incremental = true`; incremental data is stored in `target` and is used only for workspace members and path dependencies [S11]. `CARGO_INCREMENTAL=0` forces it off, and `1` forces it on [S12].

For CI, incremental compilation has little to reuse on a fresh VM and writes extra artifacts. `rust-cache` already sets `CARGO_INCREMENTAL=0` and strips incremental artifacts before saving [S9]. Setting `incremental = false` in `[profile.ci]` makes the intent explicit even if the cache action changes. This affects only the workspace crate, not the 495 `Compiling` lines.

### 5.4 Optional debug-info reduction

`[profile.ci]` could also set `debug = "line-tables-only"`; Cargo documents the valid values and that `dev` defaults to full debug info [S11]. The effect on **compile time** is **UNVERIFIED**; the plausible benefit is a smaller `target` directory, which matters for the 10 GB cache and 14 GB runner disk [S5, S1]. It preserves file/line information for test backtraces. Measure before adopting.

---

## 6. Recommendation

### 6.1 Ordered by impact/effort

| # | Change | Expected effect | Effort | Risk |
|---|---|---|---|---|
| 1 | Limit `push` to `main`; keep `pull_request` | Removes 2 redundant push runs per PR commit: 70m 16s → 17m 01s aggregate job time in the observed SHA | 2 lines | Low: branch pushes without a PR stop running automatically |
| 2 | Add `Swatinem/rust-cache@v2` with `save-if` on `main` | On hit, removes the dependency rebuild that dominates the 13m 42s–21m 22s test-profile build; first run cold | One step | Medium: 10 GB cache/14 GB disk pressure; size **UNVERIFIED** |
| 3 | Add PR-only `cancel-in-progress` concurrency | Superseded PR pushes stop consuming 17–27m macOS runs; removes stale checks | 4 lines | Low: `main` runs are not cancelled mid-flight |
| 4 | Add `[profile.ci]` with lower dependency `opt-level` | Directly reduces the 392 codegen compiles; magnitude **UNVERIFIED** | ~8 lines + test validation | Medium: tests run at a different optimization level |
| 5 | Split fmt/clippy/test into parallel jobs | At best overlaps Clippy's 2m 47s–4m 45s with the 13m 58s–21m 55s test step; ~16% ceiling while the test build dominates | Medium: 3 VMs, cache keys, duplicated setup | Medium: duplicate cold builds and cache pressure |
| 6 | Move to `macos-14-xlarge` (5-core M2) or `macos-14-large` (12-core Intel) | More cores per job; speedup **UNVERIFIED** | Low code change | High cost/availability: paid, Team/Enterprise only, not free on public repos |

### 6.2 Recommended workflow inputs (combined)

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}

jobs:
  rust:
    runs-on: macos-14
    env:
      RUST_BACKTRACE: "1"
      RUST_LOG: car_game=debug,bevy_asset=warn,wgpu=error,naga=warn
    steps:
      - uses: actions/checkout@v4

      # Toolchain must be selected before rust-cache: the key includes rustc version.
      - name: Install Rust tools
        run: |
          rustup toolchain install stable --profile minimal --component rustfmt --component clippy
          rustup default stable

      - name: Cache Rust dependencies
        uses: Swatinem/rust-cache@v2
        with:
          save-if: ${{ github.ref == 'refs/heads/main' }}

      - name: Format
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy --all-targets --profile ci -- -D warnings

      - name: Rust tests
        run: cargo test --all-targets --profile ci
```

(If the `[profile.ci]` change in §5.2 is not adopted, drop `--profile ci`. The rest is independent. This snippet shows the Rust-related steps only; retain the existing Python checks and artifact-upload steps from `ci.yml`.)

### 6.3 Sequencing

1. **Do 1 and 3 first.** They are configuration-only, remove duplicate/stale runs immediately, and do not change build semantics.
2. **Add 2 next.** It is the recurring per-run win and has no effect on what is tested. Start with the default `cache-targets: true`; watch the cache usage API [S18] and the 10 GB limit [S5]. If the target cache is too large, try `cache-targets: false` or the CI profile before giving up on caching.
3. **Then evaluate 4.** It attacks the dominant 13m 42s codegen phase, but it changes the optimization level under which tests execute. Run the suite under `--profile ci` locally (and inspect the timing) before relying on it.
4. **Only then consider 5.** Splitting cannot remove the test job's dependency build; on a cold cache it merely overlaps Clippy's check pass and duplicates setup/registry work across VMs. If jobs are split, keep the default job-id cache keys; do not give Clippy and tests the same `shared-key`, because cache keys are immutable [S5] and two parallel jobs racing to save the same key can leave only one job's artifacts cached [S5, S9].
5. **Treat 6 as a paid capacity decision.** It is unavailable on the current personal repository [S2, S3, S20], and its throughput gain is unmeasured.

### 6.4 Why splitting fmt/clippy/test is not the first lever

Each GitHub-hosted runner is a new VM [S1], so jobs share no filesystem or `target` directory. On a cold cache:

- The test job still performs the full 392-compile, 13m 42s–21m 22s build [S13, S14, S15].
- The Clippy job separately performs 407 compile/check operations (103 `Compiling` + 304 `Checking`) in 2m 47s–4m 45s [S13, S14].
- Running them in parallel caps the saving at the Clippy step, i.e. 2m 47s of a 17m 01s job (16.4%) in the best case; `needs: clippy` would serialize them and save nothing [S8].
- Warm caches change the arithmetic, but Clippy artifacts (check metadata) and test artifacts (rlibs) are different, and parallel cache saves with one key can race [S5, S9]. Use per-job keys or let the test job own the target cache.

In short: the wall-time ceiling for splitting on a cold build is small, while the dependent-build duplication and cache complexity are real. Cache and profile changes reduce the dominant build itself.

### 6.5 Measured result after implementation

The workflow in this branch was run twice via `workflow_dispatch` on the same commit, once cold and once with the cache restored. Both runs passed all steps.

| Run | Job wall | Clippy | Rust tests | Test-profile build | Cache |
|---|---:|---:|---:|---:|---|
| Baseline PR run [S13] | 17:01 | 2:47 | 13:58 | 13m 42s | none |
| Cache miss + `--profile ci` (39823798149) | **9:27** | 1:44 | 6:40 | 6m 22s | saved 1.1 GB (43 s post-step) |
| Cache hit, pre-codelimit (39825181643) | **2:42** | 0:09 | 1:58 | 22 s | restored in 19 s |
| Cache hit, final workflow incl. codelimit (39826739851) | **3:48** | 0:12 | 2:42 | 31 s | restored in 31 s |

The final warm run is **4.5× faster** than the best baseline run and **6.9× faster** than the slow push runs, with the same 18 tests passing and the codelimit gate green. The active cache is **1.1 GB**, well below the 10 GB repository limit. The remaining 2m42s test step is mostly the 18 tests at dependency `opt-level = 0` (about 2m10s execution) plus a 31s workspace build; further gains would come from test execution time, not dependency builds.

---

## 7. Sources

All URLs accessed 2026-10-22.

| ID | Source |
|---|---|
| S1 | GitHub Docs — GitHub-hosted runners reference (standard runner specs, free/unlimited public runners, 14 GB SSD, arm64 limitations): https://docs.github.com/en/actions/reference/runners/github-hosted-runners |
| S2 | GitHub Docs — Larger runners reference (macOS Large/XLarge specs and labels, warm-pool/queue behavior): https://docs.github.com/en/actions/reference/runners/larger-runners |
| S3 | GitHub Docs — Larger runners concepts and billing (Team/Enterprise availability, per-minute billing, no included minutes): https://docs.github.com/en/actions/concepts/runners/larger-runners |
| S4 | GitHub Docs — Actions runner pricing (macOS standard and larger per-minute rates, larger runners not free for public repos): https://docs.github.com/en/billing/reference/actions-runner-pricing |
| S5 | GitHub Docs — Dependency caching reference (10 GB default, 7-day eviction, LRU eviction, branch/PR scope, restore-key matching, immutability, cache version): https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching |
| S6 | GitHub Docs — Control the concurrency of workflows and jobs (`group`, `cancel-in-progress`, conditional expressions, `github.workflow` grouping): https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency |
| S7 | GitHub Docs — Events that trigger workflows (`pull_request` default activity types, `refs/pull/<n>/merge` ref, `GITHUB_SHA`): https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows |
| S8 | GitHub Docs — Workflow syntax for GitHub Actions (`on.<push>.<branches>`): https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax |
| S9 | Swatinem/rust-cache v2 README (cached paths, key composition, cleanup, `CARGO_INCREMENTAL=0`, macOS workaround, effectiveness, inputs): https://github.com/Swatinem/rust-cache/blob/v2/README.md |
| S10 | Swatinem/rust-cache v2 `action.yml` (input defaults and post-save condition): https://github.com/Swatinem/rust-cache/blob/v2/action.yml |
| S11 | The Cargo Book — Profiles (`opt-level`, `test` inherits `dev`, custom-profile `inherits`, `target/<profile>`, package overrides, precedence, incremental, build-override): https://doc.rust-lang.org/cargo/reference/profiles.html |
| S12 | The Cargo Book — Environment variables (`CARGO_INCREMENTAL`): https://doc.rust-lang.org/cargo/reference/environment-variables.html |
| S13 | Actions run 39817393080 (PR baseline; job/step timings and log lines): https://github.com/nicolas-found42/car-game/actions/runs/39817393080 |
| S13b | Jobs API for run 39817393080 (per-step start/completion times): https://api.github.com/repos/nicolas-found42/car-game/actions/runs/39817393080/jobs |
| S14 | Actions run 39817389082 (push baseline, slow variant): https://github.com/nicolas-found42/car-game/actions/runs/39817389082 |
| S14b | Jobs API for run 39817389082: https://api.github.com/repos/nicolas-found42/car-game/actions/runs/39817389082/jobs |
| S15 | Actions run 39817383646 (duplicate push run): https://github.com/nicolas-found42/car-game/actions/runs/39817383646 |
| S15b | Jobs API for run 39817383646: https://api.github.com/repos/nicolas-found42/car-game/actions/runs/39817383646/jobs |
| S16 | Repository workflow at `main`: https://github.com/nicolas-found42/car-game/blob/main/.github/workflows/ci.yml |
| S17 | Repository manifest at `main`: https://github.com/nicolas-found42/car-game/blob/main/Cargo.toml |
| S18 | Actions cache usage API (reported 0 active caches, 0 bytes at analysis time): https://api.github.com/repos/nicolas-found42/car-game/actions/cache/usage |
| S19 | rustc codegen options (`-C opt-level`): https://doc.rust-lang.org/rustc/codegen-options/index.html#opt-level |
| S20 | Repository metadata API (public, owner `nicolas-found42`, default branch `main`): https://api.github.com/repos/nicolas-found42/car-game |
| S21 | Actions run 39823798149 (cache-miss run with `--profile ci`): https://github.com/nicolas-found42/car-game/actions/runs/39823798149 |
| S22 | Actions run 39825181643 (cache-hit run): https://github.com/nicolas-found42/car-game/actions/runs/39825181643 |

---

## 8. Verification limits

- The baseline in §1 is the workflow as measured in the cited runs. The implementation in this branch is measured separately in §6.5; those two runs are cited as S21 and S22.
- Step times and log lines are from the public runs above; job walls are the sum of step times as reported by the jobs API.
- Run-to-run variance is real for an identical SHA (13m 42s vs 21m 19s/21m 22s test-profile builds), so any single-run estimate should be treated as a range, not a constant.
- The following remain explicitly **UNVERIFIED**: cause of the duplicate push runs; cause of the 55.6% baseline build-time variance; whether the 18-test suite is exhaustive evidence that lower optimization preserves behavior beyond the exercised paths; larger-runner speedup.
- The local target-size figures (13 GB total / 11 GB debug / 9.8 GB deps / 1 GB incremental) are a `du` observation on the development machine, not a CI cache measurement, and are labeled as such in §3.5. The measured CI cache is 1.1 GB (§6.5).
