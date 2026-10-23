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
- Tiny physics unit-test fixtures use Bevy's single-threaded executor to avoid
  task-pool handoffs. A new differential regression compares pose, velocity,
  grounded contacts, wheel rotation and suspension with the default executor at
  every one of 1,381 ticks, including acceleration, steering, handbrake, braking,
  reset and reverse. Production scenario apps and rendered fleets retain their
  normal scheduling. Cargo profiles, including the SHA-256 repair, are unchanged.
- Rust dependency caches can now be saved for PR merge refs, using rust-cache's
  default policy. Previously a PR that changed profiles rebuilt those dependencies
  on every push until merge. PR-scoped caches cannot replace main's cache; warm
  reruns can reuse them. No assertion result or source-specific test executable
  is substituted for a fresh test execution.
  Completed dependencies are retained even if a later test/lint fails.
  Cache pruning was investigated and rejected: CI showed only its CI profile
  (10 GiB before rust-cache cleanup), so deleting dev/release directories would
  not shrink the existing archive. No artificial cache-key bump or cache purge
  was retained. Cache transfer remains a measured overhead.
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
| CI profile, parallel fleet: scheduler overrides only | 17.48 s | 12.62 s | Reject after actual CI: costly dependency fanout, no demonstrated CI test win |
| CI profile, parallel fleet: tiny-fixture single-threaded executor | 17.48 s | 5.72 s | Keep; 67% less execution time, no dependency rebuild |
| CI profile, parallel fleet: Rapier/Parry overrides only | 17.48 s | 14.92 s | Reject; only 2.56 s saved, ~14 s rebuild cost |

The first CI experiment with scheduler package overrides spent **182 seconds**
compiling tests versus the baseline's 46.77 seconds; library execution was 75.14
seconds versus 71.09. These are different hosted runners, so this is not a
controlled attribution of runtime regression. It does show a bad compilation
tradeoff and no demonstrated CI execution win. Both scheduler and physics package
overrides were removed. Final compiler profiles match #95; no dependency,
renderer feature, debug assertion, overflow check or production profile changed.
No tests, driving ticks, deadlines, capture resolution or macOS coverage were removed.

The first normal CI passed all Rust tests and real complexity gates, then a new
gate regression exposed the existing warning filter's Bash 3.2 parser failure.
The local harness had used newer Bash from PATH. The filter now compares paths
without nested case/command-substitution parsing; regressions explicitly use
`/bin/bash`, matching the script's declared interpreter. Both changed-warning
rejection and unchanged-warning acceptance are verified by result and diagnostic.

Raw baseline job/status, probes, CPU sample, pre-fix failing gate regressions and
final verification logs are retained in
`/Users/nicolas/Documents/github/car-game/artifacts/verification-speed/`.
`fleet-baseline-local.log`, `fleet-parallel-local.log`, `ci-fleet-baseline-local.log`,
`ci-fleet-ecs-local.log`, `ci-fleet-physics-local.log`, and
`ci-fleet-single-executor.log` retain compiler and harness
timings separately. The CPU sample contains waiting threads as well as active
stacks; it is not a CPU-percentage attribution.

## Shutdown control and retained failures

Run 39921783100 confirmed the fixture speedup (19 library tests in 17.44 s,
versus 18 in 71.09 s in the baseline), including exact executor equivalence.
It then failed the existing busy-writer panic shutdown deadline; the other three
shutdown controls passed. No production functions or compiler profile settings
changed in that run. Its child directory was only retained in the ephemeral
runner temp folder, so the missing child state is not inferred from the parent
backtrace. The run remains a failure.

A controlled local backtrace probe passed with backtraces on (6.12 s) and off
(5.84 s). That small difference does not establish backtrace printing as the
cause; no backtrace suppression, timeout increase, test retry or assertion
relaxation was introduced. Failed shutdown controls now copy bounded process/run
diagnostics to `artifacts/logs` for the actual upload. Each file is capped at
2 MiB; missing/oversized/uncopied material remains explicit in the size inventory.
The panic/export control now obtains an exclusive process-fixture lease; the
other three independent process controls may still overlap. The lease is acquired
before spawning the application, so each child's existing startup/shutdown clocks
and assertions are unchanged. This avoids competing with other test children
for the same small runner's CPU/IO budget. A fully serialized local probe passed
all four controls in 18.88 s; final code serializes only the panic case.

A faster local suite also exposed a pre-existing `DirectoryNotEmpty` cleanup race
in the timed-out-writer metadata test: its background writer could still publish
after the fixed 150 ms cleanup sleep. The original recorder assertions and
observation windows are preserved. Only temporary-directory removal retries that
specific race for up to two seconds after the assertions; other cleanup errors
still fail. This is not a retry of a behavioral assertion or CI job.

Final normal CI is required; retained failed runs are not counted as green.

## Final validation and remaining costs

The pull request records final normal CI timings and full local verification
after they complete. Keeping final CI results in the PR avoids another source
commit solely to quote its own check result. Compare job/step timings separately
from runner queue time; this repository cannot eliminate hosted-runner capacity
delays. The macOS runner remains the same.

The recorded full run is a real 13-car/52-capture GPU workload at intermediate
source `c227edb`, before the final test-fixture refinements. It passed all 66 tests
then present, all Python controls, and inventory/build/runtime-material provenance.
The later refinements affect only cfg(test), CI/tooling and docs; production
vehicle source before cfg(test) was compared unchanged. Final validation runs 67
Rust tests including executor equivalence. The recorded GPU run is not relabeled
as an exact-source run of the later commit. No new
native screenshot-readback pass, universal handling/performance budget, platform
replay completeness or downstream P1 acceptance is inferred. The exact historical
fleet/baseline evidence boundaries and unresolved native readback limitation in
[issue-12.md](issue-12.md) remain intact.
