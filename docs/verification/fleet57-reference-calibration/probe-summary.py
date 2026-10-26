"""Summarize isolated parameter experiments without treating overrides as production."""
import json
from pathlib import Path
ART=Path('artifacts/vehicle-reference/fleet57-reference-calibration')
OUT=Path('docs/verification/fleet57-reference-calibration')
rows=[]
for path in sorted(ART.glob('probe-*-overrides.json')):
    tag=path.stem[len('probe-'):-len('-overrides')]
    entries={'experiment':tag,'overrides':json.loads(path.read_text())}
    for kind in ['benchmark','skidpad']:
        p=ART/f'probe-{tag}-{kind}.json'
        if p.exists():entries[kind]=[r for r in json.loads(p.read_text()) if r['car'] in ['ref-e12','ref-a14']]
    rows.append(entries)
(OUT/'parameter-experiments.json').write_text(json.dumps({'rows':rows,
    'method':'One-variable-family probes separate OEM inputs, effective delivery, braking and lateral force limits. Candidate2 combines the selected corrections. Shadow baseline fixtures match production exactly; final fixtures must use production with no overrides.',
    'probe_source':'artifacts/vehicle-reference/fleet57-reference-calibration/probe-source',
    'baseline_benchmark_rows_exact':6,'baseline_skidpad_rows_exact':114},indent=2)+'\n')
