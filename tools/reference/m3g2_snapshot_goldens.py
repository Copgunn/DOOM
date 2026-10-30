#!/usr/bin/env python3
"""Independent DSIM22 from actual C player/body rows and declared schema policy.

Reuse frozen bounded field/event encoding, never Rust output or actor aliases.
"""
import argparse
import collections
import csv
import hashlib
import json
import shutil
import struct
import tempfile
from pathlib import Path
import m3f3_snapshot_goldens as base
from m3f7_snapshot_goldens import fnv
ROOT = Path(__file__).resolve().parents[2]
n = base.n
original_events = base.events

def events(rows):
    out, chunk = [], []
    for r in rows:
        if r[0] in {'P','AP'}: continue
        if r[0] != 'PL': chunk.append(r); continue
        out += original_events(chunk);chunk=[]
        code,*v=r[3:]
        if code==0: out.append(bytes([22,0])+base.ident(v[0]))
        elif code==1:
            health,attacker,count,tactile=v
            out.append(bytes([22,1])+n('i',health)+base.opt(attacker)+struct.pack('<ii',count,tactile))
        elif code==2: out.append(bytes([22,2])+struct.pack('<iH',*v))
        else: raise ValueError(r)
    return out+original_events(chunk)


def frame(fields):
    data=bytearray(b'DSIM'+struct.pack('<IQ',22,len(fields)))
    for path,value in fields:
        path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
    return bytes(data)


def run(check):
    inputs=['tools/reference/m3g2_snapshot_goldens.py','tools/reference/m3f3_snapshot_goldens.py',
        'tools/reference/m3c_snapshot_goldens.py','tests/reference/m3g2_trace.csv',
        'tests/reference/m3g2_cases.csv','tests/reference/m3f5_map_0.bin']
    raw=(ROOT/inputs[3]).read_text().splitlines();assert raw.pop(0)=='DOOM-PLAYER-BODY-v1'
    rows=[[r[0],*map(int,r[1:])] for r in csv.reader(raw)]
    cases=list(csv.DictReader((ROOT/inputs[4]).open()))
    outputs={};index=['case,operation,tic,fnv1a64,bytes_fixture,bytes,fields,event_fields']
    stream=bytearray(b'EV22');event_fields=0;event_frames=0
    with tempfile.TemporaryDirectory(prefix='doom-player-encoding-') as directory:
        temp=Path(directory);(temp/'tests/reference').mkdir(parents=True)
        shutil.copyfile(ROOT/inputs[5],temp/'tests/reference/m3f3_map_0.bin')
        base.ROOT=temp;base.map_fields.cache_clear();base.metadata=lambda _c:[];base.events=events
        for which,values in enumerate(cases):
            c={k:int(v) for k,v in values.items() if k!='label'}
            c.update(mode=4,map=0,skill=2,no_monsters=0)  # Fixed declared body policy.
            selected=[r for r in rows if r[1]==which];grouped=collections.defaultdict(list)
            for r in selected: grouped[r[2]].append(r)
            capture_ops={r[2] for r in selected if r[0]=='P'}
            # Existing independent schema policy maps C allocation births to Rust
            # lowest-vacant slots/generations; this is not C allocator equivalence.
            occupied={};generations=[];base.BIRTH_ID={};queued=[]
            for op in range(c['tics']+1):
                current=grouped[op]
                for r in current:
                    if r[0]!='E':continue
                    code,birth,_=r[3:]
                    if code==1:
                        slot=next((s for s in range(len(generations)) if s not in occupied),len(generations))
                        if slot==len(generations):generations.append(1)
                        occupied[slot]=birth;base.BIRTH_ID[birth]=(slot,generations[slot])
                    elif code==3:
                        slot=base.BIRTH_ID[birth][0];del occupied[slot];generations[slot]+=1
                if op:
                    split=next(i for i,r in enumerate(current) if r[0]=='E' and r[3] in [0,3])
                    last=queued+events(current[:split])+[b'\0\0',b'\0\1']+events(current[split:])+[b'\0\2',b'\0\3'];queued=[]
                else: queued+=events(current);last=[]
                observed=[]
                for prefix,es in [('events.last',last),('events.pending',queued)]:
                    observed.append((prefix+'.count',n('Q',len(es))))
                    observed.extend((f'{prefix}.{i}',e) for i,e in enumerate(es))
                stream+=struct.pack('<III',which,op,len(observed))
                for path,value in observed:
                    path=path.encode();stream+=n('I',len(path))+path+n('Q',len(value))+value
                event_fields+=len(observed)-2;event_frames+=1
                if op not in capture_ops:continue
                _,fields=base.snapshot(c,selected,op,grouped,diagnostics=True)
                fields[2:3]=[('profile',n('I',2)),('registry.schema',b'DOOM-INFO-1.10-v1'),('context.schema',b'PLAYER-BODY-1-MEDIUM-NOTHRUST-v1')]
                p=next(r[3:] for r in current if r[0]=='P')
                mo,health,state,attacker,count,frags=p
                fields=[f for f in fields if not f[0].startswith('perception.player.')]
                revised=[]
                for path,value in fields:
                    revised.append((path,value))
                    if path.startswith('actor.') and path.endswith('.momz'):
                        # Rust inserts association immediately before momz.
                        revised.pop();prefix=path.rsplit('.',1)[0];slot=int(prefix.split('.')[1]);birth=occupied[slot]
                        association=next(r[4] for r in current if r[0]=='AP' and r[3]==birth)
                        assert association==(0 if birth==mo else -1)
                        revised.extend([(prefix+'.player',bytes([255 if association<0 else association])),(path,value)])
                fields=revised
                at=next(i for i,(p,_) in enumerate(fields) if p=='events.last.count')
                fields[at:at]=[('original_player.0.mo',base.opt(mo)),('original_player.0.health',n('i',health)),
                    ('original_player.0.state',n('B',state)),('original_player.0.attacker',base.opt(attacker)),
                    ('original_player.0.damage_count',n('i',count)),('original_player.0.frags',n('i',frags))]
                assert [(p,b) for p,b in fields if p.startswith('events.')]==observed
                data=frame(fields)
                name=f'm3g2_state_{which}_{op}.bin' if (which,op) in [(0,0),(6,72),(10,52),(13,3)] else '-'
                if name!='-':outputs['tests/reference/'+name]=data
                index.append(f'{which},{op},{op},{fnv(data):016x},{name},{len(data)},{len(fields)},{len(observed)-2}')
    outputs['tests/reference/m3g2_snapshots.csv']=('\n'.join(index)+'\n').encode()
    outputs['tests/reference/m3g2_events.bin']=bytes(stream)
    provenance=dict(schema='DSIM-v22',execution_profile=2,captures=len(index)-1,full_byte_fixtures=4,
        event_frames=event_frames,ordered_event_fields=event_fields,
        scope='Actual original player0/Demon12 source records and relationships. Separate player fields and derived association; unchanged framing. Complete ordered event fields every tic, compact selected full-state captures, four byte fixtures. No Rust output or historical fixture mutation.',
        inputs_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs},
        outputs_sha256={p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()})
    outputs['tests/reference/m3g2_snapshot_provenance.json']=(json.dumps(provenance,indent=2,sort_keys=True)+'\n').encode()
    for p,b in outputs.items():
        if check:assert (ROOT/p).read_bytes()==b,p
        else:(ROOT/p).write_bytes(b)
    print(f'{"Checked" if check else "Generated"} DSIM-v22: {len(index)-1} captures, {event_frames} event frames, {event_fields} ordered event fields, 4 full fixtures; {sum(map(len,outputs.values()))} bytes')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
