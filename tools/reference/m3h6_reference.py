#!/usr/bin/env python3
"""Genuine original-C movement, sliding, sight, hitscan, fireballs, noise and pursuit
in rooms of real-map size.

Reuses the compiled M3h2 bodies, observers and controlled inputs unchanged over the
large polygon rooms of m3h6_cases (built by m3h6_geometry's automatic node builder).
Two harness changes only: the shim's map arrays are enlarged to the sizes of these
rooms (the `intercepts[128]` array and its abort guard are kept exactly, so any
129th intercept would abort the sanitizer build), and one observer prints the
intercept count on entry to `P_TraverseIntercepts` (`TI` rows). Expectations come
from compiled original bodies over synthetic inputs; they never consume Rust output."""
import argparse,collections,csv,hashlib,io,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3h2_reference as inherited
import m3h6_geometry as geometry
from m3h6_cases import ROOMS,ROOM_NAMES,WITNESS,populate,MAX_PICKUPS
from m3h2_observers import replace_once
from evidence_pack import pack,unpack
import m3g9_reference as pickup_stage
previous=inherited.previous;core=inherited.core
LAYOUTS=populate(previous.pack)
OOF=34 # sfx_oof in sounds.h
SCHEMA='DOOM-PLAYER-REALMAP-v1'
needle='printf("PG,%d,%d,0\\n",scenario,operation);'
core.DRIVER=replace_once(core.DRIVER,needle,'if(t==v[V_PC_PUSH_AT]){body->momx=v[V_PC_PUSH_X]*FRACUNIT;body->momy=v[V_PC_PUSH_Y]*FRACUNIT;}\n'+needle)
# Harness array sizes for real-map rooms (m3h6 note §7.1); intercepts[128] is untouched.
SHIM_SIZES=[('static line_t line_storage[1024];','static line_t line_storage[4096];'),
 ('static vertex_t vertices[4096];','static vertex_t vertices[16384];'),
 ('static sector_t sector_storage[256];','static sector_t sector_storage[1024];'),
 ('static subsector_t leaf_storage[4096];','static subsector_t leaf_storage[8192];'),
 ('static node_t node_storage[4096];','static node_t node_storage[8192];'),
 ('static mobj_t* block_heads[8192];','static mobj_t* block_heads[16384];'),
 ('static mobj_t* owned[1024];','static mobj_t* owned[4096];'),
 ('static int alive[1024],','static int alive[4096],'),
 ('if(owned_count==1024)abort();','if(owned_count==4096)abort();'),
 ('static side_t side_storage[2048];','static side_t side_storage[8192];'),
 ('static line_t* sector_lines[256][1024];','static line_t* sector_lines[1024][4096];'),
 ('static seg_t seg_storage[8192];','static seg_t seg_storage[16384];'),
 ('static byte reject_storage[8192];','static byte reject_storage[131072];')]
inherited_write=pickup_stage.write
def write(path,text,*a,**kw):
    if path.name=='m3d3j_shim.h':
        assert 'intercept_t intercepts[128],*intercept_p;' in text
        for old,new in SHIM_SIZES:
            assert text.count(old)==1,old;text=text.replace(old,new)
    return inherited_write(path,text,*a,**kw)
pickup_stage.write=write
inherited_observed=core.observed_body
def observed(file,name,result,rename=None):
    text=inherited_observed(file,name,result,rename)
    if name=='P_TraverseIntercepts':
        text=replace_once(text,'    count = intercept_p - intercepts;','    count = intercept_p - intercepts;printf("TI,%d,%d,%d,%d\\n",scenario,operation,trace_stage,count);')
    return text
core.observed_body=observed
def configure():
    core.main.MAPS,core.main.THINGS,core.main.KEYS,core.main.CASES=[],[],[],[]
    for c in core.CASES:
        sectors,portals,reject,tree,segment=ROOMS[c['room']]
        m=geometry.room(sectors,portals=portals,reject=reject,tree=tree,segment=segment)
        core.main.case(c['label'],n=1,custom=m,things=LAYOUTS[c['layout']],tics=c['tics'],rejected=bool(c['rejected']),
                       **{'pc_'+k:v for k,v in c.items()if k not in['label','tics','map']})
        c['map']=core.main.CASES[-1]['map']
core.configure=configure
inherited_run=pickup_stage.real_run
def compile_subsectors(args,*a,**kw):
    """The inherited loader gives every sector one subsector; install the BSP leaves instead."""
    if args and args[0]=='cc':
        for x in args:
            if str(x).endswith('reference.c'):
                path=Path(x);text=path.read_text()
                for which,m in enumerate(core.main.MAPS):
                    leaves=''.join(f'subsectors[{k}].sector=&sectors[{s}];'for k,s in enumerate(m['leaves']))
                    text=replace_once(text,f'case {which}:numsubsectors={len(m["heights"])};',f'case {which}:numsubsectors={len(m["leaves"])};'+leaves)
                path.write_text(text)
    return inherited_run(args,*a,**kw)
pickup_stage.real_run=compile_subsectors
sha=lambda b:hashlib.sha256(b).hexdigest()
def parity(point,sectors):
    """Odd when the point is inside a sector ring (half-open horizontal ray, exact)."""
    x,y=point;crossings=0
    for rings,_,_ in sectors:
        for ring in rings:
            for i,a in enumerate(ring):
                b=ring[(i+1)%len(ring)]
                if (a[1]>y)!=(b[1]>y)and (b[0]-a[0])*(y-a[1])*(1 if b[1]>a[1]else -1)>(x-a[0])*abs(b[1]-a[1]):crossings+=1
    return crossings&1
def traces(rows):
    """Every trace of the run: (case, tic, traverser kind, intercept rows, TI count),
    opened by the TR row of each P_PathTraverse entry (kind 0 aim, 1 shoot, 2 slide,
    3 use) and closed by its TI row."""
    out=[];current=None
    for r in rows:
        if r[0]=='TR':
            current=dict(case=int(r[1]),tic=int(r[2]),stage=int(r[4]),intercepts=[],count=None);out.append(current)
        elif r[0]=='I' and current is not None:current['intercepts'].append((int(r[4]),int(r[5])))
        elif r[0]=='TI' and current is not None:current['count']=int(r[4]);current=None
    return out
def run(check,overflow,address):
    flags=list(previous.pack.legacy.base.FLAGS)
    observed_trace,_=previous.capture(flags)
    for enabled,sanitizer in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited,_=previous.capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==observed_trace['m3d3j_trace.csv']
    rows=list(csv.reader(io.StringIO(observed_trace['m3d3j_trace.csv'].decode())))
    functions=collections.Counter(r[3]for r in rows if r[0]=='FN')
    selected={0,1,2,4,8,16,32,48,64,96,128,192,256,320,384,448,512,576,640,704,768,832,896,960}
    diagonal={i:{j for j,l in enumerate(m['lines'])if m['vertices'][l['a']][0]!=m['vertices'][l['b']][0]and m['vertices'][l['a']][1]!=m['vertices'][l['b']][1]}for i,m in enumerate(core.main.MAPS)}
    case_map=[c['map']for c in core.CASES]
    landed=lambda r:r[0]=='E'and r[3]=='7'and r[5]==str(OOF)
    contact={};changes=set()
    for r in rows:
        if r[0]=='Z':
            key=(r[1],r[3])
            if contact.get(key)is not None and contact[key]!=r[10]:changes.add((int(r[1]),int(r[2])))
            contact[key]=r[10]
    pushes={(i,c['push_at'])for i,c in enumerate(core.CASES)if c['push_at']}
    # Every slide, blocked move, damage, kill, retarget, launch, explosion, drop, pickup, hitscan,
    # noise flood, landing, height refusal, floor-contact change and controlled push is captured.
    interesting={(int(r[1]),int(r[2]))for r in rows if (r[0]=='G'and r[3]in{'2','7','10'})or r[0]in{'AR','ST','PN','SL','X','N'}or(r[0]=='MS'and r[3]in{'0','5'})or(r[0]=='E'and r[3]=='1'and r[2]!='0')or landed(r)or(r[0]=='M'and r[7]=='0')}|changes|pushes
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==core.CASES[int(r[1])]['tics'])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=(SCHEMA+'\n'+text.getvalue()).encode()
    outputs={'m3h6_trace.dref':pack(expanded)}
    assert unpack(outputs['m3h6_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3h6_cases.csv']=text.getvalue().encode()
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        name=f'm3h6_geometry_{i}.bin';outputs[name]=m['bytes'];maps.append('tests/reference/'+name)
    outputs['m3h6_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    # Rust-only: the intercept-witness hall (m3h6_cases.witness), built by the same
    # builder but never run in C, where it would abort at the 129th intercept.
    witness_room=geometry.room(*WITNESS[:1],portals=WITNESS[1],reject=WITNESS[2],tree=WITNESS[3],segment=WITNESS[4])
    outputs['m3h6_witness.bin']=witness_room['bytes']
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
    reasons=collections.Counter(r[12]for r in kept if r[0]=='M'and r[7]=='0')
    zrows=[r for r in kept if r[0]=='Z']
    live=collections.Counter()
    for r in kept:
        if r[0]=='A'and r[4]=='1'and int(r[5])==31:live[(r[1],r[2])]+=1
    # A player capture whose center has even parity against its room's rings is outside the room.
    void=[r for r in kept if r[0]=='A'and r[4]=='1'and int(r[5])==0 and not parity((int(r[6])/65536,int(r[7])/65536),ROOMS[core.CASES[int(r[1])]['room']][0])]
    # Intercepts per trace: the TI count must equal the I rows of its trace (the
    # source's own intercept_p - intercepts), and never reach 128.
    all_traces=traces(rows)
    for t in all_traces:assert t['count']==len(t['intercepts']),t
    assert all(t['count']<128 for t in all_traces)
    stage_names={0:'aim',1:'shoot',2:'slide',3:'use'}
    by_stage={name:max((t['count']for t in all_traces if t['stage']==s),default=0)for s,name in stage_names.items()}
    readded=sum(len(t['intercepts'])-len(set(t['intercepts']))for t in all_traces)
    lines_max=max((sum(1 for i in t['intercepts']if i[0]==1)for t in all_traces),default=0)
    things_max=max((sum(1 for i in t['intercepts']if i[0]==0)for t in all_traces),default=0)
    # Sight queries per tic (the per-tic budget evidence of the M3h6 note §4.6).
    queries=collections.Counter((r[1],r[2])for r in rows if r[0]=='Q')
    per_case=[]
    for which,c in enumerate(core.CASES):
        mine=[r for r in kept if r[1]==str(which)]
        sectors,portals,reject,tree,segment=ROOMS[c['room']]
        mine_z=[r for r in mine if r[0]=='Z']
        player=[r for r in mine if r[0]=='A'and r[4]=='1'and int(r[5])==0]
        own=[t for t in all_traces if t['case']==which]
        m=core.main.MAPS[c['map']]
        per_case.append(dict(label=c['label'],room=ROOM_NAMES[c['room']],
          map=dict(linedefs=len(m['lines']),sectors=len(m['heights']),subsectors=len(m['leaves']),nodes=len(m['nodes']),cells=m['cells'],words=len(m['blocks'])),
          push=[c['push_at'],c['push_x'],c['push_y']],
          launches=sum(r[0]=='MS'and r[3]=='5'for r in mine),explosions=sum(r[0]=='MS'and r[3]=='0'for r in mine),
          damage=sum(r[0]=='G'and r[3]=='2'for r in mine),kills=sum(r[0]=='G'and r[3]=='7'for r in mine),
          slides=sum(r[0]=='SL'for r in mine),blocked_moves=sum(r[0]=='M'and r[7]=='0'for r in mine),
          refusals={k:sum(r[0]=='M'and r[7]=='0'and r[12]==k for r in mine)for k in['1','2','3','4','5','6']},
          vertical_steps=len(mine_z),landings=sum(landed(r)for r in mine),
          sight_queries=sum(r[0]=='Q'for r in mine),sight_visible=sum(r[0]=='Q'and r[4]=='1'for r in mine),
          max_sight_queries_per_tic=max((n for (s,_),n in queries.items()if s==str(which)),default=0),
          hitscans=sum(r[0]=='X'and r[3]=='0'for r in mine),hitscan_line_impacts=sum(r[0]=='X'and r[3]=='1'and r[4]=='1'for r in mine),
          noise_floods=sum(r[0]=='N'for r in mine),
          traces=len(own),max_intercepts=max((t['count']for t in own),default=0),
          max_intercepts_by_stage={name:max((t['count']for t in own if t['stage']==s),default=0)for s,name in stage_names.items()},
          pickups=sum(r[0]=='PN'for r in mine),
          player_final=[int(player[-1][6]),int(player[-1][7])]if player else None,
          player_void_captures=sum(r[1]==str(which)for r in void)))
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),captures=len(keys),rooms=len(core.main.MAPS),
      sectors=sum(len(m['heights'])for m in core.main.MAPS),linedefs=sum(len(m['lines'])for m in core.main.MAPS),
      largest_room_linedefs=max(len(m['lines'])for m in core.main.MAPS),largest_room_sectors=max(len(m['heights'])for m in core.main.MAPS),
      largest_blockmap_cells=max(m['cells']for m in core.main.MAPS),longest_line_norm=max(abs(m['vertices'][l['b']][0]-m['vertices'][l['a']][0])+abs(m['vertices'][l['b']][1]-m['vertices'][l['a']][1])for m in core.main.MAPS for l in m['lines']),
      largest_slide_window=max(geometry.window_lines(m)for m in core.main.MAPS),
      two_sided_lines=sum(l['back']is not None for m in core.main.MAPS for l in m['lines']),
      diagonal_lines=sum(len(d)for d in diagonal.values()),
      bsp_nodes=sum(len(m['nodes'])for m in core.main.MAPS),subsectors=sum(len(m['leaves'])for m in core.main.MAPS),
      selected_original_bodies=len(bodies),exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      point_in_subsector=functions['r_main.c:R_PointInSubsector'],box_on_line_side=functions['p_maputl.c:P_BoxOnLineSide'],
      hit_slide_line=functions['p_map.c:P_HitSlideLine'],slide_entries=functions['p_map.c:P_SlideMove'],
      path_traverse=functions['p_maputl.c:P_PathTraverse'],traverse_intercepts=functions['p_maputl.c:P_TraverseIntercepts'],
      cross_subsector=functions['p_sight.c:P_CrossSubsector'],recursive_sound=functions['p_enemy.c:P_RecursiveSound'],new_chase_dir=functions['p_enemy.c:P_NewChaseDir'],
      traces=len(all_traces),max_intercepts_per_trace=max((t['count']for t in all_traces),default=0),max_intercepts_by_stage=by_stage,
      max_line_intercepts_per_trace=lines_max,max_thing_intercepts_per_trace=things_max,thing_readds=readded,
      traces_with_at_least_64_intercepts=sum(t['count']>=64 for t in all_traces),
      max_sight_queries_per_tic=max(queries.values(),default=0),
      slide_observations=counter('SL'),blocked_moves=sum(reasons.values()),refusal_reasons={k:reasons[k]for k in['1','2','3','4','5','6']},
      vertical_steps=len(zrows),landing_sounds=sum(landed(r)for r in kept),
      fireball_launches=counter('MS',5),fireball_explosions=counter('MS',0),
      fireball_damage_requests=len(missile_damage),fireball_player_hits=sum(kind(r,4)==0 for r in missile_damage),
      sight_queries=counter('Q'),sight_visible=sum(r[0]=='Q'and r[4]=='1'for r in kept),sight_rejected=sum(r[0]=='V'and r[4]=='0'and r[8]=='1'for r in kept),
      hitscans=sum(r[0]=='X'and r[3]=='0'for r in kept),hitscan_line_impacts=sum(r[0]=='X'and r[3]=='1'and r[4]=='1'for r in kept),
      noise_floods=counter('N'),pickups=counter('PN'),
      monster_kills=sum(kind(r,4)in monster_types for r in kills),player_deaths=sum(kind(r,4)==0 for r in kills),
      infighting_retargets=sum(kind(r,4)in monster_types and kind(r,5)in monster_types for r in retargets),kill_credits=counter('ST',1),
      drops=sum(r[0]=='E'and r[3]=='1'and r[2]!='0'and kind(r,4)in{63,73,77}for r in kept),damage_requests=len(damages),
      controlled_pushes=len(pushes),player_void_captures=len(void),
      peak_live_fireballs=max(live.values(),default=0),actor_fields=26*sum(r[0]=='A'and r[4]=='1'for r in kept),gameplay_draws=counter('E',8),
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3h6_trace.dref']))
    inputs=sorted({Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    contract=dict(schema=SCHEMA,revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,cases=per_case,maps=maps,functions=bodies,
      rust_only_maps=dict(witness=dict(path='tests/reference/m3h6_witness.bin',linedefs=len(witness_room['lines']),slide_window=geometry.window_lines(witness_room),
                                       note='65 thin teeth on one wall: a 2,048-unit hitscan straddles 130 lines; never run in C')),
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(set(manifest['source_sha256'])|{'d_items.c','dstrings.h'})},
      function_sha256={f'{f}:{n}':sha(core.raw_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       f'Genuine player0 plus at most eight map-THING MT_TROOP/MT_POSSESSED/MT_SHOTGUY/MT_CHAINGUY/MT_SERGEANT and at most {MAX_PICKUPS} map-THING pickups in polygon rooms of real-map size (up to 516 linedefs, 74 sectors, 182 subsectors, a 60 x 60 cell BLOCKMAP, a 3,850-norm wall, 7,552-unit spans); no Rust expected observations.',
       'Every compiled body, observer and controlled input of M3h2/M3h5 is reused unchanged. The rooms come from the M3h5 polygon builder with an automatic recursive node builder (DoomBSP grade, split vertices rounded to integers, leaves asserted single-sector and convex within rounding), not from an original level loader or node builder. The adapter installs sector line lists, sidedef sectors, segs, subsectors with their leaf sectors, node partitions and REJECT from the generated lumps.',
       'The harness shim arrays for lines, sides, sectors, segs, subsectors, nodes, BLOCKMAP cells, REJECT bytes and owned objects are enlarged to hold these rooms; intercept_t intercepts[128] and the abort guard before its 129th entry are kept exactly, so every trace observed here holds fewer than 128 intercepts and an overflow would have aborted the run. No source overflow of MAXINTERCEPTS is observed or claimed.',
       'TI rows print intercept_p - intercepts on entry to P_TraverseIntercepts (an observer, not a source change); the count equals the I rows of its trace. Things are added once per visit of their cell, never deduplicated (mobj_t::validcount is never read); thing_readds counts repeated (trace, thing) entries.',
       'One harness-only controlled input exists where a case declares push_at (none here); hit_* and attack_* keep their M3h2 meaning.',
       'Slides, blocked moves, hitscans with dozens of intercepts, long sight across a 2,683-unit two-sided line and down a 6,784-unit corridor, fireballs over 2,100 units, noise floods across 74 sectors and pursuit across many sectors are source observations of the compiled bodies; their admission in Rust is the new policy, not a source claim. No line or sector special, moving sector or sky ceiling exists in any room.',
       'player_void_captures counts captured player positions whose center has even ray parity against the room rings (outside every sector), computed by this script from the trace, not by any original body.',
       'Bounded original command/weapon stage precedes original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn.',
       'All M3h1/M3h2/M3h4/M3h5 assumptions and adapters apply, including the strict-build retyping of attack-angle int locals; MS/SL rows only observe genuine statements.'])
    outputs['m3h6_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original real-map rooms: '+json.dumps(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
    return per_case
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');parser.add_argument('--cases',action='store_true');args=parser.parse_args()
    table=run(args.check,args.overflow_check,args.address_check)
    if args.cases:
        for row in table:print(json.dumps(row))
