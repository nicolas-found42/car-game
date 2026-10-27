"""Resolve hashes and asynchronous capture links through recorded ticks/frames."""
import hashlib
import json
from pathlib import Path
from trace_integrity import rows


def capture_hash(run, capture, findings, name):
    relative = capture.get('path')
    target = (Path(run) / str(relative)).resolve()
    if not target.is_relative_to(Path(run).resolve()):
        findings.add('capture_path_invalid', name, path=relative)
        return
    digest = hashlib.sha256(target.read_bytes()).hexdigest() if target.is_file() else None
    if digest is None or digest != capture.get('sha256'):
        findings.add('capture_hash_mismatch', name, path=relative)
    if capture.get('metrics', {}).get('nonblank') is not True:
        findings.add('capture_invalid', name, path=relative)


def capture_events(run, findings):
    requested, validated, frames = {}, {}, {}
    for line, record in rows(run, 'events.jsonl', findings):
        kind, fields = record.get('kind'), record.get('fields', {})
        key = (fields.get('car'), fields.get('view'))
        if kind == 'simulation.completed':
            frames[(record.get('tick'), record.get('actor_id'))] = record.get('render_frame')
        if kind not in ('screenshot.requested', 'screenshot.validated'):
            continue
        destination = requested if kind == 'screenshot.requested' else validated
        if key in destination:
            findings.add('capture_link_duplicate', 'events.jsonl', line=line)
        destination[key] = record
    return requested, validated, frames


def relative_capture_path(event):
    path = Path(str(event.get('fields', {}).get('path')))
    if '..' in path.parts:
        return None
    if path.parts and path.parts[0] == 'captures':
        return path.as_posix()
    run_id = event.get('run_id')
    if run_id in path.parts:
        return Path(*path.parts[path.parts.index(run_id) + 1:]).as_posix()
    return None


def capture_link(key, capture, events, findings):
    requested, validated, frames = events
    request, result = requested.get(key), validated.get(key)
    if request is None or result is None:
        findings.add('capture_link_missing', 'events.jsonl', car=key[0], view=key[1])
        return
    actor = request.get('actor_id')
    ticks = [request.get('tick'), result.get('tick')]
    render_frames = [request.get('render_frame'), result.get('render_frame')]
    valid = all(type(x) is int and x >= 0 for x in ticks + render_frames)
    valid = valid and ticks[0] <= ticks[1] and render_frames[0] <= render_frames[1]
    valid = valid and actor == result.get('actor_id') == 'player-vehicle'
    valid = valid and all(frames.get((t, actor), -1) <= f and (t, actor) in frames
                          for t, f in zip(ticks, render_frames))
    names = [relative_capture_path(row) for row in (request, result)]
    valid = valid and names[0] == names[1] == capture.get('path')
    valid = valid and result.get('fields', {}).get('metrics') == capture.get('metrics')
    if not valid:
        findings.add('capture_link_mismatch', 'events.jsonl', car=key[0], view=key[1])


def check_reports(run, manifest, findings):
    path = Path(run) / 'report.json'
    if not path.is_file():
        if manifest.get('mode') in ('fleet', 'smoke'):
            findings.add('report_unavailable', 'report.json')
        return
    reports = json.loads(path.read_text())
    if not isinstance(reports, list) or not reports:
        findings.add('report_incomplete', 'report.json')
        return
    events = capture_events(run, findings)
    seen = set()
    for report in reports:
        captures = report.get('captures', {})
        required = {'front', 'side', 'rear'}
        if not report.get('visual_capture_only'):
            required.add('driving')
        if not report.get('passed') or set(captures) != required:
            findings.add('report_incomplete', 'report.json', car=report.get('id'))
        for view, capture in captures.items():
            key = (report.get('id'), view)
            if key in seen:
                findings.add('capture_link_duplicate', 'report.json', car=key[0], view=view)
            seen.add(key)
            capture_hash(run, capture, findings, 'report.json')
            capture_link(key, capture, events, findings)
    if set(events[0]) != seen or set(events[1]) != seen:
        findings.add('capture_link_mismatch', 'report.json')


def scenario_complete(summary, expected, results):
    valid = (bool(expected) and summary.get('passed') is True and not summary.get('failure')
             and summary.get('assertions_declared') == summary.get('assertions_executed')
             == summary.get('assertions_passed') == len(expected) == len(results)
             and all(r.get('passed') is True for r in results)
             and [r.get('assertion') for r in results] == expected)
    return valid


def check_scenario(run, findings, completed):
    root = Path(run)
    try:
        scenario = json.loads((root / 'scenario.json').read_text())['definition']
        summary = json.loads((root / 'scenario-summary.json').read_text())
    except (OSError, ValueError, KeyError, TypeError):
        findings.add('scenario_unavailable', 'scenario-summary.json')
        return
    results = summary.get('results', [])
    expected = scenario.get('assertions', [])
    valid = scenario_complete(summary, expected, results)
    finished = [r for _, r in rows(run, 'events.jsonl', findings) if r.get('kind') == 'scenario.finished']
    if not valid or len(finished) != 1 or finished[0].get('fields') != summary:
        findings.add('scenario_incomplete', 'scenario-summary.json')
    if summary.get('completed_tick') != completed:
        findings.add('scenario_tick_mismatch', 'scenario-summary.json')
    capture = summary.get('capture')
    if scenario.get('required_capture') and not capture:
        findings.add('capture_link_missing', 'scenario-summary.json')
    if capture:
        capture_hash(run, capture, findings, 'scenario-summary.json')
        scenario_capture_link(run, capture, summary, findings)


def scenario_capture_link(run, capture, summary, findings):
    requested, validated, frames = {}, {}, {}
    for _, record in rows(run, 'events.jsonl', findings):
        kind = record.get('kind')
        if kind == 'simulation.completed':
            frames[(record.get('tick'), record.get('actor_id'))] = record.get('render_frame')
        if kind in ('scenario.capture_requested', 'scenario.capture_validated'):
            destination = requested if kind.endswith('requested') else validated
            if ('scenario', 'capture') in destination:
                findings.add('capture_link_duplicate', 'events.jsonl')
            record['fields'].update(car='scenario', view='capture')
            destination[('scenario', 'capture')] = record
    capture_link(('scenario', 'capture'), capture, (requested, validated, frames), findings)


def check_captures(run, manifest, findings, completed):
    try:
        check_reports(run, manifest, findings)
        if manifest.get('mode') == 'scenario':
            check_scenario(run, findings, completed)
    except (OSError, ValueError, AttributeError, TypeError):
        findings.add('report_malformed', 'report.json')
