#!/usr/bin/env python3
"""Independent DSIM-v17 from original-C rows, public fixture inputs and schema.
No Rust output. All captures, including transient sight/scan/A_Look/noise events.
"""
import argparse,csv,hashlib,json,struct,functools
from pathlib import Path
import m3c_snapshot_goldens as v4
ROOT=Path(__file__).resolve().parents[2]
U=65536
n=v4.n
BIRTH_ID={}
def ident(i):
 return struct.pack('<IQ',*BIRTH_ID[i])
def opt(i):
 return bytes(13) if i<0 else b'\1'+ident(i)

@functools.lru_cache(maxsize=128)
def map_fields(which):
 data=(ROOT/f'tests/reference/m3d3j_map_{which}.bin').read_bytes();count,off=struct.unpack_from('<ii',data,4);lumps=[]
 for i in range(count):
  start,size,_=struct.unpack_from('<ii8s',data,off+16*i);lumps.append(data[start:start+size])
 names=['things','linedefs','sidedefs','vertices','segs','subsectors','nodes','sectors','reject'];fields=[('map.'+name,lumps[i+1]) for i,name in enumerate(names)]
 words=struct.unpack('<'+'H'*(len(lumps[10])//2),lumps[10]);ranges=b'';cells=[]
 for start in words[4:4+words[2]*words[3]]:
  end=start
  while words[end]!=65535:end+=1
  ranges+=struct.pack('<II',start,end);cells.append(set(words[start:end]))
 vertices=list(struct.iter_unpack('<hh',lumps[4]));ox,oy=struct.unpack_from('<hh',lumps[10]);w,h=words[2:4];omissions=0
 for i,line in enumerate(struct.iter_unpack('<7H',lumps[2])):
  p=[vertices[v] for v in line[:2]];xl=max(0,(min(v[0] for v in p)-ox)//128);xh=min(w-1,(max(v[0] for v in p)-ox)//128);yl=max(0,(min(v[1] for v in p)-oy)//128);yh=min(h-1,(max(v[1] for v in p)-oy)//128)
  omissions+=sum(i not in cells[y*w+x] for x in range(xl,xh+1) for y in range(yl,yh+1))
 return fields+[('map.blockmap.words',lumps[10]),('map.blockmap.cell_ranges',ranges)],lumps[8],omissions

def metadata(c):
 return metadata_values(tuple(sorted(c.items())))
@functools.lru_cache(maxsize=256)
def metadata_values(items):
 c=dict(items)
 states,types,fields=v4.metadata(0)
 defs=[]
 aliases=[(1,'S_SARG_STND',3),(3,'S_SARG_STND2',1),(2,'S_SARG_RUN1',15),(15,'S_SARG_RUN2',16),(16,'S_SARG_RUN3',17),(17,'S_SARG_RUN4',18),(18,'S_SARG_RUN5',19),(19,'S_SARG_RUN6',20),(20,'S_SARG_RUN7',21),(21,'S_SARG_RUN8',2),(4,'S_SARG_ATK1',5),(5,'S_SARG_ATK2',6),(6,'S_SARG_ATK3',2),(7,'S_SARG_PAIN',8),(8,'S_SARG_PAIN2',2),(9,'S_SARG_DIE1',10),(10,'S_SARG_DIE2',11),(11,'S_SARG_DIE3',12),(12,'S_SARG_DIE4',13),(13,'S_SARG_DIE5',14),(14,'S_SARG_DIE6',0)]
 for i,name,nextstate in aliases:
  state=next(v for v in states if v['name']==name)
  defs.append((i,state['frame'],state['tics'],state['action'],nextstate,'FIXTURE_'+name[2:]))
 for i,frame,tics,action,nextstate,name in defs:
  raw=name.encode();b=n('B',len(raw))+raw+struct.pack('<HIi',39,frame,tics)+v4.action(action)+struct.pack('<Hii',nextstate,0,0)
  fields[next(j for j,f in enumerate(fields) if f[0]==f'registry.state.{i}')]=(f'registry.state.{i}',b)
 for i in [136,135]:
  p=types[12 if i==136 else 0][1].copy()
  changes=[(0,-1),(2,1000),(8,0),(16,16*U),(21,6)] if i==135 else [(0,-1),(1,1),(3,2),(7,7),(10,4),(12,9),(13,0),(21,4194310),(22,0)]
  for key,value in changes:p[key]=value
  name=('FIXTURE_DAMAGE_TARGET' if i==135 else 'FIXTURE_DEMON_MAP_CONTEXT_136').encode();fields[next(j for j,f in enumerate(fields) if f[0]==f'registry.type.{i}')]=(f'registry.type.{i}',n('B',len(name))+name+struct.pack('<23i',*p))
 return fields

def visit(v):
 code,*a=v;b=n('B',code)
 if code in [1,2]:b+=n('H',a[0])+n('B',a[1])
 elif code==3:b+=n('H',a[0])
 elif code==4:b+=struct.pack('<HIHB',*a)
 elif code==5:b+=struct.pack('<HBBB',*a)
 elif code==6:b+=struct.pack('<Hii',*a)
 elif code==7:b+=struct.pack('<Hiii',*a)
 elif code==8:b+=struct.pack('<HB',*a)
 else:raise ValueError(v)
 return b

def sight(start,rows,end):
 query,_,observer,target,pair,rejected=start[3:];q=end[3:];assert q[0]==query
 payload=ident(observer)+ident(target)+n('B',q[1])+n('I',pair)+bytes([rejected,not rejected])
 if not rejected:payload+=struct.pack('<iii',*q[2:])
 visits=[r[4:] for r in rows if r[0]=='V' and r[3]==query and r[4]!=0]
 return bytes([16,10])+payload+n('I',len(visits))+b''.join(visit(v) for v in visits)

def shared_events(rows):
 out=[];i=0
 while i<len(rows):
  r=rows[i];tag=r[0];v=r[3:];i+=1
  if tag=='E':
   code,actor,arg=v
   if code==8:continue
   if code==9:out.append(bytes([7,arg]));continue
   saved=v4.ident,v4.opt;v4.ident,v4.opt=ident,opt
   try:out+=v4.event(v,None)
   finally:v4.ident,v4.opt=saved

  elif tag=='Y':
   actor,reason=v;out.append(bytes([20,reason])+ident(actor))
  elif tag=='H':
   code,actor,*a=v;payload=bytes([17,code])+ident(actor)
   if code in [0,1,2,3]:payload+=bytes(a)
   else:raise ValueError(r)
   out.append(payload)
  elif tag=='M':
   attempt,actor,x,y,ok,f,ceiling,drop,floatok,reason,leaf,sector,line,blockactor=v
   candidates=[r[4:] for r in rows if r[0]=='C' and r[3]==attempt];things=[a for k,a in candidates if k==0];lines=[a for k,a in candidates if k==1];opens=[r[4:] for r in rows if r[0]=='O' and r[3]==attempt]
   payload=ident(actor)+struct.pack('<ii',x,y)+opt(blockactor)+n('I',len(things))+b''.join(ident(a) for a in things)+n('H',65535 if line<0 else line)+n('B',reason!=3)+n('I',len(lines))+b''.join(n('H',a) for a in lines)
   payload+=struct.pack('<HHiiiBBI',leaf,sector,f,ceiling,drop,floatok,reason,len(opens))+b''.join(struct.pack('<Hiiii',*o) for o in opens)
   out.append(bytes([10])+payload)
  elif tag=='Z':out.append(bytes([11])+ident(v[0])+struct.pack('<6i3B',*v[1:]))
  elif tag=='N':out.append(bytes([14,6])+n('H',v[0])+n('B',v[1])+ident(v[2]))
  elif tag=='K':
   code,actor,*a=v;b=bytes([16,code])+ident(actor)
   if code==0:b+=n('i',a[0])
   elif code==1:b+=opt(a[0])+bytes(a[1:])
   elif code in [2,3]:b+=bytes(a)
   elif code==4:b+=n('B',a[0])+n('i',a[1])
   elif code==5:b+=n('B',a[0])+n('I',a[1])+n('B',a[2])+(n('i',a[3]) if a[2] else b'')+n('B',a[4])
   elif code==7:b+=bytes(a)
   elif code==8:b+=n('H',a[0])
   elif code==9:b+=n('B',a[0])
   else:raise ValueError(r)
   out.append(b)
  elif tag=='V':
   assert v[1]==0,r
   end=next(j for j in range(i,len(rows)) if rows[j][0]=='Q' and rows[j][3]==v[0]);out.append(sight(r,rows[i:end],rows[end]));i=end+1
  elif tag=='Q':raise ValueError(('unmatched query',r))
 return out

def object_bytes(kind,index):
 return b'\0'+n('H',index) if kind else b'\1'+ident(index)
def trace_bytes(rows,stage,hitkind,hitid,position,count):
 origin=next(r[4:] for r in rows if r[0]=='D' and r[3]==stage)
 cells=[r[4:] for r in rows if r[0]=='B' and r[3]==stage]
 candidates=[r[4:] for r in rows if r[0]=='U' and r[3]==stage]
 intercepts=sorted([r[4:] for r in rows if r[0]=='I' and r[3]==stage],key=lambda r:r[2])
 b=b'\1'+struct.pack('<4i',*origin)+n('I',len(cells))+b''.join(struct.pack('<ii',*cell) for cell in cells)
 b+=n('I',len(candidates))+b''.join(object_bytes(*o) for o in candidates)
 b+=n('I',len(intercepts))+b''.join(n('i',frac)+object_bytes(kind,index) for kind,index,frac in intercepts)
 b+=n('B',hitkind>=0)+(object_bytes(hitkind,hitid) if hitkind>=0 else b'')
 b+=n('B',position is not None)+(struct.pack('<iii',*position) if position is not None else b'')+n('I',count)
 return b

def events(rows):
 out=[];i=0
 while i<len(rows):
  r=rows[i];tag=r[0];v=r[3:];i+=1
  if tag in ['D','B','U','I','T']:continue
  if tag=='W':
   code,actor,*a=v;b=bytes([19,code])+ident(actor)
   if code==1:b+=n('B',a[0])
   elif code==2:b+=n('B',a[0])+n('I',a[1])+struct.pack('<ii',*a[2:])
   elif code==3:b+=bytes(a[:2])+opt(a[2])
   elif code==4:b+=n('B',a[0])
   elif code!=0:raise ValueError(r)
   out.append(b)
  elif tag=='J':
   code,actor,*a=v;b=bytes([18,code])+ident(actor)
   if code==0:
    kind,ok,reason,hd,d,ht,t=a;b+=bytes([kind,ok,reason,hd])+(n('i',d) if hd else b'')+bytes([ht])+(n('i',t) if ht else b'')
   elif code==1:b+=bytes(a)
   elif code==2:b+=opt(a[0])+struct.pack('<4I',*a[1:])
   elif code==3:b+=n('B',a[0])+n('H',a[1])
   elif code==4:b+=n('I',a[0])
   elif code==5:
    target,slope,bottom,top,count,stopkind,stopid=a
    b+=trace_bytes(rows,0,stopkind,stopid,None,count)+opt(target)+struct.pack('<iii',slope,bottom,top)
   else:raise ValueError(r)
   out.append(b)
  elif tag=='X':
   if v[0]==1:continue
   _,shooter,angle,shot_range,slope,damage=v
   out.append(bytes([13,0])+ident(shooter)+n('I',angle)+struct.pack('<iii',shot_range,slope,damage))
   end_index=next(j for j in range(i,len(rows)) if rows[j][0]=='X' and rows[j][3]==1)
   end=rows[end_index];kind,index,x,y,z,count=end[4:]
   stage=next(r[3] for r in rows[i:end_index+1] if r[0]=='D')
   out.append(bytes([13])+trace_bytes(rows[i:end_index+1],stage,kind,index,[x,y,z] if kind>=0 else None,count))
  elif tag=='G':
   code,*a=v;b=bytes([13,code])
   if code==2:b+=ident(a[0])+opt(a[1])+opt(a[2])+n('i',a[3])
   elif code==3:b+=bytes(a)
   elif code==4:b+=ident(a[0])+n('I',a[1])+struct.pack('<iii',*a[2:])
   elif code==5:b+=ident(a[0])+struct.pack('<ii',*a[1:])
   elif code==6:b+=ident(a[0])+n('H',a[1])
   elif code==7:b+=ident(a[0])+n('H',a[1])+n('B',a[2])
   elif code==8:b+=ident(a[0])+n('i',a[1])
   elif code in [9,11,12]:b+=ident(a[0])
   elif code==10:b+=ident(a[0])+ident(a[1])
   else:raise ValueError(r)
   out.append(b)
  elif tag=='V':
   end=next(j for j in range(i,len(rows)) if rows[j][0]=='Q' and rows[j][3]==v[0]);out+=shared_events(rows[i-1:end+1]);i=end+1
  elif tag=='ST':out.append(bytes([21,v[0]])+ident(v[1])+n('i',v[2]))
  elif tag in ['A','R','S','L','C','O','F','LS','SEED']:continue
  elif tag=='M':out+=shared_events([x for x in rows if x[0] in ['C','O']]+[r])
  elif tag in ['E','H','Z','N','K','Y']:out+=shared_events([r])
  else:raise ValueError(r)
 return out

def snapshot(c,rows,operation,grouped):
 global BIRTH_ID
 spawnpoints={r[9]:r[4:9] for r in grouped.get(0,[]) if r[0]=='F' and r[3]>=0 and r[10]>r[9]}
 BIRTH_ID={};order=[];spatial=[];pending=set();generations=[];last=[];queued=[];occupied={}
 for op in range(operation+1):
  current=grouped.get(op,[])
  for r in current:
   if r[0]=='L':spatial=[i for i in spatial if i!=r[3]];spatial.insert(0,r[3])
   if r[0]=='E':
    code,i,arg=r[3:]
    if code==1:
     slot=next((s for s in range(len(generations)) if s not in occupied),len(generations))
     if slot==len(generations):generations.append(1)
     occupied[slot]=i;BIRTH_ID[i]=(slot,generations[slot]);order.append(i)
    elif code==2:pending.add(i);spatial=[j for j in spatial if j!=i]
    elif code==3:
     order.remove(i);pending.remove(i);slot=BIRTH_ID[i][0];del occupied[slot];generations[slot]+=1
  if c['mode'] in [0,6] and op>0:continue # Spawn-only observation has no persistent event mutation.
  if c['mode']==4 and op>0:
   split=next(i for i,r in enumerate(current) if r[0]=='E' and r[3] in [0,3])
   queued+=events(current[:split]);last=queued+[b'\0\0',b'\0\1']+events(current[split:])+[b'\0\2',b'\0\3'];queued=[]
  else:queued+=events(current)
 actors={r[3]:r[4:] for r in current if r[0]=='A'};sounds={r[3]:r[4] for r in current if r[0]=='S'};roster={r[3]:r[4:] for r in current if r[0]=='R'};tic=operation if c['mode']==4 else 0
 fields=[('tic',n('Q',tic)),('tic_rate',n('I',35)),('profile',n('I',17)),('paused',b'\0'),('rng.gameplay',n('B',actors[0][21])),('rng.miscellaneous',b'\0'),('actors.options',bytes([c['skill'],0,c['no_monsters']])),*[(name,n('i',value)) for name,value in zip(['level.total_kills','level.total_items','level.total_secrets','level.single_player_kill_count'],next(r[3:] for r in current if r[0]=='LS'))],('store.max_slots',n('I',64)),('store.slot_count',n('Q',len(generations))),('store.head',opt(order[0] if order else -1)),('store.tail',opt(order[-1] if order else -1)),('thinker.order',b''.join(ident(i) for i in order))]
 for i,generation in enumerate(generations):
  p=f'actor.{i}';fields +=[(p+'.generation',n('Q',generation)),(p+'.retired',b'\0'),(p+'.occupied',n('B',i in occupied))]
  if i not in occupied:continue
  i=occupied[i];a=actors[i]
  alive,ty,x,y,z,mx,my,mz,health,state,tics,flags,height,radius,reaction,threshold,target,lastlook,floor,ceiling,leaf,rng,angle,sprite,frame,movedir,movecount=a;pos=order.index(i)
  fields +=[(p+'.pending_removal',n('B',i in pending)),(p+'.previous',opt(order[pos-1] if pos else -1)),(p+'.next',opt(order[pos+1] if pos+1<len(order) else -1))]
  for key,value in [('x',x),('y',y),('z',z),('momx',mx),('momy',my),('radius',radius),('height',height),('floor',floor),('ceiling',ceiling)]:fields.append((p+'.'+key,n('i',value)))
  fields +=[(p+'.angle',n('I',angle)),(p+'.lastlook',n('B',lastlook)),(p+'.flags',n('I',flags)),(p+'.type',n('H',ty)),(p+'.state',n('H',65535 if state<0 else state)),(p+'.tics',n('i',tics)),(p+'.health',n('i',health)),(p+'.reaction_time',n('i',reaction)),(p+'.sprite',n('H',sprite)),(p+'.frame',n('I',frame)),(p+'.target',opt(target)),(p+'.death_requested',b'\0'),(p+'.spawnpoint',(b'\1'+struct.pack('<hhhhH',*spawnpoints[i])) if i in spawnpoints else b'\0'),(p+'.threshold',n('i',threshold)),(p+'.movedir',n('B',movedir)),(p+'.movecount',n('i',movecount)),(p+'.momz',n('i',mz)),(p+'.subsector',n('H',leaf)),(p+'.sector',n('H',leaf))]
 maps,sectors,omissions=map_fields(c['map']);fields +=[('spatial.link_order',b''.join(ident(i) for i in spatial)),('world.blockmap_policy',b'\1'),('world.conservative_omissions',n('I',omissions)),('sectors.count',n('I',len(sectors)//26))]
 for i in range(len(sectors)//26):
  floor,ceiling=struct.unpack_from('<hh',sectors,i*26);fields +=[(f'sector.{i}.floor',n('i',floor*U)),(f'sector.{i}.ceiling',n('i',ceiling*U)),(f'sector.{i}.actors',b''.join(ident(a) for a in spatial if actors[a][20]==i and not actors[a][11]&8)),(f'sector.{i}.soundtarget',opt(sounds[i]))]
 for i in range(4):fields +=[(f'player.{i}.actor',bytes(13)),(f'player.{i}.forward',b'\0'),(f'player.{i}.side',b'\0'),(f'player.{i}.angle_turn',bytes(2)),(f'player.{i}.buttons',b'\0')]
 for i in range(4):
  active,health,actor=roster[i];fields +=[(f'perception.player.{i}.active',n('B',active)),(f'perception.player.{i}.health',n('i',health)),(f'perception.player.{i}.actor',opt(actor))]
 for prefix,es in [('events.last',last),('events.pending',queued)]:fields +=[(prefix+'.count',n('Q',len(es)))]+[(f'{prefix}.{i}',e) for i,e in enumerate(es)]
 fields+=maps+metadata(c);data=b'DSIM'+struct.pack('<IQ',17,len(fields))
 for path,value in fields:
  path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
 return data

def empty_policy_snapshot(which,limit):
 # Independent schema/public-input expectation for Rust-only atomic capacity
 # rejection. These are not fabricated C observations or gameplay captures.
 c={'skill':2,'no_monsters':0}
 fields=[('tic',n('Q',0)),('tic_rate',n('I',35)),('profile',n('I',17)),('paused',b'\0'),('rng.gameplay',b'\0'),('rng.miscellaneous',b'\0'),('actors.options',bytes([2,0,0]))]
 fields +=[(key,n('i',0)) for key in ['level.total_kills','level.total_items','level.total_secrets','level.single_player_kill_count']]
 fields +=[('store.max_slots',n('I',limit)),('store.slot_count',n('Q',0)),('store.head',bytes(13)),('store.tail',bytes(13)),('thinker.order',b''),('spatial.link_order',b''),('world.blockmap_policy',b'\1')]
 maps,sectors,omissions=map_fields(which)
 fields +=[('world.conservative_omissions',n('I',omissions)),('sectors.count',n('I',len(sectors)//26))]
 for i in range(len(sectors)//26):
  floor,ceiling=struct.unpack_from('<hh',sectors,i*26)
  fields +=[(f'sector.{i}.floor',n('i',floor*U)),(f'sector.{i}.ceiling',n('i',ceiling*U)),(f'sector.{i}.actors',b''),(f'sector.{i}.soundtarget',bytes(13))]
 for i in range(4):fields +=[(f'player.{i}.actor',bytes(13)),(f'player.{i}.forward',b'\0'),(f'player.{i}.side',b'\0'),(f'player.{i}.angle_turn',bytes(2)),(f'player.{i}.buttons',b'\0')]
 for i in range(4):fields +=[(f'perception.player.{i}.active',b'\0'),(f'perception.player.{i}.health',n('i',0)),(f'perception.player.{i}.actor',bytes(13))]
 fields +=[('events.last.count',n('Q',0)),('events.pending.count',n('Q',0))]+maps+metadata(c)
 data=b'DSIM'+struct.pack('<IQ',17,len(fields))
 for path,value in fields:
  path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
 return data

FULL_LABELS={'hitscan_credit','empty_level','maximum_kills','terminal_no_respawn','restart_before','restart_after','reset_independent','repeated_dead_damage','corpse_removal_total','one_idle','one_wake','multiple_4','shared_target_death','spawn_lifecycle','multiple_lifecycle','corpse_source_reuse','nonzero_floor'}

def run(check):
 cases=list(csv.DictReader((ROOT/'tests/reference/m3d3j_cases.csv').open()));allrows=[[r[0],*map(int,r[1:])] for r in csv.reader((ROOT/'tests/reference/m3d3j_trace.csv').open())];outputs={};index=['case,operation,tic,fnv1a64,bytes_fixture']
 for which,values in enumerate(cases):
  c={k:int(v) for k,v in values.items() if k!='label'};rows=[r for r in allrows if r[1]==which]
  grouped={}
  for row in rows:grouped.setdefault(row[2],[]).append(row)
  rng_at={r[2]:r[25] for r in rows if r[0]=='A' and r[3]==0}
  for op in range(c['tics']+1):
   data=snapshot(c,rows,op,grouped);digest=0xcbf29ce484222325
   for byte in data:digest=((digest^byte)*0x100000001b3)&0xffffffffffffffff
   name=f'm3d3j_state_{which}_{op}.bin';stored=op==0 or values['label'] in FULL_LABELS and (op<=35 or op==c['tics']) or values['label'] in {'spawn_lifecycle','shared_target_death','multiple_4','corpse_removal_total'} or op>0 and rng_at[op]<rng_at[op-1]
   if stored:outputs['tests/reference/'+name]=data
   index.append(f'{which},{op},{op if c["mode"]==4 else 0},{digest:016x},{name if stored else "-"}')
 for which,limit in [(0,0),(1,1)]:
  outputs[f'tests/reference/m3d3j_atomic_capacity_{which}.bin']=empty_policy_snapshot(which,limit)
 outputs['tests/reference/m3d3j_snapshots.csv']=('\n'.join(index)+'\n').encode();inputs=['tools/reference/m3d3j_snapshot_goldens.py','tools/reference/m3c_snapshot_goldens.py','tests/reference/m3d3j_cases.csv','tests/reference/m3d3j_trace.csv','tests/reference/m3c_states.csv','tests/reference/m3c_types.csv']+[f'tests/reference/m3d3j_map_{i}.bin' for i in range(54)]
 provenance={'schema':'DSIM-v17','captures':len(index)-1,'rust_policy_byte_fixtures':2,'inputs_sha256':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'scope':'Original THING order, skill/single-player/nomonsters filtering, source type resolution followed by declared native136 adaptation, raw state initialization without Look, two spawn RNG draws, angles/floors/BSP/insertion, multi-owner sequential shared-target melee and spawn-to-lifecycle. Separate unadapted type12 spawn pass is component evidence, not complete actor/scoring admission. Original two-state stand10/10 and eight-state run2tics, repeated-frame Chase entries, immediate wake/target loss/reacquisition, sight/noise/ambush, complete admitted melee/pain/death/corpse and all21 map run scenarios. All original-C captures. Original Demon pain180,2/2,ordinary death8/8/4/4/4/-1,delayed sounds,Fall/flags/quarter height/terminal corpse/collision,shared damage/hitscan/knockback/source retarget and current melee/physics; real LUT and generation-safe scripted retirement/reuse. Separate injected pain-boundary components are not gameplay captures. Explicit schema and labelled fixture metadata; no Rust output.'}
 outputs['tests/reference/m3d3j_snapshot_provenance.json']=(json.dumps(provenance,indent=2)+'\n').encode()
 for p,b in outputs.items():
  if check:assert (ROOT/p).read_bytes()==b,p
  else:(ROOT/p).write_bytes(b)
 print(('Checked' if check else 'Generated')+f' {len(index)-1} independent DSIM-v17 captures, {len(outputs)-2} full byte streams (including 2 independently specified Rust-only atomicity states).')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
