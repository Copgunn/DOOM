"""Independent DSIM24 weapon-pack fields from compiled-C observations only."""
import struct
import pistol_snapshot as pistol
execution=pistol.execution
physical=pistol.physical
base=pistol.base
n=base.n

def events(rows):
    return pistol.events([r for r in rows if r[0] not in {'WX','WK'}])

def weapon_fields(row):
    ready,pending,owned,clips,shells,latch,refire,bob,light,*sprites=row
    assert ready in range(4) and pending in [0,1,2,3,10]
    assert owned&3==3 and owned&~15==0 and owned&(1<<ready)
    assert 0<=clips<=200 and 0<=shells<=50 and latch in (0,1)
    assert 0<=refire<=2147483647 and 0<=bob<=0x100000 and light in (0,1,2)
    prefix='original_player.0.weapons'
    fields=[(prefix+'.present',b'\1'),(prefix+'.ready',n('B',ready)),
            (prefix+'.pending',n('B',pending)),(prefix+'.owned',n('H',owned)),
            (prefix+'.bullets',n('H',clips)),(prefix+'.shells',n('H',shells)),
            (prefix+'.attack_down',n('B',latch)),(prefix+'.refire',n('i',refire)),
            (prefix+'.bob',n('i',bob)),(prefix+'.extra_light',n('i',light))]
    assert len(sprites)==8
    for i in range(2):
        state,tics,x,y=sprites[4*i:4*i+4]
        for key,code,value in [('state','H',65535 if state<0 else state),('tics','i',tics),('x','i',x),('y','i',y)]:
            fields.append((f'{prefix}.psprite.{i}.{key}',n(code,value)))
    return fields

def build(cases,rows,maps,prefix,full):
    selected=iter((r[1],r[2])for r in rows if r[0]=='P')
    observations={(r[1],r[2]):r[3:] for r in rows if r[0]=='WX'}
    def frame(fields):
        case,op=next(selected)
        assert next(v for p,v in fields if p=='tic')==n('Q',op)
        at=next(i for i,(p,_)in enumerate(fields)if p=='events.last.count')
        revised=fields[:at]+weapon_fields(observations[case,op])+fields[at:]
        data=bytearray(b'DSIM'+struct.pack('<IQ',24,len(revised)))
        for path,value in revised:
            path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
        return bytes(data)
    old_events,old_frame=physical.events,physical.frame
    physical.events,physical.frame=events,frame
    try:
        outputs,inventory=execution.build(cases,rows,maps,8,b'PLAYER-DEMON-1-MEDIUM-GROUNDED-WEAPONS-v1',prefix,full)
        assert next(selected,None)is None
        index='tests/reference/'+prefix+'_snapshots.csv'
        lines=outputs[index].decode().splitlines()
        for i in range(1,len(lines)):
            values=lines[i].split(',');values[6]=str(int(values[6])+18);lines[i]=','.join(values)
        outputs[index]=('\n'.join(lines)+'\n').encode()
        return outputs,inventory
    finally:physical.events,physical.frame=old_events,old_frame
