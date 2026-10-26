"""Prepare 243 additional source editions without publishing or changing the 57-car roster."""
import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import re
import sys

from prepare_fleet import ROOT, prepare

OUT=ROOT/'docs/verification/fleet-300'


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def diverse(rows):
    groups=defaultdict(list)
    for row in sorted(rows,key=lambda r:r['code']):
        key=re.sub(r'[^a-z0-9]+',' ',row['name'].lower()).split()[0]
        groups[key].append(row)
    while any(groups.values()):
        for key in sorted(groups):
            if groups[key]:yield groups[key].pop(0)


def gt4_native(decoder,code):
    native=decoder.profile(code)
    fields={}
    for row in native['default_part_rows']:
        fields.setdefault(row['table'],{f['column']:f.get('text',f['raw']) for f in row.get('fields',[])})
    engine=fields.get('ENGINE',{});gear=fields.get('GEAR',{})
    ratios=[gear.get(f'Gear{i}{suffix}',0) for i,suffix in enumerate(['st','nd','rd']+['th']*8,1)]
    ratios=[v for v in ratios if v>0]
    if not (15<=engine.get('psvalue',0)<=1800 and 50<=engine.get('torquevalue',0)<=30000 and len(ratios)>=2 and gear.get('finalgearDF',0)>0):
        raise ValueError('insufficient supported native power/torque/gearing')
    if any(gear.get(k, 0) != 0 for k in ['gearflag', 'ExtraFinalGearRatio', 'ExtraFinalGearUsage']):
        raise ValueError('unqualified extra-final or flagged native gearbox')
    if any(a<=b for a,b in zip(ratios,ratios[1:])):
        raise ValueError('unsupported non-descending gearbox')
    return {'game':'gran-turismo-4','fields':fields,'native':native}


def eligible(row,decoder):
    if row['game']=='gran-turismo-4':return gt4_native(decoder,row['code'])
    spec=(row.get('nativeMetadata') or {}).get('specifications') or {}
    if spec.get('drive') not in ['FR','FF','MR','4WD','RR'] or not (row.get('kg') or 0)>0 or not (row.get('bhp') or 0)>0:
        raise ValueError('native GT1 stock specification unavailable')
    return {'game':'gran-turismo','fields':spec,'native':row['nativeMetadata']}


def geometry_valid(car):
    wb=(car['wheels'][2][2]+car['wheels'][3][2]-car['wheels'][0][2]-car['wheels'][1][2])/2
    if not 1.7<wb<4.5:raise ValueError(f'unsupported authored wheelbase {wb}')
    for a,b in [(0,1),(2,3)]:
        track=abs(car['wheels'][a][0]-car['wheels'][b][0])
        if not .9<track<2.4:raise ValueError(f'unsupported authored track {track}')


class Preparation:
    def __init__(self, source, catalog, decoder, existing, destination):
        self.source, self.catalog, self.decoder = source, catalog, decoder
        self.existing, self.destination = existing, destination
        self.selection, self.additions, self.native, self.rejected = [], [], {}, []
        destination.mkdir(parents=True, exist_ok=True)

    def group(self, game, target):
        rows = [r for r in self.catalog.values() if r['game'] == game and r['code'] not in self.existing]
        accepted = 0
        for row in diverse(rows):
            code = row['code']; ident = f'exp-{len(self.additions)+1:03}'
            try:
                data = eligible(row, self.decoder)
                car = prepare(self.source, self.catalog, ident, code, row['name'], self.destination)
                geometry_valid(car)
            except (ValueError, AssertionError, KeyError) as error:
                self.rejected.append({'code':code,'stage':'static source/geometry eligibility','reason':str(error)})
                print('Excluded before testing', code, str(error), flush=True)
                for suffix in ['.glb', '.json']:
                    self.destination.joinpath(ident+suffix).unlink(missing_ok=True)
                continue
            self.selection.append({'id':ident,'source_code':code,'name':row['name'],'game':row['game']})
            self.additions.append(car); self.native[ident] = {'catalog':row, **data}; accepted += 1
            if accepted == target:
                return
        raise ValueError(f'only {accepted}/{target} eligible models')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root',type=Path,default=ROOT.parent/'carmodels')
    parser.add_argument('--staging-dir',type=Path,default=ROOT/'.scratch/fleet-300-build/cars')
    args=parser.parse_args()
    assert not (OUT/'selection.json').exists(),'immutable selection already exists'
    OUT.mkdir(parents=True,exist_ok=True)
    roster=read(ROOT/'assets/cars/roster.json');physics=read(ROOT/'assets/cars/physics.json');assert len(roster)==57
    for name,value in [('original-roster',roster),('original-physics',physics)]:
        (OUT/(name+'.json')).write_text(json.dumps(value,indent=2)+'\n')
    pinned={str(p.relative_to(ROOT)):sha(p) for c in roster for p in [ROOT/'assets'/c['path'],(ROOT/'assets'/c['path']).with_suffix('.json')]}
    sys.path[:0]=[str(ROOT/'docs/vehicle-reference/scripts'),str(args.source_root/'tools')]
    from collect_game_profiles import GT4
    decoder=GT4(args.source_root.resolve());source=args.source_root/'dealership/dealership'
    catalog={r['code']:r for r in read(source/'catalog.json')};existing={r['source_code'] for r in roster}
    batch=Preparation(source,catalog,decoder,existing,args.staging_dir)
    for game,target in [('gran-turismo-4',150),('gran-turismo',93)]:
        batch.group(game,target)
    assert all(sha(ROOT/p)==h for p,h in pinned.items())
    outputs={'selection':{'preserve_existing':57,'target_fleet_size':300,'cars':batch.selection},'prepared-additions':batch.additions,'native-specifications':batch.native,'preparation':{'existing_material_sha256':pinned,'rejected_before_physics_testing':batch.rejected,'native_catalog_sha256':sha(source/'catalog.json'),'decoder_sha256':sha(ROOT/'docs/vehicle-reference/scripts/collect_game_profiles.py')}}
    for name,value in outputs.items():(OUT/(name+'.json')).write_text(json.dumps(value,indent=2)+'\n')
    assert read(ROOT/'assets/cars/roster.json')==roster and read(ROOT/'assets/cars/physics.json')==physics
    print('Prepared243 additions;57-car runtime roster/profiles remain unchanged',flush=True)


if __name__=='__main__':main()
