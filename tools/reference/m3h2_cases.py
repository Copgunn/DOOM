"""Bounded source inputs for the original Imp, its fireball and mixed rosters.

Every monster is an ordinary map THING spawned by the genuine P_SpawnMapThing;
weapons and items reuse the inherited player-world inputs. Two controlled
between-tic inputs exist only where a case declares them: `hit_*` is one
P_DamageMobj(owned[hit_thing],NULL,NULL,hit_amount), the only way to reach an Imp
XDeath with the admitted damage range; `attack_*` assigns the player target and
enters S_TROO_ATK1, the only way an overlapped Imp reaches a blocked spawn check.
No Rust output."""
IMP,POSSESSED,SHOTGUY,CHAINGUY,DEMON=3001,3004,9,65,3002
HEALTH_BONUS=2014
ROSTER=(IMP,POSSESSED,SHOTGUY,CHAINGUY,DEMON)
def populate(pack):
    core=pack.base;core.CASES.clear();layouts=[]
    def monster(kind,x,y=0,angle=0,options=7):return (x,y,angle,kind,options)
    def add(label,things=(),hit_at=0,hit_thing=0,hit_amount=0,attack_at=0,attack_thing=0,**kw):
        things=list(things)
        assert sum(t[3] in ROSTER for t in things)<=8
        if things not in layouts:layouts.append(things)
        values=dict(layout=layouts.index(things),drop=0,drop_x=36,drop_y=0,
                    move_start=1,physical=0,demon_last=0,part=3,tics=256,
                    thing=0,forward=0,move_hold=0,attack_end=0,seed=6)
        values.update(kw);pack.case(label,**values)
        core.CASES[-1].update(hit_at=hit_at,hit_thing=hit_thing,hit_amount=hit_amount,attack_at=attack_at,attack_thing=attack_thing)
    west=dict(turn=16384,turn_hold=2)
    chaingun=dict(ready=3,owned=11,clips=200)
    # Melee/missile choice at varied ranges against a passive player.
    add('imp_melee_contact',[monster(IMP,-50)],tics=240)
    add('imp_melee_boundary',[monster(IMP,-59)],tics=120)
    add('imp_missile_medium',[monster(IMP,-200)],tics=360)
    add('imp_missile_far',[monster(IMP,-600)],tics=480)
    add('imp_varied_ranges',[monster(IMP,-55,40),monster(IMP,-140,-80),monster(IMP,-400,120)],tics=480)
    # Fireball hit, wall explosion and dodging against a moving player.
    add('fireball_wall_miss',[monster(IMP,-300)],tics=360,side=30,move_start=30,move_hold=70)
    add('player_dodge',[monster(IMP,-300,60)],tics=480,side=40,move_start=45,move_hold=52,forward=25)
    # The fireball meets a third party in its line of flight.
    add('fireball_hits_former_human',[monster(POSSESSED,-160),monster(IMP,-320)],tics=720)
    add('fireball_hits_shotguy',[monster(SHOTGUY,-160),monster(IMP,-320)],tics=720)
    add('fireball_hits_demon',[monster(DEMON,-160),monster(IMP,-320)],tics=720)
    add('fireball_hits_imp',[monster(IMP,-160),monster(IMP,-320)],tics=480)
    # P_CheckMissileSpawn contact: an ambush former human facing away sleeps inside
    # the launcher's half-step. See the investigation for why this needs overlap.
    add('fireball_spawn_blocked',[monster(POSSESSED,-270,0,-180,15),monster(IMP,-300)],tics=240,attack_at=30,attack_thing=1)
    add('fireball_passes_item',[(-150,0,0,HEALTH_BONUS,7),monster(IMP,-300)],tics=360)
    add('former_human_shoots_imp',[monster(IMP,-160),monster(CHAINGUY,-320)],tics=720)
    # Imp deaths; the source damage range for XDeath needs the controlled hit.
    add('imp_killed_pistol',[monster(IMP,200,0,-180)],tics=420,attack_start=15,attack_end=240,clips=60)
    add('imp_xdeath',[monster(IMP,-200)],tics=120,hit_at=20,hit_thing=0,hit_amount=130)
    add('imp_dies_with_fireball_flying',[monster(POSSESSED,-160),monster(IMP,-320)],tics=360,hit_at=72,hit_thing=1,hit_amount=60)
    # Player death by fireball, and the source's reaction to the dead target.
    add('player_death_fireball',[monster(IMP,-300,40),monster(IMP,-300,-40),monster(IMP,-340,0)],tics=960)
    # Mixed rosters against the genuine chaingun player.
    add('mixed_roster',[monster(IMP,-300,60),monster(IMP,-300,-60),monster(POSSESSED,-360,20),monster(SHOTGUY,-360,-20),
        monster(CHAINGUY,-400,30),monster(DEMON,-250,0),monster(IMP,-420,-50),monster(POSSESSED,-440,60)],tics=720,attack_start=10,attack_end=720,**chaingun,**west)
    add('roster_eight_imps',[monster(IMP,-260-40*(i%4),-90+60*(i//4)) for i in range(8)],tics=480,attack_start=10,attack_end=480,**chaingun,**west)
    add('imps_pursuit',[monster(IMP,-200,40),monster(IMP,-200,-40),monster(DEMON,-260,0),monster(SHOTGUY,-320,0)],tics=480,side=8,move_start=20,move_hold=60)
    return layouts
