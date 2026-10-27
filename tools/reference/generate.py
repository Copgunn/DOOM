#!/usr/bin/env python3
"""Reproduce isolated Linux DOOM vectors; never builds/runs the full engine."""
from pathlib import Path
import argparse
import hashlib
import json
import platform
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "linuxdoom-1.10"
FLAGS = ["-std=gnu89", "-O0", "-fwrapv", "-fno-strict-aliasing", "-ffp-contract=off", "-fexcess-precision=standard", "-fsigned-char"]


def body(file, name, result, rename=None):
    text = (SOURCE / file).read_text()
    match = re.search(r"\b" + name + r"\s*\([^;{}]*?\)\s*\{", text)
    if not match:
        raise ValueError(name)
    start, level, end = match.start(), 1, match.end()
    while level:
        level += (text[end] == "{") - (text[end] == "}")
        end += 1
    return result + " " + text[start:end].replace(name, rename or name, 1) + "\n"


def table(file, pattern):
    match = re.search(pattern + r"\s*=\s*\{(.*?)\}", (SOURCE / file).read_text(), re.S)
    return [int(n) for n in re.findall(r"-?\d+", match.group(1))]


MAIN = r'''
typedef char int_is_32[(sizeof(int)==4)?1:-1];
typedef char short_is_16[(sizeof(short)==2)?1:-1];
typedef char byte_is_8[(CHAR_BIT==8)?1:-1];
typedef char long_long_is_64[(sizeof(long long)==8)?1:-1];
typedef char arithmetic_shift[((-2>>1)==-1)?1:-1];
typedef char binary64[(DBL_MANT_DIG==53 && FLT_RADIX==2)?1:-1];
void fixed_pair(int a,int b) {
    printf("M,%d,%d,%d\n",a,b,FixedMul(a,b));
    if(a!=INT_MIN && b!=INT_MIN) printf("D,%d,%d,%d\n",a,b,FixedDiv(a,b));
    if(b) {
        printf("H,%d,%d,",a,b);
        if(!setjmp(div_failure)) printf("%d\n",FixedDiv2(a,b)); else puts("ERR");
    }
}
void fixed_vectors(void) {
    int v[]={INT_MIN,INT_MIN+1,-1073741824,-1000000000,-65537,-65536,-32769,-1,0,1,32767,65535,65536,65537,1000000000,1073741824,INT_MAX-1,INT_MAX};
    unsigned i,j,seed=1; int a,b;
    puts("op,a,b,result");
    for(i=0;i<sizeof(v)/sizeof(v[0]);i++) for(j=0;j<sizeof(v)/sizeof(v[0]);j++) fixed_pair(v[i],v[j]);
    for(i=0;i<128;i++) { seed=seed*1664525u+1013904223u; memcpy(&a,&seed,4); seed=seed*1664525u+1013904223u; memcpy(&b,&seed,4); fixed_pair(a,b); }
}
void init_line(line_t* l,vertex_t* a,vertex_t* b) {
    l->v1=a;l->v2=b;l->dx=b->x-a->x;l->dy=b->y-a->y;
    l->slopetype=!l->dx?ST_VERTICAL:!l->dy?ST_HORIZONTAL:FixedDiv(l->dy,l->dx)>0?ST_POSITIVE:ST_NEGATIVE;
    l->bbox[BOXTOP]=a->y>b->y?a->y:b->y; l->bbox[BOXBOTTOM]=a->y<b->y?a->y:b->y;
    l->bbox[BOXLEFT]=a->x<b->x?a->x:b->x;l->bbox[BOXRIGHT]=a->x>b->x?a->x:b->x;
    l->frontsector=&sector;l->backsector=NULL;l->flags=1;l->special=0;l->validcount=0;
}
void geometry_vectors(void) {
    int lv[][4]={{-64,0,64,0},{64,0,-64,0},{0,-64,0,64},{0,64,0,-64},{-64,-64,64,64},{-64,64,64,-64}};
    fixed_t points[][2]={{0,0},{1,1},{-1,1},{0,-1},{8*FRACUNIT,8*FRACUNIT},{-8*FRACUNIT,8*FRACUNIT},{64*FRACUNIT,64*FRACUNIT},{-64*FRACUNIT,-64*FRACUNIT},{2*FRACUNIT,-3*FRACUNIT}};
    fixed_t radii[]={0,1,8*FRACUNIT}; unsigned i,j,k; line_t l; vertex_t a,b; fixed_t box[4],x,y,r;
    puts("x1,y1,x2,y2,x,y,radius,point_side,box_side");
    for(i=0;i<6;i++) { a.x=lv[i][0]*FRACUNIT;a.y=lv[i][1]*FRACUNIT;b.x=lv[i][2]*FRACUNIT;b.y=lv[i][3]*FRACUNIT;init_line(&l,&a,&b);
        for(j=0;j<9;j++) for(k=0;k<3;k++) { x=points[j][0];y=points[j][1];r=radii[k];box[BOXTOP]=y+r;box[BOXBOTTOM]=y-r;box[BOXLEFT]=x-r;box[BOXRIGHT]=x+r;
            printf("%d,%d,%d,%d,%d,%d,%d,%d,%d\n",a.x,a.y,b.x,b.y,x,y,r,P_PointOnLineSide(x,y,&l),P_BoxOnLineSide(box,&l)); }
    }
}
void movement_vectors(void) {
    fixed_t cases[][4]={{0,0,FRACUNIT,0},{55*FRACUNIT,0,FRACUNIT,0},{0,0,30*FRACUNIT,0},{40*FRACUNIT,0,-30*FRACUNIT,0},{0,0,30*FRACUNIT,-29*FRACUNIT-1},{0,0,4095,-4095},{0,0,4096,-4096},{0,0,INT_MAX,INT_MIN}};
    int xy[][2]={{-64,-64},{64,-64},{64,64},{-64,64}}; unsigned i,c,t,j; mobj_t m;
    for(i=0;i<4;i++){ vertices[i].x=xy[i][0]*FRACUNIT;vertices[i].y=xy[i][1]*FRACUNIT; }
    for(i=0;i<4;i++) init_line(&lines[i],&vertices[i],&vertices[(i+1)%4]);
    puts("case,tic,start_x,start_y,input_momx,input_momy,x,y,momx,momy,attempts,candidates");
    for(c=0;c<8;c++) { memset(&m,0,sizeof(m));m.x=cases[c][0];m.y=cases[c][1];m.momx=cases[c][2];m.momy=cases[c][3];m.radius=8*FRACUNIT;m.height=56*FRACUNIT;m.z=m.floorz=sector.floorheight;m.ceilingz=sector.ceilingheight;m.subsector=&subsector;
        for(t=0;t<12;t++){ fixed_t x=m.x,y=m.y,mx=m.momx,my=m.momy;attempts=candidate_count=0;P_XYMovement(&m);
            printf("%u,%u,%d,%d,%d,%d,%d,%d,%d,%d,%d,",c,t,x,y,mx,my,m.x,m.y,m.momx,m.momy,attempts);
            if(!candidate_count) putchar('-'); for(j=0;j<(unsigned)candidate_count;j++)printf("%s%d",j?":":"",candidates[j]);putchar('\n'); }
    }
}
void thrust_vectors(void) {
    unsigned angles[]={0,0x20000000u,0x3fffffffu,0xffffffffu};short turns[]={-32768,-8192,-1,0,1,8192,32767};int cmds[][2]={{25,0},{0,24},{-50,-40},{127,-128}};unsigned a,t,c;player_t p;mobj_t m;
    puts("angle,turn,forward,side,initial_momx,initial_momy,result_angle,momx,momy");
    for(a=0;a<4;a++)for(t=0;t<7;t++)for(c=0;c<4;c++) { memset(&m,0,sizeof(m));memset(&p,0,sizeof(p));p.mo=&m;p.cmd.angleturn=turns[t];p.cmd.forwardmove=cmds[c][0];p.cmd.sidemove=cmds[c][1];m.angle=angles[a];m.momx=-1000;m.momy=1234;m.state=&states[7];P_MovePlayer(&p);
        printf("%u,%d,%d,%d,-1000,1234,%u,%d,%d\n",angles[a],turns[t],cmds[c][0],cmds[c][1],m.angle,m.momx,m.momy); }
}
typedef struct { thinker_t thinker; int id; } test_thinker;
test_thinker pool[8]; int scenario,phase;
void think(void* ptr) {
    test_thinker* t=ptr;printf(" %d",t->id);
    if(!phase && scenario==0 && t->id==1) { P_RemoveThinker(&pool[1].thinker);P_RemoveThinker(&t->thinker);P_AddThinker(&pool[3].thinker); }
    if(!phase && scenario==0 && t->id==3) P_AddThinker(&pool[4].thinker);
    if(!phase && scenario==1 && t->id<3) P_AddThinker(&pool[t->id].thinker);
    if(!phase && scenario==2 && t->id==2) P_RemoveThinker(&pool[0].thinker);
    if(!phase && scenario==3) P_RemoveThinker(&t->thinker);
}
void thinker_vectors(void) {
    int i,n;const char* labels[]={"remove_self_other_spawn","spawn_by_tail","remove_visited","remove_current","empty"};
    for(scenario=0;scenario<5;scenario++) { P_InitThinkers();for(i=0;i<8;i++){memset(&pool[i],0,sizeof(pool[i]));pool[i].id=i+1;pool[i].thinker.function.acp1=think;}
        n=scenario==0?3:scenario==2?2:scenario==4?0:1;for(i=0;i<n;i++)P_AddThinker(&pool[i].thinker);
        for(phase=0;phase<2;phase++){printf("%s,%d:",labels[scenario],phase);P_RunThinkers();putchar('\n');} }
}
int main(int argc,char** argv) {
    int i;extern int rndindex,prndindex;if(argc!=2)return 2;
    if(!strcmp(argv[1],"fixed")) fixed_vectors();
    else if(!strcmp(argv[1],"rng")){puts("lookup,p_index,m_index,p_value,m_value");M_ClearRandom();for(i=0;i<512;i++){int p=P_Random(),m=M_Random();printf("%d,%d,%d,%d,%d\n",i,prndindex,rndindex,p,m);} }
    else if(!strcmp(argv[1],"geometry")) geometry_vectors();
    else if(!strcmp(argv[1],"movement")) movement_vectors();
    else if(!strcmp(argv[1],"ordered_movement")) { static short data[]={-128,-128,2,2,8,12,16,20,0,0,3,-1,0,0,1,-1,0,2,3,-1,0,1,2,-1};blockmaplump=data;blockmap=data+4;movement_vectors(); }
    else if(!strcmp(argv[1],"thrust")) thrust_vectors();
    else if(!strcmp(argv[1],"thinkers")) thinker_vectors();
    else return 2;return 0;
}
'''


def run(check):
    if check:
        provenance=json.loads((ROOT/"tests/reference/provenance.json").read_text())
        assert provenance["flags"]==FLAGS,"reference compiler flags changed"
        for name,digest in provenance["source_sha256"].items():
            assert hashlib.sha256((SOURCE/name).read_bytes()).hexdigest()==digest,name
        for name,digest in provenance["adapter_sha256"].items():
            assert hashlib.sha256((ROOT/"tools/reference"/name).read_bytes()).hexdigest()==digest,name
    sine = table("tables.c", r"int finesine\[10240\]")
    rng = table("m_random.c", r"unsigned char rndtable\[256\]")
    assert len(sine) == 10240 and len(rng) == 256
    generated = {}
    for path, values, declaration in [
        ("crates/doom-sim/src/fine_sine.rs", sine, "static FINE_SINE: [i32; 10240]"),
        ("crates/doom-types/src/rng_table.rs", rng, "const TABLE: [u8; 256]"),
    ]:
        lines = ["// Generated from preserved id Software Linux DOOM 1.10; GPL-2.0-only.", "// Reproduce with python3 tools/reference/generate.py.", "#[rustfmt::skip]", declaration + " = ["]
        lines += ["    " + ", ".join(map(str, values[i:i+12])) + "," for i in range(0, len(values), 12)]
        generated[path] = "\n".join(lines) + "\n];\n"
    fragments = ['#include "shim.h"\n', 'int finesine[10240]={' + ','.join(map(str, sine)) + '};\nfixed_t* finecosine=finesine+2048;\n']
    functions = [
        ("p_tick.c", "P_InitThinkers", "void", None), ("p_tick.c", "P_AddThinker", "void", None),
        ("p_tick.c", "P_RemoveThinker", "void", None), ("p_tick.c", "P_RunThinkers", "void", None),
        ("p_maputl.c", "P_PointOnLineSide", "int", None), ("p_maputl.c", "P_BoxOnLineSide", "int", None),
        ("p_maputl.c", "P_BlockLinesIterator", "boolean", None),
        ("p_map.c", "PIT_CheckLine", "boolean", "Ref_CheckLine"), ("p_map.c", "P_CheckPosition", "boolean", None),
        ("p_map.c", "P_TryMove", "boolean", "Ref_TryMove"), ("p_mobj.c", "P_XYMovement", "void", None),
        ("p_user.c", "P_Thrust", "void", None), ("p_user.c", "P_MovePlayer", "void", None),
    ]
    for function in functions:
        fragment = body(*function)
        if function[1] == "P_MovePlayer":
            # Define the otherwise C-undefined negative angleturn left shift.
            fragment = fragment.replace("(cmd->angleturn<<16)", "((uint32_t)(int32_t)cmd->angleturn << 16)")
        fragments.append(fragment)
    fragments.append(MAIN)
    with tempfile.TemporaryDirectory(prefix="doom-reference-") as tmp:
        source = Path(tmp)/"reference.c"
        source.write_text("\n".join(fragments))
        binary = Path(tmp)/"reference"
        command = ["cc", *FLAGS, "-I"+str(SOURCE), "-I"+str(ROOT/"tools/reference"), str(source), str(SOURCE/"m_fixed.c"), str(SOURCE/"m_random.c"), "-o", str(binary)]
        subprocess.run(command, check=True)
        for mode in ["fixed", "rng", "geometry", "movement", "ordered_movement", "thrust", "thinkers"]:
            suffix = "txt" if mode=="thinkers" else "csv"
            generated[f"tests/reference/{mode}.{suffix}"] = subprocess.check_output([str(binary),mode],text=True)
    if not check:
        provenance = {"revision":"a77dfb96cb91780ca334d0d4cfd86957558007e0", "compiler":subprocess.check_output(["cc","--version"],text=True).splitlines()[0], "platform":platform.platform(), "machine":platform.machine(), "flags":FLAGS, "source_sha256":{n:hashlib.sha256((SOURCE/n).read_bytes()).hexdigest() for n in sorted({"m_fixed.c","m_fixed.h","m_random.c","tables.c"}|{f[0] for f in functions})}}
        provenance["adapters"] = ["Z_Free retains storage until traversal completes", "negative angleturn shift uses explicit uint32_t wrap", "single static sector, no other actors, unsupported gameplay paths abort", "FixedDiv excludes abs(INT_MIN); direct FixedDiv2 excludes divisor zero", "GNU arithmetic right shift and low-bit signed narrowing are compiler-profile assumptions"]
        provenance["adapter_sha256"] = {n:hashlib.sha256((ROOT/"tools/reference"/n).read_bytes()).hexdigest() for n in ["generate.py","shim.h"]}
        generated["tests/reference/provenance.json"] = json.dumps(provenance,indent=2)+"\n"
    for path, value in generated.items():
        target = ROOT/path
        if check:
            if target.read_text()!=value:
                raise SystemExit(f"Reference mismatch: {path}")
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(value)
    print(f"{'Checked' if check else 'Generated'} {len(generated)} reference/table files using preserved C bodies.")
    print(subprocess.check_output(["cc","--version"],text=True).splitlines()[0])


if __name__ == "__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--check",action="store_true",help="compare vectors/tables without replacing provenance")
    run(parser.parse_args().check)
