#!/usr/bin/env python3
"""Independent DSIM-v9 from original-C rows, public fixture inputs and schema.
No Rust output. All captures, including transient sight/scan/A_Look/noise events.
"""
import argparse,csv,hashlib,json,struct
from pathlib import Path
import m3c_snapshot_goldens as v4
ROOT=Path(__file__).resolve().parents[2]
U=65536
n,ident,opt=v4.n,v4.ident,v4.opt

def map_fields(which):
 data=(ROOT/f'tests/reference/m3d3b_map_{which}.bin').read_bytes();count,off=struct.unpack_from('<ii',data,4);lumps=[]
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
 states,types,fields=v4.metadata(0)
 for i,frame,tics,action,nextstate,name in [(1,0,c['stand'],43,1,'FIXTURE_LOOK'),(2,1,c['see'],15,2,'FIXTURE_CHASE')]:
  raw=name.encode();b=n('B',len(raw))+raw+struct.pack('<HIi',29,frame,tics)+v4.action(action)+struct.pack('<Hii',nextstate,0,0)
  fields[next(j for j,f in enumerate(fields) if f[0]==f'registry.state.{i}')]=(f'registry.state.{i}',b)
 for i in [19,21,136]:
  p=types[0][1].copy()
  for key,value in [(0,-1),(1,1),(3,2),(4,c['sound_id']),(5,8),(17,c['obsheight']),(21,c['flags']),(11,0),(15,c['speed']),(16,c['radius']),(20,c['active_sound'])]:p[key]=value
  name=f'FIXTURE_LOOKER_{i}'.encode();fields[next(j for j,f in enumerate(fields) if f[0]==f'registry.type.{i}')]=(f'registry.type.{i}',n('B',len(name))+name+struct.pack('<23i',*p))
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

def events(rows):
 out=[];i=0
 while i<len(rows):
  r=rows[i];tag=r[0];v=r[3:];i+=1
  if tag=='E':
   code,actor,arg=v
   if code==8:continue
   if code==9:out.append(bytes([7,arg]));continue
   out+=v4.event(v,None)

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

def snapshot(c,rows,operation):
 order=[];spatial=[];pending=set();generations=[];last=[];queued=[]
 for op in range(operation+1):
  current=[r for r in rows if r[2]==op]
  for r in current:
   if r[0]=='L':spatial=[i for i in spatial if i!=r[3]];spatial.insert(0,r[3])
   if r[0]=='E':
    code,i,arg=r[3:]
    if code==1:assert i==len(generations);generations.append(1);order.append(i)
    elif code==2:pending.add(i);spatial=[j for j in spatial if j!=i]
    elif code==3:order.remove(i);pending.remove(i);generations[i]+=1
  if c['mode']==6 and op>0:continue # Spawn-only observation has no persistent event mutation.
  if c['mode']==4 and op>0:
   split=next(i for i,r in enumerate(current) if r[0]=='E' and r[3]==0)
   queued+=events(current[:split]);last=queued+[b'\0\0',b'\0\1']+events(current[split:])+[b'\0\2',b'\0\3'];queued=[]
  else:queued+=events(current)
 actors={r[3]:r[4:] for r in rows if r[0]=='A' and r[2]==operation};sounds={r[3]:r[4] for r in rows if r[0]=='S' and r[2]==operation};roster={r[3]:r[4:] for r in rows if r[0]=='R' and r[2]==operation};tic=operation if c['mode']==4 else 0
 fields=[('tic',n('Q',tic)),('tic_rate',n('I',35)),('profile',n('I',9)),('paused',b'\0'),('rng.gameplay',n('B',actors[0][21])),('rng.miscellaneous',b'\0'),('actors.options',bytes([c['skill'],0,0])),('store.max_slots',n('I',64)),('store.slot_count',n('Q',len(generations))),('store.head',opt(order[0] if order else -1)),('store.tail',opt(order[-1] if order else -1)),('thinker.order',b''.join(ident(i) for i in order))]
 for i,generation in enumerate(generations):
  p=f'actor.{i}';a=actors[i];fields +=[(p+'.generation',n('Q',generation)),(p+'.retired',b'\0'),(p+'.occupied',n('B',a[0]))]
  if not a[0]:continue
  alive,ty,x,y,z,mx,my,mz,health,state,tics,flags,height,radius,reaction,threshold,target,lastlook,floor,ceiling,leaf,rng,angle,sprite,frame,movedir,movecount=a;pos=order.index(i)
  fields +=[(p+'.pending_removal',n('B',i in pending)),(p+'.previous',opt(order[pos-1] if pos else -1)),(p+'.next',opt(order[pos+1] if pos+1<len(order) else -1))]
  for key,value in [('x',x),('y',y),('z',z),('momx',mx),('momy',my),('radius',radius),('height',height),('floor',floor),('ceiling',ceiling)]:fields.append((p+'.'+key,n('i',value)))
  fields +=[(p+'.angle',n('I',angle)),(p+'.lastlook',n('B',lastlook)),(p+'.flags',n('I',flags)),(p+'.type',n('H',ty)),(p+'.state',n('H',65535 if state<0 else state)),(p+'.tics',n('i',tics)),(p+'.health',n('i',health)),(p+'.reaction_time',n('i',reaction)),(p+'.sprite',n('H',sprite)),(p+'.frame',n('I',frame)),(p+'.target',opt(target)),(p+'.death_requested',b'\0'),(p+'.spawnpoint',b'\0'),(p+'.threshold',n('i',threshold)),(p+'.movedir',n('B',movedir)),(p+'.movecount',n('i',movecount)),(p+'.momz',n('i',mz)),(p+'.subsector',n('H',leaf)),(p+'.sector',n('H',leaf))]
 maps,sectors,omissions=map_fields(c['map']);fields +=[('spatial.link_order',b''.join(ident(i) for i in spatial)),('world.blockmap_policy',b'\1'),('world.conservative_omissions',n('I',omissions)),('sectors.count',n('I',len(sectors)//26))]
 for i in range(len(sectors)//26):
  floor,ceiling=struct.unpack_from('<hh',sectors,i*26);fields +=[(f'sector.{i}.floor',n('i',floor*U)),(f'sector.{i}.ceiling',n('i',ceiling*U)),(f'sector.{i}.actors',b''.join(ident(a) for a in spatial if actors[a][20]==i and not actors[a][11]&8)),(f'sector.{i}.soundtarget',opt(sounds[i]))]
 for i in range(4):fields +=[(f'player.{i}.actor',bytes(13)),(f'player.{i}.forward',b'\0'),(f'player.{i}.side',b'\0'),(f'player.{i}.angle_turn',bytes(2)),(f'player.{i}.buttons',b'\0')]
 for i in range(4):
  active,health,actor=roster[i];fields +=[(f'perception.player.{i}.active',n('B',active)),(f'perception.player.{i}.health',n('i',health)),(f'perception.player.{i}.actor',opt(actor))]
 for prefix,es in [('events.last',last),('events.pending',queued)]:fields +=[(prefix+'.count',n('Q',len(es)))]+[(f'{prefix}.{i}',e) for i,e in enumerate(es)]
 fields+=maps+metadata(c);data=b'DSIM'+struct.pack('<IQ',9,len(fields))
 for path,value in fields:
  path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
 return data

FULL={
 'spawn_defaults':{0},'spawn_defaults_nightmare':{0},'move_0':{0,1},'move_1':{1},'move_8':{1},'walk_0':{1},'walk_8':{1},
 'direction_east':{1},'direction_sw':{1},'blocked_direct':{1},'turnaround_last_resort':{1},'fully_trapped':{1},'rng_wrap_decisions':{180},
 'move_step24':{1},'move_step25':{1},'move_drop25':{1},'move_drop_allowed':{4},'move_opening55':{1},'move_actor_blocks':{1},'move_infloat_clear':{1},
 'chase_turn_4':{1},'chase_count_zero':{1},'chase_missing_reacquires':{1},'chase_failed_reacquisition':{1},'chase_dead_shootable_retained':{1},
 'chase_justattacked_nightmare':{1},'chase_active_sound':{128,300},
 'thinker_acquire_and_pursue':{0,1,2,20},'thinker_wall_alternates':{1,8,40},'thinker_target_loss_spawn':{5,6,12},
 'thinker_reacquisition':{5,6,15},'thinker_momentum_before_action':{1,12},'thinker_append_order':{1,16},'thinker_state_countdown':{2,3,18}}
def run(check):
 cases=list(csv.DictReader((ROOT/'tests/reference/m3d3b_cases.csv').open()));allrows=[[r[0],*map(int,r[1:])] for r in csv.reader((ROOT/'tests/reference/m3d3b_trace.csv').open())];outputs={};index=['case,operation,tic,fnv1a64,bytes_fixture']
 for which,values in enumerate(cases):
  c={k:int(v) for k,v in values.items() if k!='label'};rows=[r for r in allrows if r[1]==which]
  for op in range(c['tics']+1):
   data=snapshot(c,rows,op);digest=0xcbf29ce484222325
   for byte in data:digest=((digest^byte)*0x100000001b3)&0xffffffffffffffff
   name=f'm3d3b_state_{which}_{op}.bin';stored=op in FULL.get(values['label'],set())
   if stored:outputs['tests/reference/'+name]=data
   index.append(f'{which},{op},{op if c["mode"]==4 else 0},{digest:016x},{name if stored else "-"}')
 outputs['tests/reference/m3d3b_snapshots.csv']=('\n'.join(index)+'\n').encode();inputs=['tools/reference/m3d3b_snapshot_goldens.py','tools/reference/m3c_snapshot_goldens.py','tests/reference/m3d3b_cases.csv','tests/reference/m3d3b_trace.csv','tests/reference/m3c_states.csv','tests/reference/m3c_types.csv']+[f'tests/reference/m3d3b_map_{i}.bin' for i in range(17)]
 provenance={'schema':'DSIM-v9','captures':len(index)-1,'inputs_sha256':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'scope':'All original-C captures. Full transient pursuit, shared movement, vertical, sight/player scan/A_Look/noise event bytes, roster health and sector sound targets. Explicit schema and labelled fixture metadata; no Rust output.'}
 outputs['tests/reference/m3d3b_snapshot_provenance.json']=(json.dumps(provenance,indent=2)+'\n').encode()
 for p,b in outputs.items():
  if check:assert (ROOT/p).read_bytes()==b,p
  else:(ROOT/p).write_bytes(b)
 print(('Checked' if check else 'Generated')+f' {len(index)-1} independent DSIM-v9 captures, {len(outputs)-2} full byte streams.')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
