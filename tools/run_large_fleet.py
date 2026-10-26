"""Execute a frozen, identical 5,024-case matrix twice on every listed car."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import time


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


def execute_stage(context, car, stage, bounds=None):
    root, binaries, city_manifest = context
    prefix = root / car / stage
    prefix.parent.mkdir(parents=True, exist_ok=True)
    failures = prefix.parent / 'failures'
    command = ([str(binaries / 'fleet_stress'), '--car', car, '--failures', str(failures)]
               if stage == 'stress' else [str(binaries / 'fleet_city_batch'), car, str(city_manifest), str(failures)])
    if bounds is not None:
        assert stage == 'stress'
        command += ['--start', str(bounds[0]), '--end', str(bounds[1])]
    rows, failed, different, seen = 0, 0, 0, set()
    started = time.monotonic()
    with prefix.with_suffix('.jsonl.gz').open('wb') as raw, gzip.GzipFile(fileobj=raw, mode='wb', mtime=0, compresslevel=1) as sink:
        with prefix.with_suffix('.log').open('w') as log:
            child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=log, text=True)
            for line in child.stdout:
                row = json.loads(line)
                assert row['car'] == car and row['repeats'] == 2 and row['case'] not in seen
                seen.add(row['case']); rows += 1
                different += row['repeat_hashes'][0] != row['repeat_hashes'][1]
                failed += (any(row['failures']) if stage == 'stress'
                           else not row['summary']['passed'] or not row['repeat_summary_equal'])
                sink.write(line.encode())
                if rows % 100 == 0:
                    sink.flush(); raw.flush()
                    write(prefix.with_suffix('.progress.json'), {'pairs': rows, 'executions': rows * 2,
                          'failed_pairs': failed, 'different_repeat_pairs': different, 'latest_case': row['case']})
            code = child.wait()
    expected = set(range(*(bounds or (0, 5000)))) if stage == 'stress' else {r['case'] for r in read(city_manifest)}
    result = {'stage': stage, 'exit_code': code, 'pairs': rows, 'executions': rows * 2,
              'complete_coverage': seen == expected, 'failed_pairs': failed, 'different_repeat_pairs': different,
              'elapsed_s': time.monotonic() - started, 'command': command,
              'result': str(prefix.with_suffix('.jsonl.gz')), 'sha256': sha(prefix.with_suffix('.jsonl.gz'))}
    write(prefix.with_suffix('.receipt.json'), result)
    assert code == 0 and seen == expected, result
    return result


def compress_failures(root):
    receipts = []
    for path in root.glob('*.f32le'):
        digest = sha(path); output = path.with_suffix('.f32le.gz')
        with path.open('rb') as source, output.open('wb') as raw, gzip.GzipFile(fileobj=raw, mode='wb', mtime=0, compresslevel=1) as sink:
            for block in iter(lambda: source.read(1024 * 1024), b''):
                sink.write(block)
        with gzip.open(output, 'rb') as stream:
            assert hashlib.file_digest(stream, 'sha256').hexdigest() == digest
        receipts.append({'original_path': str(path), 'path': str(output), 'raw_sha256': digest,
                         'sha256': sha(output), 'frame_fields': 41, 'encoding': 'little-endian float32'})
        path.unlink()
    write(root.parent / 'failure-traces.json', receipts)


def execute_car(context, car):
    root = context[0]
    status = root / car / 'complete.json'
    if status.exists():
        result = read(status)
        assert all(sha(r['result']) == r['sha256'] for r in result['stages'])
        return result
    assert not (root / car).exists(), f'Partial attempt retained at {root / car}; choose a fresh output or explicitly recover it.'
    result = {'car': car, 'stages': [execute_stage(context, car, s) for s in ['city', 'stress']]}
    compress_failures(root / car / 'failures')
    write(status, result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--binary-root', type=Path, required=True)
    parser.add_argument('--definitions', type=Path, required=True)
    parser.add_argument('--roster', type=Path, required=True)
    parser.add_argument('--stress-definitions', type=Path, required=True)
    parser.add_argument('--workers', type=int, default=12)
    args = parser.parse_args(); assert 1 <= args.workers <= 18
    root = args.output.resolve(); root.mkdir(parents=True, exist_ok=True)
    cars = [r['id'] for r in read(args.roster)]; assert len(cars) == len(set(cars)) == 300
    context = (root, args.binary_root.resolve(), args.definitions.resolve())
    method = {'cars': cars, 'new_cases_per_car': 5000, 'original_cases_per_car': 24,
              'repeats': 2, 'expected_executions': 3014400, 'workers': args.workers,
              'definitions_sha256': sha(args.definitions), 'stress_definitions_sha256': sha(args.stress_definitions),
              'coordinator_sha256': sha(__file__), 'roster_sha256': sha(args.roster),
              'binaries': {name: sha(context[1] / name) for name in ['fleet_stress', 'fleet_city_batch']}}
    if (root / 'method.json').exists():
        assert read(root / 'method.json') == method
    else:
        write(root / 'method.json', method)
    results, errors = [], []
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = {pool.submit(execute_car, context, c): c for c in cars}
        for future in as_completed(futures):
            try:
                results.append(future.result())
            except Exception as error:
                errors.append({'car': futures[future], 'error': repr(error)})
            write(root / 'progress.json', {'complete_cars': len(results), 'errors': errors,
                                          'executions': sum(s['executions'] for r in results for s in r['stages'])})
            print(len(results), '/300', futures[future], 'errors', len(errors), flush=True)
    write(root / 'execution-status.json', {'cars': results, 'errors': errors})
    assert not errors and len(results) == 300


if __name__ == '__main__':
    main()
