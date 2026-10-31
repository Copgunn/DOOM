"""Defined direct-source components; not airborne/gameplay admission."""
SOURCE=r'''
static int pickup_components(void){
 int i,ok;fixed_t delta[]={56*FRACUNIT,56*FRACUNIT+1,-8*FRACUNIT,-8*FRACUNIT-1};
 mobj_t*body,*item;player_t*p=&players[0];
 for(i=0;i<4;i++){
  reset(0);iquehead=iquetail=0;admit_native=0;G_PlayerReborn(0);
  body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);
  item=P_SpawnMobj(36*FRACUNIT,0,delta[i],MT_CLIP);
  P_TouchSpecialThing(item,body);
  printf("PCOMP,vertical,%d,%d,%d,%d,%d,%d\n",i,delta[i],item->thinker.function.acv==(actionf_v)(-1),p->ammo[0],p->bonuscount,pickup_message(p->message));
 }
 for(i=0;i<2;i++){
  reset(0);iquehead=iquetail=0;admit_native=0;G_PlayerReborn(0);
  body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);
  item=P_SpawnMobj(36*FRACUNIT,0,ONFLOORZ,MT_CLIP);
  ok=P_CheckPosition(body,i,0);
  printf("PCOMP,square_edge,%d,%d,%d,%d,%d\n",i,ok,item->thinker.function.acv==(actionf_v)(-1),p->ammo[0],p->bonuscount);
 }
 reset(0);admit_native=0;G_PlayerReborn(0);
 body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);p->health=body->health=125;
 ok=P_GiveBody(p,25);printf("PCOMP,above_health,0,%d,%d,%d\n",ok,p->health,body->health);
 reset(0);admit_native=0;G_PlayerReborn(0);
 body=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);associate(body);
 P_DamageMobj(body,NULL,NULL,100);item=P_SpawnMobj(0,0,ONFLOORZ,MT_MISC11);
 P_TouchSpecialThing(item,body);
 printf("PCOMP,dead_touch,0,%d,%d,%d,%d\n",item->thinker.function.acv==(actionf_v)(-1),p->health,body->health,p->bonuscount);
 return 0;
}
'''
