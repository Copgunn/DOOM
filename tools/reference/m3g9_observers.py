"""Source-only instrumentation; mutations still execute original function bodies."""
EXTRA=r'''
boolean PTR_SlideTraverse(intercept_t*);
boolean PTR_AimTraverse(intercept_t*);
boolean PTR_ShootTraverse(intercept_t*);
static int pickup_message(const char*p){
 if(!p)return 0;
 if(!strcmp(p,GOTCLIP))return 1;if(!strcmp(p,GOTCLIPBOX))return 2;
 if(!strcmp(p,GOTSHELLS))return 3;if(!strcmp(p,GOTSHELLBOX))return 4;
 if(!strcmp(p,GOTSTIM))return 5;if(!strcmp(p,GOTMEDIKIT))return 6;
 if(!strcmp(p,GOTMEDINEED))return 7;if(!strcmp(p,GOTSHOTGUN))return 8;
 if(!strcmp(p,GOTCHAINGUN))return 9;abort();return -1;
}
'''
WRAPPERS={
'P_GiveAmmo':r'''
boolean P_GiveAmmo(player_t*p,ammotype_t a,int num){
 int before=p->ammo[a];boolean ok=Observed_GiveAmmo(p,a,num);
 printf("PN,%d,%d,1,%d,%d,%d,%d,%d,%d\n",scenario,operation,a,ok,before,p->ammo[a],p->readyweapon,p->pendingweapon);
 return ok;
}
''',
'P_GiveBody':r'''
boolean P_GiveBody(player_t*p,int num){
 int before=p->health;boolean ok=Observed_GiveBody(p,num);
 printf("PN,%d,%d,2,%d,%d,%d,%d\n",scenario,operation,ok,before,p->health,p->mo->health);
 return ok;
}
''',
'P_GiveWeapon':r'''
boolean P_GiveWeapon(player_t*p,weapontype_t w,boolean dropped){
 int before=p->weaponowned[0]+2*p->weaponowned[1]+4*p->weaponowned[2]+8*p->weaponowned[3];
 int pending=p->pendingweapon;boolean ok=Observed_GiveWeapon(p,w,dropped);
 int after=p->weaponowned[0]+2*p->weaponowned[1]+4*p->weaponowned[2]+8*p->weaponowned[3];
 printf("PN,%d,%d,3,%d,%d,%d,%d,%d,%d\n",scenario,operation,w,ok,before,after,pending,p->pendingweapon);
 return ok;
}
''',
'P_TouchSpecialThing':r'''
void P_TouchSpecialThing(mobj_t*item,mobj_t*body){
 int head=iquehead,tail=iquetail,outcome;fixed_t delta=item->z-body->z;
 printf("PN,%d,%d,0,%d,%d,%d,%d,%d,%d\n",scenario,operation,ident(item),ident(body),item->sprite,delta,body->height,body->health);
 outcome=delta>body->height||delta< -8*FRACUNIT?0:body->health<=0?1:2;
 Observed_TouchSpecialThing(item,body);
 if(item->thinker.function.acv==(actionf_v)(-1))outcome=3;
 printf("PN,%d,%d,6,%d,%d\n",scenario,operation,ident(item),outcome);
 printf("PQ,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,ident(item),head,iquehead,tail,iquetail);
}
'''}
def instrument(core):
    previous=core.observed_body
    def observed(file,name,result,rename=None):
        if name=='P_PathTraverse':
            text=previous(file,name,result,rename)
            return text.replace('{','{\n printf("TR,%d,%d,%d,%d\\n",scenario,operation,flags,trav==PTR_AimTraverse?0:trav==PTR_ShootTraverse?1:trav==PTR_SlideTraverse?2:3);',1)
        if name not in WRAPPERS:return previous(file,name,result,rename)
        assert rename is None
        text=previous(file,name,result,'Observed_'+name.removeprefix('P_'))
        if name=='P_TouchSpecialThing':
            needle='    if (special->flags & MF_COUNTITEM)'
            assert text.count(needle)==1
            text=text.replace(needle,'    printf("PN,%d,%d,4,%d\\n",scenario,operation,pickup_message(player->message));\n'+needle)
            needle='    player->bonuscount += BONUSADD;'
            assert text.count(needle)==1
            text=text.replace(needle,needle+'\n    printf("PN,%d,%d,5,%d,%d\\n",scenario,operation,player->bonuscount,player->itemcount);')
        return text+'\n'+WRAPPERS[name]
    core.observed_body=observed
