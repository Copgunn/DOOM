"""Independent DSIM27 projection, from original C fields and ordered events."""
import struct
import armor_snapshot as historical
pickup=historical.pickup;weapon=historical.weapon;execution=historical.execution
physical=historical.physical;base=historical.base;n=base.n
CONTEXT=b'PLAYER-DEMON-1-MEDIUM-GROUNDED-BONUSES-v1'
def armor_state(kind,points):
    assert kind in (0,1,2)
    assert (kind==0 and points==0)or(kind in (1,2)and 1<=points<=200)

def events(rows):
    filtered=[];role=None
    for r in rows:
        if r[0]=='TR':role=r[4];assert role in (0,1,2);continue
        if role==2 and r[0]in {'D','B','U','I','T'}:continue
        filtered.append(r)
    out=[];chunk=[];candidates=[]
    for r in filtered:
        if r[0]in {'AX','PX','PQ','RI'}:continue
        if r[0]in {'C','O'}:candidates.append(r);continue
        if r[0]=='M':chunk+=candidates;candidates=[]
        if r[0]not in {'PN','AR','IT'}:chunk.append(r);continue
        out+=weapon.events(chunk);chunk=[]
        if r[0]=='IT':
            actor,total=r[3:];assert 1<=total<=16
            out.append(bytes([21,2])+base.ident(actor)+n('i',total));continue
        if r[0]=='AR':
            body,incoming,saved,effective,before,points,after,left=r[3:]
            armor_state(before,points);armor_state(after,left)
            assert 0<=saved<=incoming<=1000 and effective+saved==incoming
            out.append(bytes([24])+base.ident(body)+struct.pack('<iiiBiBi',incoming,saved,effective,before,points,after,left));continue
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
            assert len(v)==1 and v[0]in set(range(1,14))-{7}
            payload+=n('B',v[0])
        elif code==5:
            count,items=v;assert 0<=count<=96 and 0<=items<=16
            payload+=struct.pack('<ii',count,items)
        elif code==6:
            item,outcome=v;assert outcome in range(4)
            payload+=base.ident(item)+n('B',outcome)
        elif code==7:
            requested,accepted,before,points,after,left=v
            assert requested in(1,2)and accepted in(0,1)
            armor_state(before,points);armor_state(after,left)
            payload+=struct.pack('<BBBiBi',requested,accepted,before,points,after,left)
        elif code==8:
            before,after,body=v
            assert 1<=before<=200 and after==body==min(200,before+1)
            payload+=struct.pack('<iii',before,after,body)
        elif code==9:
            before,points,after,left=v
            armor_state(before,points);armor_state(after,left)
            assert after==(before or 1)and left==min(200,points+1)
            payload+=struct.pack('<BiBi',before,points,after,left)
        elif code==10:
            before,after=v;assert 0<=before<after<=16 and after==before+1
            payload+=struct.pack('<ii',before,after)
        else:raise ValueError(r)
        out.append(payload)
    assert not candidates,'source collision prefix lacks its movement result'
    return out+weapon.events(chunk)

def build(cases,rows,maps,prefix,full):
    selected=iter((r[1],r[2])for r in rows if r[0]=='P')
    weapons={(r[1],r[2]):r[3:]for r in rows if r[0]=='WX'}
    pickups={(r[1],r[2]):r[3:]for r in rows if r[0]=='PX'}
    armor={(r[1],r[2]):r[3:]for r in rows if r[0]=='AX'}
    def frame(fields):
        case,op=next(selected);assert next(v for p,v in fields if p=='tic')==n('Q',op)
        at=next(i for i,(p,_)in enumerate(fields)if p=='events.last.count')
        bonus,items,message=pickups[case,op];kind,points=armor[case,op]
        total=struct.unpack('<i',next(v for p,v in fields if p=='level.total_items'))[0]
        assert 0<=bonus<=96 and 0<=items<=total<=16 and message in set(range(14))-{7}
        armor_state(kind,points)
        fields=fields[:at]+weapon.weapon_fields(weapons[case,op])+[
            ('original_player.0.pickups.present',b'\1'),
            ('original_player.0.pickups.bonus_count',n('i',bonus)),
            ('original_player.0.pickups.item_count',n('i',items)),
            ('original_player.0.pickups.message',n('B',message)),
            ('original_player.0.armor.type',n('B',kind)),
            ('original_player.0.armor.points',n('i',points)),
        ]+fields[at:]
        data=bytearray(b'DSIM'+struct.pack('<IQ',27,len(fields)))
        for path,value in fields:
            path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
        return bytes(data)
    old_events,old_frame=physical.events,physical.frame
    physical.events,physical.frame=events,frame
    try:
        outputs,inventory=execution.build(cases,rows,maps,11,CONTEXT,prefix,full)
        assert next(selected,None)is None
        path='tests/reference/'+prefix+'_snapshots.csv';lines=outputs[path].decode().splitlines()
        for i in range(1,len(lines)):
            v=lines[i].split(',');v[6]=str(int(v[6])+24);lines[i]=','.join(v)
        outputs[path]=('\n'.join(lines)+'\n').encode()
        return outputs,inventory
    finally:physical.events,physical.frame=old_events,old_frame
