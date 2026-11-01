"""Bounded source inputs for original monsters at real-map scale.

Every monster is an ordinary map THING spawned by the genuine P_SpawnMapThing in a
walled rectangle built by m3h3_geometry. Rooms reach the full 2,048-unit vertex
envelope and one is offset far from the origin. The two controlled between-tic
inputs of M3h2 keep their meaning: `hit_*` is one P_DamageMobj(thing,NULL,NULL,
amount); `attack_*` assigns the player target and enters S_TROO_ATK1 so the first
launch tic is deterministic. The original has no coordinate limit, so these
scenarios simply continue where the historical Rust envelope stalled. No Rust output."""
IMP,POSSESSED,SHOTGUY,CHAINGUY,DEMON=3001,3004,9,65,3002
ROSTER=(IMP,POSSESSED,SHOTGUY,CHAINGUY,DEMON)
# (rectangle, pillars): the M3h2 stall room, the full envelope, an offset room and
# a full room with a pillar near its east wall.
ROOMS=[((-1000,-1000,1200,1000),()),((-2048,-2048,2048,2048),()),((200,-2048,2048,-200),()),((-2048,-2048,2048,2048),((1800,-100,1900,100),))]
def populate(pack):
    core=pack.base;core.CASES.clear();layouts=[]
    def monster(kind,x,y=0,angle=0,options=7):return (x,y,angle,kind,options)
    def add(label,room,things=(),hit_at=0,hit_thing=0,hit_amount=0,attack_at=0,attack_thing=0,**kw):
        things=list(things)
        assert sum(t[3] in ROSTER for t in things)<=8
        if things not in layouts:layouts.append(things)
        values=dict(layout=layouts.index(things),drop=0,drop_x=36,drop_y=0,
                    move_start=1,physical=0,demon_last=0,part=3,tics=256,
                    thing=0,forward=0,move_hold=0,attack_end=0,seed=6)
        values.update(kw);pack.case(label,**values)
        core.CASES[-1].update(room=room,hit_at=hit_at,hit_thing=hit_thing,hit_amount=hit_amount,attack_at=attack_at,attack_thing=attack_thing)
    west=dict(turn=16384,turn_hold=2)
    chaingun=dict(ready=3,owned=11,clips=200)
    # The three reproduced M3h2 stalls, now continuing: a fireball hit 16 units from
    # the wall, a point-blank shotgun volley there, and a missed fireball past 994.
    add('stall_fireball_hit_near_wall',0,[monster(IMP,800)],px=984,attack_at=1,tics=120)
    add('stall_hitscan_near_wall',0,[monster(SHOTGUY,936)],px=984,tics=120)
    add('stall_fireball_past_994',0,[monster(IMP,-900,100)],px=-600,attack_at=1,side=50,move_start=17,move_hold=21,tics=320)
    # Corner and wall impulses at the full extent.
    add('corner_fireball_pinch',1,[monster(IMP,1800,1900),monster(IMP,1900,1800),monster(IMP,1700,1700)],px=2032,py=2032,attack_at=1,tics=480)
    add('demon_corner_bites',1,[monster(DEMON,1950,1950)],px=2032,py=2032,tics=300)
    add('wall_slide_under_fire',1,[monster(CHAINGUY,1800,200)],px=2032,py=-500,side=-50,move_start=1,move_hold=200,tics=400)
    add('monster_knockback_at_wall',1,[monster(POSSESSED,2028,0,-180)],px=1600,attack_start=10,attack_end=300,tics=320)
    add('pillar_pinch',3,[monster(DEMON,1980,180,-90),monster(IMP,1500)],px=1980,attack_at=1,attack_thing=1,tics=300)
    # Long flights and the whole room.
    add('long_fireball_flight',1,[monster(IMP,-1900)],px=1900,attack_at=1,tics=480)
    add('full_room_traversal',1,[monster(IMP,-1900,1900)],px=-2000,py=-2000,forward=50,side=-50,move_start=1,move_hold=400,tics=480)
    add('eight_imps_far_room',1,[monster(IMP,-1800+300*i,-1800) for i in range(8)],px=1800,py=1800,tics=960,**chaingun)
    # Mixed roster against the chaingun player backed into the east wall.
    add('mixed_roster_corner_fight',1,[monster(IMP,1300,100),monster(IMP,1300,-100),monster(POSSESSED,1400,200),monster(SHOTGUY,1400,-200),
        monster(CHAINGUY,1500,0),monster(DEMON,1600,60),monster(IMP,1200,0),monster(POSSESSED,1200,250)],px=1900,attack_start=10,attack_end=720,tics=720,**chaingun,**west)
    # Far from the origin: kills, drops and pickups, then hitscan puffs on distant walls.
    add('drops_far_from_origin',2,[monster(POSSESSED,1600,-1780),monster(SHOTGUY,1600,-1820),monster(CHAINGUY,1550,-1800)],px=1800,py=-1800,
        attack_start=10,attack_end=200,forward=25,move_start=260,move_hold=330,tics=512,**chaingun,**west)
    add('hitscan_far_from_origin',2,[monster(POSSESSED,1300,-1800)],px=1800,py=-1800,attack_start=10,attack_end=200,tics=240,**west)
    return layouts
