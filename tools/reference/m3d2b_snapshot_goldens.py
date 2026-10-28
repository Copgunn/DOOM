#!/usr/bin/env python3
"""Independent DSIM-v7 bytes: C captures, public inputs and explicit schema only.
No Rust output. Setup for all cases; complete no-damage wall/dropoff lifecycles;
settled lethal result. Damage timelines are independently compared in Rust tests.
"""
import argparse,csv,hashlib,json,struct
from pathlib import Path
import m3c_snapshot_goldens as v4
ROOT=Path(__file__).resolve().parents[2]
U=65536
n=v4.n

def map_fields(which):
 data=(ROOT/f'tests/reference/m3d2b_map_{which}.bin').read_bytes();count,offset=struct.unpack_from('<ii',data,4);lumps=[]
 for i in range(count):
  start,size,_=struct.unpack_from('<ii8s',data,offset+i*16);lumps.append(data[start:start+size])
 names=['things','linedefs','sidedefs','vertices','segs','subsectors','nodes','sectors','reject']
 fields=[('map.'+name,lumps[i+1]) for i,name in enumerate(names)]
 words=struct.unpack('<'+'H'*(len(lumps[10])//2),lumps[10]);ranges=b'';cells=[]
 for start in words[4:4+words[2]*words[3]]:
  end=start
  while words[end]!=65535:end+=1
  ranges+=struct.pack('<II',start,end);cells.append(set(words[start:end]))
 # Independent conservative bounding-box coverage count; supplied lists stay intact.
 vertices=list(struct.iter_unpack('<hh',lumps[4]));ox,oy=struct.unpack_from('<hh',lumps[10]);w,h=words[2:4];omissions=0
 for i,line in enumerate(struct.iter_unpack('<7H',lumps[2])):
  points=[vertices[v] for v in line[:2]];xl=max(0,(min(p[0] for p in points)-ox)//128);xh=min(w-1,(max(p[0] for p in points)-ox)//128);yl=max(0,(min(p[1] for p in points)-oy)//128);yh=min(h-1,(max(p[1] for p in points)-oy)//128)
  omissions+=sum(i not in cells[y*w+x] for x in range(xl,xh+1) for y in range(yl,yh+1))
 return fields+[('map.blockmap.words',lumps[10]),('map.blockmap.cell_ranges',ranges)],lumps[8],omissions

def metadata(c):
 states,types,fields=v4.metadata(0)
 for i in [15,17,136]:
  p=types[0][1].copy()
  for j,key in [(2,'health'),(8,'pain'),(18,'mass')]:p[j]=c[key]
  p[0]=-1;p[5]=8;p[21]=c['flags'] if i==c['target'] else 6
  name=f'FIXTURE_PAWN_{i}'.encode();fields[next(j for j,f in enumerate(fields) if f[0]==f'registry.type.{i}')]=(f'registry.type.{i}',n('B',len(name))+name+struct.pack('<23i',*p))
 return fields

def snapshot(c,rows,operation):
 order=[];spatial=[];pending=set();generations=[];slots=[];handles={};last=[];queued=[]
 def ident(i):return struct.pack('<IQ',*handles[i])
 def opt(i):return bytes(13) if i<0 else b'\1'+ident(i)
 def events(current):
  out=[]
  for r in current:
   tag=r[0];v=r[3:]
   if tag=='E':
    code,i,arg=v
    if code==0:out.append(b'\5'+ident(i))
    elif code==1:out.extend([b'\1'+ident(i),bytes([12,0])+ident(i)+n('H',arg)])
    elif code in [2,3]:out.append(n('B',code)+ident(i))
    elif code==4:out.append(bytes([12,1])+ident(i)+n('H',arg))
    elif code==5:out.append(bytes([12,2])+ident(i)+v4.action(arg))
    elif code==6:out.append(bytes([12,4])+ident(i)+opt(arg))
    elif code==7:out.append(bytes([12,3])+opt(i)+n('H',arg))
    elif code!=8:raise ValueError(v)
   elif tag=='P':
    code=v[0];a=v[1:];payload=n('B',code)
    if code==0:payload+=ident(a[0])
    elif code==1:payload+=ident(a[0])+struct.pack('<Iiii',*a[1:])
    elif code==2:payload+=bytes(a)
    elif code==4:payload+=ident(a[0])+struct.pack('<4i',*a[1:])
    elif code==5:payload+=ident(a[0])+n('B',a[1])+(ident(a[2]) if a[1]==0 else b'')+n('H',a[3])
    else:raise ValueError(v)
    out.append(bytes([15])+payload)
   elif tag=='M':
    attempt,i,x,y,ok,f,ceiling,drop,floatok,reason,leaf,sector,line,actor=v
    candidates=[r[4:] for r in current if r[0]=='C' and r[3]==attempt];things=[a for k,a in candidates if k==0];lines=[a for k,a in candidates if k==1];opens=[r[4:] for r in current if r[0]=='O' and r[3]==attempt]
    payload=ident(i)+struct.pack('<ii',x,y)+opt(actor)+n('I',len(things))+b''.join(ident(a) for a in things)+n('H',65535 if line<0 else line)+n('B',reason!=3)+n('I',len(lines))+b''.join(n('H',a) for a in lines)
    payload+=struct.pack('<HHiiiBBI',leaf,sector,f,ceiling,drop,floatok,reason,len(opens))+b''.join(struct.pack('<Hiiii',*o) for o in opens)
    out.append(bytes([10])+payload)
   elif tag=='Z':out.append(bytes([11])+ident(v[0])+struct.pack('<6i3B',*v[1:]))
  return out
 for op in range(operation+1):
  current=[r for r in rows if r[2]==op]
  for r in current:
   if r[0]=='L':spatial=[a for a in spatial if a!=r[3]];spatial.insert(0,r[3])
   if r[0]=='E':
    code,i,arg=r[3:]
    if code==1:
     slot=next((j for j,v in enumerate(slots) if v is None),len(slots))
     if slot==len(slots):slots.append(None);generations.append(1)
     slots[slot]=i;handles[i]=(slot,generations[slot]);order.append(i)
    elif code==2:pending.add(i);spatial=[a for a in spatial if a!=i]
    elif code==3:order.remove(i);pending.remove(i);slot=handles[i][0];slots[slot]=None;generations[slot]+=1
  # Only selected captures require event projection. Intermediate damage frames
  # for the settled lethal result are not used as canonical expected events.
  if c['case']==24 and op not in [0,operation]:
   if op==2:queued=[]
   continue
  es=events(current)
  if op<2 or c['mode']==3 and op==c['tics']+2:queued+=es
  else:last=queued+[b'\0\0',b'\0\1']+es+[b'\0\2',b'\0\3'];queued=[]
 actors={r[3]:r[4:] for r in rows if r[0]=='A' and r[2]==operation};rng=actors[0][21]
 fields=[('tic',n('Q',max(operation-1,0)-(1 if c['mode']==3 and operation>=c['tics']+2 else 0))),('tic_rate',n('I',35)),('profile',n('I',7)),('paused',b'\0'),('rng.gameplay',n('B',rng)),('rng.miscellaneous',b'\0'),('actors.options',bytes([2,0,0])),('store.max_slots',n('I',64)),('store.slot_count',n('Q',len(slots))),('store.head',opt(order[0] if order else -1)),('store.tail',opt(order[-1] if order else -1)),('thinker.order',b''.join(ident(i) for i in order))]
 for slot,generation in enumerate(generations):
  p=f'actor.{slot}';i=slots[slot];fields +=[(p+'.generation',n('Q',generation)),(p+'.retired',b'\0'),(p+'.occupied',n('B',i is not None))]
  if i is None:continue
  alive,ty,x,y,z,mx,my,mz,health,state,tics,flags,height,radius,reaction,threshold,target,lastlook,floor,ceiling,leaf,rng,angle,sprite,frame=actors[i];assert alive
  pos=order.index(i);fields +=[(p+'.pending_removal',n('B',i in pending)),(p+'.previous',opt(order[pos-1] if pos else -1)),(p+'.next',opt(order[pos+1] if pos+1<len(order) else -1))]
  for key,val in [('x',x),('y',y),('z',z),('momx',mx),('momy',my),('radius',radius),('height',height),('floor',floor),('ceiling',ceiling)]:fields.append((p+'.'+key,n('i',val)))
  fields +=[(p+'.angle',n('I',angle)),(p+'.lastlook',n('B',lastlook)),(p+'.flags',n('I',flags)),(p+'.type',n('H',ty)),(p+'.state',n('H',65535 if state<0 else state)),(p+'.tics',n('i',tics)),(p+'.health',n('i',health)),(p+'.reaction_time',n('i',reaction)),(p+'.sprite',n('H',sprite)),(p+'.frame',n('I',frame)),(p+'.target',opt(target)),(p+'.death_requested',b'\0'),(p+'.spawnpoint',b'\0'),(p+'.threshold',n('i',threshold)),(p+'.momz',n('i',mz)),(p+'.subsector',n('H',leaf)),(p+'.sector',n('H',leaf))]
 maps,sectors,omissions=map_fields(c['map']);fields +=[('spatial.link_order',b''.join(ident(i) for i in spatial)),('world.blockmap_policy',b'\1'),('world.conservative_omissions',n('I',omissions)),('sectors.count',n('I',len(sectors)//26))]
 for i in range(len(sectors)//26):
  floor,ceiling=struct.unpack_from('<hh',sectors,i*26);fields +=[(f'sector.{i}.floor',n('i',floor*U)),(f'sector.{i}.ceiling',n('i',ceiling*U)),(f'sector.{i}.actors',b''.join(ident(a) for a in spatial if actors[a][20]==i and not actors[a][11]&8))]
 for i in range(4):fields +=[(f'player.{i}.actor',bytes(13)),(f'player.{i}.forward',b'\0'),(f'player.{i}.side',b'\0'),(f'player.{i}.angle_turn',bytes(2)),(f'player.{i}.buttons',b'\0')]
 for prefix,es in [('events.last',last),('events.pending',queued)]:fields +=[(prefix+'.count',n('Q',len(es)))]+[(f'{prefix}.{i}',e) for i,e in enumerate(es)]
 fields+=maps+metadata(c);out=b'DSIM'+struct.pack('<IQ',7,len(fields))
 for path,value in fields:
  path=path.encode();out+=n('I',len(path))+path+n('Q',len(value))+value
 return out

def run(check):
 cases=list(csv.DictReader((ROOT/'tests/reference/m3d2b_cases.csv').open()));rows=[[r[0],*map(int,r[1:])] for r in csv.reader((ROOT/'tests/reference/m3d2b_trace.csv').open())];outputs={};index=['case,operation,tic,fnv1a64,bytes_fixture']
 for which,values in enumerate(cases):
  c={k:int(v) for k,v in values.items() if k!='label'};cr=[r for r in rows if r[1]==which]
  selected=[0]+(list(range(1,c['tics']+2)) if which in [2,36] else [1,31,32,33,62] if which==30 else [c['tics']+1] if which==24 else [])
  for op in selected:
   data=snapshot(c,cr,op);digest=0xcbf29ce484222325
   for byte in data:digest=((digest^byte)*0x100000001b3)&0xffffffffffffffff
   name=f'm3d2b_state_{which}_{op}.bin';stored=(which in [0,2,24,30,36] and (op in [0,1,2,3,4,5,8,12,20,30,32,33,62,c['tics']+1]))
   if stored:outputs['tests/reference/'+name]=data
   index.append(f'{which},{op},{max(op-1,0)-(1 if c['mode']==3 and op>=c['tics']+2 else 0)},{digest:016x},{name if stored else "-"}')
 outputs['tests/reference/m3d2b_snapshots.csv']=('\n'.join(index)+'\n').encode();inputs=['tools/reference/m3d2b_snapshot_goldens.py','tools/reference/m3c_snapshot_goldens.py','tests/reference/m3d2b_cases.csv','tests/reference/m3d2b_trace.csv','tests/reference/m3c_states.csv','tests/reference/m3c_types.csv']+[f'tests/reference/m3d2b_map_{i}.bin' for i in range(14)]
 provenance={'schema':'DSIM-v7','captures':len(index)-1,'inputs_sha256':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'scope':'All setups, complete clear-wall/dropoff flight/explosion/removal, settled lethal result. No Rust output; source rows reconstruct spatial links, arena generations and events. Public schema projection is distinct from source equivalence.'}
 outputs['tests/reference/m3d2b_snapshot_provenance.json']=(json.dumps(provenance,indent=2)+'\n').encode()
 for p,b in outputs.items():
  if check:assert (ROOT/p).read_bytes()==b,p
  else:(ROOT/p).write_bytes(b)
 print(('Checked' if check else 'Generated')+f' {len(index)-1} independent DSIM-v7 captures, {len(outputs)-2} full byte fixtures.')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
