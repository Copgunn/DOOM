#!/usr/bin/env python3
"""Genuine original geometry/thinker bodies on larger public classic fixtures."""
import argparse,csv,hashlib,io,json,platform,struct,subprocess,tempfile
from pathlib import Path
import generate as base
import generate_m3b as old
ROOT,SOURCE,FLAGS=base.ROOT,base.SOURCE,base.FLAGS
U=65536

def fixture(n=1,count=None,sparse=False,reverse=False,duplicate=False,heights=None,blockers=False,reject=False):
    heights=heights or [(0,128)]*n;xs=[-128+(256*i)//n for i in range(n+1)]
    vertices=[(x,y) for x in xs for y in [-64,64]];lines=[]
    def add(a,b,front,back=None,flags=1):lines.append(dict(a=a,b=b,front=front,back=back,flags=flags))
    add(0,1,0)
    for i in range(n):add(2*i+1,2*(i+1)+1,i)
    add(2*n+1,2*n,n-1)
    for i in range(n-1,-1,-1):add(2*(i+1),2*i,i)
    for i in range(1,n):add(2*i,2*i+1,i,i-1,4)
    count=count or len(lines)
    while len(lines)<count:
        k=len(lines);x=0 if blockers and k>=count-2 else -96+(k%193)
        a=len(vertices);vertices.extend([(x,-48),(x,48)])
        add(a,a+1,0,None if blockers and k>=count-2 else 0,1 if blockers and k>=count-2 else 4)
    sides=[];linebytes=b''
    for l in lines:
        ids=[]
        for sector in [l['front'],l['back']]:
            if sector is None:ids.append(-1)
            else:ids.append(len(sides));sides.append(sector)
        l['sides']=ids;linebytes+=old.words([l['a'],l['b'],l['flags'],0,0,*ids])
    segbytes=b'';subbytes=b'';first=0
    for sector in range(n):
        entries=[]
        for i,l in enumerate(lines):
            if l['front']==sector:side=0;a,b=l['a'],l['b']
            elif l['back']==sector:side=1;a,b=l['b'],l['a']
            else:continue
            entries.append(old.words([a,b,0,i,side,0]))
        subbytes+=old.words([len(entries),first]);first+=len(entries);segbytes+=b''.join(entries)
    nodes=[];nodebytes=b''
    for i in range(n-1):
        children=[0x8000+i+1,0x8000 if i==0 else i-1];x=xs[i+1]
        nodes.append(dict(x=x,y=0,dx=0,dy=128,children=children))
        nodebytes+=old.words([x,0,0,128,64,-64,x,128,64,-64,-128,x])+struct.pack('<HH',*children)
    blocks=[-256,-128,4,2]+[0]*8
    for cell in range(8):
        x=-256+(cell%4)*128;y=-128+(cell//4)*128
        listing=[]
        for i,l in enumerate(lines):
            a,b=vertices[l['a']],vertices[l['b']]
            if not sparse or (max(a[0],b[0])>=x and min(a[0],b[0])<=x+128 and max(a[1],b[1])>=y and min(a[1],b[1])<=y+128):listing.append(i)
        if reverse:listing.reverse()
        if duplicate:listing=listing+listing[:32]+listing[-32:]
        blocks[4+cell]=len(blocks);blocks.extend([0,*listing,-1])
    rejection=bytearray((n*n+7)//8)
    if reject:
        for pair in [(n-1)*n, n-1]:rejection[pair>>3]|=1<<(pair&7)
    lumps=[('MAP01',b''),('THINGS',b''),('LINEDEFS',linebytes),('SIDEDEFS',b''.join(old.words([0,0])+old.name('-')*2+old.name('TESTWALL')+old.words([s]) for s in sides)),('VERTEXES',b''.join(old.words(list(v)) for v in vertices)),('SEGS',segbytes),('SSECTORS',subbytes),('NODES',nodebytes),('SECTORS',b''.join(old.words([f,c])+b'TESTFLR0TESTCEIL'+old.words([160,0,0]) for f,c in heights)),('REJECT',bytes(rejection)),('BLOCKMAP',old.words(blocks))]
    return dict(heights=heights,vertices=vertices,lines=lines,nodes=nodes,blocks=blocks,bytes=old.wad(lumps),lumps=dict(lumps))

MAPS=[]
for count in [4,128,129,256,1024]:
    MAPS.extend([fixture(count=count,duplicate=True,reverse=True),fixture(count=count,sparse=True)])
for sectors in [128,129,256]:
    MAPS.extend([fixture(n=sectors,duplicate=True,reverse=True),fixture(n=sectors,sparse=True)])
for floor,ceiling in [(24,128),(25,128),(-24,128),(-25,128),(0,55),(0,56),(-64,96)]:
    MAPS.append(fixture(n=2,count=256,heights=[(0,128),(floor,ceiling)]))
MAPS.extend([fixture(count=256,blockers=True),fixture(count=256,blockers=True,reverse=True,duplicate=True),fixture(n=256,reject=True)])
def collision_grid(m,width,height,suffix=False):
    listing=[0,*range(len(m['lines'])-1,0,-1)]
    if suffix:listing.extend([0,*range(511,0,-1)])
    start=4+width*height;blocks=[-256,-128,width,height]+[start+(len(m['lines']) if suffix and i%2 else 0) for i in range(width*height)]+listing+[-1]
    m['blocks']=blocks;m['lumps']['BLOCKMAP']=old.words(blocks);m['bytes']=old.wad(list(m['lumps'].items()));return m
MAPS.extend([collision_grid(fixture(count=1024),4,2),collision_grid(fixture(count=1024),4,2,True),collision_grid(fixture(count=512),64,8)])
CASES=[old.case(f'map_{i}',i,player=0,flags=1026,x=-24,mx=6,f=0) for i in range(len(MAPS))]
CASES.extend([old.case('restricted_drop',19,player=0,flags=2,x=-24,mx=6,f=0),old.case('vertical_fall',8,player=0,flags=2,x=-64,z=32,mx=0,f=0),old.case('actor_order',8,player=0,flags=2,x=-24,mx=6,f=0,obstacle=1),old.case('negative_floor',22,player=0,flags=1026,x=24,z=-32,mx=-6,f=0)])
FUNCTIONS=[(f,n,r,('Ref_ZMovement' if n=='P_ZMovement' else rename)) for f,n,r,rename in old.FUNCTIONS]
FUNCTIONS += [('p_setup.c','P_LoadBlockMap','void',None),('p_tick.c','P_InitThinkers','void',None),('p_tick.c','P_AddThinker','void',None),('p_tick.c','P_RunThinkers','void',None),('p_mobj.c','P_MobjThinker','void','Ref_MobjThinker')]
FUNCTIONS += [('p_sight.c',n,r,None) for n,r in [('P_DivlineSide','int'),('P_InterceptVector2','fixed_t'),('P_CrossSubsector','boolean'),('P_CrossBSPNode','boolean'),('P_CheckSight','boolean')]]

def loader():
    old.MAPS=MAPS;old.CASES=CASES;s=old.c_maps()
    s=s.replace('int i;memset(blocklinks,0,sizeof(blocklinks));','int i;P_InitThinkers();prndindex=0;validcount=0;memset(blocklinks,0,sizeof(block_heads));')
    s=s.replace('blockmaplump=blocks;blockmap=blocks+4;bmapwidth=4;bmapheight=2;bmaporgx=-256*FRACUNIT;bmaporgy=-128*FRACUNIT;','active_blocks=blocks;active_block_bytes=sizeof(blocks);P_LoadBlockMap(0);')
    # Install the same classic seg and REJECT inputs for the genuine sight bodies.
    end=s.index('int input[][15]');first=s[:end].replace('void load_world(', 'void load_geometry(');rest=s[end:]
    extra=['void load_world(int which){load_geometry(which);memset(segs,0,sizeof(segs));memset(rejectmatrix,0,sizeof(rejectmatrix));switch(which){']
    for k,m in enumerate(MAPS):
        extra.append(f'case {k}:numsubsectors={len(m["heights"])};')
        for i,(count,start) in enumerate(struct.iter_unpack('<HH',m['lumps']['SSECTORS'])):extra.append(f'subsectors[{i}].numlines={count};subsectors[{i}].firstline={start};')
        for i,(a,b,angle,line,side,offset) in enumerate(struct.iter_unpack('<6H',m['lumps']['SEGS'])):
            l=m['lines'][line];front=l['front'] if side==0 else l['back'];back=l['back'] if side==0 else l['front']
            extra.append(f'segs[{i}].v1=&vertices[{a}];segs[{i}].v2=&vertices[{b}];segs[{i}].linedef=&lines[{line}];segs[{i}].frontsector=&sectors[{front}];segs[{i}].backsector='+('NULL' if back is None else f'&sectors[{back}]')+';')
        for i,value in enumerate(m['lumps']['REJECT']):
            if value:extra.append(f'rejectmatrix[{i}]={value};')
        extra.append('break;')
    return first+'\n'+'\n'.join(extra)+'default:abort();}}\n'+rest

def main():
    s=old.MAIN
    s=s.replace('m->state=&states[7];P_SetThingPosition(m);','m->state=&states[7];m->tics=-1;m->thinker.function.acp1=(actionf_p1)P_MobjThinker;m->lastlook=P_Random()%4;P_SetThingPosition(m);P_AddThinker(&m->thinker);')
    s=s.replace('motion_order");','motion_order,thinkers,rng,lastlook,radius,height,flags");')
    s=s.replace('if(m.player)P_MovePlayer(&p);\n   if(m.momx||m.momy)P_XYMovement(&m);\n   if(m.z!=m.floorz||m.momz){vertical_called=1;before_z=m.z;before_momz=m.momz;P_ZMovement(&m);}','thinker_count=0;P_RunThinkers();')
    s=s.replace('motion_value[i]);putchar(\'\\n\');','motion_value[i]);putchar(\',\');ints(thinkers_seen,thinker_count);printf(",%d,%d\\n",prndindex,m.lastlook);')
    s=s.replace('printf(",%d,%d\\n",prndindex,m.lastlook);','printf(",%d,%d,%d,%d,%d\\n",prndindex,m.lastlook,m.radius,m.height,m.flags);')
    # Every leaf/boundary is exercised, including the final high sector index.
    a=s.index('void bsp(void)');b=s.index('void openings(void)',a)
    s=s[:a]+'''void bsp(void){int map,x,d;puts("map,x,y,subsector,sector");for(map=0;map<MAP_COUNT;map++){load_world(map);for(x=-128;x<=128;x++)for(d=-1;d<=1;d++){fixed_t p=x*FRACUNIT+d;subsector_t* leaf=R_PointInSubsector(p,0);printf("%d,%d,0,%ld,%ld\\n",map,p,(long)(leaf-subsectors),(long)(leaf->sector-sectors));}}}
'''+s[b:]
    a=s.index('int main(')
    sight='''void sight_vectors(void){int map,a,b;fixed_t points[]={-127*FRACUNIT-1,-64*FRACUNIT,0,64*FRACUNIT,127*FRACUNIT+1};mobj_t observer,target;puts("map,ox,tx,pair,rejected,visible,eye,bottom,top,segs,rng");for(map=0;map<MAP_COUNT;map++)for(a=0;a<5;a++)for(b=0;b<5;b++){int pair,reject,visible;load_world(map);init_actor(&observer,0,MF_SOLID,points[a],0,R_PointInSubsector(points[a],0)->sector->floorheight);init_actor(&target,1,MF_SOLID,points[b],0,R_PointInSubsector(points[b],0)->sector->floorheight);observer.height=target.height=16*FRACUNIT;sight_count=0;sightzstart=topslope=bottomslope=0;pair=(observer.subsector->sector-sectors)*numsectors+(target.subsector->sector-sectors);reject=(rejectmatrix[pair>>3]>>(pair&7))&1;visible=P_CheckSight(&observer,&target);printf("%d,%d,%d,%d,%d,%d,%d,%d,%d,",map,points[a],points[b],pair,reject,visible,sightzstart,bottomslope,topslope);ints(sight_entries,sight_count);printf(",%d\\n",prndindex);}}
'''
    s=s[:a]+sight+s[a:];s=s.replace('if(!strcmp(argv[1],"sides"))','if(!strcmp(argv[1],"sight"))sight_vectors();else if(!strcmp(argv[1],"sides"))')
    return s

def run(check):
    outputs={f'tests/reference/m3f1_map_{i}.bin':m['bytes'] for i,m in enumerate(MAPS)}
    outputs['tests/reference/m3f1_maps.rs']=('// Generated public classic fixtures; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("m3f1_map_{i}.bin"),\n' for i in range(len(MAPS)))+'];\n').encode()
    text=io.StringIO();w=csv.writer(text,lineterminator='\n');w.writerow(['case',*CASES[0]]);w.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3f1_cases.csv']=text.getvalue().encode()
    sine=base.table('tables.c',r'int finesine\[10240\]');angles=base.table('tables.c',r'angle_t tantoangle\[2049\]')
    parts=['#include "m3f1_shim.h"','int finesine[10240]={'+','.join(map(str,sine))+'};fixed_t* finecosine=finesine+2048;','angle_t tantoangle[2049]={'+','.join(map(str,angles))+'};']
    for f,n,r,rename in FUNCTIONS:
        body=base.body(f,n,r,rename)
        if n=='P_MovePlayer':body=body.replace('(cmd->angleturn<<16)','((uint32_t)(int32_t)cmd->angleturn << 16)')
        if n=='P_CrossSubsector':body=body.replace('line = seg->linedef;','line = seg->linedef; observe_seg((int)(line-lines),line->validcount==validcount);')
        parts.append(body)
    parts.extend([loader(),f'#define CASE_COUNT {len(CASES)}\n#define MAP_COUNT {len(MAPS)}',main()])
    with tempfile.TemporaryDirectory(prefix='doom-m3f1-c-') as d:
        src=Path(d)/'reference.c';src.write_text('\n'.join(parts));binary=Path(d)/'reference'
        subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),'-I'+str(ROOT/'tools/reference'),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'m_random.c'),'-o',str(binary)],check=True)
        for mode in ['movement','probes','bsp','openings','sight']:outputs[f'tests/reference/m3f1_{mode}.csv']=subprocess.check_output([str(binary),mode])
    hashes=lambda paths,parent:{n:hashlib.sha256((parent/n).read_bytes()).hexdigest() for n in paths}
    sources=sorted({f for f,_,_,_ in FUNCTIONS}|{'m_fixed.c','m_fixed.h','m_random.c','m_random.h','doomtype.h','p_mobj.h','p_local.h','doomdata.h','r_defs.h'})
    adapters=['generate.py','generate_m3a.py','generate_m3b.py','generate_m3f1.py','m3f1_shim.h']
    p=dict(source_revision='a77dfb96cb91780ca334d0d4cfd86957558007e0',flags=FLAGS,source_sha256=hashes(sources,SOURCE),adapter_sha256=hashes(adapters,ROOT/'tools/reference'),functions=[f'{f}:{n}' for f,n,_,_ in FUNCTIONS],maps=len(MAPS),scenarios=len(CASES),outputs_sha256={n:hashlib.sha256(b).hexdigest() for n,b in outputs.items()})
    manifest=ROOT/'tests/reference/m3f1_provenance.json'
    if check:
        existing=json.loads(manifest.read_text())
        for k,v in p.items():assert existing[k]==v,k
    else:
        p.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform(),assumptions=['named-field synthetic ABI adapter; not complete original actor admission','genuine P_LoadBlockMap with little-endian SHORT and bounded fixture-backed allocation/cache','genuine P_RunThinkers/P_MobjThinker with unbound flags2/512/1024 and terminal synthetic state; no state transitions, respawn, damage or player execution','geometry/seg initialization adapter from independently generated classic bytes; not complete P_SetupLevel','observational wrappers retain source bodies; unsupported callbacks abort','instrumentation intercept storage enlarged but no player binding or sliding scenario enters it','signed GNU ABI/overflow flags inherited; tested coordinates bounded, source negative fixed shifts documented','all earlier generators/fixtures/helpers unchanged; expectations never read Rust'])
        outputs['tests/reference/m3f1_provenance.json']=(json.dumps(p,indent=2)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/name
        if check:assert path.read_bytes()==data,name
        else:path.write_bytes(data)
    print(f'{"Checked" if check else "Generated"} M3f1: {len(FUNCTIONS)} genuine bodies, {len(CASES)} scripts, {len(MAPS)} maps, {len(CASES)*24} movement captures.')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
