#!/usr/bin/env python3
"""Synthetic classic maps and isolated source-derived BSP/opening/XY/Z vectors.

Never reads Rust output. Existing generators, fixtures and provenance stay frozen.
"""
import argparse
import csv
import hashlib
import io
import json
import platform
import struct
import subprocess
import tempfile
from pathlib import Path
import generate as base
import generate_m3a as m3a
ROOT,SOURCE,FLAGS=base.ROOT,base.SOURCE,base.FLAGS
U=65536


def words(v):return struct.pack('<'+'h'*len(v),*v)
def name(s):return s.encode().ljust(8,b'\0')
def wad(lumps):
    data=b'PWAD'+struct.pack('<ii',len(lumps),0);directory=b''
    for n,b in lumps:
        directory+=struct.pack('<ii8s',len(data),len(b),name(n));data+=b
    return data[:8]+struct.pack('<i',len(data))+data[12:]+directory


def fixture(heights,diagonal=False,sparse=False,omit=False,portal_flags=4):
    n=len(heights);xs=[-128,128] if n==1 else [-128,0,128] if n==2 else [-128,0,64,128]
    vertices=[]
    for i,x in enumerate(xs):
        vertices.extend([(x if not(diagonal and i==1) else -32,-64),(x if not(diagonal and i==1) else 96,64)])
    lines=[]
    def add(a,b,front,back=None,flags=1):lines.append(dict(a=a,b=b,front=front,back=back,flags=flags))
    add(0,1,0)
    for i in range(n):add(2*i+1,2*(i+1)+1,i)
    add(2*n+1,2*n,n-1)
    for i in range(n-1,-1,-1):add(2*(i+1),2*i,i)
    for i in range(1,n):add(2*i,2*i+1,i,i-1,portal_flags)
    sides=[];linebytes=b''
    for l in lines:
        indexes=[]
        for sector in [l['front'],l['back']]:
            if sector is None:indexes.append(-1);continue
            indexes.append(len(sides));sides.append(sector)
        l['sides']=indexes
        linebytes+=words([l['a'],l['b'],l['flags'],0,0,*indexes])
    segbytes=b'';subbytes=b'';first=0
    for sector in range(n):
        count=0
        for i,l in enumerate(lines):
            if l['front']==sector:side=0;a,b=l['a'],l['b']
            elif l['back']==sector:side=1;a,b=l['b'],l['a']
            else:continue
            segbytes+=words([a,b,0,i,side,0]);count+=1
        subbytes+=words([count,first]);first+=count
    nodes=[];nodebytes=b''
    for i in range(n-1):
        x=32 if diagonal else xs[i+1]
        dx,dy=(128,128) if diagonal else (0,128)
        children=[0x8000+i+1,0x8000 if i==0 else i-1]
        nodes.append(dict(x=x,y=0,dx=dx,dy=dy,children=children))
        nodebytes+=words([x,0,dx,dy,64,-64,x,128,64,-64,-128,x])+struct.pack('<HH',*children)
    blocks=[-256,-128,4,2]+[0]*8
    for cell in range(8):
        blocks[4+cell]=len(blocks)
        listing=list(range(len(lines)))
        if sparse and cell==5:listing.remove(len(lines)-1)
        if omit:listing.remove(n+1) # east outer wall, never rebuilt by the adapter
        blocks.extend([0,*listing,-1])
    lumps=[('MAP01',b''),('THINGS',words([-64,0,90,1,7])),('LINEDEFS',linebytes),('SIDEDEFS',b''.join(words([0,0])+b'-\0\0\0\0\0\0\0'*2+b'TESTWALL'+words([s]) for s in sides)),('VERTEXES',b''.join(words(list(v)) for v in vertices)),('SEGS',segbytes),('SSECTORS',subbytes),('NODES',nodebytes),('SECTORS',b''.join(words([f,c])+b'TESTFLR0TESTCEIL'+words([160,0,0]) for f,c in heights)),('REJECT',bytes((n*n+7)//8)),('BLOCKMAP',words(blocks))]
    return dict(heights=heights,vertices=vertices,lines=lines,nodes=nodes,blocks=blocks,bytes=wad(lumps))

MAPS=[
 fixture([(0,128),(0,128)]),fixture([(0,128),(8,128)]),
 fixture([(0,128),(24,128)]),fixture([(0,128),(25,128)]),
 fixture([(0,128),(0,55)]),fixture([(0,128),(0,56)]),
 fixture([(0,128),(0,80)]),fixture([(0,128),(-24,128)]),
 fixture([(0,128),(-25,128)]),fixture([(0,128),(-40,128)]),
 fixture([(0,128),(16,128)],diagonal=True),fixture([(0,128),(16,128)],diagonal=True,sparse=True),
 fixture([(0,128),(24,112),(0,128)]),fixture([(0,128),(0,0)]),
 fixture([(0,128),(72,100)]),fixture([(0,128),(0,128)],omit=True),
 fixture([(0,128),(0,128)],portal_flags=6),fixture([(0,128),(0,128)],portal_flags=5),
 fixture([(-64,64),(-32,64)]),fixture([(0,128)]),
]

def case(label,map_id,player=1,flags=1026,x=-24,y=0,z=0,mx=6,my=0,mz=0,f=25,s=0,turn=0,obstacle=0):
    return dict(label=label,map=map_id,player=player,flags=flags,x=x*U,y=y*U,z=z*U,momx=mx*U,momy=my*U,momz=mz*U,forward=f,side=s,turn=turn,obstacle=obstacle)
CASES=[case('equal',0),case('small_step',1),case('step_24',2),case('step_25',3),case('door_55',4),case('door_56',5),case('headroom',6,z=40),case('drop_24',7,flags=2),case('drop_restricted',8,player=0,flags=2),case('drop_allowed',9),case('reverse',2,x=24,z=24,mx=-6,f=-25),case('diagonal',10,x=-24,y=-16,mx=8,my=2),case('sparse_diagonal',11,x=-24,y=-16,mx=8,my=2),case('adjacent',12,mx=20),case('fall',0,x=-64,z=32,mx=0,f=0),case('ceiling',0,x=-64,z=64,mx=0,mz=16,f=0),case('negative_floor',18,x=-64,z=-16,mx=0,mz=-1,f=0),case('no_gravity',0,flags=514,x=-64,z=32,mx=0,f=0),case('exact_ceiling',0,x=-64,z=72,mx=0,f=0),case('no_gravity_ceiling',0,flags=514,x=-64,z=64,mx=0,mz=8,f=0),case('omitted_wall',15,player=0,flags=2,x=120,mx=20,f=0),case('block_monsters_player',16),case('block_monsters_actor',16,player=0,flags=2),case('blocking',17),case('zero_opening',13),case('narrow_opening',14),case('air_xy',0,z=64,mx=4,f=50,turn=512),case('boundary',1,x=0,mx=1),case('solid_z_independent',0,obstacle=1)]


def c_maps():
    code=[m3a.MAIN[:m3a.MAIN.index('int line_count;')], 'int line_count,numsectors;\nvoid load_world(int which) { int i;memset(blocklinks,0,sizeof(blocklinks));memset(sectors,0,sizeof(sectors));memset(lines,0,sizeof(lines));switch(which){']
    for k,m in enumerate(MAPS):
        code.append('case %d: {'%k)
        code.append('static short blocks[]={'+','.join(map(str,m['blocks']))+'};blockmaplump=blocks;blockmap=blocks+4;bmapwidth=4;bmapheight=2;bmaporgx=-256*FRACUNIT;bmaporgy=-128*FRACUNIT;')
        code.append(f"line_count={len(m['lines'])};numsectors={len(m['heights'])};numnodes={len(m['nodes'])};")
        for i,(f,c) in enumerate(m['heights']):code.append(f'sectors[{i}].floorheight={f}*FRACUNIT;sectors[{i}].ceilingheight={c}*FRACUNIT;subsectors[{i}].sector=&sectors[{i}];')
        for i,(x,y) in enumerate(m['vertices']):code.append(f'vertices[{i}].x={x}*FRACUNIT;vertices[{i}].y={y}*FRACUNIT;')
        for i,l in enumerate(m['lines']):
            code.append(f'init_line(&lines[{i}],&vertices[{l["a"]}],&vertices[{l["b"]}]);lines[{i}].frontsector=&sectors[{l["front"]}];lines[{i}].backsector='+('NULL' if l['back'] is None else f'&sectors[{l["back"]}]')+f';lines[{i}].flags={l["flags"]};lines[{i}].sidenum[0]={l["sides"][0]};lines[{i}].sidenum[1]={l["sides"][1]};')
        for i,n in enumerate(m['nodes']):code.append(f'nodes[{i}].x={n["x"]}*FRACUNIT;nodes[{i}].y={n["y"]}*FRACUNIT;nodes[{i}].dx={n["dx"]}*FRACUNIT;nodes[{i}].dy={n["dy"]}*FRACUNIT;nodes[{i}].children[0]={n["children"][0]};nodes[{i}].children[1]={n["children"][1]};')
        code.append('break;}')
    code.append('default:abort();}}')
    code.append('int input[][15]={'+','.join('{'+','.join(str(c[key]) for key in ['map','player','flags','x','y','z','momx','momy','momz','forward','side','turn','obstacle'])+',0,0}' for c in CASES)+'};')
    return '\n'.join(code)

MAIN=r'''
void ints(int* data,int n){int i;if(!n)putchar('-');for(i=0;i<n;i++)printf("%s%d",i?":":"",data[i]);}
void init_actor(mobj_t* m,int id,int flags,int x,int y,int z){memset(m,0,sizeof(*m));m->id=id;m->flags=flags;m->x=x;m->y=y;m->z=z;m->radius=8*FRACUNIT;m->height=56*FRACUNIT;m->state=&states[7];P_SetThingPosition(m);m->floorz=m->subsector->sector->floorheight;m->ceilingz=m->subsector->sector->ceilingheight;}
void movement(void){
 int c,t,i,j;player_t p;mobj_t m,other;
 puts("case,tic,x,y,z,momx,momy,momz,floor,ceiling,subsector,sector,angle,tries,lines,things,slides,openings,members,vertical,motion_order");
 for(c=0;c<CASE_COUNT;c++){
  load_world(input[c][0]);memset(&p,0,sizeof(p));
  if(input[c][12])init_actor(&other,1,MF_SOLID|MF_NOGRAVITY,8*FRACUNIT,0,64*FRACUNIT);
  init_actor(&m,0,input[c][2],input[c][3],input[c][4],input[c][5]);
  m.momx=input[c][6];m.momy=input[c][7];m.momz=input[c][8];
  if(input[c][1]){m.player=&p;p.mo=&m;p.viewheight=41*FRACUNIT;}
  for(t=0;t<24;t++){
   p.cmd.forwardmove=input[c][9];p.cmd.sidemove=input[c][10];p.cmd.angleturn=input[c][11];
   try_count=candidate_count=thing_count=slide_count=opening_count=motion_count=0;vertical_called=0;
   if(m.player)P_MovePlayer(&p);
   if(m.momx||m.momy)P_XYMovement(&m);
   if(m.z!=m.floorz||m.momz){vertical_called=1;before_z=m.z;before_momz=m.momz;P_ZMovement(&m);}
   printf("%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%ld,%u,",c,t,m.x,m.y,m.z,m.momx,m.momy,m.momz,m.floorz,m.ceilingz,(long)(m.subsector-subsectors),(long)(m.subsector->sector-sectors),m.angle);
   if(!try_count)putchar('-');for(i=0;i<try_count;i++){printf("%s",i?";":"");for(j=0;j<15;j++)printf("%s%d",j?":":"",try_data[i][j]);}putchar(',');
   ints(candidates,candidate_count);putchar(',');ints(things_seen,thing_count);putchar(',');ints(slides_seen,slide_count);putchar(',');
   if(!opening_count)putchar('-');for(i=0;i<opening_count;i++)for(j=0;j<5;j++)printf("%s%d",j?":":i?";":"",opening_data[i][j]);putchar(',');
   for(i=0;i<numsectors;i++){mobj_t* scan=sectors[i].thinglist;int found=0;printf("%s",i?"|":"");while(scan){printf("%s%d",found?":":"",scan->id);found++;scan=scan->snext;}if(!found)putchar('-');}putchar(',');
   if(vertical_called)printf("%d:%d",before_z,before_momz);else putchar('-');putchar(',');if(!motion_count)putchar('-');for(i=0;i<motion_count;i++)printf("%s%c%d",i?";":"",motion_kind[i]?'S':'T',motion_value[i]);putchar('\n');
  }
 }
}
void probes(void){
 int c,i;player_t p;mobj_t m;int points[][2]={{-4,0},{4,0},{-8,0},{8,0},{-1,-1},{0,0},{32,0},{40,16},{124,0}};
 puts("case,x,y,allowed,floor,ceiling,dropoff,floatok,reason,subsector,sector,line,actor,line_count,actor_count,opening_count");
 for(c=0;c<CASE_COUNT;c++)for(i=0;i<9;i++){
  load_world(input[c][0]);memset(&p,0,sizeof(p));init_actor(&m,0,input[c][2],input[c][3],input[c][4],input[c][5]);if(input[c][1]){m.player=&p;p.mo=&m;}
  try_count=candidate_count=thing_count=slide_count=opening_count=motion_count=0;
  P_TryMove(&m,points[i][0]*FRACUNIT,points[i][1]*FRACUNIT);
  printf("%d",c);{int j;for(j=0;j<15;j++)printf(",%d",try_data[0][j]);}putchar('\n');
 }
}
void bsp(void){int map,x,y;fixed_t p[]={-128*FRACUNIT,-64*FRACUNIT,-1,0,1,32*FRACUNIT,64*FRACUNIT,128*FRACUNIT};puts("map,x,y,subsector,sector");for(map=0;map<MAP_COUNT;map++){load_world(map);for(x=0;x<8;x++)for(y=0;y<8;y++){subsector_t* s=R_PointInSubsector(p[x],p[y]);printf("%d,%d,%d,%ld,%ld\n",map,p[x],p[y],(long)(s-subsectors),(long)(s->sector-sectors));}}}
void openings(void){int map,i;puts("map,line,top,bottom,range,lowfloor");for(map=0;map<MAP_COUNT;map++){load_world(map);for(i=0;i<line_count;i++)if(lines[i].backsector){Ref_LineOpening(&lines[i]);printf("%d,%d,%d,%d,%d,%d\n",map,i,opentop,openbottom,openrange,lowfloor);}}}
void sides(void){int d[][2]={{0,128},{0,-128},{128,0},{-128,0},{128,128},{128,-128},{-128,128},{-128,-128}};fixed_t p[]={-128*FRACUNIT,-64*FRACUNIT,-1,0,1,32*FRACUNIT,64*FRACUNIT,128*FRACUNIT};int i,x,y;node_t node;memset(&node,0,sizeof(node));puts("dx,dy,x,y,side");for(i=0;i<8;i++){node.dx=d[i][0]*FRACUNIT;node.dy=d[i][1]*FRACUNIT;for(x=0;x<8;x++)for(y=0;y<8;y++)printf("%d,%d,%d,%d,%d\n",node.dx,node.dy,p[x],p[y],R_PointOnSide(p[x],p[y],&node));}}
int main(int argc,char** argv){if(argc!=2)return 2;if(!strcmp(argv[1],"sides"))sides();else if(!strcmp(argv[1],"movement"))movement();else if(!strcmp(argv[1],"probes"))probes();else if(!strcmp(argv[1],"bsp"))bsp();else if(!strcmp(argv[1],"openings"))openings();else return 2;return 0;}
'''

FUNCTIONS=[('r_main.c','R_PointOnSide','int',None),('r_main.c','R_PointInSubsector','subsector_t*',None)]+[(f,n,r,('Ref_LineOpening' if n=='P_LineOpening' else rename)) for f,n,r,rename in m3a.FUNCTIONS]+[('p_maputl.c','P_LineOpening','void','Ref_LineOpening'),('p_mobj.c','P_ZMovement','void',None)]


def run(check):
    outputs={f'tests/reference/m3b_map_{i}.bin':m['bytes'] for i,m in enumerate(MAPS)}
    lines=['// Original synthetic fixtures generated independently of Rust; GPL-2.0-only.','/// Indexed classic synthetic maps for M3b comparisons.','pub const MAP_BYTES: &[&[u8]] = &[']+[f'    include_bytes!("m3b_map_{i}.bin"),' for i in range(len(MAPS))]+['];']
    outputs['tests/reference/m3b_maps.rs']=('\n'.join(lines)+'\n').encode()
    cases=io.StringIO();writer=csv.writer(cases,lineterminator='\n');writer.writerow(['case',*CASES[0].keys()]);writer.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3b_cases.csv']=cases.getvalue().encode()
    sine=base.table('tables.c',r'int finesine\[10240\]');angles=base.table('tables.c',r'angle_t tantoangle\[2049\]')
    fragments=['#include "m3b_shim.h"', 'int finesine[10240]={'+','.join(map(str,sine))+'};fixed_t* finecosine=finesine+2048;', 'angle_t tantoangle[2049]={'+','.join(map(str,angles))+'};']
    for f,n,r,rename in FUNCTIONS:
        text=base.body(f,n,r,rename)
        if n=='P_MovePlayer':text=text.replace('(cmd->angleturn<<16)','((uint32_t)(int32_t)cmd->angleturn << 16)')
        fragments.append(text)
    fragments.extend([c_maps(),f'#define CASE_COUNT {len(CASES)}\n#define MAP_COUNT {len(MAPS)}',MAIN])
    with tempfile.TemporaryDirectory(prefix='doom-m3b-reference-') as tmp:
        src=Path(tmp)/'reference.c';src.write_text('\n'.join(fragments));binary=Path(tmp)/'reference'
        subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),'-I'+str(ROOT/'tools/reference'),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'m_random.c'),'-o',str(binary)],check=True)
        for mode in ['movement','probes','bsp','openings','sides']:outputs[f'tests/reference/m3b_{mode}.csv']=subprocess.check_output([str(binary),mode])
    manifest=ROOT/'tests/reference/m3b_provenance.json'
    sources=sorted({'m_fixed.c','m_fixed.h','m_random.c','p_setup.c','p_local.h','r_defs.h','doomdata.h','p_tick.c','p_mobj.h','d_player.h'}|{f for f,_,_,_ in FUNCTIONS})
    adapters=['generate.py','generate_m3a.py','generate_m3b.py','m3b_shim.h']
    hashes=lambda paths,parent:{n:hashlib.sha256((parent/n).read_bytes()).hexdigest() for n in paths}
    evidence={'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','flags':FLAGS,'source_sha256':hashes(sources,SOURCE),'adapter_sha256':hashes(adapters,ROOT/'tools/reference'),'outputs_sha256':{n:hashlib.sha256(b).hexdigest() for n,b in outputs.items()}}
    if check:
        old=json.loads(manifest.read_text())
        for key,value in evidence.items():assert old[key]==value,key
    else:
        evidence.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform(),assumptions=['static classic synthetic 2/3-sector maps, original BSP/opening/collision/XY/Z/input bodies','motion adapter calls XY then Z using P_MobjThinker guards; no actor-state execution','C player view fields execute source side effects but are not physical outputs; sound stub counts only, no audio','supported flags zero/MF_SOLID/MF_DROPOFF/MF_NOGRAVITY; no missiles/skulls/float/corpse/combat/pickups/teleport','reference-list BLOCKMAP policy includes sparse diagonal and deliberately omitted wall; no rebuilding','GNU ABI arithmetic shift/narrowing and explicit unsigned angleturn shift; signed-overflow/abs(INT_MIN) positions excluded','old adapters/vectors/goldens unchanged; exact original synthetic bytes and input CSV emitted before Rust comparison'])
        outputs['tests/reference/m3b_provenance.json']=(json.dumps(evidence,indent=2)+'\n').encode()
    for path,data in outputs.items():
        dest=ROOT/path
        if check:assert dest.read_bytes()==data,path
        else:dest.write_bytes(data)
    print(('Checked' if check else 'Generated')+f' M3b maps and C vectors: {len(CASES)*24} movement tics, {len(CASES)*9} probes, {len(MAPS)*64} BSP lookups.')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
