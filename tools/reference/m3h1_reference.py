#!/usr/bin/env python3
"""Genuine original-C former humans, mixed rosters, infighting and drops.

Expectations come from compiled original bodies over synthetic inputs; they
never consume Rust output."""
import argparse,collections,csv,hashlib,io,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3g11_reference as inherited
from m3h1_cases import populate
from m3h1_observers import retag_shotgunner,attribute_scream,bam_attack_angles
from evidence_pack import pack,unpack
previous=inherited.previous;core=inherited.core
previous.LAYOUTS=populate(previous.pack)
import m3g9_reference as pickup_stage
real_run=pickup_stage.real_run
def compile_observed(args,*a,**kw):
    if args and args[0]=='cc':
        for x in args:
            if str(x).endswith('reference.c'):
                path=Path(x);text=attribute_scream(retag_shotgunner(path.read_text()))
                if '-fno-wrapv' in args:text=bam_attack_angles(text)
                path.write_text(text)
    return real_run(args,*a,**kw)
# The inherited pickup stage routes every compile through its module real_run.
pickup_stage.real_run=compile_observed
sha=lambda b:hashlib.sha256(b).hexdigest()
def capture(flags):
    return previous.capture(flags)
def run(check,overflow,address):
    flags=list(previous.pack.legacy.base.FLAGS)
    observed,_=capture(flags)
    for enabled,sanitizer in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited,_=capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==observed['m3d3j_trace.csv']
    rows=list(csv.reader(io.StringIO(observed['m3d3j_trace.csv'].decode())))
    functions=collections.Counter(r[3]for r in rows if r[0]=='FN')
    selected={0,1,2,4,8,16,32,48,64,96,128,192,256,320,384,448,512,576,640,704}
    interesting={(int(r[1]),int(r[2]))for r in rows if (r[0]=='G'and r[3]in{'7','10'})or r[0]in{'AR','ST','PN'}or(r[0]=='E'and r[3]=='1'and r[2]!='0')}
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==core.CASES[int(r[1])]['tics'])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=('DOOM-PLAYER-MONSTERS-v1\n'+text.getvalue()).encode()
    outputs={'m3h1_trace.dref':pack(expanded)}
    assert unpack(outputs['m3h1_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3h1_cases.csv']=text.getvalue().encode()
    known=sorted((ROOT/'tests/reference').glob('*map_*.bin'))+sorted((ROOT/'tests/reference').glob('m3g11_geometry_*.bin'))
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        match=next((p for p in known if p.read_bytes()==m['bytes']),None)
        if match is None:
            match=ROOT/f'tests/reference/m3h1_geometry_{i}.bin';outputs[match.name]=m['bytes']
        maps.append('tests/reference/'+match.name)
    outputs['m3h1_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    manifest=json.loads(observed['m3d3j_provenance.json'])
    bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in previous.pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    types={}
    for r in rows:
        if r[0]=='A'and r[4]=='1':types[(r[1],r[3])]=int(r[5])
    kind=lambda r,i:types.get((r[1],r[i]))
    monster_types={1,2,10,12}
    kills=[r for r in kept if r[0]=='G'and r[3]=='7']
    retargets=[r for r in kept if r[0]=='G'and r[3]=='10']
    counter=lambda tag,code=None:sum(r[0]==tag and(code is None or r[3]==str(code))for r in kept)
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),captures=len(keys),
      selected_original_bodies=len(bodies),exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      former_human_attacks={n:functions['p_enemy.c:'+n]for n in['A_PosAttack','A_SPosAttack','A_CPosAttack','A_CPosRefire']},
      demon_attacks=functions['p_enemy.c:A_SargAttack'],
      monster_kills=sum(kind(r,4)in monster_types for r in kills),player_deaths=sum(kind(r,4)==0 for r in kills),
      infighting_retargets=sum(kind(r,4)in monster_types and kind(r,5)in monster_types for r in retargets),
      drops=sum(r[0]=='E'and r[3]=='1'and r[2]!='0'and kind(r,4)in{63,73,77}for r in kept),
      drop_touches=sum(r[0]=='PN'and r[3]=='0'and kind(r,4)in{63,73,77}for r in kept),
      kill_credits=counter('ST',1),actor_fields=26*sum(r[0]=='A'and r[4]=='1'for r in kept),
      monster_shots=sum(r[0]=='X'and r[3]=='0'and kind(r,4)in{1,2,10}for r in kept),gameplay_draws=counter('E',8),
      damage_requests=counter('G',2),expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3h1_trace.dref']))
    inputs=sorted({Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    contract=dict(schema='DOOM-PLAYER-MONSTERS-v1',revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,maps=maps,functions=bodies,
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(set(manifest['source_sha256'])|{'d_items.c','dstrings.h'})},
      function_sha256={f'{f}:{n}':sha(core.raw_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       'Genuine player0 plus at most eight map-THING MT_POSSESSED/MT_SHOTGUY/MT_CHAINGUY/MT_SERGEANT; no Rust expected observations.',
       'Former-human attacks, A_CPosRefire, P_DamageMobj retargeting and P_KillMobj drops are the compiled original bodies; no raise/resurrection, MT_VILE or nightmare respawn.',
       'Bounded original command/weapon stage precedes original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn.',
       'Inherited explicit operand ordering, signed-shift/BAM interpretation, GNU ABI and logical-retirement quarantine apply. A_SPosAttack observation rows are renamed SG only to separate them from player weapon rows; SR rows attribute the existing A_Scream draw.',
       'Original A_PosAttack/A_SPosAttack/A_CPosAttack add spread to int angle locals; the default -fwrapv build keeps that source text, strict signed-overflow and ASan/UBSan audits retype only those locals to angle_t and must reproduce the identical trace.'])
    outputs['m3h1_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original former humans: '+json.dumps(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');args=parser.parse_args();run(args.check,args.overflow_check,args.address_check)
