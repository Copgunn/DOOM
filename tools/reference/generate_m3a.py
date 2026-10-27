#!/usr/bin/env python3
"""Independent M3a source adapter. Leaves the M2 generator/provenance unchanged."""
import argparse
import hashlib
import json
import platform
import subprocess
import tempfile
from pathlib import Path
import generate as m2

ROOT, SOURCE, FLAGS = m2.ROOT, m2.SOURCE, m2.FLAGS
MAIN = m2.MAIN[m2.MAIN.index('void init_line'):m2.MAIN.index('void geometry_vectors')] + r'''
int line_count;
void arena(int angled) {
    int square[][2]={{-64,-64},{-64,64},{64,64},{64,-64}};
    int bevel[][2]={{-64,-64},{-64,64},{16,64},{64,16},{64,-64}};
    static short blocks[32];int i,n=angled?5:4;
    memset(blocklinks,0,sizeof(blocklinks));sector.thinglist=NULL;line_count=n;
    for(i=0;i<n;i++){vertices[i].x=(angled?bevel[i][0]:square[i][0])*FRACUNIT;vertices[i].y=(angled?bevel[i][1]:square[i][1])*FRACUNIT;}
    for(i=0;i<n;i++)init_line(&lines[i],&vertices[i],&vertices[(i+1)%n]);
    blocks[0]=blocks[1]=-128;blocks[2]=blocks[3]=2;
    for(i=4;i<8;i++)blocks[i]=8;blocks[8]=0;
    for(i=0;i<n;i++)blocks[9+i]=i;blocks[9+n]=-1;
    blockmaplump=blocks;blockmap=blocks+4;
}
void actor(mobj_t* m,int id,int x,int y,int solid) {
    memset(m,0,sizeof(*m));m->x=x;m->y=y;m->id=id;m->flags=solid?MF_SOLID:0;
    m->radius=8*FRACUNIT;m->height=56*FRACUNIT;m->z=m->floorz=sector.floorheight;
    m->ceilingz=sector.ceilingheight;m->state=&states[7];P_SetThingPosition(m);
}
void ints(int* data,int n) { int i;if(!n)putchar('-');for(i=0;i<n;i++)printf("%s%d",i?":":"",data[i]); }
void vectors(void) {
    int v[][6]={
      {40,0,20*FRACUNIT,0,0,1},{40,0,20*FRACUNIT,10*FRACUNIT,0,1},
      {0,40,10*FRACUNIT,20*FRACUNIT,0,1},{40,40,20*FRACUNIT,20*FRACUNIT,0,1},
      {-40,-40,-20*FRACUNIT,-10*FRACUNIT,0,1},{40,0,4095,-4095,0,1},
      {40,0,4095,-4095,0,1},{0,0,INT_MAX,INT_MIN,0,1},
      {20,20,20*FRACUNIT,8*FRACUNIT,1,1},{0,44,20*FRACUNIT,10*FRACUNIT,1,1},
      {0,0,0,0,1,1},{55,-55,FRACUNIT,-FRACUNIT,0,1},
      {0,0,20*FRACUNIT,8*FRACUNIT,0,1},{0,0,20*FRACUNIT,8*FRACUNIT,0,0},
      {0,0,FRACUNIT,0,0,1},{0,0,20*FRACUNIT,8*FRACUNIT,0,1},
      {55,55,FRACUNIT,FRACUNIT,1,1},{0,0,20*FRACUNIT,8*FRACUNIT,0,1}
    };
    int c,t,j,alive; mobj_t m,a,b,replacement;player_t p;
    puts("case,tic,arena,player,forward,side,turn,start_x,start_y,input_momx,input_momy,x,y,momx,momy,angle,tries,lines,things,slides");
    for(c=0;c<18;c++) {
      arena(v[c][4]);memset(&p,0,sizeof(p));alive=c>=12;
      if(alive){actor(&a,1,(c==14?0:20)*FRACUNIT,0,1);actor(&b,2,(c==14?0:20)*FRACUNIT,0,c!=15);}
      actor(&m,0,v[c][0]*FRACUNIT,v[c][1]*FRACUNIT,1);m.momx=v[c][2];m.momy=v[c][3];
      if(v[c][5]){p.mo=&m;m.player=&p;}
      for(t=0;t<12;t++) {
        int x=m.x,y=m.y,mx=m.momx,my=m.momy;
        p.cmd.forwardmove=c==5?1:c==10?50:0;p.cmd.sidemove=c==10?24:0;p.cmd.angleturn=c==10?512:0;
        if(c==17&&t==2)a.momx=FRACUNIT;
        if(c==17&&t==3){P_UnsetThingPosition(&a);alive=0;}
        if(c==17&&t==4)actor(&replacement,3,20*FRACUNIT,0,1);
        if(c>=12&&alive)P_XYMovement(&a);
        if(c>=12)P_XYMovement(&b);
        try_count=candidate_count=thing_count=slide_count=0;
        if(m.player)P_MovePlayer(&p);
        P_XYMovement(&m);
        printf("%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%u,",c,t,v[c][4],v[c][5],p.cmd.forwardmove,p.cmd.sidemove,p.cmd.angleturn,x,y,mx,my,m.x,m.y,m.momx,m.momy,m.angle);
        if(!try_count)putchar('-');for(j=0;j<try_count;j++)printf("%s%d:%d:%d",j?";":"",try_x[j],try_y[j],try_allowed[j]);putchar(',');
        ints(candidates,candidate_count);putchar(',');ints(things_seen,thing_count);putchar(',');ints(slides_seen,slide_count);putchar('\n');
      }
    }
}
int main(void) { vectors();return 0; }
'''
FUNCTIONS = [
 ('p_maputl.c','P_AproxDistance','fixed_t',None),
 ('p_maputl.c','P_PointOnLineSide','int',None),('p_maputl.c','P_BoxOnLineSide','int',None),
 ('p_maputl.c','P_PointOnDivlineSide','int',None),('p_maputl.c','P_MakeDivline','void',None),
 ('p_maputl.c','P_InterceptVector','fixed_t',None),
 ('p_maputl.c','P_UnsetThingPosition','void',None),('p_maputl.c','P_SetThingPosition','void',None),
 ('p_maputl.c','P_BlockLinesIterator','boolean',None),('p_maputl.c','P_BlockThingsIterator','boolean',None),
 ('p_maputl.c','PIT_AddLineIntercepts','boolean',None),('p_maputl.c','P_TraverseIntercepts','boolean',None),
 ('p_maputl.c','P_PathTraverse','boolean',None),
 ('tables.c','SlopeDiv','int',None),('r_main.c','R_PointToAngle','angle_t',None),('r_main.c','R_PointToAngle2','angle_t',None),
 ('p_map.c','PIT_CheckLine','boolean','Ref_CheckLine'),('p_map.c','PIT_CheckThing','boolean','Ref_CheckThing'),
 ('p_map.c','P_CheckPosition','boolean',None),('p_map.c','P_TryMove','boolean','Ref_TryMove'),
 ('p_map.c','P_HitSlideLine','void','Ref_HitSlideLine'),('p_map.c','PTR_SlideTraverse','boolean',None),
 ('p_map.c','P_SlideMove','void',None),('p_mobj.c','P_XYMovement','void',None),
 ('p_user.c','P_Thrust','void',None),('p_user.c','P_MovePlayer','void',None),
]


def run(check):
    manifest=ROOT/'tests/reference/m3a_provenance.json'
    if check:
        p=json.loads(manifest.read_text())
        assert p['flags']==FLAGS
        for n,h in p['source_sha256'].items():
            assert hashlib.sha256((SOURCE/n).read_bytes()).hexdigest()==h,n
        for n,h in p['adapter_sha256'].items():
            assert hashlib.sha256((ROOT/'tools/reference'/n).read_bytes()).hexdigest()==h,n
    sine=m2.table('tables.c',r'int finesine\[10240\]')
    angles=m2.table('tables.c',r'angle_t tantoangle\[2049\]')
    fragments=['#include "m3a_shim.h"', 'int finesine[10240]={'+','.join(map(str,sine))+'};fixed_t* finecosine=finesine+2048;', 'angle_t tantoangle[2049]={'+','.join(map(str,angles))+'};']
    for file,name,result,rename in FUNCTIONS:
        fragment=m2.body(file,name,result,rename)
        if name=='P_MovePlayer':
            fragment=fragment.replace('(cmd->angleturn<<16)','((uint32_t)(int32_t)cmd->angleturn << 16)')
        fragments.append(fragment)
    fragments.append(MAIN)
    with tempfile.TemporaryDirectory(prefix='doom-m3a-reference-') as tmp:
        src=Path(tmp)/'reference.c';src.write_text('\n'.join(fragments));binary=Path(tmp)/'ref'
        subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),'-I'+str(ROOT/'tools/reference'),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'m_random.c'),'-o',str(binary)],check=True)
        output=subprocess.check_output([str(binary)],text=True)
    generated={'tests/reference/m3a_movement.csv':output}
    lines=['// Generated from preserved id Software Linux DOOM 1.10; GPL-2.0-only.', '// Reproduce with python3 tools/reference/generate_m3a.py.', '#[rustfmt::skip]', 'static TAN_TO_ANGLE: [u32; 2049] = [']
    lines += ['    '+', '.join(map(str,angles[i:i+12]))+',' for i in range(0,len(angles),12)]
    generated['crates/doom-sim/src/tan_to_angle.rs']='\n'.join(lines)+'\n];\n'
    if not check:
        names={'m_random.c','m_fixed.c','m_fixed.h','tables.h','p_local.h','p_mobj.h','d_player.h','r_defs.h'}|{f[0] for f in FUNCTIONS}
        p={'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','compiler':subprocess.check_output(['cc','--version'],text=True).splitlines()[0],'platform':platform.platform(),'flags':FLAGS,'source_sha256':{n:hashlib.sha256((SOURCE/n).read_bytes()).hexdigest() for n in sorted(names)},'adapter_sha256':{n:hashlib.sha256((ROOT/'tools/reference'/n).read_bytes()).hexdigest() for n in ['generate.py','generate_m3a.py','m3a_shim.h']},'assumptions':['one flat sector; one-sided clockwise room, optional angled bevel; source BLOCKMAP head links','zero/MF_SOLID flags; live grounded players, no cheats/reactiontime/buttons; state 7 avoids animation transitions','source bodies unchanged except explicit unsigned negative angleturn shift','enlarged intercept storage; cases stay within original 128-intercept capacity','abs(INT_MIN), overflowing position differences and unsupported gameplay effects excluded; GNU signed shifts/narrowing profile','18 explicit setups, each 12 tics; action case17: blocker momentum at tic2, unlink at tic3, replacement at tic4; inputs and results in CSV; no full engine or private assets']}
        generated['tests/reference/m3a_provenance.json']=json.dumps(p,indent=2)+'\n'
    for path,value in generated.items():
        dest=ROOT/path
        if check:
            if dest.read_text()!=value:raise SystemExit('Reference mismatch: '+path)
        else:dest.write_text(value)
    print(('Checked' if check else 'Generated')+' M3a reference vectors/tables (216 tics).')

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');run(parser.parse_args().check)
