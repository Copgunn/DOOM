#!/usr/bin/env python3
"""Compile preserved actor tables; independently emit source metadata and traces."""
import argparse,csv,hashlib,io,json,re,subprocess,tempfile,platform
from pathlib import Path
import generate as base
ROOT,SOURCE,FLAGS=base.ROOT,base.SOURCE,base.FLAGS

def names(end,prefix,file='info.h'):
    text=(SOURCE/file).read_text();pos=text.index(end)
    start=text.rfind('typedef enum',0,pos)
    return re.findall(r'\b'+prefix+r'\w+\b',text[start:pos])
STATE_NAMES=names('NUMSTATES','S_')
TYPE_NAMES=names('NUMMOBJTYPES','MT_')
SPRITE_NAMES=names('NUMSPRITES','SPR_')
SOUND_NAMES=names('NUMSFX','sfx_', 'sounds.h')
ACTIONS=['None']+sorted(set(re.findall(r'\{(A_\w+)\}',(SOURCE/'info.c').read_text())))
FIELDS=['doomednum','spawnstate','spawnhealth','seestate','seesound','reactiontime','attacksound','painstate','painchance','painsound','meleestate','missilestate','deathstate','xdeathstate','deathsound','speed','radius','height','mass','damage','activesound','flags','raisestate']

def metadata():
    fragments=['#include <stdio.h>\n#include <stdlib.h>\n#include "info.c"\n']
    fragments += [f'void {name}(mobj_t* actor){{(void)actor;abort();}}' for name in ACTIONS[1:]]
    for var,values in [('state_names',STATE_NAMES),('type_names',TYPE_NAMES)]:fragments.append('const char* '+var+'[]={'+','.join(json.dumps(s) for s in values)+'};')
    fragments.append('int action_id(state_t* s){if(!s->action.acv)return 0;')
    fragments += [f'if(s->action.acv==(actionf_v){name})return {i};' for i,name in enumerate(ACTIONS) if i]
    fragments.append('abort();return -1;}\nint main(int argc,char**argv){int i;if(NUMSTATES!=sizeof(state_names)/sizeof(state_names[0])||NUMMOBJTYPES!=sizeof(type_names)/sizeof(type_names[0]))abort();if(argc!=2)return 2;')
    fragments.append('if(argv[1][0]==\'s\'){puts("index,name,sprite,frame,tics,action,next,misc1,misc2");for(i=0;i<NUMSTATES;i++){state_t* s=&states[i];printf("%d,%s,%d,%d,%d,%d,%d,%d,%d\\n",i,state_names[i],s->sprite,s->frame,s->tics,action_id(s),s->nextstate,s->misc1,s->misc2);}}else{')
    fragments.append('puts("index,name,'+','.join(FIELDS)+'");for(i=0;i<NUMMOBJTYPES;i++){mobjinfo_t* t=&mobjinfo[i];printf("%d,%s'+',%d'*len(FIELDS)+'\\n",i,type_names[i],'+','.join('t->'+f for f in FIELDS)+');}}return 0;}')
    with tempfile.TemporaryDirectory(prefix='doom-m3c-tables-') as tmp:
        src=Path(tmp)/'tables.c';src.write_text('\n'.join(fragments));binary=Path(tmp)/'tables'
        subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),str(src),'-o',str(binary)],check=True)
        return {kind:subprocess.check_output([str(binary),kind]) for kind in ['states','types']}


def traces():
    text=(ROOT/'tools/reference/m3c_driver.c').read_text()
    bodies=[]
    for name,result in [('P_InitThinkers','void'),('P_AddThinker','void'),('P_RemoveThinker','void'),('P_RunThinkers','void')]:
        b=base.body('p_tick.c',name,result)
        if name=='P_RemoveThinker':b=b.replace('// FIXME: NOP.','event(2,ident(thinker),-1);')
        if name=='P_RunThinkers':b=b.replace('currentthinker->function.acp1 (currentthinker);','{event(0,ident(currentthinker),-1);currentthinker->function.acp1 (currentthinker);}')
        bodies.append(b)
    for name,result in [('P_SetMobjState','boolean'),('P_MobjThinker','void'),('P_SpawnMobj','mobj_t*'),('P_RemoveMobj','void'),('P_SpawnMapThing','void')]:
        b=base.body('p_mobj.c',name,result)
        if name=='P_SetMobjState':b=re.sub(r'(do\s*\{)',r'\1\n event(4,ident(mobj),state);',b,count=1)
        if name=='P_SpawnMobj':b=b.replace('return mobj;','event(1,ident(mobj),type);return mobj;')
        bodies.append(b)
    supported=['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']
    for action in ACTIONS[1:]:
        if action in supported:
            b=base.body('p_enemy.c',action,'void')
            if action=='A_Scream':b=b.replace('P_Random ()','action_random(actor)')
            b=re.sub(r'(\{)',r'\1\n event(5,ident('+('mo' if action=='A_PlayerScream' else 'actor')+'),'+str(ACTIONS.index(action))+');',b,count=1)
        else:b=f'void {action}(mobj_t* actor){{(void)actor;abort();}}'
        bodies.append(b)
    text=text.replace('RNG_TABLE',','.join(map(str,base.table('m_random.c',r'unsigned char\s+rndtable\[256\]')))).replace('/* SOURCE_BODIES */','\n'.join(bodies))
    with tempfile.TemporaryDirectory(prefix='doom-m3c-traces-') as tmp:
        src=Path(tmp)/'traces.c';src.write_text(text);binary=Path(tmp)/'traces'
        subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),str(src),'-o',str(binary)],check=True)
        return subprocess.check_output([str(binary)])

def run(check):
    tables=metadata();trace=traces();outputs={f'tests/reference/m3c_{kind}.csv':data for kind,data in tables.items()}
    outputs['tests/reference/m3c_trace.csv']=trace
    rust=['// Generated from preserved id Software Linux DOOM 1.10; GPL-2.0-only.','// Reproduce with python3 tools/reference/generate_m3c.py.']
    for var,values in [('SOURCE_ACTIONS',ACTIONS),('SOURCE_SPRITES',SPRITE_NAMES),('SOURCE_SOUNDS',SOUND_NAMES)]:
        rust.extend(['#[rustfmt::skip]',f'pub(crate) const {var}: &[&str] = &[', *['    '+json.dumps(v)+',' for v in values],'];'])
    for kind,var,count in [('states','SOURCE_STATES',7),('types','SOURCE_TYPES',23)]:
        rows=list(csv.reader(io.StringIO(tables[kind].decode())))[1:]
        rust.extend(['#[rustfmt::skip]',f'pub(crate) const {var}: &[(&str, [i32; {count}])] = &['])
        for row in rows:rust.append('    ('+json.dumps(row[1])+', ['+', '.join(row[2:])+']),')
        rust.append('];')
    outputs['crates/doom-sim/src/source_actors.rs']=('\n'.join(rust)+'\n').encode()
    inputs=sorted(set(['info.c','p_mobj.c','p_tick.c','p_setup.c','p_enemy.c','p_inter.c','m_random.c','p_map.c']+[p.name for p in SOURCE.glob('*.h')]))
    p={'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','flags':FLAGS,'source_sha256':{n:hashlib.sha256((SOURCE/n).read_bytes()).hexdigest() for n in inputs},'adapter_sha256':{n:hashlib.sha256((ROOT/'tools/reference'/n).read_bytes()).hexdigest() for n in ['generate_m3c.py','m3c_driver.c']},'outputs_sha256':{n:hashlib.sha256(b).hexdigest() for n,b in outputs.items()}}
    manifest=ROOT/'tests/reference/m3c_provenance.json'
    if check:
        old=json.loads(manifest.read_text())
        for k,v in p.items():assert old[k]==v,k
    else:
        p.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform(),assumptions=['metadata uses original info.c initializer expressions and symbolic headers; action-address mapping is observed in GNU ABI, never serialized','metadata-only callbacks abort if called; recognized actions are not thereby executable','state/spawn/action traces are separate from metadata recognition; no Rust output is read'])
        outputs['tests/reference/m3c_provenance.json']=(json.dumps(p,indent=2)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/name
        if check:assert path.read_bytes()==data,name
        else:path.write_bytes(data)
    print(('Checked' if check else 'Generated')+f' {len(STATE_NAMES)} states, {len(TYPE_NAMES)} types, {len(ACTIONS)-1} action identities.')
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');run(parser.parse_args().check)
