//! M3h9b plan step 6: exploding barrels under wire 21 (`MT_BARREL`, `A_Explode`,
//! `P_RadiusAttack`, `PIT_RadiusAttack`). Every expected value is derived from the
//! linuxdoom-1.10 source reading of M3H9B_REFERENCE_INVESTIGATION.md §2 (cited per
//! test) and the info.c record, never from Rust output: distances, damages,
//! thrusts, timers and the 3 × 3 block window are computed here from the cited
//! formulas. The rooms are the committed M3h9 rooms with their THINGs replaced.
use super::*;
use crate::barrels::{
    BLAST_WINDOW, BOMB_DAMAGE, EXPLOSION_SIGHT_CANDIDATES, EXPLOSION_SIGHT_QUERIES,
};
use crate::identity::{OriginalActorType, OriginalState};
use crate::lifecycle::ActorStateEvent;
use crate::original_player::{MF_CORPSE, MF_DROPOFF, MF_NOBLOOD, MF_SHOOTABLE, MF_SOLID};
use crate::player_e1m1_tests::room;
use crate::world_pickups::MF_AMBUSH;

const BARREL: i16 = 2035;
const ZOMBIE: i16 = 3004;
const IMP: i16 = 3001;
/// The census room (map 12): one sector `x ∈ [−512, 512]`, `y ∈ [−384, 384]`,
/// floor 0, ceiling 128, BLOCKMAP origin (−512, −384), 9 × 7 cells of 128.
const CENSUS: usize = 12;
/// The gallery room (map 5): a hall, a north ledge (`y ∈ [256, 384]`, sector 1,
/// floor 32) and two pedestals; no line or sector special.
const GALLERY: usize = 5;
/// The liftroom (map 10): halls `L0a`/`L0b` (floor 2) split by a WR 88 line at
/// `y = 128` (tag 1, the lift `P`), a DR door `D` at `x ∈ [512, 576]`,
/// `y ∈ [32, 96]` (sector 5, floor 2, ceiling 124).
const LIFTROOM: usize = 10;
const MT_BARREL: u16 = OriginalActorType::MT_BARREL.ordinal();
/// info.c:1910: `MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD`.
const BARREL_FLAGS: u32 = MF_SOLID | MF_SHOOTABLE | MF_NOBLOOD;
/// p_inter.c:675–680: `MF_SHOOTABLE` cleared, `MF_CORPSE|MF_DROPOFF` set,
/// `MF_SOLID` kept for a non-player.
const DEAD_FLAGS: u32 = MF_SOLID | MF_NOBLOOD | MF_CORPSE | MF_DROPOFF;
/// p_inter.c:817 for mass 100 (and the player's 100): `damage * (FRACUNIT >> 3)`.
const THRUST_PER_DAMAGE: i32 = 8192;

fn lumps(index: usize, things: &[[i16; 5]]) -> crate::diagonal_room::Lumps {
    let mut l = room(index);
    l.clear_things();
    for t in things {
        l.add_thing(*t);
    }
    l
}
fn options(seed: u8) -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(seed, 0),
        respawn_monsters: false,
    }
}
/// The C driver's setup order under wire 21: THINGs, the player body facing east,
/// the default weapons, `P_SpawnSpecials`, reaction time 0.
fn world_in(
    l: &crate::diagonal_room::Lumps,
    player: [i16; 2],
    seed: u8,
) -> (Simulation, ActorId, Vec<ActorId>) {
    let mut s = Simulation::original_player_barrels_map(l.map(), options(seed)).unwrap();
    let spawned = s.spawn_map_things().unwrap().spawned;
    let body = s
        .spawn_player_body(PlayerSlot::One, player.map(Fixed::from_map_units))
        .unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s.set_initial_player_reaction_time(PlayerSlot::One, 0)
        .unwrap();
    (s, body, spawned)
}
fn world(
    index: usize,
    things: &[[i16; 5]],
    player: [i16; 2],
) -> (Simulation, ActorId, Vec<ActorId>) {
    world_in(&lumps(index, things), player, 6)
}
fn life(s: &Simulation, id: ActorId) -> ActorLife {
    s.actor(id).unwrap().life().unwrap()
}
fn state(s: &Simulation, id: ActorId) -> Option<OriginalState> {
    life(s, id)
        .state
        .map(|st| OriginalState::try_from(st.0).unwrap())
}
fn live(s: &Simulation, id: ActorId) -> bool {
    s.actors.is_pending(id) == Some(false)
}
fn attack() -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0].buttons = BT_ATTACK;
    c
}
fn idle() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
fn player_health(s: &Simulation) -> i32 {
    s.original_player.as_ref().unwrap().health
}
fn kills(s: &Simulation) -> i32 {
    s.statistics.unwrap().single_player_kill_count
}
/// `P_DamageMobj` between tics through the shared checked transaction, as the C
/// harness's declared `hit` input.
fn hit(
    s: &mut Simulation,
    target: ActorId,
    inflictor: Option<ActorId>,
    source: Option<ActorId>,
    amount: i32,
) -> Result<(), SimError> {
    let request = DamageRequest {
        target,
        inflictor,
        source,
        amount,
    };
    s.checked_demon_operation(|h| -> Result<(), SimError> {
        let plan = combat::DamagePlan::current_original(h, request)?;
        lifecycle::ActorAccess::reserve(&mut h.access, plan.spawn_count())?;
        h.apply_damage(plan);
        Ok(())
    })
    .and_then(|r| r)
}
/// Advances one tic, returning its events.
fn tic(s: &mut Simulation, command: [TicCommand; 4]) -> Vec<Event> {
    s.advance(command).unwrap();
    s.events().to_vec()
}
fn combat(events: &[Event]) -> Vec<CombatEvent> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Combat(c) => Some(c.clone()),
            _ => None,
        })
        .collect()
}
/// Every `Damage` record, in order.
fn damages(events: &[Event]) -> Vec<DamageRequest> {
    combat(events)
        .into_iter()
        .filter_map(|c| match c {
            CombatEvent::Damage(r) => Some(r),
            _ => None,
        })
        .collect()
}
fn entered(events: &[Event], id: ActorId) -> Vec<OriginalState> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::ActorState(ActorStateEvent::Entered { actor, state }) if *actor == id => {
                Some(OriginalState::try_from(state.0).unwrap())
            }
            _ => None,
        })
        .collect()
}
/// The explosion's sight records `(observer, target, visible)`, in order.
fn sights(events: &[Event]) -> Vec<(ActorId, ActorId, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Perception(PerceptionEvent::Sight {
                observer,
                target,
                result,
            }) => Some((*observer, *target, result.visible)),
            _ => None,
        })
        .collect()
}
/// The source's `P_Random` stream from `seed`, for expected draw values.
fn table(seed: u8) -> DoomRng {
    DoomRng::from_indices(seed, 0)
}

// ---- Spawn (M3h9b note §2.1) ------------------------------------------------------

/// `P_SpawnMapThing` (p_mobj.c:708–797) for two barrels, one with `MTF_AMBUSH`: the
/// `lastlook` draw (506) then `tics = 1 + P_Random() % 6` (787–788) each, in lump
/// order; the info record (health 20, radius 10, height 42, reaction time 8,
/// `MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD`, plus `MF_AMBUSH` from the record, 795–796) on
/// the floor; no kill or item total (789–792, not `MF_COUNTKILL`/`MF_COUNTITEM`).
/// Idle, a barrel cycles `S_BAR1`/`S_BAR2` every 6 tics and never draws.
#[test]
fn a_barrel_spawns_with_its_info_record_two_draws_and_no_totals() {
    let l = lumps(
        GALLERY,
        &[[-200, 0, 90, BARREL, 7], [-100, 0, 0, BARREL, 7 | 8]],
    );
    let mut s = Simulation::original_player_barrels_map(l.map(), options(6)).unwrap();
    let things = s.spawn_map_things().unwrap().spawned;
    assert_eq!(things.len(), 2);
    let mut rng = table(6);
    for (i, &b) in things.iter().enumerate() {
        let a = *s.actor(b).unwrap();
        let l = a.life().unwrap();
        let lastlook = rng.p_random() % 4;
        let tics = 1 + i32::from(rng.p_random()) % 6;
        assert_eq!(l.actor_type.0, MT_BARREL);
        assert_eq!(state(&s, b), Some(OriginalState::S_BAR1));
        assert_eq!((a.lastlook(), l.tics), (lastlook, tics), "barrel {i}");
        assert_eq!(l.health, 20);
        assert_eq!(l.reaction_time, 8);
        assert_eq!(l.flags, BARREL_FLAGS | if i == 1 { MF_AMBUSH } else { 0 });
        assert_eq!(a.radius(), Fixed::from_map_units(10));
        assert_eq!(a.height(), Fixed::from_map_units(42));
        assert_eq!(a.z, a.floor);
        assert_eq!(a.solidity, Solidity::Solid);
        assert!(!a.allows_dropoff());
        assert_eq!((l.target, l.threshold), (None, 0));
        assert_eq!(l.spawnpoint.unwrap().kind, BARREL);
    }
    assert_eq!(s.actor(things[0]).unwrap().angle(), 0x4000_0000);
    assert_eq!(s.rng(), rng);
    let stats = s.statistics.unwrap();
    assert_eq!((stats.total_kills, stats.total_items), (0, 0));
    assert_eq!(s.actor_order().collect::<Vec<_>>(), things);
    // Idle: the S_BAR1/S_BAR2 cycle with no draw (gallery: no light special).
    let (mut s, _, things) = world_in(&l, [-400, 200], 6);
    let rng = s.rng();
    let first = life(&s, things[0]).tics;
    let mut log = Vec::new();
    for _ in 0..40 {
        log.extend(tic(&mut s, idle()));
    }
    assert_eq!(s.rng(), rng);
    let entries = (0..)
        .map(|k| first + 6 * k)
        .take_while(|t| *t <= 40)
        .count();
    let cycle = entered(&log, things[0]);
    assert_eq!(cycle.len(), entries);
    for (k, st) in cycle.iter().enumerate() {
        let want = if k % 2 == 0 {
            OriginalState::S_BAR2
        } else {
            OriginalState::S_BAR1
        };
        assert_eq!(*st, want);
    }
}

/// Under wire 20 (and every older policy) 2035 stays `UnsupportedType`; under wire
/// 21 a 17th barrel is `MapSpawnError::BarrelLimit` with nothing spawned, the
/// canonical state and the RNG unchanged, after 16 accepted records.
#[test]
fn wire_20_refuses_barrels_and_wire_21_refuses_a_seventeenth() {
    let one = lumps(CENSUS, &[[0, 0, 0, BARREL, 7]]);
    let mut old = Simulation::original_player_e1m1_map(one.map(), options(6)).unwrap();
    let before = old.snapshot();
    assert_eq!(
        old.spawn_map_things().map(|_| ()),
        Err(SimError::MapSpawn(MapSpawnError::UnsupportedType(BARREL)))
    );
    assert_eq!(old.snapshot(), before);
    let grid = |n: usize| -> Vec<[i16; 5]> {
        (0..n)
            .map(|i| {
                [
                    -449 + 64 * (i % 8) as i16,
                    -321 + 64 * (i / 8) as i16,
                    0,
                    BARREL,
                    7,
                ]
            })
            .collect()
    };
    let mut s = Simulation::original_player_barrels_map(lumps(CENSUS, &grid(16)).map(), options(6))
        .unwrap();
    assert_eq!(s.spawn_map_things().unwrap().spawned.len(), MAX_BARRELS);
    let mut s = Simulation::original_player_barrels_map(lumps(CENSUS, &grid(17)).map(), options(6))
        .unwrap();
    let (before, rng) = (s.snapshot(), s.rng());
    assert_eq!(
        s.spawn_map_things().map(|_| ()),
        Err(SimError::MapSpawn(MapSpawnError::BarrelLimit))
    );
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.rng(), rng);
    assert_eq!(s.actor_order().count(), 0);
    // Barrels count against neither the monster bound nor the owner rewrite: eight
    // Zombiemen and sixteen barrels spawn together, each barrel keeping its type.
    let mut both = grid(16);
    both.extend((0..8).map(|i| [-449 + 64 * i, 255, 0, ZOMBIE, 7]));
    let mut s =
        Simulation::original_player_barrels_map(lumps(CENSUS, &both).map(), options(6)).unwrap();
    let spawned = s.spawn_map_things().unwrap().spawned;
    assert_eq!(spawned.len(), 24);
    assert!(
        spawned[..16]
            .iter()
            .all(|b| life(&s, *b).actor_type.0 == MT_BARREL)
    );
    assert_eq!(s.statistics.unwrap().total_kills, 8);
}

// ---- Damage into a barrel (§2.2, §2.3) --------------------------------------------

/// `barrel_wounded`: the pistol's first shot (`P_GunShot`, accurate) hits the live
/// barrel: `MF_NOBLOOD` makes it a puff (p_map.c:1005–1008), `P_DamageMobj` thrusts
/// it by `damage * 8192` (p_inter.c:817, mass 100) away from the player, the pain
/// draw happens with painchance 0 and enters no state (894–900), `reactiontime = 0`
/// (902), and the retarget block sets `target = player`, threshold 100 and enters
/// nothing (seestate `S_NULL`, 904–915).
#[test]
fn a_pistol_wound_puffs_draws_pain_retargets_and_thrusts() {
    let (mut s, body, things) = world(CENSUS, &[[-200, 0, 0, BARREL, 7]], [-400, 0]);
    let b = things[0];
    let mut log = Vec::new();
    for _ in 0..30 {
        let events = tic(&mut s, attack());
        if !damages(&events).is_empty() {
            log = events;
            break;
        }
    }
    let c = combat(&log);
    let at = c
        .iter()
        .position(|e| matches!(e, CombatEvent::Damage(_)))
        .unwrap();
    let CombatEvent::Damage(request) = c[at] else {
        unreachable!()
    };
    assert_eq!(
        (request.target, request.inflictor, request.source),
        (b, Some(body), Some(body))
    );
    assert!([5, 10, 15].contains(&request.amount));
    // The impact before the damage is a puff.
    let effect = c[..at]
        .iter()
        .find_map(|e| match e {
            CombatEvent::Impact { actor } => Some(*actor),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        life(&s, effect).actor_type.0,
        OriginalActorType::MT_PUFF.ordinal()
    );
    let amount = request.amount;
    assert!(matches!(
        c[at + 1],
        CombatEvent::Knockback { target, angle: 0, thrust, .. }
            if target == b && thrust == Fixed::from_bits(amount * THRUST_PER_DAMAGE)
    ));
    assert_eq!(
        c[at + 2],
        CombatEvent::Health {
            target: b,
            values: [20, 20 - amount]
        }
    );
    assert!(matches!(c[at + 3], CombatEvent::Random { kind: 1, .. }));
    assert_eq!(c[at + 4], CombatEvent::Reaction(b));
    assert_eq!(
        c[at + 5],
        CombatEvent::Retarget {
            target: b,
            source: body
        }
    );
    assert!(!c.iter().any(|e| matches!(e, CombatEvent::Pain { .. })));
    let l = life(&s, b);
    assert_eq!(
        (l.target, l.threshold, l.reaction_time),
        (Some(body), 100, 0)
    );
    assert!(matches!(
        state(&s, b),
        Some(OriginalState::S_BAR1 | OriginalState::S_BAR2)
    ));
    // Slid east this tic, then FRICTION (0xE800) on the next.
    let m = s.actor(b).unwrap().momentum()[0];
    assert!(m > Fixed::ZERO && m < Fixed::from_bits(amount * THRUST_PER_DAMAGE));
    tic(&mut s, idle());
    let next = s.actor(b).unwrap().momentum()[0];
    assert_eq!(next, m * Fixed::from_bits(0xE800));
}

/// `barrel_killed`: a lethal `P_DamageMobj` with a NULL inflictor runs
/// `P_KillMobj` (p_inter.c:668–758): `MF_SHOOTABLE` cleared, `MF_CORPSE|MF_DROPOFF`
/// set, `MF_SOLID` kept, `height >>= 2` (42 → 10.5), `S_BEXP` with
/// `tics -= P_Random() & 3` (726–729), no kill credit (not `MF_COUNTKILL`), no
/// retarget for the killing blow. The chain: `S_BEXP2` (`A_Scream`, `sfx_barexp`,
/// the default branch with no draw, p_enemy.c:1555–1557) after `t0` tics, `S_BEXP4`
/// (`A_Explode`) after `t0 + 10`, `S_BEXP5` after `t0 + 20`, `S_NULL` (removal)
/// after `t0 + 30` (§2.5). While dying it is no autoaim target and lets bullets
/// through (p_map.c:867–868, 979–980): the player's shot hits the live barrel
/// behind it.
#[test]
fn a_killed_barrel_stays_solid_lets_bullets_through_and_explodes_after_its_timer() {
    let (mut s, body, things) = world(
        CENSUS,
        &[[-200, 0, 0, BARREL, 7], [150, 0, 0, BARREL, 7]],
        [-400, 0],
    );
    let (b, behind) = (things[0], things[1]);
    let kills_before = kills(&s);
    let mut rng = s.rng();
    hit(&mut s, b, None, Some(body), 25).unwrap();
    let t0 = (5 - i32::from(rng.p_random() & 3)).max(1);
    assert_eq!(s.rng(), rng);
    let a = *s.actor(b).unwrap();
    let l = a.life().unwrap();
    assert_eq!(state(&s, b), Some(OriginalState::S_BEXP));
    assert_eq!((l.health, l.tics, l.flags), (-5, t0, DEAD_FLAGS));
    assert_eq!(a.height(), Fixed::from_bits((42 << 16) >> 2));
    assert_eq!(a.solidity, Solidity::Solid);
    assert!(a.allows_dropoff());
    assert_eq!((l.target, l.threshold), (None, 0));
    assert_eq!(kills(&s), kills_before);
    let barexp = lifecycle::source_sound_id("sfx_barexp");
    let mut first_shot = None;
    let mut chain = Vec::new();
    for n in 1..=t0 + 31 {
        let events = tic(&mut s, if n <= 25 { attack() } else { idle() });
        if first_shot.is_none() {
            first_shot = damages(&events)
                .into_iter()
                .find(|d| d.inflictor == Some(body));
        }
        // The kill's own S_BEXP entry is a between-tic event, carried by tic 1.
        for st in entered(&events, b)
            .into_iter()
            .filter(|st| *st != OriginalState::S_BEXP)
        {
            chain.push((n, st));
            if st == OriginalState::S_BEXP2 {
                assert!(events.iter().any(|e| matches!(
                    e,
                    Event::ActorState(ActorStateEvent::Sound { actor: Some(a), sound })
                        if *a == b && *sound == barexp
                )));
                assert!(!events.iter().any(|e| matches!(
                    e,
                    Event::ActorState(ActorStateEvent::Random { actor, .. }) if *actor == b
                )));
            }
        }
        if n < t0 + 30 {
            assert!(live(&s, b), "tic {n}");
            assert_eq!(s.actor(b).unwrap().solidity, Solidity::Solid);
        }
    }
    assert_eq!(
        chain,
        vec![
            (t0, OriginalState::S_BEXP2),
            (t0 + 5, OriginalState::S_BEXP3),
            (t0 + 10, OriginalState::S_BEXP4),
            (t0 + 20, OriginalState::S_BEXP5),
            (t0 + 30, OriginalState::S_NULL),
        ]
    );
    assert!(!live(&s, b));
    // The player's shot passed the dying barrel and hit the live one behind it.
    let shot = first_shot.unwrap();
    assert_eq!((shot.target, shot.source), (behind, Some(body)));
}

/// A player hitscan kill in tic `T` (the player thinks before every mobj, p_tick.c:
/// 130–158, so `d = 0`): `A_Explode` runs in tic `T + t0 + 9` (§2.5).
#[test]
fn a_player_hitscan_kill_explodes_nine_tics_after_the_timer() {
    let (mut s, _body, things) = world(CENSUS, &[[-200, 0, 0, BARREL, 7]], [-400, 0]);
    let b = things[0];
    // A wound to 5 health with a NULL source: any pistol bullet (5, 10 or 15) kills.
    hit(&mut s, b, None, None, 15).unwrap();
    let mut kill = None;
    for n in 1..=60 {
        let events = tic(&mut s, if kill.is_none() { attack() } else { idle() });
        for c in combat(&events) {
            if let CombatEvent::DeathTimer { target, tics } = c {
                if target == b {
                    kill = Some((n, tics));
                }
            }
        }
        if let Some((t, t0)) = kill {
            if entered(&events, b).contains(&OriginalState::S_BEXP4) {
                assert_eq!(n, t + t0 + 9);
                return;
            }
        }
    }
    panic!("no explosion: {kill:?}");
}

// ---- The explosion (§2.2 P_RadiusAttack / PIT_RadiusAttack) ------------------------

/// p_map.c:1221 with the unparenthesized `MAXRADIUS`: the wrapped shift leaves
/// `128 << 16` (decision F5), the 3 × 3 window; the per-explosion sight bound is
/// `1 + K + B − 1` = 24 distinct candidates, each queried at most once per walk
/// pass, and at most `1 + K` passes (each of the `K` monsters relinks at most once
/// per explosion): `24 × (1 + K)` = 216 queries (decision F6).
#[test]
fn the_blast_window_and_the_explosion_sight_bound_are_the_source_values() {
    assert_eq!(BOMB_DAMAGE, 128);
    let wrapped = ((128i64 + 32 * 65536) << 16) as i32;
    assert_eq!(BLAST_WINDOW, wrapped);
    assert_eq!(BLAST_WINDOW, 128 << 16);
    assert_eq!(
        EXPLOSION_SIGHT_CANDIDATES,
        1 + MAX_PLAYER_MONSTERS + MAX_BARRELS - 1
    );
    assert_eq!(EXPLOSION_SIGHT_CANDIDATES, 24);
    assert_eq!(MAX_PLAYER_MONSTERS, 8);
    assert_eq!(
        EXPLOSION_SIGHT_QUERIES,
        EXPLOSION_SIGHT_CANDIDATES * (1 + MAX_PLAYER_MONSTERS)
    );
    assert_eq!(EXPLOSION_SIGHT_QUERIES, 216);
    // Every spot sees exactly three cells per axis.
    for v in -1000i32..1000 {
        let fixed = v << 16;
        assert_eq!(
            ((fixed + BLAST_WINDOW) >> 23) - ((fixed - BLAST_WINDOW) >> 23),
            2,
            "{v}"
        );
    }
}

/// Kills the barrel `b` between tics (20 damage, no inflictor) with `source`, and
/// runs until its `A_Explode`; returns that tic's events.
fn explode(s: &mut Simulation, b: ActorId, source: Option<ActorId>) -> Vec<Event> {
    hit(s, b, None, source, 20).unwrap();
    for _ in 0..20 {
        let events = tic(s, idle());
        if entered(&events, b).contains(&OriginalState::S_BEXP4) {
            return events;
        }
    }
    panic!("no explosion");
}

/// `player_hurt_by_barrel`: the player at several distances west of a barrel at
/// the origin (cell column 4, window columns 3..5, `x ∈ [−128, 256)`): damage
/// `128 − (|dx| − 16)` (p_map.c:1179–1194) with the thrust `damage * 8192` away from
/// the barrel, `attacker` NULL (p_inter.c:874), the pain draw; at 150 units
/// (`dist` 134) nothing; at 129 units the player stands in column 2, a fourth
/// cell, and is never visited although `dist` 113 < 128 (the window quirk, F5);
/// at 127 units (column 3) the damage is 17.
#[test]
fn the_player_takes_128_minus_dist_inside_the_window_only() {
    for (d, expected) in [
        (60, Some(84)),
        (80, Some(64)),
        (127, Some(17)),
        (129, None),
        (150, None),
    ] {
        let (mut s, body, things) = world(CENSUS, &[[0, 0, 0, BARREL, 7]], [-d, 0]);
        let events = explode(&mut s, things[0], None);
        let hits = damages(&events);
        match expected {
            Some(amount) => {
                assert_eq!(
                    hits,
                    vec![DamageRequest {
                        target: body,
                        inflictor: Some(things[0]),
                        source: None,
                        amount
                    }],
                    "{d}"
                );
                assert_eq!(player_health(&s), 100 - amount, "{d}");
                // One sight record (the player looking at the barrel) before it.
                assert_eq!(sights(&events), vec![(body, things[0], true)], "{d}");
                let c = combat(&events);
                assert!(
                    c.iter().any(|e| matches!(e,
                    CombatEvent::Knockback { target, thrust, .. }
                        if *target == body
                            && *thrust == Fixed::from_bits(amount * THRUST_PER_DAMAGE))),
                    "{d}"
                );
                assert!(
                    c.iter()
                        .any(|e| matches!(e, CombatEvent::Random { kind: 1, .. }))
                );
                assert_eq!(s.original_player.as_ref().unwrap().attacker, None);
                // Pushed west, away from the barrel.
                assert!(s.actor(body).unwrap().momentum()[0] < Fixed::ZERO);
            }
            None => {
                assert!(hits.is_empty(), "{d}");
                // Outside the window or out of range: no sight query, no record.
                assert!(sights(&events).is_empty(), "{d}");
                assert_eq!(player_health(&s), 100, "{d}");
            }
        }
    }
}

/// Armor absorbs a third for green armor (p_inter.c:854–869), and the player who
/// wounded the barrel is its `target`, hence the bomb source: `attacker` is the
/// player (`player_hurt_by_own_barrel`, §2.7).
#[test]
fn armor_absorbs_and_the_wounding_player_is_the_attacker() {
    let (mut s, body, things) = world(CENSUS, &[[0, 0, 0, BARREL, 7]], [-80, 0]);
    s.original_player.as_mut().unwrap().armor = Some(PlayerArmorState {
        kind: ArmorType::Green,
        points: 100,
    });
    hit(&mut s, things[0], None, Some(body), 5).unwrap();
    assert_eq!(life(&s, things[0]).target, Some(body));
    let events = explode(&mut s, things[0], None);
    // dist = 80 − 16 = 64, damage 64, saved 64 / 3 = 21.
    assert_eq!(damages(&events)[0].source, Some(body));
    assert_eq!(player_health(&s), 100 - (64 - 21));
    let p = s.original_player.as_ref().unwrap();
    assert_eq!(p.armor.unwrap().points, 100 - 21);
    assert_eq!(p.attacker, Some(body));
}

/// `player_killed_by_barrel`: a lethal explosion kills the player (`frags[self]++`
/// for a NULL source, p_inter.c:702–703; `PST_DEAD`); a dead player's use requests
/// a reborn and the next tic is refused with the whole state kept (M3h8).
#[test]
fn a_lethal_explosion_kills_the_player_and_the_reborn_tic_is_refused() {
    let (mut s, body, things) = world(CENSUS, &[[0, 0, 0, BARREL, 7]], [-40, 0]);
    s.original_player.as_mut().unwrap().health = 50;
    s.actors
        .get_mut(body)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .health = 50;
    let events = explode(&mut s, things[0], None);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::PlayerBody(PlayerBodyEvent::Died { frags: 1, .. })))
    );
    assert_eq!(player_health(&s), 0);
    assert_eq!(
        s.original_player.as_ref().unwrap().state,
        PlayerBodyState::Dead
    );
    let mut use_ = idle();
    use_[0].buttons = BT_USE;
    for _ in 0..40 {
        if s.original_player.as_ref().unwrap().reborn {
            break;
        }
        s.advance(use_).unwrap();
    }
    assert!(s.original_player.as_ref().unwrap().reborn);
    let before = s.snapshot();
    assert_eq!(s.advance(idle()), Err(SimError::PlayerReborn));
    assert_eq!(s.snapshot(), before);
}

/// `barrel_kills_monsters`: a Zombieman killed by an explosion with a NULL bomb
/// source credits `players[0]` (p_inter.c:692–697) and drops its clip (737–757);
/// with the player as the bomb source (`player_credit`) the player is credited
/// (683–691, the same single-player counter). An Imp hurt but alive by an
/// explosion whose source is a Zombieman retargets to that Zombieman with
/// threshold 100 (`barrel_infight`, 904–915).
#[test]
fn explosion_victims_are_credited_dropped_and_retargeted_as_the_source() {
    for wounder in [false, true] {
        let (mut s, body, things) = world(
            CENSUS,
            &[[0, 0, 0, BARREL, 7], [40, 0, 0, ZOMBIE, 7]],
            [-300, 0],
        );
        if wounder {
            hit(&mut s, things[0], None, Some(body), 5).unwrap();
        }
        let before = kills(&s);
        let events = explode(&mut s, things[0], None);
        let d = damages(&events);
        assert_eq!(d.len(), 1);
        // dist = 40 − 20 = 20: damage 108 kills the 20-health Zombieman.
        assert_eq!((d[0].target, d[0].amount), (things[1], 108));
        assert_eq!(d[0].source, wounder.then_some(body));
        assert_eq!(kills(&s), before + 1);
        let clips = s
            .actor_order()
            .filter(|id| life(&s, *id).actor_type.0 == OriginalActorType::MT_CLIP.ordinal())
            .count();
        assert_eq!(clips, 1);
    }
    // Infighting: a Zombieman's wound makes it the bomb source; the Imp 118 units
    // away (dist 98) takes 128 − 98 = 30 and survives (60 health).
    let (mut s, _body, things) = world(
        CENSUS,
        &[
            [0, 0, 0, BARREL, 7],
            [-300, 200, 0, ZOMBIE, 7],
            [118, 0, 0, IMP, 7],
        ],
        [-300, -300],
    );
    let (barrel, zombie, imp) = (things[0], things[1], things[2]);
    // No inflictor: no thrust (p_inter.c:806), so the barrel stays at the origin.
    hit(&mut s, barrel, None, Some(zombie), 5).unwrap();
    assert_eq!(life(&s, barrel).target, Some(zombie));
    let before = kills(&s);
    let events = explode(&mut s, barrel, None);
    assert_eq!(
        damages(&events),
        vec![DamageRequest {
            target: imp,
            inflictor: Some(barrel),
            source: Some(zombie),
            amount: 30
        }]
    );
    assert_eq!(life(&s, imp).health, 30);
    assert_eq!(
        (life(&s, imp).target, life(&s, imp).threshold),
        (Some(zombie), 100)
    );
    assert_eq!(kills(&s), before);
}

/// `chain_reaction`: the barrel at the origin explodes; the barrel 100 units east
/// (window column 5) takes `128 − 90` = 38 and dies with its own timer, and as it
/// follows the first in thinker order (`d = 0`) its `A_Explode` runs `t0 + 9` tics
/// after the first explosion; the barrel 137 units west sits in column 2, a fourth
/// cell: `dist` 127 < 128, yet it is never visited and survives untouched (F5).
#[test]
fn a_chain_reaction_follows_the_timers_and_the_window_quirk_spares_a_fourth_cell() {
    let (mut s, _body, things) = world(
        CENSUS,
        &[
            [0, 0, 0, BARREL, 7],
            [100, 0, 0, BARREL, 7],
            [-137, 0, 0, BARREL, 7],
        ],
        [-400, 300],
    );
    let (first, east, west) = (things[0], things[1], things[2]);
    let events = explode(&mut s, first, None);
    let d = damages(&events);
    assert_eq!(d.len(), 1);
    assert_eq!((d[0].target, d[0].amount, d[0].source), (east, 38, None));
    let t0 = combat(&events)
        .into_iter()
        .find_map(|c| match c {
            CombatEvent::DeathTimer { target, tics } if target == east => Some(tics),
            _ => None,
        })
        .unwrap();
    assert_eq!(state(&s, east), Some(OriginalState::S_BEXP));
    for n in 1..=t0 + 9 {
        let events = tic(&mut s, idle());
        let exploded = entered(&events, east).contains(&OriginalState::S_BEXP4);
        assert_eq!(exploded, n == t0 + 9, "tic {n}");
        if exploded {
            // The second window (columns 4..6) reaches neither survivor.
            assert!(damages(&events).is_empty());
        }
    }
    let w = life(&s, west);
    assert_eq!((w.health, w.target), (20, None));
    assert!(matches!(
        state(&s, west),
        Some(OriginalState::S_BAR1 | OriginalState::S_BAR2)
    ));
}

/// `radius_blocked_by_step`: `P_CheckSight (thing, bombspot)` reads the dead
/// barrel's height after `height >>= 2`: its window is the bottom 10.5 units
/// (p_sight.c:334–336, p_inter.c:681). The player on the gallery's north ledge
/// raised to `h` above a barrel 11 units in front of the ledge edge, 120 units
/// away (`dist` 104, damage 24): the line from the eye (`h + 42`) to the barrel's
/// top clears the edge iff `h < (10.5·120 + 31.5·11) / 109 ≈ 14.7`, so a 12-unit
/// step lets the blast through and a 16-unit step blocks it, where the live
/// barrel's 42 units would clear both. Either way the query is recorded with the
/// existing sight record (observer the player, target the barrel).
#[test]
fn the_dead_barrels_low_sight_window_is_blocked_by_a_step() {
    for (h, damaged) in [(12, true), (16, false)] {
        let mut l = lumps(GALLERY, &[[0, 245, 0, BARREL, 7]]);
        let ceiling = l.map().sectors()[1].flats[1];
        let mut name = [0u8; 8];
        name.copy_from_slice(&ceiling.as_bytes()[..8]);
        l.set_sector(1, [h, 128], 0, &name);
        let (mut s, body, things) = world_in(&l, [0, 365], 6);
        let events = explode(&mut s, things[0], None);
        let d = damages(&events);
        assert_eq!(sights(&events), vec![(body, things[0], damaged)], "{h}");
        if damaged {
            assert_eq!(
                d,
                vec![DamageRequest {
                    target: body,
                    inflictor: Some(things[0]),
                    source: None,
                    amount: 24
                }],
                "{h}"
            );
        } else {
            assert!(d.is_empty(), "{h}");
            assert_eq!(player_health(&s), 100);
        }
    }
}

// ---- Missiles (§2.2) ---------------------------------------------------------------

/// Runs `s` until the Imp's fireball exists, calls `between` once, then until the
/// fireball explodes; returns the damage records with an inflictor and the
/// fireball's last position.
fn fireball(
    s: &mut Simulation,
    imp: ActorId,
    target: ActorId,
    between: impl FnOnce(&mut Simulation),
) -> (Vec<DamageRequest>, [Fixed; 2]) {
    s.set_actor_target(imp, Some(target)).unwrap();
    s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
        .unwrap();
    let shot = OriginalActorType::MT_TROOPSHOT.ordinal();
    let missile = loop {
        if let Some(id) = s.actor_order().find(|id| life(s, *id).actor_type.0 == shot) {
            break id;
        }
        tic(s, idle());
    };
    between(s);
    let mut hits = Vec::new();
    for _ in 0..60 {
        let events = tic(s, idle());
        // A between-tic wound's own record (no inflictor) is carried by tic 1.
        hits.extend(
            damages(&events)
                .into_iter()
                .filter(|d| d.inflictor.is_some()),
        );
        if events.iter().any(
            |e| matches!(e, Event::Projectile(ProjectileEvent::Exploded(id)) if *id == missile),
        ) {
            return (hits, s.actor(missile).unwrap().position());
        }
    }
    panic!("the fireball never exploded: {hits:?}");
}

/// `fireball_kills_barrel`: a shootable barrel in the fireball's path is damaged
/// with the missile's draw and the Imp as source (p_map.c:323–324); the killing
/// blow does not retarget, so its bomb source stays NULL.
#[test]
fn a_fireball_kills_a_live_barrel_with_the_imp_as_source() {
    let (mut s, body, things) = world(
        CENSUS,
        &[[250, 0, 0, IMP, 7], [50, 0, 0, BARREL, 7]],
        [-180, 0],
    );
    let (imp, b) = (things[0], things[1]);
    // Pre-wound to 2 health with a NULL source: any fireball (3..24) kills.
    hit(&mut s, b, None, None, 18).unwrap();
    let (hits, _) = fireball(&mut s, imp, body, |_| {});
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].target, hits[0].source), (b, Some(imp)));
    assert_eq!(
        life(&s, hits[0].inflictor.unwrap()).actor_type.0,
        OriginalActorType::MT_TROOPSHOT.ordinal()
    );
    assert!((3..=24).contains(&hits[0].amount) && hits[0].amount % 3 == 0);
    assert_eq!(state(&s, b), Some(OriginalState::S_BEXP));
    assert_eq!(life(&s, b).target, None);
    assert_eq!(player_health(&s), 100);
}

/// A dying barrel is 10.5 units tall (p_inter.c:681) and a fireball flies 32 units
/// above the line between the Imp's and its target's feet (`P_SpawnMissile`,
/// p_mobj.c:889–930: `z + 32`, `momz` 923): on the flat floor it passes over a
/// dying barrel (the `PIT_CheckThing` height test, p_map.c:294–297) and reaches
/// the player.
/// `fireball_on_dying_barrel`: with the barrel on the gallery's pedestal lowered to
/// 24, the fireball's 32..40 meets the dying barrel's 24..34.5, and a solid,
/// non-shootable thing makes it explode without damage (316–320).
#[test]
fn a_fireball_passes_over_a_dying_barrel_or_explodes_on_it_without_damage() {
    let (mut s, body, things) = world(
        CENSUS,
        &[[250, 0, 0, IMP, 7], [50, 0, 0, BARREL, 7]],
        [-180, 0],
    );
    let (imp, b) = (things[0], things[1]);
    // Killed as the fireball leaves: it arrives after A_Explode (nobody within 128
    // units) and before the removal `t0 + 30` tics after the kill.
    let (hits, _) = fireball(&mut s, imp, body, |s| hit(s, b, None, None, 20).unwrap());
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!((hits[0].target, hits[0].source), (body, Some(imp)));
    let mut l = lumps(GALLERY, &[[200, 0, 180, IMP, 7], [0, 0, 0, BARREL, 7]]);
    let ceiling = l.map().sectors()[2].flats[1];
    let mut name = [0u8; 8];
    name.copy_from_slice(&ceiling.as_bytes()[..8]);
    l.set_sector(2, [24, 128], 0, &name);
    let (mut s, body, things) = world_in(&l, [-200, 0], 6);
    let (imp, b) = (things[0], things[1]);
    assert_eq!(s.actor(b).unwrap().z, Fixed::from_map_units(24));
    let (hits, at) = fireball(&mut s, imp, body, |s| hit(s, b, None, None, 20).unwrap());
    assert!(live(&s, b));
    // Stopped at the dying barrel (box from x = 10, fireball radius 6).
    assert!(at[0] > Fixed::from_map_units(10), "{at:?}");
    assert!(at[0] < Fixed::from_map_units(40), "{at:?}");
    assert!(hits.is_empty(), "{hits:?}");
    assert_eq!(player_health(&s), 100);
}

// ---- Movement and movers (§2.6) ----------------------------------------------------

/// A live barrel pushed into a wall stops dead (p_mobj.c:188–189); pushed across
/// the liftroom's WR 88 line it triggers the lift like any non-player crosser
/// (p_spec.c:492–534).
#[test]
fn a_pushed_barrel_stops_at_walls_and_crosses_a_lift_line() {
    let (mut s, _body, things) = world(CENSUS, &[[-490, 0, 0, BARREL, 7]], [0, 0]);
    let b = things[0];
    s.set_momentum(b, [Fixed::from_map_units(-8), Fixed::ZERO])
        .unwrap();
    for _ in 0..4 {
        tic(&mut s, idle());
    }
    assert_eq!(s.actor(b).unwrap().momentum(), [Fixed::ZERO; 2]);
    assert!(s.actor(b).unwrap().position()[0] >= Fixed::from_map_units(-502));
    let (mut s, _body, things) = world(LIFTROOM, &[[101, 111, 0, BARREL, 7]], [401, 201]);
    let b = things[0];
    s.set_momentum(b, [Fixed::ZERO, Fixed::from_map_units(8)])
        .unwrap();
    let mut crossed = false;
    for _ in 0..10 {
        let events = tic(&mut s, idle());
        crossed |= events
            .iter()
            .any(|e| matches!(e, Event::Mover(MoverEvent::Crossed { actor, .. }) if *actor == b));
    }
    assert!(crossed);
    assert!(s.actor(b).unwrap().position()[1] > Fixed::from_map_units(128));
}

/// `door_gibs_dying_barrel`: a dying barrel slid under a closing DR door no longer
/// fits once the opening is below 10.5 units: `PIT_ChangeSector` enters `S_GIBS`,
/// clears `MF_SOLID` and zeroes height and radius (p_map.c:1269–1279), so it never
/// explodes and is never removed.
#[test]
fn a_dying_barrel_crunched_by_a_closing_door_never_explodes() {
    let (mut s, _body, things) = world(LIFTROOM, &[[501, 65, 0, BARREL, 7]], [471, 65]);
    let b = things[0];
    let door = 5;
    let mut use_ = idle();
    use_[0].buttons = BT_USE;
    // First use: the door is already open (a wait); the second closes it.
    for n in 1..=200 {
        let command = if n == 3 || n == 6 { use_ } else { idle() };
        tic(&mut s, command);
        if s.sectors[door].ceiling <= Fixed::from_map_units(22) {
            break;
        }
    }
    assert!(s.sectors[door].ceiling <= Fixed::from_map_units(22));
    hit(&mut s, b, None, None, 20).unwrap();
    s.set_momentum(b, [Fixed::from_map_units(4), Fixed::ZERO])
        .unwrap();
    let mut log = Vec::new();
    for _ in 0..40 {
        log.extend(tic(&mut s, idle()));
    }
    let chain = entered(&log, b);
    assert!(chain.contains(&OriginalState::S_GIBS), "{chain:?}");
    assert!(!chain.contains(&OriginalState::S_BEXP4), "{chain:?}");
    assert!(live(&s, b));
    let a = *s.actor(b).unwrap();
    assert_eq!(state(&s, b), Some(OriginalState::S_GIBS));
    assert_eq!((a.height(), a.radius()), (Fixed::ZERO, Fixed::ZERO));
    assert_eq!(a.solidity, Solidity::NonSolid);
    assert_eq!(life(&s, b).flags & MF_SOLID, 0);
    assert!(!damages(&log).iter().any(|d| d.inflictor == Some(b)));
}

// ---- Witnesses and refusals (F4) ---------------------------------------------------

/// Decision F4(iv): a bomb source that no longer resolves (a Zombieman that
/// wounded the barrel, then removed and retired) refuses the explosion tic with
/// the canonical state preserved instead of passing NULL.
#[test]
fn a_stale_bomb_source_refuses_the_explosion_tic() {
    let (mut s, _body, things) = world(
        CENSUS,
        &[[0, 0, 0, BARREL, 7], [-300, 200, 0, ZOMBIE, 7]],
        [-300, -300],
    );
    let (barrel, zombie) = (things[0], things[1]);
    hit(&mut s, barrel, Some(zombie), Some(zombie), 5).unwrap();
    s.mark_remove(zombie).unwrap();
    tic(&mut s, idle());
    assert!(s.actor(zombie).is_none());
    hit(&mut s, barrel, None, None, 20).unwrap();
    for _ in 0..20 {
        let before = s.snapshot();
        let rng = s.rng();
        if let Err(error) = s.advance(idle()) {
            assert_eq!(error, SimError::PlayerBody(PlayerBodyError::Invariant));
            assert_eq!(s.snapshot(), before);
            assert_eq!(s.rng(), rng);
            assert_eq!(state(&s, barrel), Some(OriginalState::S_BEXP3));
            assert_eq!(life(&s, barrel).tics, 1);
            return;
        }
    }
    panic!("the explosion tic was not refused");
}

/// Decision F4(v): a target, a removal, a state entry, a death request and a
/// direct spawn are refused for a barrel with the state unchanged; a push and
/// vertical momentum stay admitted within the shared bounds, and `damage_actor`
/// with an inflictor (a Zombieman: the bound player body is never an API
/// inflictor) wounds it and thrusts it away.
#[test]
fn barrel_api_refusals_and_admissions() {
    let (mut s, body, things) = world(
        CENSUS,
        &[[0, 0, 0, BARREL, 7], [-200, 200, 0, ZOMBIE, 7]],
        [-300, 0],
    );
    let (b, zombie) = (things[0], things[1]);
    let before = s.snapshot();
    let unsupported = SimError::PlayerBody(PlayerBodyError::Unsupported);
    assert_eq!(s.set_actor_target(b, Some(body)), Err(unsupported));
    assert_eq!(s.mark_remove(b), Err(unsupported));
    assert_eq!(
        s.enter_actor_state(b, StateId(OriginalState::S_BEXP.ordinal())),
        Err(unsupported)
    );
    assert!(s.enter_original_state(b, OriginalState::S_BEXP4).is_err());
    assert!(s.request_actor_death(b).is_err());
    assert_eq!(
        s.spawn_type(
            ActorTypeId(MT_BARREL),
            [0, 100].map(Fixed::from_map_units),
            SpawnHeight::Floor
        )
        .map(|_| ()),
        Err(SimError::MapSpawn(MapSpawnError::DirectSpawn))
    );
    assert_eq!(s.snapshot(), before);
    let result = s
        .damage_actor(DamageRequest {
            target: b,
            inflictor: Some(zombie),
            source: Some(zombie),
            amount: 5,
        })
        .unwrap();
    assert_eq!(result.health, [20, 15]);
    assert_eq!(life(&s, b).target, Some(zombie));
    assert!(s.actor(b).unwrap().momentum()[0] > Fixed::ZERO);
    s.set_momentum(b, [Fixed::from_map_units(2), Fixed::ZERO])
        .unwrap();
    s.set_vertical_momentum(b, Fixed::ZERO).unwrap();
    tic(&mut s, idle());
}

/// A barrel is never a damage source in the source (`A_Explode` passes it only as
/// the inflictor), so `damage_actor` refuses one as the `source`, with or without an
/// inflictor and whatever the target, keeping every byte and the RNG; the same
/// barrel stays admitted as the inflictor. Found by the `player_barrels` fuzz seeds:
/// an admitted barrel source made a monster wounded by its explosion target the
/// barrel, and the next tic was refused with `PerceptionError::Roster`.
#[test]
fn a_barrel_is_refused_as_a_damage_source() {
    let (mut s, body, things) = world(
        CENSUS,
        &[
            [0, 0, 0, BARREL, 7],
            [64, 0, 0, BARREL, 7],
            [-200, 200, 0, ZOMBIE, 7],
        ],
        [-300, 0],
    );
    let (a, b, zombie) = (things[0], things[1], things[2]);
    let unsupported = Err(SimError::PlayerBody(PlayerBodyError::Unsupported));
    let before = s.snapshot();
    let rng = s.rng();
    for target in [b, zombie, body] {
        for inflictor in [None, Some(a), Some(zombie)] {
            let request = DamageRequest {
                target,
                inflictor,
                source: Some(a),
                amount: 5,
            };
            assert_eq!(s.damage_actor(request).map(|_| ()), unsupported);
            assert_eq!(s.snapshot(), before);
            assert_eq!(s.rng(), rng);
        }
    }
    // The barrel as the inflictor with a monster source, and as the target.
    s.damage_actor(DamageRequest {
        target: b,
        inflictor: Some(a),
        source: Some(zombie),
        amount: 5,
    })
    .unwrap();
    assert_eq!(life(&s, b).target, Some(zombie));
    tic(&mut s, idle());
}

/// Under wire 21 `A_Explode` is admitted for `MT_BARREL`'s `S_BEXP4` alone: the
/// rocket's `S_EXPLODE1` stays `UnsupportedAction`, and older policies never admit
/// the barrel's states.
#[test]
fn a_explode_is_admitted_for_the_barrel_state_alone() {
    let registry = ActorRegistry::vanilla();
    registry
        .validate_state_barrel(StateId(OriginalState::S_BAR1.ordinal()))
        .unwrap();
    registry
        .validate_state_barrel(StateId(OriginalState::S_BEXP.ordinal()))
        .unwrap();
    let rocket = StateId(OriginalState::S_EXPLODE1.ordinal());
    assert!(matches!(
        registry.validate_state_barrel(rocket),
        Err(ActorError::UnsupportedAction(_))
    ));
    assert!(matches!(
        registry.validate_state_original_monsters(StateId(OriginalState::S_BEXP.ordinal())),
        Err(ActorError::UnsupportedAction(_))
    ));
    let barrel = ActorTypeId(MT_BARREL);
    let new = OriginalExecutionProfile::PlayerMonstersBarrelsV1;
    let old = OriginalExecutionProfile::PlayerMonstersE1M1V1;
    for st in [
        OriginalState::S_BAR1,
        OriginalState::S_BAR2,
        OriginalState::S_BEXP,
        OriginalState::S_BEXP4,
        OriginalState::S_BEXP5,
    ] {
        let id = StateId(st.ordinal());
        assert!(original_registry::state_role_for(new, barrel, id).is_ok());
        assert!(original_registry::state_role_for(old, barrel, id).is_err());
    }
    assert!(
        original_registry::state_role_for(new, barrel, StateId(OriginalState::S_PLAY.ordinal()))
            .is_err()
    );
    assert!(!original_registry::is_policy_monster(new, barrel));
    assert!(crate::barrels::source_definition());
}

// ---- The barrel escape witness -------------------------------------------------

/// The M3h9b corridor-hall room (`m3h9b_cases.py` `corridor_hall()`, map 6 of
/// `m3h9b_cases.csv`, the room of case 9 and fuzz start 9): the
/// hall `x ∈ [0, 512]`, `y ∈ [−320, 320]` entered at its west wall by the corridor
/// `x ∈ [−512, 0]`, `y ∈ [−32, 32]`; the wall corner at (0, −32).
const CORRIDOR_HALL: usize = 6;

/// The corridor hall with one barrel at `at` and the player in the corridor.
fn hall_barrel(at: [i16; 2]) -> (Simulation, ActorId) {
    let mut l = crate::barrels_replay::room(CORRIDOR_HALL);
    l.clear_things();
    l.add_thing([at[0], at[1], 0, BARREL, 7]);
    let (s, _body, things) = world_in(&l, [-300, 0], 6);
    (s, things[0])
}

/// The momentum of the `player_barrels` fuzz seeds 1123/1184/1367/1428/1489/1550:
/// about (−27.19, −27.19) per axis.
const JUMP: Fixed = Fixed::from_bits(-(27 << 16) - 12_452);

/// Fuzz finding 3 (M3h9b): a barrel (radius 10) at (10, −30) with momentum
/// (−27.19, −27.19) is moved by `P_XYMovement` in one unhalved step (only positive
/// steps above 15 are halved, p_mobj.c:150–164) to (−17.19, −57.19), whose box
/// `x ∈ [−27.19, −7.19]`, `y ∈ [−67.19, −47.19]` touches neither the hall's west
/// wall (`x = 0`, `y ≤ −32`) nor the corridor's south wall (`y = −32`, `x ≤ 0`):
/// `P_TryMove` accepts it (p_map.c:451–517) behind the corner, outside the room.
/// The source move is legal; the barrels policy refuses the tic with
/// `MapScaleError::Escape` through `advance` and `step`, every byte and the RNG
/// kept.
#[test]
fn a_barrel_jumping_the_wall_corner_refuses_the_tic_with_escape() {
    for step in [false, true] {
        let (mut s, b) = hall_barrel([10, -30]);
        let boundary = s.boundary.clone().unwrap();
        let destination = [
            Fixed::from_map_units(10) + JUMP,
            Fixed::from_map_units(-30) + JUMP,
        ];
        assert!(boundary.inside(s.actor(b).unwrap().position()));
        assert!(!boundary.inside(destination));
        s.set_momentum(b, [JUMP; 2]).unwrap();
        let before = s.snapshot();
        let rng = s.rng();
        let result = if step {
            s.step(idle()).map(|_| ())
        } else {
            s.advance(idle())
        };
        assert_eq!(result, Err(SimError::MapScale(MapScaleError::Escape)));
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.rng(), rng);
    }
}

/// The control: the same barrel with momentum (−15, −15) would put its box across
/// the hall's west wall, so `P_TryMove` refuses the step and the barrel stops dead
/// in the room (p_mobj.c:188–189); the tic is admitted. A step below the box width
/// of 20 can never clear the corner.
#[test]
fn a_barrel_stopped_short_of_the_corner_is_admitted() {
    let (mut s, b) = hall_barrel([10, -30]);
    let fifteen = Fixed::from_map_units(-15);
    s.set_momentum(b, [fifteen; 2]).unwrap();
    s.advance(idle()).unwrap();
    let a = *s.actor(b).unwrap();
    assert_eq!(a.position(), [10, -30].map(Fixed::from_map_units));
    assert_eq!(a.momentum(), [Fixed::ZERO; 2]);
    assert!(s.boundary.as_ref().unwrap().inside(a.position()));
}

/// A barrel pushed at the same speed along the hall's west wall (no corner in
/// reach) and across the open hall moves every tic and stays admitted: the witness
/// refuses only a centre that left the room.
#[test]
fn a_fast_barrel_along_a_wall_or_in_the_open_is_admitted() {
    for (at, momentum) in [
        ([11, -100], [Fixed::ZERO, JUMP]),
        ([11, 100], [Fixed::ZERO, -JUMP]),
        ([200, 100], [JUMP, JUMP]),
    ] {
        let (mut s, b) = hall_barrel(at);
        s.set_momentum(b, momentum).unwrap();
        let start = s.actor(b).unwrap().position();
        for _ in 0..3 {
            s.advance(idle()).unwrap();
            assert!(
                s.boundary
                    .as_ref()
                    .unwrap()
                    .inside(s.actor(b).unwrap().position())
            );
        }
        assert_ne!(s.actor(b).unwrap().position(), start, "{at:?}");
    }
}

// ---- Soak --------------------------------------------------------------------------

/// The barrel witness every tic preflight reads (`validate_barrel`), plus the
/// soak's own checks: every barrel holds a role state or `S_GIBS`, and a live
/// barrel on the flat census floor stays on it.
fn barrel_invariants(s: &Simulation) {
    let gibs = StateId(OriginalState::S_GIBS.ordinal());
    for id in s.actor_order() {
        if !live(s, id) {
            continue;
        }
        let a = *s.actor(id).unwrap();
        let l = a.life().unwrap();
        if l.actor_type.0 != MT_BARREL {
            continue;
        }
        crate::barrels::validate_barrel(s.room(), a, s.gibbed(&a)).unwrap();
        assert!(
            l.state == Some(gibs)
                || crate::barrels::state_allowed(
                    OriginalState::try_from(l.state.unwrap().0).unwrap()
                )
        );
        if l.health > 0 {
            assert_eq!(a.z, a.floor);
        }
    }
}

/// 2,400 tics under four seeds in the census room with twelve barrels, two
/// Zombiemen and an Imp around the player, who walks, turns and fires: barrels are
/// wounded, killed and chain-explode; after every tic the tic preflight (V′ and
/// every witness, run by `advance`) passed and the barrel witness holds; every
/// 100th tic the DSIM37 capture decodes with `decode_barrels` and re-encodes to
/// the same bytes.
#[test]
fn barrels_soak_2400_tics_under_four_seeds() {
    let mut things: Vec<[i16; 5]> = (0..12)
        .map(|i| [-301 + 60 * (i % 6), -121 + 240 * (i / 6), 0, BARREL, 7])
        .collect();
    things.push([301, 201, 0, ZOMBIE, 7]);
    things.push([301, -199, 0, ZOMBIE, 7]);
    things.push([401, 1, 180, IMP, 7]);
    let mut exploded = 0;
    for seed in [1u8, 6, 77, 200] {
        let (mut s, _body, _) = world_in(&lumps(CENSUS, &things), [-449, 1], seed);
        for n in 0..2400u32 {
            let mut c = idle();
            if s.original_player.as_ref().unwrap().state != PlayerBodyState::Dead {
                c[0].forward = [0, 25, -25, 10][(n / 37 % 4) as usize];
                c[0].angle_turn = [0, 256, -256, 0, 0][(n / 23 % 5) as usize];
                c[0].buttons = if n % 9 < 4 { BT_ATTACK } else { 0 };
            }
            if let Err(error) = s.advance(c) {
                panic!("seed {seed} tic {n}: {error}");
            }
            let explode = ActorAction::Original(lifecycle::source_action_id("A_Explode"));
            exploded += s
                .events()
                .iter()
                .filter(|e| {
                    matches!(e, Event::ActorState(ActorStateEvent::Action { action, .. })
                        if *action == explode)
                })
                .count();
            barrel_invariants(&s);
            if n % 100 == 99 {
                round_trip(&s);
            }
        }
    }
    assert!(exploded > 0);
}

/// `monster_wakes_mid_walk` (M3h9b note §5.2): the player (spawned last, so the
/// newest link of cell (4, 3)) and an idle Imp share the blast's centre cell. The
/// walk visits the player (`dist` 91 − 16 = 75, damage 53), then the Imp (101 − 20
/// = 81, damage 47). With the bomb source a Zombieman (the barrel's first
/// wounder) and the Imp in its spawn state with threshold 0, the retarget enters
/// its seestate, whose `A_Chase` runs at once (p_inter.c:904–915); when the pain
/// draw fails and `A_Chase` walks, `P_TryMove` relinks the Imp at the head of its
/// cell (p_maputl.c:436–442), so its `bnext` is the player and the source walk
/// damages the player a second time. When the pain draw succeeds (seed 6) the Imp
/// enters its pain state, not its seestate, and the player is hit once.
#[test]
fn a_monster_relinked_mid_walk_makes_the_walk_revisit_its_cell() {
    let l = lumps(
        CENSUS,
        &[
            [0, 0, 0, BARREL, 7],
            [-401, 301, 180, ZOMBIE, 7],
            [101, 31, 0, IMP, 7],
        ],
    );
    for (seed, revisit) in [(10u8, true), (6, false)] {
        let (mut s, body, things) = world_in(&l, [41, 91], seed);
        s.original_player.as_mut().unwrap().health = 200;
        s.actors
            .get_mut(body)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .health = 200;
        let (barrel, zombie, imp) = (things[0], things[1], things[2]);
        hit(&mut s, barrel, None, Some(zombie), 5).unwrap();
        let events = explode(&mut s, barrel, None);
        let request = |target, amount| DamageRequest {
            target,
            inflictor: Some(barrel),
            source: Some(zombie),
            amount,
        };
        let mut expected = vec![request(body, 53), request(imp, 47)];
        if revisit {
            expected.push(request(body, 53));
        }
        assert_eq!(damages(&events), expected, "seed {seed}");
        let player_hits = if revisit { 2 } else { 1 };
        assert_eq!(player_health(&s), 200 - 53 * player_hits);
        let imp_moved = events.iter().any(|e| {
            matches!(e, Event::ActorMovement { actor, attempt }
                if *actor == imp && attempt.blocked_by_actor.is_none())
                || matches!(e, Event::SectorMovement { actor, .. } if *actor == imp)
        });
        let a_chase = ActorAction::Original(lifecycle::source_action_id("A_Chase"));
        let chase = events.iter().any(|e| {
            matches!(e, Event::ActorState(ActorStateEvent::Action { actor, action })
                if *actor == imp && *action == a_chase)
        });
        assert_eq!(chase && imp_moved, revisit, "seed {seed}");
        assert_eq!(
            (life(&s, imp).target, life(&s, imp).threshold),
            (Some(zombie), if revisit { 99 } else { 100 })
        );
    }
}

// ---- DSIM37 barrel validation (plan step 7) -----------------------------------------

/// The capture decodes with `decode_barrels` and re-encodes to the same bytes;
/// `decode_e1m1` rejects it by version.
fn round_trip(s: &Simulation) -> Vec<u8> {
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_barrels(&bytes, limits).unwrap();
    assert_eq!(view.encode(limits).unwrap(), bytes);
    assert_eq!(
        Diagnostic::decode_e1m1(&bytes, limits).map(|_| ()),
        Err(DiagnosticError::Version(37))
    );
    bytes
}
fn field(bytes: &[u8], path: &str) -> Vec<u8> {
    crate::player_e1m1_tests::dsim_fields(bytes)
        .into_iter()
        .find(|(p, _)| p == path)
        .unwrap()
        .1
}
/// `bytes` with `actor.{slot}.{name}` replaced by `value`.
fn forged(bytes: &[u8], slot: u32, name: &str, value: Vec<u8>) -> Vec<u8> {
    let path = format!("actor.{slot}.{name}");
    crate::player_e1m1_tests::forge(bytes, |p, _| (p == path).then(|| value.clone()))
}
fn rejects(bytes: &[u8], why: &str) {
    assert!(
        Diagnostic::decode_barrels(bytes, DiagnosticLimits::default()).is_err(),
        "{why}"
    );
}

/// Every barrel state the source reaches round-trips through DSIM37: live,
/// wounded (target, threshold 100, reaction time 0), dying (`S_BEXP` chain,
/// flags and height 10.5), pending removal after its `S_NULL`, and `S_GIBS` after a
/// door crunch.
#[test]
fn every_barrel_state_round_trips_through_dsim37() {
    let (mut s, body, things) = world(
        CENSUS,
        &[[0, 0, 0, BARREL, 7], [-200, 0, 0, BARREL, 7 | 8]],
        [-400, 200],
    );
    round_trip(&s);
    hit(&mut s, things[1], None, Some(body), 5).unwrap();
    round_trip(&s);
    hit(&mut s, things[0], None, None, 20).unwrap();
    round_trip(&s);
    let mut pending = false;
    for _ in 0..40 {
        tic(&mut s, idle());
        let bytes = round_trip(&s);
        if s.actors.is_pending(things[0]) == Some(true) && !pending {
            pending = true;
            // The removed record: no state, the dying flags and height kept.
            let slot = things[0].slot();
            let i32v = |v: i32| v.to_le_bytes().to_vec();
            rejects(
                &forged(&bytes, slot, "height", i32v(42 << 16)),
                "pending, live height",
            );
            rejects(&forged(&bytes, slot, "tics", i32v(1)), "pending, tics 1");
            rejects(
                &forged(&bytes, slot, "flags", BARREL_FLAGS.to_le_bytes().to_vec()),
                "pending, live flags",
            );
        }
    }
    assert!(pending);
    // The door crunch of the step 6 scenario.
    let (mut s, _body, things) = world(LIFTROOM, &[[501, 65, 0, BARREL, 7]], [471, 65]);
    let mut use_ = idle();
    use_[0].buttons = BT_USE;
    for n in 1..=200 {
        tic(&mut s, if n == 3 || n == 6 { use_ } else { idle() });
        if s.sectors[5].ceiling <= Fixed::from_map_units(22) {
            break;
        }
    }
    hit(&mut s, things[0], None, None, 20).unwrap();
    s.set_momentum(things[0], [Fixed::from_map_units(4), Fixed::ZERO])
        .unwrap();
    for _ in 0..40 {
        tic(&mut s, idle());
    }
    assert_eq!(state(&s, things[0]), Some(OriginalState::S_GIBS));
    round_trip(&s);
}

/// Each barrel check of `validate_barrels` refuses a capture mutated in that field
/// alone; the unmutated capture decodes. Every mutation below except the
/// non-barrel state and the `S_GIBS` relabel is refused by `validate_barrels`
/// alone (checked once with the call removed: the rest decode).
#[test]
fn dsim37_refuses_each_forged_barrel_field() {
    let (mut s, body, things) = world(
        CENSUS,
        &[[0, 0, 0, BARREL, 7], [-200, 0, 0, BARREL, 7]],
        [-400, 200],
    );
    hit(&mut s, things[1], None, Some(body), 5).unwrap();
    tic(&mut s, idle());
    let live = round_trip(&s);
    let slot = things[0].slot();
    let wounded = things[1].slot();
    let i32v = |v: i32| v.to_le_bytes().to_vec();
    let u16v = |v: u16| v.to_le_bytes().to_vec();
    let state_id = |st: OriginalState| u16v(st.ordinal());
    for (name, value, why) in [
        (
            "state",
            state_id(OriginalState::S_PLAY),
            "a non-barrel state",
        ),
        (
            "state",
            state_id(OriginalState::S_BEXP),
            "a death state with health 20",
        ),
        ("sprite", u16v(0), "the sprite"),
        ("frame", 7u32.to_le_bytes().to_vec(), "the frame"),
        ("tics", i32v(7), "tics above the state's 6"),
        ("tics", i32v(0), "tics 0"),
        ("health", i32v(21), "health above 20"),
        ("health", i32v(0), "health 0 while alive"),
        (
            "flags",
            (BARREL_FLAGS & !MF_SHOOTABLE).to_le_bytes().to_vec(),
            "flags",
        ),
        (
            "flags",
            (BARREL_FLAGS | MF_AMBUSH).to_le_bytes().to_vec(),
            "ambush without MTF_AMBUSH",
        ),
        ("radius", i32v(11 << 16), "the radius"),
        ("height", i32v(41 << 16), "the height"),
        ("reaction_time", i32v(7), "the reaction time"),
        ("threshold", i32v(50), "threshold 50"),
        ("threshold", i32v(100), "threshold 100 without a target"),
        ("movedir", vec![1], "movedir"),
        ("movecount", i32v(1), "movecount"),
        ("death_requested", vec![1], "a death request"),
        ("lastlook", vec![4], "lastlook"),
        ("angle", 0x2000_0000u32.to_le_bytes().to_vec(), "the angle"),
        ("pending_removal", vec![1], "pending with a live state"),
    ] {
        rejects(&forged(&live, slot, name, value), why);
    }
    // The wounded barrel: a target without threshold 100.
    assert_eq!(
        field(&live, &format!("actor.{wounded}.threshold")),
        i32v(100)
    );
    rejects(
        &forged(&live, wounded, "threshold", i32v(0)),
        "target with threshold 0",
    );
    let mut spawn = field(&live, &format!("actor.{slot}.spawnpoint"));
    spawn[7..9].copy_from_slice(&3004i16.to_le_bytes());
    rejects(
        &forged(&live, slot, "spawnpoint", spawn),
        "a non-2035 spawnpoint",
    );
    // Dying: the death flags, height 10.5 and a countdown in the state's tics.
    hit(&mut s, things[0], None, None, 20).unwrap();
    let dying = round_trip(&s);
    for (name, value, why) in [
        (
            "flags",
            BARREL_FLAGS.to_le_bytes().to_vec(),
            "live flags while dying",
        ),
        ("height", i32v(42 << 16), "live height while dying"),
        ("health", i32v(1), "positive health while dying"),
        (
            "state",
            state_id(OriginalState::S_GIBS),
            "S_GIBS with corpse dimensions",
        ),
    ] {
        rejects(&forged(&dying, slot, name, value), why);
    }
}

/// Under DSIM36 (wire 20) a barrel is no admitted identity at all; the per-event
/// allow-list and the gibs rule admit `MT_BARREL` under DSIM37 only.
#[test]
fn older_readers_never_admit_a_barrel() {
    let (s, _body, things) = world(CENSUS, &[[0, 0, 0, BARREL, 7]], [-400, 200]);
    let bytes = round_trip(&s);
    let limits = DiagnosticLimits::default();
    // Relabeled as DSIM36 the capture fails (version, profile and the barrel).
    let mut old = bytes.clone();
    old[4..8].copy_from_slice(&36u32.to_le_bytes());
    assert!(Diagnostic::decode_e1m1(&old, limits).is_err());
    let _ = things;
}
