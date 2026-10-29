#!/usr/bin/env python3
"""Independent canonical-C to frozen Profile20 projection and DSIM encoding.

No Rust source or output is read. Raw canonical observations remain intact.
The reused encoder and all historical evidence are unchanged on disk.
"""
import argparse
import contextlib
import csv
import hashlib
import io
import json
import shutil
import tempfile
from pathlib import Path
import m3f3_snapshot_goldens as encoder

ROOT = Path(__file__).resolve().parents[2]
NATIVE_STATES = [
    ('S_SARG_STND',1),('S_SARG_STND2',3),('S_SARG_RUN1',2),
    *[(f'S_SARG_RUN{i}',13+i) for i in range(2,9)],
    *[(f'S_SARG_ATK{i}',3+i) for i in range(1,4)],
    ('S_SARG_PAIN',7),('S_SARG_PAIN2',8),
    *[(f'S_SARG_DIE{i}',8+i) for i in range(1,7)],
]
source_names = {r[2]:int(r[1]) for r in csv.reader((ROOT/'tests/reference/m3f4_identities.csv').read_text().splitlines()[2:]) if r[0]=='state'}
STATE_MAP = {source_names[name]:native for name,native in NATIVE_STATES}
assert len(STATE_MAP)==len(set(STATE_MAP.values()))==21


def project(line):
    r = line.split(',')
    if r[0]=='A' and r[5]=='12':
        r[5]='136'
        if int(r[13])>=0:
            r[13]=str(STATE_MAP[int(r[13])])
    elif r[0]=='E':
        if r[3]=='1' and r[5]=='12': r[5]='136'
        elif r[3]=='4' and int(r[5]) in STATE_MAP: r[5]=str(STATE_MAP[int(r[5])])
    elif r[0]=='K' and r[3]=='8': r[5]=str(STATE_MAP[int(r[5])])
    elif r[0]=='J' and r[3]=='3': r[6]=str(STATE_MAP[int(r[6])])
    elif r[0]=='G' and r[3] in ['6','7'] and int(r[5]) in STATE_MAP: r[5]=str(STATE_MAP[int(r[5])])
    return ','.join(r)


def run(check):
    raw = (ROOT/'tests/reference/m3f5_trace.csv').read_text().splitlines()
    assert raw[0]=='DOOM-DEMON-EXEC-v1'
    outputs={}
    with tempfile.TemporaryDirectory(prefix='doom-m3f5-encode-') as directory:
        temporary=Path(directory)
        (temporary/'tests/reference').mkdir(parents=True)
        (temporary/'tools/reference').mkdir(parents=True)
        for name in ['m3f3_snapshot_goldens.py','m3c_snapshot_goldens.py']:
            shutil.copyfile(ROOT/'tools/reference'/name,temporary/'tools/reference'/name)
        for name in ['m3c_states.csv','m3c_types.csv']:
            shutil.copyfile(ROOT/'tests/reference'/name,temporary/'tests/reference'/name)
        shutil.copyfile(ROOT/'tests/reference/m3f5_cases.csv',temporary/'tests/reference/m3f3_cases.csv')
        (temporary/'tests/reference/m3f3_trace.csv').write_text('\n'.join(project(line) for line in raw[1:])+'\n')
        maps=sorted((ROOT/'tests/reference').glob('m3f5_map_*.bin'))
        for path in maps:
            shutil.copyfile(path,temporary/'tests/reference'/path.name.replace('m3f5','m3f3'))
        encoder.ROOT=temporary
        encoder.map_fields.cache_clear()
        encoder.metadata_values.cache_clear()
        encoder.FULL_LABELS={'canonical_lifecycle','canonical_clipping','canonical_shared_death'}
        with contextlib.redirect_stdout(io.StringIO()): encoder.run(False)
        old=json.loads((temporary/'tests/reference/m3f3_snapshot_provenance.json').read_text())
        for path in (temporary/'tests/reference').glob('m3f3_*'):
            if path.name in ['m3f3_cases.csv','m3f3_trace.csv','m3f3_snapshot_provenance.json'] or '_map_' in path.name or '_atomic_' in path.name:
                continue
            data=path.read_bytes()
            if path.suffix=='.csv': data=data.replace(b'm3f3_state_',b'm3f5_state_')
            outputs['tests/reference/'+path.name.replace('m3f3','m3f5')]=data
    inputs=['tools/reference/m3f5_snapshot_goldens.py','tools/reference/m3f3_snapshot_goldens.py','tools/reference/m3c_snapshot_goldens.py',
            'tests/reference/m3f5_trace.csv','tests/reference/m3f5_cases.csv','tests/reference/m3f4_identities.csv',
            'tests/reference/m3c_states.csv','tests/reference/m3c_types.csv']+[str(p.relative_to(ROOT)) for p in maps]
    full=sum('_state_' in p for p in outputs)
    manifest={'schema':'DSIM-v20', 'source_schema':'DOOM-DEMON-EXEC-v1', 'captures':old['captures'],
              'ordered_event_fields':old['ordered_event_fields'], 'full_c_derived_byte_fixtures':full, 'rust_policy_byte_fixtures':0,
              'identity_projection':{'actor':{'12':136},'states':STATE_MAP,'fields':['A.type/state','E.spawn/entered','K.see','J.selection','G.pain/kill'],
                                     'exclusions':'Raise has no mapping. Source target135/roster adaptation is retained. Nothing else is identity-rewritten.'},
              'inputs_sha256':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs},
              'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},
              'scope':'Raw canonical C observations independently projected into frozen native Profile20 identity representation, then encoded by unchanged independent encoder. No Rust inputs/outputs. No new DSIM schema or capacity fixture.'}
    outputs['tests/reference/m3f5_snapshot_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
    for name,data in outputs.items():
        if check: assert (ROOT/name).read_bytes()==data,name
        else: (ROOT/name).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} canonical C / DSIM-v20 projection: {old["captures"]} captures, {full} C-derived full fixtures, {old["ordered_event_fields"]} ordered event fields.')


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args();run(args.check)
