#!/usr/bin/env python3
"""Reusable independent DSIM22 projection of original player/source capture rows.

Historical encoders remain frozen. This utility consumes C observations and an
explicit schema capability, never Rust data or executable-world restoration.
"""
import collections
import shutil
import struct
import tempfile
from pathlib import Path
import m3g2_snapshot_goldens as player
base=player.base
n=base.n

def events(rows):
    out,chunk=[],[]
    for r in rows:
        if r[0]!='SL':chunk.append(r);continue
        out+=player.events(chunk);chunk=[]
        actor,line=r[3:]
        out.append(b'\x09'+base.ident(actor)+n('H',line))
    return out+player.events(chunk)

def frame(fields):
    data=bytearray(b'DSIM'+struct.pack('<IQ',22,len(fields)))
    for path,value in fields:
        path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
    return bytes(data)

def build(cases,rows,map_file,policy,context,prefix,full):
    """Return new independent evidence; case inputs/selected full fixtures explicit."""
    outputs={};index=['case,operation,tic,fnv1a64,bytes_fixture,bytes,fields,event_fields']
    # Versioned compact test-evidence framing, not a different DSIM wire layout.
    # Each frame has case/op/count(last)/count(pending), then length+payloads.
    stream=bytearray(b'PBE1');event_fields=0;event_frames=0
    with tempfile.TemporaryDirectory(prefix='doom-player-projection-') as directory:
        temp=Path(directory);(temp/'tests/reference').mkdir(parents=True)
        shutil.copyfile(map_file,temp/'tests/reference/m3f3_map_0.bin')
        base.ROOT=temp;base.map_fields.cache_clear();base.metadata=lambda _c:[];base.events=events
        for which,values in enumerate(cases):
            c={k:int(v) for k,v in values.items() if k!='label'}
            c.update(mode=4,map=0,skill=2,no_monsters=0)
            selected=[r for r in rows if r[1]==which];grouped=collections.defaultdict(list)
            for r in selected:grouped[r[2]].append(r)
            captures={r[2] for r in selected if r[0]=='P'}
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
                else:queued+=events(current);last=[]
                stream+=struct.pack('<4I',which,op,len(last),len(queued))
                observed=[]
                for family,es in [('events.last',last),('events.pending',queued)]:
                    observed.append((family+'.count',n('Q',len(es))))
                    for i,e in enumerate(es):
                        stream+=n('I',len(e))+e
                        observed.append((f'{family}.{i}',e))
                event_fields+=len(observed)-2;event_frames+=1
                if op not in captures:continue
                _,fields=base.snapshot(c,selected,op,grouped,diagnostics=True)
                fields[2:3]=[('profile',n('I',policy)),('registry.schema',b'DOOM-INFO-1.10-v1'),('context.schema',context)]
                mo,health,state,attacker,count,frags=next(r[3:] for r in current if r[0]=='P')
                fields=[f for f in fields if not f[0].startswith('perception.player.')]
                revised=[]
                for path,value in fields:
                    revised.append((path,value))
                    if path.startswith('actor.') and path.endswith('.momz'):
                        revised.pop();stem=path.rsplit('.',1)[0];slot=int(stem.split('.')[1]);birth=occupied[slot]
                        association=next(r[4] for r in current if r[0]=='AP' and r[3]==birth)
                        assert association==(0 if birth==mo else -1)
                        revised.extend([(stem+'.player',bytes([255 if association<0 else association])),(path,value)])
                fields=revised;at=next(i for i,(p,_) in enumerate(fields) if p=='events.last.count')
                fields[at:at]=[('original_player.0.mo',base.opt(mo)),('original_player.0.health',n('i',health)),
                    ('original_player.0.state',n('B',state)),('original_player.0.attacker',base.opt(attacker)),
                    ('original_player.0.damage_count',n('i',count)),('original_player.0.frags',n('i',frags))]
                assert [(p,b) for p,b in fields if p.startswith('events.')]==observed
                data=frame(fields);name=f'{prefix}_state_{which}_{op}.bin' if (which,op) in full else '-'
                if name!='-':outputs['tests/reference/'+name]=data
                index.append(f'{which},{op},{op},{player.fnv(data):016x},{name},{len(data)},{len(fields)},{len(observed)-2}')
    assert sum('_state_' in p for p in outputs)==len(full)
    outputs[f'tests/reference/{prefix}_snapshots.csv']=('\n'.join(index)+'\n').encode()
    outputs[f'tests/reference/{prefix}_events.bin']=bytes(stream)
    return outputs,dict(captures=len(index)-1,full_byte_fixtures=len(full),event_frames=event_frames,ordered_event_fields=event_fields)
