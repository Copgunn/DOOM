"""Independent DSIM29 projection from compiled original-C Imp/fireball rows.

Extends the frozen DSIM28 projection without editing it: MS rows become the
existing projectile event layout (tag 15), interleaved in source order with
the shared movement, state and damage events. A P_TryMove collision prefix can
straddle a contact draw, so its C/O rows wait for their M row."""
import struct
import monsters_snapshot as inherited
base=inherited.base;n=base.n;execution=inherited.execution;physical=inherited.physical
CONTEXT=b'PLAYER-MONSTERS-IMPS-1-MEDIUM-GROUNDED-v1'
MAX_LIVE_PROJECTILES=16
FIREBALL=31
def missile_bytes(v):
    code,*a=v;b=bytes([15,code])
    if code==0:return b+base.ident(a[0])
    if code==1:return b+base.ident(a[0])+struct.pack('<Iiii',*a[1:])
    if code==2:return b+bytes(a)
    if code==4:return b+base.ident(a[0])+struct.pack('<4i',*a[1:])
    if code==5:return b+base.ident(a[0])+n('B',a[1])+base.ident(a[2])+n('H',a[3])
    raise ValueError(v)
def events(rows):
    out=[];chunk=[];held=[]
    for r in rows:
        if r[0]in{'C','O'}:held.append(r);continue
        if r[0]=='M':chunk+=held+[r];held=[];continue
        if r[0]=='MS':
            out+=inherited.events(chunk);chunk=[]
            out.append(missile_bytes(r[3:]));continue
        chunk.append(r)
    assert not held,'collision prefix lacks its movement result'
    return out+inherited.events(chunk)
def build(cases,rows,maps,prefix,full):
    inherited.BODIES.clear()
    inherited.BODIES.update({r[1]:r[4]for r in rows if r[0]=='PL'and r[3]==0})
    selected=iter((r[1],r[2])for r in rows if r[0]=='P')
    weapons={(r[1],r[2]):r[3:]for r in rows if r[0]=='WX'}
    pickups={(r[1],r[2]):r[3:]for r in rows if r[0]=='PX'}
    armor={(r[1],r[2]):r[3:]for r in rows if r[0]=='AX'}
    def frame(fields):
        case,op=next(selected);assert next(v for p,v in fields if p=='tic')==n('Q',op)
        at=next(i for i,(p,_)in enumerate(fields)if p=='events.last.count')
        bonus,items,message=pickups[case,op];kind,points=armor[case,op]
        total=struct.unpack('<i',next(v for p,v in fields if p=='level.total_items'))[0]
        kills,credits=(struct.unpack('<i',next(v for p,v in fields if p==name))[0]for name in['level.total_kills','level.single_player_kill_count'])
        live=sum(p.endswith('.type')and struct.unpack('<H',v)[0]==FIREBALL for p,v in fields)
        assert 0<=bonus<=96 and 0<=items<=total<=16 and message in set(range(14))-{7}
        assert 0<=credits<=kills<=8 and live<=MAX_LIVE_PROJECTILES
        inherited.inherited.armor_state(kind,points)
        fields=fields[:at]+inherited.inherited.weapon.weapon_fields(weapons[case,op])+[
            ('original_player.0.pickups.present',b'\1'),
            ('original_player.0.pickups.bonus_count',n('i',bonus)),
            ('original_player.0.pickups.item_count',n('i',items)),
            ('original_player.0.pickups.message',n('B',message)),
            ('original_player.0.armor.type',n('B',kind)),
            ('original_player.0.armor.points',n('i',points)),
        ]+fields[at:]
        data=bytearray(b'DSIM'+struct.pack('<IQ',29,len(fields)))
        for path,value in fields:
            path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
        return bytes(data)
    old_events,old_frame=physical.events,physical.frame
    physical.events,physical.frame=events,frame
    try:
        outputs,inventory=execution.build(cases,rows,maps,13,CONTEXT,prefix,full)
        assert next(selected,None)is None
        path='tests/reference/'+prefix+'_snapshots.csv';lines=outputs[path].decode().splitlines()
        for i in range(1,len(lines)):
            v=lines[i].split(',');v[6]=str(int(v[6])+24);lines[i]=','.join(v)
        outputs[path]=('\n'.join(lines)+'\n').encode()
        return outputs,inventory
    finally:physical.events,physical.frame=old_events,old_frame
