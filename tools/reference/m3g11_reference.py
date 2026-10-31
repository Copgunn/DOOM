#!/usr/bin/env python3
"""Genuine original-C bonuses/statistics; expectations never consume Rust output."""
import argparse,collections,copy,csv,hashlib,io,json,struct,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3g10_reference as inherited
from m3g11_cases import populate
from m3g11_components import SOURCE,TOUCH
from m3g11_observers import instrument
from evidence_pack import pack,unpack
previous=inherited.previous;core=inherited.core
previous.LAYOUTS=populate(previous.pack)
instrument(core,previous)
needle='if(v[V_PC_PRE_DAMAGE])'
assert core.DRIVER.count(needle)==1
core.DRIVER=core.DRIVER.replace(needle,'if(v[V_PC_SCRIPT]==30||v[V_PC_SCRIPT]==31){p->health=body->health=200;if(v[V_PC_SCRIPT]==31){p->armortype=1;p->armorpoints=200;}}\n'+needle)
assert core.CAPTURE.count(inherited.COMPONENTS)==1
core.CAPTURE=core.CAPTURE.replace(inherited.COMPONENTS,SOURCE)
sha=lambda b:hashlib.sha256(b).hexdigest()
def run(check,overflow,address):
    flags=list(previous.pack.legacy.base.FLAGS)
    observed,components=previous.capture(flags)
    for enabled,sanitizer in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited,ac=previous.capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==observed['m3d3j_trace.csv']
            assert ac==components
    rows=list(csv.reader(io.StringIO(observed['m3d3j_trace.csv'].decode())))
    component_rows=list(csv.reader(io.StringIO(components.decode())))
    functions=collections.Counter(r[3]for r in rows if r[0]=='FN')
    component_functions=collections.Counter(r[3]for r in component_rows if r[0]=='FN')
    selected={0,1,2,3,4,5,6,7,8,12,13,16,32,36,37,64,72,73,96,128,256,512,768,1024,1280,1536}
    interesting={(int(r[1]),int(r[2]))for r in rows if r[0]=='AR' or (r[0]=='PN'and r[3]=='7'and r[5]=='1')or(r[0]=='W'and r[3]=='5')}
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==core.CASES[int(r[1])]['tics'])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys or(r[0]=='A'and r[4]=='1'and r[5]in{'43','44','45','46'}))]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=('DOOM-PLAYER-BONUSES-v1\n'+text.getvalue()).encode()
    outputs={'m3g11_trace.dref':pack(expanded),'m3g11_components.dref':pack(components)}
    assert unpack(outputs['m3g11_components.dref'])==components
    assert unpack(outputs['m3g11_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3g11_cases.csv']=text.getvalue().encode()
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(r for r in component_rows if r[0]=='BCOMP')
    outputs['m3g11_components.csv']=text.getvalue().encode()
    # New names do not enter any frozen historical map/provenance inventory glob.
    known=sorted((ROOT/'tests/reference').glob('*map_*.bin'))
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        match=next((p for p in known if p.read_bytes()==m['bytes']),None)
        if match is None:
            match=ROOT/f'tests/reference/m3g11_geometry_{i}.bin';outputs[match.name]=m['bytes']
        maps.append('tests/reference/'+match.name)
    outputs['m3g11_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    # Component geometry mirrors the declared direct C spawn calls on reset(0).
    # These maps are inputs for Touch/Damage components, not extra full timelines.
    component_maps={}
    for doomed in sorted({r[0] for r in TOUCH}):
        m=copy.deepcopy(core.main.MAPS[core.CASES[0]['map']])
        things=[(-64,0,0,3002,7),(36,0,0,doomed,7)]
        m['lumps']['THINGS']=b''.join(struct.pack('<5h',*t) for t in things)
        m['bytes']=core.main.geometry.old.wad(list(m['lumps'].items()))
        name=f'm3g11_component_geometry_{doomed}.bin';outputs[name]=m['bytes']
        component_maps[doomed]='tests/reference/'+name
    outputs['m3g11_component_maps.rs']=('// Declared direct-C component spawn inputs; GPL-2.0-only.\n'
      +'pub const COMPONENT_MAPS: &[(i16, &[u8])] = &[\n'
      +''.join(f'    ({kind}, include_bytes!("{Path(path).name}")),\n' for kind,path in component_maps.items())+'];\n').encode()
    manifest=json.loads(observed['m3d3j_provenance.json'])
    bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in previous.pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    known_bodies=set()
    for p in sorted((ROOT/'tests/reference').glob('*provenance.json'))+[ROOT/'tests/reference/m3g10_contract.json']:
        for b in json.loads(p.read_text()).get('functions',[]):
            if isinstance(b,list):known_bodies.add(':'.join(b[:2]))
            elif isinstance(b,str):known_bodies.add(b)
            elif isinstance(b,dict)and'source'in b and'name'in b:known_bodies.add(b['source']+':'+b['name'])
    counter=lambda tag,code=None:sum(r[0]==tag and(code is None or r[3]==str(code))for r in kept)
    grants=[r for r in kept if r[0]=='PN'and r[3]=='7']
    armor=[r for r in kept if r[0]=='AR']
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),captures=len(keys),
      ordinary_world_scenarios=29,sustained_encounters=6,controlled_resource_timelines=2,boundary_components=len(TOUCH),capped_repeat_observations=4,depletion_reacquisition_observations=4,
      selected_original_bodies=len(bodies),compiled_original_bodies=len(bodies)+3,
      globally_new_bodies=sorted({f'{f}:{n}'for f,n,_ in bodies}-known_bodies),
      exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      component_exercised_unique_functions=len(component_functions),component_function_entries=sum(component_functions.values()),
      actor_fields=26*sum(r[0]=='A'and r[4]=='1'for r in kept),player_fields=6*counter('P'),
      weapon_fields=17*counter('WX'),pickup_fields=3*counter('PX'),armor_fields=2*counter('AX'),
      touches=counter('PN',0),armor_grants=len(grants),accepted_armor_grants=sum(r[5]=='1'for r in grants),
      refused_armor_grants=sum(r[5]=='0'for r in grants),player_damage_requests=len(armor),
      bonus_spawn_randomizations=sum(r[0]=='F'and len(r)==13 and int(r[3])>=0 and r[7]in{'2014','2015'}and int(r[10])>int(r[9])for r in kept),
      bonus_animation_entries=sum(r[0]=='E'and r[3]=='4'and 816<=int(r[5])<=827 for r in kept),
      absorptions=sum(int(r[5])>0 for r in armor),depletions=sum(int(r[7])!=0 and int(r[9])==0 for r in armor),
      pain_action_entries=functions['p_enemy.c:A_Pain'],player_scream_entries=functions['p_enemy.c:A_PlayerScream'],kill_function_entries=functions['p_inter.c:P_KillMobj'],
      component_damage_requests=component_functions['p_inter.c:P_DamageMobj'],health_bonus_effects=counter('PN',8),armor_bonus_effects=counter('PN',9),item_count_increments=counter('PN',10),map_item_increments=counter('IT'),component_gameplay_draw_observations=sum(r[0]=='E'and r[3]=='8'for r in component_rows),
      shots=counter('W',5),gameplay_draws=counter('E',8),weapon_draws=counter('W',8),damage_requests=counter('G',2),
      movement_attempts=counter('M'),collision_candidates=counter('C'),source_only_queue_observations=counter('PQ'),
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3g11_trace.dref']))
    # Selected independent full snapshots include source-observed late outcomes.
    full={(30,1),(30,2),(28,0),(28,1),(29,0),(29,1),(29,6),(0,0),(1,0),(1,6),(1,36),(1,37),(4,6),(4,36),(7,37),(11,1),(12,1),(13,1),(14,64),(15,1),(19,64),(20,128),(21,64),(22,128),(23,128),(24,512),(25,1),(26,24)}
    for which in [20,22,24,25,26]:
        dead=[int(r[2])for r in rows if r[0]=='P'and int(r[1])==which and r[5]=='1']
        if dead:full.add((which,min(dead)))
    for which in [24]:
        depleted=[int(r[2])for r in armor if int(r[1])==which and r[7]!='0'and r[9]=='0']
        assert depleted;full.add((which,min(depleted)))
    assert full<=keys
    inputs=['m3g11_reference.py','m3g11_cases.py','m3g11_components.py','m3g11_observers.py','m3g10_reference.py','m3g10_cases.py','m3g10_components.py',*['m3g9_reference.py','m3g9_cases.py','m3g9_observers.py','m3g9_components.py','m3g8_reference.py','m3g7_reference.py','m3g6_reference.py','m3g5_reference.py','m3g4_reference.py','m3g3_reference.py','m3g2_reference.py','m3f3_shim.h','evidence_pack.py']]
    inputs=sorted(set(inputs)|{Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    source_names=set(manifest['source_sha256'])|{'d_items.c','dstrings.h'}
    contract=dict(component_maps=component_maps,schema='DOOM-PLAYER-BONUSES-v1',revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,maps=maps,functions=bodies,
      full_observation_keys=sorted(full),source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(source_names)},
      function_sha256={f'{f}:{n}':sha(core.raw_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),component_function_entries=dict(sorted(component_functions.items())),
      linked_math_bodies=['FixedMul','FixedDiv','FixedDiv2'],inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       'Genuine player0, at most one MT_SERGEANT, at most16 total world items, actual BON1/BON2 six-state loops, original map timer randomization and COUNTITEM fields; no Rust expected observations.',
       'All29 ordinary scripted worlds obtain bonuses from genuine contact. Two labelled component timelines explicitly initialize resources200 before actual movement contacts; they are not reachable encounter setup. pre_damage is controlled initial damage, not an autonomous hit. Unticked resource boundaries are separately declared components.',
       'Bounded original command/weapon/bonus fragments precede original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn.',
       'Unticked components explicitly set initial armor/health, call actual source Touch, then optionally reset damage-phase RNG and call actual physical DamageMobj. Initial states unreachable within16 world items remain component observations, not encounter setup.',
       'P_DamageMobj observations retain incoming damage for thrust, then record armor saving before health/tactile/mobj consequences; the grounded domain never reaches reverse-thrust RNG.',
       'Inherited explicit operand ordering, signed-shift/BAM interpretation, GNU ABI and logical-retirement quarantine apply. No all-inactive scan, freed-pointer dereference, invalid ammo index or commercial asset.',
       'PQ inactive respawn-queue observations remain source-only. Birth/generation projection and diagnostic schema are independent Rust safety/encoding policy, not C allocator equivalence.'])
    outputs['m3g11_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original bonuses: '+str(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');args=parser.parse_args();run(args.check,args.overflow_check,args.address_check)
