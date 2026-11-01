"""Read-only source bonus/statistics observations over the frozen C harness."""
def instrument(core,previous):
    needle='if(!p)return 0;'
    assert previous.extra.count(needle)==1
    previous.extra=previous.extra.replace(needle,needle+'if(!strcmp(p,GOTHTHBONUS))return 12;if(!strcmp(p,GOTARMBONUS))return 13;')
    old=core.observed_body
    def observe(file,name,result,rename=None):
        text=old(file,name,result,rename)
        if name=='P_TouchSpecialThing':
            text=text.replace('{','{\n int bonus_before_health=0,bonus_before_type=0,bonus_before_points=0;',1)
            needle='player = toucher->player;'
            assert text.count(needle)==1
            text=text.replace(needle,needle+'\n bonus_before_health=player->health;bonus_before_type=player->armortype;bonus_before_points=player->armorpoints;')
            needle='player->message = GOTHTHBONUS;'
            assert text.count(needle)==1
            text=text.replace(needle,'printf("PN,%d,%d,8,%d,%d,%d\\n",scenario,operation,bonus_before_health,player->health,player->mo->health);\n '+needle)
            needle='player->message = GOTARMBONUS;'
            assert text.count(needle)==1
            text=text.replace(needle,'printf("PN,%d,%d,9,%d,%d,%d,%d\\n",scenario,operation,bonus_before_type,bonus_before_points,player->armortype,player->armorpoints);\n '+needle)
            needle='player->itemcount++;'
            assert text.count(needle)==1
            text=text.replace(needle,'{int before_items=player->itemcount;player->itemcount++;printf("PN,%d,%d,10,%d,%d\\n",scenario,operation,before_items,player->itemcount);}')
        if name=='P_SpawnMapThing':
            needle='totalitems++;'
            assert text.count(needle)==1
            text=text.replace(needle,'{totalitems++;printf("IT,%d,%d,%d,%d\\n",scenario,operation,ident(mobj),totalitems);}')
        return text
    core.observed_body=observe
