# Issue 96: CI recorder deadline repair

PR #102, based on `bdd2bee`, retains one behavior change:
`[profile.ci.package.sha2] opt-level = 3`. All other CI dependencies retain their
existing profile. `.github/workflows/ci.yml` is restored byte-for-byte from
`bdd2bee`. Production code, assertions, 8-second readiness / 10-second shutdown
bounds, fixture deadlines and normal CI test concurrency are unchanged.

## Controlled experiment

[Probe 39887029794](https://github.com/nicolas-found42/car-game/actions/runs/39887029794)
at `91e67c8ed8d5c38a561f9bdafbcdcef7782430dc` ran on `macos-14`,
macOS 14.8.9, 3 logical CPUs, aarch64, rustc 1.99.1. The temporary probe built
the production executable with `profile.ci.package.sha2.opt-level=0`, ran both
shutdown controls, then rebuilt with only SHA-256 optimized and ran all targets.
The probe intentionally recorded baseline exit statuses without stopping; its
job success alone is not evidence that the baseline passed.

| Workload | Result | Test execution time |
| --- | --- | --- |
| Unoptimized, `--test-threads=1` | 0 passed, 4 failed, exit 101 | 34.17 s |
| Unoptimized, default concurrency | 0 passed, 4 failed, exit 101 | 16.09 s |
| Optimized, default concurrency | 4 passed, 0 failed | 14.24 s |
| Optimized recorder health | 21 passed | 30.17 s |
| Optimized library tests | 16 passed | 106.00 s |
| Optimized scenarios | 15 passed | 28.35 s |

Each baseline has three readiness failures and one bounded shutdown failure
(the panic/busy-writer control). Serial failure rules out concurrent-startup
contention as a sufficient explanation; optimizing only `sha2` is sufficient
for this measured workload. This is a controlled dependency-profile result,
not a per-function CPU trace or a universal latency guarantee.

All eight failed children have empty stdout/stderr and an empty `run.log`;
the retained size inventories contain no manifest, health file, provenance
capture or object. `RunRecorder::try_create_with_config` creates `run.log` before
`Manifest::new` synchronously hashes the executable, and only afterwards opens
the writer and publishes the manifest and initial health. This places the
observed stall before writer initialization, consistent with unoptimized
executable hashing. Later capture also hashes the executable and prepared
assets before app construction. No partial-object progress or exact hash
throughput was captured; absence of those files must not be reported as such.

## Retained evidence

[Probe logs and workload identity](issue-96-probe/) include the complete serial,
concurrent and optimized test logs plus all eight failed-child size inventories.
[The SHA-256 manifest](issue-96-probe/retained-evidence.json) inventories the
original failed controls, downloaded probe logs, failed-child logs/fixtures and
complete timestamped job log under the durable primary checkout:
`/Users/nicolas/Documents/github/car-game/artifacts/issue-96/`.
The Actions step timestamps retain compilation-inclusive timing; the table
above uses the test harness execution times and excludes compilation.
Original controls are unchanged main `bdd2bee` run 39880399889, predecessor
39879824498 and PR #95 run 39884795037. Their original checksum receipts remain
in `CHECKPOINT.json`; that file is a historical checkpoint, not current status.

## Final verification

The complete local ladder passed under one outer `cargo_serial.py` lease;
all 11 command exit statuses are zero. [Command timings](issue-96-probe/local-results.json)
include formatting, Clippy with warnings denied, the HEAD complexity gate,
all-target tests, Python syntax, source asset integrity, 17 provenance controls,
7 recorder-health controls, fleet asset controls, the serial shutdown control,
and `bash scripts/verify.sh`. The code-limit script correctly had no changed
Rust/Python files to inspect. No global thresholds or exceptions were added.

Local default-concurrency tests passed 16 library, 21 recorder-health, 4 shutdown
and 15 scenario tests. Shutdown execution took 4.19 s at default concurrency
and 12.50 s serially; these are local dev-profile observations, separate from
the CI-profile probe above. The full offline chain also passed 13 fleet drive
scripts, 52 capture checks and run-evidence verification. This is automated
fleet verification, not a new human visual review.

Raw logs, the lease acquire/release record, initial source diff/compiler/target
identity, local SHA-256 receipts and a complete copy of the fleet run are
retained under `artifacts/issue-96/local/` in the primary checkout. Verification
started with the workflow restoration applied to `2e7c368`; `98b57d9` committed
that same behavior and the probe evidence during execution. Subsequent edits
only update documentation/evidence. All commands, including subprocesses and
release fleet execution, ran inside the same whole-command Cargo lease.

Normal final-head CI uses the restored workflow. Its exact SHA, run URL,
conclusion and corrected logs are recorded in PR #102 and the primary
checkout's `artifacts/issue-96/final-ci/`; consult the latest check on the PR
for merge readiness. This avoids changing the head merely to embed its own
CI result. Issue #96 and its dependency relationship with #19 remain open
until human merge. This repair does not replace #92's bundle retention work.

## Standards

Independent review against `bdd2bee`: zero documented-standard breaches and
zero baseline smell findings. Corrected the evidence document's method name
to `try_create_with_config`; no production changes resulted.

## Spec

Independent review against issue #96: zero implementation findings or scope
creep. Workflow identity, all 52 retained probe-file hashes, original deadlines
and default concurrency were verified. Final normal CI is the remaining
external acceptance check; its outcome is recorded as described above.
