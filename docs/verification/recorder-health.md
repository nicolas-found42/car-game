# Recorder health and bounded incident evidence

Issue #87 extends `RunRecorder`/`PhysicsTrace` in the production application. The
Rapier schedule, 120 Hz canonical clock, commands and scenario runner are unchanged.
This adds telemetry reliability within the existing run directory, without a save
format. Full trace-semantic validation remains #86.

## Recording and resource policy

All five streams (`events`, `commands`, `physics`, `input`, `log`) share a checked
writer thread. Producers serialize before a short ordering mutex, then use a
nonblocking bounded queue. Limits count the in-flight record. On saturation the
newest disk record is rejected, health latches failed, stderr names the stage and
stream, and bounded loss ranges retain recorder sequence/tick endpoints. Later
successful writes cannot restore healthy status. The incident ring can retain a
rejected record independently of the disk queue.

Default limits, configurable with the corresponding numeric CLI flag:

| Resource | Default | Flag |
|---|---:|---|
| Queue record count | 8,192 | `--recorder-queue-records` |
| Queue allocation accounting | 8 MiB | `--recorder-queue-bytes` |
| Serialized record | 1 MiB | `--recorder-max-record-bytes` |
| History window | 3,600 canonical ticks | `--recorder-history-ticks` |
| History allocation accounting | 64 MiB | `--recorder-history-bytes` |
| History record count | 32,768 | `--recorder-history-records` |
| Tick-triggered flush | 120 ticks | `--recorder-flush-ticks` |
| Wall-triggered flush | 500 ms | `--recorder-flush-interval-ms` |
| Individual drain/export acknowledgment wait | 2,000 ms | `--recorder-drain-timeout-ms` |
| Raw output budget for provenance finalization | 512 MiB | `--recorder-finalize-max-bytes` |

Configuration validates upper bounds: 32,768 queued records/64 MiB, 65,536 history
records/128 MiB/7,200 ticks, 1 MiB per record, 5,000 ms per acknowledgment, and 1 GiB
finalization input. Metadata serialization is capped at 2 MiB; legacy tails and
failure details at 256 KiB each; incident reasons at 128 characters. Finalization
rejects more than 8,192 output entries and streams JSONL through a 2 MiB line
buffer. Five stream buffers use 64 KiB each. Control messages have four reserved
queue slots; periodic flush requests coalesce.

Queue/history bytes charge allocated payload capacity plus 128 bytes per record.
Collection/allocator overhead is additional, bounded by the record/tick caps.
Incident snapshots share record bytes, but new history can replace them during
export. One atomic reservation shared by ordinary and panic export permits at most one
queued or active incident snapshot. Reserve two history budgets including the live
ring; retries while an export is pending cannot allocate another snapshot.
Caller-owned inputs and concurrent caller stacks are additional.
Raw logs are excluded from the history ring and portable diagnostic export.

The CPU target for ordinary production payloads is producer p99 below 1 ms versus
an 8.33 ms fixed tick. This is a measured target, not a real-time deadline for an
arbitrary Rust `Serialize` implementation. Record/count caps bound ordinary work;
health exposes producer, writer, flush/sync and incident-export elapsed-time
histograms (p50/p95/p99 upper buckets, total and maximum). An OS write/fsync that
blocks indefinitely cannot be interrupted by the recorder. Timeout cancels pending
queue records and avoids joining the worker; size-limited metadata/export still
requires functioning storage. Multiple acknowledgment waits and final inventory
publication contribute to total shutdown time.

## Health, history and durability

`recorder-health.json` is the versioned heartbeat/checkpoint. It reports per-stream
accepted/written/flushed/synced sequence boundaries, queue high-water marks, known
losses, errors, last acknowledged durable canonical tick, completed tick, configured
limits, history coverage and elapsed-time measurements. Errors are also printed to
stderr. Serialization failures, oversize payloads, open/write/flush/sync errors,
unfinished drop, failed metadata publication and drain deadlines fail the run.

`last_durable_tick` advances only after all stream buffers flush and sync and the
health checkpoint publishes. It is deliberately conservative: wall flushes may
sync newer bytes before a tick marker acknowledges their completed boundary. After
a failure it never advances past the last known good boundary. Flush/sync errors
leave actual post-boundary durability unknown; per-stream counters retain what was
acknowledged. Panic explicitly marks unknown loss ranges. This does not promise
power-loss recovery; atomic rename is not a filesystem/directory durability proof.

The default ring covers 30 seconds of simulation at 120 Hz, rather than 30 seconds
of paused wall time. It holds commands, effective inputs, events, physics samples
and completed actor state, ordered by canonical tick and recorder sequence. Byte
or count pressure can shorten the requested window; `history.shortened` and
capacity/window eviction counts make that explicit. Delayed older records cannot
extend the window. `incident-history.jsonl` wraps each original record with the
same run ID, stream, tick and recorder sequence. `incident.json` records coverage,
reason and health. The first successfully published incident is preserved.

Caught unwinding panics set atomic failure state and try a separate, nonblocking
bounded context slot. The worker exports history and `panic.json`; the main catch
path adds `failure.json`. Message/location are best effort. Backtraces and unavailable
context are explicitly marked unavailable. The hook performs no recorder I/O and
never acquires producer/history/manifest locks. Abort, stack overflow, repeated
termination and a permanently blocked kernel call have no graceful-export promise.

SIGTERM/SIGINT/SIGHUP use a dedicated handler thread to request normal app exit,
including while paused. Signal identity is unavailable through this adapter; the
run says `termination_requested`, without inventing the exact signal. A second
signal can force Bevy's immediate exit. SIGKILL/power loss cannot be captured.

`finish()` drains the streams, freezes its exit result, publishes writer-finalized
health and then publishes the provenance inventory checksum followed by the index
as the final commit boundary. Repeated calls return the latched result. For runs
with provenance capture, healthy inspection requires that committed index and its
matching checksum/run ID; a kill during inventory export remains unknown even
when writer health and manifest end markers survived. Export failure latches a
nonzero result and incomplete health. A bare library recorder without provenance
capture has only the writer-health finalization contract.

Inspect a selected run explicitly:

```sh
python3 tools/verify_run.py --health-only --json artifacts/runs/<run-id>
python3 tools/evidence_manifest.py --action verify --run artifacts/runs/<run-id>
```

Health-only success means the recording completed coherently. It does not mean
scenario assertions, screenshots or gameplay passed; `behavioral_exit` remains
separate. The normal verifier also checks its original fleet/gameplay evidence.
Missing/malformed health or manifest, unclean end markers, legacy recordings and
uncommitted inventories report unknown/incomplete, never inferred success. Content
integrity alone likewise does not prove behavioral success or deterministic replay.

## Reproduction and retained evidence

Run public boundary and actual child-process controls:

```sh
cargo test --test recorder_health --test recorder_shutdown
python3 tools/test_recorder_health.py
python3 -m unittest discover -s tools -p 'test_*provenance*.py'
```

`recorder_health` includes real custom serialization failure, an occupied storage
path, metadata publication failure, oversized payloads, slow queue saturation,
write/flush/sync/drop faults, bounded tails/history, concurrent producers, deadline
cancellation, malformed export and finalization byte/checksum failures. The child
suite sends real SIGTERM while active/paused, SIGKILL during recording and inventory
export, and injects an unwinding panic while the producer ordering lock is held.
All child waits have guards and unique counter-based directories; failing child
artifacts are preserved for diagnosis. Library callers and manual test fixtures
must call `finish()` before removing a run directory. An unfinished drop is a
failed recording and can still be completing its bounded worker shutdown.

Production CLI fault controls use `--recorder-fault write:200` (also `flush`,
`sync`, `drop`, `panic`) and `--recorder-writer-delay-ms 20`. The threshold is a
recorder sequence for write/drop/panic and written count for flush/sync; it is not
a canonical tick. Queue pressure is reproducible with
`--recorder-queue-records 8 --recorder-writer-delay-ms 20`. Settings and fault
controls are captured in effective configuration/provenance. These controls fail
the recording intentionally; do not use them for valid gameplay acceptance.

Local raw runs, stderr, exact commands, compiler-bound executable metadata,
checker reports and measurements are retained under
`artifacts/issue-87-recorder-health/`. They are ignored by Git. The final validation
readout below identifies the tested source/build and measured results.

## Measured controls (2026-10-23 UTC)

Measured on macOS/aarch64, Apple M5 Pro, 48 GiB RAM, muted and render-disabled.
Compiler-bound dev-profile application source was clean commit `aac21ee`, binary
SHA-256 `0d1a8f9b5a1ffed3c82512c9d5ad6f48c082b8fd35f076d93c42645fab5246e6`.
Later commits fix test-directory isolation/teardown and the capacity-eviction
coverage marker for delayed records and incident reservation under retrying storage
failures, and document results; these measurements
identify their original source explicitly. They are local evidence,
not a platform-independent performance guarantee.

`production-final/` retains twelve cases plus the unwritable-destination control.
Every case has exact command, raw run, stdout/stderr, health checker output and
resource readout; `reproduce.py` retains the invocation helper. An initial helper
assertion confused readiness-relative scenario ticks with canonical ticks. The
corrected validator checks the observed canonical interval; its result and the
original failed assertion are both retained. No application failure was suppressed.

| Control | App exit | Health check | Observed result |
|---|---:|---:|---|
| Valid scenario | 0 | 0 | Tick 363 acknowledged durable; no dropped records |
| Write / flush / sync / drop injection | 1 | 1 | Stage-specific errors, conservative durable boundary |
| Slow writer / queue of eight | 1 | 1 | Explicit bounded backpressure and loss ranges |
| Producer panic while holding ordering lock | 1 | 1 | Panic context, failure and incident files survive |
| Real SIGTERM, active / paused | 0 | 0 | Final durable tick equals completed tick; no invented scenario pass |
| Real SIGKILL | -9 | 1 | Unknown unclean exit; last acknowledged durable tick 120 |
| 2 KiB history + failing assertion | 1 | 0 + warning | Recording durable; incident window explicitly shortened |
| 4,000-tick scenario + failing reset-count assertion | 1 | 0 | Recording durable; all 3,600 pre-incident ticks retained |
| Occupied storage path | 2 | unavailable | Structured startup error on stderr; no invented run health |

The long incident is
`production-final/long-incident/runs/20261023T000505914438Z-aac21ee-scenario-12475-1-1792713905931381000`.
Its assertion expected two resets after scheduling one. The actual reset command,
correlated events, effective inputs and completed state survive in 24,132 incident
records spanning canonical ticks 404–4003. Capacity evictions were zero. Final
history accounting was 24,029,568 bytes/24,135 records; later exit records explain
the small difference from the frozen incident. Queue high-water was 32 records/
32,256 bytes. Wall time was 35.446 s, total child user+system CPU 7.752 s.

Producer p50/p95/p99 upper bounds were 16.4/32.8/32.8 µs (maximum 958.5 µs),
meeting the 1 ms p99 target. Accumulated measured producer time was 261.7 ms,
0.74% of whole-process wall time. Writer time was 40.8 ms; worker flush/sync time
1.531 s across 74 observations (maximum 37.2 ms). Incident export took 17.8 ms.
These are elapsed-time instrumentation counters, including waits but excluding
the histogram-update operation itself; they are not an isolated CPU delta versus
a recorder-disabled build. Child CPU includes startup, simulation and provenance.
The process-set peak RSS upper bound was 282.1 MiB (macOS `ru_maxrss`, includes the
application and provenance capture, not just the ring). No other benchmark ran
concurrently with these measured controls.

Portable exports of the valid and write-failure controls both pass hash/inventory
integrity, with the latter retaining failed recorder health and nonzero behavior
result. The normal gameplay checker rejects both the lost-record control and the
intentional scenario failure. Raw and portable verification reports are retained
alongside the cases. Content integrity is kept separate from recorder/gameplay
acceptance.
