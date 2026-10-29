#!/usr/bin/env python3
"""Count the committed C evidence; inventories are not branch coverage."""
import csv,json,struct,collections
from pathlib import Path
root=Path(__file__).resolve().parents[2]/'tests/reference'
def blockmap(prefix,which):
    b=(root/f'{prefix}_map_{which}.bin').read_bytes();n,off=struct.unpack_from('<ii',b,4)
    for i in range(n):
        start,size,name=struct.unpack_from('<ii8s',b,off+16*i)
        if name.rstrip(b'\0')==b'BLOCKMAP':return struct.unpack_from('<hhHH',b,start)
    raise AssertionError('missing public blockmap')
results=[]
functions=None
for prefix in ['m3f3','m3f3_clipping']:
    p=json.loads((root/f'{prefix}_provenance.json').read_text());v=json.loads((root/f'{prefix}_snapshot_provenance.json').read_text())
    if functions is None:functions=p['function_sha256']
    else:assert functions==p['function_sha256'],'Shared bodies must not be double-counted'
    cases=list(csv.DictReader((root/f'{prefix}_cases.csv').open()));counts=collections.Counter();max_native=-1
    maps={int(c['map']):blockmap(prefix,int(c['map'])) for c in cases}
    for row in csv.reader((root/f'{prefix}_trace.csv').open()):
        tag=row[0];counts[tag]+=1
        if tag=='C':counts['ordered_line_candidates' if row[4]=='1' else 'ordered_thing_candidates']+=1
        if tag=='V' and row[4]!='0':counts['ordered_sight_visits']+=1;counts['sight_kind_'+row[4]]+=1
        if tag=='A' and row[4]!='0' and row[5]=='136':
            ox,oy,w,h=maps[int(cases[int(row[1])]['map'])];x=(int(row[6])-ox*65536)>>23;y=(int(row[7])-oy*65536)>>23
            if 0<=x<w and 0<=y<h:max_native=max(max_native,y*w+x)
    result=dict(adapter=prefix,source_inventory=p['evidence_inventory'],tics=sum(int(c['tics']) for c in cases),ordered_counts=dict(counts),maximum_observed_native_cell=max_native,captures=v['captures'],complete_dsim_byte_fixtures=sum('_state_' in p or '_atomic_' in p for p in v['outputs_sha256']),ordered_event_fields=v['ordered_event_fields'],rust_policy_byte_fixtures=v['rust_policy_byte_fixtures'])
    results.append(result)
print(json.dumps(dict(unique_original_bodies=len(functions),adapters=results),indent=2))
