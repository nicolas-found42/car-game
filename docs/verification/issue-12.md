# Issue #12 — major-part assembly

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

Initial headless handoff: formatting, warning-free all-target Clippy and unchanged complexity thresholds passed. Seven analytic/atomic assembly tests and all sixteen shared-production scenarios passed. The added scenario checks both domain and Rapier readback mass at completed tick boundaries: 1320→1295→1320 kg, ready→incomplete-inoperable→ready, unchanged identity and an actionable stale revision rejection. Additional permanent unit/recorder suites run through the all-target ladder; final results below will be updated after full offline/live checks.

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

## Remaining gate work

Run the full offline fleet chain, native/live applicable chain and exact-build scenario recording; retain summaries and any limitations here. Jev judgment/browser tools are unavailable in this session; no Jev gate or manual visual acceptance is claimed. Independent review and actual executable checks supply engineering evidence, and later #19 owns the phase acceptance judgment. No issue closing or merging is authorized.
