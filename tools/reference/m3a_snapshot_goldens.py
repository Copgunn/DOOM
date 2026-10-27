#!/usr/bin/env python3
"""Independent DSIM-v2 coasting encoder. Reads C vectors and Python DSIM-v1 only.

No Rust output is input. M2 encoding remains unchanged; new schema deltas and
clockwise synthetic map records are explicit here, separately from C behavior.
"""
import argparse
import csv
import hashlib
import json
import struct
import snapshot_goldens as m2

ROOT=m2.ROOT

def decode_python_v1(data):
    assert data[:8]==b'DSIM\1\0\0\0'
    count=struct.unpack_from('<Q',data,8)[0];offset=16;fields=[]
    for _ in range(count):
        size=struct.unpack_from('<I',data,offset)[0];offset+=4
        name=data[offset:offset+size].decode();offset+=size
        size=struct.unpack_from('<Q',data,offset)[0];offset+=8
        fields.append((name,data[offset:offset+size]));offset+=size
    assert offset==len(data)
    return fields

def snapshot(row):
    base=dict(row,attempts='1',candidates=row['lines'])
    fields=[];tic=int(row['tic'])+1;ident=m2.ident()
    assert row['tries']==f"{row['x']}:{row['y']}:1" and row['lines']=='0:1:2:3' and row['things']=='0'
    for name,value in decode_python_v1(m2.snapshot(base)):
        if name.startswith('events.'):
            continue
        if name=='profile':value=struct.pack('<I',2)
        if name=='player.0.actor':value=b'\1'+ident
        if name=='map.vertices':value=struct.pack('<8h',-64,-64,-64,64,64,64,64,-64)
        if name=='map.segs':value=b''.join(struct.pack('<6h',i,(i+1)%4,0,i,0,0) for i in range(4))
        fields.append((name,value))
        if name=='actor.0.lastlook':
            fields.extend([('actor.0.flags',struct.pack('<I',2)),('spatial.link_order',ident)])
    move=b'\10'+ident+struct.pack('<ii',int(row['x']),int(row['y']))+bytes(13)+struct.pack('<I',1)+ident+struct.pack('<HBI4H',65535,1,4,0,1,2,3)
    events=([b'\1'+ident] if tic==1 else [])+[b'\0\0',b'\4\0'+ident,b'\0\1',b'\5'+ident,move,b'\0\2',b'\0\3']
    event_fields=[('events.last.count',struct.pack('<Q',len(events)))]+[(f'events.last.{i}',value) for i,value in enumerate(events)]+[('events.pending.count',bytes(8))]
    index=next(i for i,(name,_) in enumerate(fields) if name.startswith('map.'))
    fields[index:index]=event_fields
    data=b'DSIM'+struct.pack('<IQ',2,len(fields))
    for name,value in fields:
        path=name.encode();data+=struct.pack('<I',len(path))+path+struct.pack('<Q',len(value))+value
    return data

def run(check):
    rows=[row for row in csv.DictReader((ROOT/'tests/reference/m3a_movement.csv').open()) if row['case']=='18']
    assert len(rows)==12
    lines=['tic,fnv1a64,x,y,momx,momy'];outputs={}
    for row in rows:
        data=snapshot(row);h=0xcbf29ce484222325
        for byte in data:h=((h^byte)*0x100000001b3)&0xffffffffffffffff
        lines.append(f"{int(row['tic'])+1},{h:016x},{row['x']},{row['y']},{row['momx']},{row['momy']}")
        if row['tic']=='0':outputs['tests/reference/m3a_coast_first.bin']=data
    outputs['tests/reference/m3a_snapshots.csv']=('\n'.join(lines)+'\n').encode()
    provenance={'schema':'DSIM-v2', 'scope':'explicit one-solid-player clockwise room, twelve C coasting tics; not a reference engine savegame', 'inputs_sha256':{name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in ['tests/reference/m3a_movement.csv','fuzz/corpus/map/classic_square','tests/support/mod.rs','tools/reference/snapshot_goldens.py','tools/reference/m3a_snapshot_goldens.py']}, 'outputs_sha256':{name:hashlib.sha256(value).hexdigest() for name,value in outputs.items()}}
    outputs['tests/reference/m3a_snapshot_provenance.json']=(json.dumps(provenance,indent=2)+'\n').encode()
    for name,value in outputs.items():
        path=ROOT/name
        if check:assert path.read_bytes()==value,name
        else:path.write_bytes(value)
    print(('Checked' if check else 'Generated')+' independent DSIM-v2 goldens.')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
