#!/usr/bin/env python3
"""Run all declared car/case/repeat identities, retaining failures and deduplicated immutable objects."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import json
import os
from pathlib import Path
import subprocess
import threading
import time

from fleet40_protocol import ALL_CASES, ROOT, scenario
from run_handling_study import sha

LOCK = threading.Lock()


def compact(run, cache):
    cache.mkdir(parents=True,exist_ok=True)
    with LOCK:
        paths = list((run/'objects/sha256').glob('*'))
        assert paths, 'Finalized evidence has no immutable objects'
        for path in paths:
            assert sha(path) == path.name, 'Invalid provenance object before deduplication'
            canonical = cache/path.name
            if not canonical.exists():
                os.link(path,canonical)
                canonical.chmod(0o444)
            else:
                assert sha(canonical) == path.name, 'Shared evidence object changed'
                temporary = path.with_suffix('.link')
                os.link(canonical,temporary)
                os.replace(temporary,path)


def prepare(out, binary, cars, cases, repeats):
    out.mkdir(parents=True,exist_ok=False); (out/'scenarios').mkdir()
    tasks = []
    for case in cases:
        for car in cars:
            path = out/'scenarios'/f'{case}-{car}.json'
            path.write_text(json.dumps(scenario(car,case),indent=2)+'\n')
            tasks += [{'car':car,'case':case,'repeat':r,'scenario':str(path.relative_to(ROOT)),
                       'scenario_sha256':sha(path)} for r in range(1,repeats+1)]
    config = {'schema':1,'binary':str(binary),'binary_sha256':sha(binary),'cars':cars,'cases':cases,
              'repeats':repeats,'fast_scenario':True,'fixed_hz':120,'seed':17,'tasks':tasks,
              'roster_sha256':sha(ROOT/'assets/cars/roster.json'),
              'profile_sha256':sha(ROOT/'assets/cars/physics.json'),
              'courses_sha256':sha(ROOT/'docs/verification/fleet-40/courses.json')}
    (out/'study.json').write_text(json.dumps(config,indent=2)+'\n')
    return tasks


def run_one(out, binary, task, fast=True, recorder_options=None):
    tag = f'{task["case"]}-{task["car"]}-r{task["repeat"]}'
    destination = out/'runs'/tag; log = out/f'{tag}.log'
    args = [str(binary),'--render-disabled','--scenario',task['scenario'],'--artifact-dir',str(destination)]
    if fast:
        args += ['--fast-scenario']
    args += recorder_options or []
    start = time.monotonic()
    with log.open('w') as stream:
        result = subprocess.run(args,cwd=ROOT,env={**os.environ,'CAR_GAME_MUTE':'1'},
                                stdout=stream,stderr=subprocess.STDOUT,timeout=180)
    runs = list(destination.glob('*/manifest.json'))
    run = runs[0].parent if len(runs)==1 else None
    if run and (run/'evidence-manifest.json').exists():
        compact(run,out/'shared-objects')
    return {**task,'exit_code':result.returncode,'run':str(run.relative_to(ROOT)) if run else None,
            'elapsed_s':time.monotonic()-start,'command':args,'log':str(log.relative_to(ROOT))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--binary',type=Path,default=ROOT/'target/release/car-game.recorded')
    parser.add_argument('--cars',nargs='+'); parser.add_argument('--cases',nargs='+',choices=ALL_CASES)
    parser.add_argument('--repeats',type=int,default=2); parser.add_argument('--workers',type=int,default=4)
    parser.add_argument('--recorder-queue-bytes', type=int)
    parser.add_argument('--recorder-queue-records', type=int)
    args = parser.parse_args(); out=args.output.resolve(); binary=args.binary.resolve()
    roster = json.loads((ROOT/'assets/cars/roster.json').read_text())
    cars=args.cars or [c['id'] for c in roster]; cases=args.cases or ALL_CASES
    assert set(cars) <= {c['id'] for c in roster} and args.repeats>=1 and 1<=args.workers<=4
    tasks=prepare(out,binary,cars,cases,args.repeats); failures=0
    options=[]
    for name in ['recorder_queue_bytes','recorder_queue_records']:
        value=getattr(args,name)
        if value is not None:
            assert value > 0
            options += ['--'+name.replace('_','-'),str(value)]
    config=json.loads((out/'study.json').read_text()); config['recorder_options']=options
    (out/'study.json').write_text(json.dumps(config,indent=2)+'\n')
    with ThreadPoolExecutor(max_workers=args.workers) as pool, (out/'runs.jsonl').open('a') as index:
        futures=[pool.submit(run_one,out,binary,t,recorder_options=options) for t in tasks]
        for n,future in enumerate(as_completed(futures),1):
            row=future.result(); index.write(json.dumps(row)+'\n'); index.flush()
            failures += row['exit_code']!=0
            print(f'{n}/{len(tasks)} {row["case"]} {row["car"]} r{row["repeat"]}: exit {row["exit_code"]}',flush=True)
    if failures:
        raise SystemExit(f'{failures} runs failed; retained all evidence')


if __name__ == '__main__':
    main()
