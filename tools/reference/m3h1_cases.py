"""Bounded source inputs for original former humans and mixed encounters.

Every monster is an ordinary map THING spawned by the genuine P_SpawnMapThing;
weapons and items reuse the inherited player-world inputs. No Rust output."""
POSSESSED, SHOTGUY, CHAINGUY, DEMON = 3004, 9, 65, 3002
def populate(pack):
    core=pack.base;core.CASES.clear();layouts=[]
    def monster(kind,x,y=0,angle=0,options=7):return (x,y,angle,kind,options)
    def add(label,things=(),**kw):
        things=list(things)
        assert sum(t[3] in (POSSESSED,SHOTGUY,CHAINGUY,DEMON) for t in things)<=8
        if things not in layouts:layouts.append(things)
        values=dict(layout=layouts.index(things),drop=0,drop_x=36,drop_y=0,
                    move_start=1,physical=0,demon_last=0,part=3,tics=256,
                    thing=0,forward=0,move_hold=0,attack_end=0,seed=6)
        values.update(kw);pack.case(label,**values)
    west=dict(turn=16384,turn_hold=2)
    # Each former human alone against a passive player.
    add('possessed_alone',[monster(POSSESSED,-300)],tics=480)
    add('shotguy_alone',[monster(SHOTGUY,-300)],tics=480)
    add('chainguy_alone',[monster(CHAINGUY,-300)],tics=480)
    # Player kills each type, then walks over the source drop.
    add('possessed_killed_drop',[monster(POSSESSED,200,0,-180)],tics=320,attack_start=15,attack_end=150,forward=25,move_start=170,move_hold=230)
    add('shotguy_killed_drop',[monster(SHOTGUY,200,0,-180)],tics=320,ready=2,owned=7,shells=20,attack_start=15,attack_end=150,forward=25,move_start=170,move_hold=230)
    add('chainguy_killed_drop',[monster(CHAINGUY,200,0,-180)],tics=320,ready=3,owned=11,clips=100,attack_start=15,attack_end=150,forward=25,move_start=170,move_hold=230)
    # Mixed group against the genuine chaingun player.
    add('mixed_group',[monster(POSSESSED,-300,60),monster(POSSESSED,-300,-60),monster(SHOTGUY,-360,0),
        monster(CHAINGUY,-400,30),monster(DEMON,-250,0)],tics=720,ready=3,owned=11,clips=200,attack_start=10,attack_end=720,**west)
    # Infighting: a shotgunner fires through a Demon standing in its line of fire.
    add('infighting_shotguy_demon',[monster(DEMON,-150),monster(SHOTGUY,-300)],tics=720)
    add('infighting_chainguy_possessed',[monster(POSSESSED,-160),monster(CHAINGUY,-320)],tics=720)
    add('infighting_possessed_shotguy',[monster(SHOTGUY,-160),monster(POSSESSED,-320)],tics=720)
    # Passive player death against several former humans.
    add('player_death_mixed',[monster(POSSESSED,-300,40),monster(SHOTGUY,-300,-40),monster(CHAINGUY,-340,0)],tics=720)
    # Roster cap: eight monsters of all four admitted types.
    add('roster_eight',[monster(k,-260-40*(i%4),-90+60*(i//4)) for i,k in enumerate([POSSESSED,SHOTGUY,CHAINGUY,DEMON,POSSESSED,SHOTGUY,CHAINGUY,DEMON])],
        tics=480,ready=3,owned=11,clips=200,attack_start=10,attack_end=480,**west)
    # Multi-monster pursuit and monster-monster collision around a moving player.
    add('pursuit_collision',[monster(DEMON,-200,40),monster(DEMON,-200,-40),monster(POSSESSED,-260,0),monster(SHOTGUY,-320,0)],
        tics=480,side=8,move_start=20,move_hold=60,forward=0)
    # Kills credited from several weapons; drops collected afterwards.
    add('drops_collected',[monster(POSSESSED,160,30,-180),monster(SHOTGUY,160,-30,-180),monster(CHAINGUY,220,0,-180)],
        tics=512,ready=3,owned=11,clips=200,attack_start=10,attack_end=200,forward=6,move_start=260,move_hold=330)
    return layouts
