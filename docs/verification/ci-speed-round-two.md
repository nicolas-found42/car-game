# CI performance, second pass

This follow-up starts at main `2342545`, after #95 and #103. The task is to
explain cached main's 3:37 versus the PR's 2:10, reduce remaining work, and
investigate the 16:24 empty-cache job without weakening checks.

## Where the cached difference went

Both cached runs compiled only `car-game`; dependency cache restoration succeeded.
They used Rust 1.99.1, image `macos-14-arm64`, all 67 tests, and identical source
trees. The PR run was 40015398244 attempt 2; main was 40018842266 attempt 2.

| Work | Cached PR | Cached main | Difference |
| --- | ---: | ---: | ---: |
| Whole job, excluding queue | 130 s | 217 s | 87 s |
| Workspace compilation | 45.25 s | 86 s | 40.75 s |
| Test harnesses | 33.44 s | 57.70 s | 24.26 s |
| Clippy | 3.90 s | 8.34 s | 4.44 s |
| Rust cache step | 19 s | 20 s | 1 s |

Compilation, tests and Clippy slowed together. This supports lower runner
throughput as the main explanation; it does not prove CPU contention or a
particular hardware fault. There was no extra dependency build or additional
main-only test workload. The old runs did not retain enough host detail to
identify a specific hardware difference. Future runs retain CPU/memory/compiler
inventory and Cargo's per-crate timing reports.

Main's first job took 984 s: 783 s compilation, 43.72 s test harnesses and 86 s
cold Clippy. Queue time is separate. See the [first main run](https://github.com/nicolas-found42/car-game/actions/runs/40018842266/attempts/1)
and [cached main run](https://github.com/nicolas-found42/car-game/actions/runs/40018842266/attempts/2).

## ARM hashing

`sha2` 0.10.9 selects its ARM hardware SHA-256 backend only with the `asm`
feature. Previously the macOS ARM job used the software backend, despite the
existing level-3 optimization of the hashing crate. Enable that feature only for
macOS `aarch64`. Upstream cpufeatures assumes SHA2 support on Apple ARM64; its
other ARM dispatch paths retain hardware checks and a software fallback. Other
targets keep their existing feature selection.
[Backend selection source](https://docs.rs/sha2/0.10.9/src/sha2/sha256.rs.html).

A standalone release probe hashed the same 174,721,312-byte executable five times:
software 1.492938 s, hardware 0.267649 s (5.58 times faster). Every digest matched
both the other backend and Python `hashlib`:
`82caa3e06def11e4ee28d36f03fca50912bc16a79b63d20cfedc5efe33f9e964`.
This isolates hashing CPU cost; it is not a whole-CI speedup claim.

A new regression checks empty input, `abc`, the million-`a` known-answer vector,
and 65,537 patterned bytes across one-shot, streaming, manifest-file and captured
object hashing. Captured bytes, digests and sizes must agree with fixed independent
answers. The existing 67 tests remain. No hashes, copied evidence, fsyncs, failure
checks, time bounds or driving ticks are removed.

## Controlled probes and exclusions

- Disabling local LTO increased the executable from 174,721,312 to 205,131,584
  bytes. The local harness observations were 25.02 s (control) and 29.07 s (off).
  Its 112 s dependency rebuild was not a true empty-target comparison, so it
  cannot establish a cold-build improvement. It is not selected from this data.
- Three serialized workspace-only compilations with a three-job Cargo limit
  took 21.79 s (implicit default), 22.48 s (64 units) and 21.57 s (16 units).
  No useful workspace compile improvement was established. CI disables incremental
  compilation; its effective codegen default is 16, not 256.
- A fresh-target control compiled all test targets in 192 s locally. This measures
  compiler work with an already populated registry, not a completely empty runner.
- An exploratory build-tool-only profile explicitly used 256 codegen units and
  took 249 s. That also changed runtime code generation, so it is confounded and
  cannot isolate build-tool optimization. The corrected follow-up was interrupted
  when the hardware-hashing lead was found; no completion result is claimed.
- Two earlier local workspace probes overlapped by mistake. Cleaning the target
  while the first suite was running removed a not-yet-executed test binary. Those
  timings and the resulting launch failure are excluded from performance evidence;
  they are not an application defect or a passing test run.

Local probes share the user's host with other workloads. Even with serialized
Cargo invocations, these observations are directional evidence rather than a
statistical benchmark. Hosted validation remains required.

## Shader compiler candidate

Keep the normal CI dependency level 1 and SHA-256 level 3, with named level-0
overrides only for `wesl`, `wgsl-parse` and `naga`. Physics, ECS, all dependency
features, release settings and every assertion remain unchanged. The shader
compiler's code is still built and checked; optimization work is reduced.

A fresh-target local candidate compiled all targets in 125 s, versus the earlier
192 s control. This is an observation on the shared host, not a guarantee. The
candidate also includes the hardware hash feature and one additional regression;
the baseline compiled the original 67 tests. Both targets started empty, while
registry sources were already present. Per-crate observations:

| Compiler invocation | Control | Candidate |
| --- | ---: | ---: |
| wesl | 46.47 s | 11.65 s |
| wgsl-parse | 16.66 s | 5.41 s |
| naga library | 12.98 s | 7.47 s |
| unchanged bevy_ecs | 21.18 s | 17.36 s |

The unchanged ECS time also varied, so the complete 67 s difference is not
attributed solely to the overrides. All 68 tests and the full headless ladder
passed with the candidate. Its executable is 202,327,232 bytes versus the earlier
174,721,312 bytes. Fast hashing makes that tradeoff less costly, but hosted
cold/cached jobs and actual CI-profile rendering remain required before acceptance.

The hardware-only intermediate normal CI at `d89133c` passed in 199 s, including
new cache publication. Its test harnesses took 24.15 s versus main's 57.70 s;
workspace compilation was 78 s versus 86 s and Clippy was 7.02 s versus 8.34 s.
It used an Apple M1 virtual runner with 3 CPUs and 7 GiB RAM. This is not a fully
warm repeat or a measurement of the later shader profile.

## Cold measurement and evidence

Manual CI dispatch accepts `cold_build=true`. It requires an absent target
directory and bypasses both Rust-cache restore and save; a fresh hosted runner
then downloads and compiles the normal locked graph. The same tests, Clippy,
complexity, Python controls and log retention run. Normal push/PR jobs still use
the existing cache namespace and compatible fallback inventories.

The accompanying [primary-source research](ci-build-research.md) distinguishes
actual compilation savings from moving work into a cache, and records rejected
shortcuts including lost backtrace symbols and reduced platform/features.

Raw evidence is retained under
`/Users/nicolas/Documents/github/car-game/artifacts/verification-speed/`, with
`round2-*` filenames. Final hosted measurements are recorded in the follow-up PR
to avoid a source commit solely to quote its own CI result.

Historical fleet and baseline comparisons retain their original source bounds.
These compiler/hash experiments do not establish a new native screenshot-readback
pass or resolve the existing native limitation.
