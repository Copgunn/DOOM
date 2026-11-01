"""Declared initial resource states for genuine-C boundary observations."""
# DoomEd, initial health, initial armor type, initial points, following raw damage.
TOUCH=[(2014,h,0,0,0)for h in [1,99,100,150,199,200]]
TOUCH += [(2015,100,k,p,0)for k,p in [(0,0),(1,99),(1,100),(1,101),(1,199),(1,200),(2,100),(2,199),(2,200)]]
TOUCH += [(kind,h,0,0,0)for kind in [2011,2012]for h in [100,120,150,199,200]]
TOUCH += [(kind,100,k,p,0)for kind in [2018,2019]for k,p in [(1,101),(1,120),(1,199),(1,200),(2,101),(2,200)]]
TOUCH += [(2014,h,0,0,d)for h,d in [(100,60),(150,60),(199,60),(199,300)]]
TOUCH += [(2015,150,k,p,d)for k,p,d in [(1,100,60),(1,199,60),(2,100,60),(2,199,60),(0,0,12),(1,2,12),(2,5,12)]]
SOURCE=r'''
static int pickup_components(void){
 int i,j;player_t*p=&players[0];mobj_t *body,*item,*origin;
 const int touch[][5]={TOUCH};
 for(i=0;i<(int)(sizeof(touch)/sizeof(touch[0]));i++){
  mapthing_t thing={-64,0,0,3002,7};mapthing_t placed={36,0,0,0,7};
  reset(0);iquehead=iquetail=0;admit_native=0;G_PlayerReborn(0);
  P_SpawnMapThing(&thing);origin=(mobj_t*)thinkercap.prev;
  placed.type=touch[i][0];P_SpawnMapThing(&placed);item=(mobj_t*)thinkercap.prev;
  body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);P_SetupPsprites(p);
  p->health=body->health=touch[i][1];p->armortype=touch[i][2];p->armorpoints=touch[i][3];
  P_TouchSpecialThing(item,body);
  if(touch[i][4]){prndindex=0;P_DamageMobj(body,origin,origin,touch[i][4]);}
  printf("BCOMP,T,%d",i);for(j=0;j<5;j++)printf(",%d",touch[i][j]);
  {int fields[]={p->health,body->health,p->armortype,p->armorpoints,p->itemcount,totalitems,p->bonuscount,pickup_message(p->message),
   item->thinker.function.acv==(actionf_v)(-1),prndindex,p->damagecount,p->attacker?ident(p->attacker):-1,body->momx,body->momy,
   (int)(body->state-states),body->tics,body->flags,body->reactiontime,body->threshold,body->target?ident(body->target):-1,
   body->height,p->playerstate,p->frags[0]};
   for(j=0;j<(int)(sizeof(fields)/sizeof(fields[0]));j++)printf(",%d",fields[j]);puts("");}
 }

 /* Counted capped resources still collect both actual map actors. */
 for(i=0;i<2;i++){
  mobj_t*items[2];
  reset(0);iquehead=iquetail=0;admit_native=0;G_PlayerReborn(0);
  for(j=0;j<2;j++){mapthing_t placed={36,0,0,0,7};placed.type=i?2015:2014;P_SpawnMapThing(&placed);items[j]=(mobj_t*)thinkercap.prev;}
  body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);P_SetupPsprites(p);
  p->health=body->health=200;if(i){p->armortype=1;p->armorpoints=200;}
  for(j=0;j<2;j++){
   P_TouchSpecialThing(items[j],body);
   printf("BCOMP,C,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",i,j,p->health,body->health,p->armortype,p->armorpoints,p->itemcount,totalitems,p->bonuscount,pickup_message(p->message),items[j]->thinker.function.acv==(actionf_v)(-1));
  }
 }
 /* Real source depletion followed by reacquisition, without resetting armor. */
 {
  mobj_t*items[2];mapthing_t demon={-64,0,0,3002,7};
  reset(0);iquehead=iquetail=0;admit_native=0;G_PlayerReborn(0);
  P_SpawnMapThing(&demon);origin=(mobj_t*)thinkercap.prev;
  for(j=0;j<2;j++){mapthing_t placed={36,0,0,2015,7};P_SpawnMapThing(&placed);items[j]=(mobj_t*)thinkercap.prev;}
  body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);P_SetupPsprites(p);
  for(j=0;j<2;j++){
   P_TouchSpecialThing(items[j],body);
   printf("BCOMP,R,%d,0,%d,%d,%d,%d,%d,%d,%d\n",j,p->health,body->health,p->armortype,p->armorpoints,p->itemcount,totalitems,p->bonuscount);
   prndindex=0;P_DamageMobj(body,NULL,origin,12);
   printf("BCOMP,R,%d,1,%d,%d,%d,%d,%d,%d,%d\n",j,p->health,body->health,p->armortype,p->armorpoints,p->itemcount,totalitems,p->bonuscount);
  }
 }
 return 0;
}
'''.replace('{TOUCH}','{'+','.join('{'+','.join(map(str,r))+'}'for r in TOUCH)+'}')
