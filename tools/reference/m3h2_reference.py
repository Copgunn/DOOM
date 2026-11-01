#!/usr/bin/env python3
"""Genuine original-C Imp melee/missile choice, fireballs and mixed rosters.

Expectations come from compiled original bodies over synthetic inputs; they
never consume Rust output."""
import argparse,collections,csv,hashlib,io,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3h1_reference as inherited
from m3h2_cases import populate
from m3h2_observers import HELPERS,missile_body,replace_once
from evidence_pack import pack,unpack
previous=inherited.previous;core=inherited.core
import m3g9_reference as pickup_stage
previous.LAYOUTS=populate(previous.pack)
ADDED=[('p_mobj.c','P_SpawnMissile','mobj_t*'),('p_mobj.c','P_CheckMissileSpawn','void'),('p_mobj.c','P_ExplodeMissile','void'),('p_enemy.c','A_TroopAttack','void')]
core.ADDED.extend(ADDED)
ACTION=core.legacy.actors.ACTIONS.index('A_TroopAttack')
needle='printf("PG,%d,%d,0\\n",scenario,operation);'
core.DRIVER=replace_once(core.DRIVER,needle,'if(t==v[V_PC_HIT_AT])P_DamageMobj(owned[v[V_PC_HIT_THING]],NULL,NULL,v[V_PC_HIT_AMOUNT]);\n'
  'if(t==v[V_PC_ATTACK_AT]){mobj_t*a=owned[v[V_PC_ATTACK_THING]];a->target=body;observe(6,ident(a),ident(body));P_SetMobjState(a,S_TROO_ATK1);}\n'+needle)
old_body=core.observed_body
def observed(file,name,result,rename=None):
    return missile_body(name,old_body(file,name,result,rename),ACTION)
core.observed_body=observed
inherited_run=pickup_stage.real_run
def compile_observed(args,*a,**kw):
    if args and args[0]=='cc':
        for x in args:
            if str(x).endswith('reference.c'):
                path=Path(x);text=path.read_text()
                text=replace_once(text,'void P_ExplodeMissile(mobj_t* p){(void)p;abort();}',HELPERS)
                text=replace_once(text,'void A_TroopAttack(mobj_t* actor){(void)actor;abort();}','')
                sarg='if(st->action.acp1==(actionf_p1)A_SargAttack)observe(5,ident(mobj),SARG_ACTION);'
                text=replace_once(text,sarg,f'if(st->action.acp1==(actionf_p1)A_TroopAttack)observe(5,ident(mobj),{ACTION});'+sarg)
                path.write_text(text)
    return inherited_run(args,*a,**kw)
pickup_stage.real_run=compile_observed
sha=lambda b:hashlib.sha256(b).hexdigest()
def run(check,overflow,address):
    flags=list(previous.pack.legacy.base.FLAGS)
    observed_trace,_=previous.capture(flags)
    for enabled,sanitizer in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited,_=previous.capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==observed_trace['m3d3j_trace.csv']
    rows=list(csv.reader(io.StringIO(observed_trace['m3d3j_trace.csv'].decode())))
    functions=collections.Counter(r[3]for r in rows if r[0]=='FN')
    selected={0,1,2,4,8,16,32,48,64,96,128,192,256,320,384,448,512,576,640,704}
    interesting={(int(r[1]),int(r[2]))for r in rows if (r[0]=='G'and r[3]in{'7','10'})or r[0]in{'AR','ST','PN'}or(r[0]=='MS'and r[3]in{'0','5'})or(r[0]=='E'and r[3]=='1'and r[2]!='0')}
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==core.CASES[int(r[1])]['tics'])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=('DOOM-PLAYER-IMPS-v1\n'+text.getvalue()).encode()
    outputs={'m3h2_trace.dref':pack(expanded)}
    assert unpack(outputs['m3h2_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3h2_cases.csv']=text.getvalue().encode()
    known=sorted((ROOT/'tests/reference').glob('*map_*.bin'))+sorted((ROOT/'tests/reference').glob('m3g11_geometry_*.bin'))+sorted((ROOT/'tests/reference').glob('m3h1_geometry_*.bin'))
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        match=next((p for p in known if p.read_bytes()==m['bytes']),None)
        if match is None:
            match=ROOT/f'tests/reference/m3h2_geometry_{i}.bin';outputs[match.name]=m['bytes']
        maps.append('tests/reference/'+match.name)
    outputs['m3h2_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    manifest=json.loads(observed_trace['m3d3j_provenance.json'])
    bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in previous.pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    types={}
    for r in rows:
        if r[0]=='A'and r[4]=='1':types.setdefault((r[1],r[3]),int(r[5]))
    births={(r[1],r[4]):int(r[5])for r in rows if r[0]=='E'and r[3]=='1'}
    kind=lambda r,i:births.get((r[1],r[i]),types.get((r[1],r[i])))
    monster_types={1,2,10,11,12}
    kills=[r for r in kept if r[0]=='G'and r[3]=='7']
    retargets=[r for r in kept if r[0]=='G'and r[3]=='10']
    damages=[r for r in kept if r[0]=='G'and r[3]=='2']
    counter=lambda tag,code=None:sum(r[0]==tag and(code is None or r[3]==str(code))for r in kept)
    missile_damage=[r for r in damages if kind(r,5)==31]
    per_case=[]
    for which,c in enumerate(core.CASES):
        mine=[r for r in kept if r[1]==str(which)]
        per_case.append(dict(label=c['label'],launches=sum(r[0]=='MS'and r[3]=='5'for r in mine),explosions=sum(r[0]=='MS'and r[3]=='0'for r in mine),
          contact_draws=sum(r[0]=='MS'and r[3:5]==['2','3']for r in mine),
          claws=sum(r[0]=='E'and r[3]=='7'and r[5]=='55'for r in mine),kills=sum(r[0]=='G'and r[3]=='7'for r in mine)))
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),captures=len(keys),
      selected_original_bodies=len(bodies),exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      imp_attacks=functions['p_enemy.c:A_TroopAttack'],spawn_missile=functions['p_mobj.c:P_SpawnMissile'],explode_missile=functions['p_mobj.c:P_ExplodeMissile'],
      fireball_launches=counter('MS',5),fireball_explosions=counter('MS',0),fireball_damage_requests=len(missile_damage),
      fireball_player_hits=sum(kind(r,4)==0 for r in missile_damage),fireball_monster_hits=sum(kind(r,4)in monster_types for r in missile_damage),
      monster_kills=sum(kind(r,4)in monster_types for r in kills),imp_kills=sum(kind(r,4)==11 for r in kills),player_deaths=sum(kind(r,4)==0 for r in kills),
      infighting_retargets=sum(kind(r,4)in monster_types and kind(r,5)in monster_types for r in retargets),
      kill_credits=counter('ST',1),actor_fields=26*sum(r[0]=='A'and r[4]=='1'for r in kept),gameplay_draws=counter('E',8),
      damage_requests=len(damages),expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3h2_trace.dref']))
    inputs=sorted({Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    contract=dict(schema='DOOM-PLAYER-IMPS-v1',revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,cases=per_case,maps=maps,functions=bodies,
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(set(manifest['source_sha256'])|{'d_items.c','dstrings.h'})},
      function_sha256={f'{f}:{n}':sha(core.raw_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       'Genuine player0 plus at most eight map-THING MT_TROOP/MT_POSSESSED/MT_SHOTGUY/MT_CHAINGUY/MT_SERGEANT; no Rust expected observations.',
       'A_TroopAttack, P_SpawnMissile, P_CheckMissileSpawn, P_ExplodeMissile, P_XYMovement, P_ZMovement, PIT_CheckThing, P_DamageMobj and P_KillMobj are the compiled original bodies; no raise/resurrection, MT_VILE or nightmare respawn.',
       'One controlled between-tic P_DamageMobj(target,NULL,NULL,amount) exists only where a case declares hit_at; it reaches XDeath, which the admitted in-world damage range cannot. One controlled target assignment plus S_TROO_ATK1 entry exists only where a case declares attack_at; it reaches the blocked P_CheckMissileSpawn path, which needs overlapping placement.',
       'Bounded original command/weapon stage precedes original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn.',
       'All M3h1 assumptions and adapters apply, including the strict-build retyping of attack-angle int locals; MS rows only observe genuine statements.'])
    outputs['m3h2_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original Imps: '+json.dumps(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
    return per_case
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');parser.add_argument('--cases',action='store_true');args=parser.parse_args()
    table=run(args.check,args.overflow_check,args.address_check)
    if args.cases:
        for row in table:print(row)
