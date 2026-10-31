"""Independent planned DSIM25 projection from C rows; no Rust observations."""
import collections,csv,json,struct,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import weapon_snapshot as weapon
execution=weapon.execution;physical=weapon.physical;base=weapon.base;n=base.n

def events(rows):
    # A trace's actual source callback identifies its role. Geometry-only slide
    # traversal rows are read-only observations, unlike hitscan event traces.
    filtered=[];role=None
    for row in rows:
        if row[0]=='TR':role=row[4];assert role in (0,1,2);continue
        if role==2 and row[0]in {'D','B','U','I','T'}:continue
        filtered.append(row)
    rows=filtered
    out=[];chunk=[]
    for r in rows:
        if r[0]in {'PX','PQ','RI'}:continue
        if r[0]!='PN':chunk.append(r);continue
        out+=weapon.events(chunk);chunk=[]
        code,*v=r[3:];payload=bytes([23,code])
        if code==0:
            item,body,sprite,delta,height,health=v
            payload+=base.ident(item)+base.ident(body)+n('H',sprite)+struct.pack('<iii',delta,height,health)
        elif code==1:
            ammo,accepted,before,after,ready,pending=v
            assert ammo in(0,1)and accepted in(0,1)
            payload+=bytes([ammo,accepted])+struct.pack('<ii',before,after)+bytes([ready,pending])
        elif code==2:
            accepted,before,after,body=v
            payload+=n('B',accepted)+struct.pack('<iii',before,after,body)
        elif code==3:
            w,accepted,before,after,pending_before,pending=v
            assert w in(2,3)and accepted in(0,1)
            payload+=bytes([w,accepted])+struct.pack('<HH',before,after)+bytes([pending_before,pending])
        elif code==4:
            assert len(v)==1 and v[0]in range(1,10)
            payload+=n('B',v[0])
        elif code==5:
            count,items=v;assert 0<=count<=96 and items==0
            payload+=struct.pack('<ii',count,items)
        elif code==6:
            item,outcome=v;assert outcome in range(4)
            payload+=base.ident(item)+n('B',outcome)
        else:raise ValueError(r)
        out.append(payload)
    return out+weapon.events(chunk)

def build(cases,rows,maps,prefix,full):
    selected=iter((r[1],r[2])for r in rows if r[0]=='P')
    weapons={(r[1],r[2]):r[3:]for r in rows if r[0]=='WX'}
    pickups={(r[1],r[2]):r[3:]for r in rows if r[0]=='PX'}
    def frame(fields):
        case,op=next(selected)
        assert next(v for p,v in fields if p=='tic')==n('Q',op)
        at=next(i for i,(p,_)in enumerate(fields)if p=='events.last.count')
        bonus,items,message=pickups[case,op]
        assert 0<=bonus<=96 and items==0 and message in range(10)
        fields=fields[:at]+weapon.weapon_fields(weapons[case,op])+[
            ('original_player.0.pickups.present',b'\1'),
            ('original_player.0.pickups.bonus_count',n('i',bonus)),
            ('original_player.0.pickups.item_count',n('i',items)),
            ('original_player.0.pickups.message',n('B',message)),
        ]+fields[at:]
        data=bytearray(b'DSIM'+struct.pack('<IQ',25,len(fields)))
        for path,value in fields:
            path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
        return bytes(data)
    old_events,old_frame=physical.events,physical.frame
    physical.events,physical.frame=events,frame
    try:
        outputs,inventory=execution.build(cases,rows,maps,9,b'PLAYER-DEMON-1-MEDIUM-GROUNDED-PICKUPS-v1',prefix,full)
        assert next(selected,None)is None
        index='tests/reference/'+prefix+'_snapshots.csv';lines=outputs[index].decode().splitlines()
        for i in range(1,len(lines)):
            values=lines[i].split(',');values[6]=str(int(values[6])+22);lines[i]=','.join(values)
        outputs[index]=('\n'.join(lines)+'\n').encode()
        return outputs,inventory
    finally:physical.events,physical.frame=old_events,old_frame

