#!/usr/bin/env python3
"""Compiled-original-C armor evidence. Never consumes Rust gameplay output."""
import argparse,collections,csv,hashlib,io,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3g9_reference as previous
from m3g10_cases import populate
from m3g10_components import SOURCE as COMPONENTS,GRANTS,DAMAGE
from evidence_pack import pack,unpack
core=previous.core
previous.LAYOUTS=populate(previous.pack)
core.ADDED.append(('p_inter.c','P_GiveArmor','boolean'))
stub='boolean P_GiveArmor(player_t*p,int a){(void)p;(void)a;abort();}'
assert previous.extra.count(stub)==1
previous.extra=previous.extra.replace(stub,'boolean P_GiveArmor(player_t*,int);')
needle='if(!p)return 0;'
assert previous.extra.count(needle)==1
previous.extra=previous.extra.replace(needle,needle+'\n if(!strcmp(p,GOTARMOR))return 10;if(!strcmp(p,GOTMEGA))return 11;')
assert core.CAPTURE.count(previous.COMPONENT_SOURCE)==1
core.CAPTURE=core.CAPTURE.replace(previous.COMPONENT_SOURCE,COMPONENTS)
core.CAPTURE+='\nstatic void armor_capture(void){printf("AX,%d,%d,%d,%d\\n",scenario,operation,players[0].armortype,players[0].armorpoints);}\n'
core.DRIVER=core.DRIVER.replace('pickup_capture();','pickup_capture();armor_capture();')
old_body=core.observed_body
def observed_body(file,name,result,rename=None):
    if name=='P_GiveArmor':
        assert rename is None
        text=old_body(file,name,result,'Observed_GiveArmor')
        return text+r'''
boolean P_GiveArmor(player_t*p,int kind){
 int before_type=p->armortype,before_points=p->armorpoints;
 boolean ok=Observed_GiveArmor(p,kind);
 printf("PN,%d,%d,7,%d,%d,%d,%d,%d,%d\n",scenario,operation,kind,ok,before_type,before_points,p->armortype,p->armorpoints);
 return ok;
}
'''
    text=old_body(file,name,result,rename)
    if name=='P_DamageMobj':
        text=text.replace('{','{\n int armor_incoming=damage,armor_before_type=0,armor_before_points=0;',1)
        needle='player = target->player;'
        assert text.count(needle)==1
        text=text.replace(needle,needle+'\n if(player){armor_before_type=player->armortype;armor_before_points=player->armorpoints;}')
        needle='player->health -= damage;'
        assert text.count(needle)==1
        text=text.replace(needle,r'''printf("AR,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,ident(target),armor_incoming,armor_incoming-damage,damage,armor_before_type,armor_before_points,player->armortype,player->armorpoints);
        '''+needle)
    return text
core.observed_body=observed_body
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
    selected={0,1,2,3,4,5,6,7,8,12,13,16,32,64,96,128,256,512,768,1024,1280,2048}
    interesting={(int(r[1]),int(r[2]))for r in rows if r[0]=='AR' or (r[0]=='PN'and r[3]=='7'and r[5]=='1')or(r[0]=='W'and r[3]=='5')}
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==core.CASES[int(r[1])]['tics'])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys or(r[0]=='A'and r[4]=='1'and r[5]in{'43','44'}))]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=('DOOM-PLAYER-ARMOR-v1\n'+text.getvalue()).encode()
    outputs={'m3g10_trace.dref':pack(expanded)}
    assert unpack(outputs['m3g10_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3g10_cases.csv']=text.getvalue().encode()
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(r for r in component_rows if r[0]=='ACOMP')
    outputs['m3g10_components.csv']=text.getvalue().encode()
    # New names do not enter any frozen historical map/provenance inventory glob.
    known=sorted((ROOT/'tests/reference').glob('*map_*.bin'))
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        match=next((p for p in known if p.read_bytes()==m['bytes']),None)
        if match is None:
            match=ROOT/f'tests/reference/m3g10_geometry_{i}.bin';outputs[match.name]=m['bytes']
        maps.append('tests/reference/'+match.name)
    outputs['m3g10_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    manifest=json.loads(observed['m3d3j_provenance.json'])
    bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in previous.pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    known_bodies=set()
    for p in sorted((ROOT/'tests/reference').glob('*provenance.json')):
        for b in json.loads(p.read_text()).get('functions',[]):
            if isinstance(b,list):known_bodies.add(':'.join(b[:2]))
            elif isinstance(b,str):known_bodies.add(b)
            elif isinstance(b,dict)and'source'in b and'name'in b:known_bodies.add(b['source']+':'+b['name'])
    counter=lambda tag,code=None:sum(r[0]==tag and(code is None or r[3]==str(code))for r in kept)
    grants=[r for r in kept if r[0]=='PN'and r[3]=='7']
    armor=[r for r in kept if r[0]=='AR']
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),captures=len(keys),
      grant_components=len(GRANTS),damage_components=len(DAMAGE),repeated_component_hits=6,ignored_after_death_components=2,
      selected_original_bodies=len(bodies),compiled_original_bodies=len(bodies)+3,
      globally_new_bodies=sorted({f'{f}:{n}'for f,n,_ in bodies}-known_bodies),
      exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      component_exercised_unique_functions=len(component_functions),component_function_entries=sum(component_functions.values()),
      actor_fields=26*sum(r[0]=='A'and r[4]=='1'for r in kept),player_fields=6*counter('P'),
      weapon_fields=17*counter('WX'),pickup_fields=3*counter('PX'),armor_fields=2*counter('AX'),
      touches=counter('PN',0),armor_grants=len(grants),accepted_armor_grants=sum(r[5]=='1'for r in grants),
      refused_armor_grants=sum(r[5]=='0'for r in grants),player_damage_requests=len(armor),
      armor_spawn_randomizations=sum(r[0]=='F'and len(r)==13 and int(r[3])>=0 and r[7]in{'2018','2019'}and int(r[10])>int(r[9])for r in kept),
      armor_animation_entries=sum(r[0]=='E'and r[3]=='4'and r[5]in{'802','803','804','805'}for r in kept),
      absorptions=sum(int(r[5])>0 for r in armor),depletions=sum(int(r[7])!=0 and int(r[9])==0 for r in armor),
      pain_action_entries=functions['p_enemy.c:A_Pain'],player_scream_entries=functions['p_enemy.c:A_PlayerScream'],kill_function_entries=functions['p_inter.c:P_KillMobj'],
      component_damage_requests=component_functions['p_inter.c:P_DamageMobj'],component_gameplay_draw_observations=sum(r[0]=='E'and r[3]=='8'for r in component_rows),
      shots=counter('W',5),gameplay_draws=counter('E',8),weapon_draws=counter('W',8),damage_requests=counter('G',2),
      movement_attempts=counter('M'),collision_candidates=counter('C'),source_only_queue_observations=counter('PQ'),
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3g10_trace.dref']))
    # Selected independent full snapshots include source-observed late outcomes.
    full={(0,0),(1,6),(2,6),(8,1),(9,1),(13,1),(17,1),(22,64),(23,128),(25,512),(26,1024),(27,128)}
    for which in [22,23,25,26,27,28]:
        dead=[int(r[2])for r in rows if r[0]=='P'and int(r[1])==which and r[5]=='1']
        if dead:full.add((which,min(dead)))
    for which in [25,26]:
        depleted=[int(r[2])for r in armor if int(r[1])==which and r[7]!='0'and r[9]=='0']
        assert depleted;full.add((which,min(depleted)))
    assert full<=keys
    inputs=['m3g10_reference.py','m3g10_cases.py','m3g10_components.py',*['m3g9_reference.py','m3g9_cases.py','m3g9_observers.py','m3g9_components.py','m3g8_reference.py','m3g7_reference.py','m3g6_reference.py','m3g5_reference.py','m3g4_reference.py','m3g3_reference.py','m3g2_reference.py','m3f3_shim.h','evidence_pack.py']]
    source_names=set(manifest['source_sha256'])|{'d_items.c','dstrings.h'}
    contract=dict(schema='DOOM-PLAYER-ARMOR-v1',revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,maps=maps,functions=bodies,
      full_observation_keys=sorted(full),source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(source_names)},
      function_sha256={f'{f}:{n}':sha(core.raw_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),component_function_entries=dict(sorted(component_functions.items())),
      linked_math_bodies=['FixedMul','FixedDiv','FixedDiv2'],inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       'Genuine player0, at most one MT_SERGEANT, at most16 total world items, actual original ARM1/ARM2 state loops and positive map-tic randomization; no Rust expected observations.',
       'Source armor fields are initialized only by G_PlayerReborn and actual world Touch/GiveArmor in all scripted scenarios. pre_damage is controlled initial damage, not an autonomous hit.',
       'Bounded original command/weapon/bonus fragments precede original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn.',
       'Unticked components explicitly set initial armor/health and damage-phase RNG for isolated source comparisons; physical high inputs outside the Rust travel proof are not implied admitted.',
       'P_DamageMobj observations retain incoming damage for thrust, then record armor saving before health/tactile/mobj consequences; the grounded domain never reaches reverse-thrust RNG.',
       'Inherited explicit operand ordering, signed-shift/BAM interpretation, GNU ABI and logical-retirement quarantine apply. No all-inactive scan, freed-pointer dereference, invalid ammo index or commercial asset.',
       'PQ inactive respawn-queue observations remain source-only. Birth/generation projection and diagnostic schema are independent Rust safety/encoding policy, not C allocator equivalence.'])
    outputs['m3g10_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original armor: '+str(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');args=parser.parse_args();run(args.check,args.overflow_check,args.address_check)
