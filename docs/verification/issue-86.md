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

`tools/test_trace_integrity.py` (30 cases, written failing first; wired into `scripts/verify.sh`):
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

## Full-send completion

The continuation reproduced two false successes against the checkpoint verifier:
a declared second actor with no trace, and an unrelated event carrying a command
ID in place of its outcome. Both negative controls now fail. Thirty focused
controls pass, including supported120-Hz seconds/step alignment, nested finite
values, wheel vector shapes, completed-state/effective-control agreement,
actor populations, targeted player-reset identity, capture frame/actor links,
and capture-free scenario completion alignment.

Real production copies preserve the original run unchanged. The valid13-car
control passes; empty streams, absent actor, unrelated outcome, wrong reset actor,
corrupt capture and corrupt tail all fail with machine-readable locations.
Results and source hashes are in `issue-86-controls/production-results.json`.
The production independent-two-car scenario also passes this verifier, including
its targeted player reset (`issue-86-controls/multi-validation.json`).

Scenario capture request/validation callbacks now emit additive actor/tick/frame
links. Required scenario captures are checked against those links and summary
hashes; older capture runs without links cannot be promoted to verified by
assuming the missing context. Recorder-health-only inspection remains available.

The fresh full recorded ladder passed on the pre-#16 integration. Final
verification after rebasing on merged#16 is retained at
`artifacts/issue-86/final-verification.log` in the primary checkout. Raw controls
and unchanged originals stay under `artifacts/issue-86/full-send-controls/`.

Sol-loop completed three rounds with gpt-6.1-sol on medium. The scenario
completion-tick finding was fixed; final verdict approved. Repository complexity
limits remain unchanged. This verifier does not promise exact replay or infer a
crash from a missing exit record.

Final post-#16 recorded verification passed: all91 Rust tests, doc tests,
format/Clippy/complexity, all30 integrity controls and the Python/source-asset
ladder;13 fleet cars/52 captures and strict full-run validation pass. Both the
independent-two-car Metal offscreen and native scenarios pass their assertions,
required captures and full semantic validation, including the new asynchronous
capture links. The native capture was visually inspected. Recorded binary:
`a5486cf54dd47644b6a22972e8515ff45dc12bb5cdcba0ccd2bdbafb590e981a`.
Later corpus-only and expected-test-fixture-cleanup merges do not change those
production Rust bytes. Final full-run logs and accepted scenario bundles remain
under the primary checkout's `artifacts/issue-86/`.

Bot review additionally exposed and reproduced four cases: three-view visual
reports, malformed evidence/tick/actor shapes, nonnumeric wheel vectors and
same-basename capture substitution from a different directory. All are fixed;
the new controls fail against the prior PR head and pass the corrected verifier.
All34 focused tests and the supplementary full headless ladder pass. A real
13-car visual-only run with39captures passes strict verification, as do the
retained full fleet and two-car scenario runs. Sol's fourth review round approved
the state/identity fixes. No production Rust changed in this review pass.

Final review also rejects empty smoke reports and malformed envelopes with
located findings. Scalar/vector types and event-versus-physics frame identity
are checked. All36 focused controls pass; the real native/offscreen two-car and
13-car visual-only runs pass the final validator. Sol's fifth and final state
review approved these changes. Supplementary gates are retained in
`artifacts/issue-86/final-bot-verification.log`.

### Final review controls

The 47-test integrity suite includes event-backed reset counter deltas (including two same-car reselections in one tick), malformed command identities, strict boolean/numeric correlation and absent scenario definitions. These controls produced nine failing subcases on the preceding implementation and all pass after the fixes. The retained full fleet, visual-only and native/offscreen multi-actor runs still verify with these checks. The final headless ladder passed in `artifacts/issue-86/latest-bot-verification.log`.

A malformed switch command regression (null/list/string) failed in three subcases before the final nested-command guard; all 47 integrity tests now pass.

A recursive command-payload correlation control rejects a numeric handbrake flag replacing a boolean; it failed before the type-aware comparison and passes afterward.

The same boolean/numeric class is rejected at completed event frames, evidence query ticks, step and integration counts. Four clock-alias controls failed before explicit integer guards; all 45 focused tests pass after.

A future command outcome beyond the final completed boundary cannot certify an ingress action. The regression passed incorrectly before the upper-bound guard; all 47 tests pass after.

An additional check of the same type-confusion class rejects booleans in assertion counts and numeric report pass flags. All four subcases passed incorrectly before the guards and now fail; the full focused suite contains 47 tests.

Completed reset counters compare as exact integers; float tolerances apply only to physical values. Canonical ingress commands require an object before target lookup. Fractional/float counter and malformed targeted-reset controls fail before these guards and pass afterward.

A contradictory scenario.finished envelope tick is rejected even if its embedded summary agrees with the file. The focused control failed before the alignment guard and passes afterward.
