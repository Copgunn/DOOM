#!/usr/bin/env python3
"""Independent original-C pistol/owned-player/Demon observations."""
import contextlib,csv,hashlib,io,json,re,shutil,subprocess,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/reference'))
import m3g6_reference as previous
commands=previous.commands;main=previous.main;legacy=previous.legacy
raw_body=commands.player._original_body
BASE_FUNCTIONS=list(legacy.FUNCTIONS)
WEAPON_ACTIONS=['A_WeaponReady','A_Lower','A_Raise','A_ReFire','A_FirePistol','A_Light0','A_Light1']
ADDED=[('p_pspr.c',n,'boolean'if n=='P_CheckAmmo'else'void')for n in ['P_SetupPsprites','P_MovePsprites','P_SetPsprite','P_BringUpWeapon','P_CheckAmmo','P_FireWeapon','P_BulletSlope','P_GunShot']]
ADDED += [('p_user.c','P_CalcHeight','void'),('g_game.c','G_PlayerReborn','void')]
CASES=[]
def case(label,**change):
    c=dict(previous.CASES[0]);c.pop('map',None)
    c.update(label=label,tics=128,dx=128,dy=0,px=0,py=0,thing_angle=-180,
             pre_damage=0,forward=0,side=0,move_hold=0,turn=0,turn_hold=0,
             clips=50,attack_start=1,attack_end=128,script=0)
    c.update(change);CASES.append(c)
case('autonomous_pistol',tics=288,attack_end=256,dx=-128,thing_angle=0,turn=16384,turn_hold=2,seed=6)
case('idle_raise',tics=128,thing=0,attack_end=0)
case('single_press',tics=128,attack_start=21,attack_end=21)
case('held_refire',tics=160,attack_start=21,attack_end=160)
case('release_repress',tics=140,attack_start=21,attack_end=100,script=1)
case('empty_world_miss',tics=128,thing=0)
case('wall_blocked',tics=128,wall=1,thing_angle=0)
case('fallback_does_not_turn',tics=80,dy=40,thing_angle=0,attack_start=21,attack_end=21)
case('beyond_aim_range',tics=128,dx=850,px=-400,offset_x=128,thing_angle=0)
case('death_during_windup',tics=128,dx=-59,thing_angle=0,turn=16384,turn_hold=2,pre_damage=96,attack_start=24,attack_end=24)
case('last_round',tics=28,clips=1)
case('zero_ammo_idle',tics=128,clips=0,attack_end=0)
case('reaction_weapon_independent',tics=128,reaction=20,forward=8,move_hold=40)
case('autonomous_player_loss',tics=384,attack_end=384,dx=-128,thing_angle=0,turn=16384,turn_hold=2)
case('moving_while_firing',tics=128,attack_end=128,dx=-256,thing_angle=0,turn=16384,turn_hold=2,side=6,move_hold=48)
case('noise_rear',tics=128,thing_angle=0,turn=16384,turn_hold=1,attack_start=21)
case('ambush_behind_wall',tics=128,thing=2,thing_angle=0,wall=1,py=-32,dy=32,turn=16384,turn_hold=1,attack_start=21)

def configure():
    main.MAPS,main.THINGS,main.KEYS,main.CASES=[],[],[],[]
    for c in CASES:
        m=previous.geometry.fixture(n=1,count=5 if c['wall'] else 4,blockers=bool(c['wall']))
        if c['wall']==1:m['vertices'][4:6]=[(16,y)for x,y in m['vertices'][4:6]]
        m['vertices']=[(6*x+c['offset_x'],12*y)for x,y in m['vertices']]
        m['lumps']['VERTEXES']=b''.join(previous.geometry.old.words(v)for v in m['vertices'])
        m['range']=(-768+c['offset_x'],768+c['offset_x']);m['center']=(c['offset_x'],0)
        m=previous.extent.grid(m,(-1024,-1024),16,16)
        things=[]if not c['thing']else[(c['dx'],c['dy'],c['thing_angle'],3002,15 if c['thing']==2 else 7)]
        main.case(c['label'],n=1,custom=m,things=things,tics=c['tics'],rejected=bool(c['rejected']),
                  **{'pc_'+k:v for k,v in c.items()if k not in ['label','tics']})
        c['map']=main.CASES[-1]['map']

HELPERS=r'''
#include <stdint.h>
#define rcsid pistol_data_rcsid
#include "d_items.c"
#undef rcsid
#include "d_event.h"
boolean onground;
fixed_t bulletslope;
int maxammo[NUMAMMO]={200,50,300,50};
#define MAXBOB 0x100000
#define LOWERSPEED FRACUNIT*6
#define RAISESPEED FRACUNIT*6
#define WEAPONBOTTOM 128*FRACUNIT
#define WEAPONTOP 32*FRACUNIT
#define BFGCELLS 40
void P_SetPsprite(player_t*,int,statenum_t);void P_BringUpWeapon(player_t*);
boolean P_CheckAmmo(player_t*);void P_FireWeapon(player_t*);void P_BulletSlope(mobj_t*);
void P_GunShot(mobj_t*,boolean);void G_PlayerReborn(int);
static void w_entry(player_t*p,int position,int state){printf("W,%d,%d,0,%ld,%d,%d\n",scenario,operation,(long)(p-players),position,state);}
static void w_action(player_t*p,pspdef_t*ps,int action){printf("W,%d,%d,1,%ld,%ld,%d\n",scenario,operation,(long)(p-players),(long)(ps-p->psprites),action);}
static int w_random(int kind){int r=P_Random();printf("W,%d,%d,8,%d,%d\n",scenario,operation,kind,r);return r;}
static int pistol_spread(void){int left=w_random(1);int right=w_random(2);return(left-right)*262144;}
static void bullet(mobj_t*m,unsigned angle,fixed_t slope,int damage){printf("W,%d,%d,5,%d,%u,%d,%d\n",scenario,operation,ident(m),angle,slope,damage);}
'''
CAPTURE=r'''
static void weapon_capture(void){int i;player_t*p=&players[0];
printf("WP,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d",scenario,operation,p->readyweapon,p->pendingweapon,p->weaponowned[0]+2*p->weaponowned[1],p->ammo[0],p->maxammo[0],p->attackdown,p->refire,p->bob,p->extralight);
for(i=0;i<2;i++)printf(",%ld,%d,%d,%d",p->psprites[i].state?(long)(p->psprites[i].state-states):-1L,p->psprites[i].tics,p->psprites[i].sx,p->psprites[i].sy);
printf("\n");}
static void weapon_world(void){int i,j;for(i=0;i<numsectors;i++){sectors[i].linecount=0;sectors[i].lines=sector_lines[i];for(j=0;j<numlines;j++)if(lines[j].frontsector==&sectors[i]||lines[j].backsector==&sectors[i])sector_lines[i][sectors[i].linecount++]=&lines[j];}}
static int pistol_components(void){mobj_t*body;player_t*p=&players[0];int ok;
reset(0);G_PlayerReborn(0);body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);p->viewheight=VIEWHEIGHT;P_SetupPsprites(p);P_SetPsprite(p,ps_weapon,S_PISTOL);
p->ammo[am_clip]=0;ok=P_CheckAmmo(p);printf("PC,empty,%d,%d,%d,%ld,%d,%d,%d\n",ok,p->readyweapon,p->pendingweapon,(long)(p->psprites[0].state-states),p->psprites[0].tics,p->psprites[0].sy,p->ammo[0]);
reset(0);G_PlayerReborn(0);body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);p->viewheight=VIEWHEIGHT;P_SetupPsprites(p);P_DamageMobj(body,NULL,NULL,100);printf("PC,dead_lower,%d,%d,%ld,%d,%d\n",p->readyweapon,p->pendingweapon,(long)(p->psprites[0].state-states),p->psprites[0].tics,p->psprites[0].sy);
return 0;}
'''
DRIVER=r'''
for(c=0;c<CASE_COUNT;c++){
int* v=input[c];mobj_t *body,*origin=NULL;player_t*p=&players[0];
reset(c);onground=0;prndindex=v[V_PC_SEED];printf("SEED,%d,0,%d\n",c,prndindex);admit_native=0;
G_PlayerReborn(0);playeringame[0]=1;P_LoadThings(0);weapon_world();if(owned_count)origin=owned[0];
printf("F,%d,0,-1,0,0,0,0,0,%d,%d,%d,%d\n",scenario,owned_count,prndindex,totalkills,totalitems);
body=P_SpawnMobj(v[V_PC_PX]*FRACUNIT,v[V_PC_PY]*FRACUNIT,ONFLOORZ,MT_PLAYER);associate(body);p->viewheight=VIEWHEIGHT;
p->ammo[am_clip]=v[V_PC_CLIPS];printf("W,%d,0,7,0,%d\n",scenario,ident(body));P_SetupPsprites(p);
if(v[V_PC_PRE_DAMAGE])P_DamageMobj(body,NULL,origin,v[V_PC_PRE_DAMAGE]);
body->reactiontime=v[V_PC_REACTION];capture();command_capture();demon_capture(origin);weapon_capture();
for(t=1;t<=v[V_TICS];t++){
int before;operation=t;query_index=-1;attempt=-1;
if(!playeringame[0]||!p->mo||p->mo!=body||body->player!=p)abort();
p->cmd.forwardmove=t<=v[V_PC_MOVE_HOLD]?v[V_PC_FORWARD]:0;
p->cmd.sidemove=t<=v[V_PC_MOVE_HOLD]?v[V_PC_SIDE]:0;
p->cmd.angleturn=t<=v[V_PC_TURN_HOLD]?v[V_PC_TURN]:0;
p->cmd.buttons=t>=v[V_PC_ATTACK_START]&&t<=v[V_PC_ATTACK_END]?BT_ATTACK:0;
if(v[V_PC_SCRIPT]==1)p->cmd.buttons=(t>=21&&t<=34)||(t>=70&&t<=90)?BT_ATTACK:0;
if(p->playerstate==PST_DEAD){p->cmd.forwardmove=p->cmd.sidemove=p->cmd.angleturn=p->cmd.buttons=0;}
printf("PG,%d,%d,0\n",scenario,operation);
if(p->playerstate==PST_LIVE){before=body->reactiontime;source_command_gate(p);if(!before)printf("PG,%d,%d,1,%d\n",scenario,operation,ident(body));P_CalcHeight(p);}
P_MovePsprites(p);
printf("PG,%d,%d,2\n",scenario,operation);
P_RunThinkers();leveltime++;capture();command_capture();demon_capture(origin);weapon_capture();
}}
return 0;}
'''

def observed_body(file,name,result,rename=None):
    text=commands.observed_body(file,name,result,rename)
    if name=='P_SetPsprite':text=re.sub(r'(do\s*\{)',r'\1\n w_entry(player,position,stnum);',text,count=1)
    if name in WEAPON_ACTIONS:text=text.replace('{','{\n w_action(player,psp,'+str(legacy.actors.ACTIONS.index(name))+');',1)
    if name=='A_FirePistol':text=text.replace('player->ammo[weaponinfo[player->readyweapon].ammo]--;','player->ammo[weaponinfo[player->readyweapon].ammo]--;printf("W,%d,%d,2,%ld,%d,%d,%d\\n",scenario,operation,(long)(player-players),weaponinfo[player->readyweapon].ammo,player->ammo[weaponinfo[player->readyweapon].ammo]+1,player->ammo[weaponinfo[player->readyweapon].ammo]);')
    if name in ['A_Light0','A_Light1']:text=re.sub(r'(player->extralight = [01];)',r'\1\n printf("W,%d,%d,3,%ld,%d\\n",scenario,operation,(long)(player-players),player->extralight);',text)
    if name=='P_FireWeapon':text=text.replace('    P_SetMobjState (player->mo, S_PLAY_ATK1);','    printf("W,%d,%d,4,%ld,%d,%d\\n",scenario,operation,(long)(player-players),player->readyweapon,player->refire);\n    P_SetMobjState (player->mo, S_PLAY_ATK1);')
    if name=='P_GunShot':text=text.replace('P_Random ()%3','w_random(0)%3').replace('(P_Random()-P_Random())<<18','pistol_spread()').replace('    P_LineAttack (mo, angle, MISSILERANGE, bulletslope, damage);','    bullet(mo,angle,bulletslope,damage);\n    P_LineAttack (mo, angle, MISSILERANGE, bulletslope, damage);')
    if name=='P_DropWeapon':text=text.replace('{','{\n printf("PL,%d,%d,2,%d,%d\\n",scenario,operation,player->frags[0],weaponinfo[player->readyweapon].downstate);',1)
    return text

def capture(flags):
    configure()
    header=commands.header.replace('if(argc<2)return 2;','if(argc<2)return 2;\nif(!strcmp(argv[1],"pistol-components"))return pistol_components();')
    legacy.MAIN=commands.prefix+'static void source_command_gate(player_t*player){\n'+commands.GATE+'}\n'+commands.CAPTURE+CAPTURE+header+DRIVER
    with tempfile.TemporaryDirectory(prefix='doom-pistol-source-')as directory:
        root=Path(directory);(root/'tools/reference').mkdir(parents=True);(root/'tests/reference').mkdir(parents=True)
        for p in (ROOT/'tools/reference').glob('generate*.py'):shutil.copyfile(p,root/'tools/reference'/p.name)
        shim=(ROOT/'tools/reference/m3f3_shim.h').read_text()
        shim=shim.replace('void P_DropWeapon(player_t* p){(void)p;abort();}','')
        shim=shim.replace('void I_Tactile(int a,int b,int c){(void)a;(void)b;(void)c;abort();}',r'''void I_Tactile(int a,int b,int c){if(a!=40||b!=10)abort();printf("PL,%d,%d,1,%d,%d,%d,%d\n",scenario,operation,players[0].health,players[0].attacker?ident(players[0].attacker):-1,players[0].damagecount,c);}''')
        shim=shim.replace('void P_SlideMove(mobj_t* p){(void)p;abort();}','fixed_t bestslidefrac,secondslidefrac,tmxmove,tmymove;line_t *bestslideline,*secondslideline;mobj_t*slidemo;')
        shim=shim.replace('static void record_move(','static int observer_query;\nstatic void record_move(').replace('subsector_t* s=R_PointInSubsector(x,y);','subsector_t*s;observer_query=1;s=R_PointInSubsector(x,y);observer_query=0;')
        caps=re.search(r'int\s+maxammo\[NUMAMMO\][^;]+;',(legacy.SOURCE/'p_inter.c').read_text()).group(0)
        helpers=HELPERS.replace('int maxammo[NUMAMMO]={200,50,300,50};',caps)
        shim=shim.replace('/* SOURCE_BODIES */',helpers+'\n/* SOURCE_BODIES */')
        (root/'tools/reference/m3d3j_shim.h').write_text(shim)
        oldrun=subprocess.run;oldbody=legacy.base.body;oldroot=legacy.ROOT;component=[]
        def compile_run(args,*a,**kw):
            if args[0]=='cc' and any(str(x).endswith('reference.c') for x in args):
                path=next(Path(x)for x in args if str(x).endswith('reference.c'))
                source=path.read_text()
                for name in WEAPON_ACTIONS:
                    stub=f'void {name}(mobj_t* actor){{(void)actor;abort();}}'
                    assert source.count(stub)==1,name
                    source=source.replace(stub,'')
                source+='\n'+'\n'.join(observed_body('p_pspr.c',n,'void')for n in WEAPON_ACTIONS)
                path.write_text(source)
                result=oldrun(args,*a,**kw)
                if result.returncode==0:
                    binary=args[args.index('-o')+1]
                    component.append(subprocess.check_output([str(binary),'pistol-components']))
                return result
            return oldrun(args,*a,**kw)
        legacy.ROOT=root;legacy.MAPS=main.MAPS;legacy.CASES=main.CASES;legacy.THING_INPUTS=main.THINGS
        legacy.GEOMETRIES=[(i,())for i in range(len(main.MAPS))];legacy.FLAGS=flags;legacy.combat.c_maps=main.loader
        legacy.FUNCTIONS=BASE_FUNCTIONS+ADDED;legacy.base.body=observed_body;subprocess.run=compile_run
        try:
            with contextlib.redirect_stdout(io.StringIO()):legacy.run(False)
        finally:subprocess.run=oldrun;legacy.base.body=oldbody;legacy.ROOT=oldroot
        output={p.name:p.read_bytes()for p in (root/'tests/reference').iterdir()}
        assert len(component)==1
        output['pistol_components.csv']=component[0]
        return output



def sha(data):
    return hashlib.sha256(data).hexdigest()

def run(check, overflow, address):
    flags=list(legacy.base.FLAGS)
    observed=capture(flags)
    for enabled, sanitizers in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited=capture([flag for flag in flags if flag!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizers,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==observed['m3d3j_trace.csv']
            assert audited['pistol_components.csv']==observed['pistol_components.csv']
    import collections
    observer=sys.modules[__name__]
    rows = list(csv.reader(io.StringIO(observed['m3d3j_trace.csv'].decode())))
    functions = collections.Counter(row[3] for row in rows if row[0] == 'FN')
    shots = [row for row in rows if row[0] == 'W' and row[3] == '5']
    shot_keys = {tuple(map(int, row[1:3])) for row in shots}
    damage_keys = {tuple(map(int, row[1:3])) for row in rows if row[0] == 'G' and row[3] == '2'}
    selected = {0, 1, 2, 8, 14, 15, 18, 19, 20, 21, 24, 25, 26, 28, 29, 33,
                39, 43, 47, 64, 96, 128, 160, 192, 215, 256, 257, 288, 384}
    capture_keys = {tuple(map(int, row[1:3])) for row in rows if row[0] == 'P'
                    and (int(row[2]) in selected or tuple(map(int, row[1:3])) in shot_keys | damage_keys
                         or int(row[2]) == observer.CASES[int(row[1])]['tics'])}
    keep = [row for row in rows if row[0] != 'FN' and
            (row[0] not in {'A', 'P', 'AP', 'R', 'S', 'LS'} or tuple(map(int, row[1:3])) in capture_keys)]
    text = io.StringIO()
    csv.writer(text, lineterminator='\n').writerows(keep)
    outputs = {'m3g7_trace.csv': ('DOOM-PLAYER-DEMON-PISTOL-v1\n' + text.getvalue()).encode()}
    text = io.StringIO()
    writer = csv.writer(text, lineterminator='\n')
    writer.writerow(['case', *observer.CASES[0]])
    writer.writerows([index, *values.values()] for index, values in enumerate(observer.CASES))
    outputs['m3g7_cases.csv'] = text.getvalue().encode()
    components = [r for r in csv.reader(io.StringIO(observed['pistol_components.csv'].decode())) if r[0] == 'PC']
    text = io.StringIO()
    csv.writer(text, lineterminator='\n').writerows(components)
    outputs['m3g7_components.csv'] = text.getvalue().encode()
    known = sorted(path for path in (ROOT/'tests/reference').glob('*map_*.bin') if not path.name.startswith('m3g7_'))
    maps, map_paths = [], []
    for index, fixture in enumerate(observer.main.MAPS):
        path = next((path for path in known if path.read_bytes() == fixture['bytes']), None)
        if path is None:
            path = ROOT/f'tests/reference/m3g7_map_{index}.bin'
            outputs[path.name] = fixture['bytes']
        maps.append(path)
        map_paths.append('tests/reference/' + path.name)
    outputs['m3g7_maps.rs'] = ('// Public synthetic source inputs; GPL-2.0-only.\n'
        'pub const MAP_BYTES: &[&[u8]] = &[\n' + ''.join(
            f'    include_bytes!("{path.name}"),\n' for path in maps) + '];\n').encode()
    manifest = json.loads(observed['m3d3j_provenance.json'])
    bodies = manifest['functions'] + [['p_pspr.c', name, 'void'] for name in observer.WEAPON_ACTIONS]
    assert len(bodies) == len(set(tuple(body) for body in bodies))
    known_bodies = set()
    for path in (ROOT/'tests/reference').glob('*provenance.json'):
        for body in json.loads(path.read_text()).get('functions', []):
            if isinstance(body, list): known_bodies.add(':'.join(body[:2]))
            elif isinstance(body,str): known_bodies.add(body)
            elif isinstance(body,dict) and 'source' in body and 'name' in body: known_bodies.add(body['source']+':'+body['name'])
    inventory = dict(new_cases=len(observer.CASES), tics=sum(c['tics'] for c in observer.CASES),
        captures=len(capture_keys), selected_original_bodies=len(bodies), compiled_original_bodies=len(bodies)+3,
        observer_added_bodies=len(observer.ADDED)+len(observer.WEAPON_ACTIONS),
        globally_new_bodies=len({f'{f}:{name}' for f,name,_ in bodies} - known_bodies),
        exercised_unique_functions=len(functions), function_entries=sum(functions.values()),
        weapon_action_entries=sum(functions[f'p_pspr.c:{name}'] for name in observer.WEAPON_ACTIONS),
        shots=len(shots), damage_values=sorted({int(row[-1]) for row in shots}),
        actor_fields=26*sum(row[0]=='A' and row[4]=='1' for row in keep),
        player_fields=6*sum(row[0]=='P' for row in keep),
        weapon_fields=17*sum(row[0]=='WP' for row in keep),
        gameplay_draws=sum(row[0]=='E' and row[3]=='8' for row in keep),
        weapon_draws=sum(row[0]=='W' and row[3]=='8' for row in keep),
        noise_alerts=sum(row[0]=='N' for row in keep),
        shot_trace_ends=sum(row[0]=='X' and row[3]=='1' for row in keep),
        aim_queries=sum(row[0]=='J' and row[3]=='5' for row in keep),
        damage_requests=sum(row[0]=='G' and row[3]=='2' for row in keep),
        impact_spawns=sum(row[0]=='G' and row[3]=='11' for row in keep),
        movement_attempts=sum(row[0]=='M' for row in keep),
        collision_candidates=sum(row[0]=='C' for row in keep))
    assert inventory['damage_values'] == [5, 10, 15]
    source_names = set(manifest['source_sha256']) | {'d_items.c'}
    provenance = dict(schema='DOOM-PLAYER-DEMON-PISTOL-v1', revision=manifest['revision'],
        flags=manifest['flags'], inventory=inventory, maps=map_paths, functions=bodies,
        function_sha256={f'{f}:{name}':sha(observer.raw_body(f,name,result).encode()) for f,name,result in bodies},
        source_sha256={name:sha((observer.legacy.SOURCE/name).read_bytes()) for name in sorted(source_names)},
        executed_function_entries=dict(sorted(functions.items())),
        assumptions=[
            'Genuine original MT_PLAYER0 and map-THING MT_SERGEANT12, one active player0; original player_t and pristine info.c.',
            'Actual G_PlayerReborn plus declared source mo association, initial ammo setup and P_SetupPsprites. Default clips50; cases1/0 are controlled inventory inputs, not pickups.',
            'Declared command/reaction fragment, actual P_CalcHeight and P_MovePsprites precede original P_RunThinkers. After death only P_MovePsprites executes. This is not full P_Ticker, P_PlayerThink or P_DeathThink.',
            'All supplied turns are nonnegative and map angles0/-180 remain in the source-defined signed multiplication domain. Existing source spread/jitter operands use declared left-before-right ordering and representable multiplication; this is qualified GNU ABI evidence, not portable unmodified ISO-C equivalence.',
            'One flat sector with genuine one-sided line lists: actual P_NoiseAlert/RecursiveSound set its soundtarget. No generalized propagation.',
            'Source aim queries are read-only observation rows, not invented monster aim events. Source trace/effect/damage/AI bodies execute on current actors.',
            'Autonomous winning and losing encounters have no injected damage. Death-during-windup alone starts after controlled source damage96. Driver supplies default commands after source death.',
            'Empty-ammo P_CheckAmmo fallback is independently observed only as a component; fist execution is unsupported. Last-round source case stops before fallback. Rejection atomicity is Rust policy.',
            'No freed C pointers, empty active-roster loops, proprietary assets or Rust expected observations. Birth/generation and diagnostic projection are separately specified Rust schema policies.'],
        inputs_sha256={name:sha((ROOT/name).read_bytes()) for name in ['tools/reference/m3g7_reference.py','tools/reference/m3g6_reference.py','tools/reference/m3g5_reference.py','tools/reference/m3g4_reference.py','tools/reference/m3g3_reference.py','tools/reference/m3g2_reference.py','tools/reference/m3f3_shim.h']},
        linked_math_bodies=['FixedMul','FixedDiv','FixedDiv2'],
        outputs_sha256={name:sha(data) for name,data in outputs.items()})
    outputs['m3g7_provenance.json'] = (json.dumps(provenance, indent=2, sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check: assert path.read_bytes()==data,path
        else: path.write_bytes(data)
    print(('Checked' if check else 'Generated')+' DOOM-PLAYER-DEMON-PISTOL-v1: '+str(inventory)+f'; strict={overflow}; ASan/UBSan={address}; bytes={sum(map(len,outputs.values()))}')

if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser()
    parser.add_argument('--check',action='store_true')
    parser.add_argument('--overflow-check',action='store_true')
    parser.add_argument('--address-check',action='store_true')
    args=parser.parse_args()
    run(args.check,args.overflow_check,args.address_check)
