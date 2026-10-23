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
The incident snapshot shares record bytes, but new history can replace them while
the snapshot is exporting: reserve up to another history budget for this overlap.
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
All child waits have guards; failing child artifacts are preserved for diagnosis.

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
