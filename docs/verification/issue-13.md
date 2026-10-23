# Issue #13 — identity-preserving assembly snapshots

Implementation, applicable verification and source review are complete. Reviewable [PR #97](https://github.com/nicolas-found42/car-game/pull/97) is open against #12's [PR #95](https://github.com/nicolas-found42/car-game/pull/95). Native dependency #12 remains open pending human merge. No merge or issue closure is authorized. This branch is stacked on #12's graph implementation and canonical-location provenance fix.

## Production contract

`SnapshotAssembly { key }` runs at the fixed command boundary, serializes version 1 UTF-8 JSON, retains a named snapshot and emits `assembly.snapshot`. Standalone evidence lives under the run's `assembly-snapshots/<SHA256(key)>.json`; hashing the key prevents path interpretation. This is an assembly DTO/evidence facility, not P5's durable career storage/recovery system.

`RestoreAssembly { expected_revision, source }` accepts `{"kind":"saved","key":"removed"}` or `{"kind":"inline","snapshot":<DTO>}`. It stages the entire candidate using the active catalog and #12's graph validation. Only a valid candidate replaces the ledger. The commit revision advances above both the active and saved revision, so restore cannot reopen a stale-revision window. Previously committed transaction identities and contents survive the round trip.

The normalized DTO stores opaque vehicle/owner/part/definition IDs, part condition and secured state, attachment rows, exclusive independent-root location rows, prerequisites, functional networks, transaction rows, snapshot tick and catalog hash. Descendants carry no duplicate owner/location. Arrays retain duplicate evidence until validation; they are not deserialized into maps that silently overwrite duplicate keys. Unknown assembly versions, catalog mismatches, dangling references, duplicate IDs/parents/root rows/occupied locations and graph invalidity reject before live state changes. Runtime handles, definitions, derived mass/COM/inertia, controller state and caches are not snapshot authority.

`AssemblyRuntime.parts` maps durable PartId values to disposable `PartIdentity` ECS anchors. Production preparation validates these mappings against the ledger before installing graph-backed physical mass and wheel capabilities. Valid restore allocates all fresh anchors before despawning old anchors and invalidates prepared state. Same-application restore preserves the existing chassis and controls; a second freshly built production application proves that the DTO restores correctly with distinct chassis and part ECS allocations. Future visual/physics owners can consume the same part map.

## Actual behavioral evidence

`tests/assembly_snapshots.rs` exercises the real `build_app`, production command bus, graph transactions, scenario plugin and Rapier readback:

- A non-stock authored fixture partitions existing front-left wheel/tire visual node bindings into a centered 20 kg wheel and 5 kg tire child, with respective diagonal inertias 1.6 and 0.4 kg m². It preserves the permanent pilot's intact 1320 kg aggregate. The default pilot and source assets/receipts are unchanged. The child is deliberately worn (0.37) and unsecured; restore/reattach preserves both values and the unsafe readiness result.
- The existing scenario framework removes the wheel+tire subtree into inventory, snapshots, restores six part identities into fresh ECS anchors, and reattaches the same subtree. All four declared assertions must execute. The removed child inherits the same inventory root and owner.
- A separate production application reserves 1024 unrelated ECS entities before startup, ensuring distinct physical/part allocations. It imports the serialized removed assembly, reads back actual Rapier mass 1295 kg and disabled front-left support, reattaches the same IDs, then reads back 1320 kg and all four enabled corners. `fresh-runtime-proof.json` retains the actual observed mappings and inspection.
- Nineteen invalid restore cases compare the complete ledger and live runtime mappings, preparation revision, wheel mask and generation before/after, and require nineteen actual production restore rejection outcomes.
- `examples/scenarios/assembly-snapshot.json` independently exercises the permanent five-part pilot via the ordinary CLI with no custom runner.

## Reproduction and retained evidence

Managed worktree: `/Users/nicolas/.codex/worktrees/issue-13-snapshots/car-game`; branch: `codex/issue-13-snapshots`. Primary raw evidence: `/Users/nicolas/Documents/github/car-game/artifacts/issue-13/`. Failed development attempts remain identifiable alongside final successful runs; they are not acceptance evidence.

```sh
export CARGO_TARGET_DIR=/Users/nicolas/Documents/github/car-game/target
cargo fmt --check
cargo clippy --all-targets -- -D warnings
scripts/check_codelimit.sh HEAD
ASSEMBLY_SNAPSHOT_EVIDENCE=/Users/nicolas/Documents/github/car-game/artifacts/issue-13/production-snapshots cargo test --all-targets
```

The ordered ladder log is `artifacts/issue-13/verification-ladder.log`. Routine checks use the shared debug cache. Recorded release builds use the checkout-local release target and `python3 tools/record_build.py --profile release`; this prevents another worktree from overwriting the bound executable. GPU use is serialized by the parent coordinator. Exact release run IDs, hashes, validation and review results follow below.

Jev judgment tools are unavailable in this session; the independent source review and real execution evidence remain required.

## Completed headless and source review gate

At runtime-source commit `f4c98dd`, the ordered format → warning-free all-target Clippy → complexity → all-target test ladder passed all 69 Rust tests (18 unit, 7 graph/numerical, 3 snapshot, 21 recorder-health, 4 shutdown and 16 existing production-scenario tests). True-base complexity against `82ada61` also passed. Test-only refinement `177fbc2` records measured Rapier masses and the actual descendant record; its three focused snapshot tests, formatting, Clippy and true-base complexity passed. Python tools compile, and actual original-source geometry/PNG integrity checks pass for all 13 cars; the managed checkout links the existing `carmodels` sibling data for those checks.

The checkout-local recorded release binary SHA-256 is `fc51b3dc3f88c946098774ab01baf222e41644b1740a2eba54f4fe5f633ecd7e`. Its compiler-embedded source fingerprint is `b9c12ef8024665539d94dcb30a2e8b8d9f735e4e241bbf7e602559bf6d0d3e11`, with no build omissions and complete rebuild inputs. Permanent-pilot CLI run `recorded-headless/20261023T013052729843Z-177fbc2-scenario-14229-1-1792719052755526000` passed all four assertions, wrote the standalone normalized DTO and passed recording-health verification. Provenance confirms `source_build_match`, inventory integrity and complete runtime material. Cross-platform deterministic replay remains explicitly unproven; it is not an assembly acceptance claim.

Independent Standards review and independent Spec review of `82ada61...f4c98dd` each reported zero blockers. The parent coordinator owns these reviews and downstream #15 integration. [The checked-in headless receipt](issue-13-receipt.json) preserves exact log, scenario, fixture-source and provenance hashes. Raw final nested-scenario/fresh-application/negative runs are retained under `production-snapshots/`, alongside earlier development failures. This was the initial evidence boundary; final evidence and the transaction-history correction follow below.


## Final acceptance and correction boundary

Final runtime source is `6ef2ce5` (history guard `4641ead`, test-complexity cleanup `6ef2ce5`). Self-review found that imported committed transactions could otherwise carry dangling historical destinations even while the current graph was valid. Candidate graph validation now precedes history lookups, and each historical transaction validates its part/parent/mount/interface, P1 owner/vehicle/location fields and saved revision. Historical occupancy is deliberately not compared against the present graph. A combined unknown-definition/history case also rejects without a panic. The parent reviewed this correction and reported no additional findings.

The final isolated whole-command Cargo lease ran formatting, warning-free all-target Clippy, true-base complexity against `82ada61`, and all 69 Rust tests successfully. `leased-final-verification.log` is authoritative; `leased-production-snapshots/*6ef2ce5*` contains final nested-subtree, fresh-application and nineteen rejected-restore run evidence. The declared workload uses `CARGO_INCREMENTAL=0` and `RUST_TEST_THREADS=1`, with no concurrent Cargo processes outside the coordinator's POSIX lease. Earlier shared-cache/output collisions and an incident-export deadline failure remain preserved and labeled as non-acceptance attempts in the receipt. No production deadline or assertion was relaxed.

The final recorded binary SHA-256 is `7689fc0669030060937e93bc44ac27aa4fefd1651297559b080530d6d7ceb17a`, with complete rebuild inputs and no omissions:

| Final production run | Result |
| --- | --- |
| `final-positive/20261023T020034493459Z-6ef2ce5-scenario-58031-1-1792720834508981000` | Snapshot/restore/reattach: 4/4 assertions |
| `final-negative/20261023T020034491601Z-6ef2ce5-scenario-58032-1-1792720834508982000` | Forged dangling historical parent: rejected; unchanged revision 0, 1320 kg and original domain snapshot; 3/3 assertions |
| `final-metal/20261023T020042360132Z-6ef2ce5-scenario-58226-1-1792720842377039000` | 4/4 assertions plus required nonblank 1920×1080 capture, visually inspected as the 350Z, city and HUD |

All three final runs pass recorder health and report `source_build_match`, inventory integrity, complete rebuild inputs and complete runtime material. The final Metal image SHA-256 is `368a479fb60d5610c6a3845531f7aaeab9c07b9c07265104817fc54a11f0f59d`.

The physical regression gate ran the actual complete `scripts/verify.sh` under the whole-command lease before the import-only guard. Run `worktree-artifacts/runs/20261023T014533681601Z-5161a3b-fleet-42705-1-1792719933701072000` passed all 13 driving scripts, wheel animations, 52 nonblank captured views, 160,965 recorded events, original-source asset checks and both fleet/run verification tools. Its executable is the exact earlier `fc51b3…` binary with matching recorded sidecar and complete source/build/runtime material. The import guard changes no physics, rendering, roster or catalog code; the coordinator approved retaining this exact full-fleet regression and rerunning the final-source production snapshot paths/capture. The environment router in `exact-offline-environment.sh` transparently routes ordinary debug checks to the shared cache and `cargo run --release` to the checkout-local identical bound executable with its compiler fingerprint.

Native regression run `native-startup/20261023T014046124853Z-5161a3b-interactive-34902-1-1792719646159573000` retained an actual focused 1920×1080 Metal window for 30 seconds, then handled SIGTERM and finalized with exit 0 and healthy recording. Window resize/focus/environment records establish creation; the configured log filter omitted the optional info log string. This is a startup/shutdown check, with no native surface-image assertion.

[The final checked-in receipt](issue-13-receipt.json) preserves source/binary boundaries, exact run IDs, workload, nineteen rejection reasons, provenance completeness and SHA-256 hashes. The raw copy is `artifacts/issue-13/final-receipt.json` in the primary checkout. All GPU and Cargo leases are released. Stop before merge or issue closure; downstream integration consumes the same API plus the history-guard commits.
