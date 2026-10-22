# Shared production runtime and scenario evidence (#11)

`car_game::build_app` now constructs the same City, Vehicle, Rapier, commands and
recorder for interactive play, existing verification flags and bounded scenarios.
See [the scenario interface](../../agents/scenarios.md). This extraction retains
120-Hz outer physics and the existing pre-SyncBackend force computation. It does
not move the physics bridge or add later mechanical/career subsystems.

Durable, ignored evidence is retained at
`/Users/nicolas/Documents/github/car-game/artifacts/issue-11-runtime/`.
`retained-runs.json` maps original run IDs to retained directories. Source/binary
identity and premerge/combined provenance availability are explicit in each run;
copying historical evidence does not replace its original metadata.

## Free-drive compatibility

Both `scripts/verify.sh` and `scripts/verify_live.sh` passed. The retained
`offline-fleet/` and `live-fleet/` each contain all 13 original roster cars, 13
successful driving/braking/turn/reset reports and 52 validated GPU captures.
Asset integrity, Rust formatting, all-target Clippy, complexity gates and
Python syntax checks passed. Final combined acceptance includes 31 Rust tests. Existing keyboard wrap and gamepad bumper behavior
are exercised through the production application/input ingress. Replay tests
cover legacy flat input, empty replay returning live control and a final replay
sample whose reset must execute before ownership returns to live input.

`handling-comparison.json` compares the actual new fleet trace with #10's retained
350Z reference: 600 uninterrupted full-throttle, zero-brake/steer/handbrake samples
and the endpoint exactly 600 fixed steps later. The old reference is 86.705568 km/h
and 66.47365 m planar displacement at five seconds. The new run is 86.705586 km/h
and 66.47366 m: speed differs by 0.00002076% and the planar endpoint by 0.000010 m.
These are the legacy pre-integration samples, not completed-tick or frame-time
proxies. This comparison supports preservation on this host/workload; it does not
ratify the parent's proposed fleet-wide or cross-platform numerical/performance
budgets.

The actual Apple M5 Pro/Metal native window was created at 1920×1080 and focused;
`live-platform/native-window.log` and `native-window/` retain the observed window
and focus context. Its external 30-second termination is an expected bounded
launch check, not a cleanly finalized gameplay run. All GPU runs are muted.
Physical gamepad compatibility and audible sound quality remain unverified.

## Scenario acceptance

The real Metal driving/reset scenario executes all three assertions, moves and
accelerates the actual car, records one reset command correlated with one actual
reset transition and its completed canonical tick, and saves a validated
1920×1080 screenshot. The explicit offscreen career scenario executes all four
assertions: unowned and unknown selection requests are rejected, the 350Z remains
selected, and it subsequently drives. Pause and resume command ingress retain
distinct frames/monotonic timestamps at the same stopped canonical tick.

Actual CLI negative controls retain nonzero exit codes and failed summaries for
unavailable required captures, readiness timeout and a paused real wall deadline;
none executes any assertions. Empty assertions, an empty filter and a filter
matching zero scenarios are rejected before application construction. The first
experimental readiness failure is retained as failed evidence and is not counted
among passing scenarios.

Additional production-seam regressions require late elapsed commands to reach
actual dispatch before success, and stop catch-up work exactly at the declared
completed-tick bound. The late-selection test was observed red with a skipped
command, then green with an applied transition. No subsystem-specific test
application or alternate force solver was introduced.

## Recording boundaries

The canonical simulation clock advances independently of per-car trace sample
count, including reset ticks. Legacy physics samples keep their pre-integration
meaning; additive completed-tick records identify post-writeback observations.
Commands are persisted once at ingress, before one-shot consumption, with common
versioned run/actor/command/correlation/frame/wall/monotonic context. Applied,
rejected and skipped effects preserve that correlation; legacy event names remain
available additively. The production Last schedule flushes live-world traces and
accounts for pending commands before runner teardown and provenance finalization.

Combined recorded builds use the exact resolved runtime asset root and embedded
catalog, effective scenario/context/seed settings and scenario/replay prerequisites.
The common [provenance contract](../provenance.md) supplies content/build identities
and portable integrity checks. Ordinary premerge Cargo runs retain their legacy
identity limitations explicitly; their real gameplay/capture evidence remains
available without being upgraded to compiler-bound replay claims.


The #88 review controls are retained separately at
`/Users/nicolas/Documents/github/car-game/artifacts/issue-88-provenance/review-fixes/`.
They exercise compiler-bound sidecar mutation rejection, canonical build-input
fingerprint validation, inventory/schema/material closure, and refusal to upgrade
unknown or missing prerequisites by changing completeness flags. The exact
recorded executable and matching sidecar/source objects for the final combined
run are retained under this ticket's durable artifact directory, alongside
portable scenario exports and their independent integrity results. Compatible
platform/driver availability and deterministic replay remain explicit external
prerequisites; valid content integrity does not claim they are established.
