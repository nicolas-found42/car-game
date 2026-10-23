# Issue 14 — independent per-vehicle controls

## Durable status

Preflight only. No implementation or acceptance claim. Isolated managed worktree:
`/Users/nicolas/.codex/worktrees/issue-14-controls/car-game`, branch
`codex/issue-14-controls`, based on main. Native prerequisite #12 is still open
and under active implementation. Integrate its verified commit before coding.
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
