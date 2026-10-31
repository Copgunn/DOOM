"""Source inputs: animated originals, world grants and sustained armored combat."""
def populate(pack):
    core=pack.base;core.CASES.clear();layouts=[]
    def item(kind,x=36,y=0,options=7):return (x,y,0,kind,options)
    def add(label,items=(),**kw):
        items=list(items)
        if items not in layouts:layouts.append(items)
        values=dict(layout=layouts.index(items),drop=0,drop_x=36,drop_y=0,
                    move_start=1,physical=0,demon_last=0,part=0,tics=64,
                    thing=0,forward=4,move_hold=8,attack_end=0)
        values.update(kw);pack.case(label,**values)
    add('idle_none',forward=0,tics=128)
    add('idle_green',[item(2018,160)],forward=0,tics=128)
    add('idle_blue',[item(2019,160)],forward=0,tics=128)
    add('idle_mixed',[item(2018,-140),item(2019,140)],forward=0,tics=64)
    add('sixteen_animated',[item(2018+i%2,100+5*i)for i in range(16)],forward=0,tics=64)
    add('filtered_armor',[item(2018,options=1),item(2019,options=23),item(32000,options=1)])
    for last in [0,1]:
        add('thing_order_'+str(last),[item(2018,100),item(2019,120)],
            thing=1,dx=500,demon_last=last,forward=0)
    add('collect_green',[item(2018)])
    add('collect_blue',[item(2019)])
    add('collect_fullbright_green',[item(2018)],move_start=3,move_hold=10,tics=32)
    add('green_refusals',[item(2018),item(2018)],forward=1,move_hold=64)
    add('blue_refusals',[item(2019),item(2019)],forward=1,move_hold=64)
    add('green_then_blue',[item(2019),item(2018)])
    add('blue_then_green',[item(2018),item(2019)])
    add('armor_ammo_healing',[item(2018),item(2007),item(2012)],pre_damage=50,part=1)
    add('armor_weapon',[item(2019),item(2001),item(2008)],tics=128,
        thing=1,dx=300,attack_start=50,attack_end=128,part=1)
    add('armor_before_wall',[item(2018,116)],px=80,wall=1)
    add('armor_before_solid',[item(2018)],thing=1,dx=40)
    add('solid_before_armor',[item(2018)],thing=1,dx=40,demon_last=1)
    add('dead_body_refuses_armor',[item(2019,12)],thing=1,dx=-64,thing_angle=0,
        pre_damage=100,physical=1,forward=0,tics=128,part=1)
    add('sixteen_overlap',[item(2018+i%2)for i in range(16)],forward=1,move_hold=64)
    add('green_melee_pistol',[item(2018,-264)],px=-300,thing=1,dx=-44,
        thing_angle=-180,seed=6,tics=768,attack_start=50,attack_end=768,part=3)
    add('blue_survival',[item(2019,-264)],px=-300,thing=1,dx=-44,
        thing_angle=-180,seed=6,tics=960,part=3)
    add('unarmored_matched_spawn',[item(2019,500)],px=-300,thing=1,dx=-44,
        thing_angle=-180,seed=6,tics=960,part=3)
    for kind in [2018,2019]:
        add('green_depletion'if kind==2018 else'blue_depletion',
            [item(kind,-264)]+[item(2012,-204+40*i)for i in range(15)],
            px=-300,thing=1,dx=-359,thing_angle=0,seed=6,tics=1408,part=3)
    add('mixed_armored_resources',[item(2018,-264),item(2012,-336),item(2007,-336),
        item(2001,-336),item(2008,-340)],px=-300,thing=1,dx=-44,
        thing_angle=-180,seed=6,tics=640,attack_start=80,attack_end=640,part=3)
    add('armored_death_pending_weapon',[item(2019),item(2002)],thing=1,dx=-59,
        thing_angle=0,seed=6,pre_damage=99,tics=128,attack_start=15,attack_end=128,part=3)
    add('blue_chaingun',[item(2019)],ready=3,owned=11,clips=200,thing=1,
        dx=256,seed=6,tics=384,attack_start=50,attack_end=384,part=3)
    add('green_fist',[item(2018)],ready=0,clips=0,thing=1,dx=64,seed=8,
        tics=960,attack_start=15,attack_end=960,part=3)
    add('blue_shotgun',[item(2019),item(2012,-36)],ready=2,owned=7,shells=5,
        thing=1,dx=128,seed=6,tics=640,attack_start=50,attack_end=640,part=3)
    add('collect_on_transition',[item(2018)],seed=1,move_start=5,move_hold=10,tics=32)
    add('unarmored_component_control',thing=1,dx=-64,thing_angle=0,forward=0,tics=16)
    return layouts
