# Verification performance audit

Baseline: PR #95 head `74ae517`, normal macOS CI
[39916542889](https://github.com/nicolas-found42/car-game/actions/runs/39916542889).
This follow-up is stacked on #95 and does not change its prepared branch.

## Measured bottlenecks

The baseline job took **416 seconds (6:56)**, excluding runner queue time.

| Step | Seconds | Finding |
| --- | ---: | --- |
| Rust tests | 180 | 46.77 s build; 71.09 s library, 24.29 s recorder health, 12.82 s shutdown, 24.38 s scenarios |
| Complexity | 173 | Script invoked default dev Clippy after CI-profile tests and Clippy; built another dependency graph, hiding its output |
| Rust cache restore | 40 | Real transfer overhead; not test execution |
| Clippy | 5 | Already reuses the CI profile |
| Rust install | 7 | Small compared with the duplicate build |
| Remaining setup/Python/upload | 11 | Minor relative to the two dominant steps |

The complexity step's Homebrew setup occupied about four seconds in the job log;
the remaining time was the hidden Cargo invocation. Replacing Homebrew alone
would not fix this bottleneck. The original gate also swallowed compiler failures
through `grep ... || true`; that is now a regression-tested hard failure.

The recorded local GPU fleet took 290 seconds. Its script deliberately spends
22 seconds on each of 13 cars: a **286-second workload floor**, plus loading and
finalization. Running a recorded fleet and then running the entire offline chain
again duplicated that workload. A full ladder followed by `verify.sh` also
duplicated all headless gates. These are orchestration costs, separate from CI.

## Implemented changes

- The complexity script honors `CODELIMIT_CARGO_PROFILE`; CI sets `ci`, local
  calls default to `dev`. Compiler failure is preserved independently of whether
  the warning filter matches. Thresholds and changed-file ratcheting are unchanged.
- CI installs the same codelimit 0.21.0 / lizard 1.24.0 versions in a Python 3.13
  virtual environment with a pip cache. Python is explicit because codelimit's
  published metadata excludes the local host's Python 3.15. A real 3.13 install
  and both tool CLIs were checked. No Homebrew update/cleanup runs on the CI path.
- The fleet physics test uses at most four workers, bounded by available CPUs.
  Each car still gets its own production app and exactly the same tick sequence
  and assertions. All catalog entries are discovered dynamically, as before;
  child panic propagates through scoped joins. Production source outside
  `#[cfg(test)]` is unchanged.
- Only CI's measured ECS/tasks scheduler dependencies receive opt-level 3.
  Other CI dependency optimizations, SHA-256 repair, debug assertions, overflow
  checks, production profiles, features and recorder bounds are unchanged.
- `verify.sh --base REF` owns the full ladder once, with per-command timings.
  `--recorded` uses one recorded release build and one fleet run for both full
  acceptance and source provenance. It now explicitly includes all targets,
  doctests, Python syntax, provenance/health/asset controls and gate regressions.
  `--headless` provides an explicit iteration mode; it does not claim GPU acceptance.

The [Cargo profile documentation](https://doc.rust-lang.org/cargo/reference/profiles.html)
describes the compile/runtime tradeoff; settings were selected using executions,
not an assumption that maximum optimization is always fastest. Pinned Python and
pip caching follow [setup-python's documented support](https://github.com/actions/setup-python/blob/v5/README.md).

## Controlled local comparisons

Same Apple M5 Pro host, same target, one whole-command Cargo lease. Test execution
times below exclude compilation. They are individual observations, not latency
guarantees or a statistical benchmark. Fleet assertions were compared after
whitespace normalization and found identical; every comparison passed.

| Comparison | Before | After | Decision |
| --- | ---: | ---: | --- |
| Dev fleet contract: sequential vs bounded per-car parallelism | 16.10 s | 9.42 s | Keep; 41% less execution time |
| CI profile, parallel fleet: scheduler overrides only | 17.48 s | 12.62 s | Keep; 28% less execution time; ~35 s local rebuild cost |
| CI profile, parallel fleet: Rapier/Parry overrides only | 17.48 s | 14.92 s | Reject; only 2.56 s saved, ~14 s rebuild cost |

The scheduler override rebuilds dependent Bevy crates once. Its advantage is
primarily repeated warm execution; this is not a claim of a cold-build speedup.
The narrow physics probe was not retained. No broad renderer optimization,
feature removal, skipped tests, shorter simulated driving, relaxed deadlines,
lower capture resolution or replacement of macOS coverage was used.

Raw baseline job/status, probes, CPU sample, pre-fix failing gate regressions and
final verification logs are retained in
`/Users/nicolas/Documents/github/car-game/artifacts/verification-speed/`.
`fleet-baseline-local.log`, `fleet-parallel-local.log`, `ci-fleet-baseline-local.log`,
`ci-fleet-ecs-local.log`, and `ci-fleet-physics-local.log` retain compiler and harness
timings separately. The CPU sample contains waiting threads as well as active
stacks; it is not a CPU-percentage attribution.

## Final validation and remaining costs

The pull request records final normal CI timings and full local verification
after they complete. Keeping final CI results in the PR avoids another source
commit solely to quote its own check result. Compare job/step timings separately
from runner queue time; this repository cannot eliminate hosted-runner capacity
delays. The macOS runner remains the same.

The final recorded full run remains a real 13-car/52-capture GPU workload. No new
native screenshot-readback pass, universal handling/performance budget, platform
replay completeness or downstream P1 acceptance is inferred. The exact historical
fleet/baseline evidence boundaries and unresolved native readback limitation in
[issue-12.md](issue-12.md) remain intact.
