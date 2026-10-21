#!/usr/bin/env python3
"""Check native fleet evidence and assemble matched comparison sheets.

Run GRID --fleet-check first. Pillow is needed for image-content checks/sheets.
This verifies usable captures and behavioral evidence; visual review is separate.
"""
import json
import hashlib
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]


def main():
    roster = json.loads((ROOT / 'assets/cars/roster.json').read_text())
    reports = json.loads((ROOT / 'artifacts/fleet-report.json').read_text())
    assert len(reports) == len(roster) == 13, 'A complete native fleet run is required'
    assert [r['id'] for r in reports] == [c['id'] for c in roster]
    for car, report in zip(roster, reports, strict=True):
        asset = ROOT / 'assets' / car['path']
        receipt = json.loads(asset.with_suffix('.json').read_text())
        assert hashlib.sha256(asset.read_bytes()).hexdigest() == receipt['prepared_sha256']
        bundled = ROOT / 'dist/GRID.app/Contents/Resources/assets' / car['path']
        if bundled.exists():
            assert bundled.read_bytes() == asset.read_bytes(), f'Stale bundled asset: {car["id"]}'
        assert report['passed'] and not report['visual_capture_only'], car['id']
        assert report['model_nodes'] == report['animated_nodes'] == car['wheel_nodes']
        assert report['reset_count'] >= 1 and report['grounded_after_reset'] == 4
        assert report['turned'] and report['max_speed_kmh'] > 30
        assert report['brake_end_kmh'] < report['brake_start_kmh'] * 0.5
        assert report['nonblank_captures'] == 4 and report['frames'] >= 300
        assert report['resolution'] == [1920, 1080]
        for view in ('front', 'side', 'rear', 'driving'):
            path = ROOT / f'artifacts/fleet/{car["id"]}-{view}.png'
            image = Image.open(path).convert('RGB')
            assert image.size == (1920, 1080), path
            # Uniform/locked-desktop output cannot establish rendered fidelity.
            assert any(a != b for a, b in image.getextrema()), f'Blank capture: {path}'

    out = ROOT / 'artifacts/fleet'
    for game in dict.fromkeys(c['game_id'] for c in roster[1:]):
        group = [roster[0]] + [c for c in roster[1:] if c['game_id'] == game]
        sheet = Image.new('RGB', (1800, 1260), (19, 26, 33))
        draw = ImageDraw.Draw(sheet)
        for row, car in enumerate(group):
            for col, view in enumerate(('front', 'side', 'rear')):
                image = Image.open(out / f'{car["id"]}-{view}.png').convert('RGB')
                crop = image.crop((560, 340, 1360, 840)).resize((600, 375))
                sheet.paste(crop, (col * 600, row * 420 + 35))
                draw.text((col * 600 + 10, row * 420 + 10), f'{car["name"]} | {view}', fill='white')
        sheet.save(out / f'{game}-comparison.jpg', quality=92)

    overview = Image.new('RGB', (1800, 1600), (19, 26, 33))
    draw = ImageDraw.Draw(overview)
    for i, car in enumerate(roster):
        x, y = (i % 3) * 600, (i // 3) * 320
        image = Image.open(out / f'{car["id"]}-front.png').convert('RGB')
        overview.paste(image.crop((610, 380, 1330, 820)).resize((480, 280)), (x + 60, y + 38))
        draw.text((x + 14, y + 5), car['name'], fill='white')
        draw.text((x + 14, y + 20), car['game'] + (' / add-on' if car['variant'] == 'add-on' else ''), fill=(174, 190, 201))
    overview.save(ROOT / 'artifacts/fleet-overview.jpg', quality=94)
    rows = ['# GRID fleet native validation', '',
            'Thirteen cars: the original 350Z plus two additions from each of six games.', '',
            'Every row passed the same native acceleration/turn/handbrake/braking/reset script, animated every wheel/tire node, and produced four nonblank 1920×1080 captures. Matched static comparisons put the original 350Z on the first row of each game sheet.', '',
            '| Car | Source | Peak km/h | Brake km/h | FPS | p95 ms |',
            '| --- | --- | ---: | ---: | ---: | ---: |']
    for car, report in zip(roster, reports, strict=True):
        rows.append(f'| {car["name"]} | {car["game"]} | {report["max_speed_kmh"]:.1f} | {report["brake_start_kmh"]:.1f} → {report["brake_end_kmh"]:.1f} | {report["average_fps"]:.1f} | {report["p95_frame_ms"]:.2f} |')
    modes = sorted({r['render_target'] for r in reports})
    rows += ['', f'Render targets: {", ".join(modes)}. Offscreen uses the native Bevy scene, materials, camera and HUD; its frame measurements are not a display refresh/window benchmark.', '',
             'Source-era detail varies. GT1 source geometry is much simpler than GT4; preservation and integration quality do not imply equal polygon detail. Paint/glass/metal translation choices are recorded in preparation receipts. Redline additions are labelled add-ons.', '',
             'Visual review of these images remains a separate step; successful screenshot capture alone does not certify visual parity.']
    (ROOT / 'artifacts/fleet-validation.md').write_text('\n'.join(rows) + '\n')
    print('FLEET NATIVE EVIDENCE PASS: 13 drive scripts, all wheel animations, 52 nonblank captures, six baseline comparison sheets')


if __name__ == '__main__':
    main()
