#!/usr/bin/env python3
"""Compact compiled-original-C formula evidence; not full weapon/trace coverage."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/reference'))
from generate import body,FLAGS
PREFIX=r'''
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
typedef uint32_t angle_t;
typedef int boolean;
typedef struct {angle_t angle;int x,y;} mobj_t;
typedef struct {mobj_t* mo;int powers[1];} player_t;
typedef struct {int dummy;} pspdef_t;
#define pw_strength 0
#define MELEERANGE (64*65536)
#define MISSILERANGE (32*64*65536)
#define sfx_punch 0
static int draws[3],at,damage_result,bulletslope;
static angle_t angle_result;
static mobj_t *linetarget;
static int P_Random(void){if(at>=3)abort();return draws[at++];}
static int ordered_spread(void){int left=P_Random();int right=P_Random();return (left-right)*262144;}
static int P_AimLineAttack(mobj_t*m,angle_t a,int r){(void)m;(void)a;(void)r;return 0;}
static void P_LineAttack(mobj_t*m,angle_t a,int r,int s,int d){(void)m;(void)r;(void)s;angle_result=a;damage_result=d;}
static void S_StartSound(mobj_t*m,int s){(void)m;(void)s;abort();}
static angle_t R_PointToAngle2(int a,int b,int c,int d){(void)a;(void)b;(void)c;(void)d;abort();}
'''
MAIN=r'''
static uint64_t hash=14695981039346656037ull;
static void word(uint32_t v){unsigned k;for(k=0;k<4;k++){hash^=(v>>(8*k))&255;hash*=1099511628211ull;}}
int main(void){mobj_t m={0,0,0};player_t p={&m,{0}};pspdef_t ps={0};int i,j,k,d;
puts("rng,bullet,punch");
for(i=0;i<256;i++){draws[0]=i;draws[1]=draws[2]=0;at=0;P_GunShot(&m,1);if(at!=1)abort();d=damage_result;at=0;A_Punch(&p,&ps);if(at!=3)abort();printf("%d,%d,%d\n",i,d,damage_result);}
for(k=0;k<2;k++){hash=14695981039346656037ull;for(d=0;d<3;d++){m.angle=d==0?0:d==1?0x80000000u:0xffffffffu;for(i=0;i<256;i++)for(j=0;j<256;j++){draws[0]=123;draws[1]=i;draws[2]=j;at=0;if(k)A_Punch(&p,&ps);else P_GunShot(&m,0);if(at!=3)abort();word(angle_result);word((uint32_t)damage_result);word((uint32_t)at);}}printf("%s,%016llx\n",k?"punch_spread":"bullet_spread",(unsigned long long)hash);}
return 0;}
'''
def observe(sanitize):
    bodies=[body('p_pspr.c',name,'void')for name in ['P_GunShot','A_Punch']]
    adapted=[b.replace('(P_Random()-P_Random())<<18','ordered_spread()')for b in bodies]
    assert all('ordered_spread()'in b for b in adapted)
    with tempfile.TemporaryDirectory(prefix='doom-weapon-formulas-')as directory:
        src=Path(directory)/'observe.c';binary=Path(directory)/'observe';src.write_text(PREFIX+'\n'.join(adapted)+MAIN)
        flags=[*FLAGS,'-Wno-unused-parameter']
        if sanitize: flags+=['-fsanitize=address,undefined','-fno-sanitize-recover=all','-fno-omit-frame-pointer']
        subprocess.run(['cc',*flags,str(src),'-o',str(binary)],check=True)
        observed=subprocess.check_output([str(binary)])
    return observed,bodies

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--sanitize',action='store_true');a=parser.parse_args()
    observed,bodies=observe(a.sanitize)
    path=ROOT/'tests/reference/weapon_formula_observations.csv'
    meta=ROOT/'tests/reference/weapon_formula_contract.json'
    contract=dict(schema='DOOM-WEAPON-FORMULA-v1',source_revision='a77dfb96cb91780ca334d0d4cfd86957558007e0',flags=FLAGS,observer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),source_sha256=hashlib.sha256((ROOT/'linuxdoom-1.10/p_pspr.c').read_bytes()).hexdigest(),function_sha256={name:hashlib.sha256(b.encode()).hexdigest()for name,b in zip(['P_GunShot','A_Punch'],bodies)},observations_sha256=hashlib.sha256(observed).hexdigest(),damage_inputs=256,spread_pairs_per_body=65536,starting_angles=[0,2147483648,4294967295],function_entries_per_body=196864,globally_new_source_bodies=0,scope='Inherited full source bodies, unpowered punch, no hit target, stubbed aim/line/sound. Explicit left-then-right draws and representable multiplication replace undefined negative signed spread shifts; unsigned angle accumulation wraps. Component arithmetic/draw evidence only, not trace, impact, combat or branch coverage.')
    encoded=(json.dumps(contract,indent=2)+'\n').encode()
    if a.check: assert path.read_bytes()==observed and meta.read_bytes()==encoded
    else: path.write_bytes(observed);meta.write_bytes(encoded)
    print('Observed 256 damage inputs and 196608 spread cases per source body; 393728 total function entries')
if __name__=='__main__': main()
