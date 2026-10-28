#!/usr/bin/env python3
"""Independent DSIM-v4 encoder: C rows + explicit fixture setup, never Rust output."""
import argparse,csv,hashlib,json,re,struct
from pathlib import Path
import generate_m3c as source
import m3b_snapshot_goldens as v3
ROOT=source.ROOT
U=65536
def n(code,v):return struct.pack('<'+code,int(v))
def ident(i):return struct.pack('<IQ',i,1)
def opt(i):return bytes(13) if i<0 else b'\1'+ident(i)
def action(code,arg=0):return b'\0' if code==0 else b'\1'+n('H',code) if code<100 else bytes([2,code-100])+(n('H',arg) if code in [101,102] else b'')
def metadata(case):
    states=list(csv.DictReader((ROOT/'tests/reference/m3c_states.csv').open()));types=list(csv.DictReader((ROOT/'tests/reference/m3c_types.csv').open()))
    ss=[dict(name=r['name'],sprite=int(r['sprite']),frame=int(r['frame']),tics=int(r['tics']),action=int(r['action']),arg=0,next=int(r['next']),misc1=int(r['misc1']),misc2=int(r['misc2'])) for r in states]
    tt=[(r['name'],[int(r[f]) for f in source.FIELDS]) for r in types]
    if case>=6:
        for i,tics,next,code,arg in [(1,1,2,0,0),(2,0,3,100,0),(3,0,4,102,132),(4,0,8,101,5),(5,0,6,100,0),(6,2,0,100,0),(8,-1,0,0,0),(9,1,10,0,0),(10,0,11,104,0),(11,1,0,103,0)]:
            ss[i]=dict(name=f'PROBE_{i}',sprite=0,frame=i,tics=tics,action=code,arg=arg,next=next,misc1=0,misc2=0)
        for i in [131,132]:
            p=[0]*23;p[0]=-1;p[1]=1 if i==131 else 9;p[2]=100;p[16]=8*U;p[17]=16*U;p[21]=512;tt[i]=(f'NATIVE_{i}',p)
        if case>=7:
            ss[1].update(tics=-1,next=0,action=0);tt[131][1][9]=26;tt[131][1][14]=59
            for i,name in [(2,'A_Pain'),(3,'A_Scream'),(4,'A_XScream'),(5,'A_PlayerScream'),(6,'A_Fall')]:ss[i].update(tics=-1,next=0,action=source.ACTIONS.index(name),arg=0)
        if case==8:ss[1].update(tics=0,next=2,action=100)
    fields=[('registry.states.count',n('Q',len(ss)))]
    for i,s in enumerate(ss):
        name=s['name'].encode();b=n('B',len(name))+name+struct.pack('<HIi',s['sprite'],s['frame'],s['tics'])+action(s['action'],s['arg'])+struct.pack('<Hii',s['next'],s['misc1'],s['misc2']);fields.append((f'registry.state.{i}',b))
    fields.append(('registry.types.count',n('Q',len(tt))))
    for i,(name,p) in enumerate(tt):
        name=name.encode();fields.append((f'registry.type.{i}',n('B',len(name))+name+struct.pack('<23i',*p)))
    return ss,tt,fields

def thing_inputs():
    text=(ROOT/'tools/reference/m3c_driver.c').read_text();fragment=re.search(r'mapthing_t things\[\]=\{(.*?)\};',text).group(1)
    return [[int(x) for x in row.split(',')] for row in re.findall(r'\{([^{}]+)\}',fragment)]
def event(e,ss):
    code,i,arg=e
    if code==0:return [b'\5'+ident(i)]
    if code==1:return [b'\1'+ident(i),bytes([12,0])+ident(i)+n('H',arg)]
    if code in [2,3]:return [n('B',code)+ident(i)]
    if code==4:return [bytes([12,1])+ident(i)+n('H',arg)]
    if code==5:return [bytes([12,2])+ident(i)+action(arg,5 if arg==101 else 132 if arg==102 else 0)]
    if code==6:return [bytes([12,4])+ident(i)+opt(arg)]
    if code==7:return [bytes([12,3])+opt(i)+n('H',arg)]
    if code==8:return [bytes([12,7])+ident(i)+n('B',arg)]
    raise ValueError(e)
def encode(fields):
    out=b'DSIM'+struct.pack('<IQ',4,len(fields))
    for path,value in fields:
        path=path.encode();out+=n('I',len(path))+path+n('Q',len(value))+value
    return out
def run(check):
    rows=[line.split(',') for line in (ROOT/'tests/reference/m3c_trace.csv').read_text().splitlines()];outputs={};index=['case,step,tic,fnv1a64,bytes_fixture'];captures=0
    for case in range(9):
        ss,tt,registry=metadata(case);cr=[r for r in rows if int(r[1])==case];end=max(int(r[2]) for r in cr);order=[];spatial=[];pending=set();generations=[];last=[];queued=[];things=thing_inputs() if 1<=case<=5 else []
        selected=[t for t in things if t[3]>4 and t[3]!=11 and not t[4]&16 and t[4]&([1,1,2,4,4][case-1] if 1<=case<=5 else 2) and t[3]!=3004]
        maps,_=v3.map_fields(19);maps[0]=('map.things',b''.join(struct.pack('<5h',*t) for t in things))
        for step in range(end+1):
            er=[tuple(map(int,r[3:])) for r in cr if r[0]=='E' and int(r[2])==step];actors={int(r[3]):list(map(int,r[4:])) for r in cr if r[0]=='A' and int(r[2])==step};events=[]
            for e in er:
                code,i,arg=e
                if code==1:order.append(i);spatial.insert(0,i);generations.append(1)
                if code==2:pending.add(i);spatial=[a for a in spatial if a!=i]
                if code==3:order.remove(i);pending.remove(i);generations[i]+=1
                if case==0 and step==0 and code==4:events.append(bytes([12,5])+ident(i))
                events.extend(event(e,ss))
            external=step==0 or case==7 or case==8 and step==2
            if external:queued+=events
            else:last=queued+[b'\0\0',b'\0\1']+events+[b'\0\2',b'\0\3'];queued=[]
            tic=0 if case==7 else min(step,1) if case==8 else step
            rng=next(iter(actors.values()))[-1]
            fields=[('tic',n('Q',tic)),('tic_rate',n('I',35)),('profile',n('I',4)),('paused',b'\0'),('rng.gameplay',n('B',rng)),('rng.miscellaneous',b'\0'),('actors.options',bytes([case-1 if 1<=case<=5 else 2,0,int(1<=case<=5)])),('store.max_slots',n('I',1024)),('store.slot_count',n('Q',len(generations))),('store.head',opt(order[0] if order else -1)),('store.tail',opt(order[-1] if order else -1)),('thinker.order',b''.join(ident(i) for i in order))]
            for i,generation in enumerate(generations):
                p=f'actor.{i}';a=actors[i];alive,ty,state,tics,sprite,frame,health,reaction,flags,lastlook,z,radius,height,angle,target,_=a
                fields += [(p+'.generation',n('Q',generation)),(p+'.retired',b'\0'),(p+'.occupied',n('B',alive))]
                if not alive:continue
                pos=order.index(i);thing=selected[i] if 1<=case<=5 else None;x,y=(thing[:2] if thing else [64 if case==0 and i==1 else 0,0])
                fields += [(p+'.pending_removal',n('B',i in pending)),(p+'.previous',opt(order[pos-1] if pos else -1)),(p+'.next',opt(order[pos+1] if pos+1<len(order) else -1))]
                for label,value in [('x',x*U),('y',y*U),('z',z),('momx',0),('momy',0),('radius',radius),('height',height),('floor',0),('ceiling',128*U)]:fields.append((p+'.'+label,n('i',value)))
                fields += [(p+'.angle',n('I',angle)),(p+'.lastlook',n('B',lastlook)),(p+'.flags',n('I',flags)),(p+'.type',n('H',ty)),(p+'.state',n('H',65535 if state<0 else state)),(p+'.tics',n('i',tics)),(p+'.health',n('i',health)),(p+'.reaction_time',n('i',reaction)),(p+'.sprite',n('H',sprite)),(p+'.frame',n('I',frame)),(p+'.target',opt(target)),(p+'.death_requested',n('B',case==0 and i==0)),(p+'.spawnpoint',b'\0' if thing is None else b'\1'+struct.pack('<5h',*thing)),(p+'.momz',bytes(4)),(p+'.subsector',bytes(2)),(p+'.sector',bytes(2))]
            fields += [('spatial.link_order',b''.join(ident(i) for i in spatial)),('world.blockmap_policy',b'\1'),('world.conservative_omissions',bytes(4)),('sectors.count',n('I',1)),('sector.0.floor',bytes(4)),('sector.0.ceiling',n('i',128*U)),('sector.0.actors',b''.join(ident(i) for i in spatial if not actors[i][8]&8))]
            for i in range(4):fields += [(f'player.{i}.actor',bytes(13)),(f'player.{i}.forward',b'\0'),(f'player.{i}.side',b'\0'),(f'player.{i}.angle_turn',bytes(2)),(f'player.{i}.buttons',b'\0')]
            for prefix,es in [('events.last',last),('events.pending',queued)]:fields += [(prefix+'.count',n('Q',len(es)))]+[(f'{prefix}.{j}',e) for j,e in enumerate(es)]
            data=encode(fields+maps+registry);digest=0xcbf29ce484222325
            for byte in data:digest=((digest^byte)*0x100000001b3)&0xffffffffffffffff
            name=f'm3c_state_{case}_{step}.bin' if (case==0 and step in [0,10,20,60,72,73]) or case==6 or case>=7 or step==0 else '-'
            if name!='-':outputs['tests/reference/'+name]=data
            index.append(f'{case},{step},{tic},{digest:016x},{name}');captures+=1
    outputs['tests/reference/m3c_snapshots.csv']=('\n'.join(index)+'\n').encode()
    inputs=['tools/reference/m3c_snapshot_goldens.py','tools/reference/generate_m3c.py','tools/reference/m3c_driver.c','tools/reference/m3b_snapshot_goldens.py','tests/reference/m3c_trace.csv','tests/reference/m3c_states.csv','tests/reference/m3c_types.csv','tests/reference/m3b_map_19.bin']
    provenance={'schema':'DSIM-v4','captures':captures,'scope':'Explicit static setups, C state/action/RNG/order values, project schema; never Rust expected output','inputs_sha256':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()}}
    outputs['tests/reference/m3c_snapshot_provenance.json']=(json.dumps(provenance,indent=2)+'\n').encode()
    for name,data in outputs.items():
        p=ROOT/name
        if check:assert p.read_bytes()==data,name
        else:p.write_bytes(data)
    print(('Checked' if check else 'Generated')+f' {captures} independent DSIM-v4 digests and {len(outputs)-2} full states.')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
