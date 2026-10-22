"""Health/lifecycle checks shared by the existing run verifier; trace semantics belong to #86."""
import json
from pathlib import Path

STREAMS = {'events', 'commands', 'physics', 'input', 'log'}
COUNTERS = ('produced', 'accepted', 'written', 'durable_seq', 'queued_bytes',
            'queued_records', 'last_durable_tick', 'completed_tick', 'dropped')


def inspect_health(run, manifest):
    base = {'schema': 1, 'run_id': manifest.get('run_id'), 'ok': False,
            'status': 'unknown', 'behavioral_exit': manifest.get('exit'), 'errors': []}
    declaration = manifest.get('recorder_health')
    if not isinstance(declaration, dict) or declaration.get('schema') != 1:
        return failure(base, 'legacy_health_unknown')
    path = Path(run) / 'recorder-health.json'
    try:
        health = json.loads(path.read_text())
    except (OSError, ValueError):
        return failure(base, 'recorder_health_unavailable')
    if not valid_schema(health, manifest):
        return failure(base, 'invalid_recorder_health')
    base.update(last_durable_tick=health['last_durable_tick'], completed_tick=health['completed_tick'],
                losses=health['losses'], phase=health['phase'])
    if health['failed'] or health['errors'] or health['dropped'] or health['losses'] or health['loss_ranges_truncated'] or health.get('loss_ranges_unknown', False):
        base['status'] = 'degraded'
        return failure(base, 'recorder_evidence_loss', recorded_errors=health['errors'])
    if health['phase'] != 'finalized' or not manifest.get('ended_at') or not isinstance(manifest.get('exit'), dict):
        return failure(base, 'recorder_unclean_exit', termination_cause='unknown')
    if not coherent_finalization(health):
        base['status'] = 'incomplete'
        return failure(base, 'recorder_durability_incomplete')
    base.update(ok=True, status='healthy')
    return base


def failure(result, code, **fields):
    result['errors'].append({'code': code, **fields})
    return result


def natural(value):
    return type(value) is int and value >= 0


def valid_schema(health, manifest):
    if not isinstance(health, dict) or type(health.get('schema')) is not int or health['schema'] != 1:
        return False
    if health.get('run_id') != manifest.get('run_id') or type(health.get('failed')) is not bool:
        return False
    if not all(natural(health.get(key)) for key in COUNTERS):
        return False
    if not isinstance(health.get('errors'), list) or not isinstance(health.get('losses'), list):
        return False
    if type(health.get('loss_ranges_truncated')) is not bool or not isinstance(health.get('phase'), str):
        return False
    if type(health.get('loss_ranges_unknown', False)) is not bool:
        return False
    return valid_streams(health.get('streams'))


def valid_streams(streams):
    if not isinstance(streams, dict) or set(streams) != STREAMS:
        return False
    return all(isinstance(value, dict) and all(natural(value.get(key)) for key in
               ('accepted', 'written', 'written_seq', 'flushed_seq', 'synced_seq'))
               for value in streams.values())


def coherent_finalization(health):
    if not (health['produced'] == health['accepted'] == health['written'] == health['durable_seq']):
        return False
    if health['queued_bytes'] or health['queued_records'] or health['last_durable_tick'] != health['completed_tick']:
        return False
    return all(stream['accepted'] == stream['written'] and
               stream['written_seq'] == stream['flushed_seq'] == stream['synced_seq'] <= health['durable_seq']
               for stream in health['streams'].values())
