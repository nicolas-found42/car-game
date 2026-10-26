# Issue 15: synchronized physics adapter

Current acceptance: [recorded and native proof on October 26, 2026](issues-14-15-acceptance.md).
The historical preflight and checkpoints below retain their original source and
limitations; their outstanding recorded/native gates are superseded by that acceptance.

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

### Focused verification handoff

Implementation head `0f47939` passed the serialized focused ladder: format,
all-target warning-free Clippy, complexity against the real #13 base `177fbc2`, and
all seven new production tests. The scenario executed and passed all eight declared
assertions. Evidence: primary checkout `artifacts/issue-15/fast-check-4.log`.
Previous attempts remain in `fast-check*.log`; no failed assertion was skipped.

The focused tests caught pending Rapier derived mass: changing additional mass during
SyncBackend does not update raw mass/COM until the normal step. The adapter now calls
the pinned public `recompute_mass_properties_from_colliders` on each vehicle body
before mechanics, without consuming modified queues. This makes first-tick 1,295-kg
wheel-removed mass/COM current for force lever arms and mass-dependent braking/load
limits; intact 1,320-kg tuning remains unchanged. The airborne test verifies the
actual one-step gravity velocity and absence of stale drive force analytically.
Inactive or missing physics contexts produce zero executed steps and an explicit
unavailable outcome rather than a fabricated completed integration claim.

The test clock uses the repository's exact `STEP as f64` convention; nominal 1/120
rounded to integer nanoseconds was slightly shorter than the existing f32 fixed
period. First-tick support fixtures use heights within their measured ray range,
and elevated platforms stay inside road corridors rather than inside existing city
buildings. Reparent and old-reference tests explicitly demonstrate different query
readiness on the edit boundary, followed by intact production collision response.

An isolated APFS release dependency cache is prepared for exact recorded builds.
Worktree `artifacts/runs` and `artifacts/logs` point into the primary durable
`artifacts/issue-15/` directory. Asset-source inspection uses the existing carmodels
repository through the normal sibling symlink. Full suite, external review and real
GPU acceptance remain pending; the focused handoff is not the P1 acceptance gate.

### Exact CLI/schema checkpoint

Consumed verified #13 restore guards (`4641ead`/`6ef2ce5`) as `8f7563b`/`8141e45`.
Serialized format, all-target Clippy, real-base complexity and all seven focused
production tests passed again; their actual runs are preserved in primary
`artifacts/issue-15/production-tests/`. Recorded release build `8141e45` is complete
with no omissions; SHA-256 is
`be2731d7996380de87f9453e7d7686c980fb664e13610435b59b28e248c2dfa0`.
Actual headless CLI passed all eight scenario assertions and portable integrity
verification (134 entries, runtime material/rebuild complete). It preserved 103
consecutive records including reset tick 9, observed backend forces equal composed
forces at every tick, and zero recorder drops/errors through durable tick 103.

See `docs/verification/issue-15-schema.md` for the downstream schema/evidence handoff.
Root owns coordination of the next active slot; this issue awaits #14 follow-up,
full combined verification, independent reviews, final exact rebind/GPU/fleet checks
and PR creation. No GPU acceptance, issue completion, merge or closure is claimed.

## Rebuild on main after #13 and #14 — source `cd46952`

Rebuilt from `origin/main` `34e8baf` (#14 merged) by cherry-picking only this
issue's commits: `c7a7468 f03ed6b 86493c6 8769160 0f47939 c885c1f 878f7a9 de1e9cb`.
Skipped `55754ea` and `06a363e` (pre-review copies of #14, superseded by the
merged `8f7df51` helpers) and all #13 commits already on main. Two conflicts
in `f03ed6b`: `AssertionKind` adds `PhysicsState` beside `SnapshotMatches` and
`VehicleState`; the reserved-actor regression keeps main's shared
`reserved-player-actor.json` fixture. A tree diff against the old branch
`de1e9cb` shows only main's changes (shared `queue_player_reset`, CI test
speedups, provenance hashing); no issue-15 code was lost.

`CAR_GAME_MUTE=1 bash scripts/verify.sh --base origin/main` passed every rung
with exit 0 as the only Cargo workload on the shared target: fmt, all-target
Clippy, complexity against `HEAD` and `origin/main`, all-target Rust tests
(**81 passed**, including
`inserted_removed_and_relocated_road_queries_and_forces_are_current_on_the_first_tick`,
`authored_query_force_and_reset_scenario_executes_all_eight_first_tick_assertions`,
`simultaneous_cars_use_independent_geometry_inputs_and_exclusive_controllers`
and `serialized_snapshot_restores_in_fresh_application_without_runtime_handles`),
Python syntax/controls, asset integrity, and the offscreen release fleet check
(13 car reports, run
`20261026T174930292793Z-cd46952-fleet-51534-1-1793036970328087000`, 161204
events, `RUN EVIDENCE PASS`). Log: primary `artifacts/issue-15/verify-cd46952.log`.

Not run in this pass: `--recorded` provenance build, native Metal runs of
`synchronized-physics-metal.json`, and `scripts/verify_live.sh`.
