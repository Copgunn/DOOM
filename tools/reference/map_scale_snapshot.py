"""Independent DSIM30 projection from compiled original-C real-map-scale rows.

Extends the frozen DSIM29 projection without editing it: the same field layout
and event vocabulary under version 30, policy 14, the map-scale context, the
256-slot store and the derived 192 live-fireball bound of the new policy."""
import struct
import imps_snapshot as inherited
base=inherited.base;n=base.n;execution=inherited.execution;physical=inherited.physical
CONTEXT=b'PLAYER-MONSTERS-MAP-SCALE-1-MEDIUM-GROUNDED-v1'
VERSION=30;POLICY=14;MAX_SLOTS=256;MAX_LIVE_PROJECTILES=192;FIREBALL=31
events=inherited.events
def build(cases,rows,maps,prefix,full):
    inherited.inherited.BODIES.clear()
    inherited.inherited.BODIES.update({r[1]:r[4]for r in rows if r[0]=='PL'and r[3]==0})
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
        inherited.inherited.inherited.armor_state(kind,points)
        fields=[(p,n('I',MAX_SLOTS)if p=='store.max_slots'else v)for p,v in fields]
        fields=fields[:at]+inherited.inherited.inherited.weapon.weapon_fields(weapons[case,op])+[
            ('original_player.0.pickups.present',b'\1'),
            ('original_player.0.pickups.bonus_count',n('i',bonus)),
            ('original_player.0.pickups.item_count',n('i',items)),
            ('original_player.0.pickups.message',n('B',message)),
            ('original_player.0.armor.type',n('B',kind)),
            ('original_player.0.armor.points',n('i',points)),
        ]+fields[at:]
        data=bytearray(b'DSIM'+struct.pack('<IQ',VERSION,len(fields)))
        for path,value in fields:
            path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
        return bytes(data)
    old_events,old_frame=physical.events,physical.frame
    physical.events,physical.frame=events,frame
    try:
        outputs,inventory=execution.build(cases,rows,maps,POLICY,CONTEXT,prefix,full)
        assert next(selected,None)is None
        path='tests/reference/'+prefix+'_snapshots.csv';lines=outputs[path].decode().splitlines()
        for i in range(1,len(lines)):
            v=lines[i].split(',');v[6]=str(int(v[6])+24);lines[i]=','.join(v)
        outputs[path]=('\n'.join(lines)+'\n').encode()
        return outputs,inventory
    finally:physical.events,physical.frame=old_events,old_frame
