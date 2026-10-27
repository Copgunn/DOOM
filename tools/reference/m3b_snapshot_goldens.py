#!/usr/bin/env python3
"""Independent DSIM-v3 encoding of explicit setups and original C trace values.

No Rust output is read. Per-attempt counts and chronological motion observations
come from C. Physical contact bits use pre-Z C values and the source predicates.
"""
import argparse
import csv
import hashlib
import json
import struct
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
SELECTED=[2,9,14,26]
def num(code,value):return struct.pack('<'+code,int(value))
def ident(value=0):return struct.pack('<IQ',value,1)
def optional(value):return bytes(13) if value<0 else b'\1'+ident(value)
def ints(value):return [] if value=='-' else [int(x) for x in value.split(':')]
def records(value):return [] if value=='-' else [ints(x) for x in value.split(';')]
def map_fields(index):
    data=(ROOT/f'tests/reference/m3b_map_{index}.bin').read_bytes();count,offset=struct.unpack_from('<ii',data,4);entries=[]
    for i in range(count):
        start,size,name=struct.unpack_from('<ii8s',data,offset+16*i);entries.append((name,data[start:start+size]))
    names=['things','linedefs','sidedefs','vertices','segs','subsectors','nodes','sectors','reject']
    fields=[('map.'+name,entries[i+1][1]) for i,name in enumerate(names)]
    block=entries[10][1];words=struct.unpack('<'+'H'*(len(block)//2),block);ranges=b''
    for start in words[4:4+words[2]*words[3]]:
        end=start
        while words[end]!=65535:end+=1
        # Selected fixtures explicitly list all lines in every cell, hence no
        # conservative coverage omissions. No Rust collision code is consulted.
        assert set(words[start:end])==set(range(len(entries[2][1])//14))
        ranges+=struct.pack('<II',start,end)
    return fields+[('map.blockmap.words',block),('map.blockmap.cell_ranges',ranges)],entries[8][1]
def snapshot(case,row):
    tic=int(row['tic'])+1;actor=ident();flags=int(case['flags']);maps,sectorbytes=map_fields(int(case['map']))
    fields=[('tic',num('Q',tic)),('tic_rate',num('I',35)),('profile',num('I',3)),('paused',b'\0'),('rng.gameplay',b'\1'),('rng.miscellaneous',b'\0'),('store.max_slots',num('I',1024)),('store.slot_count',num('Q',1)),('store.head',optional(0)),('store.tail',optional(0)),('thinker.order',actor),('actor.0.generation',num('Q',1)),('actor.0.retired',b'\0'),('actor.0.occupied',b'\1'),('actor.0.pending_removal',b'\0'),('actor.0.previous',bytes(13)),('actor.0.next',bytes(13))]
    for name,value in [('x',row['x']),('y',row['y']),('z',row['z']),('momx',row['momx']),('momy',row['momy']),('radius',8*65536),('height',56*65536),('floor',row['floor']),('ceiling',row['ceiling'])]:fields.append(('actor.0.'+name,num('i',value)))
    fields += [('actor.0.angle',num('I',row['angle'])),('actor.0.lastlook',b'\0'),('actor.0.flags',num('I',flags)),('actor.0.momz',num('i',row['momz'])),('actor.0.subsector',num('H',row['subsector'])),('actor.0.sector',num('H',row['sector'])),('spatial.link_order',actor),('world.blockmap_policy',b'\1'),('world.conservative_omissions',bytes(4)),('sectors.count',num('I',len(sectorbytes)//26))]
    member_lists=row['members'].split('|')
    for i in range(len(sectorbytes)//26):
        f,c=struct.unpack_from('<hh',sectorbytes,26*i)
        members=ints(member_lists[i]);assert members in [[],[0]]
        fields += [(f'sector.{i}.floor',num('i',f*65536)),(f'sector.{i}.ceiling',num('i',c*65536)),(f'sector.{i}.actors',b''.join(ident(m) for m in members))]
    for i in range(4):
        active=i==0
        fields += [(f'player.{i}.actor',optional(0) if active and case['player']=='1' else bytes(13)),(f'player.{i}.forward',num('b',case['forward'] if active else 0)),(f'player.{i}.side',num('b',case['side'] if active else 0)),(f'player.{i}.angle_turn',num('h',case['turn'] if active else 0)),(f'player.{i}.buttons',b'\0')]
    tries=records(row['tries']);lines=ints(row['lines']);things=ints(row['things']);opens=records(row['openings']);l=t=o=0;encoded=[]
    for move in tries:
        x,y,allowed,f,c,d,floatok,reason,leaf,sector,line,blockactor,lc,tc,oc=move
        ls=lines[l:l+lc];ts=things[t:t+tc];os=opens[o:o+oc];l+=lc;t+=tc;o+=oc
        assert all(actor==0 for actor in ts) and blockactor==-1
        payload=actor+struct.pack('<ii',x,y)+optional(blockactor)+num('I',tc)+b''.join(ident(a) for a in ts)+num('H',65535 if line<0 else line)+num('B',reason!=3)+num('I',lc)+b''.join(num('H',a) for a in ls)
        payload+=struct.pack('<HHiiiBBI',leaf,sector,f,c,d,floatok,reason,oc)
        for index,top,bottom,span,low in os:payload+=struct.pack('<Hiiii',index,top,bottom,span,low)
        encoded.append(b'\12'+payload)
    assert l==len(lines) and t==len(things) and o==len(opens)
    events=([b'\1'+actor] if tic==1 else [])+[b'\0\0']+([b'\4\0'+actor] if case['player']=='1' else [])+[b'\0\1',b'\5'+actor]
    for action in ([] if row['motion_order']=='-' else row['motion_order'].split(';')):
        value=int(action[1:]);events.append(encoded[value] if action[0]=='T' else b'\11'+actor+num('H',value))
    before=ints(row['vertical'])
    if before:
        z,mom=before;floor=int(row['floor']);ceiling=int(row['ceiling']);temporary=z+mom
        floor_contact=temporary<=floor
        gravity=temporary>floor and not flags&512
        clipped=floor if floor_contact else temporary
        ceiling_contact=clipped+56*65536>ceiling
        events.append(b'\13'+actor+struct.pack('<6i3B',z,mom,int(row['z']),int(row['momz']),floor,ceiling,floor_contact,ceiling_contact,gravity))
    events.extend([b'\0\2',b'\0\3'])
    fields += [('events.last.count',num('Q',len(events)))]+[(f'events.last.{i}',e) for i,e in enumerate(events)]+[('events.pending.count',bytes(8))]+maps
    out=b'DSIM'+struct.pack('<IQ',3,len(fields))
    for path,value in fields:
        path=path.encode();out+=num('I',len(path))+path+num('Q',len(value))+value
    return out

def run(check):
    cases={int(c['case']):c for c in csv.DictReader((ROOT/'tests/reference/m3b_cases.csv').open())}
    rows=[r for r in csv.DictReader((ROOT/'tests/reference/m3b_movement.csv').open()) if int(r['case']) in SELECTED]
    assert len(rows)==96
    outputs={};goldens=['case,tic,fnv1a64,z,momz,sector,bytes_fixture'];saved=set()
    for row in rows:
        case=int(row['case']);data=snapshot(cases[case],row);fixture='-'
        name=None
        if case==14 and row['tic']=='0':name='m3b_fall_first.bin'
        if case==2 and row['z']==str(24*65536) and 'step' not in saved:name='m3b_step.bin';saved.add('step')
        if case==2 and row['sector']=='1' and 'transition' not in saved:name='m3b_transition.bin';saved.add('transition')
        if case==9 and int(row['momz'])<0 and 'air' not in saved:name='m3b_drop_air.bin';saved.add('air')
        if case==9 and row['vertical']!='-' and int(row['z'])==int(row['floor']) and 'landing' not in saved:name='m3b_landing.bin';saved.add('landing')
        if name:fixture=name;outputs['tests/reference/'+name]=data
        h=0xcbf29ce484222325
        for byte in data:h=((h^byte)*0x100000001b3)&0xffffffffffffffff
        goldens.append(f"{case},{int(row['tic'])+1},{h:016x},{row['z']},{row['momz']},{row['sector']},{fixture}")
    assert len(outputs)==5
    outputs['tests/reference/m3b_snapshots.csv']=('\n'.join(goldens)+'\n').encode()
    inputs=['tools/reference/m3b_snapshot_goldens.py','tests/reference/m3b_cases.csv','tests/reference/m3b_movement.csv']+[f'tests/reference/m3b_map_{i}.bin' for i in sorted({int(cases[c]['map']) for c in SELECTED})]
    p={'schema':'DSIM-v3','scope':'96 physical C-trace states: step/transition, drop/gravity/landing, vertical fall, airborne input; project schema, not engine saves','inputs_sha256':{name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in inputs},'outputs_sha256':{name:hashlib.sha256(data).hexdigest() for name,data in outputs.items()}}
    outputs['tests/reference/m3b_snapshot_provenance.json']=(json.dumps(p,indent=2)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/name
        if check:assert path.read_bytes()==data,name
        else:path.write_bytes(data)
    print(('Checked' if check else 'Generated')+' independent DSIM-v3 goldens: 96 digests, five full states.')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
