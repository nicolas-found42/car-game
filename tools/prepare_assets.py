#!/usr/bin/env python3
"""Prepare the personal-use 350Z copy and deterministic placeholder sounds/textures.

No source files are changed. Only default-paint materials are retained. Geometry,
UVs, node transforms and embedded PNG bytes are preserved. GT4's paint alpha and
marker-glass preview repairs follow carmodels/dealership/gt4-materials.mjs.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import random
import struct
import wave
import zlib

ROOT = Path(__file__).resolve().parents[1]


def png_pixels(doc, binary, texture):
    image = doc['images'][doc['textures'][texture]['source']]
    view = doc['bufferViews'][image['bufferView']]
    data = binary[view.get('byteOffset', 0):][:view['byteLength']]
    width, height = struct.unpack_from('>II', data, 16)
    if data[24:29] != bytes([8, 6, 0, 0, 0]):
        raise ValueError('Expected source RGBA8, non-interlaced PNG')
    pos, compressed = 8, bytearray()
    while pos < len(data):
        length = struct.unpack_from('>I', data, pos)[0]
        if data[pos + 4:pos + 8] == b'IDAT':
            compressed.extend(data[pos + 8:pos + 8 + length])
        pos += length + 12
    raw = zlib.decompress(compressed)
    stride = width * 4 + 1
    if len(raw) != stride * height or any(raw[y * stride] for y in range(height)):
        raise ValueError('Expected source PNG filter 0')
    return b''.join(raw[y * stride + 1:(y + 1) * stride] for y in range(height))


def prepare_car(source):
    original = source.read_bytes()
    if original[:8] != b'glTF\x02\x00\x00\x00':
        raise ValueError('Expected GLB2')
    length = struct.unpack_from('<I', original, 12)[0]
    doc = json.loads(original[20:20 + length])
    binary = original[28 + length:]
    if doc.get('extras', {}).get('car') != 'niss0004':
        raise ValueError('Expected GT4 Nissan 350Z niss0004')
    primitives = [p for mesh in doc['meshes'] for p in mesh['primitives']]
    used = sorted({p['material'] for p in primitives})
    fixes = []
    for index in used:
        material = doc['materials'][index]
        pbr = material['pbrMetallicRoughness']
        native = material.get('extras', {}).get('gt4_material', {})
        finish = max(native.get('specular', [0, 0, 0])[:3])
        if material['name'] == 'body/untextured/mat-1/BLEND/paint0':
            # The paired front/rear cabin backing is opaque in the static export.
            # This is an artistic glass preview, not recovered GT4 reflection shading.
            pbr['baseColorFactor'] = [0.025, 0.045, 0.06, 1.0]
            pbr['metallicFactor'] = 0.25
            pbr['roughnessFactor'] = 0.18
            fixes.append({'material': index, 'repair': 'artistic dark cabin-glass backing'})
        if finish > 0 and material.get('alphaMode') == 'MASK':
            material['alphaCutoff'] = 1 / 255
            pbr['baseColorFactor'][3] = 1
            fixes.append({'material': index, 'repair': 'retain nonzero paint alpha'})
        if finish > 0:
            pbr['roughnessFactor'] = 0.32
        if material.get('alphaMode') == 'BLEND' and 'baseColorTexture' in pbr:
            pixels = png_pixels(doc, binary, pbr['baseColorTexture']['index'])
            marker = sum(pixels[i] == 8 and pixels[i + 1] == 255
                         and pixels[i + 2] in (16, 55) and pixels[i + 3] >= 63
                         for i in range(0, len(pixels), 4))
            if marker >= len(pixels) / 16:
                pbr['baseColorFactor'] = [0.10, 0.15, 0.20, 0.35]
                pbr.pop('baseColorTexture')
                pbr['roughnessFactor'] = 0.16
                material['doubleSided'] = True
                fixes.append({'material': index, 'repair': 'neutral marker-glass preview'})
        if material['name'].startswith('tire_'):
            pbr['baseColorFactor'] = [0.12, 0.12, 0.12, 1]
            pbr['roughnessFactor'] = 0.95
    mapping = {old: new for new, old in enumerate(used)}
    doc['materials'] = [doc['materials'][i] for i in used]
    for p in primitives:
        p['material'] = mapping[p['material']]
        p.pop('extensions', None)
    doc.pop('extensions', None)
    doc.pop('extensionsUsed', None)
    doc.pop('extensionsRequired', None)

    def texture_refs(value):
        if isinstance(value, dict):
            for key, child in value.items():
                if key.endswith('Texture') and isinstance(child, dict) and 'index' in child:
                    yield child
                else:
                    yield from texture_refs(child)
        elif isinstance(value, list):
            for child in value:
                yield from texture_refs(child)

    refs = list(texture_refs(doc['materials']))
    textures = sorted({ref['index'] for ref in refs})
    mapping = {old: new for new, old in enumerate(textures)}
    for ref in refs:
        ref['index'] = mapping[ref['index']]
    doc['textures'] = [doc['textures'][i] for i in textures]
    images = sorted({t['source'] for t in doc['textures']})
    mapping = {old: new for new, old in enumerate(images)}
    for texture in doc['textures']:
        texture['source'] = mapping[texture['source']]
    doc['images'] = [doc['images'][i] for i in images]
    receipt = {'source': 'carmodels/dealership/dealership/models/GT4_NISS0004.glb',
               'source_sha256': hashlib.sha256(original).hexdigest(),
               'edition': 'default paint; material preview repairs; source geometry unchanged',
               'repairs': fixes, 'materials': len(used), 'images': len(images)}
    doc['extras']['carGamePreparation'] = receipt
    data = json.dumps(doc, separators=(',', ':')).encode()
    data += b' ' * (-len(data) % 4)
    binary += b'\0' * (-len(binary) % 4)
    result = (struct.pack('<4sII', b'glTF', 2, 28 + len(data) + len(binary))
              + struct.pack('<I4s', len(data), b'JSON') + data
              + struct.pack('<I4s', len(binary), b'BIN\0') + binary)
    output = ROOT / 'assets/cars/350z.glb'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(result)
    receipt['prepared_sha256'] = hashlib.sha256(result).hexdigest()
    output.with_suffix('.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(f'350Z: {len(original):,} -> {len(result):,} bytes; {len(used)} materials, {len(images)} images')


def png(path, width, height, pixel):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    raw = b''.join(b'\0' + bytes(c for x in range(width) for c in pixel(x, y)) for y in range(height))
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
                     + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def generated_assets():
    textures = ROOT / 'assets/textures'
    audio = ROOT / 'assets/audio'
    textures.mkdir(parents=True, exist_ok=True)
    audio.mkdir(parents=True, exist_ok=True)
    for variant in range(4):
        def facade(x, y, v=variant):
            if 3 < x % 16 < 13 and 5 < y % 22 < 17:
                shade = 38 + ((x // 16 * 7 + y // 22 * 3 + v * 5) % 5) * 9
                return (shade, shade + 16, shade + 25, 255)
            return (163 + v * 10, 166 + v * 8, 161 + v * 6, 255)
        png(textures / f'facade-{variant}.png', 128, 264, facade)
    rng = random.Random(350)
    sample_rate = 22050
    for name, seconds in [('engine', 1.0), ('tires', 1.0), ('impact', 0.3)]:
        frames = bytearray()
        previous = 0.0
        for i in range(int(sample_rate * seconds)):
            t = i / sample_rate
            if name == 'engine':
                value = sum(math.sin(2 * math.pi * 40 * h * t) / h for h in (1, 2, 3, 4, 6, 8)) * 0.24
            elif name == 'tires':
                noise = rng.uniform(-1, 1)
                value = (noise - previous) * 0.2
                previous = noise
            else:
                value = (rng.uniform(-1, 1) * 0.65 + math.sin(2 * math.pi * 65 * t) * 0.3) * math.exp(-t * 20)
            frames.extend(struct.pack('<h', int(max(-1, min(1, value)) * 32767)))
        with wave.open(str(audio / f'{name}.wav'), 'wb') as out:
            out.setparams((1, 2, sample_rate, 0, 'NONE', 'not compressed'))
            out.writeframes(frames)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, default=ROOT.parent / 'carmodels')
    args = parser.parse_args()
    prepare_car(args.source_root / 'dealership/dealership/models/GT4_NISS0004.glb')
    generated_assets()
