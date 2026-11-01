"""Bounded source inputs for two-sided lines, static heights and vertical movement.

Every monster is an ordinary map THING spawned by the genuine P_SpawnMapThing in a
room built by m3h4_geometry; the player body spawns ONFLOORZ at (px, py), so its
height comes from the real BSP lookup. The controlled between-tic inputs of M3h2
keep their meaning: `hit_*` is one P_DamageMobj(thing,NULL,NULL,amount); `attack_*`
assigns the player target and enters S_TROO_ATK1 so the first launch tic is
deterministic. The original has no flat-world assumption, so these scenarios simply
observe what the compiled bodies do on steps, ledges, windows and fences. No Rust
output."""
IMP,POSSESSED,SHOTGUY,CHAINGUY,DEMON=3001,3004,9,65,3002
ROSTER=(IMP,POSSESSED,SHOTGUY,CHAINGUY,DEMON)
BLOCKING,BLOCKMONSTERS,SOUNDBLOCK=1,2,64
# (spans, heights, portals, reject): see the investigation note, section 6.1. High sides lie
# west and targets east because every fresh mobj has movedir DI_EAST, so a target due west is
# the first P_NewChaseDir's turnaround and the monster walks east before turning (p_enemy.c).
STAIRS=([-768,-512,-256,0,256,1024],[(97,256),(72,256),(48,256),(24,256),(0,256)],{},())
LEDGE=([-1024,0,64,1024],[(96,256),(0,256),(0,100)],{},())
WINDOW=([-1024,-64,64,512,1024],[(0,128),(40,88),(-64,256),(64,256)],{},())
CLOSED=([-512,0,64,512],[(0,256),(128,128),(0,256)],{},())
FENCE=([-512,0,512],[(0,256),(0,256)],{1:BLOCKING},())
CRAWL=([-512,0,512],[(0,128),(0,32)],{},())
TOWER=([-1024,0,1024],[(0,160),(96,160)],{},())
ROOMS=[STAIRS,LEDGE,WINDOW,CLOSED,FENCE,CRAWL,TOWER,
  (WINDOW[0],WINDOW[1],{},((0,3),)),                               # 7: window with a REJECT pair
  (FENCE[0],FENCE[1],{1:BLOCKMONSTERS},()),                        # 8: fence that blocks monsters only
  (WINDOW[0],WINDOW[1],{1:SOUNDBLOCK,2:SOUNDBLOCK},())]            # 9: window with two sound-blocking lines
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
    pistol=dict(attack_start=10,attack_end=200)
    shotgun=dict(ready=2,owned=7,shells=50)
    # Stairs descending east: one 25-unit riser then 24-unit treads.
    add('stairs_up',0,[],px=900,forward=50,move_start=3,move_hold=150,tics=180,**west)
    add('stairs_down',0,[],px=-700,forward=50,move_start=1,move_hold=150,tics=180)
    add('too_high_step',0,[],px=-400,forward=50,move_start=3,move_hold=60,tics=90,**west)
    add('imp_pursuit_stairs',0,[monster(IMP,-400)],px=900,tics=480)
    add('demon_pursuit_stairs',0,[monster(DEMON,-400)],px=900,tics=480)
    add('monster_refuses_riser',0,[monster(POSSESSED,-640)],px=900,tics=240)
    add('autoaim_up_step',0,[monster(POSSESSED,-100)],px=500,tics=120,**pistol,**west)
    add('autoaim_down_step',0,[monster(POSSESSED,500,0,-180)],px=-400,tics=120,**pistol)
    # Ledge: a 96-unit drop into a 64-unit landing strip, then a 100-unit ceiling.
    add('drop_landing_oof',1,[],px=-500,forward=50,move_start=1,move_hold=120,tics=150)
    add('monster_refuses_ledge',1,[monster(POSSESSED,-200)],px=300,tics=240)
    add('corpse_pushed_off_ledge',1,[monster(SHOTGUY,-24,0,-180)],px=-500,tics=240,attack_start=10,attack_end=60,**shotgun)
    add('fall_forward_from_below',1,[monster(POSSESSED,-100)],px=200,tics=120,hit_at=1,hit_thing=0,hit_amount=12,**pistol,**west)
    add('fireball_down_ledge_floor',1,[monster(IMP,-124)],px=300,attack_at=1,side=50,move_start=17,move_hold=40,tics=180)
    # Window: a [40,88] opening between the floor-0 room, a pit and a 64-unit step.
    add('window_sight_pit_then_step',2,[monster(POSSESSED,300,0,-180),monster(POSSESSED,700,0,-180)],px=-300,tics=240)
    add('window_lower_wall_puff',2,[],px=-300,tics=60,attack_start=10,attack_end=40)
    add('window_upper_wall_puff',2,[],px=700,tics=60,attack_start=10,attack_end=40,**west)
    add('noise_through_window',2,[monster(POSSESSED,300,0,-180)],px=-300,tics=120,attack_start=10,attack_end=40)
    add('noise_through_soundblock',9,[monster(POSSESSED,300,0,-180)],px=-300,tics=120,attack_start=10,attack_end=40)
    add('reject_bit',7,[monster(POSSESSED,700,0,-180)],px=-300,tics=120)
    # Closed sector: a zero-opening line stops sight, noise, hitscan and movement.
    add('closed_opening_blocks_all',3,[monster(POSSESSED,300,0,-180)],px=-300,forward=50,move_start=1,move_hold=60,tics=120,attack_start=10,attack_end=40)
    # Fences: two-sided blocking lines between equal sectors.
    add('fence_slide',4,[monster(IMP,300,0,-180)],px=-300,forward=50,side=20,move_start=1,move_hold=120,tics=300,**pistol)
    add('fence_blocks_monsters_only',8,[monster(POSSESSED,-100)],px=300,py=100,forward=50,move_start=60,move_hold=90,tics=200,**west)
    # Crawlspace: a 32-unit ceiling refuses the player and takes an upper-wall puff.
    add('crawlspace_refusal',5,[],px=-300,forward=50,move_start=1,move_hold=60,tics=90,attack_start=10,attack_end=40)
    # Tower: a fireball from far below rises past a dodging player into the ceiling.
    add('fireball_up_ceiling',6,[monster(IMP,-900)],px=300,attack_at=1,side=50,move_start=17,move_hold=60,tics=200)
    return layouts
