#!/usr/bin/env python3
"""Cross-check exported corpus against reviewed inputs, raw hashes and SQL relationships."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import sys

from build_corpus import ROOT, encoded, native_facts, read, real_cars


def source_pins(value):
    if isinstance(value, dict):
        if {'path', 'sha256', 'bytes'} <= value.keys():
            yield value
        for child in value.values():
            yield from source_pins(child)
    elif isinstance(value, list):
        for child in value:
            yield from source_pins(child)


def verify_pins(games, archive):
    unique = {}
    for pin in source_pins({k: v for k, v in games.items() if k != 'implementation_sources'}):
        assert '..' not in Path(pin['path']).parts and not Path(pin['path']).is_absolute()
        key = pin['path']
        assert key not in unique or unique[key] == pin, 'Conflicting source pins: ' + key
        unique[key] = pin
    for name, pin in unique.items():
        path = archive / name
        assert path.is_file(), name
        data = path.read_bytes()
        assert len(data) == pin['bytes'], 'Source length changed: ' + name
        assert hashlib.sha256(data).hexdigest() == pin['sha256'], 'Source hash changed: ' + name
    for pin in games['implementation_sources']:
        path = ROOT.parents[1] / pin['path']
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != pin['sha256']:
            path = ROOT / 'evidence/source' / (pin['path'] + '.txt')
        assert hashlib.sha256(path.read_bytes()).hexdigest() == pin['sha256'], path
    return len(unique)


def verify_sql(cars, games):
    db = sqlite3.connect(ROOT / 'corpus.sqlite')
    assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    assert not db.execute('PRAGMA foreign_key_check').fetchall()
    assert db.execute('SELECT count(*) FROM car').fetchone()[0] == 50
    assert read(ROOT / 'real-profiles.json')['cars'] == cars
    expected = {code for car in cars for code in car['asset_codes']}
    actual = {row[0] for row in db.execute('SELECT asset_code FROM counterpart')}
    assert actual == expected
    count = 0
    for profile in games['profiles']:
        code = profile['asset_code']
        row = db.execute('SELECT original_json,model_sha256 FROM counterpart WHERE asset_code=?', (code,)).fetchone()
        assert json.loads(row[0]) == profile and row[1] == profile['model']['sha256'], code
        expected_facts = {(path, encoded(value), encoding) for path, value, encoding in native_facts(profile)}
        actual_facts = set(db.execute('SELECT json_path,value_json,encoding FROM game_fact WHERE asset_code=?', (code,)))
        assert actual_facts == expected_facts, 'Game parameters differ: ' + code
        count += len(actual_facts)
    assert count == db.execute('SELECT count(*) FROM game_fact').fetchone()[0]
    db.close()
    return count


def verify_facts(cars):
    facts = 0
    for car in cars:
        text = (ROOT / 'profiles' / (car['id'] + '.md')).read_text()
        assert car['name'] in text
        sources = {s['id']: s for s in car['sources']}
        for fact in car['facts']:
            assert sources[fact['source_id']]['url'] in text, (car['id'], fact)
            assert fact['value'] is not None, (car['id'], fact)
            facts += 1
    return facts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True)
    args = parser.parse_args()
    cars, games = real_cars(), read(ROOT / 'game-profiles.json')
    report = {'status': 'passed', 'real_cars': len(cars), 'game_assets': len(games['profiles']),
              'source_files_hash_checked': verify_pins(games, args.archive),
              'game_parameter_values_cross_checked': verify_sql(cars, games),
              'real_facts_with_resolved_sources': verify_facts(cars),
              'scope': 'Structural consistency, source-file byte identity and export fidelity; research source applicability also requires cited identity notes.'}
    (ROOT / 'evidence/corpus-checks.json').write_text(json.dumps(report, indent=2) + '\n')
    print(encoded(report))


if __name__ == '__main__':
    main()
