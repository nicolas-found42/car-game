"""Repeat the corrected common slope course on all 300 cars, retaining prior results."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from run_large_fleet import execute_stage, compress_failures, read, write, sha


def run(context, car):
    root=context[0];status=root/car/'complete.json'
    if status.exists():
        result=read(status);assert sha(result['stage']['result'])==result['stage']['sha256'];return result
    assert not (root/car).exists(),f'Incomplete attempt retained at {root/car}'
    result={'car':car,'stage':execute_stage(context,car,'stress',(4000,4500))}
    compress_failures(root/car/'failures');write(status,result)
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['output','binary-root','roster','definitions']:
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--workers',type=int,default=12);args=parser.parse_args()
    assert 1<=args.workers<=18
    root=args.output.resolve();root.mkdir(parents=True,exist_ok=True)
    cars=[r['id'] for r in read(args.roster)];assert len(cars)==len(set(cars))==300
    method={'cars':cars,'case_ids':list(range(4000,4500)),'repeats':2,'expected_executions':300000,
            'roster_sha256':sha(args.roster),'definitions_sha256':sha(args.definitions),
            'binary_sha256':sha(args.binary_root/'fleet_stress'),'coordinator_sha256':sha(__file__)}
    if (root/'method.json').exists():assert read(root/'method.json')==method
    else:write(root/'method.json',method)
    context=(root,args.binary_root.resolve(),args.definitions.resolve());results=[];errors=[]
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures={pool.submit(run,context,c):c for c in cars}
        for future in as_completed(futures):
            try:results.append(future.result())
            except Exception as error:errors.append({'car':futures[future],'error':repr(error)})
            write(root/'progress.json',{'complete_cars':len(results),'errors':errors})
            print(len(results),'/300',futures[future],'errors',len(errors),flush=True)
    write(root/'execution-status.json',{'cars':results,'errors':errors})
    assert not errors and len(results)==300


if __name__=='__main__':main()
