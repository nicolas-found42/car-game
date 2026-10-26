#!/usr/bin/env python3
"""Run the preregistered matched protocol using the existing production CLI."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

from handling_protocol import CASES, scenario

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def prepare(study, repeats, executable):
    study.mkdir(parents=True, exist_ok=False)
    (study / 'scenarios').mkdir()
    cars = json.loads((ROOT / 'assets/cars/roster.json').read_text())
    tasks = []
    for case in CASES:
        for car in cars:
            path = study / 'scenarios' / f'{case}-{car["id"]}.json'
            path.write_text(json.dumps(scenario(car['id'], case), indent=2) + '\n')
            for repeat in range(1, repeats + 1):
                tasks.append({'car': car['id'], 'case': case, 'repeat': repeat,
                              'scenario': str(path.relative_to(ROOT)),
                              'scenario_sha256': sha(path)})
    config = {'schema': 1, 'commit': subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'binary': str(executable), 'binary_sha256': sha(executable),
        'roster_sha256': sha(ROOT / 'assets/cars/roster.json'),
        'repeats': repeats, 'cases': CASES, 'cars': [c['id'] for c in cars],
        'backend': 'disabled', 'fixed_hz': 120, 'seed': 17, 'tasks': tasks}
    (study / 'study.json').write_text(json.dumps(config, indent=2) + '\n')
    return tasks


def run_one(study, executable, task):
    tag = f'{task["case"]}-{task["car"]}-r{task["repeat"]}'
    destination = study / 'runs' / tag
    log = study / f'{tag}.log'
    args = [str(executable), '--render-disabled', '--scenario', task['scenario'],
            '--artifact-dir', str(destination)]
    start = time.monotonic()
    with log.open('w') as output:
        result = subprocess.run(args, cwd=ROOT, env={**os.environ, 'CAR_GAME_MUTE': '1'},
                                stdout=output, stderr=subprocess.STDOUT, timeout=100)
    runs = list(destination.glob('*/manifest.json'))
    return {**task, 'exit_code': result.returncode, 'command': args,
            'elapsed_s': time.monotonic() - start,
            'run': str(runs[0].parent.relative_to(ROOT)) if len(runs) == 1 else None,
            'launcher_log': str(log.relative_to(ROOT))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/debug/car-game')
    parser.add_argument('--repeats', type=int, default=2)
    parser.add_argument('--workers', type=int, default=4)
    args = parser.parse_args()
    if args.repeats < 2 or not 1 <= args.workers <= 4:
        parser.error('require at least two repeats and one to four workers')
    study, binary = args.output.resolve(), args.binary.resolve()
    tasks = prepare(study, args.repeats, binary)
    failures = 0
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = [pool.submit(run_one, study, binary, task) for task in tasks]
        with (study / 'runs.jsonl').open('a') as index:
            for count, future in enumerate(as_completed(futures), 1):
                row = future.result()
                index.write(json.dumps(row) + '\n')
                index.flush()
                failures += row['exit_code'] != 0
                print(f'{count}/{len(tasks)} {row["case"]} {row["car"]} '
                      f'r{row["repeat"]}: exit {row["exit_code"]}', flush=True)
    if failures:
        raise SystemExit(f'{failures} runs failed; evidence retained')


if __name__ == '__main__':
    main()
