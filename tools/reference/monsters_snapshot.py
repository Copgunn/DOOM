"""Independent DSIM28 projection from compiled original-C former-human rows.

Extends the frozen DSIM27 bonus projection without editing it: monster
P_AimLineAttack spans become MonsterEvent::Aim payloads, renamed SG shotgunner
rows use the historical shotgunner event layout, and player armor/pickup
observations raised inside a hitscan keep their source position."""
import struct
import bonus_snapshot as inherited
import m3f3_snapshot_goldens as source_events
base=inherited.base;n=base.n;execution=inherited.execution;physical=inherited.physical
# Captured before player_execution_snapshot.build swaps the module hook.
SOURCE_EVENTS=source_events.events
CONTEXT=b'PLAYER-MONSTERS-1-MEDIUM-GROUNDED-v1'
BODIES={}
def shotgun_bytes(r):
    code,actor,*a=r[3:];b=bytes([19,code])+base.ident(actor)
    if code==1:b+=n('B',a[0])
    elif code==2:b+=n('B',a[0])+n('I',a[1])+struct.pack('<ii',*a[2:])
    elif code==3:b+=bytes(a[:2])+base.opt(a[2])
    elif code==4:b+=n('B',a[0])
    elif code!=0:raise ValueError(r)
    return b
def events(rows):
    out=[];chunk=[];i=0;shooting=False
    while i<len(rows):
        r=rows[i]
        if r[0]=='X':shooting=r[3]==0
        if r[0]=='D'and r[3]==0 and not shooting:
            end=next(j for j in range(i,len(rows))if rows[j][0]=='J'and rows[j][3]==5)
            if rows[end][4]!=BODIES[r[1]]:
                out+=inherited.events(chunk);chunk=[]
                out+=SOURCE_EVENTS(rows[i:end+1])
                i=end+1;continue
        if r[0]=='SR':
            out+=inherited.events(chunk);chunk=[]
            out.append(bytes([12,7])+base.ident(r[3])+n('B',r[4]));i+=1;continue
        if r[0]=='SG':
            out+=inherited.events(chunk);chunk=[]
            out.append(shotgun_bytes(r));i+=1;continue
        if r[0]in{'AR','PN','IT'}and shooting:
            finish=next(row for row in rows[i:]if row[0]=='X'and row[3]==1)
            out+=inherited.events(chunk+[finish]);chunk=[]
            out+=inherited.events([r]);i+=1;continue
        chunk.append(r);i+=1
    return out+inherited.events(chunk)
def build(cases,rows,maps,prefix,full):
    BODIES.clear()
    BODIES.update({r[1]:r[4]for r in rows if r[0]=='PL'and r[3]==0})
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
        assert 0<=bonus<=96 and 0<=items<=total<=16 and message in set(range(14))-{7}
        assert 0<=credits<=kills<=8
        inherited.armor_state(kind,points)
        fields=fields[:at]+inherited.weapon.weapon_fields(weapons[case,op])+[
            ('original_player.0.pickups.present',b'\1'),
            ('original_player.0.pickups.bonus_count',n('i',bonus)),
            ('original_player.0.pickups.item_count',n('i',items)),
            ('original_player.0.pickups.message',n('B',message)),
            ('original_player.0.armor.type',n('B',kind)),
            ('original_player.0.armor.points',n('i',points)),
        ]+fields[at:]
        data=bytearray(b'DSIM'+struct.pack('<IQ',28,len(fields)))
        for path,value in fields:
            path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
        return bytes(data)
    old_events,old_frame=physical.events,physical.frame
    physical.events,physical.frame=events,frame
    try:
        outputs,inventory=execution.build(cases,rows,maps,12,CONTEXT,prefix,full)
        assert next(selected,None)is None
        path='tests/reference/'+prefix+'_snapshots.csv';lines=outputs[path].decode().splitlines()
        for i in range(1,len(lines)):
            v=lines[i].split(',');v[6]=str(int(v[6])+24);lines[i]=','.join(v)
        outputs[path]=('\n'.join(lines)+'\n').encode()
        return outputs,inventory
    finally:physical.events,physical.frame=old_events,old_frame
