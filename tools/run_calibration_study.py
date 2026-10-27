#!/usr/bin/env python3
"""Run an explicit calibration matrix through the recorded production executable."""
import argparse
import os
from pathlib import Path
import subprocess
from calibration_io import ROOT, sha, write
from calibration_plan import prepare
from calibration_protocol import ALL_CASES


def run_one(study, binary, task, number):
    folder = study / 'runs' / f'{number:04d}'
    folder.mkdir(parents=True)
    command = [str(binary), '--render-disabled', '--fast-scenario', '--reference-physics',
               '--scenario', str(study / task['scenario']), '--artifact-dir', str(folder),
               '--recorder-queue-bytes', '33554432', '--recorder-queue-records', '32768']
    with (folder / 'process.log').open('w') as log:
        try:
            result = subprocess.run(command, cwd=ROOT, env={**os.environ, 'CAR_GAME_MUTE': '1'},
                                    stdout=log, stderr=subprocess.STDOUT, timeout=180, check=False)
            code = result.returncode
        except subprocess.TimeoutExpired:
            code = 'timeout'
        except OSError as error:
            log.write(str(error) + '\n')
            code = 'launch_error'
    runs = sorted(folder.glob('*/manifest.json'))
    return {**task, 'exit_code': code, 'log': str((folder / 'process.log').relative_to(study)),
            'run': str(runs[0].parent.relative_to(study)) if len(runs) == 1 else None}


def execute(study, binary, plan):
    results = []
    for number, task in enumerate(plan['tasks']):
        if sha(binary) != plan['binary_sha256']:
            raise ValueError('executable changed during study')
        results.append(run_one(study, binary, task, number))
        write(study / 'runs.json', results)
        print(f"{number + 1}/{len(plan['tasks'])}: {task['car']} {task['case']}: {results[-1]['exit_code']}", flush=True)
    return all(row['exit_code'] == 0 and row['run'] for row in results)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--cars', nargs='+', required=True)
    parser.add_argument('--cases', nargs='+', choices=ALL_CASES, required=True)
    parser.add_argument('--repeats', type=int, default=2)
    args = parser.parse_args()
    plan = prepare(args.destination, args.binary, args.cars, args.cases, args.repeats)
    return 0 if execute(args.destination.resolve(), args.binary.resolve(), plan) else 1


if __name__ == '__main__':
    raise SystemExit(main())
