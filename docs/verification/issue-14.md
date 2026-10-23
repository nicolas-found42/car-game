# Issue 14 — independent per-vehicle controls

## Durable status

Implementation passes initial headless verification; GPU evidence/review/PR remain pending. Isolated managed worktree:
`/Users/nicolas/.codex/worktrees/issue-14-controls/car-game`, branch
`codex/issue-14-controls`, based on #12 handoff `ed31f39`, with provenance fix
`82ada61` cherry-picked as `21f604e`. Native prerequisite #12 remains open
pending human merge; stacked integration does not close the live native gate.
Do not merge or close issues during this execution.

## Specification and contracts

Read current #9 and #14 on 2026-10-22 (America/New_York). #14 owns F02/F20:
two cars, independent held controls, exclusive controller per car/tick,
per-car geometry, distinct one-shot commands, one existing dynamics solver,
and preserved keyboard/gamepad/replay/reset/pause/fleet behavior. Native
`issues/14/dependencies/blocked_by` reports #12 as the prerequisite.

## Integration plan

- Reuse #12 typed vehicle identity and runtime mapping API after its handoff.
- Attach configuration and actuator/controller ownership to each vehicle.
- Keep legacy selected-player resources only as compatibility ingress while
  callers migrate; never feed one shared input/config to every vehicle.
- Route targeted production commands through the existing CommandBus;
  retain existing unqualified player commands and additive telemetry fields.
- Controller assignment clears stale held state; reset is consumed once by
  its target, retains command correlation, and cannot reset another car.
- Extend the existing scenario runner with an explicit two-car fixture and
  target observations; do not create a second simulation/test application.
- Existing physics raycast/force solver remains the sole dynamics path.

## Reconciled code findings

`VehiclePlugin` spawns one chassis. `simulate` currently reads a global
`CarSpec`, `DriveInput` and `ControllerAssignment`, then loops all chassis.
Runtime selection/reset and scenario readiness use single-target queries.
Presentation is selected-player oriented. `PhysicsSample` already carries
`actor_id`; input and completion events currently use `player-vehicle`.
The migration must preserve selected-player rendering/ingress while adding
explicit target identity for secondary controls/evidence.

## Pending verification

No builds/GPU runs yet; prerequisite is not ready. After implementation run
format, all-target warning-free Clippy, code-limit ratchet, all-target tests,
Python/assets checks, full offline fleet verification and applicable Metal
validation. Preserve command/physics/scenario evidence under primary checkout
`artifacts/issue-14/`; use shared primary `CARGO_TARGET_DIR` to avoid duplicate
build storage. Include physical opposite-command and distinct-geometry results,
exclusive ownership rejection, one-shot reset isolation, player/replay/gamepad
regressions, and reviewed PR links here before handoff.

## Implementation handoff

`VehicleControl` is the per-chassis component carrying stable `VehicleId`, owned
`CarSpec`, selected `ControllerOwner`, held `DriveInput`, and optional correlated
one-shot reset. `PlayerVehicle` explicitly scopes legacy resource ingress and
selected camera/HUD/audio; secondary visuals/wheels follow their own chassis.
`ProductionCommand::Vehicle` routes normalized drive/reset/assignment to a target.
`SpawnVehicle` creates a fleet fixture through the same production chassis bundle
and solver. Ownership transfer clears stale held controls; wrong-source commands
are rejected. Player replay does not lock a separately owned secondary vehicle.

At scenario tick 240, the baseline 350Z drives forward at 11.33 m/s and an NSX
reverses at -4.54 m/s, with distinct wheelbase/radii and four supported corners.
The scenario checks player reset isolation and exclusive secondary ownership.
A second production regression proves a live-owned secondary resets exactly once
while player replay retains its ownership. Original gamepad/keyboard/replay/fleet
and render-cadence tests passed in the initial all-target run (66 tests).

### Downstream integration boundary

This issue owns independent control/configuration, not multi-assembly graphs.
Secondary chassis currently uses an explicitly intact fleet fixture; no claim of
independent graph authority is made. Assembly preparation and the assembly wheel
mask affect only `PlayerVehicle`, preventing global pilot mass/support from leaking
onto other cars. #15 must attach/derive per-chassis assembly capabilities using
`VehicleControl.id`; #16 must register each simultaneous vehicle against its own
graph and derive wheel masks/mass/config without aliasing the selected player's
graph. There remains exactly one raycast/tire/drive solver.

### Verification progress

Initial formatting, Clippy all-targets, complexity against preflight HEAD, all-target
Rust tests, Python syntax, and source-byte asset integrity passed. Worktree's sibling
`carmodels` symlink resolves the existing source asset repository; an initial missing
relative source-path asset check was repaired and rerun successfully. No assets are
missing. GPU/live queue remains owned by #12 then #13; this issue has not claimed
render acceptance. Additional `independent-vehicles-metal.json` requests a real
capture while both independently controlled cars remain in camera range.
