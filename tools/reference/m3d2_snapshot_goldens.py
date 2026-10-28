#!/usr/bin/env python3
"""Independent v6 schema encoding from C/setup only; historical encoders unchanged."""
import argparse,csv,hashlib,json,struct
from pathlib import Path
import m3d1_snapshot_goldens as v5
ROOT=v5.ROOT
n,ident,opt=v5.n,v5.ident,v5.opt
FINAL=[0,5,9,11,12,13,14,15,16,17,18,20,21,28,29,30,33,34,35,36,37]
def map_fields(which):
 path=ROOT/f'tests/reference/m3d{"1" if which<9 else "2"}_map_{which}.bin';data=path.read_bytes();count,offset=struct.unpack_from('<ii',data,4);lumps=[]
 for i in range(count):
  start,size,_=struct.unpack_from('<ii8s',data,offset+i*16);lumps.append(data[start:start+size])
 names=['things','linedefs','sidedefs','vertices','segs','subsectors','nodes','sectors','reject'];fields=[('map.'+name,lumps[i+1]) for i,name in enumerate(names)];words=struct.unpack('<'+'H'*(len(lumps[10])//2),lumps[10]);ranges=b''
 for start in words[4:4+words[2]*words[3]]:
  end=start
  while words[end]!=65535:end+=1
  assert set(words[start:end])==set(range(len(lumps[2])//14));ranges+=struct.pack('<II',start,end)
 return fields+[('map.blockmap.words',lumps[10]),('map.blockmap.cell_ranges',ranges)],lumps[8]
def decode(data):
 version,count=struct.unpack_from('<IQ',data,4);offset=16;fields=[]
 for _ in range(count):
  size=struct.unpack_from('<I',data,offset)[0];offset+=4;path=data[offset:offset+size].decode();offset+=size;size=struct.unpack_from('<Q',data,offset)[0];offset+=8;fields.append((path,data[offset:offset+size]));offset+=size
 assert offset==len(data);return fields
def weapon_fields(p):
 # P fields: slot,actor,health,ready,pending,owned,ammo4,max4,attack,refire,bob,light,psprite4*2,rng
 result=[('weapons.player.present',b'\1')]
 for key,code,value in [('slot','B',p[0]),('actor',None,p[1]),('health','i',p[2]),('ready','B',p[3]),('pending','B',p[4]),('owned','H',p[5])]:result.append(('weapons.player.'+key,ident(value) if code is None else n(code,value)))
 result += [('weapons.player.ammo',struct.pack('<4i',*p[6:10])),('weapons.player.max_ammo',struct.pack('<4i',*p[10:14]))]
 for key,code,value in [('attack_down','B',p[14]),('refire','i',p[15]),('bob','i',p[16]),('extra_light','i',p[17])]:result.append(('weapons.player.'+key,n(code,value)))
 for i in range(2):
  s=p[18+4*i:22+4*i]
  for key,code,value in [('state','H',65535 if s[0]<0 else s[0]),('tics','i',s[1]),('x','i',s[2]),('y','i',s[3])]:result.append((f'weapons.player.psprite.{i}.'+key,n(code,value)))
 return result
def event(r,states):
 if r[0]=='E':
  assert r[3]!=8,'selected states exclude RNG effects'
  return v5.v4.event(r[3:],states)
 code,*v=r[3:];b=bytes([14,code])
 if code in [0,1]:b+=bytes(v[:2])+n('H',v[2])
 elif code==2:b+=bytes(v[:2])+struct.pack('<ii',*v[2:])
 elif code==3:b+=n('B',v[0])+n('i',v[1])
 elif code==4:b+=bytes(v[:2])+n('i',v[2])
 elif code==6:b+=n('H',v[0])+n('B',v[1])+ident(v[2])
 elif code==7:b+=n('B',v[0])+ident(v[1])
 else:raise ValueError(('transient firing not selected',r))
 return [b]
def snapshot(c,rows,tic):
 current=[r for r in rows if r[2]==tic];p=next(r[3:] for r in current if r[0]=='P');states,_,_=v5.metadata(c)
 # Reuse v5's field assembler with only its supported settled observation subset;
 # reconstruct the complete v6 pending/last event stream independently below.
 filtered=[r for r in rows if not(r[2]==tic and r[0]=='E' and r[3]!=0)] if tic else rows
 base=decode(v5.snapshot(c,filtered,tic if tic>1 else 0))
 # tic 1 requires its current actors/store; use the same settled branch with a
 # temporary operation label 2, then restore the actual logical tic.
 if tic==1:
  adapted=[r[:2]+[2]+r[3:] if r[2]==1 else r for r in filtered]
  base=decode(v5.snapshot(c,adapted,2))
 events=[]
 if tic:
  if tic==1:
   for r in rows:
    if r[2]==0 and r[0] in ['E','W'] and not(r[0]=='E' and r[3]==8):events+=event(r,states)
  events += [b'\0\0',b'\4'+n('B',p[0])+ident(p[1])];thinkers=False
  for r in current:
   if r[0] not in ['E','W']:continue
   if r[0]=='E' and r[3]==0 and not thinkers:events.append(b'\0\1');thinkers=True
   events+=event(r,states)
  assert thinkers;events += [b'\0\2',b'\0\3']
 else:
  for r in current:
   if r[0] in ['E','W'] and not(r[0]=='E' and r[3]==8):events+=event(r,states)
 angles={r[3]:r[4] for r in current if r[0]=='G'};sounds={r[3]:r[4] for r in current if r[0]=='S'}
 fields=[];inserted=False
 for path,value in base:
  if path.startswith('events.'):
   if not inserted:
    fields+=weapon_fields(p)
    for prefix,es in [('events.last',events if tic else []),('events.pending',[] if tic else events)]:fields += [(prefix+'.count',n('Q',len(es)))]+[(f'{prefix}.{i}',e) for i,e in enumerate(es)]
    inserted=True
   continue
  if path=='tic':value=n('Q',tic)
  if path=='profile':value=n('I',6)
  if path=='store.max_slots':value=n('I',64)
  if path.startswith('actor.') and path.endswith('.angle'):value=n('I',angles[int(path.split('.')[1])])
  if path.startswith('player.'):
   slot=int(path.split('.')[1]);key=path.split('.')[2]
   if key=='actor':value=opt(0) if slot==p[0] else bytes(13)
   # Selected captures have no movement/change commands except t21 attack-start.
   if tic:
    cmd=COMMANDS[(c['case'],tic)];active=slot==p[0]
    if key=='forward':value=n('b',cmd[0] if active else 0)
    if key=='side':value=n('b',cmd[1] if active else 0)
    if key=='angle_turn':value=n('h',cmd[2] if active else 0)
    if key=='buttons':value=n('B',cmd[3] if active else 0)
  fields.append((path,value))
  if path.startswith('sector.') and path.endswith('.actors'):fields.append((path.rsplit('.',1)[0]+'.soundtarget',opt(sounds[int(path.split('.')[1])])))
 data=b'DSIM'+struct.pack('<IQ',6,len(fields))
 for path,value in fields:
  path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
 return data
COMMANDS={}
def run(check):
 v5.map_fields=map_fields
 global COMMANDS
 COMMANDS={(int(r['case']),int(r['tic'])):[int(r[k]) for k in ['forward','side','turn','buttons']] for r in csv.DictReader((ROOT/'tests/reference/m3d2_commands.csv').open())}
 cases=list(csv.DictReader((ROOT/'tests/reference/m3d2_cases.csv').open()));groups={}
 for r in csv.reader((ROOT/'tests/reference/m3d2_trace.csv').open()):
  row=[r[0],*map(int,r[1:])];groups.setdefault(row[1],[]).append(row)
 outputs={};index=['case,tic,fnv1a64,bytes_fixture']
 for which,c in enumerate(cases):
  c={k:int(v) for k,v in c.items() if k not in ['label','script']};c['flags']=6
  selected=[0]+([1,10,15,21,22] if which in [0,5] else [])+([c['tics']] if which in FINAL else [])
  for tic in selected:
   data=snapshot(c,groups[which],tic);digest=0xcbf29ce484222325
   for byte in data:digest=((digest^byte)*0x100000001b3)&0xffffffffffffffff
   stored=which in [0,5,9,15,16,17,35,36,37] and (tic in [0,c['tics'],1,10,21])
   name=f'm3d2_state_{which}_{tic}.bin' if stored else '-'
   if stored:outputs['tests/reference/'+name]=data
   index.append(f'{which},{tic},{digest:016x},{name}')
 outputs['tests/reference/m3d2_snapshots.csv']=('\n'.join(index)+'\n').encode()
 inputs=['tools/reference/m3d2_snapshot_goldens.py','tools/reference/m3d1_snapshot_goldens.py','tools/reference/m3c_snapshot_goldens.py','tests/reference/m3d2_cases.csv','tests/reference/m3d2_commands.csv','tests/reference/m3d2_trace.csv','tests/reference/m3c_states.csv','tests/reference/m3c_types.csv']+[f'tests/reference/m3d1_map_{i}.bin' for i in range(9)]+[f'tests/reference/m3d2_map_{i}.bin' for i in range(9,12)]
 manifest={'schema':'DSIM-v6','captures':len(index)-1,'inputs_sha256':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'assumptions':['no Rust output read; reuse v5 framing/field assembler and source metadata, replace v6 fields/events explicitly','selected captures have stationary actors and no shot effects on that tic; source weapon events and actor transitions included','selected single-burst cases have no retired-slot reuse; C link/removal history reconstructs order/generations','source wp_nochange=10, not NUMWEAPONS=9; source initial Raise executes immediately']}
 outputs['tests/reference/m3d2_snapshot_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
 for p,data in outputs.items():
  if check:assert (ROOT/p).read_bytes()==data,p
  else:(ROOT/p).write_bytes(data)
 print(('Checked' if check else 'Generated')+f' {len(index)-1} independent DSIM-v6 captures, {len(outputs)-2} byte fixtures.')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
