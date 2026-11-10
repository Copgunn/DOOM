//! M4a Episode 1 policy, plan step 6: the chainsaw and the rocket launcher
//! (`P_GiveWeapon`, the weapon change of `P_PlayerThink`, `A_Saw` with the lunge,
//! `A_FireMissile` with `P_SpawnPlayerMissile`, the rocket's `A_Explode` with self
//! damage and the in-action `P_DropWeapon`) and the Spectre. Rust-only policy
//! tests: the expected values come from the source reading (p_pspr.c, p_user.c,
//! p_mobj.c, p_inter.c, p_enemy.c, info.c, tables.h), never from the C engine; the
//! C-side checks are the DSIM39 goldens of the M4a synthetic WADs (SAW1, ROCKET1,
//! ROCKDIE, SPECT1, SPECT2, SPECTD).
use crate::diagonal_room::Lumps;
use crate::identity::{OriginalActorType, OriginalState};
use crate::world_pickups::PickupEvent;
use crate::*;

const EPISODE1: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V1;
const LEVEL: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersLevelV1;

/// The doorway room: the west room spans x −512..0, y −384..384.
fn room() -> Lumps {
    crate::player_movers_tests::room(0)
}
fn thing(l: &mut Lumps, x: i16, y: i16, angle: i16, kind: i16) {
    l.add_thing([x, y, angle, kind, 7]);
}
fn options() -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(6, 0),
        respawn_monsters: false,
    }
}
fn try_sim(l: &Lumps, policy: OriginalExecutionProfile) -> Result<Simulation, SimError> {
    let mut s = if policy == EPISODE1 {
        Simulation::original_player_episode1_map(l.map(), options())
    } else {
        Simulation::original_player_level_map(l.map(), options())
    }
    .map_err(|e| match e {
        OriginalRegistryError::Runtime(e) => e,
        other => panic!("{other:?}"),
    })?;
    s.spawn_map_things()?;
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())?;
    s.spawn_level_specials()?;
    Ok(s)
}
fn sim(l: &Lumps) -> Simulation {
    try_sim(l, EPISODE1).unwrap()
}
fn with(things: &[(i16, i16, i16, i16)]) -> Lumps {
    let mut l = room();
    for &(x, y, angle, kind) in things {
        thing(&mut l, x, y, angle, kind);
    }
    l
}
fn command(forward: i8, buttons: u8) -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0].forward = forward;
    c[0].buttons = buttons;
    c
}
fn runtime(s: &Simulation) -> &original_player::PlayerRuntime {
    s.original_player.as_ref().unwrap()
}
fn pack(s: &Simulation) -> PlayerWeaponState {
    runtime(s).weapon.unwrap().pack().unwrap()
}
fn body(s: &Simulation) -> Actor {
    *s.actor(runtime(s).mo.unwrap()).unwrap()
}
fn decode(s: &Simulation) {
    let capture = s.snapshot().canonical_bytes();
    Diagnostic::decode_episode1(&capture, DiagnosticLimits::default()).unwrap();
}
fn state(s: OriginalState) -> StateId {
    StateId(s.ordinal())
}
/// Replace the controlled loadout before tic 0: `ready` raised (its ready state
/// with one tic left), owning `owned`, with `rockets`.
fn arm(s: &mut Simulation, ready: Weapon, owned: u16, rockets: u16) {
    let p = s.original_player.as_mut().unwrap();
    let Some(player_weapons::PlayerWeaponRuntime::Pack(w)) = p.weapon.as_mut() else {
        panic!("pack");
    };
    w.ready = ready;
    w.pending = None;
    w.owned = owned;
    w.rockets = rockets;
    w.psprites[0].state = Some(ready.states()[2]);
    w.psprites[0].tics = 1;
    w.psprites[0].position = [Fixed::ONE, Fixed::from_map_units(32)];
}
fn live_of(s: &Simulation, ty: OriginalActorType) -> usize {
    s.actors.live_of_type(ActorTypeId(ty.ordinal()))
}

/// tables.h:68–71 and p_pspr.c:516–539: the raw `MELEERANGE+1` and the BAM
/// constants of `A_Saw`'s turn (`-ANG90/20` is an `int` converted to `angle_t`).
#[test]
fn saw_constants_are_the_source_values() {
    assert_eq!(player_pistol::SAW_RANGE.bits(), 64 * 65536 + 1);
    assert_eq!(player_pistol::ANG180, 0x8000_0000);
    assert_eq!(player_pistol::MINUS_ANG90_20, 0xFCCC_CCCD);
    assert_eq!(player_pistol::ANG90_20, 53_687_091);
    assert_eq!(player_pistol::ANG90_21, 51_130_563);
}

/// `P_GiveWeapon` (p_inter.c:168–222): the chainsaw gives the weapon and no ammo
/// and is refused once owned (it stays); the launcher gives two rocket loads
/// (2) and the weapon, and a second launcher still gives its rockets. Both
/// switch to the new weapon and assign their message; wire 22 refuses both.
#[test]
fn chainsaw_and_launcher_pickups() {
    let mut s = sim(&with(&[(-32, 0, 0, 1), (-40, 0, 0, 2005)]));
    s.advance(command(1, 0)).unwrap();
    let w = pack(&s);
    assert_eq!(w.owned, 3 | 1 << 7);
    assert_eq!(w.pending, Some(Weapon::Chainsaw));
    assert_eq!(w.ammo(), [50, 0, 0, 0]);
    assert_eq!(
        s.original_player_pickups(PlayerSlot::One).unwrap().message,
        PickupMessage::Chainsaw
    );
    decode(&s);
    let mut s = sim(&with(&[
        (-32, 0, 0, 1),
        (-40, 0, 0, 2005),
        (-40, 0, 0, 2005),
    ]));
    s.advance(command(1, 0)).unwrap();
    assert_eq!(live_of(&s, OriginalActorType::MT_MISC26), 1);

    let mut s = sim(&with(&[
        (-32, 0, 0, 1),
        (-40, 0, 0, 2003),
        (-40, 0, 0, 2003),
    ]));
    s.advance(command(1, 0)).unwrap();
    let w = pack(&s);
    assert_eq!(w.owned, 3 | 1 << 4);
    assert_eq!(w.rockets, 4);
    assert_eq!(w.pending, Some(Weapon::Rocket));
    assert_eq!(live_of(&s, OriginalActorType::MT_MISC27), 0);
    let gave: Vec<_> = s
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Pickup(PickupEvent::Weapon(w, gave, ..)) => Some((*w, *gave)),
            _ => None,
        })
        .collect();
    assert_eq!(gave, [(Weapon::Rocket, true), (Weapon::Rocket, true)]);
    decode(&s);
    for kind in [2005, 2003] {
        assert_eq!(
            try_sim(&with(&[(-32, 0, 0, 1), (-200, 0, 0, kind)]), LEVEL).err(),
            Some(SimError::MapSpawn(MapSpawnError::UnsupportedType(kind)))
        );
    }
}

/// The weapon change of `P_PlayerThink` (p_user.c:283–312) under wire 23: any
/// three-bit selection; key 1 (selection 0) picks the chainsaw once owned; an
/// unowned selection (5) is ignored. Wire 22 still refuses selection 4.
#[test]
fn weapon_selections_under_wire_23() {
    let l = with(&[(-32, 0, 0, 1)]);
    let mut s = sim(&l);
    arm(&mut s, Weapon::Pistol, 3 | 1 << 7, 0);
    s.advance(command(0, BT_CHANGE)).unwrap();
    assert_eq!(pack(&s).pending, Some(Weapon::Chainsaw));
    let mut s = sim(&l);
    arm(&mut s, Weapon::Pistol, 3 | 1 << 7, 0);
    s.advance(command(0, BT_CHANGE | 7 << 3)).unwrap();
    assert_eq!(pack(&s).pending, Some(Weapon::Chainsaw));
    let mut s = sim(&l);
    arm(&mut s, Weapon::Pistol, 3 | 1 << 7, 0);
    s.advance(command(0, BT_CHANGE | 5 << 3)).unwrap();
    assert_eq!(pack(&s).pending, None);
    // The chainsaw comes up after the pistol goes down.
    let mut s = sim(&l);
    arm(&mut s, Weapon::Pistol, 3 | 1 << 7, 0);
    s.advance(command(0, BT_CHANGE)).unwrap();
    for _ in 0..40 {
        s.advance(command(0, 0)).unwrap();
        decode(&s);
    }
    assert_eq!(pack(&s).ready, Weapon::Chainsaw);
    let mut w22 = try_sim(&l, LEVEL).unwrap();
    let before = w22.snapshot().canonical_bytes();
    assert!(w22.advance(command(0, BT_CHANGE | 4 << 3)).is_err());
    assert_eq!(w22.snapshot().canonical_bytes(), before);
}

/// `A_Saw` on an adjacent zombieman: a hit sets `MF_JUSTATTACKED`; the next
/// `P_PlayerThink` rewrites the command to turn 0, forward 100, side 0 and
/// clears the flag (p_user.c:249–255, decision D5); the victim gets no thrust
/// (p_inter.c:806–811).
#[test]
fn saw_hit_lunges_and_does_not_thrust() {
    let mut s = sim(&with(&[(-200, 0, 180, 1), (-250, 0, 0, 3004)]));
    arm(&mut s, Weapon::Chainsaw, 3 | 1 << 7, 0);
    let mut flagged = false;
    for _ in 0..12 {
        s.advance(command(0, BT_ATTACK)).unwrap();
        decode(&s);
        assert!(
            !s.events
                .iter()
                .any(|e| matches!(e, Event::Combat(combat::CombatEvent::Knockback { .. })))
        );
        if flagged {
            assert_eq!(s.commands[0].forward, 100);
            assert_eq!(s.commands[0].angle_turn, 0);
            assert_eq!(s.commands[0].buttons, BT_ATTACK);
            assert_eq!(
                body(&s).life.unwrap().flags & player_commands::MF_JUSTATTACKED,
                0
            );
            return;
        }
        flagged = body(&s).life.unwrap().flags & player_commands::MF_JUSTATTACKED != 0;
    }
    panic!("no saw hit");
}

/// The launcher does not fire from `A_WeaponReady` while `attackdown` is set
/// (p_pspr.c:315–323; `G_PlayerReborn` sets it), but once fired a held attack
/// refires through `A_ReFire` (p_pspr.c:348–360), every 20 tics (`S_MISSILE1` 8
/// + `S_MISSILE2` 12). The rockets fly west and explode on the far wall.
#[test]
fn launcher_waits_for_a_release_then_refires() {
    let mut s = sim(&with(&[(-32, 0, 180, 1)]));
    arm(&mut s, Weapon::Rocket, 3 | 1 << 4, 5);
    for _ in 0..10 {
        s.advance(command(0, BT_ATTACK)).unwrap();
    }
    assert_eq!(pack(&s).rockets, 5);
    s.advance(command(0, 0)).unwrap();
    let mut exploded = false;
    for _ in 0..60 {
        s.advance(command(0, BT_ATTACK)).unwrap();
        decode(&s);
        exploded |= s.actors.order().any(|id| {
            s.actor(id).and_then(Actor::life).is_some_and(|l| {
                l.actor_type.0 == OriginalActorType::MT_ROCKET.ordinal()
                    && l.state == Some(state(OriginalState::S_EXPLODE1))
            })
        });
    }
    assert!(exploded);
    assert_eq!(pack(&s).rockets, 2);
    assert_eq!(runtime(&s).health, 100);
}

/// ROCKDIE in miniature: a rocket fired at a touching wall explodes inside
/// `P_CheckMissileSpawn` (p_mobj.c:880–881); its 128 splash with the shooter as
/// the source kills the player inside `A_FireMissile`, and the player's
/// `P_DropWeapon` (p_inter.c:715) leaves the weapon psprite in `S_MISSILEDOWN`.
#[test]
fn rocket_at_a_touching_wall_kills_the_shooter() {
    let mut s = sim(&with(&[(-496, 0, 180, 1)]));
    arm(&mut s, Weapon::Rocket, 3 | 1 << 4, 2);
    s.advance(command(0, 0)).unwrap();
    for _ in 0..12 {
        s.advance(command(0, BT_ATTACK)).unwrap();
        decode(&s);
        if runtime(&s).state == PlayerBodyState::Dead {
            let w = pack(&s);
            assert_eq!(w.rockets, 1);
            assert_eq!(
                w.psprites[0].state,
                Some(state(OriginalState::S_MISSILEDOWN))
            );
            assert_eq!(runtime(&s).frags, 1);
            assert_eq!(runtime(&s).attacker, runtime(&s).mo);
            assert_eq!(runtime(&s).weapon_stage, original_player::WeaponStage::Idle);
            return;
        }
    }
    panic!("the shooter survived");
}

/// The Spectre (doomednum 58) loads under wire 23 only, as a counted monster
/// with `MF_SHADOW` and exactly the Demon's state roles (info.c:1420–1470).
#[test]
fn spectre_is_a_shadowed_demon_under_wire_23_only() {
    let l = with(&[(-32, 0, 0, 1), (-300, 0, 0, 58)]);
    let s = sim(&l);
    decode(&s);
    let spectre = ActorTypeId(OriginalActorType::MT_SHADOWS.ordinal());
    let id = s
        .actors
        .order()
        .find(|id| s.actor(*id).unwrap().life.unwrap().actor_type == spectre)
        .unwrap();
    let flags = s.actor(id).unwrap().life.unwrap().flags;
    assert_ne!(flags & original_player::MF_SHADOW, 0);
    assert_ne!(flags & world_pickups::MF_COUNTKILL, 0);
    let demon = ActorTypeId(OriginalActorType::MT_SERGEANT.ordinal());
    for st in OriginalState::all() {
        let id = StateId(st.ordinal());
        assert_eq!(
            original_registry::state_role_for(EPISODE1, spectre, id).is_ok(),
            original_registry::state_role_for(EPISODE1, demon, id).is_ok(),
            "{st:?}"
        );
    }
    assert!(original_registry::is_policy_monster(EPISODE1, spectre));
    assert!(!original_registry::is_policy_monster(LEVEL, spectre));
    assert_eq!(
        try_sim(&l, LEVEL).err(),
        Some(SimError::MapSpawn(MapSpawnError::UnsupportedType(58)))
    );
}

/// The rocket's role: `S_ROCKET` and the `S_EXPLODE` chain, under wire 23 only.
#[test]
fn rocket_role_is_episode1_only() {
    let rocket = ActorTypeId(OriginalActorType::MT_ROCKET.ordinal());
    for st in [
        OriginalState::S_ROCKET,
        OriginalState::S_EXPLODE1,
        OriginalState::S_EXPLODE2,
        OriginalState::S_EXPLODE3,
    ] {
        assert!(original_registry::state_role_for(EPISODE1, rocket, state(st)).is_ok());
        assert!(original_registry::state_role_for(LEVEL, rocket, state(st)).is_err());
    }
    assert!(
        original_registry::state_role_for(EPISODE1, rocket, state(OriginalState::S_BEXP4)).is_err()
    );
}

// ---- M4a mutation-testing step (run 1, shard 1) ---------------------------
//
// Observable-behaviour tests for mutants that survived the whole suite. The
// loadouts come from pickups and weapon changes through `advance`; the reads are
// the public `original_player_weapons`, `events`, `actor` and the snapshot.

/// The empty loadout: pistol ready with no bullets, fist and pistol owned.
const EMPTY: WeaponInventory = WeaponInventory {
    ready: Weapon::Pistol,
    owned: 3,
    ammo: [0; 4],
};
fn armed(l: &Lumps, inventory: WeaponInventory) -> Simulation {
    let mut s = Simulation::original_player_episode1_map(l.map(), options()).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, inventory)
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn weapons(s: &Simulation) -> PlayerWeaponState {
    s.original_player_weapons(PlayerSlot::One).unwrap().unwrap()
}
/// `ready` is up (`WEAPONTOP`, 32 units, p_pspr.c `A_Raise`) with nothing pending.
fn raised(s: &Simulation, ready: Weapon) -> bool {
    let w = weapons(s);
    w.ready == ready
        && w.pending.is_none()
        && w.psprites[0].position[1] == Fixed::from_map_units(32)
}
/// Advances with `commands` until `done` holds, at most `limit` tics.
fn until(
    s: &mut Simulation,
    commands: [TicCommand; 4],
    limit: usize,
    done: impl Fn(&Simulation) -> bool,
) {
    for _ in 0..limit {
        if done(s) {
            return;
        }
        s.advance(commands).unwrap();
        decode(s);
    }
    assert!(done(s), "not reached in {limit} tics");
}
fn select(weapon: u8) -> [TicCommand; 4] {
    command(0, BT_CHANGE | weapon << 3)
}
fn sounded(s: &Simulation, name: &str) -> bool {
    let id = lifecycle::source_sound_id(name);
    s.events().iter().any(|e| {
        matches!(e, Event::ActorState(lifecycle::ActorStateEvent::Sound { sound, .. }) if *sound == id)
    })
}

/// `P_CheckAmmo`'s fallback order (p_pspr.c:197–229): with no clip and no shell
/// the chainsaw comes before the launcher (211–214). The pistol is ready with
/// no bullets after the chainsaw was picked up and the pistol selected again;
/// firing it fails the ammo check and names the chainsaw.
#[test]
fn empty_pistol_falls_back_to_an_owned_chainsaw() {
    let mut s = armed(&with(&[(-32, 0, 180, 1), (-40, 0, 0, 2005)]), EMPTY);
    s.advance(command(1, 0)).unwrap();
    assert_eq!(weapons(&s).owned, 3 | 1 << 7);
    until(&mut s, command(0, 0), 100, |s| raised(s, Weapon::Chainsaw));
    s.advance(select(1)).unwrap();
    until(&mut s, command(0, 0), 100, |s| raised(s, Weapon::Pistol));
    s.advance(command(0, BT_ATTACK)).unwrap();
    assert_eq!(weapons(&s).pending, Some(Weapon::Chainsaw));
    decode(&s);
}

/// `P_CheckAmmo` (p_pspr.c:215–219): the launcher is the fallback only when it
/// is owned and has rockets. Owning it with two rockets (the pickup's two
/// loads, p_inter.c `P_GiveWeapon`), an empty pistol falls back to it; with a
/// rocket but no launcher (a rocket pickup does not switch from the pistol,
/// p_inter.c:149–154) the fallback is the fist (225–228).
#[test]
fn empty_pistol_falls_back_to_the_launcher_only_with_both() {
    let mut s = armed(&with(&[(-32, 0, 180, 1), (-40, 0, 0, 2003)]), EMPTY);
    s.advance(command(1, 0)).unwrap();
    assert_eq!((weapons(&s).owned, weapons(&s).rockets), (3 | 1 << 4, 2));
    until(&mut s, command(0, 0), 100, |s| raised(s, Weapon::Rocket));
    s.advance(select(1)).unwrap();
    until(&mut s, command(0, 0), 100, |s| raised(s, Weapon::Pistol));
    s.advance(command(0, BT_ATTACK)).unwrap();
    assert_eq!(weapons(&s).pending, Some(Weapon::Rocket));
    decode(&s);

    let mut s = armed(&with(&[(-32, 0, 180, 1), (-40, 0, 0, 2010)]), EMPTY);
    s.advance(command(1, 0)).unwrap();
    assert_eq!((weapons(&s).owned, weapons(&s).rockets), (3, 1));
    until(&mut s, command(0, 0), 100, |s| raised(s, Weapon::Pistol));
    s.advance(command(0, BT_ATTACK)).unwrap();
    assert_eq!(weapons(&s).pending, Some(Weapon::Fist));
    decode(&s);
}

/// `A_WeaponReady` plays `sfx_sawidl` only when the chainsaw's psprite is in
/// `S_SAW` (p_pspr.c:296–299). `S_SAW` and `S_SAWB` both run `A_WeaponReady`
/// on entry, 4 tics each, and alternate (info.c:203–204), so the idle sound
/// comes on every `S_SAW` entry (every 8 tics) and never on an `S_SAWB` one.
#[test]
fn chainsaw_idle_sound_only_on_s_saw_entries() {
    let mut s = armed(&with(&[(-32, 0, 180, 1), (-40, 0, 0, 2005)]), EMPTY);
    s.advance(command(1, 0)).unwrap();
    let mut sounds = 0;
    for _ in 0..80 {
        s.advance(command(0, 0)).unwrap();
        let w = weapons(&s);
        let entered =
            |st: OriginalState| w.psprites[0].state == Some(state(st)) && w.psprites[0].tics == 4;
        assert_eq!(sounded(&s, "sfx_sawidl"), entered(OriginalState::S_SAW));
        assert!(!(entered(OriginalState::S_SAWB) && sounded(&s, "sfx_sawidl")));
        sounds += usize::from(sounded(&s, "sfx_sawidl"));
    }
    assert!(raised(&s, Weapon::Chainsaw));
    assert!(sounds >= 5, "{sounds}");
}

/// Key 1 with the chainsaw ready and no berserk (`pw_strength` is never given
/// in Episode 1 single player): `newweapon` becomes the chainsaw again
/// (p_user.c:290–296), which is the ready weapon, so nothing changes.
#[test]
fn key_1_with_the_chainsaw_ready_keeps_it() {
    let mut s = armed(&with(&[(-32, 0, 180, 1), (-40, 0, 0, 2005)]), EMPTY);
    s.advance(command(1, 0)).unwrap();
    until(&mut s, command(0, 0), 100, |s| raised(s, Weapon::Chainsaw));
    s.advance(select(0)).unwrap();
    assert_eq!(weapons(&s).pending, None);
    for _ in 0..30 {
        s.advance(command(0, 0)).unwrap();
    }
    assert!(raised(&s, Weapon::Chainsaw));
}

/// The initial loadout names only the four weapons the older policies give
/// (`WeaponInventory`: an initial *supported* ready weapon; the launcher and the
/// chainsaw come from pickups under wire 23): a ready launcher (4) or chainsaw
/// (7) is the unsupported-weapon error, under wire 23 too.
#[test]
fn initial_loadout_cannot_ready_the_launcher_or_the_chainsaw() {
    for (ready, bit) in [(Weapon::Rocket, 4), (Weapon::Chainsaw, 7)] {
        let mut s =
            Simulation::original_player_episode1_map(with(&[(-32, 0, 0, 1)]).map(), options())
                .unwrap();
        s.spawn_map_things().unwrap();
        let inventory = WeaponInventory {
            ready,
            owned: 3 | 1 << bit,
            ammo: [50, 0, 0, 0],
        };
        assert_eq!(
            s.initialize_player_weapons(PlayerSlot::One, inventory),
            Err(SimError::Weapon(WeaponError::UnsupportedWeapon)),
            "{ready:?}"
        );
    }
}

/// `P_KillMobj` asks for `P_DropWeapon` (p_inter.c:707), which sets the ready
/// weapon's `downstate` (p_pspr.c:265–270): `S_SAWDOWN` for the chainsaw and
/// `S_MISSILEDOWN` for the launcher (d_items.c:89, 116). Killed by the special-16
/// floor (20 points every 32 tics, p_spec.c:1036–1047) with each weapon ready,
/// the `Died` event names that state.
#[test]
fn death_names_the_chainsaw_and_launcher_downstates() {
    for (pickup, weapon, down) in [
        (2005, Weapon::Chainsaw, OriginalState::S_SAWDOWN),
        (2003, Weapon::Rocket, OriginalState::S_MISSILEDOWN),
    ] {
        let mut l = with(&[(-32, 0, 180, 1), (-40, 0, 0, pickup)]);
        let sectors = l
            .lumps
            .iter()
            .find(|(n, _)| n == "SECTORS")
            .unwrap()
            .1
            .len()
            / 26;
        for sector in 0..sectors {
            let lump = &mut l.lumps.iter_mut().find(|(n, _)| n == "SECTORS").unwrap().1;
            lump[26 * sector + 22..26 * sector + 24].copy_from_slice(&16i16.to_le_bytes());
        }
        let mut s = armed(&l, EMPTY);
        s.advance(command(1, 0)).unwrap();
        let mut died = None;
        for _ in 0..200 {
            s.advance(command(0, 0)).unwrap();
            died = s.events().iter().find_map(|e| match e {
                Event::PlayerBody(PlayerBodyEvent::Died { weapon_down, .. }) => Some(*weapon_down),
                _ => None,
            });
            if died.is_some() {
                assert_eq!(weapons(&s).ready, weapon);
                break;
            }
        }
        assert_eq!(died, Some(down), "{weapon:?}");
    }
}

/// `A_Saw`'s turn (p_pspr.c:524–541): with the target more than `ANG90/20`
/// counter-clockwise of the facing (and less than `ANG180`), the player turns to
/// `angle - ANG90/21` (537–538), not by `ANG90/20`. A barrel (shootable, and it
/// never moves) stands about 10° to the left of the east-facing player, inside
/// the saw's reach; the
/// expected angle is `R_PointToAngle2` of the two positions before the hit tic,
/// taken here from `atan2` within 2²⁰ BAM (the tantoangle table's resolution is
/// about 2¹⁸ there), minus `ANG90/21` = 51 130 563 (tables.h `ANG90` 0x40000000).
#[test]
fn saw_turns_to_the_target_minus_ang90_21() {
    let mut s = armed(
        &with(&[(-300, 0, 0, 1), (-292, 0, 0, 2005), (-251, 9, 0, 2035)]),
        EMPTY,
    );
    let find = |s: &Simulation, ty: OriginalActorType| {
        s.actor_order()
            .find(|id| {
                s.actor(*id)
                    .and_then(Actor::life)
                    .is_some_and(|l| l.actor_type.0 == ty.ordinal())
            })
            .unwrap()
    };
    let player = find(&s, OriginalActorType::MT_PLAYER);
    let zombie = find(&s, OriginalActorType::MT_BARREL);
    s.advance(command(1, 0)).unwrap();
    until(&mut s, command(0, 0), 100, |s| raised(s, Weapon::Chainsaw));
    for _ in 0..40 {
        let (p, z) = (*s.actor(player).unwrap(), *s.actor(zombie).unwrap());
        let current = p.spec.angle;
        s.advance(command(0, BT_ATTACK)).unwrap();
        if !sounded(&s, "sfx_sawhit") {
            continue;
        }
        let [dx, dy] = [0, 1].map(|i| f64::from(z.position()[i].bits() - p.position()[i].bits()));
        let bam = |radians: f64| {
            (radians / std::f64::consts::TAU * 4_294_967_296.0).rem_euclid(4_294_967_296.0)
        };
        let angle = bam(dy.atan2(dx));
        let delta = (angle - f64::from(current)).rem_euclid(4_294_967_296.0);
        // Between 6° and 30° counter-clockwise: well inside the (ANG90/20, ANG180] branch.
        assert!((71_582_788.0..357_913_941.0).contains(&delta), "{delta}");
        let expected = angle - 51_130_563.0;
        let turned = f64::from(s.actor(player).unwrap().spec.angle);
        assert!(
            (turned - expected).abs() < f64::from(1u32 << 20),
            "{turned} {expected}"
        );
        return;
    }
    panic!("no saw hit");
}

/// `A_Saw`'s turn at its two exact boundaries (p_pspr.c:524–541). In a
/// corridor exactly as wide as the player (32 units), a barrel stands due east
/// on the player's line, so `R_PointToAngle2` of the two is exactly 0
/// (`tantoangle[SlopeDiv(0, dx)]` = `tantoangle[0]` = 0, tables.c:1870); the
/// lunges slide along the walls (`P_HitSlideLine` zeroes `tmymove` on a
/// horizontal line) and stop at the barrel, so the bearing stays 0. The barrel
/// starts 55 units ahead: a trace meets a thing on a diagonal through its
/// centre (`PIT_AddThingIntercepts`, p_maputl.c:632–663), so the centre must be
/// within about `MELEERANGE+1`. The chainsaw gives no thrust (p_inter.c:805–810), so the
/// barrel stays put; it has 20 health (info.c:1891), and the seed is one whose
/// first three saw hits (2–20 each, p_pspr.c:510) total less than 20, so it is
/// still standing for the fourth. Facing 0,
/// a hit has delta 0: `+= ANG90/20` (540). Then the delta is exactly −ANG90/20
/// (`0xFCCCCCCD`): above `ANG180` and not below `-ANG90/20`, so `-= ANG90/20`
/// (533), back to 0, not `angle + ANG90/21`. A turn of +1 (`angleturn` 1 << 16)
/// and a hit (delta −2¹⁶: `-= ANG90/20`), then a turn of −1, leave the facing at
/// −ANG90/20; the next delta is exactly +ANG90/20, not above it: `+= ANG90/20`
/// (540), back to 0, not `angle − ANG90/21`.
#[test]
fn saw_turn_at_exactly_ang90_20_either_way() {
    use crate::heights_room::HeightsRoom;
    let mut corridor = HeightsRoom::new(&[-512, 512], &[[0, 128]]).with_things(&[
        [-400, 0, 0, 1, 7],
        [-392, 0, 0, 2005, 7],
        [-345, 0, 0, 2035, 7],
    ]);
    corridor.y = [-16, 16];
    let options = MapContextOptions {
        initial_rng: DoomRng::from_indices(SAW_SEED, 0),
        respawn_monsters: false,
    };
    let mut s = Simulation::original_player_episode1_map(corridor.map(), options).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, EMPTY).unwrap();
    s.spawn_level_specials().unwrap();
    let player = s
        .actor_order()
        .find(|id| {
            s.actor(*id)
                .and_then(Actor::life)
                .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_PLAYER.ordinal())
        })
        .unwrap();
    s.advance(command(1, 0)).unwrap();
    until(&mut s, command(0, 0), 100, |s| raised(s, Weapon::Chainsaw));
    const A: u32 = 53_687_091;
    // Facing after each hit, and the turn applied two tics after it (the tic
    // after a hit is the lunge, whose `angleturn` is rewritten to 0).
    let plan: [(u32, i16); 4] = [(A, 0), (0, 1), (65_536u32.wrapping_sub(A), -1), (0, 0)];
    let mut hits = 0;
    let mut turn_at = None;
    for tic in 0..200 {
        let mut c = command(0, BT_ATTACK);
        if turn_at == Some(tic) {
            c[0].angle_turn = plan[hits - 1].1;
        }
        s.advance(c).unwrap();
        if sounded(&s, "sfx_sawhit") {
            assert_eq!(
                s.actor(player).unwrap().spec.angle,
                plan[hits].0,
                "hit {hits}"
            );
            assert_eq!(s.actor(player).unwrap().position()[1], Fixed::ZERO);
            hits += 1;
            if hits == plan.len() {
                return;
            }
            turn_at = Some(tic + 2);
        }
    }
    panic!("only {hits} saw hits");
}
const SAW_SEED: u8 = 11;

// ---- M4a mutation-testing step (run 1, shard 1): adapter and missile witnesses
//
// Rust-only safety tests: each corrupts one value from inside the crate (no
// public operation produces it) and checks that the witness refuses it.

/// The weapon adapter's identity rule (`PlayerWeaponState::after_adapter`: the
/// weapon machine never changes the adapter's actor, slot or maxima): the
/// unchanged adapter is accepted, and an adapter naming another actor, another
/// slot or other maxima is refused alone as `WeaponError::Inventory`.
///
/// Refusal witness below the public API: `PlayerWeaponState::after_adapter`
/// (player_weapons.rs:87–88, the adapter identity rule). The weapon machine never
/// changes the adapter's actor, slot or maxima, so the public API cannot produce such
/// an adapter; it is built from inside the crate.
#[test]
fn a_weapon_adapter_with_another_actor_slot_or_maxima_is_refused() {
    let s = sim(&with(&[(-32, 0, 0, 1), (200, 0, 0, 2014)]));
    let mo = runtime(&s).mo.unwrap();
    let other = s.actor_order().find(|id| *id != mo).unwrap();
    let w = pack(&s);
    assert!(w.after_adapter(w.adapter(mo, 100), mo, true).is_ok());
    let mut actor = w.adapter(mo, 100);
    actor.actor = other;
    let mut slot = w.adapter(mo, 100);
    slot.slot = PlayerSlot::One.ordinal() + 1;
    let mut maxima = w.adapter(mo, 100);
    maxima.max_ammo[0] += 1;
    for (name, bad) in [("actor", actor), ("slot", slot), ("maxima", maxima)] {
        assert_eq!(
            w.after_adapter(bad, mo, true).err(),
            Some(SimError::Weapon(WeaponError::Inventory)),
            "{name}"
        );
    }
}

/// A flying rocket's owner: `P_SpawnPlayerMissile` sets `th->target = source`
/// (p_mobj.c:978) and the owner-removal preflight keeps it live, so the
/// live-missile witness of the tic preflight refuses a flying rocket without
/// an owner as `ProjectileError::Owner`.
///
/// Refusal witness below the public API: `Simulation::preflight_live_missiles`
/// (projectiles.rs:855, the live-missile owner witness) via `advance`.
/// `P_SpawnPlayerMissile` always sets the owner and the owner-removal preflight keeps
/// it live, so no map yields an ownerless flying rocket; the owner is cleared from
/// inside the crate.
#[test]
fn a_flying_rocket_without_an_owner_is_refused() {
    let mut s = sim(&with(&[(-32, 0, 180, 1)]));
    arm(&mut s, Weapon::Rocket, 3 | 1 << 4, 5);
    s.advance(command(0, 0)).unwrap();
    let rocket = ActorTypeId(OriginalActorType::MT_ROCKET.ordinal());
    let mut flying = None;
    for _ in 0..20 {
        s.advance(command(0, BT_ATTACK)).unwrap();
        flying = s.actor_order().find(|id| {
            s.actor(*id).and_then(Actor::life).is_some_and(|l| {
                l.actor_type == rocket && l.flags & original_player::MF_MISSILE != 0
            })
        });
        if flying.is_some() {
            break;
        }
    }
    let flying = flying.expect("a flying rocket");
    assert_eq!(s.preflight_live_missiles(), Ok(()));
    s.actors
        .get_mut(flying)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .target = None;
    assert_eq!(
        s.preflight_live_missiles(),
        Err(SimError::Projectile(ProjectileError::Owner))
    );
}

/// Pins the mutation `Simulation::validate_entry` `&&` -> `||` (spawning.rs)
/// recorded as equivalent in docs/validation/M4A.md. Written admission rule:
/// under the Imp-and-later policies a monster's states are admitted by the demon
/// state-entry validator (`enter_demon_state`), and the wire-23 roster includes
/// `PlayerMonstersEpisode1V1` (player_roster.rs), so entering an Imp's `S_TROO_RUN1` through
/// the public `enter_actor_state` is `Ok`. Under the mutant every wire-23 state
/// goes to the rocket validator, which refuses monster states
/// (`Actor(UnsupportedAction(ActionId(15)))`).
#[test]
fn a_wire_23_imp_can_enter_its_run_state() {
    let mut s = sim(&with(&[(-32, 0, 0, 1), (-300, 0, 0, 3001)]));
    let imp = s
        .actors
        .order()
        .find(|id| {
            s.actor(*id)
                .and_then(Actor::life)
                .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_TROOP.ordinal())
        })
        .unwrap();
    assert_eq!(
        s.enter_actor_state(imp, state(OriginalState::S_TROO_RUN1)),
        Ok(())
    );
}

/// Pins the mutation `Simulation::spawn_admitted` `&&` -> `||` in the
/// `has_episode1()` branch (player_map_scale.rs) recorded as equivalent in
/// docs/validation/M4A.md. Written admission rule: the spawn rule S′
/// (COMPATIBILITY.md, "The spawn rule S′"; `Boundary::spawn_admitted_relaxed`):
/// an XY mover (monster) whose open box meets a wall needs its center at least
/// `(31 - r)` units in front of the wall's line for an axis-aligned wall; the Imp's
/// radius is 20 (info.c MT_TROOP), so the minimum is 11. In the doorway room (west
/// wall at x = -512) an Imp 5 units from the wall is refused with
/// `MonsterError::Arithmetic`, and one 15 units from it is admitted. Wire 23
/// derives a boundary here, and the boundary clause is optional (decision D4, so
/// room containment alone applies without one); under the mutant the
/// room-containment clause alone admits the spawn, so the 5-unit Imp is spawned.
#[test]
fn a_wire_23_imp_too_close_to_a_wall_is_refused_at_spawn() {
    let near = with(&[(-32, 0, 0, 1), (-507, 0, 0, 3001)]);
    let mut s = Simulation::original_player_episode1_map(near.map(), options()).unwrap();
    assert!(s.boundary.is_some());
    assert_eq!(
        s.spawn_map_things().err(),
        Some(SimError::Monster(MonsterError::Arithmetic))
    );
    let far = with(&[(-32, 0, 0, 1), (-497, 0, 0, 3001)]);
    let mut s = Simulation::original_player_episode1_map(far.map(), options()).unwrap();
    assert!(s.boundary.is_some());
    assert_eq!(s.spawn_map_things().map(|_| ()), Ok(()));
}
