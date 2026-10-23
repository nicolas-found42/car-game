# Issue #12 — major-part assembly

Review: [PR #95](https://github.com/nicolas-found42/car-game/pull/95), open against `main`. Stop before merge. Issue #12 remains open until merge; downstream work consumes the stacked branch.

Resumed after CI repair #102: [combined-source verification and completed review](issue-12-integration.md), [fresh integration receipt](issue-12-integration.json). The original results and CI failure discussion below remain historical; final normal CI status is on PR #95.

Owner worktree: `issue-12-assembly`, branch `codex/issue-12-assembly`, based on main `bdd2bee` (merged #10/#11/#87/#88). Current #9 and #12 read in full. No predecessor PR was open to reconcile. Native issue blockers remain open until user-approved merge; this branch provides a stacked code prerequisite only.

## Implemented seam

- `assembly::Assembly` owns opaque typed VehicleId, PartId, DefinitionId, OwnerId and TransactionId. Persisted parts use one attachment or exclusive root location, independent prerequisite edges and functional networks. Runtime ECS body map and compiled revision/wheel mask are excluded from the ledger.
- `ProductionCommand::{InspectAssembly,AssemblyTransaction}` commit in the existing `FixedFirst` dispatch boundary. Commands validate a cloned candidate and replace the ledger only on success. Exact retries are idempotent; transaction-ID conflicts and stale revisions reject.
- Public `validate`, `inspect`, `aggregate`, `transact`, `part`, `root`, `installed`, `wheel_mask` support snapshot/control/physics integration without creating another owner.
- Pilot vehicle `pilot-350z`, chassis `350z-chassis`, wheel instances `350z-wheel-0`..`3`, chassis mounts `wheel-0`..`3`. Four 25 kg authored test wheels and a 1220 kg residual chassis calibrate intact mass=1320 kg, COM=0 and inertia diag(1800,2300,650) kg·m². These are explicitly P1 approximations, not recovered internals or operating manifests.
- The pilot manifest binds exact prepared 350Z asset SHA, body node 0 and eight wheel/tire node names+indices. Residual collider references the existing compound chassis shape; wheels retain the existing raycast contact model. Picking geometry is explicitly unavailable until P2; no asset was invented by spatial splitting.
- Mass uses rotated complete tensors and parallel-axis terms, deterministic part-ID reduction and finite/positive physical tensor validation before commit and f32 adapter conversion. Collider density remains zero. Derived mass/corner capabilities compile on revision before the preserved pre-sync force path; wheel removal suppresses that corner support/traction/braking. Other fleet cars explicitly retain the legacy reference.

## Verification and evidence

Raw logs and production runs are retained outside this worktree at `/Users/nicolas/Documents/github/car-game/artifacts/issue-12/`. Worktree `artifacts` points there, keeping default legacy script evidence isolated from other branches.

Initial headless handoff: formatting, warning-free all-target Clippy and unchanged complexity thresholds passed. Seven analytic/atomic assembly tests and all sixteen shared-production scenarios passed. The added scenario checks both domain and Rapier readback mass at completed tick boundaries: 1320→1295→1320 kg, ready→incomplete-inoperable→ready, unchanged identity and an actionable stale revision rejection. The complete final all-target ladder passed 66 tests: 18 unit (including the two location privacy controls), 7 assembly, 21 recorder health, 4 shutdown, and 16 production scenarios.

Reproduction uses `CARGO_TARGET_DIR=/Users/nicolas/Documents/github/car-game/target` and `CARGO_INCREMENTAL=0` for routine headless checks; exact recorded release builds use a checkout-local target (shared-target provenance is deliberately rejected by #88).

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
scripts/check_codelimit.sh HEAD
cargo test --all-targets
python3 -m compileall -q tools
python3 tools/check_assets.py
# production command scenario
cargo run -- --render-disabled --scenario examples/scenarios/assembly-wheel.json --artifact-dir <unique-directory>
```

Two independent review findings have been addressed: explicit visual/collision manifest bindings and non-finite/overflow aggregate rejection. Standards review's hardcoded corner IDs moved out of the solver into compiled domain capabilities; baseline mass/inertia literals now share the vehicle baseline contract.

## Completed verification

All results below are real executions on macOS/Apple M5 Pro with Metal; audio was muted. No complexity exceptions or global thresholds were added.

| Check | Result / retained evidence |
| --- | --- |
| `cargo fmt --check`, all-target Clippy, required code-limit `HEAD` and true branch-base `bdd2bee` | Pass; `logs/serialized-final/clippy.log`, `logs/serialized-final/codelimit-base.log`, `logs/serialized-final/codelimit-head.log` |
| `cargo test --all-targets` after the final code change | 66/66 authoritative serialized rerun; `logs/serialized-final/tests.log` |
| Python syntax, asset/source integrity | Pass; `logs/assets.log`; original 350Z and all fleet source geometry/images unchanged |
| Existing portable evidence/privacy controls | 17/17 Python tests; `logs/provenance-privacy-regressions.log`; two new Rust controls preserve exact typed locations and reject host/unknown locations |
| Full `scripts/verify.sh` | Script exited 0 on mechanical commit `ed31f39`; its shared-target fast-test phase is superseded by the serialized rerun below; 13/13 driving scripts, 52 validated nonblank 1920×1080 captures, 157,506 events; `logs/full-offline.log` |
| Explicit fleet/run/provenance validation | Pass; `logs/fleet-explicit.log`, `logs/fleet-run-explicit.json`, `logs/fleet-provenance.json`; healthy finalized recorder, complete inventory/build/runtime material |
| Preserved main versus assembly 350Z | 601 matching post-Writeback completed ticks (readiness-relative 120–720); maximum position and velocity error, longitudinal response error and straight endpoint error all exactly zero on this machine; `preservation-comparison.json`, reproducible `compare_reference.py` |
| Corrected production assembly scenario | 4/4 assertions; 1320→1295→1320 kg including Rapier readback, revision 0→1→2, stale command rejected, nonblank 1920×1080 capture, healthy finalized recorder; complete immutable inventory/build/runtime material |
| Actual native window/control inspection | Standard bundle of the same exact recorded executable; CUA inspected visible car/city/HUD, held W 1500 ms, observed 24 km/h, reset with R, paused with Escape, quit with Q; exit 0, healthy finalized recorder and complete build/runtime material |

Exact artifact identifiers under the raw evidence root:

- Full fleet: `runs/20261023T011838842667Z-ed31f39-fleet-93610-1-1792718318866918000`; exact executable SHA-256 `95a4a0c28673ffbbeaabde74229c3083cf31fc001f5b8ac52dbc7250d648862d`.
- Corrected assembly: `assembly-capture-complete/20261023T012558353969Z-82ada61-scenario-5879-1-1792718758373968000`; final code executable SHA-256 `729140e094d6e38c89ed92f2459b512900cc3c24a34eed3b94ee16a5ee98db35`.
- Native: `native-bundle-observation/20261023T01302656349Z-82ada61-interactive-13953-1-1792719026584464000`; `native-visible.png`, `native-driving-visible.png`, `native-paused-visible.png`; checksum/physical-command summary `native-observation-summary.json`.
- Native window screenshots include window chrome around the 1920×1080 viewport. They are separate from the required fleet PNGs. Both the issue owner and coordinator inspected meaningful visible native geometry, HUD and pause state. Native recording measured a peak 32.009 km/h, one reset, one pause and one successful quit; the saved driving frame displays 24 km/h.

The exact-build packages include immutable source/toolchain/dependency inputs. Routine checks shared only derived debug objects. Checkout-local release cache was APFS cloned, and the ordinary exact base executable received a copy of its matching recorded sidecar (identical executable hash). No shared executable was represented as having independent source provenance. The bundle was built using existing `tools/bundle_macos.sh release` and retained the same final executable bytes.

## Preserved findings and boundaries

- First exact assembly run passed behavior/capture but truthfully marked runtime material incomplete: the generic evidence sanitizer redacted the domain field named `location`. Followup `82ada61` permits only canonical typed vehicle/inventory locations whose scalar IDs/slot satisfy the portable-token policy; host paths, extra fields and unrelated location objects remain redacted. The first run remains at `assembly-capture/20261023T011725519312Z-ed31f39-scenario-90666-1-1792718245545081000`; final corrected run has zero omissions. Vehicle mechanics did not change after the full fleet/reference checks.
- Raw executable native `--smoke` was attempted and correctly failed blank surface screenshots; unchanged main failed the same check. One transient `caffeinate -u -t 300` display activity assertion retry also failed a later driving capture after three meaningful native captures. These runs remain in `native-smoke`, `native-reference`, `native-smoke-active-display`; they are never counted as passing smoke runs. OS evidence says the console was unlocked/on-console. The cause of native built-in screenshot readback intermittency is unresolved; it is distinct from the independently inspected visible native window and from the passing offscreen Metal captures. No rendering code or capture threshold was changed to conceal it. The documented applicable live window/startup requirement was supplemented with actual visible native keyboard/reset/pause/quit validation; the complete `verify_live.sh` remains an integration-owner check at #19.
- Full fleet regression validation and the 601-tick matching comparison do not ratify universal handling budgets or CPU p99 targets. No new performance/fidelity/audio quality claim is made.
- Jev judgment/browser tools are unavailable in this session; no Jev grade is claimed. Website interactions were unnecessary. CUA was used only for native app inspection. Engineering checks and independent review are preserved separately from the later #19 phase acceptance decision.
- P2 still owns authored fastener/picking/access and workshop mesh removal/reinstallation. The P1 manifest represents existing major wheel/tire assemblies, not complete engine internals. #13 snapshots, #14 controls and #15 query adapter consume this graph in stacked worktrees; their phase acceptance is not claimed here. Stable handoff `ed31f39`, evidence followup `82ada61`; no merge or issue close was performed.

## Serialized continuation and pre-existing CI failure

Authoritative local source check: `7ba9abe1eff51016ec858b1dbe9190b051b3e8e7`, 2026-10-23 01:50:27–01:52:05 UTC. One outer process held the universal `artifacts/p1-coordination/cargo_serial.py` lease across format, all-target Clippy, both HEAD and true-base `bdd2bee` complexity checks and all-target tests, until every test child exited. `CARGO_INCREMENTAL=0` and `RUST_TEST_THREADS=1`. All checks passed; 18+7+21+4+16 = **66 Rust tests**. Retained outer log `logs/serialized-fast-ladder.log`, script `serialized-fast-ladder.sh`, command logs `logs/serialized-final/`.

This rerun supersedes the earlier shared-target Rust-test claims: another worktree could overwrite the shared CLI/test executable while earlier tests were running. Those older logs remain preserved as non-authoritative for source-specific Rust integration verification. Exact checkout-local recorded builds, fleet captures/provenance, handling comparison and native CUA evidence remain valid independently. No new GPU run or runtime change was necessary.

CI is **not green**. PR #95 run [39884795037](https://github.com/nicolas-found42/car-game/actions/runs/39884795037) failed all four recorder-shutdown tests at existing 8-second readiness / 10-second shutdown bounds. Unchanged main `bdd2bee` run [39880399889](https://github.com/nicolas-found42/car-game/actions/runs/39880399889) and its predecessor run [39879824498](https://github.com/nicolas-found42/car-game/actions/runs/39879824498) failed the same four tests, so this symptom predates the assembly implementation. The causal explanation remains unproven; candidate CI-profile/hash/concurrency/readiness hypotheses are explicitly distinguished from measurements.

Raw retained comparison: `ci-diagnosis.json`; logs `logs/ci-main-bdd2bee-failed.log`, `logs/ci-pr94-predecessor-failed.log`, downloaded CI artifact `ci-failure-39884795037/`. The upload contains only formatter/test logs; child `/var/folders` evidence was not uploaded and its contents are not inferred. [Issue #96](https://github.com/nicolas-found42/car-game/issues/96) owns diagnosis, the narrow repair and real green CI proof in a separate subagent/worktree/PR. No deadline, CI profile, harness or failure threshold was changed in #12. #19 remains blocked on that verification repair; this is not a CI waiver.

Independent Standards review of #14 at `ed31f39...bc9ef37` was also completed during continuation, without editing #14: no hard documented-standard violation, two possible duplication smells (player reset enqueue policy and wheel pose math). Receipt: `artifacts/p1-coordination/issue-14-standards-review-issue12.md`; its owner/coordinator handles any fixes.
