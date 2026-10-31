#!/usr/bin/env python3
"""Independent four-weapon source observations; no Rust expected output."""
from pathlib import Path
import sys,csv,io,collections,json
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3g7_reference as base
base.CASES=[]
def case(label,part=0,**change):
    values=dict(ready=1,owned=3,shells=0,part=part,attack_end=change.get('tics',128))
    values.update(change);base.case(label,**values)
case('fresh_pistol',tics=128,thing=0,attack_start=15,attack_end=15)
case('last_clip_fallback',tics=256,clips=1,dx=-64,thing_angle=0,turn=16384,turn_hold=2,seed=6)
case('zero_clip_fallback',tics=128,clips=0,thing=0)
case('fallback_release_repress',tics=160,clips=1,script=1)
case('fist_under_pressure',tics=384,ready=0,clips=0,dx=-64,thing_angle=0,turn=16384,turn_hold=2,seed=6)
case('fist_current_facing',tics=128,ready=0,clips=0,dx=64,dy=16,thing_angle=0,seed=13,attack_start=15,attack_end=15)
case('fist_out_of_range',tics=96,ready=0,clips=0,dx=850,attack_start=15,attack_end=15)
case('fist_wall',tics=96,ready=0,clips=0,wall=1,attack_start=15,attack_end=15)
case('manual_switch_moving',tics=256,clips=4,script=2,side=4,move_hold=40)
case('empty_pistol_request',tics=160,ready=0,clips=0,script=3,thing=0)
case('death_while_switching',tics=256,dx=-59,thing_angle=0,script=5)
case('unowned_same_requests',tics=128,thing=0,script=8,attack_end=0)
case('shotgun_seven',part=1,tics=128,ready=2,owned=7,shells=5,attack_start=15,attack_end=15)
case('shotgun_held_kill',part=1,tics=256,ready=2,owned=7,shells=5,seed=6)
case('shotgun_mixed_impacts',part=1,tics=128,ready=2,owned=7,shells=1,clips=0,dx=128,dy=32,seed=5,attack_start=15,attack_end=15)
case('shotgun_empty_to_pistol',part=1,tics=160,ready=2,owned=7,shells=0,clips=3)
case('shotgun_last_to_fist',part=1,tics=192,ready=2,owned=7,shells=1,clips=0)
case('shotgun_player_death',part=1,tics=256,ready=2,owned=7,shells=1,clips=0,dx=-59,thing_angle=0,turn=16384,turn_hold=1)
case('chaingun_held_kill',part=2,tics=192,ready=3,owned=11,clips=50,seed=6)
case('chaingun_last_round',part=2,tics=160,ready=3,owned=11,clips=1)
case('chaingun_retaliation',part=2,tics=128,ready=3,owned=11,clips=50,thing_angle=0,seed=13)
case('chaingun_release',part=2,tics=160,ready=3,owned=11,clips=8,script=1)
case('chaingun_player_death',part=2,tics=256,ready=3,owned=11,clips=1,dx=-59,thing_angle=0,turn=16384,turn_hold=1)
case('empty_chaingun_to_shotgun',part=3,tics=256,ready=3,owned=15,clips=0,shells=2)
case('fallback_overrides_pending',part=3,tics=192,ready=2,owned=15,clips=4,shells=1,script=7)
case('all_weapons_one_world',part=3,tics=512,ready=1,owned=15,clips=20,shells=3,script=4,dx=-128,thing_angle=0,turn=16384,turn_hold=2)
case('fist_kill',tics=512,ready=0,clips=0,dx=60,seed=8)
old_body=base.observed_body
base.WEAPON_ACTIONS+=['A_Punch','A_FireShotgun','A_FireCGun','A_Light2']
source=(ROOT/'linuxdoom-1.10/p_user.c').read_text();a=source.index('    // Check for weapon change.');b=source.index('    // check for use',a)
selection=source[a:b]
base.HELPERS+='\nstatic void source_weapon_selection(player_t*player){ticcmd_t*cmd=&player->cmd;weapontype_t newweapon;\n'+selection+'\n}\n'
base.CAPTURE=base.CAPTURE.replace('"WP,','"WX,').replace('p->weaponowned[0]+2*p->weaponowned[1]','p->weaponowned[0]+2*p->weaponowned[1]+4*p->weaponowned[2]+8*p->weaponowned[3]').replace('p->maxammo[0]','p->ammo[1]')
base.DRIVER=base.DRIVER.replace('int before;operation=t;','int before;operation=t;')
base.DRIVER=base.DRIVER.replace('int* v=input[c];','int wi;int* v=input[c];')
base.DRIVER=base.DRIVER.replace('p->ammo[am_clip]=v[V_PC_CLIPS];','p->readyweapon=v[V_PC_READY];for(wi=0;wi<NUMWEAPONS;wi++)p->weaponowned[wi]=(v[V_PC_OWNED]>>wi)&1;p->ammo[am_clip]=v[V_PC_CLIPS];p->ammo[am_shell]=v[V_PC_SHELLS];')
scripts=r'''
if(v[V_PC_SCRIPT]==2){p->cmd.buttons=((t>=16&&t<=27)||(t>=65&&t<=119)||t>=170)?BT_ATTACK:0;if(t==30)p->cmd.buttons|=BT_CHANGE;if(t==130)p->cmd.buttons|=BT_CHANGE|(wp_pistol<<BT_WEAPONSHIFT);}
if(v[V_PC_SCRIPT]==3&&t==21)p->cmd.buttons=BT_ATTACK|BT_CHANGE|(wp_pistol<<BT_WEAPONSHIFT);
if(v[V_PC_SCRIPT]==4){if(t==30||t==410)p->cmd.buttons|=BT_CHANGE;if(t==120)p->cmd.buttons|=BT_CHANGE|(wp_shotgun<<BT_WEAPONSHIFT);if(t==220)p->cmd.buttons|=BT_CHANGE|(wp_chaingun<<BT_WEAPONSHIFT);if(t==310)p->cmd.buttons|=BT_CHANGE|(wp_pistol<<BT_WEAPONSHIFT);}
if(v[V_PC_SCRIPT]==5)p->cmd.buttons=BT_CHANGE|(((t/8)&1)<<BT_WEAPONSHIFT);
if(v[V_PC_SCRIPT]==7&&t==51)p->cmd.buttons|=BT_CHANGE|(wp_pistol<<BT_WEAPONSHIFT);

if(v[V_PC_SCRIPT]==8){p->cmd.buttons=0;if(t==20)p->cmd.buttons=BT_CHANGE|(wp_shotgun<<BT_WEAPONSHIFT);if(t==40)p->cmd.buttons=BT_CHANGE|(wp_chaingun<<BT_WEAPONSHIFT);if(t==60)p->cmd.buttons=BT_CHANGE|(wp_pistol<<BT_WEAPONSHIFT);if(t==80)p->cmd.buttons=BT_CHANGE;}
'''
base.DRIVER=base.DRIVER.replace('if(p->playerstate==PST_DEAD)',scripts+'\nif(p->playerstate==PST_DEAD)')
base.DRIVER=base.DRIVER.replace('P_CalcHeight(p);}','P_CalcHeight(p);source_weapon_selection(p);}')
def observed_body(file,name,result,rename=None):
    text=old_body(file,name,result,rename)
    if name in ['A_FireShotgun','A_FireCGun']:
        needle='player->ammo[weaponinfo[player->readyweapon].ammo]--;'
        assert needle in text
        text=text.replace(needle,needle+'printf("W,%d,%d,2,%ld,%d,%d,%d\\n",scenario,operation,(long)(player-players),weaponinfo[player->readyweapon].ammo,player->ammo[weaponinfo[player->readyweapon].ammo]+1,player->ammo[weaponinfo[player->readyweapon].ammo]);')
    if name=='A_Light2':text=text.replace('player->extralight = 2;','player->extralight = 2;printf("W,%d,%d,3,%ld,%d\\n",scenario,operation,(long)(player-players),player->extralight);')
    if name=='A_Punch':
        text=text.replace('P_Random ()%10','w_random(0)%10').replace('(P_Random()-P_Random())<<18','pistol_spread()')
        needle='P_LineAttack (player->mo, angle, MELEERANGE, slope, damage);'
        assert needle in text
        text=text.replace(needle,'bullet(player->mo,angle,slope,damage);\n    '+needle)
        text=text.replace('{','{\n printf("WK,%d,%d,0,0\\n",scenario,operation);',1)
    if name=='P_GunShot':text=text.replace('{','{\n printf("WK,%d,%d,%d,%d\\n",scenario,operation,mo->player->readyweapon,accurate);',1)
    return text
base.observed_body=observed_body

legacy=base.legacy;main=base.main;commands=base.commands;ADDED=base.ADDED;WEAPON_ACTIONS=base.WEAPON_ACTIONS;CASES=base.CASES;raw_body=base.raw_body
def capture(flags):return base.capture(flags)
import hashlib,json
def sha(data):return hashlib.sha256(data).hexdigest()

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
    from evidence_pack import pack
    expanded = ('DOOM-PLAYER-DEMON-WEAPONS-v1\n' + text.getvalue()).encode()
    outputs = {'m3g8_trace.dref': pack(expanded)}
    text = io.StringIO()
    writer = csv.writer(text, lineterminator='\n')
    writer.writerow(['case', *observer.CASES[0]])
    writer.writerows([index, *values.values()] for index, values in enumerate(observer.CASES))
    outputs['m3g8_cases.csv'] = text.getvalue().encode()
    components = [r for r in csv.reader(io.StringIO(observed['pistol_components.csv'].decode())) if r[0] == 'PC']
    text = io.StringIO()
    csv.writer(text, lineterminator='\n').writerows(components)
    outputs['m3g8_components.csv'] = text.getvalue().encode()
    known = sorted(path for path in (ROOT/'tests/reference').glob('*map_*.bin') if not path.name.startswith('m3g8_'))
    maps, map_paths = [], []
    for index, fixture in enumerate(observer.main.MAPS):
        path = next((path for path in known if path.read_bytes() == fixture['bytes']), None)
        if path is None:
            path = ROOT/f'tests/reference/m3g8_map_{index}.bin'
            outputs[path.name] = fixture['bytes']
        maps.append(path)
        map_paths.append('tests/reference/' + path.name)
    outputs['m3g8_maps.rs'] = ('// Public synthetic source inputs; GPL-2.0-only.\n'
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
        shots=len(shots), shots_by_weapon=dict(sorted(collections.Counter(row[3] for row in rows if row[0]=='WK').items())),
        accuracy_by_weapon=dict(sorted(collections.Counter(':'.join(row[3:]) for row in rows if row[0]=='WK').items())), damage_values=sorted({int(row[-1]) for row in shots}),
        actor_fields=26*sum(row[0]=='A' and row[4]=='1' for row in keep),
        player_fields=6*sum(row[0]=='P' for row in keep),
        weapon_fields=17*sum(row[0]=='WX' for row in keep),
        gameplay_draws=sum(row[0]=='E' and row[3]=='8' for row in keep),
        weapon_draws=sum(row[0]=='W' and row[3]=='8' for row in keep),
        noise_alerts=sum(row[0]=='N' for row in keep),
        shot_trace_ends=sum(row[0]=='X' and row[3]=='1' for row in keep),
        aim_queries=sum(row[0]=='J' and row[3]=='5' for row in keep),
        damage_requests=sum(row[0]=='G' and row[3]=='2' for row in keep),
        impact_spawns=sum(row[0]=='G' and row[3]=='11' for row in keep),
        movement_attempts=sum(row[0]=='M' for row in keep),
        collision_candidates=sum(row[0]=='C' for row in keep))
    assert set(inventory['damage_values']) == set(range(2,21,2)) | {5,15}
    inventory['packed_trace_bytes']=len(outputs['m3g8_trace.dref'])
    inventory['expanded_trace_bytes']=len(expanded)
    source_names = set(manifest['source_sha256']) | {'d_items.c'}
    provenance = dict(schema='DOOM-PLAYER-DEMON-WEAPONS-v1', revision=manifest['revision'],
        flags=manifest['flags'], inventory=inventory, maps=map_paths, functions=bodies,
        function_sha256={f'{f}:{name}':sha(observer.raw_body(f,name,result).encode()) for f,name,result in bodies},
        source_sha256={name:sha((observer.legacy.SOURCE/name).read_bytes()) for name in sorted(source_names)},
        executed_function_entries=dict(sorted(functions.items())),
        assumptions=[
            'Genuine original MT_PLAYER0 and map-THING MT_SERGEANT12, one active player0; original player_t and pristine info.c.',
            'Actual G_PlayerReborn plus declared mo association, controlled initial owned Fist/Pistol/Shotgun/Chaingun and clip/shell loadouts, then P_SetupPsprites. No pickups.',
            'Declared command/reaction fragment, actual P_CalcHeight and P_MovePsprites precede original P_RunThinkers. After death only P_MovePsprites executes. This is not full P_Ticker, P_PlayerThink or P_DeathThink.',
            'All supplied turns are nonnegative and map angles0/-180 remain in the source-defined signed multiplication domain. Existing source spread/jitter operands use declared left-before-right ordering and representable multiplication; this is qualified GNU ABI evidence, not portable unmodified ISO-C equivalence.',
            'One flat sector with genuine one-sided line lists: actual P_NoiseAlert/RecursiveSound set its soundtarget. No generalized propagation.',
            'Source aim queries are read-only observation rows, not invented monster aim events. Source trace/effect/damage/AI bodies execute on current actors.',
            'All encounters use normal health100/150 and no injected damage. Fist winning seed8 was selected from a bounded64-seed original-C experiment; no altered RNG tables or random values. Driver supplies default commands after source death.',
            'Actual fallback, four weapon actions, down/up switching and source weapon-selection fragment execute. Non-owned supported selection is source ignored. Rejection atomicity is Rust policy.',
            'No freed C pointers, empty active-roster loops, proprietary assets or Rust expected observations. Birth/generation and diagnostic projection are separately specified Rust schema policies.'],
        inputs_sha256={name:sha((ROOT/name).read_bytes()) for name in ['tools/reference/m3g8_reference.py','tools/reference/evidence_pack.py','tools/reference/m3g7_reference.py','tools/reference/m3g6_reference.py','tools/reference/m3g5_reference.py','tools/reference/m3g4_reference.py','tools/reference/m3g3_reference.py','tools/reference/m3g2_reference.py','tools/reference/m3f3_shim.h']},
        linked_math_bodies=['FixedMul','FixedDiv','FixedDiv2'],
        expanded_trace_sha256=sha(expanded),
        outputs_sha256={name:sha(data) for name,data in outputs.items()})
    outputs['m3g8_provenance.json'] = (json.dumps(provenance, indent=2, sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check: assert path.read_bytes()==data,path
        else: path.write_bytes(data)
    print(('Checked' if check else 'Generated')+' DOOM-PLAYER-DEMON-WEAPONS-v1: '+str(inventory)+f'; strict={overflow}; ASan/UBSan={address}; bytes={sum(map(len,outputs.values()))}')

if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser()
    parser.add_argument('--check',action='store_true')
    parser.add_argument('--overflow-check',action='store_true')
    parser.add_argument('--address-check',action='store_true')
    args=parser.parse_args()
    run(args.check,args.overflow_check,args.address_check)
