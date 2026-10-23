# CI build research

Research date: 2026-10-23. Scope: reduce the normal macOS CI wall time, including
the first build with no usable dependency cache. This note records primary-source
findings and proposed experiments. It does not claim measured improvements from
those experiments. Actual measurements belong in the accompanying performance
audit and retained job logs.

The investigation started from main after #95 and #103: CI dependencies at
optimization level 1, SHA-256 at level 3, no debug information, no incremental
compilation, and all 67 Rust tests. The reported main job was 16:24 cold and 3:37
cached; the earlier PR's cached job was 2:10. The parent investigation compares
their individual steps and hardware records. A different elapsed time alone
cannot identify the cause.

## Measure the critical path first

Add `--timings` to the existing Cargo test and Clippy invocations and retain the
HTML reports. Cargo records compiler invocation durations, dependency unlocks,
features, code generation and concurrency. Its graph does not expose all
parallelism inside rustc. Use the reports to identify slow crates, duplicated
builds and long dependency chains before choosing a profile change.
[Cargo build timing documentation](https://doc.rust-lang.org/cargo/reference/timings.html)

For each experiment retain: source and tree hash; `rustc -Vv`; runner image and
architecture; CPU count; cache match/key and archive size; compiler invocation
flags; Cargo build time; test execution time per executable; Clippy time; cache
restore/save time; and total job time excluding queue. Use an empty target
directory for a true cold compilation comparison. A warm target directory is
not a cold build merely because the source commit changed.

Do not run two compiler experiments on the same host at once. Do not mix profile
variants in the target directory used for a paired comparison. Repeat promising
hosted results on the same source because VM, transfer and CPU variation remain.

## Profile findings

Cargo defines level 0 as unoptimized and level 1 as basic optimization; larger
levels trade compilation for runtime and require measurement. `lto = false`
still permits thin local LTO; `lto = "off"` disables LTO. More codegen units may
reduce compilation time at a runtime cost. Named package overrides win over
wildcard package overrides, which win over build overrides. Build overrides can
also cause shared runtime/build dependencies to compile twice. Level 1 permits
sharing monomorphized generic code across crates; levels 2 and 3 inhibit this
sharing. Stripping symbols can reduce diagnostics. These are tradeoffs, not
guaranteed improvements.
[Cargo profile documentation](https://doc.rust-lang.org/cargo/reference/profiles.html)

Current `debug = 0` already removes debug information. Disabling debug information
again offers no additional saving. `strip = "symbols"` is not a suitable first
experiment because useful names in failed shutdown backtraces must remain.
Do not change debug assertions, overflow checks, panic unwinding or the release
profile to obtain a lower CI number.

Apple's linker manual explicitly says `-x` removes non-global symbols useful for
backtrace names; `-S` removes debug information. Consequently, neither stripping
local symbols nor spelling `strip = "debuginfo"` establishes a safe way to remove
the large existing symbol/string table while preserving names. `-X`, which drops
local `L` symbols, is already the documented default. No additional safe symbol
table reduction was established in this research. Optimization that eliminates
code and its associated symbols remains a different approach.
[Apple linker symbol-table options](https://github.com/apple-oss-distributions/ld64/blob/main/doc/man/man1/ld-classic.1)

The current wildcard dependency override also applies to compiler-time crates.
Adding only `[profile.ci.build-override] opt-level = 0` will therefore not undo
their wildcard level 1. Inspect actual rustc flags before assuming that a setting
changed. Prefer named opt-level 0 overrides for measured build-only hot crates
over a sweeping inheritance rewrite. A crate used both at build time and runtime
requires special care. This is a candidate, not an implemented change.

The rustc documentation confirms that separate codegen units allow parallel
LLVM processing. It also says Cargo already disables embedded bitcode where
possible. Do not add `embed-bitcode=no` and attribute an improvement without
first confirming that the original compiler invocation enabled it.
[rustc code generation options](https://doc.rust-lang.org/rustc/codegen-options/index.html)

Cargo's job limit is configurable. On a small runner, increasing it may increase
memory pressure rather than throughput. Try the runner's CPU count and one lower
value only after timing/CPU records suggest contention. Do not substitute local
M5 measurements for the hosted runner result.
[Cargo build configuration](https://doc.rust-lang.org/cargo/reference/config.html#buildjobs)

The effective existing nonincremental CI compilation uses 16 codegen units.
Do not infer 256 from an incremental development default. The first local
build-tool optimization-level-0 candidate also set 256 units, so it changed two
variables and is excluded as evidence for the optimization-level change alone.
Future comparisons must retain the same effective codegen units.

## Measured SHA-256 backend opportunity

The locked `sha2` 0.10.9 selects its aarch64 SHA-256 hardware backend only when
the crate's `asm` feature is enabled. Without that feature, aarch64 reaches the
software compression path. This is a backend selection change, not a change to
the SHA-256 algorithm or the material hashed.
[sha2 0.10.9 backend selection source](https://docs.rs/sha2/0.10.9/src/sha2/sha256.rs.html)

The hardware backend checks `sha2_hwcap::get()` before invoking a function
annotated with `target_feature(enable = "sha2")`; the false branch calls software
compression. The existence of this branch should not be described as a runtime
probe on every platform.
[sha2 aarch64 implementation at the release tag](https://github.com/RustCrypto/hashes/blob/sha2-v0.10.9/sha2/src/sha256/aarch64.rs)

In locked `cpufeatures` 0.2.17, the Apple aarch64 SHA-2 check returns true because
the crate assumes SHA-2 instructions are present on Apple ARM64 hardware.
Linux/Android use the operating system's hardware capability flags. Other
unsupported platforms report no detected feature. Thus the acceleration retains
the library's platform policy and software fallback rather than imposing a
repository-wide `target-cpu=native` or unconditional hardware-instruction flag.
[exact cpufeatures source revision from its packaged VCS metadata](https://github.com/RustCrypto/utils/blob/9d92d5e95ab4c07c5d8bfd024bf2a17e96d20feb/cpufeatures/src/aarch64.rs)

Cargo supports dependencies conditional on `target_arch`. Add the `asm` feature
under an aarch64 dependency declaration so the additional feature request is
limited to that architecture. This target condition applies to compilation,
not only test builds; retain other architectures' dependency declarations.
Verify resolved flags/features for both architectures and keep the lockfile
consistent. Cargo target dependency tables cannot express `cfg(test)` or
profile-specific dependency selection.
[Cargo target-specific dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)

The parent audit measured a standalone release probe over the same
174,721,312-byte retained CI executable, hashing all bytes five times. Software
took **1.492938 seconds**; the enabled hardware backend took **0.267649 seconds**,
about 5.58 times faster for this in-memory workload. Both logs contain five
identical hashes:
`82caa3e06def11e4ee28d36f03fca50912bc16a79b63d20cfedc5efe33f9e964`.
The parent also checked this digest against Python `hashlib`. Raw local evidence:
`artifacts/round2-sha-soft.log`, `artifacts/round2-sha-hardware.log`, and
`artifacts/round2-sha-bench/`.

This probe isolates hashing; it excludes executable reads, binary capture,
process startup and simulation. It is not an established whole-suite or hosted-CI
speedup. Keep known-vector and streaming/file hashing checks and all existing
provenance tests. The parent is implementing the target-scoped backend request
with these checks. After confirming the real suite, reconsider dependency
optimization choices: a large executable's repeated software hashing need no
longer force broad compiler optimization solely to make hashing cheap. File
read/capture cost still exists, so executable size remains relevant.

## Dependency features

The repository already sets `default-features = false` for Bevy and Rapier.
However, Bevy 0.20's `3d` is a broad collection: application/platform defaults,
3D rendering, scenes and picking. Its render collection includes anti-aliasing,
glTF, PBR, post-processing and glTF animation. Platform defaults include gamepad,
clipboard, windowing, default font, multithreading, custom cursors and system
information. The enabled UI rendering collection adds UI API/rendering.
[Bevy 0.20 feature definitions](https://github.com/bevyengine/bevy/blob/v0.20.0/Cargo.toml)

Replacing `3d` with fine-grained features could remove real compilation work.
It could also remove behavior registered through default plugins without any
source import failing. A successful headless test suite is insufficient evidence
for this change. First produce an expanded feature/plugin inventory and determine
which capabilities the product promises. Preserve 3D rendering, UI, WAV audio,
keyboard/mouse, macOS windowing, multithreading and the existing scene/asset
pipeline. Preserve gamepad/clipboard/cursor behavior unless their removal is an
explicit product decision. Keep this as a separate product change if it alters
the supported surface. No feature removal is recommended for the immediate
performance patch.

Removing target-specific X11/Wayland feature labels is not automatically a large
macOS saving: a feature name does not prove that Linux-only dependencies execute
on macOS. Check the resolved host build graph before claiming a reduction.

A narrower profile experiment can keep every renderer feature while changing
only compiler optimization for named shader-processing crates. Candidate names
are `wesl`, `wgsl-parse` and `naga`, subject to the timing graph showing they are
material. Bevy's shader crate directly depends on `wesl` with `naga-ext`; inspect
the resolved graph for the downstream crates rather than assuming all shaders
are processed at build time.
[Bevy shader crate manifest](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_shader/Cargo.toml)

Keep ECS, task scheduling, math/physics, SHA-256 and the application at their
control settings for that comparison. Compiler optimization level 0 does not
remove shader validation, parsing or features, but can make runtime shader
preparation slower. Accept the experiment only after recorded rendering/startup
verification as well as the headless suite; preserve the release profile and
do not describe the candidate as behavior-neutral merely because it compiles.

## Cache improvements are a separate category

`rust-cache` caches dependency artifacts, keys them by compiler/environment and
manifest inputs, and normally excludes workspace artifacts and incremental data.
It sets `CARGO_INCREMENTAL=0`. It can restore an older compatible dependency
inventory after lockfile changes. Thus a cached run can still compile and link
the application and test executables. Enabling workspace caching should be a
measured comparison of extra archive bytes against recompilation saved, not an
assumption that it makes every source change free.
[rust-cache's documented behavior](https://github.com/Swatinem/rust-cache)

Main cannot consume a PR merge-ref cache as its own saved cache. PR caches have
merge-ref scope; cache lookup can consult accessible default-branch caches, and
existing cache contents are immutable. A new namespace causes a deliberate miss.
Avoid changing cache prefixes during routine profile/source experiments. Keep
trust boundaries intact and do not attempt to promote untrusted PR cache material
into main through a privileged workflow.
[GitHub cache reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)

Pinning a supported Rust version can avoid an unplanned compiler-version cache
change and make comparisons reproducible. It does not reduce a true empty-cache
compile. Rustup supports exact release toolchains; keep a documented update path
and test new stable versions rather than freezing the compiler indefinitely.
[Rustup toolchain names](https://rust-lang.github.io/rustup/concepts/toolchains.html)

`sccache` is worth testing only after measuring the current dependency archive.
Its Rust backend requires incremental compilation off and cannot cache crates
that invoke the system linker, including binaries and proc macros. It also warns
about procedural macros reading filesystem input. A populated remote cache may
reduce transfer/rebuild costs; it cannot remove computation from the first build
when both caches are empty. Report its hit/miss/uncacheable counters and total
job time, including setup and storage requests.
[sccache Rust limitations](https://github.com/mozilla/sccache/blob/main/docs/Rust.md)

Prewarming main or scheduling a dependency build can move cold work outside a PR
feedback window. That improves feedback latency, not the CPU cost of a real cold
build. It also consumes runner time. Do not report it as a cold compilation win.

## Hardware and linker options

GitHub currently documents the standard `macos-14` arm64 runner as 3 M1 CPUs and
7 GB RAM. Changing `macos-14` to another standard arm64 macOS label does not
promise more hardware. Standard Intel runners use a different architecture and
therefore require different artifacts and can change platform coverage.
[Standard runner specifications](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)

The documented larger arm64 macOS runner has 5 M2 CPUs and 14 GB RAM. It is a
credible paid experiment for reducing cold CPU work elapsed time, but speedup
will not equal the CPU-count ratio if dependencies serialize. Larger runners
require billing setup; keep cost per successful check and architecture alongside
elapsed time. This is an option, not a silent workflow change.
[Larger runner specifications and billing requirements](https://docs.github.com/en/actions/reference/runners/larger-runners)

LLVM provides `ld64.lld` for Mach-O and documents how to select it through clang.
Its performance claim is against ld64, not a guarantee against this runner's
current Xcode linker. Measure final link time first; replacing a linker cannot
eliminate time spent compiling Bevy. Include installation/download costs and
check SDK/framework linkage, application startup, child shutdown/backtraces and
recorded executable provenance before adoption.
[LLVM Mach-O linker documentation](https://lld.llvm.org/MachO/index.html)

Current evidence weakens the case for a linker migration: in September 2026 the
LLVM Mach-O maintainers reported that the newer Xcode linker was 2–3 times faster
than ld64.lld and opened an optimization effort. This is their workload, not this
repository's benchmark, but it means old claims that lld always beats Apple's
linker are unsuitable justification. Keep the runner's modern Apple linker as
the control and rank a replacement below profile/codegen experiments.
[LLVM maintainers' performance discussion](https://discourse.llvm.org/t/plan-to-improve-ld64-lld-performance/91715),
[upstream tracking issue](https://github.com/llvm/llvm-project/issues/222068)

## Ranked experiments

| Rank | Experiment | Why test it | Acceptance condition |
| --- | --- | --- | --- |
| 1 | Retain Cargo timing reports and host/cache metadata | Separates repeated workspace/link work, CPU compilation and transfer | Same gates; no claimed speedup from instrumentation itself |
| 2 | CI `lto = "off"`, then a separate codegen-units comparison | Can reduce actual optimization/codegen work without dropping code | Cold plus cached whole-job wins; all 67 tests and unchanged diagnostics pass |
| 3 | Opt-level 0 only for timing-identified build-only crates | Removes optimization from compiler tools, keeping runtime profile | Actual flags confirmed; no duplicate-build regression; macro execution included |
| 4 | Selective runtime opt-level 0 for expensive renderer-only crates | Could cut cold compilation while simulation paths stay optimized | Test executable size, startup hashing, all tests and full rendering verification remain acceptable |
| 5 | Measure workspace cache or sccache as alternatives | Can reduce cached rebuilding/transfer | Include archive costs and cache misses; never reuse a prior test result |
| 6 | Paid larger arm64 runner | More CPU/RAM can reduce genuine cold elapsed time | Record cost, same macOS behavior/checks; user selects spending change |
| 7 | Mach-O linker comparison | Potential repeated-link saving | Only worthwhile if linker is material after setup; no diagnostic/provenance regression |

Changing several settings at once obscures causality and risks another expensive
cold experiment with no explanation. Keep one control and one candidate at a
time. First eliminate a candidate locally if it loses clearly; then validate
the promising candidate on the real small macOS runner.

`cargo-nextest` is lower priority here. Its process-per-test model can schedule
tests across binaries, but existing shutdown test leases are in-process locks.
A runner migration would require equivalent cross-process resource isolation.
It also does not remove dependency compilation, and doctests require a separate
Cargo invocation. Do not adopt it solely from advertised speedup figures.
[Nextest execution model](https://nexte.st/docs/design/why-process-per-test/),
[Nextest documentation](https://nexte.st/)

All experiments retain the existing fleet/baseline evidence boundaries. A new
test runner, profile or linker does not resolve native screenshot readback or
create new exact-source GPU evidence.
