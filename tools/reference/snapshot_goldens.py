#!/usr/bin/env python3
"""Independent DSIM-v1 encoding of C coasting vectors and explicit setup state.

No Rust output is read. Static bytes come only from the original synthetic seed.
These are project-schema goldens with C-derived XY values, not engine savegames.
"""
from pathlib import Path
import argparse
import csv
import struct

ROOT=Path(__file__).resolve().parents[2]


def number(kind,value):
    return struct.pack('<'+kind,value)


def ident():
    return struct.pack('<IQ',0,1)


def map_fields():
    data=(ROOT/'fuzz/corpus/map/classic_square').read_bytes()
    count,offset=struct.unpack_from('<ii',data,4)
    entries=[]
    for i in range(count):
        start,size,name=struct.unpack_from('<ii8s',data,offset+16*i)
        entries.append((name.rstrip(b'\0').decode(),data[start:start+size]))
    names=['things','linedefs','sidedefs','vertices','segs','subsectors','nodes','sectors','reject']
    fields=[('map.'+n,entries[i+1][1]) for i,n in enumerate(names)]
    block=entries[10][1]
    words=struct.unpack('<'+'H'*(len(block)//2),block)
    ranges=b''
    for start in words[4:4+words[2]*words[3]]:
        end=start
        while words[end]!=65535:
            end+=1
        ranges+=struct.pack('<II',start,end)
    return fields+[('map.blockmap.words',block),('map.blockmap.cell_ranges',ranges)]


def snapshot(row):
    tic=int(row['tic'])+1
    fields=[('tic',number('Q',tic)),('tic_rate',number('I',35)),('profile',number('I',1)),('paused',b'\0'),('rng.gameplay',b'\1'),('rng.miscellaneous',b'\0'),('store.max_slots',number('I',1024)),('store.slot_count',number('Q',1)),('store.head',b'\1'+ident()),('store.tail',b'\1'+ident()),('thinker.order',ident()),('actor.0.generation',number('Q',1)),('actor.0.retired',b'\0'),('actor.0.occupied',b'\1'),('actor.0.pending_removal',b'\0'),('actor.0.previous',bytes(13)),('actor.0.next',bytes(13))]
    for name,value in [('x',int(row['x'])),('y',int(row['y'])),('z',-16*65536),('momx',int(row['momx'])),('momy',int(row['momy'])),('radius',8*65536),('height',56*65536),('floor',-16*65536),('ceiling',128*65536)]:
        fields.append(('actor.0.'+name,number('i',value)))
    fields += [('actor.0.angle',number('I',0)),('actor.0.lastlook',b'\0')]
    for i in range(4):
        fields += [(f'player.{i}.actor',bytes(13)),(f'player.{i}.forward',b'\0'),(f'player.{i}.side',b'\0'),(f'player.{i}.angle_turn',bytes(2)),(f'player.{i}.buttons',b'\0')]
    assert row['attempts']=='1' and row['candidates']=='0:1:2:3'
    move=b'\6'+ident()+struct.pack('<iiHBI',int(row['x']),int(row['y']),65535,1,4)+struct.pack('<4H',0,1,2,3)
    events=([b'\1'+ident()] if tic==1 else [])+[b'\0\0',b'\0\1',b'\5'+ident(),move,b'\0\2',b'\0\3']
    fields += [('events.last.count',number('Q',len(events)))]+[(f'events.last.{i}',value) for i,value in enumerate(events)]+[('events.pending.count',number('Q',0))]
    fields += map_fields()
    out=b'DSIM'+struct.pack('<IQ',1,len(fields))
    for path,value in fields:
        path=path.encode()
        out+=number('I',len(path))+path+number('Q',len(value))+value
    return out


def run(check):
    with (ROOT/'tests/reference/movement.csv').open() as source:
        rows=[r for r in csv.DictReader(source) if r['case']=='0']
    assert len(rows)==12
    goldens=['tic,fnv1a64,x,y,momx,momy']
    outputs={}
    for row in rows:
        encoded=snapshot(row)
        h=0xcbf29ce484222325
        for byte in encoded:
            h=((h^byte)*0x100000001b3)&0xffffffffffffffff
        goldens.append(f"{int(row['tic'])+1},{h:016x},{row['x']},{row['y']},{row['momx']},{row['momy']}")
        if row['tic']=='0':
            outputs['tests/reference/coast_first.bin']=encoded
    outputs['tests/reference/snapshots.csv']=('\n'.join(goldens)+'\n').encode()
    for name,value in outputs.items():
        target=ROOT/name
        if check:
            assert target.read_bytes()==value,name
        else:
            target.write_bytes(value)
    print('Checked independent snapshot goldens.' if check else 'Generated independent snapshot goldens.')


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--check',action='store_true')
    run(parser.parse_args().check)
