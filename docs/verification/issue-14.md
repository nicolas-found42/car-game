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

Coordinator evidence review found targeted-player normalization could obscure the
original intent in effect records. Fixed: queued normalized commands retain ingress
intent, command outcomes expose original `command` and explicit `normalized_command`,
and reset effects record stable `vehicle_id` plus `requested_command`. A regression
joins targeted reset evidence back to its original ingress command ID/target.

`input.jsonl` deliberately remains the selected-player legacy replay stream; secondary
input consumption is preserved with actor identity in `controls.effective` and each
secondary `physics.jsonl` sample's `input`. Consumers must use those streams for
multi-vehicle evidence; treating the legacy replay stream as all actors is invalid.

## Exact verification checkpoint — source `902adb5`

- Format, warning-free all-target Clippy, complexity against both `ed31f39` and
  `HEAD`, all-target Rust tests (**68 passed**), Python syntax and all 13 source-byte
  asset checks passed. Exact logs reside in primary `artifacts/issue-14/`, named
  `*-902adb5.log`; hashes are in `evidence-checksums.json`.
- Checkout-local recorded release build succeeded with `rebuild_complete=true`,
  no omissions, executable SHA-256
  `e561e2f620e2dd97c69ebbfa8e1ac3bea9e358503c5fea3f6e337d7b80344549`.
  Source target dependencies were copied with APFS clones from #12; recorder built
  and bound this issue's clean source independently.
- Recorded production headless scenario:
  `headless/20261023T012848860135Z-902adb5-scenario-12816-1-1792718928933583000`.
  All **5 assertions passed**, 284 completed ticks, forward 11.33329 m/s versus
  reverse -4.541967 m/s; command/physics/input/event streams finalized with no losses.
  Content verification reports 120 valid inventory entries, complete runtime
  material and rebuild material, manifest SHA-256
  `ccf2f230b7a62d88eb5e9a7f5b29c8443120aba53f9599710cfec27cbb8cbcc6`.
  Exact runtime replay remains unproven, as explicitly reported by the bundle.

## Continuation work

Source handoff `902adb5` is stable for #15 integration and independent review.
**Issue #14 is not complete and has no PR yet.** Coordinator requested releasing
this worker slot after headless/build evidence. Resume on coordinator followup:

1. Receive independent standards/spec review and fix actionable findings.
2. Acquire coordinator GPU lock (currently #12, followed by #13 then #14).
3. Run full offline fleet chain using this worktree's local target and isolated
   `artifacts` symlink; preserve actual 13-car/52-capture reports.
4. Run `independent-vehicles-metal.json` offscreen and native with recorded binary;
   inspect actual two-model capture, confirm finalized scenario/recorder evidence.
   Exercise applicable native live workflow; do not count unobserved input/captures.
5. Update this document with evidence/limits and review resolutions, create stacked
   reviewable PR against #12's branch, attach PR, notify coordinator. No merge/close.

Worktree `artifacts` now points to primary `artifacts/issue-14/`. Local release
recorded executable/sidecar are retained in this managed worktree `target/release`.
The existing sibling `carmodels` symlink resolves all original source files. Routine
headless checks still use shared primary target; preserve local release target for
exact recorded runs. See `evidence-checksums.json` before using raw evidence.

## Independent spec review resolution — reserved player evidence alias

Review found a secondary `VehicleId("player-vehicle")` could collide with the
selected player's legacy trace actor (`pilot-350z` remains its domain ID). Secondary
spawn now rejects that reserved alias as `reserved-vehicle-id` before creating any
entity. The production command regression submits this exact spawn, checks its
rejection, verifies every completed-tick vehicle count stays one, and verifies
unique `(actor_id, tick)` effective-control consumption. Focused regression passed.
Earlier `902adb5` build/run claims above remain historical and are not transferred
to this followup source; the alias fix receives fresh checks/build evidence.

### Shared-target overlap discovered after the alias fix

Coordinator/#15 reproduced cross-worktree test-executable replacement despite
Cargo's compilation lock: compilation locking does not cover the entire test
execution. The initial `*-bc9ef37.log` routine checks are therefore historical
and **not acceptance evidence** even though 69 tests nominally passed. No further
shared Cargo commands will start until the coordinator's whole-command lease is
available; rerun checks with that lease and preserve new `-leased` logs. Existing
bound recorded builds/runs use a separate checkout-local target and retain their
independent source attribution. No active target was cleaned.

Alias-fix checkout-local recorded release build on source `bc9ef37` succeeded with
`rebuild_complete=true`, no omissions, binary SHA-256
`49cbd5814ba7d88af1b23ddeb92e593c0be7b88691471a53731643fd55b804ef`.
See `record-build-bc9ef37.log`; no scenario or GPU run has yet used this new binary.
Source fix `bc9ef37` was sent to #15 for integration; current full acceptance remains
pending the serialized rerun.

All subsequent Cargo acceptance workloads, including checkout-local builds, must
hold the coordinator's **outer whole-command lease exactly once**:

```sh
python3 /Users/nicolas/Documents/github/car-game/artifacts/p1-coordination/cargo_serial.py bash <ladder-script>
```

Do not nest the wrapper inside an already leased script. Coordinator priority is
#13 complete script, then #14 exact ladder/GPU chain, then #15. At this checkpoint
no Cargo/acceptance command from #14 is running. Await coordinator followup before
starting another. Never clean active build caches. Keep older evidence as historical
source-bound checkpoints; do not silently overwrite earlier logs or claims.
