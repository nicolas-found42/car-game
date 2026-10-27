# Issue #86 — evidence integrity

`tools/verify_run.py` remains the single public verifier. It now calls
`tools/trace_integrity.py` after the existing recorder-health precheck, so an
incomplete or corrupt trace can no longer print `RUN EVIDENCE PASS`.

## Result contract

Failures print one JSON object and exit 1 (success prints `RUN EVIDENCE PASS`, or the same
object with `--json`):

```json
{"schema": 2, "ok": false, "status": "failed|unsupported|unknown|verified",
 "run_id": "...", "errors": [{"code": "...", "file": "physics.jsonl", "line": 4, "tick": 5, "actor_id": "player-vehicle"}],
 "recorder_health": {}}
```

`status` meanings: `failed` (evidence contradicts itself or is missing), `unsupported`
(a record carries a schema version this verifier does not know; never guessed),
`unknown` (manifest/health unavailable, or a verifier fault; no traceback escapes),
`verified`. A missing exit is not read as a crash. A run that completed zero ticks
reports `simulation_not_started` and is never `verified`.

## What is checked (all run modes)

| Stream | Checks | Codes |
| --- | --- | --- |
| `physics.jsonl` (#15 completed-step records) | nonempty; parseable UTF-8 JSON; envelope schema; finite typed `position`/`rotation`/velocities/`speed`/`reset_count`; per-actor ticks start at 1, contiguous, unique, ordered; last tick equals recorder `completed_tick` | `trace_empty`, `trace_malformed_record`, `trace_schema_unsupported`, `trace_invalid_value`, `trace_tick_gap`, `trace_tick_duplicate`, `trace_tick_out_of_order`, `trace_tick_coverage` |
| `input.jsonl` | same tick continuity; each row equals the completed record's effective `input` for the selected player | `trace_input_mismatch` + tick codes |
| reset | reset one-shot ⇔ `evidence.outcome == reset`, counter advances by one, one `vehicle.reset` event per reset tick | `trace_reset_mismatch` |
| `events.jsonl` | strictly increasing `seq`; `simulation.tick_completed` for every tick 1..completed | `trace_event_sequence`, `trace_event_incomplete` |
| `commands.jsonl` | unique command ids; every ingress before the final boundary has a `command.outcome` or effect event | `trace_command_duplicate`, `trace_command_unresolved` |
| `report.json` captures | recomputed SHA-256 equals the recorded hash | `capture_hash_mismatch` |

Mode requirements: fleet keeps 13 reports, 13 `car.report`, 52 validated captures and
`run.complete`; GPU record is required only for rendered modes (`fleet`, `smoke`);
interactive and scenario runs need no smoke report or GPU capture.
Historical runs without a schema-1 recorder health declaration still stop at
`legacy_health_unknown` (status `unknown`), so they are never reported verified.
Fleet car swaps bump `reset_count` without a reset outcome; that is accepted (delta of at most one).

## Tests

`tools/test_trace_integrity.py` (17 cases, written failing first; wired into `scripts/verify.sh`):
valid control, empty traces, dropped/duplicated/out-of-order tick, truncated final tick,
corrupt tail, invalid UTF-8, non-finite/mistyped values, missing reset outcome/event/counter,
input vs state mismatch, duplicate command, unresolved command, event gaps, capture hash
mismatch, unknown schema, zero-tick failure.

## Real-run evidence

Raw artifacts (primary checkout `artifacts/issue-86/`): `mutate.py`, `controls/*.stdout.json`.
Source run: the verified #15 production scenario (`8141e45`, 103 ticks, reset at tick 9),
copied read-only before each mutation.

| Control | exit | status | first codes |
| --- | --- | --- | --- |
| valid-control | 0 | verified | — |
| empty-input-and-physics | 1 | failed | `trace_empty` ×2 |
| dropped-tick | 1 | failed | `trace_tick_gap`, `trace_input_mismatch` |
| duplicated-tick | 1 | failed | `trace_tick_duplicate` |
| out-of-order-tick | 1 | failed | `trace_tick_gap`, `trace_tick_out_of_order` |
| missing-reset-outcome | 1 | failed | `trace_reset_mismatch` |
| corrupt-tail | 1 | failed | `trace_malformed_record` |
| non-finite-value | 1 | failed | `trace_invalid_value` |
| unknown-schema | 1 | unsupported | `trace_schema_unsupported` |

Both real #15 production fleet runs (`…cd46952…`, `…d985f49…`, ~34k ticks, 144 MB physics)
verify cleanly in about 3 s, since streams are read incrementally.

## Not covered

Exact replay is not implied (provenance replay completeness stays as declared by the
provenance contract). Per-actor coverage for secondary vehicles holds only if the
recorder emits their physics records; actor population is checked per actor seen,
because input lacks `actor_id` until #16 registers further actors.

## Local verification

`CAR_GAME_MUTE=1 bash scripts/verify.sh --base origin/main`: see PR description for the result and log path.
