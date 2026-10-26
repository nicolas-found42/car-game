# Searchable vehicle diagnosis and automatic incident windows

The local interface is `python3 tools/diagnose.py`. It emits schema-versioned JSON
for search, analysis, bounded timelines and exact physical-state comparisons.
It reads retained raw run directories without replacing historical assets with
current ones. It does not launch or retune a car. `--diagnose` on the native
executable retains its older human-readable tail view.

```sh
python3 tools/diagnose.py search --car redline-gt500 --status healthy
python3 tools/diagnose.py search --root artifacts/runs --reason contact_loss
python3 tools/diagnose.py search --root YOUR_RUN_ROOT --scenario driving --since 2026-10-25
python3 tools/diagnose.py analyze RUN --car exp-231
python3 tools/diagnose.py analyze RUN --incident 1
python3 tools/diagnose.py timeline RUN --stream events --kind vehicle.shift_decision --limit 20
python3 tools/diagnose.py timeline RUN --car exp-037 --first 100 --last 300 --limit 100
python3 tools/diagnose.py compare BASELINE_RUN CANDIDATE_RUN
```

Search accepts repeated roots, car (including completed fleet members), mode,
recording status, binary-hash prefix, date bounds, scenario substring and recorded
incident reason. Dates use the manifest's UTC ISO timestamp; date-only bounds
compare lexically. `--until` is exclusive of later timestamps on a date-only bound;
use the next date to include a whole day. Roots are explicit: custom study roots
are not silently assumed to live beneath `artifacts/runs`. Missing roots and
malformed metadata are reported. Search and timelines cap returned rows at 1,000
and expose truncation. Timelines also cap serialized returned records at 4 MiB.
Command, correlation and actor IDs can filter timeline records. Search discovers manifests on demand rather than maintaining
a second potentially stale database. Reason search reads recorded incident metadata;
it does not analyze every trace during discovery.

`analyze` and `timeline` also accept `--incident 1` through `--incident 8` to read
a frozen automatic window directly. They return its coverage metadata; timeline
records retain the outer recorder sequence. This remains usable when the full
session streams are absent. Partial coverage is exposed rather than silently
treated as a complete history. Run-level health and window coverage remain separate.

Analysis streams the complete selected physics interval, reports gear-transition
counts and flags repeated shifting (eight transitions within 120 ticks), sustained
zero drive demand under throttle (120 supported ticks), and loss of all wheel
support after prior four-wheel support (12 ticks). Detector state resets across
car changes and vehicle resets. Startup airborne placement is not contact loss.
Whole-run event gaps are reported separately, with monotonic-time evidence and
unknown host cause. Legacy traces explicitly lack the new decision fields.
Missing/corrupt streams do not become a clean diagnosis. A diagnostic finding is
an observed symptom, not proof that the physical response is wrong.

Comparison checks all pre-existing physics sample fields in order, exactly,
including inputs, ticks and contact/load/force measurements. It excludes only
the additive decision object. The first differing sample/fields, missing fields,
unequal lengths and damaged records are explicit. This is a captured-state
comparison, not automatic world restoration or a cross-platform replay claim.

## Decision telemetry

Every normal production physics sample includes `decisions.schema_version=1`.
The boundary is `force_calculation_before_integration`; the separate
`simulation.completed` event retains its post-integration meaning.

- Engine gearbox observations include gear before/after, RPM before selection,
  up/down thresholds, shift reason, remaining shift time, engine-demand cutoff
  reason, requested engine force and driven radius. Reverse/brake routing can
  subsequently override engine demand; the existing per-wheel demand and applied
  force remain the authoritative final longitudinal observations.
- Each wheel includes accepted/missed/disabled/inverted/rejected-normal contact,
  accepted contact point/normal, requested and applied longitudinal/lateral tire
  forces, and service/handbrake demand. Comparing requested/applied forces exposes
  combined-slip saturation. This does not identify chassis collision impulses or
  the contacted collider's semantic identity.
- `vehicle.shift_decision` and `vehicle.contact_decision` events expose changes
  without emitting a second full event on every unchanged tick. They carry the
  same actor/tick and explicit pre-integration boundary.
- `diagnostic.clock_gap` records real/virtual frame deltas when real time exceeds
  one second and exceeds virtual time by more than half a second. It does not
  infer sleep, suspension or GPU stalls as a proven cause.

These observations do not feed back into force, transmission or controller state.
No profile, force equation, threshold, gear ratio, collision geometry or scenario
input was tuned as part of this instrumentation.

## Automatic windows and bounds

Production recording automatically triggers nonfatal windows for those three
vehicle symptoms and clock gaps. Each trigger requests the previous 1,800 ticks
(15 simulation seconds) and following 240 ticks (two seconds). Evidence is written
by the existing writer thread to `incidents/0001/incident-history.jsonl` plus
`incident.json`. At orderly shutdown an unfinished window is exported with
`post_complete=false`. Metadata reports requested/observed coverage, shortened
history and pre/post completeness. Neither pauses nor process death promise a
post-trigger simulation interval.

One window is active at a time, one automatic export is queued/active at a time,
and each run permits at most eight windows, with 360 ticks of cooldown following
an export request. `diagnostic-windows.json` reports triggered, exported and
suppressed counts. A long fleet session can exhaust that cap; full normal traces
still retain subsequent decision data for focused analysis. Automatic findings do
not change behavioral acceptance. Lost exports/storage errors fail recorder health.

The legacy first fatal incident and panic slot remain separate. Automatic export
adds at most one history snapshot: budget for up to three history budgets (live,
fatal, automatic), plus bounded metadata/Arc vectors and the existing queue. The
pre-window is selected from the live bounded history at export time, so capacity
pressure can shorten it; it is never labeled complete merely because time passed.
This does not add an unbounded queue, I/O on the physics thread, or a new thread
per incident. Existing provenance finalization includes the new JSON/JSONL files.

Richer per-tick samples increase disk volume. Normal full-session JSONL streams
still grow with session length; the incident cap is not a disk-retention policy.
The ordinary 512 MiB provenance-finalization allowance remains unchanged, while
the 300-car verification run explicitly requests 8 GiB. Long sessions need an
appropriate existing allowance or a separate retention/chunking workflow. A
finalization-budget failure remains an explicit recording failure, never an
invented successful export.

The CLI shows local evidence and is not a redaction/export boundary. Use the
existing allowlisted portable exporter when sharing data. This first CLI consumes
raw run directories; portable object-only bundles, compact binary traces, archived
tar files, managed retention, GPU profiling and automatic replay/minimization
remain separate work. Raw passing compact trajectories are still not retained.

## Verification

The frozen instrumentation receipt, `diagnosis/validation.json`, verifies the
historical NSX's 592 gear changes and Mini curb contact loss through production
vehicle systems, including automatic windows and decision evidence. This receipt
binds the pre-fix source and binary; it is intentionally separate from subsequent
physics changes. The current `tests/diagnostic_vehicle.rs` retains the Mini control
and checks the corrected NSX gearbox on the common obstacle-free launch fixture.
The unchanged historical city reproduction reaches a road boundary after the
gearbox correction; its result is retained separately in the NSX investigation. The clock observer has a real 30-second manual-time gap control.
`tests/diagnostic_windows.rs` covers post history, interrupted windows, repeated
bounded exports, independent fatal incidents, shortened history and storage errors.
`tools/test_diagnosis.py` covers machine-readable failures, fleet-member search,
legacy evidence, corruption/truncation and first-divergence comparison. The full
verification script and CI include the Python controls.

Historical analyses, baseline snapshots, exact comparison receipts and verification
logs for this change live under `artifacts/diagnosis-upgrade/`. Recorded build/source
identity is retained with the final native run; detailed results belong in its
validation receipt rather than being inferred from the existence of these tests.

Raw-stream analysis reads the files currently present in a run directory. Recorder
health describes the recording lifecycle; it does not authenticate later edits to
those files. Verify provenance separately for captured object integrity. Historical
validation binds the specific analyzed raw files to hashes. Incident `post_complete`
means the end tick was reached with a healthy recorder; inspect `history.shortened`
and retained endpoints as well when judging trace coverage.

The instrumentation-only validation passed: all 300 native cars and 1,200 views,
104 differential case pairs covering eight cars, 2,413 historical NSX samples and
1,812 Mini curb samples. Historical hunting, contact loss and clock gaps were
recognized; frozen portable windows preserved decisions and coverage. See the
[frozen receipt](diagnosis/validation.json). The subsequent
[NSX transmission correction](nsx-hunting/README.md) has its own source-bound
validation and complete repeated 300 × 5,024-case comparison.
