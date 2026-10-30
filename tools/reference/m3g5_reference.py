#!/usr/bin/env python3
"""Original active-player/Demon observations; no Rust expected values."""
import argparse, hashlib, shutil, subprocess, tempfile
import sys,copy,struct,csv,io,collections,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3g4_reference as commands
main,legacy,geometry,extent=commands.main,commands.legacy,commands.main.geometry,commands.main.extent
CASES=[]
def case(label,tics=64,**kw):
 c=dict(label=label,tics=tics,forward=0,side=0,turn=0,move_hold=0,turn_hold=0,px=256,py=0,reaction=0,thing=1,pre_damage=0,hit_at=-1,hit_amount=0,physical=0,dx=-512,dy=0,thing_angle=0,seed=0,threshold=0,clear_at=-1,retire_at=-1,puff_at=-1,existing=0,see=0,wall=0,rejected=0);c.update(kw);CASES.append(c)
case('active_player_without_demon',tics=16,thing=0)
case('front_acquisition')
case('stop_slot_first_failure',seed=1,tics=32)
case('lastlook_two',seed=3,tics=24)
case('lastlook_three',seed=6,tics=24)
case('behind_far',dx=0,px=-128,tics=32)
case('close_rear_62',dx=0,px=-62,tics=10)
case('rear_boundary_64',dx=0,px=-64,tics=10)
case('outside_rear_65',dx=0,px=-65,tics=32)
case('reject_sight',rejected=1,tics=32)
case('wall_sight',wall=1,dx=-256,tics=32)
case('zero_axis_sight_quirk',wall=2,dx=-256,tics=32)
case('ambush_visual',thing=2,tics=32)
case('threshold_reset',threshold=17,tics=32)
case('moving_player',forward=25,turn=1024,move_hold=64,turn_hold=64)
case('reaction_gate',forward=25,turn=-512,move_hold=12,turn_hold=12,reaction=3)
case('controlled_knockback',forward=25,move_hold=12,hit_at=0,hit_amount=20,physical=1)
case('pursuit',tics=128)
case('dead_active_player',pre_damage=100,tics=48)
case('target_dies',hit_at=24,hit_amount=100,tics=64)
case('target_loss_reacquires',clear_at=24,tics=64)
case('existing_target',existing=1,see=1,tics=48)
case('chase_fallback_allaround',thing_angle=-180,see=1,tics=48)
case('retired_source_reuse',pre_damage=1,retire_at=24,puff_at=28,tics=48)
case('moving_away',forward=25,move_hold=24,tics=80)
main.MAPS,main.THINGS,main.KEYS,main.CASES=[],[],[],[]
for c in CASES:
 m=geometry.fixture(n=1,count=5 if c['wall'] else 4,blockers=bool(c['wall']))
 
 if c['wall']==1:m['vertices'][4:6]=[(16,y)for x,y in m['vertices'][4:6]]
 m['vertices']=[(7*x,14*y)for x,y in m['vertices']];m['lumps']['VERTEXES']=b''.join(geometry.old.words(v)for v in m['vertices']);m['range']=(-896,896);m['center']=(0,0)
 m=extent.grid(m,(-1024,-1024),16,16)
 things=[] if not c['thing']else[(c['dx'],c['dy'],c['thing_angle'],3002,15 if c['thing']==2 else 7)]
 main.case(c['label'],n=1,custom=m,things=things,tics=c['tics'],rejected=bool(c['rejected']),**{'pc_'+k:v for k,v in c.items()if k not in ['label','tics']});c['map']=main.CASES[-1]['map']
commands.CASES=CASES
commands.CAPTURE+=r'''
static void demon_capture(mobj_t* m){if(!m||!alive[ident(m)])return;
printf("DM,%d,%d,%d,%u,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%u\n",scenario,operation,ident(m),m->angle,m->x,m->y,m->momx,m->momy,(long)(m->state-states),m->tics,m->lastlook,m->reactiontime,m->threshold,m->movedir,m->movecount,m->target?ident(m->target):-1,m->health,(unsigned)m->flags);}
'''
commands.DRIVER=r'''
for(c=0;c<CASE_COUNT;c++){
int* v=input[c];mobj_t *body,*origin=NULL;
/* CASE_FILTER */
reset(c);onground=0;prndindex=v[V_PC_SEED];printf("SEED,%d,0,%d\n",c,prndindex);admit_native=0;
playeringame[0]=1;P_LoadThings(0);if(owned_count)origin=owned[0];
printf("F,%d,0,-1,0,0,0,0,0,%d,%d,%d,%d\n",scenario,owned_count,prndindex,totalkills,totalitems);
players[0].health=100;players[0].readyweapon=wp_pistol;
body=P_SpawnMobj(v[V_PC_PX]*FRACUNIT,v[V_PC_PY]*FRACUNIT,ONFLOORZ,MT_PLAYER);associate(body);
if(origin)origin->threshold=v[V_PC_THRESHOLD];
if(v[V_PC_PRE_DAMAGE])P_DamageMobj(body,NULL,origin,v[V_PC_PRE_DAMAGE]);
if(v[V_PC_HIT_AT]==0)P_DamageMobj(body,v[V_PC_PHYSICAL]?origin:NULL,origin,v[V_PC_HIT_AMOUNT]);
body->reactiontime=v[V_PC_REACTION];
if(origin&&v[V_PC_EXISTING]){origin->target=body;observe(6,ident(origin),ident(body));}
if(origin&&v[V_PC_SEE])P_SetMobjState(origin,S_SARG_RUN1);
capture();command_capture();demon_capture(origin);
for(t=1;t<=v[V_TICS];t++){
int before;operation=t;query_index=-1;attempt=-1;
if(!playeringame[0]||!players[0].mo||players[0].mo!=body||body->player!=&players[0])abort();
if(t==v[V_PC_HIT_AT])P_DamageMobj(body,v[V_PC_PHYSICAL]?origin:NULL,origin,v[V_PC_HIT_AMOUNT]);
if(t==v[V_PC_CLEAR_AT]){origin->target=NULL;observe(6,ident(origin),-1);}
if(t==v[V_PC_RETIRE_AT])P_RemoveMobj(origin);
if(t==v[V_PC_PUFF_AT])P_SpawnMobj(-512*FRACUNIT,0,ONFLOORZ,MT_PUFF);
players[0].cmd.forwardmove=t<=v[V_PC_MOVE_HOLD]?v[V_PC_FORWARD]:0;
players[0].cmd.sidemove=t<=v[V_PC_MOVE_HOLD]?v[V_PC_SIDE]:0;
players[0].cmd.angleturn=t<=v[V_PC_TURN_HOLD]?v[V_PC_TURN]:0;
printf("PG,%d,%d,0\n",scenario,operation);
if(players[0].playerstate==PST_LIVE){before=body->reactiontime;source_command_gate(&players[0]);if(!before)printf("PG,%d,%d,1,%d\n",scenario,operation,ident(body));}
printf("PG,%d,%d,2\n",scenario,operation);
P_RunThinkers();leveltime++;capture();command_capture();demon_capture(origin);
}}
return 0;}
'''

def run(check, overflow, address):
    flags = list(legacy.base.FLAGS)
    output = commands.capture(flags); raw = output['m3d3j_trace.csv']
    for enabled, extra in [(overflow, ['-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow']),
                           (address, ['-fsanitize=address,undefined'])]:
        if enabled:
            strict = commands.capture([f for f in flags if f != '-fwrapv'] + ['-fno-wrapv', *extra, '-fno-sanitize-recover=all'])
            assert strict['m3d3j_trace.csv'] == raw
    rows = list(csv.reader(io.StringIO(raw.decode())))
    positive = {i for i, c in enumerate(CASES) if c['turn'] >= 0}
    literal = commands.capture([f for f in flags if f != '-fwrapv'] + ['-fno-wrapv', '-fsanitize=undefined', '-fno-sanitize-recover=all'], True)
    literal_rows = [r for r in csv.reader(io.StringIO(literal['m3d3j_trace.csv'].decode())) if int(r[1]) in positive]
    assert literal_rows == [r for r in rows if int(r[1]) in positive]
    probe = commands.signed_shift_probe()
    functions = collections.Counter(r[3] for r in rows if r[0] == 'FN')
    selected = {0, 1, 2, 3, 4, 6, 7, 8, 10, 11, 12, 16, 24, 32, 40, 48, 64, 72, 96, 128, 255, 256, 257}
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
        roster_fields=4*sum(r[0] == 'R' for r in keep), demon_fields=16*sum(r[0]=='DM' for r in keep),
        scan_observations=sum(r[0]=='K' for r in keep),
        statistics_fields=4*sum(r[0] == 'LS' for r in keep), motion_fields=13*sum(r[0] == 'PM' for r in keep),
        command_fields=4*sum(r[0] == 'UC' for r in keep), command_calls=calls, command_stage_rng_draws=stage_rng,
        gameplay_draws=sum(r[0] == 'E' and r[3] == '8' for r in keep), setup_discards=0,
        movement_attempts=sum(r[0] == 'M' for r in keep), collision_candidates=sum(r[0] == 'C' for r in keep),
        slide_line_observations=sum(r[0] == 'SL' for r in keep), compiled_original_bodies=len(manifest['functions']),
        added_observer_bodies=0, new_engine_source_bodies=0,
        exercised_unique_functions=len(functions), function_entries=sum(functions.values()),
        literal_defined_cases=len(positive), literal_defined_tics=sum(CASES[i]['tics'] for i in positive))
    text = io.StringIO(); csv.writer(text, lineterminator='\n').writerows(keep)
    cases = io.StringIO(); writer = csv.writer(cases, lineterminator='\n')
    writer.writerow(['case', *CASES[0]]);writer.writerows([i, *c.values()] for i, c in enumerate(CASES))
    outputs = {'tests/reference/m3g5_trace.csv': ('DOOM-PLAYER-DEMON-v1'+'\n'+text.getvalue()).encode(),
               'tests/reference/m3g5_cases.csv': cases.getvalue().encode()}
    known = [ROOT/'tests/reference/m3f5_map_0.bin', *sorted((ROOT/'tests/reference').glob('m3g[234]_map_*.bin'))]
    maps = []
    for i, m in enumerate(main.MAPS):
        path = next((p for p in known if p.read_bytes() == m['bytes']), None)
        if path is None:
            path = ROOT/f'tests/reference/m3g5_map_{i}.bin'; outputs[str(path.relative_to(ROOT))] = m['bytes']
        maps.append(str(path.relative_to(ROOT)))
    outputs['tests/reference/m3g5_maps.rs'] = ('// Public synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+
        ''.join(f'    include_bytes!("{Path(p).name}"),\n' for p in maps)+'];\n').encode()
    hashes = {f+':'+n:hashlib.sha256(commands.player._original_body(f,n,r).encode()).hexdigest() for f,n,r in manifest['functions']}
    for k, v in json.loads((ROOT/'tests/reference/m3g3_provenance.json').read_text())['function_sha256'].items():
        assert hashes[k] == v, k
    provenance = dict(schema='DOOM-PLAYER-DEMON-v1', revision=manifest['revision'], flags=flags, inventory=inventory, maps=maps,
        source_sha256={**manifest['source_sha256'], **{f:hashlib.sha256((legacy.SOURCE/f).read_bytes()).hexdigest() for f in ['p_user.c','d_ticcmd.h','d_player.h','g_game.c','p_tick.c','p_sight.c']}},
        functions=manifest['functions'], function_sha256=hashes, executed_function_entries=dict(sorted(functions.items())),
        reaction_fragment_sha256=hashlib.sha256(commands.GATE.encode()).hexdigest(), negative_shift_probe=probe,
        turn_adapter=dict(literal='(cmd->angleturn<<16)', normalized='((uint32_t)(int32_t)cmd->angleturn << 16)',
            normalized_body_sha256=hashlib.sha256(commands.player._original_body('p_user.c','P_MovePlayer','void').replace('(cmd->angleturn<<16)', '((uint32_t)(int32_t)cmd->angleturn << 16)').encode()).hexdigest()),
        inputs_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in ['tools/reference/m3g5_reference.py','tools/reference/m3g4_reference.py','tools/reference/m3g3_reference.py','tools/reference/m3g2_reference.py','tools/reference/m3f3_shim.h']},
        assumptions=[
            'Genuine MT_PLAYER0, MT_SERGEANT12 and fixed active roster [1,0,0,0]. Canonical association is asserted each tic; no alias or copied player state. Death retains roster membership and mo association.',
            'Verbatim P_PlayerThink movement conditional with explicit PST_LIVE driver before P_RunThinkers; not complete PlayerThink/Ticker/DeathThink. No view/powers/weapons/sector effects. Original P_LookForPlayers, A_Look and A_Chase execute against the real roster. Never execute all-inactive original scanning.',
            'Same 76 compiled source bodies as M3g4: zero additions and zero globally new bodies. Function entries count actual scenarios, not branch coverage. Negative turns retain declared unsigned-bit interpretation; literal nonnegative cases and separate intentional UB probe remain qualified.',
            'One source player0, Medium/commercial, grounded static axis walls, no sector sound target or automatic melee. Cases stop before source melee. Controlled between-tic damage/target/state/retirement operations are explicit driver setup, not normal input gameplay.',
            'Source thinker order is Demon before player when present; commands change angle/momentum before thinkers, not player XY before the earlier Demon. Single sgtsit wake sound takes no random draw; shared subsequent chase can draw RNG.',
            'All per-tic PM/UC/DM and decision/event rows retained; selected full actor/player/roster/statistics captures overlap those fields and are counted separately. Original x=0 horizontal sight quirk is distinct from actually blocked x=112 wall. Birth/generation reuse projection is independently specified Rust schema policy, not C allocator identity.',
            'Demon retirement uses the inherited deferred-removal/quarantine adapter. No freed C pointer is dereferenced; the active player body is never retired. Stale/foreign/body-removal rejection remains Rust policy, not source gameplay. No Rust observations generate expected values.'],
        outputs_sha256={p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()})
    outputs['tests/reference/m3g5_provenance.json'] = (json.dumps(provenance, indent=2, sort_keys=True)+'\n').encode()
    for path, data in outputs.items():
        if check: assert (ROOT/path).read_bytes() == data, path
        else: (ROOT/path).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} {'DOOM-PLAYER-DEMON-v1'}: {inventory}; strict={overflow}; ASan/UBSan={address}; bytes={sum(map(len, outputs.values()))}')


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--check', action='store_true')
    p.add_argument('--overflow-check', action='store_true'); p.add_argument('--address-check', action='store_true')
    a = p.parse_args(); run(a.check, a.overflow_check, a.address_check)
