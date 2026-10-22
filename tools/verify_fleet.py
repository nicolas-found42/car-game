#!/usr/bin/env python3
"""Verify one persisted GRID fleet run and build matched comparison sheets.

The game writes `artifacts/runs/<id>/` (manifest, events, report, captures).
This tool never rewrites a previous run; it only reads that directory and
writes generated validation material inside it.

Run GRID --fleet-check first. Pillow is needed for image-content checks/sheets.
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]


def latest_run() -> Path:
    latest = ROOT / 'artifacts/latest'
    if latest.is_file():
        value = latest.read_text().strip()
        path = Path(value)
        if not path.is_absolute():
            path = ROOT / path
        if path.is_dir():
            return path
    runs = [p for p in (ROOT / 'artifacts/runs').glob('*') if (p / 'report.json').is_file()]
    if not runs:
        raise SystemExit('No completed run found. Run: cargo run -- --fleet-check')
    return max(runs, key=lambda p: p.stat().st_mtime)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open('rb') as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: Path):
    return json.loads(path.read_text())


def verify_capture(run: Path, car: dict, report: dict, view: str, baseline: dict | None) -> dict:
    captures = report.get('captures') or {}
    metrics_map = report.get('capture_metrics') or {}
    assert view in captures, f'{car["id"]}: missing capture record for {view}'
    record = captures[view]
    path = run / record['path']
    assert path.is_file(), f'{car["id"]}: missing capture file {path}'
    assert sha256(path) == record['sha256'], f'{car["id"]}: capture hash mismatch {view}'
    image = Image.open(path).convert('RGB')
    assert image.size == (1920, 1080), f'{car["id"]}: capture resolution {view}'
    assert any(a != b for a, b in image.getextrema()), f'{car["id"]}: blank capture {view}'
    metrics = metrics_map.get(view)
    assert metrics is not None, f'{car["id"]}: missing capture metrics {view}'
    assert metrics['nonblank'], f'{car["id"]}: capture metrics marked blank {view}'
    assert metrics['width'] == 1920 and metrics['height'] == 1080
    assert metrics['unique_colors'] >= 2, f'{car["id"]}: no color variation {view}'
    assert metrics['edge_count'] >= 1, f'{car["id"]}: no edges {view}'
    if baseline:
        base = baseline.get(car['id'], {}).get(view)
        if base:
            assert metrics['unique_colors'] >= base['unique_colors'] * 0.5, f'{car["id"]}: color regression {view}'
            assert metrics['luma_stddev'] >= base['luma_stddev'] * 0.5, f'{car["id"]}: contrast regression {view}'
            assert metrics['edge_count'] >= base['edge_count'] * 0.5, f'{car["id"]}: detail regression {view}'
    return metrics


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', type=Path, help='run directory; defaults to artifacts/latest')
    parser.add_argument('--baseline', type=Path, help='optional capture-metrics baseline JSON')
    args = parser.parse_args()

    run = (args.run or latest_run()).resolve()
    manifest = load_json(run / 'manifest.json')
    assert manifest.get('exit', {}).get('code') == 0, f'run exited with {manifest.get("exit")}'
    assert not (run / 'failure.json').exists(), 'run contains failure.json'
    reports = load_json(run / 'report.json')
    roster = load_json(ROOT / 'assets/cars/roster.json')
    assert len(reports) == len(roster) == 13, 'A complete native fleet run is required'
    assert [r['id'] for r in reports] == [c['id'] for c in roster]
    baseline = load_json(args.baseline) if args.baseline else None

    for car, report in zip(roster, reports, strict=True):
        asset = ROOT / 'assets' / car['path']
        receipt = json.loads(asset.with_suffix('.json').read_text())
        assert sha256(asset) == receipt['prepared_sha256']
        if report.get('asset_sha256'):
            assert report['asset_sha256'] == sha256(asset), f'{car["id"]}: report asset hash'
        assert report['passed'] and not report['visual_capture_only'], car['id']
        assert report['model_nodes'] == report['animated_nodes'] == car['wheel_nodes']
        assert not report.get('missing_node_names'), f'{car["id"]}: missing nodes'
        assert report['reset_count'] >= 1 and report['grounded_after_reset'] == 4
        assert report['turned'] and report['max_speed_kmh'] > 30
        assert report['brake_end_kmh'] < report['brake_start_kmh'] * 0.5
        assert report['nonblank_captures'] == 4 and report['frames'] >= 300
        assert len(report.get('frame_times_ms', [])) == report['frames']
        assert report['resolution'] == [1920, 1080]
        for view in ('front', 'side', 'rear', 'driving'):
            verify_capture(run, car, report, view, baseline)

    try:
        from evidence_manifest import build_manifest
        evidence = build_manifest(run)
        assert evidence['report_sha256'] == sha256(run / 'report.json')
    except ImportError:
        evidence = None

    out = run / 'comparisons'
    out.mkdir(exist_ok=True)
    for game in dict.fromkeys(c['game_id'] for c in roster[1:]):
        group = [roster[0]] + [c for c in roster[1:] if c['game_id'] == game]
        sheet = Image.new('RGB', (1800, 1260), (19, 26, 33))
        draw = ImageDraw.Draw(sheet)
        for row, car in enumerate(group):
            for col, view in enumerate(('front', 'side', 'rear')):
                path = run / reports[roster.index(car)]['captures'][view]['path']
                image = Image.open(path).convert('RGB')
                crop = image.crop((560, 340, 1360, 840)).resize((600, 375))
                sheet.paste(crop, (col * 600, row * 420 + 35))
                draw.text((col * 600 + 10, row * 420 + 10), f'{car["name"]} | {view}', fill='white')
        sheet.save(out / f'{game}-comparison.jpg', quality=92)

    overview = Image.new('RGB', (1800, 1600), (19, 26, 33))
    draw = ImageDraw.Draw(overview)
    for index, (car, report) in enumerate(zip(roster, reports, strict=True)):
        x, y = (index % 3) * 600, (index // 3) * 320
        path = run / report['captures']['front']['path']
        image = Image.open(path).convert('RGB')
        overview.paste(image.crop((610, 380, 1330, 820)).resize((480, 280)), (x + 60, y + 38))
        draw.text((x + 14, y + 5), car['name'], fill='white')
        draw.text((x + 14, y + 20), car['game'] + (' / add-on' if car['variant'] == 'add-on' else ''), fill=(174, 190, 201))
    overview.save(run / 'fleet-overview.jpg', quality=94)

    rows = [
        '# GRID fleet native validation', '',
        f'Run: `{manifest["run_id"]}`',
        f'Commit: `{manifest["git"]["sha"]}` ({"dirty" if manifest["git"]["dirty"] else "clean"})',
        f'Binary: `{manifest["binary_sha256"]}`', '',
        'Thirteen cars: the original 350Z plus two additions from each of six games.', '',
        'Every row passed the same native acceleration/turn/handbrake/braking/reset script, animated every wheel/tire node, and produced four nonblank 1920x1080 captures. Matched static comparisons put the original 350Z on the first row of each game sheet.', '',
        '| Car | Source | Peak km/h | Brake km/h | FPS | p95 ms |',
        '| --- | --- | ---: | ---: | ---: | ---: |',
    ]
    for car, report in zip(roster, reports, strict=True):
        rows.append(f'| {car["name"]} | {car["game"]} | {report["max_speed_kmh"]:.1f} | {report["brake_start_kmh"]:.1f} -> {report["brake_end_kmh"]:.1f} | {report["average_fps"]:.1f} | {report["p95_frame_ms"]:.2f} |')
    modes = sorted({r['render_target'] for r in reports})
    rows += [
        '', f'Render targets: {", ".join(modes)}. Offscreen uses the native Bevy scene, materials, camera and HUD; its frame measurements are not a display refresh/window benchmark.', '',
        'Source-era detail varies. GT1 source geometry is much simpler than GT4; preservation and integration quality do not imply equal polygon detail. Paint/glass/metal translation choices are recorded in preparation receipts. Redline additions are labelled add-ons.', '',
        'Capture metrics are recorded per view; visual review of these images remains a separate step.', '',
        'Generated by `python3 tools/verify_fleet.py`; this file is never mixed with manually authored review notes.', '',
    ]
    (run / 'validation.md').write_text('\n'.join(rows) + '\n')
    if evidence:
        print(f'Evidence manifest: {run / "evidence-manifest.json"}')
    print(f'FLEET NATIVE EVIDENCE PASS: {manifest["run_id"]}; 13 drive scripts, all wheel animations, 52 captured views')
    return 0


if __name__ == '__main__':
    sys.exit(main())
