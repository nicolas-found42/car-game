# Issue #13 — identity-preserving assembly snapshots

Implementation is complete; final release/Metal evidence and independent review are pending. Native dependency #12 remains open pending human merge. No merge or issue closure is authorized. This branch is stacked on #12's graph implementation and canonical-location provenance fix.

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
- Eleven invalid restore cases compare the complete ledger and live runtime mappings, preparation revision, wheel mask and generation before/after, and require eleven actual production restore rejection outcomes.
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

The ordered ladder log is `artifacts/issue-13/verification-ladder.log`. Routine checks use the shared debug cache. Recorded release builds use the checkout-local release target and `python3 tools/record_build.py --profile release`; this prevents another worktree from overwriting the bound executable. GPU use is serialized by the parent coordinator. Exact release run IDs, hashes, validation and review results will be appended after completion.

Jev judgment tools are unavailable in this session; the independent source review and real execution evidence remain required.

## Completed headless and source review gate

At runtime-source commit `f4c98dd`, the ordered format → warning-free all-target Clippy → complexity → all-target test ladder passed all 69 Rust tests (18 unit, 7 graph/numerical, 3 snapshot, 21 recorder-health, 4 shutdown and 16 existing production-scenario tests). True-base complexity against `82ada61` also passed. Test-only refinement `177fbc2` records measured Rapier masses and the actual descendant record; its three focused snapshot tests, formatting, Clippy and true-base complexity passed. Python tools compile, and actual original-source geometry/PNG integrity checks pass for all 13 cars; the managed checkout links the existing `carmodels` sibling data for those checks.

The checkout-local recorded release binary SHA-256 is `fc51b3dc3f88c946098774ab01baf222e41644b1740a2eba54f4fe5f633ecd7e`. Its compiler-embedded source fingerprint is `b9c12ef8024665539d94dcb30a2e8b8d9f735e4e241bbf7e602559bf6d0d3e11`, with no build omissions and complete rebuild inputs. Permanent-pilot CLI run `recorded-headless/20261023T013052729843Z-177fbc2-scenario-14229-1-1792719052755526000` passed all four assertions, wrote the standalone normalized DTO and passed recording-health verification. Provenance confirms `source_build_match`, inventory integrity and complete runtime material. Cross-platform deterministic replay remains explicitly unproven; it is not an assembly acceptance claim.

Independent Standards review and independent Spec review of `82ada61...f4c98dd` each reported zero blockers. The parent coordinator owns these reviews and downstream #15 integration. [The checked-in headless receipt](issue-13-receipt.json) preserves exact log, scenario, fixture-source and provenance hashes. Raw final nested-scenario/fresh-application/negative runs are retained under `production-snapshots/`, alongside earlier development failures. GPU evidence remains pending the coordinator's lock release; no PR has been opened yet.
