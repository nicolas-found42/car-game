#!/usr/bin/env python3
"""Verify every finalized bundle as the expanded study completes."""
import argparse
import json
from pathlib import Path
import time

from provenance_bundle import verify, captured_entry


def audit(study, output, expected):
    output.parent.mkdir(parents=True,exist_ok=True)
    results=[]; deadline=time.monotonic()+14400
    with output.open('w') as sink:
        while len(results)<expected:
            assert time.monotonic()<deadline,'Evidence audit deadline exceeded'
            index=study/'runs.jsonl'
            text=index.read_text() if index.exists() else ''
            rows=[json.loads(s) for s in text[:text.rfind('\n')+1].splitlines()]
            for row in rows[len(results):]:
                assert row['run'],'Run did not retain an evidence directory'
                result=verify(Path(row['run']))
                assert result['ok'],(row['run'],result['errors'])
                assert result['completeness']['runtime_material_complete']
                assert result['completeness']['rebuild_complete']
                _,path=captured_entry(Path(row['run']),role='effective_config')
                config=json.loads(path.read_text())
                assert config['settings']['fast_scenario']
                assert '--fast-scenario' in config['arguments']
                record={'run':row['run'],**result}
                sink.write(json.dumps(record)+'\n');sink.flush();results.append(record)
                if len(results)%40==0:print(f'Verified {len(results)}/{expected} bundles',flush=True)
            if len(results)<expected:time.sleep(5)
    print(f'All {expected} bundles verified',flush=True)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('study',type=Path);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--expected',type=int,default=1920)
    args=parser.parse_args();audit(args.study,args.output,args.expected)


if __name__=='__main__':main()
