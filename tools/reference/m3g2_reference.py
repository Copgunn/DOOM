#!/usr/bin/env python3
"""Genuine MT_PLAYER body observations; no Rust-derived expected data.

Reuse the frozen source extractor, augment only declared setup/service observers.
Old tools, original source, metadata and fixtures are never rewritten.
"""
import argparse
import collections
import csv
import hashlib
import io
import json
import re
import shutil
import tempfile
from pathlib import Path
import m3f5_reference as source

ROOT = Path(__file__).resolve().parents[2]
main = source.main
legacy = main.legacy
SCHEMA = 'DOOM-PLAYER-BODY-v1'
CASES = [
    ('initial', 10, 0), ('environment_pain', 16, 0),
    ('live_source_retired', 24, 0), ('corpse_source', 72, 0),
    ('nonpain255', 6, 0), ('replace_clear_self_zero', 16, 0),
    ('environment_death', 72, 0), ('normal_minus50', 72, 0),
    ('high_scream_minus51', 72, 0), ('normal_minus100', 72, 0),
    ('extreme_minus101', 52, 0), ('self_death', 72, 0),
    ('attacker_slot_reuse', 24, 0), ('body_retirement_reuse', 5, 0),
    ('nonpain255_retarget', 20, 0),
]
# The next pain lookup after the two Demon map-spawn draws and player lastlook.
rng = legacy.base.table('m_random.c', r'unsigned char\s+rndtable\[256\]')
CASES[4] = ('nonpain255', 6, (rng.index(255) - 4) % 256)
CASES[14] = ('nonpain255_retarget', 20, (rng.index(255) - 4) % 256)
main.MAPS, main.THINGS, main.KEYS, main.CASES = [], [], [], []
for label, tics, skip in CASES:
    main.case(label, n=1, tics=tics, skip=skip)
assert main.MAPS[0]['bytes'] == (ROOT / 'tests/reference/m3f5_map_0.bin').read_bytes()

# Exact association statements from P_SpawnPlayer; source spawn coordinates and
# angle are explicit fixture inputs. HUD/psprites/reborn and view fields excluded.
spawn = (legacy.SOURCE / 'p_mobj.c').read_text()
start = spawn.index('    mobj->player = p;')
end = spawn.index('    p->viewheight = VIEWHEIGHT;', start)
ASSOCIATION = spawn[start:end]

# Reuse source capture format for the existing independent actor/event encoder.
old_capture = legacy.MAIN[legacy.MAIN.index('static void capture(void)'):legacy.MAIN.index('static void definitions(void)')]
old_capture = old_capture.replace('for(i=0;i<4;i++)printf(', 'for(i=0;i<4;i++)printf(')
# Additional source player and reverse association observations; ident compares
# opaque quarantined pointers and never dereferences a retired mo/attacker.
extra_capture = r'''
printf("P,%d,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,
players[0].mo?ident(players[0].mo):-1,players[0].health,players[0].playerstate,
players[0].attacker?ident(players[0].attacker):-1,players[0].damagecount,players[0].frags[0]);
for(i=0;i<owned_count;i++)if(alive[i])printf("AP,%d,%d,%d,%d\n",scenario,operation,i,owned[i]->player? (int)(owned[i]->player-players):-1);
'''
old_capture = old_capture.rstrip()[:-1] + extra_capture + '}\n'
reset = legacy.MAIN[legacy.MAIN.index('static void reset('):legacy.MAIN.index('static int components(')]
# Leave every original definition pristine, including decoration135/136.
components = legacy.MAIN[legacy.MAIN.index('static int components('):legacy.MAIN.index('static mobj_t* damage_source(')]
MAIN = old_capture + 'static void definitions(void){}\n' + reset + components + r'''
static void associate(mobj_t* mobj){player_t* p=&players[0];
''' + ASSOCIATION + r'''
printf("PL,%d,%d,0,%d\n",scenario,operation,ident(mobj));}
int main(int argc,char** argv){int c,i,t;int seeds[CASE_COUNT];
if(argc<2)return 2;
if(!strcmp(argv[1],"setup")){reset(0);prndindex=211;rndindex=87;setup_statistics();setup_leveltime();printf("SETUP,%d,%d,%d,%d,%d,%d,%d,%d",totalkills,totalitems,totalsecret,wminfo.maxfrags,wminfo.partime,leveltime,prndindex,rndindex);for(i=0;i<4;i++)printf(",%d,%d,%d",players[i].killcount,players[i].itemcount,players[i].secretcount);printf("\n");return 0;}
if(!strcmp(argv[1],"components")){
int result=components();credit_components();return result;}
if(!strcmp(argv[1],"spawn")){FILE* f;if(argc!=3)return 3;f=fopen(argv[2],"r");if(!f)return 4;for(i=0;i<CASE_COUNT;i++)if(fscanf(f,"%d",&seeds[i])!=1)return 5;fclose(f);}
for(c=0;c<CASE_COUNT;c++){int* v=input[c];mobj_t* body;
reset(c);printf("SEED,%d,0,0\n",c);admit_native=0;
for(i=0;i<v[V_SKIP];i++){int draw=P_Random();observe(9,-1,draw);}
P_LoadThings(0);
printf("F,%d,0,-1,0,0,0,0,0,%d,%d,%d,%d\n",scenario,owned_count,prndindex,totalkills,totalitems);
players[0].health=100;players[0].readyweapon=wp_pistol;
body=P_SpawnMobj(96*FRACUNIT,0,ONFLOORZ,MT_PLAYER);associate(body);
/* Unlisted player body: no P_PlayerThink or automatic roster admission. */
if(c==3){P_DamageMobj(owned[0],NULL,body,150);}
else {
if(c==2||c==12||c==14)P_DamageMobj(body,NULL,owned[0],1);
if(c>=7&&c<=10)P_DamageMobj(body,NULL,owned[0],c==7?150:c==8?151:c==9?200:201);
P_RemoveMobj(owned[0]);
}
capture();
for(t=1;t<=v[V_TICS];t++){operation=t;query_index=-1;attempt=-1;
if(t==1&&(c==1||c==4))P_DamageMobj(body,NULL,NULL,1);
if(t==1&&c==3)P_DamageMobj(body,NULL,owned[0],1);
if(c==5){if(t==1)P_DamageMobj(body,NULL,body,1);if(t==2)P_DamageMobj(body,NULL,NULL,0);if(t==3)P_DamageMobj(body,NULL,NULL,10);}
if(c==6&&t==1)P_DamageMobj(body,NULL,NULL,100);
if(c==11&&t==1)P_DamageMobj(body,NULL,body,100);
if(c>=6&&c<=11&&t==2)P_DamageMobj(body,NULL,NULL,1);
if(c==12&&t==2)P_SpawnMobj(-96*FRACUNIT,0,ONFLOORZ,MT_PUFF);
if(c==13&&t==2)P_RemoveMobj(body);
if(c==13&&t==3){P_SpawnMobj(-96*FRACUNIT,0,ONFLOORZ,MT_PUFF);P_SpawnMobj(96*FRACUNIT,0,ONFLOORZ,MT_PUFF);}
P_RunThinkers();leveltime++;capture();
}}
return 0;}
'''
legacy.MAIN = MAIN
legacy.FUNCTIONS = [*legacy.FUNCTIONS, ('p_pspr.c', 'P_DropWeapon', 'void')]
_original_body = legacy.base.body

def observed_body(file, name, result, rename=None):
    text = _original_body(file, name, result, rename)
    # Function entry inventory is separate from source/action/event chronology.
    key = file + ':' + name
    text = text.replace('{', '{\n printf("FN,%d,%d,' + key + '\\n",scenario,operation);', 1)
    if name == 'P_KillMobj':
        text = text.replace('source->player->killcount++;', '{source->player->killcount++;printf("ST,%d,%d,1,%d,%d\\n",scenario,operation,ident(target),source->player->killcount);}')
    return text


def capture(flags):
    """New service context over the same frozen genuine-body extraction engine."""
    with tempfile.TemporaryDirectory(prefix='doom-player-observer-') as directory:
        root = Path(directory)
        (root / 'tools/reference').mkdir(parents=True)
        (root / 'tests/reference').mkdir(parents=True)
        for p in (ROOT / 'tools/reference').glob('generate*.py'):
            shutil.copyfile(p, root / 'tools/reference' / p.name)
        shim = (ROOT / 'tools/reference/m3f3_shim.h').read_text()
        shim = shim.replace('void P_DropWeapon(player_t* p){(void)p;abort();}', '')
        shim = shim.replace('void I_Tactile(int a,int b,int c){(void)a;(void)b;(void)c;abort();}', r'''void I_Tactile(int a,int b,int c){if(a!=40||b!=10)abort();printf("PL,%d,%d,1,%d,%d,%d,%d\n",scenario,operation,players[0].health,players[0].attacker?ident(players[0].attacker):-1,players[0].damagecount,c);}''')
        shim = shim.replace('/* SOURCE_BODIES */', r'''
#define rcsid player_weapon_rcsid
#include "d_items.c"
#undef rcsid
void P_SetPsprite(player_t* p,int slot,statenum_t state){if(p!=&players[0]||slot!=ps_weapon)abort();printf("PL,%d,%d,2,%d,%d\n",scenario,operation,p->frags[0],state);}
/* SOURCE_BODIES */''')
        (root / 'tools/reference/m3d3j_shim.h').write_text(shim)
        legacy.ROOT = root
        legacy.MAPS, legacy.CASES, legacy.THING_INPUTS = main.MAPS, main.CASES, main.THINGS
        legacy.GEOMETRIES = [(i, ()) for i in range(len(main.MAPS))]
        legacy.FLAGS = flags
        legacy.combat.c_maps = main.loader
        legacy.base.body = observed_body
        try:
            import contextlib
            with contextlib.redirect_stdout(io.StringIO()): legacy.run(False)
        finally:
            legacy.base.body = _original_body
        return {p.name:p.read_bytes() for p in (root / 'tests/reference').iterdir()}


def run(check, overflow, address):
    flags = list(legacy.base.FLAGS)
    output = capture(flags)
    raw = output['m3d3j_trace.csv']
    for enabled, extra in [(overflow,['-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow']),
                           (address,['-fsanitize=address,undefined'])]:
        if enabled:
            strict = capture([f for f in flags if f != '-fwrapv'] + ['-fno-wrapv', *extra, '-fno-sanitize-recover=all'])
            assert strict['m3d3j_trace.csv'] == raw
    rows = list(csv.reader(io.StringIO(raw.decode())))
    functions = collections.Counter(r[3] for r in rows if r[0] == 'FN')
    # Capture fields at meaningful source states/boundaries, retain all chronology.
    selected = {0,1,2,3,4,5,6,8,10,16,24,32,48,52,64,72}
    final = {i:tics for i,(_,tics,_) in enumerate(CASES)}
    keep = [r for r in rows if r[0] != 'FN' and (r[0] not in {'A','P','AP','R','S','LS'} or int(r[2]) in selected or int(r[2]) == final[int(r[1])])]
    text = io.StringIO(); writer = csv.writer(text,lineterminator='\n');writer.writerows(keep)
    trace = (SCHEMA+'\n'+text.getvalue()).encode()
    manifest = json.loads(output['m3d3j_provenance.json'])
    inherited = json.loads((ROOT/'tests/reference/m3f5_provenance.json').read_text())
    inventory = dict(new_cases=len(CASES), tics=sum(t for _,t,_ in CASES),
        captures=sum(r[0]=='P' for r in keep), actor_fields=26*sum(r[0]=='A' and r[4]=='1' for r in keep),
        player_fields=6*sum(r[0]=='P' for r in keep), association_fields=sum(r[0]=='AP' for r in keep),
        gameplay_draws=sum(r[0]=='E' and r[3]=='8' for r in keep),
        compiled_original_bodies=len(manifest['functions']), new_unique_bodies=1,
        exercised_unique_functions=len(functions), function_entries=sum(functions.values()))
    cases = io.StringIO(); cw = csv.writer(cases, lineterminator='\n')
    cw.writerow(['case','label','tics','skip'])
    cw.writerows((i,label,tics,skip) for i,(label,tics,skip) in enumerate(CASES))
    outputs={'tests/reference/m3g2_trace.csv':trace, 'tests/reference/m3g2_cases.csv':cases.getvalue().encode()}
    # Strip observation instrumentation before comparing inherited genuine bodies.
    hashes={f+':'+n:hashlib.sha256(_original_body(f,n,r).encode()).hexdigest() for f,n,r in manifest['functions']}
    for k,v in inherited['function_sha256'].items(): assert hashes[k] == v,k
    provenance=dict(schema=SCHEMA, revision=manifest['revision'], flags=flags, inventory=inventory,
        source_sha256={**manifest['source_sha256'],'d_items.c':hashlib.sha256((legacy.SOURCE/'d_items.c').read_bytes()).hexdigest()},
        functions=manifest['functions'], function_sha256=hashes, executed_function_entries=dict(sorted(functions.items())),
        association_fragment_sha256=hashlib.sha256(ASSOCIATION.encode()).hexdigest(),
        input_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in ['tools/reference/m3g2_reference.py','tests/reference/m3f5_map_0.bin','tools/reference/m3f3_shim.h']},
        assumptions=[
            'Actual pristine MT_PLAYER0, MT_SERGEANT12 and MT_PUFF37. No synthetic body alias or metadata mutation; independent of Rust output.',
            'P_SpawnMobj plus verbatim P_SpawnPlayer association/counter fragment; explicit fresh health100/pistol/floor XY/angle0, not full P_SpawnPlayer/reborn/HUD/psprite initialization.',
            'Medium/commercial/single-player, no armor/cheats/powers or physical inflictor. playeringame=false; no PlayerThink/DeathThink/automatic targeting. Live Demon source explicitly removed before traversal; corpse source retains allocated identity.',
            'Genuine P_DropWeapon runs; P_SetPsprite observes requested downstate only. I_Tactile records presentation request. Neither service executes presentation/weapon animation.',
            'Retired allocations quarantined until reset. Retained mo/attacker are compared opaquely without freed-pointer dereference; no C address-reuse or Rust-generation equivalence claimed.',
            'Inherited GNU ABI/arithmetic/extraction/instrumentation and synthetic map adapters remain. Inherited isolated setup/filter/credit probes execute but are excluded from new player evidence and not republished.',
            'Compiled body inventory is not execution coverage. FN entries count actually executed functions; existing source bodies with new player branches are reused functions, not new function inventory.',
            'Compact selected actor/player captures retain every source event/RNG/thinker row; skipped physical captures are not claimed as field comparisons.'],
        outputs_sha256={p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()})
    outputs['tests/reference/m3g2_provenance.json']=(json.dumps(provenance,indent=2,sort_keys=True)+'\n').encode()
    for p,b in outputs.items():
        if check: assert (ROOT/p).read_bytes()==b,p
        else: (ROOT/p).write_bytes(b)
    print(f'{"Checked" if check else "Generated"} {SCHEMA}: {inventory}; strict={overflow}; ASan/UBSan={address}; bytes={sum(map(len,outputs.values()))}')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');p.add_argument('--overflow-check',action='store_true');p.add_argument('--address-check',action='store_true');a=p.parse_args();run(a.check,a.overflow_check,a.address_check)
