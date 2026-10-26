"""Compare timed controls at the scenario-ready epoch, retaining warmup differences."""
import hashlib
import json
from pathlib import Path


def normalized(row):
    run = Path(row['run'])
    summary = json.load(open(run/'scenario-summary.json'))
    ready = summary['readiness']['tick']; ticks = summary['ticks']
    controls = []
    digest = hashlib.sha256()
    with (run/'input.jsonl').open('rb') as source:
        for line in source:
            digest.update(line)
            v = json.loads(line)
            if ready < v['tick'] <= ready+ticks:
                controls.append({'relative_tick': v['tick']-ready,
                                 **{k:v[k] for k in ['controller','throttle','brake','steer','handbrake','reset']}})
    assert len(controls) == ticks
    return ready, controls, digest.hexdigest()


def verify_pair(before, after):
    a, old, before_digest = normalized(before)
    b, new, after_digest = normalized(after)
    assert old == new, (after['car'], after['case'], after['repeat'])
    raw_equal = before_digest == after_digest
    digest = hashlib.sha256(json.dumps(new, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    return {'car': after['car'], 'case': after['case'], 'repeat': after['repeat'],
            'baseline_readiness_tick': a, 'after_readiness_tick': b,
            'baseline_raw_input_sha256': before_digest, 'after_raw_input_sha256': after_digest,
            'whole_raw_input_hash_equal': raw_equal, 'ready_relative_controls_equal': True,
            'normalized_controls_sha256': digest}
