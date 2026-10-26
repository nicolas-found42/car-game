"""Write a durable, qualified outcome for the completed 300-car common matrix."""
import csv
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'docs/verification/fleet-300'


def read(path):
    return json.loads(path.read_text())


def profiles_csv(roster, profiles):
    rows=[]
    for car in roster:
        p=profiles[car['id']];row={'id':car['id'],'name':car['name'],'game':car['game'],
            'source_code':car['source_code'],'corpus_reference':p.get('corpus_reference'),
            'wheelbase_m':(car['wheels'][2][2]+car['wheels'][3][2]-car['wheels'][0][2]-car['wheels'][1][2])/2,
            'front_track_m':abs(car['wheels'][1][0]-car['wheels'][0][0]),
            'rear_track_m':abs(car['wheels'][3][0]-car['wheels'][2][0]),
            'front_radius_m':sum(car['radii'][:2])/2,'rear_radius_m':sum(car['radii'][2:])/2,
            'applicability':json.dumps(p['applicability'],ensure_ascii=False)}
        for key,value in p['parameters'].items():
            row[key]=json.dumps(value) if isinstance(value,list) else value
            row[key+'_status']=p['evidence'][key]['status']
        rows.append(row)
    with (OUT/'vehicle-profiles.csv').open('w') as sink:
        writer=csv.DictWriter(sink,sorted({k for r in rows for k in r}));writer.writeheader();writer.writerows(rows)


def findings_table(evaluation, names):
    ranked=sorted(evaluation['car_results'],key=lambda c:sum(r['failed_pairs'] for r in c['families']),reverse=True)[:15]
    lines=['| Source edition | Flagged case pairs | Main flags |','|---|---:|---|']
    for car in ranked:
        count=sum(r['failed_pairs'] for r in car['families'])
        reasons=sorted({reason for r in car['families'] for reason in r['failure_reasons']})
        if count:lines.append(f"| {names[car['car']]} | {count} | {', '.join(reasons)} |")
    return '\n'.join(lines)


def main():
    evaluation=read(OUT/'final/evaluation.json');initial=read(OUT/'evaluation.json')
    source=ROOT/'artifacts/vehicle-reference/fleet300/source-final'
    roster=read(source/'assets/cars/roster.json');profiles={r['id']:r for r in read(source/'assets/cars/physics.json')['cars']}
    names={r['id']:r['name'] for r in roster};profiles_csv(roster,profiles)
    bridge=read(OUT/'original-city-comparison.json');prior=read(OUT.parent/'fleet57-next-calibration/comparison.json')
    slope_before=sum(c['original_slope_before']['failure_reasons'].get('unexpected-reset',0) for c in evaluation['car_results'])
    slope_after=sum(r['failure_reasons'].get('unexpected-reset',0) for r in evaluation['family_metrics'] if r['family']=='slope')
    flags=evaluation['failure_reasons'];native=read(OUT/'native-review.json')['native']
    text=f'''# 300-car common-scenario evaluation

The fleet now contains **300 source editions**, preserving all 57 accepted editions and all 50 real-life corpus reference cars. Every car receives **the same 5,024 scenarios**: 24 original city scenarios and 5,000 new proving-ground definitions, each run twice in fresh physics worlds. Final common coverage is **{evaluation['executions']:,} executions**. The retained initial sweep, all-car slope rerun and complete repaired-profile rerun total **3,324,448 matrix executions**, excluding development probes and the interrupted throughput trial.

A zero-torque placeholder import in the GT1 ConceptCar was also corrected with an explicit torque estimate derived from rated power/RPM; all 5,024 cases were rerun twice for that car. The remaining 299 profiles are exact. The preceding supported reference corrections are the 350Z delivery estimate and early-NSX lateral-grip estimate. GT500, C5, RS4 and CTS-V improvements remain intact. See [the qualified 57-car comparison](../fleet57-next-calibration/README.md), [all 57 paired results](../fleet57-next-calibration/all-57-cars.csv), and [its source notes](../fleet57-next-calibration/remaining-source-notes.md).

## What the larger sweep found

The new downhill course exposed a fixture error: supported vehicles could cross the existing city fall-reset boundary at world Y below −12 m. The Viper trace in case 4409 reaches −12.012 m on the −5° surface. Raising the common slope course to a 128 m base corrects that conflict without modifying fall protection, vehicle power, geometry or physics. All 500 revised slope cases were rerun twice on **every** car. Unexpected-reset pairs on slopes change **{slope_before:,} → {slope_after:,}**. All original results and failure traces remain available.

Final results have **{evaluation['failed_city_pairs']:,} flagged original-city pairs**, **{evaluation['failed_stress_pairs']:,} flagged proving-ground pairs**, and **{evaluation['different_stress_repeat_pairs']:,} differing proving-ground repeat pairs**. Final stress-flag counts by reason are `{json.dumps(flags,sort_keys=True)}`; a pair can have multiple reasons. Physical guard flags are findings, not automatically proven model defects. High-speed abrupt steering and combined braking can roll taller vehicles. CG, inertia, tire and suspension inputs lack matched rollover-test ground truth, so they were not changed merely to clear a stress guard. The remaining 39 flags comprise 35 Hummer H2 cases and four Daihatsu Midget II handbrake cases. [The H2 evidence audit](h2-rollover-source-notes.md) found no matched official CG/SSF measurement; the related NHTSA entry is explicitly Not Rated, and missing-value zeros must not be treated as measured zero risk.

{findings_table(evaluation,names)}

The plots and CSVs retain the remaining risks for inspection. The geometry plot compares declared CG/track ratio with maximum trail-brake tilt; most CG values are estimates and this association is not independent causal proof. The launch comparison uses the same full-throttle 20-second case on every car. It is not a top-speed measurement or a real-world performance validation. Native game outputs, gearing, estimated tires, delivery, drag and shifts all affect those results. Preserve variant, equipment and source qualifiers before treating a ranking inversion as a tuning bug.

![Common fleet outcome](final/fleet-summary.png)

![Geometry and tilt](final/geometry-and-tilt.png)

## Data quality and interpretation

The original Fiat 500F reverse-recovery distance flag is retained, along with all other unchanged city-scenario assertions. The shared unit-test mobility gate was corrected from an unqualified 54 km/h-in-six-seconds sports-car requirement to basic propulsion response; the Fiat profile was not increased to satisfy it.

The additional 243 editions comprise 150 GT4 and 93 GT1 source models. They have individual mass, nominal output, torque, RPM and drive-layout proxies from decoded native stock specifications. Simple GT4 gearing is imported only after excluding unresolved gear flags and extra reduction stages. GT1 gearing, missing RPMs, tire forces, CG, suspension, drag, delivery and differential split remain explicit estimates. These additions are **game-derived reference candidates, not 243 newly validated real-world profiles**. Source editions can share real-world model families. Prepared source poses also need qualification: the Mercedes 300 SL has raised gullwing doors; [geometry review notes](geometry-review-notes.json) retain that observation for a bounds/inertia audit. The original real-reference corpus and its calibrated editions remain separate and exact.

`vehicle-profiles.csv` exposes every runtime parameter, evidence status and geometry measure for all 300 cars. `native-specifications.json` retains complete native rows, displayed ranges, identities and source qualifications. `native-interpretation.md` records unit conversions and exclusions. `preparation.json` records static exclusions made before physics tests; no performance outcome was used to drop a car. The original 57 profile rows and geometry receipts are preserved, including the C5’s reference geometry and driven-radius averaging.

Every car receives the same parameter grids, support surfaces and inputs. The ten new families cover launch, braking, coast-down, steering steps, slalom, trail braking, handbrake, reverse, slopes and bumps. Heading variants test rotational consistency. The pad is 8 km square, slopes span −5° to +5°, and five transverse bumps vary from 1 to 5 cm. Original city tracks, routes, scenario definitions and assertions remain unchanged. See [method and evidence limits](method.md).

## Verification and preservation

All {bridge['recorded_executions']:,} frozen 57-car recorded city outcomes match their compact counterparts exactly in assertion summaries, final positions, completed ticks and reset counts. Separate regression tests compare every sampled physics tick in the full production application and compact runner on the 350Z and four protected cars. Scheduling optimization trials preserve 1,000 proving-ground pairs and all 24 city pairs exactly. Two trajectory digests over 41-field per-tick states are retained for every final pair; complete binary traces remain for all flagged or differing pairs, with verified lossless compression.

The full repository verification ladder passed: formatting, Clippy, complexity limits, all-target/doc tests, Python controls, source asset integrity and recorded native acceptance. The native run covers **{native['cars']} cars**, wheel-animation checks and **{native['views']} captured views**. It uses a bounded 8 GiB finalization allowance. The export ceiling is opt-in; the normal recorder default remains 512 MiB. The root aggregate report has a shared 32 MiB publication/export bound, while ordinary metadata retains 2 MiB. An interrupted 64-car native report attempt is retained separately; the final native cohort reruns all 300 cars with the corrected exporter. A later Mini One failure was traced to a city-block collision before braking. The fleet-only native corridor now clears the same block columns for every car; the original thresholds and all 24 city-scenario geometries remain unchanged. [The all-300 regression](native-corridor-validation.json) reproduces the old collision and verifies the clear course. A later wall/physics clock gap skipped native stages after only 2.1 physics seconds. Native phases now follow simulation time while the original capture/load wall deadlines and real FPS measurements remain intact; [the 30-second gap control](native-clock-validation.json) verifies this correction. Verification holds a process-scoped macOS idle-sleep assertion. Equivalent provenance allowlist caching and serialized shutdown controls preserve their original deadlines. Build/source manifests, binary hashes, common scenario hashes, full coverage audits and native evidence are bound in `validation.json`. Native capture sheets and both result figures were visually reviewed. Material translation artifacts remain visible, including the C5 front body and some colored GT4 glass. The gate establishes model integration, captures and animation; it does not establish visual fidelity to the source games.

Preparation/evidence mistakes and throughput trials are retained with explicit dispositions, rather than counted as complete coverage. The 57-car comparison excludes 143 evidence-incomplete staging attempts and reruns their tasks with identical physics. The compact-matrix throughput trial was interrupted before the final run; its recoverable prefixes are retained separately. Wall-clock metadata, display-name prefixes and recorder artifact lists intentionally differ between recorded and compact execution. Content integrity and deterministic local repeats do not prove portable cross-platform replay.

## Inspectable results and next supported questions

- [Final car/family metrics](final/car-family-metrics.csv), [selected common-case metrics](final/selected-common-case-metrics.csv), and [complete final evaluation](final/evaluation.json).
- [Initial pre-track-fix evaluation](evaluation.json), [300-car profile table](vehicle-profiles.csv), and [validation receipt](validation.json).
- A remaining software mismatch is confirmed in the GT1 NSX-R LM GT2 (`exp-231`): it reaches only 85.71 km/h in the common 20-second launch. Its estimated gearbox lands at 5,296.97 rpm after the first upshift, below the 5,330 rpm downshift threshold. An independent recorded straight-road probe confirms **592 first/second gear transitions in 20 seconds**, with zero drive demand in 94.7% of the final ten seconds and four contacts throughout that stall window. See [threshold calculation](gear-hunting-candidate.json), [full diagnostic and trace binding](gear-hunting-trace.json), and [probe definition](gear-hunting-probe.json). Transmission hysteresis is the next supported software fix; increasing power or grip would conceal the mechanism. This new expanded-fleet finding remains unresolved in this benchmark, which provides the common baseline for that follow-up.

- Highest-value next evidence: matched rollover/step-steer measurements for flagged tall vehicles; tire-specific cornering/braking data rather than shared priors; GT1 transmission ratios and actual game modifier chains; test-load/rollout/market qualifiers for the deferred 911, GNX and Elise comparisons. These would distinguish incorrect data or model behavior from plausible responses to extreme inputs.
'''
    (OUT/'README.md').write_text(text)


if __name__=='__main__':main()
