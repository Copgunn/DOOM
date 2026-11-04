//! M3h9 rejections, rollback and soaks (plan step 9). Rust-only safety tests: what
//! stays out of scope under wire 20 (every unadmitted THING type, 65 scrollers, the
//! crushers, the fixed medium single-player options) is refused at load with the
//! state unchanged; a bit-16 decoration is filtered with no draw; the seven
//! decoration types stay `UnsupportedType` under wires 18 and 19; a spawn batch that
//! fails after decorations, monsters and pickups were accepted leaves every byte, the
//! RNG and the events as they were; and multi-thousand-tic soaks in the gallery and
//! liftroom keep `step == advance`, V′, the decorations' static state and the DSIM36
//! round trip. Expected facts come from the source reading of the M3h9 note (§2.1,
//! §2.5, §5.1, §5.3), never from Rust output.
use crate::e1m1_rules::{E1M1_DECORATIONS, E1M1_LINKED_THINGS, MAX_DECORATIONS};
use crate::identity::OriginalActorType;
use crate::player_e1m1_tests::{e1m1, room};
use crate::player_movers_tests::options;
use crate::v_prime_tests::{Commands, assert_v_prime};
use crate::*;

/// Everything a refused load or spawn batch must leave as it was: the canonical
/// capture bytes, both RNG streams, the actor order, the event log and pending
/// events, the level statistics and the tic.
#[derive(Debug, PartialEq)]
struct Witness {
    bytes: Vec<u8>,
    rng: doom_types::DoomRng,
    actors: Vec<ActorId>,
    events: Vec<Event>,
    pending: Vec<Event>,
    statistics: String,
    tic: u64,
}
fn witness(s: &Simulation) -> Witness {
    Witness {
        bytes: s
            .snapshot()
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap(),
        rng: s.rng,
        actors: s.actor_order().collect(),
        events: s.events().to_vec(),
        pending: s.pending_events().to_vec(),
        statistics: format!("{:?}", s.statistics),
        tic: s.tic(),
    }
}
fn rejected(result: Result<Simulation, OriginalRegistryError>) -> SimError {
    match result {
        Err(OriginalRegistryError::Runtime(error)) => error,
        Err(other) => panic!("unexpected error {other:?}"),
        Ok(_) => panic!("admitted"),
    }
}
/// Spawns the THINGs of `lumps` under wire 20 and asserts the batch is refused
/// with `expected` and the witness unchanged.
fn spawn_refused(lumps: &crate::diagonal_room::Lumps, expected: SimError) {
    let mut s = e1m1(lumps).unwrap();
    let before = witness(&s);
    assert_eq!(s.spawn_map_things().map(|_| ()), Err(expected));
    assert_eq!(witness(&s), before);
    assert_eq!(s.actor_order().count(), 0);
}

/// §5.1: every info.c type with a DoomEd number that wire 20 does not admit (the
/// barrel 2035, the hanging corpses, the candle 34, the other columns, trees and
/// lamps that satisfy the static predicate but are not in the list, keys, power-ups,
/// teleport destinations, every other monster and the Doom II types) is
/// `UnsupportedType` when it follows the ten gallery decorations in the lump, with
/// nothing spawned, no draw and the state unchanged; the same batch without it loads.
#[test]
fn every_unadmitted_doomed_type_is_refused_after_the_gallery_decorations() {
    let registry =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersE1M1V1).unwrap();
    let mut s = e1m1(&room(5)).unwrap();
    assert_eq!(s.spawn_map_things().unwrap().spawned.len(), 10);
    let (mut refused, mut static_but_unlisted) = (0, 0);
    for ty in OriginalActorType::all() {
        let d = ty.descriptor();
        if d.doomed_number <= 0 {
            continue;
        }
        let admitted = registry
            .admit_definition(DefinitionIdentity::Original(ty))
            .is_ok();
        assert_eq!(
            E1M1_DECORATIONS.contains(&ty),
            admitted && crate::e1m1_rules::static_decoration(ty),
            "{}",
            ty.name()
        );
        if admitted {
            continue;
        }
        let kind = i16::try_from(d.doomed_number).unwrap();
        let mut lumps = room(5);
        lumps.add_thing([300, 100, 0, kind, 7]);
        spawn_refused(
            &lumps,
            SimError::MapSpawn(MapSpawnError::UnsupportedType(kind)),
        );
        refused += 1;
        static_but_unlisted += usize::from(crate::e1m1_rules::static_decoration(ty));
    }
    // The barrel, the hanging corpse MT_MISC51 (49) and the candle (34) are among them.
    for kind in [2035, 49, 34] {
        assert!(
            OriginalActorType::all()
                .any(|t| t.descriptor().doomed_number == kind && !E1M1_DECORATIONS.contains(&t))
        );
    }
    assert!(refused > 80, "{refused}");
    assert!(static_but_unlisted > 0, "{static_but_unlisted}");
}

/// p_mobj.c:740–750: bit 16 (`MTF_NOTSINGLE`) and a missing medium skill bit filter a
/// record before its type is looked up, so a filtered decoration draws nothing and
/// a filtered barrel is not refused; the counters do not move.
#[test]
fn a_filtered_record_draws_nothing_and_is_not_type_checked() {
    for (kind, options) in [
        (2028, 16 | 7),
        (48, 16 | 2),
        (24, 1 | 4),
        (2035, 16 | 7),
        (2035, 1),
    ] {
        let mut lumps = room(5);
        lumps.clear_things();
        lumps.add_thing([300, 100, 0, kind, options]);
        let mut s = e1m1(&lumps).unwrap();
        let rng = s.rng;
        let report = s.spawn_map_things().unwrap();
        assert_eq!((report.spawned.len(), report.filtered), (0, 1), "{kind}");
        assert_eq!(s.rng, rng, "{kind} {options}");
        assert_eq!(s.actor_order().count(), 0);
        let stats = s.statistics.unwrap();
        assert_eq!((stats.total_kills, stats.total_items), (0, 0));
    }
    // The same lamp without bit 16 spawns with its one `lastlook` draw.
    let mut lumps = room(5);
    lumps.clear_things();
    lumps.add_thing([300, 100, 0, 2028, 7]);
    let mut s = e1m1(&lumps).unwrap();
    let [p0, m0] = s.rng.indices();
    assert_eq!(s.spawn_map_things().unwrap().spawned.len(), 1);
    assert_eq!(s.rng.indices(), [p0.wrapping_add(1), m0]);
}

/// Wire 20 fixes the setup options at medium single player (the context
/// `PLAYER-MONSTERS-E1M1-1-MEDIUM-v1`): the constructor takes no skill, the options
/// are medium, single player, monsters on, and the DSIM36 capture records them.
#[test]
fn the_e1m1_policy_is_medium_single_player_only() {
    let s = e1m1(&room(12)).unwrap();
    assert_eq!(s.actor_options.skill, Skill::Medium);
    assert_eq!(s.actor_options.mode, ActorGameMode::SinglePlayer);
    assert!(!s.actor_options.no_monsters);
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    Diagnostic::decode_e1m1(&bytes, limits).unwrap();
    let fields = crate::player_e1m1_tests::dsim_fields(&bytes);
    assert!(
        fields
            .iter()
            .any(|(p, v)| p == "actors.options" && *v == [2, 0, 0])
    );
}

/// Rule 5 (`MAXLINEANIMS`, M3h9 note §5.3 "Scrollers ≤ 64") under wire 20: 64
/// scrollers load and 65 are refused with `PursuitError::Specials`.
#[test]
fn e1m1_rejects_sixty_five_scrollers() {
    use crate::heights_room::{Fault, HeightsRoom};
    let mut walls = HeightsRoom::new(&[-512, 512], &[[0, 128]]);
    walls.segment = 32;
    let scrollers = |n: usize| {
        let faults: Vec<_> = (0..n).map(|l| Fault::TaggedSpecial(l, 48, 0)).collect();
        walls.clone_with(&faults)
    };
    assert_eq!(crate::sector_thinkers::MAX_LINE_ANIMS, 64);
    let map = scrollers(64).map();
    assert_eq!(
        map.linedefs().iter().filter(|l| l.special == 48).count(),
        64
    );
    Simulation::original_player_e1m1_map(map, options()).unwrap();
    assert_eq!(
        rejected(Simulation::original_player_e1m1_map(
            scrollers(65).map(),
            options()
        )),
        SimError::Pursuit(PursuitError::Specials)
    );
}

/// The crushers (p_spec.c line types 6, 25, 49, 55, 56, 57, 73, 74, 77, 141) stay
/// deferred under wire 20: on the scroller hall's DR door line or on a one-sided
/// wall each is `PursuitError::Specials`; the unchanged room loads.
#[test]
fn e1m1_rejects_every_crusher_line() {
    let base = room(0);
    e1m1(&base).unwrap();
    let mut probe = base.clone();
    let specials: Vec<i16> = {
        let l = &probe.lumps.iter().find(|(n, _)| n == "LINEDEFS").unwrap().1;
        (0..l.len() / 14)
            .map(|i| i16::from_le_bytes([l[14 * i + 6], l[14 * i + 7]]))
            .collect()
    };
    let door = specials.iter().position(|s| *s == 1).unwrap();
    let wall = probe.find_line([-512, 256], [0, 256]);
    assert_eq!(specials[wall], 0);
    for crusher in [6, 25, 49, 55, 56, 57, 73, 74, 77, 141] {
        for line in [door, wall] {
            let mut lumps = base.clone();
            lumps.set_line_special(line, crusher);
            assert_eq!(
                rejected(e1m1(&lumps)),
                SimError::Pursuit(PursuitError::Specials),
                "{crusher} on line {line}"
            );
        }
    }
}

/// Decorations stay out of the older policies: each of the seven types alone is
/// `UnsupportedType` under wires 18 (movers) and 19 (specials) with the state
/// unchanged, and loads under wire 20.
#[test]
fn each_decoration_type_is_refused_under_wires_18_and_19() {
    for ty in E1M1_DECORATIONS {
        let kind = i16::try_from(ty.descriptor().doomed_number).unwrap();
        let mut lumps = room(5);
        lumps.clear_things();
        lumps.add_thing([300, 100, 0, kind, 7]);
        for construct in [
            Simulation::original_player_movers_map as fn(_, _) -> _,
            Simulation::original_player_specials_map,
        ] {
            let mut s = construct(lumps.map(), options()).unwrap();
            let before = witness(&s);
            assert_eq!(
                s.spawn_map_things().map(|_| ()),
                Err(SimError::MapSpawn(MapSpawnError::UnsupportedType(kind)))
            );
            assert_eq!(witness(&s), before);
        }
        let mut s = e1m1(&lumps).unwrap();
        assert_eq!(s.spawn_map_things().unwrap().spawned.len(), 1);
    }
}

/// Free spots of the gallery floor (clear of the pedestals, the ledge and the
/// alcove), in lump order.
fn gallery_spots() -> impl Iterator<Item = [i16; 2]> {
    [-340i16, -280, -100, 100, 180]
        .into_iter()
        .flat_map(|y| (-8i16..=8).filter(|i| *i != 0).map(move |i| [60 * i, y]))
}
/// A gallery batch: `monsters` Zombiemen and Imps, `pickups` clips and stimpacks
/// and `decorations` of the seven types, interleaved in lump order, then `tail`.
fn gallery_batch(
    monsters: usize,
    pickups: usize,
    decorations: usize,
    tail: Option<i16>,
) -> crate::diagonal_room::Lumps {
    let mut lumps = room(5);
    lumps.clear_things();
    let mut spots = gallery_spots();
    let kinds = E1M1_DECORATIONS.map(|t| i16::try_from(t.descriptor().doomed_number).unwrap());
    let total = monsters.max(pickups).max(decorations);
    for i in 0..total {
        if i < monsters {
            let [x, y] = spots.next().unwrap();
            lumps.add_thing([x, y, 0, [3004, 3001][i % 2], 7]);
        }
        if i < decorations {
            let [x, y] = spots.next().unwrap();
            lumps.add_thing([x, y, 0, kinds[i % 7], 7]);
        }
        if i < pickups {
            let [x, y] = spots.next().unwrap();
            lumps.add_thing([x, y, 0, [2007, 2011][i % 2], 7]);
        }
    }
    if let Some(kind) = tail {
        let [x, y] = spots.next().unwrap();
        lumps.add_thing([x, y, 0, kind, 7]);
    }
    lumps
}

/// Rollback and atomicity of a spawn batch: with eight monsters, sixteen pickups and
/// 32 decorations interleaved the batch spawns all 56; a 33rd decoration at the end
/// is `DecorationLimit`, and a barrel after the decorations is `UnsupportedType`,
/// each leaving the capture bytes, both RNG streams, the actor order, the events and
/// the statistics unchanged. The successful batch draws one `lastlook` per thing plus
/// the tics draws of the monsters' spawn states.
#[test]
fn a_failed_spawn_batch_after_accepted_records_changes_nothing() {
    let full = gallery_batch(8, 16, MAX_DECORATIONS, None);
    let mut s = e1m1(&full).unwrap();
    let report = s.spawn_map_things().unwrap();
    assert_eq!(report.spawned.len(), 8 + 16 + MAX_DECORATIONS);
    let decorations = report
        .spawned
        .iter()
        .filter(|id| {
            crate::e1m1_rules::decoration(s.actor(**id).unwrap().life().unwrap().actor_type)
                .is_some()
        })
        .count();
    assert_eq!(decorations, MAX_DECORATIONS);
    spawn_refused(
        &gallery_batch(8, 16, MAX_DECORATIONS, Some(2028)),
        SimError::MapSpawn(MapSpawnError::DecorationLimit),
    );
    spawn_refused(
        &gallery_batch(8, 16, MAX_DECORATIONS + 1, None),
        SimError::MapSpawn(MapSpawnError::DecorationLimit),
    );
    spawn_refused(
        &gallery_batch(8, 16, 20, Some(2035)),
        SimError::MapSpawn(MapSpawnError::UnsupportedType(2035)),
    );
    spawn_refused(
        &gallery_batch(3, 2, 5, Some(49)),
        SimError::MapSpawn(MapSpawnError::UnsupportedType(49)),
    );
    // A ninth monster after the decorations is the roster bound (`WorkLimit`).
    let mut s = e1m1(&gallery_batch(8, 4, 12, Some(3004))).unwrap();
    let before = witness(&s);
    assert_eq!(
        s.spawn_map_things().map(|_| ()),
        Err(SimError::MapSpawn(MapSpawnError::WorkLimit))
    );
    assert_eq!(witness(&s), before);
}

/// `Simulation::set_momentum` refuses every nonzero push on every gallery
/// decoration, solid or flags 0 (`P_ThrustMobj` is reached only through damage of
/// a shootable thing), with the state unchanged; the zero push (what a decoration
/// already has) is accepted as for items and changes nothing.
#[test]
fn set_momentum_is_refused_on_every_decoration() {
    let (mut s, _, things) = crate::player_e1m1_tests::setup(5, [-128, 128]);
    assert_eq!(things.len(), 10);
    for id in things {
        let before = witness(&s);
        for push in [
            [Fixed::from_map_units(1), Fixed::ZERO],
            [Fixed::ZERO, Fixed::from_bits(-1)],
            [Fixed::from_map_units(-30), Fixed::from_map_units(30)],
        ] {
            assert_eq!(
                s.set_momentum(id, push),
                Err(SimError::PlayerBody(PlayerBodyError::Unsupported))
            );
        }
        // The shared 64-unit input bound precedes the role check, as for an item:
        // a push past it is that bound's error, found by the `player_e1m1` fuzz
        // campaign (a 30,976-unit push); 64 units exactly is still the refusal.
        assert_eq!(
            s.set_momentum(id, [Fixed::from_map_units(64), Fixed::ZERO]),
            Err(SimError::PlayerBody(PlayerBodyError::Unsupported))
        );
        for push in [
            [Fixed::from_map_units(30976), Fixed::from_map_units(112)],
            [Fixed::ZERO, Fixed::from_map_units(64) + Fixed::from_bits(1)],
            [Fixed::from_map_units(-32768), Fixed::ZERO],
        ] {
            assert_eq!(
                s.set_momentum(id, push),
                Err(SimError::Geometry(GeometryError::InvalidActor))
            );
        }
        assert_eq!(witness(&s), before);
        s.set_momentum(id, [Fixed::ZERO; 2]).unwrap();
        assert_eq!(witness(&s), before);
    }
}

/// The static fields a decoration keeps for its whole life (M3h9 note §5.1 item 2).
fn decoration_state(
    s: &Simulation,
    id: ActorId,
) -> (ActorTypeId, Option<StateId>, i32, i32, u32, Fixed, Fixed) {
    let a = s.actor(id).unwrap();
    let l = a.life().unwrap();
    (
        l.actor_type,
        l.state,
        l.tics,
        l.health,
        l.flags,
        a.radius(),
        a.height(),
    )
}

/// 2,400 tics with four seeds on the five gallery rooms (the ten decorations alone,
/// with a Zombieman beside the pillar or in the alcove, with an Imp before the
/// candelabra or the pool of blood) and the two liftroom rooms (a lamp and a corpse
/// on the lift, a pillar under the door) with a command stream that walks, turns,
/// fires and uses: after every tic V′ holds and every decoration is live with its
/// type, state, tics, health, flags, radius and height unchanged and `z` equal to its
/// floor; the decoration count never changes; every 10th tic `step` returns the
/// capture `advance` reaches; every 100th tic the DSIM36 capture decodes with
/// `decode_e1m1` and re-encodes to the same bytes.
#[test]
fn soaks_keep_decorations_static_step_equality_and_the_dsim36_round_trip() {
    let limits = DiagnosticLimits::default();
    let rooms = [
        "walk_into_lamp",
        "monster_path_around_pillar",
        "monster_hitscan_through_pillar",
        "fireball_hits_candelabra",
        "fireball_over_corpse",
        "decorations_ride_lift",
        "door_closes_on_pillar",
    ];
    let cases = crate::e1m1_replay::medium_cases();
    let (mut tics, mut uses, mut deco_tics, mut round_trips, mut moved_floors) = (0, 0, 0, 0, 0);
    for label in rooms {
        let base = cases.iter().find(|c| c.label == label).unwrap();
        for seed in [5u8, 77, 160, 241] {
            let mut c = base.clone();
            c.seed = seed;
            let (mut s, _, things) = crate::e1m1_replay::setup(&c);
            let decorations: Vec<_> = things
                .iter()
                .copied()
                .filter(|id| {
                    crate::e1m1_rules::decoration(s.actor(*id).unwrap().life().unwrap().actor_type)
                        .is_some()
                })
                .collect();
            assert!(!decorations.is_empty(), "{label}");
            let fixed: Vec<_> = decorations
                .iter()
                .map(|id| decoration_state(&s, *id))
                .collect();
            let floors: Vec<_> = decorations
                .iter()
                .map(|id| s.actor(*id).unwrap().z)
                .collect();
            let count = |s: &Simulation| {
                s.actor_order()
                    .filter(|id| {
                        crate::e1m1_rules::decoration(
                            s.actor(*id).unwrap().life().unwrap().actor_type,
                        )
                        .is_some()
                    })
                    .count()
            };
            let mut commands = Commands(0x9e37_79b9 ^ (u32::from(seed) << 9) ^ c.map as u32);
            for op in 1..=2400 {
                let dead = s.original_player.as_ref().unwrap().state == PlayerBodyState::Dead;
                let command = commands.command(dead);
                uses += usize::from(command[0].buttons & 2 != 0);
                if op % 10 == 0 {
                    let mut stepped = s.clone();
                    let snapshot = stepped.step(command);
                    let advanced = s.advance(command).map(|()| s.snapshot());
                    assert_eq!(snapshot, advanced, "{label} seed {seed} op{op}");
                } else {
                    s.advance(command)
                        .unwrap_or_else(|e| panic!("{label} seed {seed} op{op}: {e}"));
                }
                tics += 1;
                assert_v_prime(&s, label, op);
                assert_eq!(count(&s), decorations.len(), "{label} seed {seed} op{op}");
                for (i, id) in decorations.iter().enumerate() {
                    assert_eq!(s.actors.is_pending(*id), Some(false), "{label} op{op}");
                    assert_eq!(
                        decoration_state(&s, *id),
                        fixed[i],
                        "{label} seed {seed} op{op}"
                    );
                    let a = s.actor(*id).unwrap();
                    assert_eq!(a.z, a.floor, "{label} seed {seed} op{op} decoration {i}");
                    moved_floors += usize::from(a.z != floors[i]);
                    deco_tics += 1;
                }
                if op % 100 == 0 {
                    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
                    let view = Diagnostic::decode_e1m1(&bytes, limits)
                        .unwrap_or_else(|e| panic!("{label} seed {seed} op{op}: {e:?}"));
                    assert_eq!(view.encode(limits).unwrap(), bytes);
                    round_trips += 1;
                }
            }
        }
    }
    assert_eq!(tics, 7 * 4 * 2400);
    assert_eq!(round_trips, 7 * 4 * 24);
    assert!(uses > 1000 && moved_floors > 0, "{uses} {moved_floors}");
    println!(
        "e1m1 soaks: {tics} tics, {uses} use commands, {deco_tics} decoration-tics, {moved_floors} decoration-tics off the spawn floor height, {round_trips} round trips"
    );
}

/// The §5.3 budgets as formulas over the policy's bounds (`D` = decorations, `K` =
/// monsters, `P` = pickups, `R` = drops, `k` = Imps): `N = 1 + K + P + R + D` = 65;
/// the live actors `1 + K + P + 26k + (K−k) + 8(K−k) + R + D` peak at 273 (`k` = 8)
/// and, with the `39 − 3k` spawns of one pass, stay within the 320 actor slots for
/// every `k` (288 at `k` = 8, 176 at `k` = 0; the crude 273 + 39 = 312 too); the crude hitscan intercept bound
/// (E1M1 window 28 lines + `N` things) is 93 ≤ `MAX_INTERCEPTS`; `MAX_BONUS` is
/// `6 × P` = 96; scrollers ≤ `MAXLINEANIMS` 64.
#[test]
fn the_section_5_3_budgets_hold_as_formulas() {
    let (d, k_max, p, r) = (
        MAX_DECORATIONS,
        crate::player_monsters::MAX_PLAYER_MONSTERS,
        crate::world_pickups::MAX_WORLD_PICKUPS,
        crate::player_monsters::MAX_PLAYER_MONSTERS,
    );
    assert_eq!((d, k_max, p, r), (32, 8, 16, 8));
    let n = 1 + k_max + p + r + d;
    assert_eq!(n, E1M1_LINKED_THINGS);
    assert_eq!(n, 65);
    let slots = crate::player_map_scale::MAP_SCALE_ACTOR_SLOTS as usize;
    assert_eq!(slots, 320);
    let live = |k: usize| 1 + k_max + p + 26 * k + (k_max - k) + 8 * (k_max - k) + r + d;
    let spawns = |k: usize| 39 - 3 * k;
    assert_eq!((live(8), spawns(8), live(0), spawns(0)), (273, 15, 137, 39));
    assert_eq!((live(8) + spawns(8), live(0) + spawns(0)), (288, 176));
    // The crude bound (the most live actors plus the most spawns) fits too.
    assert!(live(8) + spawns(0) <= slots);
    assert_eq!(live(8) + spawns(0), 312);
    for k in 0..=k_max {
        assert!(live(k) <= 273, "{k}");
        assert!(live(k) + spawns(k) <= slots, "{k}");
    }
    assert_eq!((0..=k_max).map(live).max(), Some(273));
    assert!(28 + n <= crate::player_map_scale::MAX_INTERCEPTS);
    assert_eq!(28 + n, 93);
    assert_eq!(crate::world_pickups::MAX_BONUS, 6 * p as i32);
    assert_eq!(crate::world_pickups::MAX_BONUS, 96);
    assert_eq!(crate::sector_thinkers::MAX_LINE_ANIMS, 64);
}
/// The API refuses what the source never does to a decoration, as it refuses a push,
/// a state entry and damage: a target (`set_actor_target`) and a removal
/// (`mark_remove`) on any of the ten gallery decorations are
/// `PlayerBodyError::Unsupported` with the state unchanged, and a monster may not
/// target one (`PerceptionError::Roster`); the world then still runs and every
/// capture decodes. Before this refusal both were accepted and left a state the tic
/// preflight (`PlayerBodyError::Invariant`) or the DSIM36 reader (`Identity`)
/// refused.
#[test]
fn a_decoration_refuses_a_target_and_a_removal() {
    let (mut s, body, things) = crate::player_e1m1_tests::setup(6, [-128, 128]);
    let zombie = things[10];
    let limits = DiagnosticLimits::default();
    for &id in &things[..10] {
        let before = witness(&s);
        for target in [Some(body), Some(zombie), None] {
            assert_eq!(
                s.set_actor_target(id, target),
                Err(SimError::PlayerBody(PlayerBodyError::Unsupported))
            );
        }
        assert_eq!(
            s.mark_remove(id),
            Err(SimError::PlayerBody(PlayerBodyError::Unsupported))
        );
        assert_eq!(
            s.set_actor_target(zombie, Some(id)),
            Err(SimError::Perception(PerceptionError::Roster))
        );
        assert_eq!(witness(&s), before);
    }
    s.advance([TicCommand::default(); 4]).unwrap();
    assert_eq!(s.actor_order().count(), 12);
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    Diagnostic::decode_e1m1(&bytes, limits).unwrap();
    // The Zombieman itself may still be targeted and removed.
    s.set_actor_target(zombie, Some(body)).unwrap();
    s.mark_remove(zombie).unwrap();
}

/// `e1m1_mover_bounds` admits all thirteen committed M3h9 rooms; where the movers
/// rules also admit a room (no two-sided 48) the bounds are the same, and the
/// rooms with a two-sided 48 are `PursuitError::Specials` under the movers rules.
#[test]
fn e1m1_mover_bounds_extend_the_movers_bounds_by_the_two_sided_scrollers() {
    for index in 0..13 {
        let map = room(index).map();
        let bounds = crate::e1m1_mover_bounds(&map).unwrap();
        let two_sided_48 = map
            .linedefs()
            .iter()
            .any(|l| l.special == 48 && l.sides[1].is_some());
        match crate::mover_bounds(&map) {
            Ok(old) => {
                assert!(!two_sided_48, "room {index}");
                assert_eq!(old, bounds);
            }
            Err(error) => {
                assert!(two_sided_48, "room {index}");
                assert_eq!(error, SimError::Pursuit(PursuitError::Specials));
            }
        }
    }
    assert_eq!(
        crate::e1m1_mover_bounds(
            &crate::diagonal_room::Lumps::parse(include_bytes!(
                "../../../tests/reference/m3h9_dense9.bin"
            ))
            .map()
        ),
        Err(SimError::MapScale(MapScaleError::SpecialWindow))
    );
}
