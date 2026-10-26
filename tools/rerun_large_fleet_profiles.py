"""Repeat every common case for explicitly repaired profiles without changing inputs."""
import argparse
from pathlib import Path
from run_large_fleet import execute_car, read, write, sha


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['output','binary-root','definitions','repair']:
        parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args();root=args.output.resolve();root.mkdir(parents=True,exist_ok=True)
    cars=read(args.repair)['repaired_cars'];assert cars and len(cars)==len(set(cars))
    method={'cars':cars,'cases_per_car':5024,'repeats':2,'expected_executions':len(cars)*10048,
            'binary_sha256':sha(args.binary_root/'fleet_stress'),
            'city_binary_sha256':sha(args.binary_root/'fleet_city_batch'),
            'definitions_sha256':sha(args.definitions),'repair_sha256':sha(args.repair)}
    if (root/'method.json').exists():assert read(root/'method.json')==method
    else:write(root/'method.json',method)
    context=(root,args.binary_root.resolve(),args.definitions.resolve())
    results=[execute_car(context,car) for car in cars]
    write(root/'execution-status.json',{'cars':results,'errors':[]})
    print('Repaired profiles rerun through all5024 common cases twice:',cars,flush=True)


if __name__=='__main__':main()
