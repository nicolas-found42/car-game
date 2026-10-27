#!/usr/bin/env python3
"""Compile reviewed research and hash-pinned game data into JSON, SQL, CSV and profiles."""
import argparse
from collections import Counter
import csv
import hashlib
import json
from pathlib import Path
import sqlite3

ROOT = Path(__file__).resolve().parents[1]


def read(path):
    return json.loads(path.read_text())


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + '\n')


def encoded(value):
    return json.dumps(value, ensure_ascii=False, allow_nan=False)


def real_cars():
    cars = []
    for group in ('american', 'european', 'japanese'):
        cars.extend(read(ROOT / 'research' / (group + '.json'))['cars'])
    for car in cars:
        for related in car.get('related_assets', []):
            code = related['asset_code']
            if code not in car['asset_codes']:
                car['asset_codes'].append(code)
            car.setdefault('asset_matches', {})[code] = {'status': related['relationship'], 'note': related['note']}
    ids = [car['id'] for car in cars]
    assert len(ids) == len(set(ids)) == 50, 'Expected 50 unique real counterpart IDs'
    for car in cars:
        assert car['asset_codes'] and car['facts'] and car['sources'], car['id']
        sources = {s['id'] for s in car['sources']}
        assert len(sources) == len(car['sources']), car['id']
        for fact in car['facts']:
            assert fact['source_id'] in sources, (car['id'], fact)
            assert 'value' in fact and fact.get('field') and 'unit' in fact, fact
        assert all(s['url'].startswith('https://') for s in car['sources']), car['id']
    return cars


def leaves(value, path='$'):
    if isinstance(value, dict):
        for key, child in value.items():
            yield from leaves(child, path + '.' + key)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from leaves(child, f'{path}[{index}]')
    else:
        yield path, value


def native_facts(profile):
    native = profile['native_profile']
    game = profile['game']
    if game == 'gran-turismo-4':
        for path, value in leaves(native['decoded_dimensions'], '$.decoded_dimensions'):
            yield path, value, 'decoded; see dictionary'
        for row in native['default_part_rows']:
            base = f'$.part[{row["table"]}:{row["row_id"]}]'
            yield base + '.row_hex', row['hex'], 'native_row_bytes_hex'
            for index, field in enumerate(row.get('fields', [])):
                yield base + f'.{field["column"]}[{index}]', field.get('text', field['raw']), field['type']
    elif game == 'redline':
        for index, field in enumerate(native['configuration']):
            yield f'$.configuration[{index}].{field["key"]}[{field["index"]}]', field['value'], f'native; line {field["line"]}'
    elif game == 'midnight-club-los-angeles':
        for resource in native['tuning_resources']:
            for section in resource['tuning']['sections']:
                base = f'$.tuning.{resource["member"]}.slot[{section["slot"]}:{section["class"]}]'
                for field in section['fields']:
                    yield base + f'.offset[{field["offset"]}]', field['value'], field['type'] + '; semantic meaning unresolved'
    elif game == 'midnight-club-3-remix':
        for index, config in enumerate(native['customization_configurations']):
            for field in config['values']:
                yield f'$.CarCfg[{index}].{field["key"]}', field['value'], f'native; line {field["line"]}'
    else:
        data = native.get('specifications', native.get('catalog_parameters', {}))
        for path, value in leaves(data):
            yield path, value, 'native; see dictionary'


def physics_status(profile):
    return {'gran-turismo-4': 'decoded_factory_part_rows_and_raw_parameters',
            'gran-turismo': 'decoded_native_base_specs',
            'redline': 'authored_configuration_semantics_partly_unverified',
            'midnight-club-3-remix': 'customization_values_only_runtime_physics_unresolved',
            'midnight-club-los-angeles': 'typed_tuning_sections_semantic_fields_unresolved',
            'ford-racing-2': 'catalog_values_and_model_geometry_only'}[profile['game']]


def match_info(car, profile):
    # A source-code correspondence alone cannot establish exact year/market/equipment equality.
    for mapping in car.get('asset_mappings', []):
        if mapping['code'] == profile['asset_code']:
            return mapping['match'], mapping.get('reason', car['identity_notes'])
    if car['id'] == 'a16':
        return 'exact_named_model_year_trim', car['identity_notes']
    explicit = car.get('asset_matches', {})
    if isinstance(explicit, dict) and profile['asset_code'] in explicit:
        value = explicit[profile['asset_code']]
        if isinstance(value, dict):
            return value.get('status', 'review_identity_notes'), value.get('note', car['identity_notes'])
    return ('selected_reference' if profile['asset_code'] == car['asset_codes'][0]
            else 'related_counterpart_review_year_trim_market'), car['identity_notes']


def sql_schema(db):
    db.executescript('''
    PRAGMA foreign_keys=ON;
    CREATE TABLE car(id TEXT PRIMARY KEY,name TEXT NOT NULL,year_json TEXT,market TEXT,
      variant TEXT,vehicle_type TEXT,identity_status TEXT,identity_notes TEXT,profile_path TEXT);
    CREATE TABLE source(id TEXT PRIMARY KEY,car_id TEXT REFERENCES car(id),url TEXT NOT NULL,
      title TEXT,accessed TEXT,source_type TEXT,scope TEXT,original_json TEXT NOT NULL);
    CREATE TABLE real_fact(id TEXT PRIMARY KEY,car_id TEXT REFERENCES car(id),field TEXT NOT NULL,
      value_json TEXT NOT NULL,unit TEXT,source_id TEXT REFERENCES source(id),note TEXT,original_json TEXT);
    CREATE TABLE counterpart(asset_code TEXT PRIMARY KEY,car_id TEXT REFERENCES car(id),game TEXT,
      game_name TEXT,catalog_name TEXT,match_status TEXT,match_note TEXT,model_path TEXT,
      model_sha256 TEXT,physics_status TEXT,original_json TEXT);
    CREATE TABLE game_fact(asset_code TEXT REFERENCES counterpart(asset_code),json_path TEXT,
      value_json TEXT,encoding TEXT,PRIMARY KEY(asset_code,json_path));
    CREATE TABLE model_measurement(asset_code TEXT REFERENCES counterpart(asset_code),field TEXT,
      value_json TEXT,unit TEXT,basis TEXT,PRIMARY KEY(asset_code,field));
    CREATE TABLE unknown(car_id TEXT REFERENCES car(id),description TEXT);
    CREATE INDEX real_fact_field ON real_fact(field);
    CREATE INDEX counterpart_car ON counterpart(car_id);
    CREATE INDEX game_fact_path ON game_fact(json_path);
    ''')


def insert_car(db, car):
    db.execute('INSERT INTO car VALUES(?,?,?,?,?,?,?,?,?)',
               (car['id'], car['name'], encoded(car['year']), car['market'], car['variant'],
                car['vehicle_type'], car['identity_status'], car['identity_notes'], f'profiles/{car["id"]}.md'))
    for source in car['sources']:
        db.execute('INSERT INTO source VALUES(?,?,?,?,?,?,?,?)',
                   (source['id'], car['id'], source['url'], source['title'], source['accessed'],
                    source.get('source_type', source.get('kind', 'see_source_notes')), source.get('scope'), encoded(source)))
    for index, fact in enumerate(car['facts']):
        db.execute('INSERT INTO real_fact VALUES(?,?,?,?,?,?,?,?)',
                   (f'{car["id"]}-f{index:03}', car['id'], fact['field'], encoded(fact['value']),
                    fact['unit'], fact['source_id'], fact.get('note'), encoded(fact)))
    for item in car['unknowns']:
        db.execute('INSERT INTO unknown VALUES(?,?)', (car['id'], str(item)))


def insert_game(db, car, profile):
    status, note = match_info(car, profile)
    code = profile['asset_code']
    db.execute('INSERT INTO counterpart VALUES(?,?,?,?,?,?,?,?,?,?,?)',
               (code, car['id'], profile['game'], profile['game_label'], profile['catalog']['name'],
                status, note, profile['model']['path'], profile['model']['sha256'],
                physics_status(profile), encoded(profile)))
    for path, value, encoding in native_facts(profile):
        db.execute('INSERT INTO game_fact VALUES(?,?,?,?)', (code, path, encoded(value), encoding))
    for field in ('default_scene_aabb', 'default_scene_extent', 'wheel_pivots', 'wheelbase_from_pivots_m',
                  'track_at_negative_z_m', 'track_at_positive_z_m'):
        if field in profile['geometry']:
            db.execute('INSERT INTO model_measurement VALUES(?,?,?,?,?)',
                       (code, field, encoded(profile['geometry'][field]), 'm', 'static_exporter_space; not runtime physics'))


def export_csv(db, table):
    result = db.execute('SELECT * FROM ' + table)
    with (ROOT / (table + '.csv')).open('w', newline='') as stream:
        writer = csv.writer(stream)
        writer.writerow([column[0] for column in result.description])
        writer.writerows(result)


def cell(value):
    text = str(value) if isinstance(value, str) else encoded(value)
    return text.replace('|', '\\|').replace('\n', ' ')


def render_game(car, profile):
    lines = []
    code = profile['asset_code']
    status, note = match_info(car, profile)
    lines += [f'### {profile["game_label"]}: {profile["catalog"]["name"]}', '',
              f'Asset: `{code}` · Relationship: `{status}`', '', note, '',
              f'Model: `{profile["model"]["path"]}`', '', f'SHA-256: `{profile["model"]["sha256"]}`', '',
              f'Physics coverage: `{physics_status(profile)}`.', '',
              'Full raw fields, provenance and geometry: [game-profiles.json](../game-profiles.json) '
              f'(find `{code}`), or query `counterpart.original_json` in [corpus.sqlite](../corpus.sqlite).', '',
              '| Static model measurement | Value (metres) |', '|---|---|']
    for key in ('default_scene_extent', 'wheelbase_from_pivots_m', 'track_at_negative_z_m', 'track_at_positive_z_m'):
        if key in profile['geometry']:
            lines.append(f'| {key} | {cell(profile["geometry"][key])} |')
    native = profile['native_profile']
    if profile['game'] == 'gran-turismo-4':
        lines += ['', 'Native default-part parameters (raw encodings; units require the [dictionary](../DATA-DICTIONARY.md)):', '',
                  '| Part / row | Parameter | Value |', '|---|---|---|']
        for row in native['default_part_rows']:
            for field in row.get('fields', []):
                lines.append(f'| {row["table"]} / {row["row_id"]} | {field["column"]} | {cell(field.get("text", field["raw"]))} |')
    if profile['game'] == 'gran-turismo':
        lines += ['', '| Native base specification | Value |', '|---|---|']
        lines += [f'| {k} | {cell(v)} |' for k, v in native['specifications'].items()]
    if profile['game'] == 'redline':
        lines += ['', 'Active authored configuration; source line numbers are retained:', '',
                  '| Key | Index | Value | Line |', '|---|---|---|---|']
        lines += [f'| {v["key"]} | {v["index"]} | {cell(v["value"])} | {v["line"]} |' for v in native['configuration']]
    lines += ['', *[f'- {limit}' for limit in native.get('limits', [])], '']
    return lines


def render_profile(car, profiles):
    lines = [f'# {car["name"]}', '', f'ID: `{car["id"]}` · Year: {cell(car["year"])} · Market: {car["market"]}',
             '', car['variant'], '', f'Identity status: **{car["identity_status"]}**. {car["identity_notes"]}',
             '', '## Real-life reference', '', '| Parameter | Value | Unit | Evidence / qualification |',
             '|---|---|---|---|']
    sources = {s['id']: s for s in car['sources']}
    for fact in car['facts']:
        source = sources[fact['source_id']]
        lines.append(f'| {cell(fact["field"])} | {cell(fact["value"])} | {cell(fact["unit"])} | '
                     f'[{source["id"]}]({source["url"]}) {cell(fact.get("note", ""))} |')
    lines += ['', '## Game counterparts', '', 'Each counterpart remains separate. Additional models may differ in year, trim, market or namespace.', '']
    for profile in profiles:
        lines += render_game(car, profile)
    lines += ['## Missing or unresolved', '', *[f'- {item}' for item in car['unknowns']], '', '## Sources', '']
    for source in car['sources']:
        lines += [f'- **{source["id"]}**: [{source["title"]}]({source["url"]}); accessed {source["accessed"]}. '
                  f'{source.get("source_type", source.get("kind", "See research notes for authorship."))} {source.get("scope", "")}']
    return '\n'.join(lines) + '\n'


def build(cars, games):
    by_code = {p['asset_code']: p for p in games['profiles']}
    expected = [code for car in cars for code in car['asset_codes']]
    assert len(expected) == len(set(expected)), 'An asset was assigned to more than one real counterpart'
    assert set(expected) == set(by_code), 'Game selection does not match the 50 profiles'
    database = ROOT / 'corpus.sqlite'
    if database.exists():
        database.unlink()
    db = sqlite3.connect(database)
    sql_schema(db)
    for car in cars:
        insert_car(db, car)
        profiles = [by_code[code] for code in car['asset_codes']]
        for profile in profiles:
            insert_game(db, car, profile)
        path = ROOT / 'profiles' / (car['id'] + '.md')
        path.parent.mkdir(exist_ok=True)
        path.write_text(render_profile(car, profiles))
    assert db.execute('PRAGMA foreign_key_check').fetchall() == []
    assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    db.commit()
    tables = ('car', 'source', 'real_fact', 'counterpart', 'game_fact', 'model_measurement', 'unknown')
    counts = {table: db.execute('SELECT count(*) FROM ' + table).fetchone()[0] for table in tables}
    for table in tables:
        export_csv(db, table)
    db.close()
    write(ROOT / 'real-profiles.json', {'schema': 'real-vehicle-reference/v1', 'cars': cars})
    write(ROOT / 'validation.json', {'counts': counts, 'foreign_key_check': 'passed', 'sqlite_integrity_check': 'ok',
          'unique_real_counterparts': len(cars), 'game_counts': dict(Counter(p['game'] for p in games['profiles'])),
          'multi_game_real_counterparts': sum(len({by_code[c]['game'] for c in car['asset_codes']}) > 1 for car in cars),
          'nonempty_sourced_real_profiles': sum(bool(c['facts']) and bool(c['sources']) for c in cars),
          'runtime_physics_changes': False})
    print(encoded(counts))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--selection-only', action='store_true')
    args = parser.parse_args()
    cars = real_cars()
    if args.selection_only:
        write(ROOT / 'selection.json', {'cars': [{'id': c['id'], 'name': c['name'], 'asset_codes': c['asset_codes']} for c in cars]})
    else:
        build(cars, read(ROOT / 'game-profiles.json'))


if __name__ == '__main__':
    main()
