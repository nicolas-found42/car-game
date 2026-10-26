# Completed-step recorder schema handoff (#15 → #86/#89/#91)

Verified implementation source: `8141e45` on `codex/issue-15-physics`.
Exact recorded executable SHA-256:
`be2731d7996380de87f9453e7d7686c980fb664e13610435b59b28e248c2dfa0`.

Actual headless production CLI evidence:
`/Users/nicolas/Documents/github/car-game/artifacts/issue-15/runs/20261023T02014468296Z-8141e45-scenario-58590-1-1792720904713000000`.

The scenario passed all 8 declared/executed assertions, exited 0, and recorded 103
consecutive completed physics records at canonical ticks 1–103. Reset is tick 9 and
has a normal completed record. Portable provenance verification passed 134 inventory
entries, complete runtime material, and complete rebuild identity without omissions.
Exact runtime replay completeness remains false, as declared by the existing
provenance contract; this handoff does not assert otherwise. Recorder health has
zero drops/errors and durable tick 103. Machine-readable checks and example ordinary
and reset records are in primary `artifacts/issue-15/headless-validation.json` and
`artifacts/issue-15/schema-handoff.json`.

## `physics.jsonl`: one record per active vehicle per executed fixed boundary

Existing scalar driving fields remain additive compatibility fields. The record
reflects actual ECS Transform/Velocity after Rapier Writeback. Wheel mechanics and
force contributors reflect the current tick's explicitly labeled pre-step query.

| Field | Contract |
| --- | --- |
| `tick`, `step`, `time` | Canonical #11 tick; zero-based step = tick − 1; existing f32 fixed elapsed seconds. Use tick for joins and tolerate f32 time representation. |
| `envelope` | Existing #11 Envelope; same tick, `completed_tick` boundary, run/render frame/wall/elapsed identifiers and legacy actor alias. |
| `actor_id`, `vehicle_id`, `body_id`, `body_part_id` | Selected-player compatibility actor stays `player-vehicle`; stable typed vehicle ID is separate; stable chassis body ID and bound residual-chassis PartId. ECS/Rapier handles are never serialized. |
| `boundary`, `query_mode`, `integration_steps` | `completed_tick`; synchronized or explicitly selected `legacy_pre_sync`; one outer fixed integration when the context is active. Unavailable/inactive contexts publish zero steps and an explicit outcome. |
| `position`, `rotation`, `linear_velocity`, `angular_velocity`, `speed` | Completed actual pose/velocity and completed forward speed. |
| `input` and `evidence.input` | Effective held input/controller and consumed one-shot reset for this vehicle/tick. `input.jsonl` remains selected-player legacy replay ingress; secondary input is preserved here. |
| `evidence.outcome` | `integrated`, `reset`, or explicit unavailable reason. Reset preparation clears composed forces but still executes the normal gravity integration. |
| `evidence.query_tick`, `query_boundary`, `geometry_revision` | Synchronized current tick; `synchronized_pre_step` phase; query-publication epoch incremented every executed boundary. Legacy uses `previous_step_queries`. Geometry epoch is not a persistent assembly revision. |
| `evidence.assembly_binding`, `assembly_revision`, `assembly_generation` | `bound` with actual graph revision and derived anchor generation, or explicit `no_assembly_binding` with null revisions. #16 must register secondary/all-fleet graphs before P1 acceptance. |
| `evidence.mass`, `center_of_mass` | Actual freshly recomputed Rapier mass and world COM used for this tick's force lever arms. |
| `evidence.pre_step_position`, `pre_step_linear_velocity` | Explicitly labeled preparation state; never treat as completed pose/velocity. |
| `evidence.wheels` | Exactly four corners, each with the PartId of the wheel mounted in that corner's socket (none when the socket is empty), availability/rejection reason, stable hit collider ID, origin/direction/range, optional hit distance/point/normal and force contributors. |
| Wheel `suspension`, `tire`, `antiroll` | Force and COM-relative torque arrays from the single production mechanics solver. Missing/rejected/inverted/reset corners retain explicit reasons and zero contribution. |
| `evidence.chassis`, `evidence.total` | Chassis drag/downforce/yaw contribution and once-composed current-step sum. |
| `backend_applied` | Actual Rapier user force/torque read after the integration; compare with the composed total to detect bridge/overwrite mistakes. |
| `phase_durations` | Nanosecond brackets for `sync_ns`, `query_ns` (includes derived-mass/query publication), `mechanics_ns`, `bridge_ns`, `step_ns`, `writeback_ns`. These are measurements, not declared universal performance guarantees. |

## Domain and effect joins

`physics.assembly_checkpoint` events reuse the #13 `AssemblySnapshot` DTO without
inventing a second assembly/save schema. An initial prepared graph and each changed
revision/generation emit a snapshot at the same canonical tick with
`physics_prepared` boundary and stable vehicle actor. Restore/remapping can therefore
be followed across different ephemeral ECS entities. Snapshot DTO catalog hashes,
part/vehicle IDs, root locations, attachments, state and transaction history retain
#13 authority.

`commands.jsonl`, `command.outcome`, `controls.effective`, `vehicle.reset`,
`simulation.tick_completed` and `simulation.completed` retain #11 correlation and
canonical tick semantics. Completed physics publication precedes FixedLast tick
completion/flush, so durable boundaries do not get ahead of their actor records.

#86 owns production validation and integrity-negative controls; #89 owns checkpoint
restore; #91 owns read-only timeline/capture exposure. None should add a second
force solver or serialize runtime entity/physics/GPU handles as domain authority.

## Acceptance still pending

This is a verified headless/schema handoff, not the completed issue or phase gate.
Remaining work: consume #14 helper review fixes, full combined test ladder and
independent review/fixes, exact final rebuild, offline fleet/real Metal checks,
numerical baseline comparisons, evidence receipts, attached reviewable PR, and stop
before merging or closing issues.
