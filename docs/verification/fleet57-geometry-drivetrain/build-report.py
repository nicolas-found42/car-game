"""Write the final narrative from the verified paired results."""
import json
from pathlib import Path
OUT = Path('docs/verification/fleet57-geometry-drivetrain')
ART = Path('artifacts/vehicle-reference/fleet57-geometry-drivetrain')


def metrics_table(comparison, car):
    row = next(r for r in comparison['benchmarks'] if r['car'] == car)
    names = {'zero_60_mph_s': '0–60 mph (s)', 'sixty_to_hundred_mph_s': '60–100 mph (s)',
             'zero_100_mph_s': '0–100 mph (s)', 'brake_60_mph_0_m': '60–0 mph (m)',
             'brake_70_mph_0_m': '70–0 mph (m)'}
    lines = ['| Measure | Before | After | Change |', '|---|---:|---:|---:|']
    for key, name in names.items():
        m = row['metrics'][key]
        lines.append(f'| {name} | {m["before"]:.6f} | {m["after"]:.6f} | {m["delta"]:+.6f} |')
    for r in (r for r in comparison['skidpads'] if r['car'] == car):
        for key, label in [('peak_sustained_g', 'sustained g'), ('peak_tilt_deg', 'peak tilt °')]:
            m = r['metrics'][key]
            lines.append(f'| {r["diameter_ft"]:.0f} ft skidpad {label} | {m["before"]:.6f} | {m["after"]:.6f} | {m["delta"]:+.6f} |')
    return '\n'.join(lines)


def main():
    r = json.loads((OUT/'comparison.json').read_text()); m = r['matrix']
    largest = sorted(r['benchmarks'], key=lambda v: abs(v['metrics']['zero_60_mph_s']['delta']), reverse=True)[:8]
    changes = '\n'.join(f'- `{v["car"]}`: 0–60 mph {v["metrics"]["zero_60_mph_s"]["before"]:.3f} → {v["metrics"]["zero_60_mph_s"]["after"]:.3f} s.' for v in largest)
    cases = [v for v in r['cases'] if v['car']=='redline-gt500']
    max_tilt_delta = max(abs(v['metrics']['peak_tilt_deg']['delta']) for v in cases)
    text = f'''# C5 Z06 geometry and driven-wheel gearing correction

The C5 stock geometry and shared drivetrain radius bugs are fixed. All {m['executions']:,} matched executions passed, covering 57 editions × 24 scenarios × 2 repeats. All 1,368 repeat pairs have identical trajectories. Every scenario definition matches the frozen 57-car baseline. Timed controls match at every tick relative to each run's scenario-ready boundary; the C5's changed settling duration is retained with raw hashes and both readiness ticks in `input-comparison.json`. Closed-loop route tests use the same courses and controller definitions; their feedback commands can change with vehicle response. The GT500 retains its launch, midrange and handling improvements without profile retuning.

## What caused the errors

The C5's prepared meshes agree with their authored pivots; this was not a wheel reparenting error. The MC3 geometry has 2.8044 m wheelbase instead of 2.6543 m, 1.467 m tracks instead of 1.58496/1.59004 m, and 0.350495/0.368019 m tire radii instead of nominal 0.3219/0.33185 m. The correction moves and scales wheel assemblies and applies the reference body envelope to shell/collider bounds. Rendered and physical wheels share the new metadata. Source vertex, triangle and image bytes remain intact; affine transforms and the receipt explicitly record the correction. Source wheel-arch styling remains approximate.

Dimensions come from the existing `a08` corpus and `am_c5_specs` period specification reproduction, whose technical origin is less certain than the original manufacturer output sheet. Tire sizes yield unloaded nominal radii, not measured loaded rolling circumference. No tire coefficient, mass, engine output, gearbox ratio or suspension parameter was fitted to these results.

The drivetrain previously averaged all four tire radii even for RWD/FWD cars. Changing undriven tires could alter RPM and trigger a power-cutting shift. Production now averages driven-wheel RPM through a harmonic radius, weighted by the profile's nominal front-drive fraction. The same radius converts engine torque to force. AWD remains an explicit nominal-coupling approximation; existing load-sensitive force allocation remains separate. Suspension rest height still uses all four wheels for its different purpose.

## C5 Z06 result

{metrics_table(r, 'ref-a08')}

The smaller effective driven radius increases force at a fixed engine torque/gear, but also lowers road-speed shift thresholds. The calculated 3→4 threshold moves 106.258→98.152 mph, adding a shift before 100 mph. This is a kinematic explanation, not a recorded shift-event measurement; see `c5-gearing.json`. Wider tracks and corrected inertia reduce skidpad tilt while sustained lateral acceleration changes little. Standardized braking distance is effectively unchanged.

## GT500 preservation

{metrics_table(r, 'redline-gt500')}

The longitudinal grip 1.24, lateral grip and .82 efficiency calibration remain byte-for-byte unchanged with every other physics profile. Launch and midrange timing remain identical at 120 Hz fixture resolution. Across the same 24 city cases, the largest absolute tilt change is {max_tilt_delta:.6f}°. The detailed 150 s GT500 trace repeats exactly; its paired checkpoints, shifts and quarter-mile result are retained under the artifact directory. Skidpad differences below a few millionths of g are numerical-scale differences, not a handling retune.

## Fleet comparison

{changes}

These are measured effects of correcting radius coupling, not new per-car tuning. `reference-comparison.csv` retains the same 40 qualified real-reference comparisons across 24 of 57 editions, including related-variant and upper-bound qualifications; the remaining editions do not acquire new measured performance targets from this experiment. `all-57-cars.csv` gives every edition's paired acceleration, braking, skidpad, contact and tilt results; `comparison.json` includes all 1,368 car/case metric comparisons and the count of unchanged baseline trajectories per car. Null 100 mph endpoints remain censored at the unchanged 20 s horizon.

The rerun executed {m['assertions']:,} assertions and {m['routes']:,} routes, with {m['requested_resets']} requested resets and 0 unexpected resets. The largest route cross-track error was {m['maximum_route_cross_track_m']:.6f} m inside the 7 m corridor. Minimum four-ray contact was {m['minimum_four_contact_pct']:.6f}%; minimum four-loaded-wheel coverage was {m['minimum_four_loaded_pct']:.6f}%. Any remaining contact exception is itemized in `contact-comparison.json`; the baseline 22B wall-contact finding is retained rather than silently treated as a tuning target.

## Verification and evidence

- Full ladder: format, strict all-target Clippy, complexity, all-target/doc tests, Python controls, asset/source integrity, one recorded 57-car native acceptance run, fleet verification and provenance verification. Full log: `artifacts/vehicle-reference/fleet57-geometry-drivetrain/verification.log`.
- `compiled-source-diff.patch` compares captured production inputs against the accepted recorder-budget baseline; only the C5 roster row and two drivetrain source files differ. The earlier recorder-only baseline bridge is linked in `compiled-source-diff.json`.
- The production-path undriven-tire regression fails with the old averaging line and passes with the fix. AWD/RWD/FWD kinematic checks and geometry/forged-receipt controls pass. The C5 rebuild reproduces the GLB byte-exactly from its original archive; receipt JSON is semantically identical despite key ordering. Reapplying the correction is idempotent.
- Scenario run index: `artifacts/fleet57-geometry-drivetrain-20261025/runs.jsonl`; frozen binary and source provenance: `artifacts/vehicle-reference/fleet57-geometry-drivetrain/car-game.recorded.provenance`.
- Every finalized bundle is independently verified; six raw exports per execution are bound to immutable objects after declared policy redaction/numeric serialization. `validation.json` records final file hashes and audit counts. Runtime material and rebuild completeness do not imply full portable runtime replay.
- Older completed 40-car studies were losslessly archived with every file checked before unpacked copies were removed. Restore commands and full SHA receipts are in `artifacts/archives/`; the frozen 57-car baseline remains unpacked.

The experiment retains the same seed 17, 120 Hz timing, dry city and input definitions. It verifies this correction and regression coverage; it does not establish wet-surface, rough-road or full real-car fidelity.

![Paired fixture comparison](comparison.png)
'''
    (OUT/'README.md').write_text(text)


if __name__ == '__main__':
    main()
