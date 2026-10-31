"""DSIM26 armor projection from original C fields, reusing frozen assemblers."""
import struct
import pickup_snapshot as pickup
weapon=pickup.weapon;execution=pickup.execution;physical=pickup.physical;base=pickup.base;n=base.n
CONTEXT=b'PLAYER-DEMON-1-MEDIUM-GROUNDED-ARMOR-v1'
def armor_state(kind,points):
    assert kind in(0,1,2)
    assert (kind==0 and points==0)or(kind==1 and 1<=points<=100)or(kind==2 and 1<=points<=200)

def events(rows):
    # Keep the frozen things-before-lines candidate buffer. New rows are handled
    # only inside its already delimited weapon/event chunks. An AR observation
    # is player damage from melee/control, never an in-progress hitscan traversal.
    original=weapon.events
    def extra(chunk):
        out=[];pending=[]
        for row in chunk:
            if row[0]not in{'AR','XA'}:pending.append(row);continue
            out+=original(pending);pending=[]
            if row[0]=='AR':
                body,incoming,saved,effective,before,points,after,left=row[3:]
                armor_state(before,points);armor_state(after,left)
                assert 0<=saved<=incoming<=1000 and effective+saved==incoming
                out.append(bytes([24])+base.ident(body)+struct.pack('<iiiBiBi',incoming,saved,effective,before,points,after,left))
            else:
                requested,accepted,before,points,after,left=row[3:]
                assert requested in(1,2)and accepted in(0,1)
                armor_state(before,points);armor_state(after,left)
                out.append(bytes([23,7])+struct.pack('<BBBiBi',requested,accepted,before,points,after,left))
        return out+original(pending)
    translated=[]
    for r in rows:
        if r[0]=='AX':continue
        if r[0]=='PN'and r[3]==7:translated.append(['XA',*r[1:3],*r[4:]])
        elif r[0]=='PN'and r[3]==4 and r[4]in(10,11):
            # The frozen parser bounds its historical messages to1..9. Route
            # only the two new actual message observations through the same hook.
            translated.append(['XA_MESSAGE',*r[1:3],r[4]])
        else:translated.append(r)
    def messages(chunk):
        out=[];pending=[]
        for r in chunk:
            if r[0]!='XA_MESSAGE':pending.append(r);continue
            out+=extra(pending);pending=[];out.append(bytes([23,4,r[3]]))
        return out+extra(pending)
    weapon.events=messages
    try:return pickup.events(translated)
    finally:weapon.events=original

def build(cases,rows,maps,prefix,full):
    selected=iter((r[1],r[2])for r in rows if r[0]=='P')
    weapons={(r[1],r[2]):r[3:]for r in rows if r[0]=='WX'}
    pickups={(r[1],r[2]):r[3:]for r in rows if r[0]=='PX'}
    armor={(r[1],r[2]):r[3:]for r in rows if r[0]=='AX'}
    def frame(fields):
        case,op=next(selected);assert next(v for p,v in fields if p=='tic')==n('Q',op)
        at=next(i for i,(p,_)in enumerate(fields)if p=='events.last.count')
        bonus,items,message=pickups[case,op];kind,points=armor[case,op]
        assert 0<=bonus<=96 and items==0 and message in set(range(12))-{7}
        armor_state(kind,points)
        fields=fields[:at]+weapon.weapon_fields(weapons[case,op])+[
            ('original_player.0.pickups.present',b'\1'),
            ('original_player.0.pickups.bonus_count',n('i',bonus)),
            ('original_player.0.pickups.item_count',n('i',items)),
            ('original_player.0.pickups.message',n('B',message)),
            ('original_player.0.armor.type',n('B',kind)),
            ('original_player.0.armor.points',n('i',points)),
        ]+fields[at:]
        data=bytearray(b'DSIM'+struct.pack('<IQ',26,len(fields)))
        for path,value in fields:
            path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
        return bytes(data)
    old_events,old_frame=physical.events,physical.frame
    physical.events,physical.frame=events,frame
    try:
        outputs,inventory=execution.build(cases,rows,maps,10,CONTEXT,prefix,full)
        assert next(selected,None)is None
        path='tests/reference/'+prefix+'_snapshots.csv';lines=outputs[path].decode().splitlines()
        for i in range(1,len(lines)):
            v=lines[i].split(',');v[6]=str(int(v[6])+24);lines[i]=','.join(v)
        outputs[path]=('\n'.join(lines)+'\n').encode()
        return outputs,inventory
    finally:physical.events,physical.frame=old_events,old_frame
