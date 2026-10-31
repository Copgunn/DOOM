"""Unticked defined source components; setup is separate from world encounters."""
import re
from pathlib import Path
GRANTS=[(0,0,1),(0,0,2),(1,99,1),(1,100,1),(1,100,2),(2,199,2),
        (2,200,2),(2,80,1),(2,99,1),(2,100,1),(2,180,1),(1,1,2),
        (2,1,1),(1,5,1),(2,5,2),(2,101,1)]
# kind, points, health, raw incoming, physical. Positive source scalar setups;
# high nonphysical inputs do not admit telefragging or source cheat branches.
DAMAGE=[(k,p,100,d,0)for k,p in [(0,0),(1,100),(2,200)]for d in [0,1,2,5,10,60]]
DAMAGE += [(1,p,100,60,0)for p in [19,20,21]]+[(2,p,100,60,0)for p in [29,30,31]]
DAMAGE += [(k,p,100,60,1)for k,p in [(0,0),(1,100),(2,200)]]
DAMAGE += [(1,100,100,150,0),(2,200,100,200,0),(1,5,50,60,1),
           (2,5,50,60,1),(1,3,1,12,1),(2,6,1,12,1),(1,100,100,140,1),
           (2,200,100,140,1),(1,100,100,1000,0),(2,200,100,1000,0)]
table=list(map(int,re.findall(r'\d+', (Path(__file__).resolve().parents[2]/'linuxdoom-1.10/m_random.c').read_text().split('rndtable[256] = {',1)[1].split('}',1)[0])))
assert len(table)==256
no_pain_seed=(table.index(255)-1)&255
DAMAGE=[(*r,0)for r in DAMAGE]+[(1,100,100,5,0,no_pain_seed),(2,200,100,5,0,no_pain_seed)]
DAMAGE.append((2,200,100,0,1,0))
SOURCE=r'''
static int pickup_components(void){
 int i,j,ok,before,ptype,points;player_t*p=&players[0];mobj_t *body,*origin;
 const int grants[][3]={GRANTS};const int damage[][6]={DAMAGE};
 for(i=0;i<(int)(sizeof(grants)/sizeof(grants[0]));i++){
  reset(0);iquehead=iquetail=0;admit_native=0;G_PlayerReborn(0);
  body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);
  p->armortype=grants[i][0];p->armorpoints=grants[i][1];
  ok=P_GiveArmor(p,grants[i][2]);
  printf("ACOMP,G,%d,%d,%d,%d,%d,%d,%d\n",i,grants[i][2],grants[i][0],grants[i][1],ok,p->armortype,p->armorpoints);
 }
 for(i=0;i<(int)(sizeof(damage)/sizeof(damage[0]));i++){
  mapthing_t thing={-64,0,0,3002,7};
  reset(0);iquehead=iquetail=0;admit_native=0;G_PlayerReborn(0);
  P_SpawnMapThing(&thing);origin=(mobj_t*)thinkercap.prev;
  body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);P_SetupPsprites(p);
  p->health=body->health=damage[i][2];p->armortype=damage[i][0];p->armorpoints=damage[i][1];
  prndindex=damage[i][5];P_DamageMobj(body,damage[i][4]?origin:NULL,origin,damage[i][3]);
  printf("ACOMP,D,%d",i);for(j=0;j<6;j++)printf(",%d",damage[i][j]);
  {int fields[]={p->armortype,p->armorpoints,p->health,body->health,p->damagecount,
   ident(p->attacker),body->momx,body->momy,(int)(body->state-states),body->tics,
   body->flags,body->reactiontime,body->threshold,body->target?ident(body->target):-1,
   prndindex,body->height,p->playerstate,p->frags[0]};
   for(j=0;j<(int)(sizeof(fields)/sizeof(fields[0]));j++)printf(",%d",fields[j]);puts("");}
 }
 for(i=1;i<=2;i++){
  reset(0);iquehead=iquetail=0;admit_native=0;G_PlayerReborn(0);
  body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);P_SetupPsprites(p);
  p->armortype=i;p->armorpoints=5;prndindex=0;
  for(j=0;j<3;j++){
   before=body->health;ptype=p->armortype;points=p->armorpoints;
   P_DamageMobj(body,NULL,NULL,10);
   printf("ACOMP,R,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",i,j,ptype,points,p->armortype,p->armorpoints,before,p->health,body->health,prndindex);
  }
 }
 for(i=1;i<=2;i++){
  mapthing_t thing={-64,0,0,3002,7};int health,body_health,count,index,momx,momy;
  reset(0);iquehead=iquetail=0;admit_native=0;G_PlayerReborn(0);
  P_SpawnMapThing(&thing);origin=(mobj_t*)thinkercap.prev;
  body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);P_SetupPsprites(p);
  p->armortype=i;p->armorpoints=i*100;prndindex=0;
  P_DamageMobj(body,NULL,origin,i==1?150:200);
  health=p->health;body_health=body->health;count=p->damagecount;index=prndindex;momx=body->momx;momy=body->momy;
  ptype=p->armortype;points=p->armorpoints;
  P_DamageMobj(body,origin,origin,10);
  printf("ACOMP,I,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",
   i,ptype,points,p->armortype,p->armorpoints,health,p->health,body_health,body->health,
   count,p->damagecount,index,prndindex,momx,body->momx,momy,body->momy);
 }
 return 0;
}
'''.replace('{GRANTS}','{'+','.join('{'+','.join(map(str,r))+'}'for r in GRANTS)+'}').replace('{DAMAGE}','{'+','.join('{'+','.join(map(str,r))+'}'for r in DAMAGE)+'}')
