#!/usr/bin/env python3
"""Read-only, hash-pinned game specifications and model measurements for the corpus."""
import argparse
from collections import defaultdict
import hashlib
import importlib
import itertools
import json
from pathlib import Path
import re
import struct
import sys
import zipfile

HERE = Path(__file__).resolve().parents[1]
TYPES = {'Byte': 'B', 'SByte': 'b', 'Bool': '?', 'Short': 'h', 'UShort': 'H', 'Int': 'i', 'String': 'i', 'Key': 'ii'}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def pin(path, root):
    return {'path': str(Path(path).relative_to(root)), 'sha256': digest(path), 'bytes': Path(path).stat().st_size}


def save(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False, allow_nan=False) + '\n')


def decode_row(row, columns, text):
    fmt = '<' + ''.join(TYPES[x['type']] for x in columns)
    if struct.calcsize(fmt) != len(row):
        raise ValueError(f'Schema/row length mismatch: {len(row)} != {struct.calcsize(fmt)}')
    result = []
    values = iter(struct.unpack(fmt, row))
    for col in columns:
        value = next(values)
        if col['type'] == 'Key':
            value = {'row_id': value, 'table_id': next(values)}
        item = {'column': col['name'], 'type': col['type'], 'raw': value}
        if col['type'] == 'String':
            if not 0 <= value < len(text):
                raise ValueError('String reference out of bounds')
            item['text'] = text[value]
        result.append(item)
    return result


class GT4:
    def __init__(self, root):
        self.folder = root / 'games/gran-turismo-4/cars/provenance/specdb/GT4_EU2560'
        module = importlib.import_module('gt4_specdb')
        self.tables = {}
        for path in sorted(self.folder.glob('*.dbt')):
            if struct.unpack_from('<I', path.read_bytes(), 8)[0] == 0:
                continue
            try:
                table = module.Table(self.folder, path.stem)
                self.tables[table.table_id] = table
            except (KeyError, ValueError):
                raise
        self.by_name = {t.name: t for t in self.tables.values()}
        self.rows = {i: t.by_id() for i, t in self.tables.items()}
        self.specs = module.car_specs(self.folder)
        self.columns = json.loads((HERE / 'evidence/gt4-column-map.json').read_text())
        self.text = module.strings(self.folder / 'UnistrDB.sdb')
        self.root = root

    def row(self, table_id, row_id):
        table = self.tables[table_id]
        row = self.rows[table_id][row_id]
        result = {'table': table.name, 'table_id': table_id, 'row_id': row_id,
                  'label': table.labels.get(row_id), 'hex': row.hex(),
                  'source': pin(self.folder / (table.name + '.dbt'), self.root)}
        if table.name in self.columns:
            result['fields'] = decode_row(row, self.columns[table.name]['columns'], self.text)
        return result

    def profile(self, code):
        model = code.removeprefix('GT4_').lower()
        spec = self.specs[model]
        generic = self.by_name['GENERIC_CAR']
        car_id = next(i for i, label in generic.labels.items() if label == spec['label'])
        car = self.rows[generic.table_id][car_id]
        parts_id = struct.unpack_from('<i', car)[0]
        parts = self.by_name['DEFAULT_PARTS']
        row = self.rows[parts.table_id][parts_id]
        output = [self.row(generic.table_id, car_id), self.row(parts.table_id, parts_id)]
        for offset in range(0, len(row), 8):
            key, table_id = struct.unpack_from('<ii', row, offset)
            if table_id >= 0 and key >= 0:
                output.append(self.row(table_id, key))
        seen = {(r['table_id'], r['row_id']) for r in output}
        for record in output:
            for field in record.get('fields', []):
                if field['type'] != 'Key':
                    continue
                key = field['raw']
                pair = (key['table_id'], key['row_id'])
                if min(pair) >= 0 and pair not in seen:
                    seen.add(pair)
                    output.append(self.row(*pair))
        return {'decoded_dimensions': spec, 'default_part_rows': output,
                'limits': ['Factory default parts joined by numeric table IDs, not similar model names.',
                           'Raw tuning values retain native encoding; do not interpret arbitrary bytes as SI.',
                           'ENGINE display PS/torque values are separate from torque curve and installed modifiers.',
                           'CHASSIS tread and exporter wheel placement are not manufacturer track measurements.']}


def parse_redline(data):
    values, index = [], 0
    for number, line in enumerate(data.decode('mac_roman').splitlines(), 1):
        line = line.split(';', 1)[0].strip()
        match = re.fullmatch(r'#\s*(\d+)', line)
        if match:
            index = int(match[1])
            continue
        match = re.match(r'([A-Za-z][A-Za-z0-9_.]*)\s+(.+)', line)
        if not match:
            continue
        key, raw = match.groups()
        raw = raw.strip()
        if raw.startswith('"'):
            value = raw.split('"', 2)[1]
        elif raw.startswith('{'):
            value = [float(x.strip()) for x in raw.strip('{} ').split(',')]
        else:
            try:
                value = float(raw)
            except ValueError:
                value = raw
        values.append({'key': key, 'index': index if '.' in key or key in ('gearRatios', 'colors') else None,
                       'value': value, 'line': number})
    return values


def redline_profile(root, catalog, public, index):
    native = next(x for x in public['cars'] if x['code'] == catalog['sourceCode'])
    source = next(x for x in index['cars'] if x['id'] == native['native_car_id'])
    path = root / 'games/redline/cars' / source['file']
    if digest(path) != source['sha256']:
        raise ValueError('Redline source hash mismatch')
    return {'source': pin(path, root), 'native_car_id': source['id'],
            'configuration': parse_redline(path.read_bytes()),
            'limits': ['Configuration parameters as authored; add-ons are community cars, not official factory specifications.',
                       'Units and runtime interpretation remain unverified unless explicitly documented.',
                       'Commented values are excluded. Repeated array indices and active assignments retained in source order.']}


def model_geometry(path, game):
    # Existing exporter conventions; AABBs enclose transformed accessor bounds.
    from prepare_fleet import parse, default_meshes, mesh_bounds, merged_bounds, wheel_owner
    doc, _ = parse(path.read_bytes())
    meshes = default_meshes(doc)
    bounds = merged_bounds([mesh_bounds(doc, doc['nodes'][i]['mesh'], world) for i, world, _ in meshes])
    wheels = {}
    for i, world, chain in meshes:
        found = wheel_owner(doc, i, world, chain, game)
        if found:
            key, position = found
            wheels[str(key)] = position
    result = {'default_scene_aabb': bounds,
              'default_scene_extent': [bounds[1][i] - bounds[0][i] for i in range(3)],
              'wheel_pivots': wheels, 'source_extras': doc.get('extras', {}),
              'limits': ['Static default-scene exported geometry; not a runtime physics measurement.',
                         'Extent order XYZ; source exporter metres, not independently calibrated against real car.',
                         'Accessor bounds transformed as boxes: conservative where rotated; may include mirrors/interior.',
                         'Wheel layout may be an exporter reconstruction; source_extras records its limitations.']}
    if len(wheels) == 4:
        ordered = sorted(wheels.values(), key=lambda p: p[2])
        result['wheelbase_from_pivots_m'] = abs(sum(p[2] for p in ordered[:2]) / 2 - sum(p[2] for p in ordered[2:]) / 2)
        result['track_at_negative_z_m'] = abs(ordered[0][0] - ordered[1][0])
        result['track_at_positive_z_m'] = abs(ordered[2][0] - ordered[3][0])
    return result


def mc3_profile(root, row, files):
    name = row['sourceCode']
    cfg = [x for x in files if Path(x['file']).name == name + '.carcfg']
    output = []
    for record in cfg:
        path = root / 'games/midnight-club-3-remix/cars' / record['file']
        if digest(path) != record['sha256']:
            raise ValueError('MC3 configuration hash mismatch')
        lines = []
        for number, line in enumerate(path.read_text().splitlines(), 1):
            match = re.match(r'\s*([A-Za-z][A-Za-z0-9_]*)\s+([-+\d.]+)\s*$', line)
            if match:
                lines.append({'key': match[1], 'value': float(match[2]), 'line': number})
        output.append({'source': pin(path, root), 'values': lines})
    return {'customization_configurations': output,
            'limits': ['All exact unsuffixed CarCfg matches retained; duplicate archive entries have no assumed precedence.',
                       'CarCfg wheel width/profile and upgrade values may be indices, not physical dimensions.',
                       'Player engine, traction, differential and suspension simulation tuning not decoded here.',
                       'No in-game acceleration, braking or top-speed observations were run.']}


def mcla_profile(root, row):
    import mcla_lzx
    import mcla_tuning
    folder = root / 'dealership/public/midnight-club-los-angeles'
    public = json.loads((folder / 'index.json').read_text())
    native = next(x for x in public['cars'] if x['code'] == row['sourceCode'])
    package = folder / native['native_package']['file']
    if digest(package) != native['native_package']['sha256']:
        raise ValueError('MCLA native package hash mismatch')
    decoded = []
    with zipfile.ZipFile(package) as archive:
        for name in archive.namelist():
            if not name.endswith('.xct'):
                continue
            data = archive.read(name)
            raw, header = mcla_lzx.decompress(data)
            value = mcla_tuning.decode(raw, header['virtual'], header['physical'])
            if value['code'] != row['sourceCode']:
                raise ValueError('MCLA tuning code mismatch')
            decoded.append({'member': name, 'sha256': hashlib.sha256(data).hexdigest(),
                            'bytes': len(data), 'decompressed_sha256': hashlib.sha256(raw).hexdigest(),
                            'header': header, 'tuning': value})
    return {'package': pin(package, root), 'tuning_resources': decoded,
            'limits': ['Tuning resource typed words by class/offset, not recovered semantic field names.',
                       'Base and mods sections retained separately. No asserted SI mapping for unknown floats.',
                       'No game runtime measurements were performed; no fabricated acceleration or grip coefficients.']}


def collect(root, codes):
    catalog = {x['code']: x for x in json.loads((root / 'dealership/dealership/catalog.json').read_text())}
    origins = {x['code']: x for x in json.loads((root / 'dealership/dealership/origins.json').read_text())['cars']}
    gt4 = GT4(root)
    red_public = json.loads((root / 'dealership/public/redline/index.json').read_text())
    red_index = json.loads((root / 'games/redline/cars/index.json').read_text())
    mc3 = json.loads((root / 'games/midnight-club-3-remix/cars/index.json').read_text())
    output = []
    for code in sorted(codes):
        row = catalog[code]
        model = root / 'dealership/dealership/models' / (code + '.glb')
        result = {'asset_code': code, 'game': row['game'], 'game_label': row['gameLabel'],
                  'catalog': row, 'model': pin(model, root), 'origin': origins[code],
                  'geometry': model_geometry(model, row['game'])}
        if row['game'] == 'gran-turismo-4':
            result['native_profile'] = gt4.profile(code)
        elif row['game'] == 'gran-turismo':
            result['native_profile'] = row['nativeMetadata']
        elif row['game'] == 'redline':
            result['native_profile'] = redline_profile(root, row, red_public, red_index)
        elif row['game'] == 'midnight-club-3-remix':
            result['native_profile'] = mc3_profile(root, row, mc3['files'])
        elif row['game'] == 'midnight-club-los-angeles':
            result['native_profile'] = mcla_profile(root, row)
        else:
            result['native_profile'] = {'catalog_parameters': {k: row.get(k) for k in ('year', 'topSpeed', 'bhp', 'kg', 'agility', 'accel', 'speed', 'weight')}, 'limits': row.get('claimLimits', []) + ['No additional validated runtime physics decoding in this corpus.']}
        output.append(result)
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--selection', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    sys.path.insert(0, str(args.archive / 'tools'))
    sys.path.insert(0, str(HERE.parents[1] / 'tools'))
    selection = json.loads(args.selection.read_text())
    codes = {code for car in selection['cars'] for code in car['asset_codes']}
    output = collect(args.archive, codes)
    source_files = [args.archive / 'dealership/dealership/catalog.json', args.archive / 'dealership/dealership/origins.json']
    source_files += [args.archive / 'tools' / name for name in ('gt4_specdb.py', 'mc3_archive.py', 'mcla_tuning.py', 'mcla_lzx.py', 'mcla_resource.py')]
    source_files += list((args.archive / 'games').glob('*/cars/index.json'))
    specdb = args.archive / 'games/gran-turismo-4/cars/provenance/specdb/GT4_EU2560'
    source_files += list(specdb.glob('*.idi')) + list(specdb.glob('*.sdb'))
    save(args.output, {'schema': 'vehicle-reference-game-profiles/v1', 'accessed': '2026-10-23',
                       'archive_root': str(args.archive), 'implementation_sources': [
                           pin(Path(__file__), HERE.parents[1]),
                           pin(HERE.parents[1] / 'tools/prepare_fleet.py', HERE.parents[1]),
                           pin(HERE / 'evidence/gt4-column-map.json', HERE.parents[1])], 'sources': [pin(p, args.archive) for p in source_files],
                       'profiles': output})
    print(f'Collected {len(output)} game assets for {len(selection["cars"])} real counterparts')


if __name__ == '__main__':
    main()
