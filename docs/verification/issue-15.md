# Issue 15: synchronized physics adapter

Status: preflight/design only. Branch `codex/issue-15-physics`; isolated managed checkout
`/Users/nicolas/.codex/worktrees/issue-15-physics/car-game`; initial base `bdd2bee`.
Implementation waits for verified issue 13 and 14 branches integrated by the coordinator.
Native GitHub dependencies were read directly: #13 snapshots and #14 per-car controls.
Current #9 and #15 include the completed-step observability amendment.

## Pinned API findings

Source inspected in the local Cargo registry: `bevy_rapier3d-0.37.0`,
`rapier3d-0.36.1`.

* Rapier `SyncBackend` already chains transform propagation, removals, collider/body
  initialization and user changes. Its regular query pipeline borrows the production
  broad-phase BVH, refreshed by `step_simulation`, so a query view after sync alone
  still misses newly inserted and relocated colliders.
* `ColliderSet::take_modified` and `take_removed` explicitly forbid use alongside
  `PhysicsPipeline`; these queues belong to the production solver. Never drain them.
* `RigidBodySet::propagate_modified_body_positions_to_colliders(&self, &mut ColliderSet)`
  walks modified bodies without consuming the queue. Prefer an entirely read-only
  snapshot composed from `body.position * collider.position_wrt_parent` to avoid even
  changing production collider bookkeeping during query preparation.
* `ColliderSet::iter` and each collider's immutable shape/pose/parent give a safe
  current query snapshot after sync. An independent `BroadPhaseBvh` populated using
  `set_aabb` can index this snapshot without touching production broad-phase state.
  Snapshot handles are derived mappings, never persistent domain identity.
* `apply_rigid_body_user_changes` resets backend force/torque then adds changed ECS
  `ExternalForce`. Moving mechanics after sync therefore requires an explicit
  backend force bridge before StepSimulation; do not run the whole sync system twice.
  The bridge replaces force/torque once from the composed current ECS value. Next
  tick's sync may copy the prior value briefly, but it must always be replaced before
  integration, including reset/unavailable cases.
* Preserve `Time<Fixed>=1/120`, `TimestepMode::Fixed {dt:1/120, substeps:1}` and the
  single standard Rapier StepSimulation system. No extra integration/query step.

## Planned production ordering and ownership

Commit/commands → preparation/reset/relocation and applied deferred changes →
per-vehicle controls → Rapier SyncBackend (includes transform readiness) →
current read-only query snapshot → existing single mechanics solver → force bridge →
Rapier StepSimulation once → Writeback → completed observations → recorder.

Keep the original pre-sync path available until synchronized tests pass. Any legacy
sample retained is explicitly `pre_step`; completed snapshots use canonical #11 tick,
post-Writeback Transform and Velocity, #12/#13 stable vehicle/body/part identities,
assembly revision and #14 controller/geometry state. Reset is prepared before sync,
then receives a completed record at the normal boundary rather than disappearing via
an early `continue`. Each executed boundary yields a state or explicit reason.

Bounded four-wheel diagnostics retain origin/direction/range, hit/miss/rejection,
stable collider binding, world point/normal, suspension load, tire and antiroll
force/torque contributors, and final summed chassis force/torque. Include geometry
and assembly revisions. Capture phase duration measurements around sync, query,
mechanics, bridge, step, writeback and record with `Instant`; publish additively through
the existing recorder adapters and event envelope, with no second force solver.
Integrity validation, diagnostic checkpoint restore and timeline UI remain #86/#89/#91.

## Acceptance evidence to implement after dependencies

Use shared `build_app` and production commands, with focused numeric tests only for
mechanics/API invariants. At the very next executed fixed tick assert:

1. A newly spawned road supports raycasts; its removal yields miss/no support.
2. Relocated roots and collider reparenting query the current world geometry.
3. A new throttle/brake force changes the same step's velocity, then zero/reset
   removes the prior force; exactly one outer dt advances.
4. Two independently controlled geometries retain stable actor/contact identities.
5. Completed record pose/velocity equals actual post-Writeback world state; reset,
   relocation and consecutive ticks are present, with per-wheel provenance/timings.
6. Solver collision contacts still initialize/update after query publication, proving
   query refresh did not steal solver bookkeeping.

Run the required formatting, Clippy, codelimit, all-target tests, Python syntax,
assets and full offline verification; then real native macOS/Metal verification.
Preserve exact build manifests, logs, scenarios, captures and comparison evidence.
Keep original fleet/cycling/reset/replay captures and baseline comparison provenance.
Create an attached reviewable PR only after checks, fix review findings, stop before
merge, and leave issues open until the coordinator/user accepts the dependency gate.

## Open integration seams

Await #12 stable ID/revision mapping, #13 versioned domain DTO interface and #14
configuration/controller/reset migration before choosing final component signatures.
Measure independent query index rebuild cost on the declared Mac workload; optimize
only if observed overhead warrants caching, preserving correct insert/remove/reparent.
No product decision or missing asset has been identified during preflight.

## Implementation checkpoint (2026-10-22)

Verified #13 code `177fbc2` was integrated before #14 implementation `fd5a7b1` /
`902adb5`; controller alias review fix `bc9ef37` is included. Existing combined
71-test suite passed locally before physics changes. A later shared-target run was
invalidated when another checkout overwrote test executables during execution;
acceptance checks now use an isolated APFS clone and the coordinator's whole-command
Cargo lease. A reliable isolated run of the original 71 tests passed after the
synchronized adapter landed. New focused acceptance tests are in progress.

`physics` owns current collider queries, force bridge, parent mapping readiness,
control/reset preparation, and completed-step recording. The solver is still the
single `vehicle::simulate`; `QueryMode::LegacyPreSync` selects its baseline position
in the schedule, and the default selects synchronized mechanics after backend sync.
The query database clones only immutable collider shapes/properties, computes
current body × local poses, resets links in its own ColliderSet and builds its own
BVH. Production collider queues and broad phase remain untouched.

Pinned-version integration also lacks same-context collider reparenting on ChildOf
changes. A derived nearest-rigidbody parent cache invalidates changed collider
handles before normal backend sync, allowing Rapier's own removal/initialization and
mass events to run. Stable collider/entity/domain identities survive that remapping.

Per-car assembly preparation matches `VehicleControl.id`, the actual bound body and
its configured car; masks/revisions cannot leak from the player to a secondary.
Secondary fleet fixtures report `assembly_binding=no_assembly_binding` and null
assembly revision/part binding explicitly. #16 must replace those fixtures with
actual per-vehicle graphs before the final P1 gate. No multi-graph completion is
claimed here.

Completed `PhysicsSample` preserves the existing top-level driving fields and adds
#11's event envelope, stable vehicle/body/root-part identifiers, post-writeback
position/velocity, one integration-step count, phase durations, and bounded
`evidence`: effective input/reset, query tick/geometry epoch, assembly revision and
runtime generation, explicitly named pre-step position/velocity, four wheel
contacts/IDs/geometry and suspension/tire/antiroll force+torque, chassis and final
composed force. `backend_applied` reads Rapier's actual user force/torque after the
step, so evidence can distinguish computed from applied forces. Reset ticks publish
zero composed force with `reset_no_mechanics` wheel reasons and normal post-reset
integration. Prepared assembly revision/generation changes publish the existing #13
`AssemblySnapshot` DTO in `physics.assembly_checkpoint`; no parallel save schema.

`PhysicsEdit` commands exercise road insertion/removal, root relocation and collider
reparenting through fixed production dispatch. The scenario framework adds
`PhysicsState`, checking same-tick query/completed boundaries, one step, contacts,
force application, grounded count, outcome and motion. Both
`examples/scenarios/synchronized-physics.json` and its required-capture Metal variant
carry eight assertions, including the first boundary after each edit and reset.

Outstanding: focused tests, full serialized ladder, independent review/fixes,
recorded exact build, offline fleet chain, real native Metal validation, retained
provenance/comparison evidence and attached PR. Nothing is merged or closed.
