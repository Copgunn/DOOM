#!/usr/bin/env python3
"""Original player/psprite/weapon bodies with real combat, effects and noise."""
import argparse,csv,hashlib,io,json,platform,re,subprocess,tempfile
from pathlib import Path
import generate as base
import generate_m3c as actors
import generate_m3d1 as combat
ROOT,SOURCE,FLAGS=base.ROOT,base.SOURCE,base.FLAGS
U=65536
WEAPON_ACTIONS=['A_WeaponReady','A_Lower','A_Raise','A_ReFire','A_FirePistol','A_FireShotgun','A_FireCGun','A_GunFlash','A_Light0','A_Light1','A_Light2','A_Punch']
FUNCTIONS=combat.FUNCTIONS+ [('p_user.c',n,'void') for n in ['P_Thrust','P_CalcHeight','P_MovePlayer','P_PlayerThink']]+ [('p_pspr.c',n,'boolean' if n=='P_CheckAmmo' else 'void') for n in ['P_SetupPsprites','P_MovePsprites','P_SetPsprite','P_BringUpWeapon','P_DropWeapon','P_CheckAmmo','P_FireWeapon','P_BulletSlope','P_GunShot']+WEAPON_ACTIONS]+ [('p_enemy.c',n,'void') for n in ['P_RecursiveSound','P_NoiseAlert']]+ [('g_game.c','G_PlayerReborn','void')]
def case(label,ready=1,owned=3,clips=50,shells=0,tics=100,health=1000,pain=128,tx=64,ty=0,tz=0,height=56,slot=0,map=0,script='single',mass=100,other=0):
 return dict(label=label,ready=ready,owned=owned,clips=clips,shells=shells,tics=tics,health=health,pain=pain,tx=tx*U,ty=ty*U,tz=tz*U,height=height*U,slot=slot,map=map,script=script,mass=mass,other=other)
CASES=[case('pistol_single'),case('pistol_held',tics=160,script='hold'),case('pistol_release_repress',script='repress',tics=130),case('pistol_last_round',clips=1,script='hold',tics=110),case('pistol_zero_fist',clips=0,script='hold'),case('shotgun_single',ready=2,owned=7,shells=8),case('shotgun_held',ready=2,owned=7,shells=8,tics=210,script='hold'),case('shotgun_last_shell',ready=2,owned=7,shells=1,clips=5,script='hold',tics=150),case('shotgun_zero_fallback',ready=2,owned=7,shells=0,clips=4,script='hold',tics=120),case('switch_while_ready',owned=7,shells=5,script='switch',tics=130),case('switch_during_attack',owned=7,shells=5,script='switch_attack',tics=150),case('wall_pistol',tx=160),case('wall_shotgun',ready=2,owned=7,shells=8,tx=160),case('target_above',tz=64,height=56),case('aim_side_probe',ty=8,tx=100),case('lethal_pistol',health=10),case('lethal_shotgun',ready=2,owned=7,shells=8,health=20),case('slot_one',slot=1),case('slot_three',slot=3),case('bob_motion',script='motion',tics=120),case('portal_sound',map=1),case('closed_sound',map=3,tz=128),case('rng_wrap',script='hold',tics=512,health=5000),case('fist_fallback_hit',ready=0,clips=0,tx=-32,script='hold',health=100,tics=160),case('chaingun_single',ready=3,owned=11),case('chaingun_held',ready=3,owned=15,shells=4,tics=160,script='hold'),case('chaingun_last_round',ready=3,owned=15,clips=1,shells=3,script='hold',tics=180),case('chaingun_zero',ready=3,owned=15,clips=0,shells=3,script='hold',tics=160),case('bob_no_fire',script='bob',tics=100),case('turn_positive',script='turn'),case('turn_negative',script='negative_turn'),case('attack_while_raising',script='early_hold'),case('switch_while_raising',owned=7,shells=4,script='early_switch'),case('slot_two',slot=2),case('shotgun_two_targets',ready=2,owned=7,shells=3,health=10,other=1),case('noise_three_sectors',map=9),case('noise_two_blocks',map=10),case('noise_one_block',map=11)]
def commands(c):
 result=[]
 for t in range(c['tics']):
  script=c['script'];button=0;forward=side=turn=0
  if script in ['hold','motion']:button=1 if t>=20 else 0
  elif script=='bob':button=0
  elif script in ['turn','negative_turn']:button=1 if t==20 else 0;turn=(128 if script=='turn' else -128) if t<12 else 0
  elif script=='early_hold':button=1 if t<40 else 0
  elif script=='early_switch':button=4+(2<<3) if t==3 else 1 if t==60 else 0
  elif script=='repress':button=1 if 20<=t<40 or 65<=t<90 else 0
  elif script=='switch':button=4+(2<<3) if t==25 else 1 if 70<=t<72 else 0
  elif script=='switch_attack':button=1 if 20<=t<70 else 0;button|=4+(2<<3) if t==27 else 0
  else:button=1 if t==20 else 0
  if script in ['motion','bob'] and t<8:forward=12;side=-4
  result.append([forward,side,turn,button])
 return result
MAIN=r'''
static void load_weapon_world(int which){int i,j;load_world(which);memset(sides,0,sizeof(side_storage));for(i=0;i<numsectors;i++){sectors[i].linecount=0;sectors[i].lines=sector_lines[i];sectors[i].soundtarget=NULL;sectors[i].soundtraversed=0;sectors[i].validcount=0;for(j=0;j<numlines;j++)if(lines[j].frontsector==&sectors[i]||lines[j].backsector==&sectors[i])sector_lines[i][sectors[i].linecount++]=&lines[j];}for(i=0;i<numlines;i++){sides[lines[i].sidenum[0]].sector=lines[i].frontsector;if(lines[i].sidenum[1]>=0)sides[lines[i].sidenum[1]].sector=lines[i].backsector;}}
static void capture(player_t* p){int i,ownedmask=0;for(i=0;i<NUMWEAPONS;i++)if(p->weaponowned[i])ownedmask|=1<<i;printf("P,%d,%d,%ld,%d,%d,%d,%d,%d",scenario,operation,(long)(p-players),ident(p->mo),p->health,p->readyweapon,p->pendingweapon,ownedmask);for(i=0;i<4;i++)printf(",%d",p->ammo[i]);for(i=0;i<4;i++)printf(",%d",p->maxammo[i]);printf(",%d,%d,%d,%d",p->attackdown,p->refire,p->bob,p->extralight);for(i=0;i<2;i++)printf(",%ld,%d,%d,%d",p->psprites[i].state?(long)(p->psprites[i].state-states):-1L,p->psprites[i].tics,p->psprites[i].sx,p->psprites[i].sy);printf(",%d\n",prndindex);
 for(i=0;i<owned_count;i++){mobj_t* q=owned[i];printf("A,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,i,alive[i],q->type,q->x,q->y,q->z,q->momx,q->momy,q->momz,q->health,q->state?(long)(q->state-states):-1L,q->tics,q->flags,q->height,q->radius,q->reactiontime,q->threshold,q->target?ident(q->target):-1,q->lastlook,q->floorz,q->ceilingz,(int)(q->subsector-subsectors),prndindex);}
 for(i=0;i<owned_count;i++)printf("G,%d,%d,%d,%u\n",scenario,operation,i,owned[i]->angle);
 for(i=0;i<numsectors;i++)printf("S,%d,%d,%d,%d\n",scenario,operation,i,sectors[i].soundtarget?ident(sectors[i].soundtarget):-1);
}
int main(void){int c,i,t;player_t* p;for(i=0;i<NUMWEAPONS;i++)printf("V,%d,%d,%d,%d,%d,%d,%d\n",i,weaponinfo[i].ammo,weaponinfo[i].upstate,weaponinfo[i].downstate,weaponinfo[i].readystate,weaponinfo[i].atkstate,weaponinfo[i].flashstate);
 for(c=0;c<CASE_COUNT;c++){int* v=input[c];for(i=0;i<owned_count;i++)free(owned[i]);owned_count=0;memset(alive,0,sizeof(alive));memset(players,0,sizeof(players));memset(playeringame,0,sizeof(playeringame));scenario=c;operation=0;leveltime=0;prndindex=0;P_InitThinkers();load_weapon_world(v[12]);
 mobjinfo[136]=mobjinfo[MT_PLAYER];mobjinfo[136].doomednum=-1;mobjinfo[136].spawnhealth=v[5];mobjinfo[136].painchance=v[6];mobjinfo[136].mass=v[13];mobjinfo[136].flags=6;mobjinfo[136].reactiontime=8;mobjinfo[136].height=v[10];
 p=&players[v[11]];G_PlayerReborn(v[11]);p->mo=P_SpawnMobj(-80*FRACUNIT,0,ONFLOORZ,MT_PLAYER);p->mo->player=p;p->mo->flags|=v[11]<<MF_TRANSSHIFT;p->mo->health=p->health;p->viewheight=VIEWHEIGHT;
 P_SpawnMobj(v[7],v[8],v[9],136);if(v[14])P_SpawnMobj(96*FRACUNIT,0,ONFLOORZ,136);for(i=0;i<NUMWEAPONS;i++)p->weaponowned[i]=(v[1]>>i)&1;p->ammo[am_clip]=v[2];p->ammo[am_shell]=v[3];p->readyweapon=p->pendingweapon=v[0];
 printf("W,%d,%d,7,%d,%d\n",scenario,operation,v[11],ident(p->mo));P_SetupPsprites(p);playeringame[v[11]]=true;capture(p);
 for(t=0;t<v[4];t++){operation=t+1;p->cmd.forwardmove=cmds[c][t][0];p->cmd.sidemove=cmds[c][t][1];p->cmd.angleturn=cmds[c][t][2];p->cmd.buttons=cmds[c][t][3];for(i=0;i<4;i++)if(playeringame[i])P_PlayerThink(&players[i]);P_RunThinkers();leveltime++;capture(p);}
 }for(i=0;i<owned_count;i++)free(owned[i]);return 0;}
'''
def run(check):
 combat.MAPS=combat.MAPS[:9]
 for flags in [4,68,68]:
  m=combat.maps.fixture([(0,128)]*3,portal_flags=flags)
  combat.MAPS.append(m)
 # Last map has only its first portal sound-blocked. Rebuild through the fixture helper below.
 m=combat.MAPS[-1]
 portals=[i for i,l in enumerate(m['lines']) if l['back'] is not None]
 m['lines'][portals[-1]]['flags']=4
 import struct
 raw=m['bytes'];count,offset=struct.unpack_from('<ii',raw,4);lumps=[]
 for i in range(count):
  start,size,name=struct.unpack_from('<ii8s',raw,offset+16*i);data=raw[start:start+size]
  if name.rstrip(b'\0')==b'LINEDEFS':
   data=bytearray(data);struct.pack_into('<H',data,portals[-1]*14+4,4);data=bytes(data)
  lumps.append((name.rstrip(b'\0').decode(),data))
 m['bytes']=combat.maps.wad(lumps)
 shim=(ROOT/'tools/reference/m3d2_shim.h').read_text().replace('RNG_TABLE',','.join(map(str,base.table('m_random.c',r'unsigned char\s+rndtable\[256\]'))))
 parts=[]
 for file,name,result in FUNCTIONS:
  b=base.body(file,name,result)
  if name=='P_SetThingPosition':b=b.rstrip()[:-1]+'printf("L,%d,%d,%d\\n",scenario,operation,ident(thing));}\n'
  if name=='P_SetMobjState':b=re.sub(r'(do\s*\{)',r'\1\n observe(4,ident(mobj),state);',b,count=1)
  if name=='P_SpawnMobj':b=b.replace('return mobj;','observe(1,ident(mobj),type);return mobj;')
  if name=='P_RemoveThinker':b=b.replace('// FIXME: NOP.','observe(2,ident(thinker),0);')
  if name=='P_RunThinkers':b=b.replace('currentthinker->function.acp1 (currentthinker);','{observe(0,ident(currentthinker),0);currentthinker->function.acp1(currentthinker);}')
  if name in ['PIT_AddLineIntercepts','PIT_AddThingIntercepts']:b=b.replace('    intercept_p->frac = frac;','    if(intercept_p-intercepts>=128)abort();\n    intercept_p->frac = frac;')
  if name in ['P_SpawnPuff','P_SpawnBlood']:b=b.replace('    z += ((P_Random()-P_Random())<<10);','    z += jitter();')
  if name=='P_MovePlayer':b=b.replace('(cmd->angleturn<<16)','(cmd->angleturn*65536)')
  if name=='P_SetPsprite':b=re.sub(r'(do\s*\{)',r'\1\n w_entry(player,position,stnum);',b,count=1)
  if name in WEAPON_ACTIONS:b=re.sub(r'(\{)',r'\1\n w_action(player,psp,'+str(actors.ACTIONS.index(name))+');',b,count=1)
  if name in ['A_FirePistol','A_FireShotgun','A_FireCGun']:
   b=b.replace('player->ammo[weaponinfo[player->readyweapon].ammo]--;','player->ammo[weaponinfo[player->readyweapon].ammo]--;\n printf("W,%d,%d,2,%ld,%d,%d,%d\\n",scenario,operation,(long)(player-players),weaponinfo[player->readyweapon].ammo,player->ammo[weaponinfo[player->readyweapon].ammo]+1,player->ammo[weaponinfo[player->readyweapon].ammo]);')
  if name.startswith('A_Light'):b=re.sub(r'(player->extralight = [012];)',r'\1\n printf("W,%d,%d,3,%ld,%d\\n",scenario,operation,(long)(player-players),player->extralight);',b)
  if name=='P_FireWeapon':b=b.replace('    P_SetMobjState (player->mo, S_PLAY_ATK1);','    printf("W,%d,%d,4,%ld,%d,%d\\n",scenario,operation,(long)(player-players),player->readyweapon,player->refire);\n    P_SetMobjState (player->mo, S_PLAY_ATK1);')
  if name in ['P_GunShot','A_Punch']:
   b=b.replace('P_Random ()%3','w_random(0)%3').replace('P_Random ()%10','w_random(0)%10').replace('(P_Random()-P_Random())<<18','spread()')
   b=b.replace('    P_LineAttack (mo, angle, MISSILERANGE, bulletslope, damage);','    bullet(mo,angle,bulletslope,damage);\n    P_LineAttack (mo, angle, MISSILERANGE, bulletslope, damage);').replace('    P_LineAttack (player->mo, angle, MELEERANGE, slope, damage);','    bullet(player->mo,angle,slope,damage);\n    P_LineAttack (player->mo, angle, MELEERANGE, slope, damage);')
  if name=='P_RecursiveSound':b=b.replace('    sec->soundtarget = soundtarget;','    sec->soundtarget = soundtarget;\n printf("W,%d,%d,6,%ld,%d,%d\\n",scenario,operation,(long)(sec-sectors),soundblocks,ident(soundtarget));')
  parts.append(b)
 parts.insert(0,base.body('m_random.c','P_Random','int','Raw_Random')+'int P_Random(void){int value=Raw_Random();observe(8,-1,value);return value;}\nstatic int jitter(void){int a=P_Random();int b=P_Random();return (a-b)*1024;}')
 for name in actors.ACTIONS[1:]:
  if name in WEAPON_ACTIONS:continue
  if name in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']:
   b=base.body('p_enemy.c',name,'void');arg='mo' if name=='A_PlayerScream' else 'actor';b=re.sub(r'(\{)',r'\1\n observe(5,ident('+arg+'),'+str(actors.ACTIONS.index(name))+');',b,count=1)
  else:b=f'void {name}(mobj_t* actor){{(void)actor;abort();}}'
  parts.append(b)
 text=shim.replace('/* SOURCE_BODIES */','\n'.join(parts))+'\n'+combat.c_maps()
 fields=['ready','owned','clips','shells','tics','health','pain','tx','ty','tz','height','slot','map','mass','other']
 text+='\n#define CASE_COUNT '+str(len(CASES))+'\nint input[][15]={' + ','.join('{'+','.join(str(c[f]) for f in fields)+'}' for c in CASES)+'};\n'
 text+='int cmds[CASE_COUNT][512][4]={' + ','.join('{'+','.join('{'+','.join(map(str,row))+'}' for row in commands(c))+'}' for c in CASES)+'};\n'+MAIN
 outputs={f'tests/reference/m3d2_map_{i}.bin':m['bytes'] for i,m in enumerate(combat.MAPS) if i>=9}
 with tempfile.TemporaryDirectory(prefix='doom-m3d2-reference-') as tmp:
  src=Path(tmp)/'reference.c';src.write_text(text);binary=Path(tmp)/'reference';subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'tables.c'),'-o',str(binary)],check=True);rows=subprocess.check_output([str(binary)]).decode().splitlines()
 outputs['tests/reference/m3d2_trace.csv']=('\n'.join(r for r in rows if not r.startswith('V,'))+'\n').encode()
 outputs['tests/reference/m3d2_weapons.csv']=('weapon,ammo,up,down,ready,attack,flash\n'+'\n'.join(r[2:] for r in rows if r.startswith('V,'))+'\n').encode()
 table=io.StringIO();w=csv.writer(table,lineterminator='\n');w.writerow(['case',*CASES[0].keys()]);w.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3d2_cases.csv']=table.getvalue().encode()
 table=io.StringIO();w=csv.writer(table,lineterminator='\n');w.writerow(['case','tic','slot','forward','side','turn','buttons']);w.writerows([[i,t+1,c['slot'],*cmd] for i,c in enumerate(CASES) for t,cmd in enumerate(commands(c))]);outputs['tests/reference/m3d2_commands.csv']=table.getvalue().encode()
 paths=sorted({f for f,_,_ in FUNCTIONS}|{'info.c','m_random.c','m_fixed.c','tables.c','d_items.c'}|{p.name for p in SOURCE.glob('*.h')})
 manifest={'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','functions':FUNCTIONS,'flags':FLAGS,'source_sha256':{p:hashlib.sha256((SOURCE/p).read_bytes()).hexdigest() for p in paths},'adapter_sha256':{p:hashlib.sha256((ROOT/'tools/reference'/p).read_bytes()).hexdigest() for p in ['generate.py','generate_m3b.py','generate_m3c.py','generate_m3d1.py','generate_m3d2.py','m3d2_shim.h']},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'assumptions':['GNU C original player/state structs, 32-bit int/fixed and 64-bit pointers; source function unions/sentinel','negative spread/jitter shifts use ordered draws and multiplication; command shift uses multiplication','no player damage/powers/use/specials/projectiles/AI or blocked player sliding in these fixtures; stubs abort','manual driver preserves source active-slot/player-before-thinker ordering; no active special or respawn paths','retirement storage retained until reset; C ordinal identities mapped to Rust generations independently']}
 path=ROOT/'tests/reference/m3d2_provenance.json'
 if check:
  old=json.loads(path.read_text())
  for k,v in manifest.items():assert old[k]==json.loads(json.dumps(v)),k
 else:
  manifest.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform());outputs['tests/reference/m3d2_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
 for p,b in outputs.items():
  if check:assert (ROOT/p).read_bytes()==b,p
  else:(ROOT/p).write_bytes(b)
 print(('Checked' if check else 'Generated')+f' M3d2: {len(CASES)} original-body player weapon scenarios.')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
