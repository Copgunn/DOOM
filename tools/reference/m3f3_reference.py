#!/usr/bin/env python3
"""Combined larger-map gameplay from immutable original-C extraction services.
The historical writer runs only inside a temporary directory. No Rust is read.
"""
import argparse, copy, csv, hashlib, io, json, re, shutil, struct, subprocess, tempfile
from pathlib import Path
import generate_m3d3j as legacy
import m3f1_reference as geometry
import m3f2_reference as extent
ROOT=Path(__file__).resolve().parents[2]
MAPS=[]; THINGS=[]; KEYS=[]; CASES=[]

def case(label,n=256,count=None,things=None,tics=100,scale=1,sparse=False,heights=None,rejected=False,custom=None,**values):
    things=things if things is not None else [(-96,0,0,3002,7)]
    m=copy.deepcopy(custom) if custom is not None else (extent.scene(n=n,count=count,heights=heights,sx=scale) if scale!=1 else
       geometry.fixture(n=n,count=count,heights=heights,sparse=sparse,reverse=True,duplicate=True))
    if rejected:m['lumps']['REJECT']=bytes([255])*len(m['lumps']['REJECT'])
    data=b''.join(struct.pack('<5h',*thing) for thing in things)
    m['lumps']['THINGS']=data;m['bytes']=geometry.old.wad(list(m['lumps'].items()))
    key=hashlib.sha256(m['bytes']).hexdigest()
    if key in KEYS:index=KEYS.index(key)
    else:index=len(MAPS);KEYS.append(key);MAPS.append(m);THINGS.append(data)
    c=copy.deepcopy(legacy.CASES[1]);c.update(label=label,map=index,tics=tics,damage_tick=-1,kill_tick=-1,
        sound_tick=-1,retire_tick=-1,remove_corpse=-1,hitscan=0,continue_rng=0,seed=0,
        source_kind=1,inflictor_kind=0,kill_all=0)
    c.update(values);CASES.append(c)

case('long_lifecycle',damage_tick=45,kill_tick=75,tics=160)
case('multiple_lifecycle',things=[(-96,-32,0,3002,7),(-96,32,0,3002,7)],damage_tick=45,kill_tick=75,kill_all=1,tics=140)
case('four_owners',things=[(-96,-40,0,3002,7),(-96,40,0,3002,7),(-32,-40,0,3002,7),(-32,40,0,3002,7)],tics=80)
case('old_sector_boundary',n=128,tics=65)
case('new_sector_boundary',n=129,tics=65)
case('new_line_boundary',n=1,count=129,tics=65)
case('line_table',n=1,count=1024,tics=65)
case('sparse_lists',sparse=True)
case('floor_negative',heights=[(-24,104)]*256)
case('floor_positive',heights=[(24,152)]*256)
for floor in [24,25,-24,-25]:case('step_'+str(floor),heights=[(0,128)]*128+[(floor,128)]*128)
case('nomonsters',no_monsters=1,tics=4)
for skill in range(4):case('skill_'+str(skill),skill=skill,things=[(-96,-32,0,3002,1),(-96,32,0,3002,4)],tics=25)
case('ambush_sound',things=[(-96,0,0,3002,15)],sound_tick=2,p0health=0,tics=85)
case('reject_matrix',rejected=True,tics=25)
case('rng_wrap',skip=253,tics=85)
case('shared_target_death',things=[(-32,-32,0,3002,7),(32,32,-180,3002,7)],enter_all=5,body_health=1,p0x=0,tics=70)
case('corpse_source_reuse',damage_tick=1,damage=150,retire_tick=40,tics=75)
case('wide_map_lifecycle',scale=16,things=[(-900,0,0,3002,7)],p0x=900*65536,damage_tick=425,kill_tick=455,tics=560)
for angle in [-224,-180,-135,-90,-45,45,90,135,179]:case('angle_'+str(angle),things=[(-96,0,angle,3002,7)],tics=0)

case('expanded_sight_dag',custom=extent.dag(12),tics=20)

case('self_damage',source_kind=2,inflictor_kind=2,damage_tick=1,kill_tick=3,tics=65)
case('body_inflictor',source_kind=1,inflictor_kind=1,damage_tick=20,kill_tick=45,tics=100)
case('native_lethal_credit',things=[(-96,-32,0,3002,7),(-96,32,0,3002,7)],source_kind=3,inflictor_kind=3,kill_tick=2,tics=85)

case('small_control',n=1,tics=80)

_ORIGINAL_LOADER=legacy.combat.c_maps

def loader():
    text=_ORIGINAL_LOADER()
    pattern=r'(case (\d+):\{static short blocks\[\]=\{[^}]+\};blockmaplump=blocks;blockmap=blocks\+4;)bmapwidth=4;bmapheight=2;bmaporgx=-256\*FRACUNIT;bmaporgy=-128\*FRACUNIT;'
    def replace(match):
        ox,oy,w,h=MAPS[int(match[2])]['blocks'][:4]
        return match[1]+f'bmapwidth={w};bmapheight={h};bmaporgx={ox}*FRACUNIT;bmaporgy={oy}*FRACUNIT;'
    text,count=re.subn(pattern,replace,text);assert count==len(MAPS)
    return text

def extracted(flags):
    """Execute unchanged genuine-body extraction, with explicit new data/storage."""
    with tempfile.TemporaryDirectory(prefix='doom-m3f3-writer-') as directory:
        root=Path(directory);(root/'tools/reference').mkdir(parents=True);(root/'tests/reference').mkdir(parents=True)
        for p in (ROOT/'tools/reference').glob('generate*.py'):shutil.copyfile(p,root/'tools/reference'/p.name)
        shutil.copyfile(ROOT/'tools/reference/m3f3_shim.h',root/'tools/reference/m3d3j_shim.h')
        legacy.ROOT=root;legacy.MAPS=MAPS;legacy.CASES=CASES;legacy.THING_INPUTS=THINGS
        legacy.GEOMETRIES=[(i,()) for i in range(len(MAPS))];legacy.FLAGS=flags
        legacy.combat.c_maps=loader
        with io.StringIO() as log:
            import contextlib
            with contextlib.redirect_stdout(log):legacy.run(False)
        return {p.name:p.read_bytes() for p in (root/'tests/reference').iterdir()}

def probe_undefined_angle():
    # Exact original expression component, not a fabricated full spawn result.
    body=legacy.base.body('p_mobj.c','P_SpawnMapThing','void')
    statement=re.search(r'mobj->angle\s*=\s*ANG45 \* \(mthing->angle/45\);',body).group(0)
    original=(legacy.SOURCE/'p_mobj.c').read_text()
    line=original[:original.index(statement)].count('\n')+1
    code='#include <stdio.h>\n#include "doomtype.h"\n#include "m_fixed.h"\n#include "tables.h"\n#include "doomdata.h"\n'
    code+='int main(void){struct {angle_t angle;} object,*mobj=&object;mapthing_t thing,*mthing=&thing;volatile short input=180;mthing->angle=input;\n'
    code+=f'#line {line} "linuxdoom-1.10/p_mobj.c"\n'+statement+'\nprintf("%u\\n",mobj->angle);return 0;}\n'
    with tempfile.TemporaryDirectory(prefix='doom-m3f3-angle-') as directory:
        source=Path(directory)/'angle.c';binary=Path(directory)/'angle';source.write_text(code)
        subprocess.run(['cc','-std=gnu89','-O0','-fno-wrapv','-fsanitize=signed-integer-overflow','-fno-sanitize-recover=all','-I'+str(legacy.SOURCE),str(source),'-o',str(binary)],check=True)
        result=subprocess.run([str(binary)],capture_output=True,text=True)
        assert result.returncode!=0 and 'signed integer overflow' in result.stderr,result
        print('Expected original signed spawn-angle failure at 180 degrees: '+result.stderr.strip())

def run(check,overflow):
    flags=list(legacy.base.FLAGS)
    outputs=extracted(flags)
    if overflow:
        audited=extracted([f for f in flags if f!='-fwrapv']+['-fno-wrapv',
            '-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow','-fno-sanitize-recover=all'])
        assert {p:b for p,b in outputs.items() if p.endswith(('.bin','.csv'))}=={p:b for p,b in audited.items() if p.endswith(('.bin','.csv'))}
    manifest=json.loads(outputs.pop('m3d3j_provenance.json'))
    manifest['assumptions']=[('Negative fixed-coordinate spawn shifts use representable multiplication. The inherited A_Chase direction shift uses an explicit angle_t cast under the recorded BAM ABI policy; shadow/spread/jitter draws use explicit left-before-right order. These adapters and GNU signed-shift assumptions are not portable unmodified ISO-C or exhaustive shift-UB coverage.' if line.startswith('negative-coordinate signed shifts') else line) for line in manifest['assumptions']]
    manifest['adapter_sha256']['m3f3_shim.h']=manifest['adapter_sha256'].pop('m3d3j_shim.h')
    for name in ['m3f3_reference.py','m3f1_reference.py','m3f2_reference.py']:
        manifest['adapter_sha256'][name]=hashlib.sha256((ROOT/'tools/reference'/name).read_bytes()).hexdigest()
    published={'tests/reference/'+name.replace('m3d3j','m3f3'):data for name,data in outputs.items()}
    published['tests/reference/m3f3_maps.rs']=('// Generated public synthetic inputs. GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("m3f3_map_{i}.bin"),\n' for i in range(len(MAPS)))+'];\n').encode()
    manifest['outputs_sha256']={p:hashlib.sha256(b).hexdigest() for p,b in published.items()}
    manifest['assumptions'] += [
        'Immutable historical writer runs only in a temporary directory; published outputs are new m3f3 paths. No historical helper/fixture bytes are modified.',
        'New synthetic table capacities and actual BLOCKMAP origin/dimensions; named-field geometry plus sector-line/seg adapters, not complete original level loading.',
        'New gameplay retains 978-owner/994-target pre-tic XY and 1024 readonly bounds, despite geometry reaching 2048; sector heights remain512.',
        'Selected THING angle quotient[-4,3] avoids original signed ANG45 product overflow; legacy positive180 fails strict signed-overflow probe and is excluded, not replaced by an unsigned C rewrite.',
        'Strict overflow mode removes fwrapv and audits signed-overflow/division/float-cast on admitted inputs; source signed shifts/GNU ABI remain explicit, not portable ISO C or exhaustive UB coverage.',
        'Function inventory is70 genuine bodies plus two original setup fragments; not function/branch coverage. All supplied maps are actually exercised.']
    path=ROOT/'tests/reference/m3f3_provenance.json'
    if check:
        old=json.loads(path.read_text())
        for key,value in manifest.items():
            if key not in ['compiler','platform']:assert old[key]==value,key
    else:published['tests/reference/m3f3_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
    for name,data in published.items():
        p=ROOT/name
        if check:assert p.read_bytes()==data,name
        else:p.write_bytes(data)
    print(('Checked' if check else 'Generated')+f' M3f3: {len(CASES)} scripts, {len(MAPS)} exercised maps, {sum(c["tics"] for c in CASES)} tics; overflow audit={overflow}.')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');p.add_argument('--overflow-check',action='store_true');p.add_argument('--probe-undefined-angle',action='store_true')
    a=p.parse_args();run(a.check,a.overflow_check)
    if a.probe_undefined_angle:probe_undefined_angle()
