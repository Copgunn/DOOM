#!/usr/bin/env python3
"""Expanded public inputs, genuine Linux 1.10 bodies; no Rust expectations."""
import argparse, csv, hashlib, io, json, platform, struct, subprocess, tempfile
from pathlib import Path
import generate as base
import generate_m3b as old
import m3f1_reference as prior
ROOT, SOURCE, FLAGS, U = base.ROOT, base.SOURCE, base.FLAGS, 65536
TICS = 128

def finish(m):
    m['bytes'] = old.wad(list(m['lumps'].items()))
    return m

def grid(m, origin, width, height, suffix=False, sparse=False):
    listing = [0, *range(len(m['lines'])-1, -1, -1), 0, *range(len(m['lines']))]
    start = 4+width*height
    xs, ys = zip(*m['vertices'])
    offsets=[]
    for i in range(width*height):
        x=origin[0]+(i%width)*128; y=origin[1]+(i//width)*128
        empty=sparse and (x>max(xs) or x+128<min(xs) or y>max(ys) or y+128<min(ys))
        offsets.append(start+len(listing)+1 if empty else start+(len(m['lines'])+1 if suffix and i%2!=0 else 0))
    # Classic parser requires every referenced list/suffix to start with zero.
    # A conventionally empty cell still visits source line zero.
    m['blocks']=[*origin,width,height,*offsets,*listing,-1,0,-1]
    m['lumps']['BLOCKMAP']=old.words(m['blocks'])
    return finish(m)

def scene(n=1, count=None, tx=0, ty=0, sx=1, sy=1, shear=0, heights=None, reject=False):
    m=prior.fixture(n=n,count=count,heights=heights,reject=reject)
    m['vertices']=[(sx*x+shear*y+tx,sy*y+ty) for x,y in m['vertices']]
    m['lumps']['VERTEXES']=b''.join(old.words(v) for v in m['vertices'])
    nodes=[]; raw=b''
    xs,ys=zip(*m['vertices'])
    for node in m['nodes']:
        x=sx*node['x']+shear*node['y']+tx; y=sy*node['y']+ty
        dx=sx*node['dx']+shear*node['dy']; dy=sy*node['dy']
        nodes.append(dict(x=x,y=y,dx=dx,dy=dy,children=node['children']))
        raw+=old.words([x,y,dx,dy,*([max(ys),min(ys),min(xs),max(xs)]*2)])+struct.pack('<HH',*node['children'])
    m['nodes']=nodes; m['lumps']['NODES']=raw; m['center']=(tx,ty)
    m['range']=(min(xs),max(xs))
    origin=(max(-2048,min(xs)//128*128-128),max(-2048,min(ys)//128*128-128))
    width=(max(xs)-origin[0])//128+2; height=(max(ys)-origin[1])//128+2
    return grid(m,origin,width,height)

MAPS=[scene(),scene(tx=384),scene(tx=385),scene(n=4,tx=1500,ty=1500),
      scene(n=4,tx=-1500,ty=-1500),scene(tx=1920),scene(tx=-1920),
      scene(n=16,sx=16),scene(n=16,sx=16,heights=[(-64,96)]*16),
      scene(sx=6,tx=1200),scene(n=2,tx=1536,ty=1536,sy=6,shear=6,heights=[(0,128),(24,96)])]
for floor,ceil in [(24,128),(25,128),(-24,128),(-25,128),(0,55),(0,56)]:
    MAPS.append(scene(n=2,tx=1500,heights=[(0,128),(floor,ceil)]))
MAPS += [scene(n=2,tx=-1500,heights=[(-1024,-896),(-1000,-896)]),
         scene(heights=[(896,1024)],tx=1500),scene(heights=[(-1024,1024)]),
         grid(scene(count=32,tx=1500,ty=1500),(-2048,-2048),256,32),
         grid(scene(count=32,tx=1500,ty=1500),(-2048,-2048),256,32,suffix=True),
         grid(scene(count=32,tx=1500,ty=1500),(-2048,-2048),256,32,sparse=True),
         scene(n=16,sx=16,reject=True),
         grid(scene(tx=1500,ty=1500),(1024,1024),8,8),
         grid(scene(tx=-1500,ty=-1500),(-2048,-2048),8,8)]

def dag(levels):
    m=scene();m['lumps']['SSECTORS']=old.words([1,0]);m['nodes']=[];raw=b''
    for i in range(levels):
        child=0x8000 if i==0 else i-1
        m['nodes'].append(dict(x=0,y=0,dx=0,dy=128,children=[child,child]))
        raw+=old.words([0,0,0,128,64,-64,-128,128,64,-64,-128,128])+struct.pack('<HH',child,child)
    m['lumps']['NODES']=raw
    return finish(m)
MAPS += [dag(11),dag(12)]
MAPS += [scene(n=2,tx=1500,heights=[(0,128),(128,256)])]
MAPS += [scene(count=1024,tx=1500,ty=1500)]
CASES=[]
for i,m in enumerate(MAPS):
    x,y=m['center']; f=m['heights'][0][0]
    CASES.append(old.case(f'map_{i}',i,player=0,flags=1026,x=x-24,y=y,z=f,mx=6,f=0))
CASES += [old.case('wide_positive',7,player=0,flags=1538,x=-1920,z=16,mx=30,f=0),
          old.case('wide_negative',7,player=0,flags=1538,x=1920,z=16,mx=-30,f=0),
          old.case('wide_negative_floor',8,player=0,flags=1538,x=-1920,z=-48,mx=30,f=0),
          old.case('gravity',3,player=0,flags=2,x=1476,y=1500,z=64,mx=8,mz=-1,f=0),
          old.case('restricted_drop',14,player=0,flags=2,x=1476,mx=6,f=0),
          old.case('two_actors_high_cell',20,player=0,flags=2,x=1476,y=1500,mx=6,f=0,obstacle=1)]
FUNCTIONS=prior.FUNCTIONS

def loader():
    prior.MAPS=MAPS;prior.CASES=CASES
    text=prior.loader()
    centers='{'+','.join('{%d,%d}'%m['center'] for m in MAPS)+'}'
    return text+f'\nint centers[][2]={centers};\n'

def sight_body(body,name):
    if name=='P_CrossBSPNode':
        body=body.replace('int\t\tside;','int\t\tside; int ending;')
        body=body.replace('if (side == 2)','observe(0,bspnum,side,0,0);\n    if (side == 2)')
        body=body.replace('if (side == P_DivlineSide (t2x, t2y,(divline_t *)bsp))',
            'ending=P_DivlineSide(t2x,t2y,(divline_t*)bsp); observe(1,bspnum,ending,0,0);\n    if (side == ending)')
    if name=='P_CrossSubsector':
        body=body.replace('sub = &subsectors[num];','observe(2,num,0,0,0); sub = &subsectors[num];')
        body=body.replace('line = seg->linedef;', 'line = seg->linedef; observe_seg((int)(line-lines),line->validcount==validcount); observe(3,num,(int)(seg-segs),(int)(line-lines),line->validcount==validcount);')
        body=body.replace('s2 = P_DivlineSide (v2->x, v2->y, &strace);','s2 = P_DivlineSide (v2->x, v2->y, &strace); observe(4,(int)(line-lines),0,s1,s2);')
        body=body.replace('s2 = P_DivlineSide (t2x, t2y, &divl);','s2 = P_DivlineSide (t2x, t2y, &divl); observe(4,(int)(line-lines),1,s1,s2);')
        body=body.replace('if ( !(line->flags & ML_TWOSIDED) )\n\t    return false;',
            'if ( !(line->flags & ML_TWOSIDED) ) { observe(7,(int)(line-lines),0,0,0); return false; }')
        body=body.replace('if (openbottom >= opentop)', 'observe(5,(int)(line-lines),opentop,openbottom,0);\n\tif (openbottom >= opentop)')
        body=body.replace('if (openbottom >= opentop)\t\n\t    return false;',
            'if (openbottom >= opentop) { observe(7,(int)(line-lines),1,0,0); return false; }')
        body=body.replace('if (topslope <= bottomslope)', 'observe(6,(int)(line-lines),frac,bottomslope,topslope);\n\tif (topslope <= bottomslope)')
        body=body.replace('if (topslope <= bottomslope)\n\t    return false;',
            'if (topslope <= bottomslope) { observe(7,(int)(line-lines),2,0,0); return false; }')
    return body

def main():
    text=prior.main().replace('t<24','t<128')
    text=text.replace('8*FRACUNIT,0,64*FRACUNIT','(centers[input[c][0]][0]+8)*FRACUNIT,centers[input[c][0]][1]*FRACUNIT,64*FRACUNIT')
    text=text.replace('points[i][0]*FRACUNIT,points[i][1]*FRACUNIT','(centers[input[c][0]][0]+points[i][0])*FRACUNIT,(centers[input[c][0]][1]+points[i][1])*FRACUNIT')
    a=text.index('void bsp(void)');b=text.index('void openings(void)',a)
    queries=[]
    for k,m in enumerate(MAPS):
        points=set([m['range'][0],m['range'][1],m['center'][0],-2018,2018])
        points.update(n['x'] for n in m['nodes'])
        for x in sorted(points):
            for d in [-1,0,1]:
                raw=x*U+d
                if abs(raw)<=2048*U:queries.append([k,raw,m['center'][1]*U])
    text=text[:a]+'''void bsp(void){int i;int points[][3]={'''+','.join('{'+','.join(map(str,q))+'}' for q in queries)+'''};puts("map,x,y,subsector,sector");for(i=0;i<(int)(sizeof(points)/sizeof(points[0]));i++){subsector_t* leaf;load_world(points[i][0]);leaf=R_PointInSubsector(points[i][1],points[i][2]);printf("%d,%d,%d,%ld,%ld\\n",points[i][0],points[i][1],points[i][2],(long)(leaf-subsectors),(long)(leaf->sector-sectors));}}
'''+text[b:]
    a=text.index('void sight_vectors(void)');b=text.index('int main(',a)
    text=text[:a]+r'''void sight_vectors(void){int map,a,b;int offsets[][2]={{-127,0},{-64,-16},{0,0},{64,16},{127,0}};mobj_t observer,target;
puts("map,ox,oy,tx,ty,height,pair,rejected,visible,eye,bottom,top,segs,rng,visits");
for(map=0;map<MAP_COUNT;map++)for(a=0;a<5;a++)for(b=0;b<5;b++){int pair,reject,visible,ox,oy,tx,ty,height;
load_world(map);ox=(centers[map][0]+offsets[a][0])*FRACUNIT;oy=(centers[map][1]+offsets[a][1])*FRACUNIT;tx=(centers[map][0]+offsets[b][0])*FRACUNIT;ty=(centers[map][1]+offsets[b][1])*FRACUNIT;
if(map==5||map==6){ox=(centers[map][0]+offsets[a][0]/2)*FRACUNIT;tx=(centers[map][0]+offsets[b][0]/2)*FRACUNIT;}
if(map==7||map==8||map==23){ox=offsets[a][0]*15*FRACUNIT;tx=offsets[b][0]*15*FRACUNIT;oy=ty=0;}
init_actor(&observer,0,MF_SOLID,ox,oy,R_PointInSubsector(ox,oy)->sector->floorheight);init_actor(&target,1,MF_SOLID,tx,ty,R_PointInSubsector(tx,ty)->sector->floorheight);
height=map==19?2048:16;observer.height=target.height=height*FRACUNIT;sight_count=sight_order_count=0;sightzstart=topslope=bottomslope=0;pair=(observer.subsector->sector-sectors)*numsectors+(target.subsector->sector-sectors);reject=(rejectmatrix[pair>>3]>>(pair&7))&1;visible=P_CheckSight(&observer,&target);
printf("%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,",map,ox,oy,tx,ty,height,pair,reject,visible,sightzstart,bottomslope,topslope);ints(sight_entries,sight_count);printf(",%d,",prndindex);ints(sight_order,sight_order_count);putchar('\n');}}
void arithmetic(void){int x,y,dx,flip;int delta[][2]={{1536,0},{0,1536},{768,768},{768,-768},{1,1}};puts("lx,ly,dx,dy,rx,ry,rdx,rdy,fraction,side,nx,ny,ndx,ndy,node_div,node_point");
for(x=-1;x<=1;x++)for(y=-1;y<=1;y++)for(dx=0;dx<5;dx++)for(flip=-1;flip<=1;flip+=2){divline_t line,ray;node_t node;
line.x=x*512*FRACUNIT;line.y=y*512*FRACUNIT;line.dx=delta[dx][0]*flip*FRACUNIT;line.dy=delta[dx][1]*FRACUNIT;
ray.x=-2048*FRACUNIT+1;ray.y=2048*FRACUNIT-1;ray.dx=4096*FRACUNIT-2;ray.dy=-4096*FRACUNIT+2;
node.x=x*2048*FRACUNIT;node.y=y*2048*FRACUNIT;node.dx=flip*2048*FRACUNIT;node.dy=2048*FRACUNIT;
printf("%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",line.x,line.y,line.dx,line.dy,ray.x,ray.y,ray.dx,ray.dy,P_InterceptVector2(&ray,&line),P_DivlineSide(ray.x,ray.y,&line),node.x,node.y,node.dx,node.dy,P_DivlineSide(ray.x,ray.y,(divline_t*)&node),R_PointOnSide(ray.x,ray.y,&node));}}
void boundary(void){divline_t line={0,0,5898240,15335424},ray={-4394478,-11489288,3686402,9584642};
puts("lx,ly,dx,dy,rx,ry,rdx,rdy,line_start_side,line_end_side,observer_side,target_side,fraction");
printf("%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",line.x,line.y,line.dx,line.dy,ray.x,ray.y,ray.dx,ray.dy,P_DivlineSide(line.x,line.y,&ray),P_DivlineSide(line.x+line.dx,line.y+line.dy,&ray),P_DivlineSide(ray.x,ray.y,&line),P_DivlineSide(ray.x+ray.dx,ray.y+ray.dy,&line),P_InterceptVector2(&ray,&line));
/* Do not call source FixedDiv with that fraction: abs(INT_MIN) is undefined.
 * This component records the trigger, not a fabricated whole-sight result. */}
void undefined_clip(void){divline_t line={0,0,5898240,15335424},ray={-4394478,-11489288,3686402,9584642};fixed_t frac=P_InterceptVector2(&ray,&line);printf("excluded source fraction=%d\n",frac);fflush(stdout);printf("%d\n",FixedDiv(12*FRACUNIT,frac));}
'''+text[b:]
    return text.replace('if(!strcmp(argv[1],"sight"))','if(!strcmp(argv[1],"undefined-clip"))undefined_clip();else if(!strcmp(argv[1],"boundary"))boundary();else if(!strcmp(argv[1],"arithmetic"))arithmetic();else if(!strcmp(argv[1],"sight"))')

def run(check, overflow, probe=False):
    outputs={f'tests/reference/m3f2_map_{i}.bin':m['bytes'] for i,m in enumerate(MAPS)}
    outputs['tests/reference/m3f2_maps.rs']=('// Generated synthetic classic maps; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("m3f2_map_{i}.bin"),\n' for i in range(len(MAPS)))+'];\n').encode()
    stream=io.StringIO();w=csv.writer(stream,lineterminator='\n');w.writerow(['case',*CASES[0]]);w.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3f2_cases.csv']=stream.getvalue().encode()
    parts=['#include "m3f2_shim.h"','int finesine[10240]={'+','.join(map(str,base.table('tables.c',r'int finesine\[10240\]')))+'};fixed_t* finecosine=finesine+2048;', 'angle_t tantoangle[2049]={'+','.join(map(str,base.table('tables.c',r'angle_t tantoangle\[2049\]')))+'};']
    for f,n,r,rename in FUNCTIONS:
        body=base.body(f,n,r,rename)
        if n=='P_MovePlayer':body=body.replace('(cmd->angleturn<<16)','((uint32_t)(int32_t)cmd->angleturn << 16)')
        parts.append(sight_body(body,n))
    parts.extend([loader(),f'#define CASE_COUNT {len(CASES)}\n#define MAP_COUNT {len(MAPS)}',main()])
    modes=['movement','probes','bsp','openings','sight','arithmetic','boundary']
    audit_flags=['-fno-wrapv','-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow','-fno-sanitize-recover=all']
    with tempfile.TemporaryDirectory(prefix='doom-m3f2-c-') as directory:
        src=Path(directory)/'reference.c';src.write_text('\n'.join(parts));binary=Path(directory)/'reference'
        command=['cc',*FLAGS,'-I'+str(SOURCE),'-I'+str(ROOT/'tools/reference'),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'m_random.c'),'-o',str(binary)]
        subprocess.run(command,check=True)
        for mode in modes:outputs[f'tests/reference/m3f2_{mode}.csv']=subprocess.check_output([str(binary),mode])
        if overflow or probe:
            subprocess.run([*command,*audit_flags],check=True)
            for mode in modes:assert subprocess.check_output([str(binary),mode])==outputs[f'tests/reference/m3f2_{mode}.csv'],mode
        if probe:
            result=subprocess.run([str(binary),'undefined-clip'],capture_output=True,text=True)
            assert result.returncode!=0 and 'negation of -2147483648' in result.stderr
            print('Expected failure in excluded original-C clip: '+result.stderr.strip().replace(str(SOURCE)+'/',''))
    sources=sorted({f for f,_,_,_ in FUNCTIONS}|{'m_fixed.c','m_fixed.h','m_random.c','m_random.h','tables.c','doomtype.h','p_mobj.h','p_local.h','doomdata.h','r_defs.h'})
    adapters=['generate.py','generate_m3a.py','generate_m3b.py','m3f1_reference.py','m3f2_reference.py','m3f2_shim.h']
    hashes=lambda paths,parent:{p:hashlib.sha256((parent/p).read_bytes()).hexdigest() for p in paths}
    p=dict(source_revision='a77dfb96cb91780ca334d0d4cfd86957558007e0',flags=FLAGS,overflow_audit_flags=audit_flags,source_sha256=hashes(sources,SOURCE),adapter_sha256=hashes(adapters,ROOT/'tools/reference'),functions=[f'{f}:{n}' for f,n,_,_ in FUNCTIONS],maps=len(MAPS),scenarios=len(CASES),tics=TICS,outputs_sha256={n:hashlib.sha256(b).hexdigest() for n,b in outputs.items()})
    if check:
        existing=json.loads((ROOT/'tests/reference/m3f2_provenance.json').read_text())
        for k,v in p.items():assert existing[k]==v,k
    else:
        p.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform(),assumptions=['GNU89 int32 fixed/int, signed16 short, arithmetic right shift, GNU negative fixed-left-shift extension, little-endian SHORT, IEEE binary64','named-field geometry/actor/state adapter, not complete original ABI or P_SetupLevel/P_GroupLines','genuine P_LoadBlockMap, P_RunThinkers, P_MobjThinker, XY/Z/collision/BSP/sight bodies; unbound terminal state; unsupported callbacks abort','full ordered sight observations instrument original expressions once, preserving source decisions','negative setup shifts are a pinned GNU extension; overflow audit covers signed addition/multiplication, integer division and float casts, not shift portability','one raw fixed unit inside actor extrema in primitive arithmetic; unsafe outside-domain policies are Rust-only','source RNG indexes are compared; no exhaustive raw-draw or function/branch coverage claim'])
        outputs['tests/reference/m3f2_provenance.json']=(json.dumps(p,indent=2)+'\n').encode()
    for n,data in outputs.items():
        if check:assert (ROOT/n).read_bytes()==data,n
        else:(ROOT/n).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} M3f2: {len(FUNCTIONS)} genuine bodies, {len(CASES)} scripts, {len(MAPS)} maps, {len(CASES)*TICS} tics; overflow audit={overflow}.')
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--probe-undefined-clip',action='store_true',help='expect original abs(INT_MIN) sanitizer failure outside the admitted slice');args=parser.parse_args();run(args.check,args.overflow_check,args.probe_undefined_clip)
