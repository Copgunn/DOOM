"""Bounded source inputs for non-axis (diagonal) walls in the heights world.

Every monster is an ordinary map THING spawned by the genuine P_SpawnMapThing in a
polygon room built by m3h5_geometry; the player body spawns ONFLOORZ at (px, py).
The controlled between-tic inputs of M3h2 keep their meaning (`hit_*` is one
P_DamageMobj(thing,NULL,NULL,amount); `attack_*` assigns the player target and
enters S_TROO_ATK1). One new harness-only input exists for the corner jump:
`push_at/push_x/push_y` sets the player body's momx/momy (map units) before that
tic's command stage, standing in for stacked knockback that the admitted roster
cannot deliver in one tic; it is declared as an adapter, not a source claim. The
original has no axis-alignment assumption, so these scenarios simply observe what
the compiled bodies do against walls of several angles, diagonal corners, a
diagonal riser and window, a diagonal pillar and a diagonal corridor. No Rust
output."""
IMP,POSSESSED,SHOTGUY,CHAINGUY,DEMON=3001,3004,9,65,3002
ROSTER=(IMP,POSSESSED,SHOTGUY,CHAINGUY,DEMON)
# Rooms: (sectors, portals, reject, partition tree, wall segment length); see the
# investigation note, section 6.1. Rings are clockwise (holes counterclockwise).
WEDGE_RING=[(-1024,0),(-1024,768),(-256,768),(256,256),(768,256),(1024,0),(768,-256),(768,-512),(288,-712),(-672,-904),(-1024,-904)]
WEDGE=([([WEDGE_RING],0,128)],{},(),((-256,768),(256,256),None,None),1024)
PILLAR_OUTER=[(-512,-512),(-512,512),(512,512),(512,-512)]
PILLAR_HOLE=[(-64,-128),(64,-128),(128,-64),(128,64),(64,128),(-64,128),(-128,64),(-128,-64)]
PILLAR_TREE=((64,128),(-64,128),None,((-64,-128),(64,-128),None,((128,-64),(128,64),None,((-128,64),(-128,-64),None,
             ((64,-128),(128,-64),None,((128,64),(64,128),None,((-64,128),(-128,64),None,None)))))))
PILLAR=([([PILLAR_OUTER,PILLAR_HOLE],0,128)],{},(),PILLAR_TREE,1024)
CORRIDOR_A=[(-1024,-128),(-384,-128),(-384,-288),(-384,-480),(-384,-768),(-1024,-768)]
CORRIDOR_C=[(-384,-288),(384,480),(384,288),(-384,-480)]
CORRIDOR_B=[(384,128),(384,288),(384,480),(384,768),(1024,768),(1024,128)]
CORRIDOR=([([CORRIDOR_A],0,128),([CORRIDOR_C],0,128),([CORRIDOR_B],0,128)],{},(),((-384,-288),(-384,-480),None,((384,480),(384,288),None,None)),1024)
STEP=([([[(-384,-384),(384,384),(384,-384)]],0,128),([[(-384,-384),(-384,384),(384,384)]],48,128)],{},(),((-384,-384),(384,384),None,None),1024)
WINDOW=([([[(-192,-384),(384,192),(384,-384)]],0,128),([[(-384,-192),(192,384),(384,384),(384,192),(-192,-384),(-384,-384)]],40,88),([[(-384,-192),(-384,384),(192,384)]],64,256)],
        {},(),((-192,-384),(384,192),None,((-384,-192),(192,384),None,None)),1024)
CORNER=([([[(-1024,0),(0,0),(-383,-924),(-1024,-924)]],0,128)],{},(),None,1536)
ROOMS=[WEDGE,PILLAR,CORRIDOR,STEP,WINDOW,CORNER]
ROOM_NAMES=['wedge','pillar','corridor','step','window','corner']
def populate(pack):
    core=pack.base;core.CASES.clear();layouts=[]
    def monster(kind,x,y=0,angle=0,options=7):return (x,y,angle,kind,options)
    def add(label,room,things=(),hit_at=0,hit_thing=0,hit_amount=0,attack_at=0,attack_thing=0,push_at=0,push_x=0,push_y=0,**kw):
        things=list(things)
        assert sum(t[3] in ROSTER for t in things)<=8
        if things not in layouts:layouts.append(things)
        values=dict(layout=layouts.index(things),drop=0,drop_x=36,drop_y=0,
                    move_start=1,physical=0,demon_last=0,part=3,tics=256,
                    thing=0,forward=0,move_hold=0,attack_end=0,seed=6)
        values.update(kw);pack.case(label,**values)
        core.CASES[-1].update(room=room,hit_at=hit_at,hit_thing=hit_thing,hit_amount=hit_amount,attack_at=attack_at,attack_thing=attack_thing,
                              push_at=push_at,push_x=push_x,push_y=push_y)
    north=dict(turn=16384,turn_hold=1);south=dict(turn=-16384,turn_hold=1);west=dict(turn=16384,turn_hold=2)
    northwest=dict(turn=24576,turn_hold=1);southeast=dict(turn=-8192,turn_hold=1)
    pistol=dict(attack_start=10,attack_end=200)
    # Wedge: a 45-degree wall, a 12:5 wall, a 5:1 wall, a convex 45/45 tip and a concave 45/axis corner.
    add('slide_45',0,[],px=-100,py=560,forward=50,move_hold=120,tics=150,**north)
    add('slide_22',0,[],px=600,py=-400,forward=50,move_hold=120,tics=150,**south)
    add('slide_shallow',0,[],px=0,py=-700,forward=50,move_hold=150,tics=180,**south)
    add('convex_corner',0,[],px=600,py=0,forward=50,move_hold=100,tics=120)
    add('concave_corner',0,[],px=-100,py=570,forward=50,move_hold=150,tics=180)
    add('fireball_diagonal_wall',0,[monster(IMP,-900,-200)],px=-100,py=560,attack_at=1,side=50,move_start=17,move_hold=56,tics=180)
    add('puff_diagonal_wall',0,[],px=600,py=-400,tics=60,attack_start=10,attack_end=40,**south)
    # Pillar: an octagonal one-sided pillar with axis and 45-degree faces in a square room.
    add('sight_past_pillar',1,[monster(POSSESSED,300,0,-180)],px=-300,side=50,move_hold=40,tics=120)
    # The pillar hides the player; a pistol shot's noise sets the monster chasing straight into it.
    add('zombie_around_pillar',1,[monster(POSSESSED,300,-100,0)],px=-300,attack_start=10,attack_end=40,tics=400)
    add('imp_around_pillar',1,[monster(IMP,300,0,0)],px=-300,attack_start=10,attack_end=40,tics=400)
    # Corridor: two rooms joined by a 45-degree corridor.
    add('corridor_pursuit',2,[monster(DEMON,600,560,-180)],px=-640,py=-560,side=-50,move_hold=50,tics=480)
    add('corridor_slide_east',2,[],px=-600,py=-400,forward=50,move_hold=150,tics=180)
    add('corridor_slide_west',2,[],px=600,py=400,forward=50,move_start=3,move_hold=150,tics=180,**west)
    # Step: a 45-degree two-sided riser of 48 units (reason 5) with a lower-wall puff and sight across it.
    add('riser_slide_puff',3,[monster(POSSESSED,-200,200,-180)],px=100,py=-200,attack_start=10,attack_end=40,forward=50,move_start=50,move_hold=150,tics=180,**north)
    # Window: a [40,88] band between two 45-degree two-sided lines, a floor-0 room and a 64-unit step.
    add('window_slide_low',4,[],px=0,py=-300,forward=50,move_hold=100,tics=120,**north)
    add('window_slide_high',4,[],px=-300,py=0,forward=50,move_hold=100,tics=120)
    add('window_upper_puff',4,[],px=-250,py=150,tics=60,attack_start=10,attack_end=40,**southeast)
    add('autoaim_up_window',4,[monster(POSSESSED,-250,150,-45)],px=150,py=-250,tics=120,**pistol,**northwest)
    add('autoaim_down_window',4,[monster(POSSESSED,150,-250,135)],px=-250,py=150,tics=120,**pistol,**southeast)
    add('noise_diagonal_window',4,[monster(POSSESSED,-250,150,135)],px=150,py=-250,tics=120,attack_start=10,attack_end=40,**northwest)
    # Corner: the 67.5-degree wedge of the investigation note, section 4.4.
    add('corner_jump',5,[],px=-31,py=-20,push_at=1,push_x=30,push_y=30,tics=40)
    add('corner_no_jump',5,[],px=-31,py=-20,forward=50,side=-50,move_hold=30,tics=40)
    return layouts
