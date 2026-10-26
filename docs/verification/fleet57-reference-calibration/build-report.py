"""Write the final calibration report from paired production and qualified data."""
import json
import re
from pathlib import Path
OUT=Path('docs/verification/fleet57-reference-calibration')


def table(rows, car):
    names={'zero_60_mph':'0–60 mph (s)','zero_100_mph':'0–100 mph (s)',
           'brake_70_mph_0':'70–0 mph (m)','brake_60_mph_0':'60–0 mph (m)',
           'skidpad_200ft':'200 ft skidpad (g)','skidpad_300ft':'300 ft skidpad (g)'}
    result=['| Measure | Before | After | Qualified reference |','|---|---:|---:|---:|']
    for row in rows:
        if row['car']==car:
            result.append(f'| {names[row["metric"]]} | {row["before"]:.3f} | {row["after"]:.3f} | {row["reference"]:.3f} |')
    return '\n'.join(result)


def remaining_table(rows):
    candidates=[r for r in rows if r['delta_pct'] is not None and r['car'] not in ['ref-e12','ref-a14']]
    ranked=sorted(candidates,key=lambda r:abs(r['delta_pct']),reverse=True)[:10]
    roster={r['id']:r['name'] for r in json.loads(Path('assets/cars/roster.json').read_text())}
    names={'zero_60_mph_s':'0–60 mph','zero_100_kmh_s':'0–100 km/h',
           'zero_100_mph_s':'0–100 mph','brake_70_mph_0_m':'70–0 mph'}
    result=['| Remaining comparison | Simulated | Reference | Error | Reference qualification |',
            '|---|---:|---:|---:|---|']
    for r in ranked:
        name=roster[r['car']]+' '+names.get(r['metric'],r['metric'])
        qualification=r['reference_status'].replace('_',' ')
        result.append(f'| {name} | {r["simulation"]:.3f} {r["unit"]} | {r["reference"]:.3f} {r["unit"]} | {r["delta_pct"]:+.2f}% | {qualification} |')
    return '\n'.join(result)


def space_prose(text):
    pieces=re.split(r'(`[^`]*`|\[[^\]]*\]\([^)]+\))',text)
    for i in range(0,len(pieces),2):
        part=re.sub(r'(?<=[a-z])(?=\d)', ' ', pieces[i])
        part=re.sub(r'(?<=[a-z])(?=\.\d)', ' ', part)
        part=re.sub(r'(?<=\d)(?=(?:kg|lb|ft|g|s|mph|m²|m/s²|m|Hz|km/h)\b)', ' ', part)
        part=re.sub(r'\b(\d{4})(US|EU)\b', r'\1 \2', part)
        part=re.sub(r'(?<=[a-z%]),(?=[A-Za-z0-9])', ', ', part)
        pieces[i]=part.replace('GT5000–60','GT500 0–60').replace('GNX4.7','GNX 4.7').replace('earlyNSX','early NSX').replace('andCd','and Cd ').replace('C/D2007','C/D 2007').replace('kg,58%', 'kg, 58%').replace('km/h5.', 'km/h 5.').replace('mph9.', 'mph 9.').replace('mph{', 'mph {')
    return ''.join(pieces)


def main():
    r=json.load(open(OUT/'comparison.json')); e=json.load(open(OUT/'reference-evaluation.json'))
    m=r['matrix'];score=e['selected_target_score'];b={v['car']:v for v in r['benchmarks']}
    preserved=sum(v==48 for car,v in m['unchanged_baseline_trajectories_by_car'].items() if car not in ['ref-e12','ref-a14'])
    assert preserved==55
    text=f'''# Qualified RS4 and CTS-V reference calibration

The strongest supported remaining gaps were an incorrectly light US RS4 profile, estimated gearing, excessive RS4/CTS-V lateral grip, excessive RS4 service braking, and optimistic effective power delivery. Two profiles changed. All57 geometries and the other55 profiles are unchanged, including the GT500 and C5. All {m['executions']:,} matched executions passed; all1,368 repeat pairs have identical trajectories. The 55 untouched editions retain all2,640 baseline trajectories exactly.

## What the comparison supports

All 50 corpus reference cars are represented by 51 editions; six preserved legacy editions use separate sources without an exact corpus ID. `profile-coverage.csv` lists all 57 editions, their available corpus links, mass evidence, estimated/game-proxy parameters and standardized performance-comparison coverage. Blank corpus links remain explicit rather than forcing different variants together. The earlier40 qualified performance comparisons cover24 editions; missing measured targets do not become validation passes. Newly retrieved original road tests establish matched-diameter RS4 and CTS-V grip envelopes. Across eight selected reference observations, mean absolute percentage error fell {score['before_mean_absolute_error_pct']:.2f}%→{score['after_mean_absolute_error_pct']:.2f}%. This is a calibration score on selected targets, not held-out proof of real-world fidelity.

The GNX4.7s result is a prototype test. The early NSX production/tire configuration and hot-weather correction remain unresolved, and no matched CSL tire/circle test was retrieved. The911 asset market is inferred and its unladen simulation versus published performance-load basis is not established comparable. The Zonda's1280kg specification is dry mass, while1370kg is an explicit operating estimate; launch/test basis remains unknown. These discrepancies remain visible in `reference-evaluation.json`, with reasons for deferral. A mass-only911 sensitivity changes0–100km/h5.617→5.950→6.400s for illustrative added loads0/75/170kg. The170kg equals half the handbook's gross-minus-unladen difference, but neither payload CG nor equivalence to the separate6.8s dealer claim is established. This explains sensitivity; it is not a production calibration. See `911-load-sensitivity.json`. Mini's less-than11s claim remains an upper bound rather than an11s equality target.

The largest remaining numerical differences in the original performance comparisons are listed below. The prototype, market, transmission and equipment qualifications matter: these are investigation priorities, not exact interchangeable targets. The Viper, Supra, Elise and 350Z need better matched test/configuration evidence before changing otherwise plausible engine delivery or launch traction.

{remaining_table(e['original_qualified_rows'])}

## Audi RS4

Audi's2007 US sedan technical sheet gives1795kg rather than the previously selected related European1650kg,58% front weight, six forward ratios,4.111 final drive and Cd 0.31 × 2.17 m² frontal area. These replace mismatched/estimated inputs. The2007 sheet remains related-year evidence for the2008US edition. Conflicting engine unit/rpm labels do not override the later engine reference. Direct PDF access returned403; indexed original tables supplied the values. Source qualifications and retrieval evidence are retained in the supplement.

Separate probes showed that OEM inputs alone change0–100mph9.417→10.058s. Corrected inputs plus estimated effective delivery 0.80 produce11.083s. Estimated `lateral_mu` 0.955 reduces excessive grip while leaving longitudinal tire grip separate. Estimated brake deceleration9.75m/s² corrects the overly short70mph stop. These model parameters are not measurements of drivetrain loss, tire friction or brake hardware.

{table(e['primary_rows'],'ref-e12')}

The C/D2007 US sedan test used PZeroRosso road tires,3980lb published test-car weight and weather correction for97°F/2000ft conditions. Its separate race-track session used different tires and is not the source of the 0.91 g target. [Audi original technical sheet](https://www.audiworld.com/model/rs4/07/07AudiRS4_TechSpecs.pdf), [C/D original test](https://www.caranddriver.com/reviews/a18200959/2007-audi-rs-4-quattro-road-test-review/).

## Cadillac CTS-V

The existing nominal LS6 output,1746kg mass, gears and longitudinal tire coefficient are unchanged. The default 0.88 delivery assumption made60–100mph too quick: 5.975s versus MotorTrend's7.2s interval. Independent delivery probes selected an effective 0.77 model estimate, giving6.900s. Lowering `lateral_mu` to 0.92 corrects the 0.88 g 200 ft observation without altering standardized braking or straight-line traction limits. Lateral-only probes leave acceleration and braking unchanged, so engine delivery is not hidden inside a tire-grip correction.

{table(e['primary_rows'],'ref-a14')}

MotorTrend supplies the2004 LS6 acceleration/braking observations; Autoweek supplies the matched200ft skidpad. Autoweek's separate5.35s launch and112ft braking differ from MotorTrend's5.1s/121ft, illustrating test-condition spread. The new5.242s launch sits between those observations. Unknown rollout is not converted. [MotorTrend original comparison](https://www.motortrend.com/reviews/comparison-2004-cadillac-cts-v-vs-jaguar-s-type-r-archive), [Autoweek original test](https://www.autoweek.com/news/a2086961/2004-cadillac-cts-v-flying-v-cadillacs-cts-v-simply-shreds/).

## Preservation and same-scenario evaluation

GT5000–60mph remains{b['redline-gt500']['after']['zero_60_mph_s']:.6f}s and60–100mph remains{b['redline-gt500']['after']['sixty_to_hundred_mph_s']:.6f}s. Its150s trace and both skidpads match the accepted baseline exactly. Its longitudinal 1.25/lateral 0.935 grip estimates and 0.82 delivery calibration are unchanged. The C5 retains its corrected2.6543m wheelbase, tracks, tire radii and body envelope, with0–60mph{b['ref-a08']['after']['zero_60_mph_s']:.6f}s and both skidpads exactly unchanged. 

All57-car fixture results repeat exactly. The 55 unchanged acceleration/braking rows and110 unchanged circle rows match the baseline exactly. The same seed 17, 120 Hz timing,24 scenario definitions and route/course hashes are retained. Timed controls compare at each recorded scenario-ready epoch; raw hashes/readiness shifts are retained. Closed-loop route feedback may adapt to changed vehicle response, with controller definitions unchanged.

The matrix passed {m['assertions']:,} assertions and all {m['routes']} routes. There were {m['requested_resets']} requested resets and 0 unexpected resets. Maximum cross-track error was {m['maximum_route_cross_track_m']:.6f}m within the 7 m corridor. Minimum four-ray contact was {m['minimum_four_contact_pct']:.6f}%; minimum four-loaded-wheel coverage was {m['minimum_four_loaded_pct']:.6f}%. `contact-comparison.json` retains every exception and its baseline, including the earlier22B wall-contact investigation.

## Verification and inspectable evidence

- Both new production regression guards fail on the old profiles and pass after correction. Tests reuse the production skidpad fixture and compare launch,100mph,midrange and standardized braking envelopes.
- Isolated probes separate OEM inputs, delivery, grip and braking; shadow baseline fixtures match production exactly. Sources, binaries, parameter files and all attempts are retained under `artifacts/vehicle-reference/fleet57-reference-calibration/` and summarized in `parameter-experiments.json`.
- Comparing the two recorded source snapshots finds only `assets/cars/physics.json` changed among compiled application inputs; runtime physics code is preserved. See `compiled-source-diff.json`.
- Full ladder: format, strict Clippy, complexity, all-target/doc tests, Python/asset controls, one recorded57-car native acceptance run and provenance verification. Log: `artifacts/vehicle-reference/fleet57-reference-calibration/verification.log`.
- Frozen recorded executable and source provenance: `artifacts/vehicle-reference/fleet57-reference-calibration/car-game.recorded.provenance`. Full study index: `artifacts/fleet57-reference-calibration-20261025/runs.jsonl`.
- All2,736 evidence bundles independently verify runtime material/rebuild completeness. Six raw exports per execution are bound to immutable objects after declared redaction/numeric serialization; full portable runtime replay is not claimed. `validation.json` records the final hashes.
- `all-57-cars.csv` contains every paired result; `comparison.json` contains all1,368 car/case comparisons. `primary-comparison.csv` and `reference-comparison.csv` retain reference qualifications. Sources: `handling-source-notes.md` and `docs/vehicle-reference/calibration-supplement-20261025.json`.
- The older initial57-car study was losslessly archived, with every file read back and verified before removing its unpacked directory. Restore instructions are in its `artifacts/archives/` receipt. The current comparison baseline remains unpacked.

Remaining high-leverage evidence: matched production GNX/earlyNSX tests, a same-market911 test at known load, a Zonda operating mass/launch basis, and a CSL test on identified tires/circle. These would justify further calibration; arbitrary changes to nominal outputs would not resolve those reference gaps. Wet tires, rough surfaces and full suspension/differential fidelity remain outside this dry-city experiment.

![Qualified fixture comparison](comparison.png)
'''
    replacements={'All57':'All 57','all57':'all 57','other55':'other 55','All2,736':'All 2,736','all1,368':'all 1,368','all2,640':'all 2,640','The 55':'The 55','the55':'the 55','and110':'and 110'}
    for a,z in replacements.items():text=text.replace(a,z)
    for name in ['profile-coverage.csv','reference-evaluation.json','911-load-sensitivity.json','contact-comparison.json','parameter-experiments.json','compiled-source-diff.json','validation.json','all-57-cars.csv','comparison.json','primary-comparison.csv','reference-comparison.csv','handling-source-notes.md']:
        text=text.replace('`'+name+'`','['+name+']('+name+')')
    (OUT/'README.md').write_text(space_prose(text))


if __name__=='__main__':
    main()
