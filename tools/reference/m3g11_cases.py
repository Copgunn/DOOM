"""Bounded source inputs; every integrated bonus point comes from map contact."""
def populate(pack):
    core=pack.base;core.CASES.clear();layouts=[]
    def item(kind,x=36,y=0,options=7):return (x,y,0,kind,options)
    def add(label,items=(),**kw):
        items=list(items)
        assert len(items)<=16
        if items not in layouts:layouts.append(items)
        values=dict(layout=layouts.index(items),drop=0,drop_x=36,drop_y=0,
                    move_start=1,physical=0,demon_last=0,part=0,tics=64,
                    thing=0,forward=4,move_hold=8,attack_end=0)
        values.update(kw);pack.case(label,**values)
    add('idle_none',forward=0,tics=128)
    for kind,name in [(2014,'health'),(2015,'armor')]:
        add('idle_'+name,[item(kind,160)],forward=0,tics=128)
        add('collect_'+name,[item(kind)])
        add('transition_'+name,[item(kind)],seed=1,move_start=5,move_hold=10,tics=32)
    add('sixteen_animated',[item(2014+i%2,100+5*i)for i in range(16)],forward=0,tics=128)
    add('filtered_bonuses',[item(2014,options=1),item(2015,options=23),item(32000,options=1)])
    for last in [0,1]:
        add('thing_order_'+str(last),[item(2014,100),item(2018,120),item(2015,140),item(2019,160)],
            thing=1,dx=500,demon_last=last,forward=0,tics=128)
    add('sixteen_overlap',[item(2014+i%2)for i in range(16)],forward=1,move_hold=64)
    add('above100_normal_health_refusal',[item(2011),item(2012)]+[item(2014)for _ in range(8)],forward=1,move_hold=64)
    add('green_above100_refusal',[item(2018)]+[item(2015)for _ in range(8)]+[item(2018)],forward=1,move_hold=64)
    add('green_bonus_then_blue',[item(2019,100)]+[item(2015)for _ in range(8)]+[item(2018)],forward=4,move_hold=16,tics=128)
    add('bonus_before_wall',[item(2014,116),item(2015,116)],px=80,wall=1)
    add('bonus_before_solid',[item(2014),item(2015)],thing=1,dx=40)
    add('solid_before_bonus',[item(2014),item(2015)],thing=1,dx=40,demon_last=1)
    add('dead_body_refuses_bonus',[item(2014,12),item(2015,12)],thing=1,dx=-64,
        pre_damage=100,physical=1,forward=0,tics=128,part=1)
    for armor in [False,True]:
        bonuses=[item(2015 if armor else 2014,-264)for _ in range(8)]
        if armor:bonuses+=[item(2018,-264)]
        add('green_bonus_melee_pistol'if armor else'health_bonus_melee_pistol',bonuses,
            px=-300,thing=1,dx=-44,thing_angle=-180,seed=6,tics=960,
            attack_start=50,attack_end=960,part=3)
        add('green_bonus_death'if armor else'health_bonus_death',bonuses,
            px=-300,thing=1,dx=-44,thing_angle=-180,seed=6,tics=1280,part=3)
    add('mixed_counted_resources',[item(2014,-264),item(2015,-264),item(2018,-264),
        item(2012,-336),item(2007,-336),item(2001,-336),item(2008,-340)],
        px=-300,thing=1,dx=-44,thing_angle=-180,seed=6,tics=768,
        attack_start=80,attack_end=768,part=3)
    add('bonus_depletion',[item(2015,-264)for _ in range(8)]+[item(2018,-264)]+
        [item(2012,-204+40*i)for i in range(7)],px=-300,thing=1,dx=-359,
        thing_angle=0,seed=6,tics=1536,part=3)
    add('bonus_death_uncollected',[item(2014),item(2015),item(2014,500),item(2015,500)],
        thing=1,dx=-59,seed=6,pre_damage=99,tics=128,part=3)
    add('bonus_pending_weapon',[item(2014),item(2015),item(2002)],thing=1,dx=-59,
        seed=6,pre_damage=99,tics=128,attack_start=15,attack_end=128,part=3)
    add('unarmored_component_control',thing=1,dx=-64,forward=0,tics=16)
    # Explicit component timelines: controlled initial resources, never encounters.
    add('component_health_cap',[item(2014),item(2014),item(2011),item(2012)],script=30,tics=32)
    add('component_green_cap',[item(2015),item(2015),item(2018),item(2019)],script=31,tics=32)
    add('pending_retirement_second_contact',[item(2014,37),item(2015,39)],forward=50,move_hold=2,tics=16)
    return layouts
