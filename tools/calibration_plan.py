"""Freeze an explicitly selected production cohort without publishing any cars."""
import itertools
import shutil
from pathlib import Path
from calibration_io import ROOT, digest, require, sha, write
from calibration_protocol import ALL_CASES, scenario
from calibration_io import load

SOURCES = ['tools/handling_protocol.py', 'tools/calibration_protocol.py',
           'docs/verification/scenario-courses/courses.json', 'src/scenario/route.rs',
           'src/city.rs', 'src/city/curve.rs']


def selection(values, allowed, label):
    require(bool(values) and len(values) == len(set(values)), f'empty or duplicate {label}')
    require(set(values) <= set(allowed), f'unknown or inactive {label}: {set(values) - set(allowed)}')


def prepare(destination, binary, cars, cases, repeats):
    roster = load(ROOT / 'assets/cars/roster.json')
    selection(cars, [car['id'] for car in roster], 'cars')
    selection(cases, ALL_CASES, 'cases')
    require(type(repeats) is int and 1 <= repeats <= 20, 'repeats must be 1..20')
    binary = Path(binary).resolve(strict=True)
    destination = Path(destination).resolve()
    destination.mkdir(parents=True, exist_ok=False)
    frozen = destination / 'inputs'
    frozen.mkdir()
    paths = ['assets/cars/roster.json', 'assets/cars/physics.json', *SOURCES]
    bindings = {}
    for number, name in enumerate(paths):
        target = frozen / f'{number:02d}-{Path(name).name}'
        shutil.copyfile(ROOT / name, target)
        bindings[name] = {'path': str(target.relative_to(destination)), 'sha256': sha(target)}
    tasks = make_tasks(destination, cars, cases, repeats)
    plan = {'schema': 1, 'binary_sha256': sha(binary), 'cars': cars, 'cases': cases,
            'repeats': repeats, 'bindings': bindings, 'tasks': tasks,
            'assets': {car: sha(ROOT / 'assets' / next(r['path'] for r in roster if r['id'] == car)) for car in cars},
            'policy': {'reference_physics': True, 'fast_scenario': True, 'fixed_hz': 120, 'seed': 17}}
    plan['cohort_sha256'] = digest([cars, cases, repeats])
    write(destination / 'study.json', plan)
    return plan


def make_tasks(destination, cars, cases, repeats):
    tasks = []
    for number, (car, case, repeat) in enumerate(itertools.product(cars, cases, range(repeats))):
        name = f'inputs/scenario-{number:04d}.json'
        write(destination / name, scenario(car, case))
        tasks.append({'car': car, 'case': case, 'repeat': repeat,
                      'scenario': name, 'scenario_sha256': sha(destination / name)})
    return tasks
