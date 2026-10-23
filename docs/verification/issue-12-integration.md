# PR #95 — main integration and merge review

Combined production source: `4fa5ef6b9e0973844ec943d69934d2c5ae116642`, merging main
`2fbd42a439724e27d1e88a7e068b5ffe830ac36e` (CI repair #102) into
`codex/issue-12-assembly`. The merge was clean. `Cargo.toml` and the normal CI
workflow match main exactly. No assembly, rendering, recorder, assertion,
deadline, complexity threshold or exception changed during this integration.

## Fresh verification

The [integration receipt](issue-12-integration.json) records exact runs and hashes.
Raw material lives at
`/Users/nicolas/Documents/github/car-game/artifacts/issue-12/integration-main-20261023/`.
Checkout-local target objects were APFS-cloned from the primary cache, then rebuilt
for this checkout. One outer universal `cargo_serial.py` lease covered the fast
ladder through every test child; a second covered production scenarios, recorded
fleet execution and the full offline script. Both use `CARGO_INCREMENTAL=0`,
`RUST_TEST_THREADS=1`, and no external `CARGO_TARGET_DIR`. See
`serialized-integration.sh`, `serialized-runtime.sh` and `logs/*lease.log`.

- Format, warning-free all-target Clippy, HEAD complexity and true-base complexity
  against `2fbd42a` passed. Real codelimit/lizard tools were present; unchanged
  Python was correctly skipped. No thresholds or whitelist entries changed.
- All **66 Rust tests** passed: 18 library, 7 assembly, 21 recorder health,
  4 shutdown, 16 production scenarios. Python syntax, source asset integrity,
  17 provenance controls, recorder-health and fleet-asset controls passed.
- Recorded assembly: **4/4 assertions**, revision 0→1→2, domain and Rapier mass
  1320→1295→1320 kg, readiness and stale rejection; nonblank 1920×1080 capture.
- Recorded driving/reset: **3/3 assertions**, displacement, speed and one reset;
  nonblank 1920×1080 capture.
- Recorded fleet: **13 drive scripts / 52 captures**; explicit fleet, run,
  health and portable provenance verification passed. This is automated fleet
  verification, not a new human visual review or a new baseline equality test.
- These three runs share executable SHA-256
  `b216586bc64657db655e947d0fbb692df53773bc3ebfb429d38d631c53ff333e`.
  All have healthy finalized recorders, complete inventory/build/runtime material
  and `source_build_match`. Runtime replay completeness remains false; platform
  replay prerequisites remain external. Executable and sidecar are preserved in
  `preserved-build/` outside the disposable build cache.

The first assembly capture attempt remains a **failed run**: all four assertions
passed, but the 30-tick cap expired before capture completion (`tick-limit`).
`assembly-capture-fixture.json` and `assembly-scenario/` retain that exact input
and failure. The successful `assembly-capture-bounded.json` keeps identical
commands/assertions and allows 900 ticks for asynchronous capture. Both rendering
fixtures use explicit 20-second readiness / 30-second wall bounds. No tracked
fixture, harness deadline or production threshold was changed.

The full `bash scripts/verify.sh` command and its completion are retained in
`logs/full-offline.log` and `logs/runtime-lease.log`. Final completion and normal
CI links are recorded in [PR #95](https://github.com/nicolas-found42/car-game/pull/95)
after the checks finish, avoiding a documentation commit merely to embed its own
CI result. Normal CI retains its default test concurrency and main's repaired
SHA-256 dependency profile. The earlier failing CI is historical, not a waiver.

## Standards

Independent review of `2fbd42a...4fa5ef6`: zero documented-standard violations
and zero actionable baseline smells. Repository instructions, domain glossary,
ADRs, production changes, tests and evidence boundaries were reviewed.

## Spec

Independent review against #12 and parent #9: zero actionable missing/incorrect
requirements or scope creep. Typed identities, graph separation, atomic rejection,
mass/tensor derivation, corner capabilities and production inspection/scenario
integration satisfy the issue's implementation boundary. This review does not
grant downstream phase acceptance.

## Retained boundaries

The [original receipt](issue-12-evidence.json) is unchanged. All **20** retained
artifact hashes were rechecked successfully (`historical-receipt-recheck.json`).
The original fleet run and its exact executable remain historical evidence.
The **601 matching readiness-relative post-Writeback ticks 120–720**, with zero
position/velocity/longitudinal/endpoint error, compare the original recorded
`bdd2bee` reference and `82ada61` candidate only. They are not relabeled as a
measurement of this merged source. No new universal handling, CPU p99,
performance, fidelity or audio-quality claim is made.

Native built-in screenshot-readback intermittency remains unresolved. The original
failed raw native smoke runs, including unchanged-main control, remain failures.
Original visible native driving/reset/pause/quit observations are distinct from
those failures and from the new passing offscreen Metal captures. No new native
window inspection or passing `verify_live.sh` is claimed. Full P1/live integration
acceptance remains #19; P2 fasteners/picking/workshop work and downstream #13–#15
acceptance retain their owners. Stop before merge; issue #12 closes on merge.
