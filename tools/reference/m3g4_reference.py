#!/usr/bin/env python3
"""Original player command/gate observations; expected values never come from Rust."""
import argparse
import collections
import csv
import hashlib
import io
import json
import shutil
import subprocess
import tempfile
from pathlib import Path
import m3g3_reference as source

ROOT = Path(__file__).resolve().parents[2]
player, main, legacy = source.player, source.main, source.legacy
SCHEMA = 'DOOM-PLAYER-COMMAND-v1'
CASES = []


def case(label, tics=64, **values):
    c = dict(label=label, tics=tics, forward=0, side=0, turn=0,
             move_hold=12, turn_hold=0, px=0, py=0, reaction=0,
             thing=0, pre_damage=0, hit_at=-1, hit_amount=0, physical=0)
    c.update(values)
    CASES.append(c)


case('idle', tics=16, move_hold=0)
case('forward', forward=25, px=-64)
case('backward', forward=-25, px=64)
case('side_positive', side=24, py=64)
case('side_negative', side=-24, py=-64)
case('positive_turn_boundary', tics=8, px=994, turn=32767, turn_hold=8)
case('negative_turn_boundary', tics=8, py=-994, turn=-32768, turn_hold=8)
case('turn_forward', forward=25, turn=1024, move_hold=24, turn_hold=24)
case('turn_backward', forward=-25, turn=-1024, move_hold=24, turn_hold=24)
case('combined', forward=25, side=24, turn=512, move_hold=24, turn_hold=24)
case('continuous', tics=128, forward=25, turn=1024, move_hold=128, turn_hold=128)
case('reaction_one', tics=48, forward=25, turn=2048, turn_hold=12, reaction=1)
case('reaction_three', tics=48, side=-24, turn=-1024, turn_hold=12, reaction=3)
case('delayed_knockback', forward=25, side=-24, turn=512, move_hold=16,
     turn_hold=16, reaction=3, thing=1, hit_at=0, hit_amount=20, physical=1)
case('damage_clears_delay', forward=25, turn=-512, move_hold=16, turn_hold=16,
     reaction=3, thing=1, hit_at=1, hit_amount=20, physical=1)
case('wall_slide', px=96, forward=25, side=24, move_hold=32)
case('solid_dying_blocker', tics=48, forward=50, move_hold=8, thing=2)
case('moving_normal_death', tics=72, forward=25, move_hold=8,
     hit_at=9, hit_amount=100)
case('moving_extreme_death', tics=72, forward=-25, move_hold=4, pre_damage=99,
     thing=1, hit_at=5, hit_amount=120, physical=1)
case('turning_friction_stop', forward=25, px=-64, turn=512, turn_hold=64)
case('inward_boundary', tics=32, px=980, forward=-1, move_hold=16)
case('reaction_maximum', tics=257, forward=1, move_hold=257, reaction=255)

main.MAPS, main.THINGS, main.KEYS, main.CASES = [], [], [], []
for c in CASES:
    things = [] if not c['thing'] else [(-96 if c['thing'] == 1 else 64, 0, 0, 3002, 7)]
    main.case(c['label'], n=1, things=things, tics=c['tics'], skip=0,
              **{'pc_' + k: v for k, v in c.items() if k not in ['label', 'tics']})
    c['map'] = main.CASES[-1]['map']

user = (legacy.SOURCE / 'p_user.c').read_text()
start = user.index('    if (player->mo->reactiontime)')
GATE = user[start:user.index('    P_CalcHeight (player);', start)]
prefix = player.MAIN[:player.MAIN.index('int main(')]
header = player.MAIN[player.MAIN.index('int main('):player.MAIN.index('for(c=0;c<CASE_COUNT;c++)')]
DRIVER = r'''
for(c=0;c<CASE_COUNT;c++){
int* v=input[c];mobj_t *body,*origin=NULL;
/* CASE_FILTER */
reset(c);onground=0;printf("SEED,%d,0,0\n",c);admit_native=0;
P_LoadThings(0);if(owned_count)origin=owned[0];
printf("F,%d,0,-1,0,0,0,0,0,%d,%d,%d,%d\n",scenario,owned_count,prndindex,totalkills,totalitems);
players[0].health=100;players[0].readyweapon=wp_pistol;
body=P_SpawnMobj(v[V_PC_PX]*FRACUNIT,v[V_PC_PY]*FRACUNIT,ONFLOORZ,MT_PLAYER);associate(body);
if(origin)P_DamageMobj(origin,NULL,NULL,150);
if(v[V_PC_PRE_DAMAGE])P_DamageMobj(body,NULL,NULL,v[V_PC_PRE_DAMAGE]);
if(v[V_PC_HIT_AT]==0)P_DamageMobj(body,v[V_PC_PHYSICAL]?origin:NULL,origin,v[V_PC_HIT_AMOUNT]);
/* Explicit controlled setup, not teleport or source spawn initialization. */
body->reactiontime=v[V_PC_REACTION];capture();command_capture();
for(t=1;t<=v[V_TICS];t++){
int before;operation=t;query_index=-1;attempt=-1;
if(t==v[V_PC_HIT_AT])P_DamageMobj(body,v[V_PC_PHYSICAL]?origin:NULL,origin,v[V_PC_HIT_AMOUNT]);
players[0].cmd.forwardmove=t<=v[V_PC_MOVE_HOLD]?v[V_PC_FORWARD]:0;
players[0].cmd.sidemove=t<=v[V_PC_MOVE_HOLD]?v[V_PC_SIDE]:0;
players[0].cmd.angleturn=t<=v[V_PC_TURN_HOLD]?v[V_PC_TURN]:0;
printf("PG,%d,%d,0\n",scenario,operation);
if(players[0].playerstate==PST_LIVE){
before=body->reactiontime;source_command_gate(&players[0]);
if(!before)printf("PG,%d,%d,1,%d\n",scenario,operation,ident(body));
}
printf("PG,%d,%d,2\n",scenario,operation);
P_RunThinkers();leveltime++;capture();command_capture();
}}
return 0;}
'''
CAPTURE = r'''
static void command_capture(void){mobj_t* m=players[0].mo;
printf("UC,%d,%d,%d,%d,%d,%d\n",scenario,operation,
players[0].cmd.forwardmove,players[0].cmd.sidemove,players[0].cmd.angleturn,players[0].cmd.buttons);
printf("PM,%d,%d,%u,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d\n",scenario,operation,
m->angle,m->x,m->y,m->momx,m->momy,m->z,m->momz,m->reactiontime,
(long)(m->state-states),m->tics,m->health,players[0].health,players[0].playerstate);
}
'''
legacy.FUNCTIONS = [*legacy.FUNCTIONS, ('p_user.c', 'P_Thrust', 'void'),
                    ('p_user.c', 'P_MovePlayer', 'void')]
_observed_body = source.observed_body
normalize_turns = True


def observed_body(file, name, result, rename=None):
    text = _observed_body(file, name, result, rename)
    if name == 'P_MovePlayer' and normalize_turns:
        assert text.count('(cmd->angleturn<<16)') == 1
        text = text.replace('(cmd->angleturn<<16)', '((uint32_t)(int32_t)cmd->angleturn << 16)')
    return text


def capture(flags, literal_positive=False):
    global normalize_turns
    normalize_turns = not literal_positive
    driver = DRIVER.replace('/* CASE_FILTER */',
                            'if(v[V_PC_TURN]<0){printf("SEED,%d,0,0\\n",c);continue;}' if literal_positive else '')
    legacy.MAIN = prefix + 'static void source_command_gate(player_t* player){\n' + GATE + '}\n' + CAPTURE + header + driver
    with tempfile.TemporaryDirectory(prefix='doom-command-services-') as directory:
        temp = Path(directory)
        (temp / 'tools/reference').mkdir(parents=True)
        for p in (ROOT / 'tools/reference').glob('generate*.py'):
            shutil.copyfile(p, temp / 'tools/reference' / p.name)
        shim = (ROOT / 'tools/reference/m3f3_shim.h').read_text()
        shim = shim.replace('/* SOURCE_BODIES */', '#include <stdint.h>\nboolean onground;\n'
                            'typedef char require_signed_char[(CHAR_MIN<0 && sizeof(short)==2)?1:-1];\n/* SOURCE_BODIES */')
        (temp / 'tools/reference/m3f3_shim.h').write_text(shim)
        old_root, old_observer = source.ROOT, source.observed_body
        source.ROOT, source.observed_body = temp, observed_body
        try:
            return source.capture(flags)
        finally:
            source.ROOT, source.observed_body = old_root, old_observer


def signed_shift_probe():
    """Intentional source-expression UB diagnostic, never a gameplay oracle."""
    expression = '(cmd->angleturn<<16)'
    assert expression in user
    code = '#include <stdint.h>\nint main(void){struct command{short angleturn;} value;struct command* cmd=&value;volatile short turn=-1;value.angleturn=turn;\n'
    code += 'uint32_t angle=0;angle+=' + expression + ';return angle==0;}\n'
    flags = ['-std=c11', '-O0', '-fno-wrapv', '-fsanitize=shift', '-fno-sanitize-recover=all']
    with tempfile.TemporaryDirectory(prefix='doom-angle-shift-probe-') as directory:
        c = Path(directory) / 'probe.c'; binary = Path(directory) / 'probe'
        c.write_text(code)
        subprocess.run(['cc', *flags, str(c), '-o', str(binary)], check=True)
        result = subprocess.run([str(binary)], capture_output=True, text=True)
    assert result.returncode != 0 and 'left shift of negative value -1' in result.stderr
    return dict(flags=flags, expected_exit=result.returncode, diagnostic='left shift of negative value -1',
                expression=expression, source_sha256=hashlib.sha256(code.encode()).hexdigest(),
                gameplay_expected_values=False)


def run(check, overflow, address):
    flags = list(legacy.base.FLAGS)
    output = capture(flags); raw = output['m3d3j_trace.csv']
    for enabled, extra in [(overflow, ['-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow']),
                           (address, ['-fsanitize=address,undefined'])]:
        if enabled:
            strict = capture([f for f in flags if f != '-fwrapv'] + ['-fno-wrapv', *extra, '-fno-sanitize-recover=all'])
            assert strict['m3d3j_trace.csv'] == raw
    rows = list(csv.reader(io.StringIO(raw.decode())))
    positive = {i for i, c in enumerate(CASES) if c['turn'] >= 0}
    literal = capture([f for f in flags if f != '-fwrapv'] + ['-fno-wrapv', '-fsanitize=undefined', '-fno-sanitize-recover=all'], True)
    literal_rows = [r for r in csv.reader(io.StringIO(literal['m3d3j_trace.csv'].decode())) if int(r[1]) in positive]
    assert literal_rows == [r for r in rows if int(r[1]) in positive]
    probe = signed_shift_probe()
    functions = collections.Counter(r[3] for r in rows if r[0] == 'FN')
    selected = {0, 1, 2, 3, 4, 6, 7, 8, 12, 16, 24, 32, 40, 48, 64, 72, 96, 128, 255, 256, 257}
    keep = [r for r in rows if r[0] not in {'FN', 'D', 'B', 'U', 'I', 'T'} and
            (r[0] not in {'A', 'P', 'AP', 'R', 'S', 'LS'} or int(r[2]) in selected or
             int(r[2]) == CASES[int(r[1])]['tics'])]
    stage_rng = 0; inside = False
    for r in keep:
        if r[0] == 'PG' and r[3] == '0': inside = True
        if r[0] == 'PG' and r[3] == '2': inside = False
        stage_rng += int(inside and r[0] == 'E' and r[3] == '8')
    calls = sum(r[0] == 'PG' and r[3] == '1' for r in keep)
    assert stage_rng == 0 and calls == functions['p_user.c:P_MovePlayer']
    manifest = json.loads(output['m3d3j_provenance.json'])
    inventory = dict(new_cases=len(CASES), tics=sum(c['tics'] for c in CASES),
        captures=sum(r[0] == 'P' for r in keep), actor_fields=26*sum(r[0] == 'A' and r[4] == '1' for r in keep),
        player_fields=6*sum(r[0] == 'P' for r in keep), association_fields=sum(r[0] == 'AP' for r in keep),
        statistics_fields=4*sum(r[0] == 'LS' for r in keep), motion_fields=13*sum(r[0] == 'PM' for r in keep),
        command_fields=4*sum(r[0] == 'UC' for r in keep), command_calls=calls, command_stage_rng_draws=stage_rng,
        gameplay_draws=sum(r[0] == 'E' and r[3] == '8' for r in keep), setup_discards=0,
        movement_attempts=sum(r[0] == 'M' for r in keep), collision_candidates=sum(r[0] == 'C' for r in keep),
        slide_line_observations=sum(r[0] == 'SL' for r in keep), compiled_original_bodies=len(manifest['functions']),
        added_observer_bodies=2, new_engine_source_bodies=0,
        exercised_unique_functions=len(functions), function_entries=sum(functions.values()),
        literal_defined_cases=len(positive), literal_defined_tics=sum(CASES[i]['tics'] for i in positive))
    text = io.StringIO(); csv.writer(text, lineterminator='\n').writerows(keep)
    cases = io.StringIO(); writer = csv.writer(cases, lineterminator='\n')
    writer.writerow(['case', *CASES[0]]);writer.writerows([i, *c.values()] for i, c in enumerate(CASES))
    outputs = {'tests/reference/m3g4_trace.csv': (SCHEMA+'\n'+text.getvalue()).encode(),
               'tests/reference/m3g4_cases.csv': cases.getvalue().encode()}
    known = [ROOT/'tests/reference/m3f5_map_0.bin', *sorted((ROOT/'tests/reference').glob('m3g3_map_*.bin'))]
    maps = []
    for i, m in enumerate(main.MAPS):
        path = next((p for p in known if p.read_bytes() == m['bytes']), None)
        if path is None:
            path = ROOT/f'tests/reference/m3g4_map_{i}.bin'; outputs[str(path.relative_to(ROOT))] = m['bytes']
        maps.append(str(path.relative_to(ROOT)))
    outputs['tests/reference/m3g4_maps.rs'] = ('// Public synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+
        ''.join(f'    include_bytes!("{Path(p).name}"),\n' for p in maps)+'];\n').encode()
    hashes = {f+':'+n:hashlib.sha256(player._original_body(f,n,r).encode()).hexdigest() for f,n,r in manifest['functions']}
    for k, v in json.loads((ROOT/'tests/reference/m3g3_provenance.json').read_text())['function_sha256'].items():
        assert hashes[k] == v, k
    provenance = dict(schema=SCHEMA, revision=manifest['revision'], flags=flags, inventory=inventory, maps=maps,
        source_sha256={**manifest['source_sha256'], **{f:hashlib.sha256((legacy.SOURCE/f).read_bytes()).hexdigest() for f in ['p_user.c','d_ticcmd.h','d_player.h','g_game.c']}},
        functions=manifest['functions'], function_sha256=hashes, executed_function_entries=dict(sorted(functions.items())),
        reaction_fragment_sha256=hashlib.sha256(GATE.encode()).hexdigest(), negative_shift_probe=probe,
        turn_adapter=dict(literal='(cmd->angleturn<<16)', normalized='((uint32_t)(int32_t)cmd->angleturn << 16)',
            normalized_body_sha256=hashlib.sha256(player._original_body('p_user.c','P_MovePlayer','void').replace('(cmd->angleturn<<16)', '((uint32_t)(int32_t)cmd->angleturn << 16)').encode()).hexdigest()),
        inputs_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in ['tools/reference/m3g4_reference.py','tools/reference/m3g3_reference.py','tools/reference/m3g2_reference.py','tools/reference/m3f3_shim.h']},
        assumptions=[
            'Genuine MT_PLAYER0 and map-THING MT_SERGEANT12. Source player association fragment; no alias, roster or full spawn/reborn claim.',
            'Verbatim P_PlayerThink movement conditional plus explicit PST_LIVE driver before P_RunThinkers; not full PlayerThink/Ticker/DeathThink. Zero dead input runs corpse physics only. Reaction delay is controlled initial actor-counter setup, not teleport gameplay.',
            'P_Thrust/P_MovePlayer already compiled historically: two additions to this observer, zero globally new source bodies. Literal defined positive-turn cases agree with normalized model. Negative shifts use the existing uint32_t adapter; no ISO-C equivalence for undefined negative left shift. Separate source-expression probe intentionally diagnoses UB and supplies no gameplay values.',
            'Directional input -50..50, signed 16-bit turn, buttons0, only source player0; Medium/commercial/pistol, no cheats/powers/armor/view/weapon/counter-decay services, flat grounded axis walls, dying/corpse Demon closure.',
            'Per-tic PM/UC fields overlap selected full actor/player captures and are reported separately. Every compared movement/collision/state/RNG/thinker/slide row retained; auxiliary path internals omitted. Birth/generation projection is independent Rust schema policy, not C allocator equivalence.',
            'Inherited GNU ABI, arithmetic, presentation service boundaries and quarantined retirement remain; source compiled function inventory is not branch coverage. No Rust output used.'],
        outputs_sha256={p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()})
    outputs['tests/reference/m3g4_provenance.json'] = (json.dumps(provenance, indent=2, sort_keys=True)+'\n').encode()
    for path, data in outputs.items():
        if check: assert (ROOT/path).read_bytes() == data, path
        else: (ROOT/path).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} {SCHEMA}: {inventory}; strict={overflow}; ASan/UBSan={address}; bytes={sum(map(len, outputs.values()))}')


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--check', action='store_true')
    p.add_argument('--overflow-check', action='store_true'); p.add_argument('--address-check', action='store_true')
    a = p.parse_args(); run(a.check, a.overflow_check, a.address_check)
