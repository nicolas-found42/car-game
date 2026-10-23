# Issue #13 — identity-preserving assembly snapshots

Status: preflight complete; waiting for verified #12 graph prerequisite. Implementation has not started. Native GitHub dependency inspected: #13 blocked by open #12. No merge or issue closure is authorized in this workstream.

## Scope and implementation plan

Current requirements are #9 F01/F04/F10 and #13. Normalize versioned vehicle/part/attachment/settings/state DTOs. Store exactly one location for every independent root; descendants inherit root location and owner. DTO authority excludes Bevy Entity IDs, GPU/physics/asset handles and derived mass/query caches. Snapshot validation builds an entirely staged candidate graph, rejects unsupported versions, duplicate IDs/parents/locations, dangling references and all #12 graph invalidity before changing the active domain.

Use #12's production graph and transaction rules, adding snapshot/restore commands to the same command bus. Runtime mappings must respawn into fresh ECS entities while preserving opaque domain IDs. A production scenario removes a subtree to inventory, snapshots, restores, verifies identity/state/location preservation and new ECS mappings, then reattaches the same identities. Focused rejection tests compare the complete pre/post active graph and runtime mapping. This is assembly persistence only; career durable save/recovery belongs to P5.

## Integration and conflicts

Managed worktree: `/Users/nicolas/.codex/worktrees/issue-13-snapshots/car-game`, branch `codex/issue-13-snapshots`. Expected new owner module: `src/assembly/snapshot.rs`; minimal changes to production dispatch and scenario assertions. #14 controls agent notified of shared `runtime.rs`/`lib.rs` seams. Must rebase on the verified #12 commit before implementation. Parent coordinator owns dependency scheduling and serialized GPU access.

## Verification and evidence

Run in order: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check_codelimit.sh HEAD`, `cargo test --all-targets`, plus applicable offline/live chains if physics/simulation/fleet changes require them. Shared build cache: `CARGO_TARGET_DIR=/Users/nicolas/Documents/github/car-game/target`. Raw evidence destination: primary repository `artifacts/issue-13/`; preserve scenario definition, summary, input/physics/domain events and build/asset/catalog provenance. Snapshot scenario does not require visual-only features or fabricated assets; any required capture must be real and nonblank. Jev judgment tools unavailable per coordinator, so evidence-backed review remains mandatory.

No acceptance claim or pass evidence exists yet.
