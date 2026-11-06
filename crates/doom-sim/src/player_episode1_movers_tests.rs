//! M4a Episode 1 policy, plan step 4: every part-1 line special (decision H1) on
//! small synthetic rooms built from the committed M3h8 rooms, the dispatch tables
//! (use, cross, shot) with their retrigger classes, and the witnesses of decisions
//! H7 and H8 (`SecondDoor`, `StairBlocked`, `PlatBlocked`, `DonutBusy`) and the
//! per-tic `MoverFamily` on full tics. Rust-only policy tests: the expected values
//! come from the source reading (p_doors.c, p_floor.c, p_plats.c, p_spec.c,
//! p_switch.c, p_lights.c, g_game.c), never from the C engine; the C-side checks
//! are the DSIM39 goldens of the M4a synthetic WADs.
use crate::diagonal_room::Lumps;
use crate::movers::{DoorKind, FloorKind, PlatStatus, PlatType};
use crate::*;

const EPISODE1: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V1;
const LEVEL: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersLevelV1;

fn room(index: usize) -> Lumps {
    crate::player_movers_tests::room(index)
}
fn lump<'a>(l: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn set_line(l: &mut Lumps, line: usize, special: i16, tag: i16) {
    let v = lump(l, "LINEDEFS");
    v[14 * line + 6..14 * line + 8].copy_from_slice(&special.to_le_bytes());
    v[14 * line + 8..14 * line + 10].copy_from_slice(&tag.to_le_bytes());
}
fn set_heights(l: &mut Lumps, sector: usize, floor: i16, ceiling: i16) {
    let s = lump(l, "SECTORS");
    s[26 * sector..26 * sector + 2].copy_from_slice(&floor.to_le_bytes());
    s[26 * sector + 2..26 * sector + 4].copy_from_slice(&ceiling.to_le_bytes());
}
fn set_floor_flat(l: &mut Lumps, sector: usize, name: &[u8; 8]) {
    lump(l, "SECTORS")[26 * sector + 4..26 * sector + 12].copy_from_slice(name);
}
fn set_sector_special(l: &mut Lumps, sector: usize, special: i16, tag: i16) {
    let s = lump(l, "SECTORS");
    s[26 * sector + 22..26 * sector + 24].copy_from_slice(&special.to_le_bytes());
    s[26 * sector + 24..26 * sector + 26].copy_from_slice(&tag.to_le_bytes());
}
fn set_front_texture(l: &mut Lumps, line: usize, name: &[u8; 8]) {
    let v = lump(l, "LINEDEFS");
    let front = usize::from(u16::from_le_bytes([v[14 * line + 10], v[14 * line + 11]]));
    lump(l, "SIDEDEFS")[30 * front + 20..30 * front + 28].copy_from_slice(name);
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
fn load(l: &Lumps, policy: OriginalExecutionProfile) -> Result<Simulation, SimError> {
    let result = if policy == EPISODE1 {
        Simulation::original_player_episode1_map(l.map(), options())
    } else {
        Simulation::original_player_level_map(l.map(), options())
    };
    result.map_err(|e| match e {
        OriginalRegistryError::Runtime(e) => e,
        other => panic!("{other:?}"),
    })
}
/// Adds a player start at (1000, 64) facing west (floors room sector 4) when the
/// room has none.
fn with_player(l: &Lumps) -> Lumps {
    let mut l = l.clone();
    let things = lump(&mut l, "THINGS").clone();
    if !things
        .chunks(10)
        .any(|t| i16::from_le_bytes([t[6], t[7]]) == 1)
    {
        thing(&mut l, 1000, 64, 180, 1);
    }
    l
}
fn player(s: &Simulation) -> ActorId {
    s.original_player.as_ref().and_then(|p| p.mo).unwrap()
}
fn sim(l: &Lumps) -> Simulation {
    let mut s = load(&with_player(l), EPISODE1).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn zero() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
fn use_command() -> [TicCommand; 4] {
    let mut c = zero();
    c[0].buttons = weapons::BT_USE;
    c
}
fn units(v: i16) -> Fixed {
    Fixed::from_map_units(v)
}
/// The live sector movers in shared order.
fn movers(s: &Simulation) -> Vec<SectorThinker> {
    s.shared_thinker_order()
        .filter_map(ThinkerId::sector)
        .filter(|&id| s.actors.sector_is_pending(id) == Some(false))
        .filter_map(|id| s.sector_thinker(id).copied())
        .filter(|t| t.mover_sector().is_some())
        .collect()
}
fn floor(s: &Simulation, sector: usize) -> Fixed {
    s.sectors[sector].floor
}
fn ceiling(s: &Simulation, sector: usize) -> Fixed {
    s.sectors[sector].ceiling
}
fn special(s: &Simulation, sector: usize) -> sector_thinkers::SectorSpecial {
    s.specials.as_ref().unwrap().sectors[sector]
}
fn sfx(name: &str) -> u16 {
    lifecycle::source_sound_id(name)
}
fn has_sound(s: &Simulation, name: &str) -> bool {
    s.events.iter().any(|e| match e {
        Event::Mover(MoverEvent::SectorSound { sound, .. }) => *sound == sfx(name),
        Event::ActorState(lifecycle::ActorStateEvent::Sound { sound, .. }) => *sound == sfx(name),
        _ => false,
    })
}
/// Runs zero commands until no mover is live (at most `limit` tics).
fn settle(s: &mut Simulation, limit: usize) {
    for _ in 0..limit {
        if movers(s).is_empty() {
            return;
        }
        s.advance(zero()).unwrap();
    }
    panic!("movers still live: {:?}", movers(s));
}
/// Asserts that `advance(commands)` refuses with `error`, the canonical state
/// unchanged, and that the next valid tic (zero commands) still advances.
fn refuses(s: &mut Simulation, commands: [TicCommand; 4], error: MapScaleError) {
    let before = s.snapshot().canonical_bytes();
    assert_eq!(s.advance(commands), Err(SimError::MapScale(error)));
    assert_eq!(s.snapshot().canonical_bytes(), before);
    Diagnostic::decode_episode1(&before, DiagnosticLimits::default()).unwrap();
}

/// The doorway room (0) with the player 32 units west of the middle DR line 4
/// (door sector 2), facing it, and that line given `special`.
fn doorway(special: i16) -> Lumps {
    let mut l = room(0);
    set_line(&mut l, 4, special, 0);
    thing(&mut l, -32, 0, 0, 1);
    l
}
/// Releases the use key (`usedown` starts true) and then uses once.
fn use_once(s: &mut Simulation) -> Result<(), SimError> {
    s.advance(zero())?;
    s.advance(use_command())
}

// ---- Dispatch tables -------------------------------------------------------

/// Rule 1 under wire 23: the 29 specials of decision H1 load (each on the floors
/// room's trigger line 42 with a tag, a manual door on the doorway room's line 4)
/// while wire 22 refuses them; 97, 24, 47 and other numbers still refuse.
#[test]
fn rule_1_admits_exactly_the_29_part_1_specials() {
    let manual = [26, 27, 28, 31, 32, 33, 34];
    let tagged = [
        2, 5, 7, 8, 9, 16, 18, 20, 22, 23, 35, 46, 51, 62, 70, 76, 82, 86, 90, 91, 98, 103,
    ];
    assert_eq!(manual.len() + tagged.len(), 29);
    for special in manual {
        let l = doorway(special);
        load(&l, EPISODE1).unwrap_or_else(|e| panic!("{special}: {e:?}"));
        assert_eq!(
            load(&l, LEVEL).err(),
            Some(SimError::Pursuit(PursuitError::Specials)),
            "{special}"
        );
    }
    for special in tagged {
        let mut l = room(13);
        set_sector_special(&mut l, 10, 0, 9);
        set_line(&mut l, 42, special, 9);
        if special == 9 {
            // The donut ring needs its geometry (decision H8).
            l = donut_room();
        }
        load(&l, EPISODE1).unwrap_or_else(|e| panic!("{special}: {e:?}"));
        assert!(load(&l, LEVEL).is_err(), "{special}");
    }
    for special in [3, 4, 10, 24, 47, 97, 99, 117] {
        let mut l = room(13);
        set_line(&mut l, 42, special, 9);
        assert_eq!(
            load(&l, EPISODE1).err(),
            Some(SimError::Pursuit(PursuitError::Specials)),
            "{special}"
        );
    }
}

/// `P_CrossSpecialLine` for a player: the W1 lines clear after their call even
/// when it starts nothing, the WR lines keep their special, and each runs its
/// starter (the call record names the function and its type argument).
#[test]
fn cross_dispatch_routes_every_new_line_with_its_retrigger_class() {
    use crate::planes::MoverFunction as F;
    let cases: [(i16, F, i32, bool); 12] = [
        (2, F::DoDoor, 3, true),
        (5, F::DoFloor, 3, true),
        (8, F::BuildStairs, 0, true),
        (16, F::DoDoor, 1, true),
        (22, F::DoPlat, 3, true),
        (76, F::DoDoor, 1, false),
        (82, F::DoFloor, 1, false),
        (86, F::DoDoor, 3, false),
        (90, F::DoDoor, 0, false),
        (91, F::DoFloor, 3, false),
        (98, F::DoFloor, 2, false),
        (35, F::DoDoor, -1, true),
    ];
    for (special, function, argument, w1) in cases {
        let mut l = room(13);
        set_sector_special(&mut l, 10, 0, 9);
        set_line(&mut l, 42, special, 9);
        thing(&mut l, 900, 250, 0, 1);
        let mut s = sim(&l);
        let id = player(&s);
        let crossing = line_triggers::Crossing {
            line: 42,
            side: 0,
            special,
        };
        let after = s
            .start_movers(|m| line_triggers::cross(m, id, line_triggers::Crosser::Player, crossing))
            .unwrap();
        assert_eq!(after == 0, w1, "{special}");
        let call = s.pending.iter().find_map(|e| match e {
            Event::Mover(MoverEvent::Call {
                function, argument, ..
            }) => Some((*function, *argument)),
            _ => None,
        });
        if special == 35 {
            assert_eq!(call, None);
            assert_eq!(special_of(&s, 10).light, 35);
        } else {
            assert_eq!(call, Some((function, argument)), "{special}");
        }
        // A monster crossing does nothing on any of them.
        let mut s = sim(&l);
        let after = s
            .start_movers(|m| {
                line_triggers::cross(m, id, line_triggers::Crosser::Monster, crossing)
            })
            .unwrap();
        assert_eq!(after, special);
        assert!(movers(&s).is_empty());
    }
}
fn special_of(s: &Simulation, sector: usize) -> sector_thinkers::SectorSpecial {
    special(s, sector)
}

/// `P_UseSpecialLine`: the S1 switches change their switch (clearing the line)
/// only when their call returned nonzero, 51 always; the SR switches keep the
/// special and start a button only on success; a monster may use only 1 and the
/// locked D1 lines (returning at once).
#[test]
fn use_dispatch_routes_every_switch_with_its_success_test() {
    use crate::planes::MoverFunction as F;
    // (special, function, argument, reusable)
    let cases: [(i16, F, i32, bool); 8] = [
        (7, F::BuildStairs, 0, false),
        (18, F::DoFloor, 4, false),
        (20, F::DoPlat, 3, false),
        (23, F::DoFloor, 1, false),
        (103, F::DoDoor, 3, false),
        (62, F::DoPlat, 1, true),
        (70, F::DoFloor, 2, true),
        (51, F::DoDoor, -1, false),
    ];
    for (special, function, argument, reusable) in cases {
        let mut l = room(13);
        set_sector_special(&mut l, 10, 0, 9);
        set_heights(&mut l, 10, 0, 128);
        set_line(&mut l, 42, special, 9);
        set_front_texture(&mut l, 42, b"SW1COMM\0");
        thing(&mut l, 900, 250, 0, 1);
        let mut s = sim(&l);
        let id = player(&s);
        let mut buttons = [None; mover_rules::MAX_BUTTONS];
        let mut outcome = use_lines::UseOutcome::default();
        let used = s
            .start_movers(|m| {
                use_lines::use_special_line(
                    m,
                    Some(&mut buttons),
                    id,
                    use_lines::User::Player,
                    42,
                    0,
                    special,
                    &mut outcome,
                )
            })
            .unwrap();
        assert!(used);
        assert_eq!(outcome.cleared.is_some(), !reusable, "{special}");
        assert_eq!(buttons[0].is_some(), reusable, "{special}");
        assert_eq!(
            (outcome.exit, outcome.secret),
            (special == 51, special == 51)
        );
        let call = s.pending.iter().find_map(|e| match e {
            Event::Mover(MoverEvent::Call {
                function, argument, ..
            }) => Some((*function, *argument)),
            _ => None,
        });
        if special == 51 {
            assert_eq!(call, None);
        } else {
            assert_eq!(call, Some((function, argument)), "{special}");
            // Used again with nothing to start: the special and buttons stay.
            let mut buttons = [None; mover_rules::MAX_BUTTONS];
            let mut outcome = use_lines::UseOutcome::default();
            s.start_movers(|m| {
                use_lines::use_special_line(
                    m,
                    Some(&mut buttons),
                    id,
                    use_lines::User::Player,
                    42,
                    0,
                    special,
                    &mut outcome,
                )
            })
            .unwrap();
            assert_eq!(outcome.cleared, None, "{special}");
            assert!(buttons.iter().all(Option::is_none), "{special}");
        }
        // A monster: false on every switch.
        let mut outcome = use_lines::UseOutcome::default();
        let monster = s
            .start_movers(|m| {
                use_lines::use_special_line(
                    m,
                    None,
                    id,
                    use_lines::User::Monster,
                    42,
                    0,
                    special,
                    &mut outcome,
                )
            })
            .unwrap();
        assert!(!monster, "{special}");
    }
    // A monster on a locked D1 line: true, and nothing happens.
    for special in [32, 33, 34] {
        let mut s = sim(&doorway(special));
        let id = player(&s);
        let mut outcome = use_lines::UseOutcome::default();
        let used = s
            .start_movers(|m| {
                use_lines::use_special_line(
                    m,
                    None,
                    id,
                    use_lines::User::Monster,
                    4,
                    0,
                    special,
                    &mut outcome,
                )
            })
            .unwrap();
        assert!(used);
        assert!(movers(&s).is_empty());
        assert_eq!(outcome, use_lines::UseOutcome::default());
    }
}

// ---- Doors -----------------------------------------------------------------

/// D1 31 by a full-tic use: an `open` door rising to the lowest neighbour
/// ceiling − 4, the line cleared; removed at the top. A second D1 use on the busy
/// sector would spawn a second thinker in the source: `SecondDoor`.
#[test]
fn d1_open_door_clears_its_line_and_a_busy_sector_is_second_door() {
    let mut s = sim(&doorway(31));
    use_once(&mut s).unwrap();
    match movers(&s).as_slice() {
        [SectorThinker::Door(d)] => {
            assert_eq!((d.sector, d.kind, d.direction), (2, DoorKind::Open, 1));
            assert_eq!(d.topheight, units(124));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(s.line_specials[4], 0);
    assert!(has_sound(&s, "sfx_doropn"));
    settle(&mut s, 200);
    assert_eq!(ceiling(&s, 2), units(124));
    // SecondDoor: the D1 line restored on a fresh run, its sector busy with a
    // lift started directly.
    let mut l = doorway(32);
    set_line(&mut l, 2, 88, 7);
    set_sector_special(&mut l, 2, 0, 7);
    let mut s = sim(&l);
    s.original_player
        .as_mut()
        .unwrap()
        .powers
        .as_mut()
        .unwrap()
        .cards[0] = true;
    assert!(
        s.start_movers(|m| m.do_plat(2, PlatType::DownWaitUpStay))
            .unwrap()
    );
    s.advance(zero()).unwrap();
    refuses(&mut s, use_command(), MapScaleError::SecondDoor);
    s.advance(zero()).unwrap();
}

/// The locked DR lines 26–28 without the key: the `PD_*` message (DSIM codes
/// 27–29, decision D7), `sfx_oof`, no thinker; with the key (card or skull of the
/// colour) a `normal` door. The D1 locked lines read the same key pairs.
#[test]
fn locked_doors_check_the_cards_and_set_the_message() {
    for (special, card, message) in [
        (26, 0, PickupMessage::BlueKeyNeeded),
        (27, 1, PickupMessage::YellowKeyNeeded),
        (28, 2, PickupMessage::RedKeyNeeded),
        (32, 0, PickupMessage::BlueKeyNeeded),
        (33, 2, PickupMessage::RedKeyNeeded),
        (34, 1, PickupMessage::YellowKeyNeeded),
    ] {
        let mut s = sim(&doorway(special));
        use_once(&mut s).unwrap();
        assert!(movers(&s).is_empty(), "{special}");
        let p = s.original_player.as_ref().unwrap();
        assert_eq!(p.pickups.unwrap().message, message);
        assert!(has_sound(&s, "sfx_oof"));
        let capture = s.snapshot().canonical_bytes();
        Diagnostic::decode_episode1(&capture, DiagnosticLimits::default()).unwrap();
        // The skull of the colour (card index + 3) opens it too.
        for key in [card, card + 3] {
            let mut s = sim(&doorway(special));
            s.original_player
                .as_mut()
                .unwrap()
                .powers
                .as_mut()
                .unwrap()
                .cards[key] = true;
            use_once(&mut s).unwrap();
            let kind = if special >= 31 {
                DoorKind::Open
            } else {
                DoorKind::Normal
            };
            match movers(&s).as_slice() {
                [SectorThinker::Door(d)] => assert_eq!(d.kind, kind, "{special}"),
                other => panic!("{special}: {other:?}"),
            }
            assert_eq!(s.line_specials[4] == 0, special >= 31);
        }
    }
}

/// A DR line (here the locked 26 with its key) on a sector whose mover is a lift:
/// `specialdata` cast to a door reads a plat, refused at the tic as `MoverFamily`.
#[test]
fn dr_on_a_plat_sector_is_mover_family_at_the_tic() {
    let mut l = doorway(26);
    set_line(&mut l, 2, 88, 7);
    set_sector_special(&mut l, 2, 0, 7);
    let mut s = sim(&l);
    s.original_player
        .as_mut()
        .unwrap()
        .powers
        .as_mut()
        .unwrap()
        .cards[0] = true;
    assert!(
        s.start_movers(|m| m.do_plat(2, PlatType::DownWaitUpStay))
            .unwrap()
    );
    s.advance(zero()).unwrap();
    refuses(&mut s, use_command(), MapScaleError::MoverFamily);
}

/// `EV_DoDoor(close30ThenOpen)` (W1 16): top at the current ceiling, closing with
/// `sfx_dorcls`; waits 35·30 tics at the floor, reopens with `sfx_doropn` and is
/// removed at its old ceiling. `open` (WR 86) is removed at the top, `normal` (WR
/// 90) waits 150 tics there and closes.
#[test]
fn close30_open_and_normal_doors_follow_t_verticaldoor() {
    let mut l = room(13);
    set_sector_special(&mut l, 10, 0, 9);
    set_line(&mut l, 42, 16, 9);
    let mut s = sim(&l);
    assert!(
        s.start_movers(|m| m.do_door(42, DoorKind::Close30ThenOpen))
            .unwrap()
    );
    match movers(&s).as_slice() {
        [SectorThinker::Door(d)] => {
            assert_eq!((d.direction, d.topheight), (-1, units(128)));
        }
        other => panic!("{other:?}"),
    }
    assert!(s.pending.iter().any(|e| matches!(
        e,
        Event::Mover(MoverEvent::SectorSound { sound, .. }) if *sound == sfx("sfx_dorcls")
    )));
    // 64 steps of 2 units, then the pastdest step at the floor.
    for _ in 0..65 {
        s.advance(zero()).unwrap();
    }
    assert_eq!(ceiling(&s, 10), Fixed::ZERO);
    match movers(&s).as_slice() {
        [SectorThinker::Door(d)] => assert_eq!((d.direction, d.topcountdown), (0, 1050)),
        other => panic!("{other:?}"),
    }
    for _ in 0..1050 {
        s.advance(zero()).unwrap();
    }
    assert!(has_sound(&s, "sfx_doropn"));
    settle(&mut s, 80);
    assert_eq!(ceiling(&s, 10), units(128));

    // open: removed at the top; normal: waits 150 and comes back down.
    for (kind, waits) in [(DoorKind::Open, false), (DoorKind::Normal, true)] {
        let mut l = room(13);
        set_sector_special(&mut l, 10, 0, 9);
        set_heights(&mut l, 10, 0, 0);
        set_heights(&mut l, 11, 0, 128);
        set_line(&mut l, 42, if waits { 90 } else { 86 }, 9);
        let mut s = sim(&l);
        assert!(s.start_movers(|m| m.do_door(42, kind)).unwrap());
        let mut top = false;
        for _ in 0..400 {
            s.advance(zero()).unwrap();
            if let [SectorThinker::Door(d)] = movers(&s).as_slice() {
                top |= d.direction == 0 && d.topcountdown == 150;
            }
            if movers(&s).is_empty() {
                break;
            }
        }
        assert!(movers(&s).is_empty());
        assert_eq!(top, waits);
        assert_eq!(
            ceiling(&s, 10),
            if waits { Fixed::ZERO } else { units(124) }
        );
    }
}

/// GR 46 shot by the player's pistol: `EV_DoDoor(open)` and a button whatever the
/// call returned (decision D3: the switch texture matches), the door appended
/// before the puff; the special stays.
#[test]
fn shot_line_opens_its_door_and_restarts_its_button() {
    let mut l = room(13);
    set_sector_special(&mut l, 10, 0, 9);
    set_heights(&mut l, 10, 0, 0);
    set_heights(&mut l, 11, 0, 128);
    // The east wall of sector 4 (line 24) as the gun line, the player west of it.
    set_line(&mut l, 24, 46, 9);
    set_front_texture(&mut l, 24, b"SW1COMM\0");
    thing(&mut l, 1000, 64, 0, 1);
    let mut s = sim(&l);
    let mut fire = zero();
    fire[0].buttons = weapons::BT_ATTACK;
    let mut opened = false;
    for _ in 0..20 {
        s.advance(fire).unwrap();
        if !movers(&s).is_empty() {
            opened = true;
            break;
        }
    }
    assert!(opened, "the shot started no door");
    match movers(&s).as_slice() {
        [SectorThinker::Door(d)] => assert_eq!((d.sector, d.kind), (10, DoorKind::Open)),
        other => panic!("{other:?}"),
    }
    assert_eq!(s.line_specials[24], 46);
    assert!(s.buttons.iter().flatten().any(|b| b.line == 24));
    // The door precedes the puff in the shared order.
    let order: Vec<_> = s.shared_thinker_order().collect();
    let door = order.iter().position(|t| {
        t.sector().is_some_and(|id| {
            s.sector_thinker(id)
                .is_some_and(|m| m.mover_sector() == Some(10))
        })
    });
    let last_actor = order.iter().rposition(|t| t.sector().is_none());
    assert!(door < last_actor, "{order:?}");
    Diagnostic::decode_episode1(&s.snapshot().canonical_bytes(), DiagnosticLimits::default())
        .unwrap();
}

// ---- Floors ----------------------------------------------------------------

/// `EV_DoFloor`: `raiseFloor` to the lowest neighbour ceiling (from `MAXINT`,
/// at most its own), `raiseFloorToNearest` to the next higher neighbour floor,
/// `lowerFloorToLowest` and `turboLower`; the DSIM39 `direction`/`speed` tail.
#[test]
fn floors_reach_their_source_destinations() {
    // Sector 5 (floor 32, tag 2, a zombieman on it): neighbours 0 (floor 0,
    // ceiling 192) and 6 (floor 64, ceiling lowered to 100); its own ceiling 192.
    for (kind, dest, direction, speed) in [
        (FloorKind::RaiseFloor, 100, 1, 1),
        (FloorKind::RaiseFloorToNearest, 64, 1, 1),
        (FloorKind::LowerFloorToLowest, 0, -1, 1),
        (FloorKind::TurboLower, 72, -1, 4),
    ] {
        let mut l = room(13);
        set_heights(&mut l, 5, 32, 192);
        set_heights(&mut l, 6, 64, 100);
        let mut s = sim(&l);
        assert!(s.start_movers(|m| m.do_floor(4, kind)).unwrap());
        match movers(&s).as_slice() {
            [SectorThinker::Floor(f)] => {
                assert_eq!(f.floordestheight, units(dest), "{kind:?}");
                assert_eq!((f.direction, f.speed), (direction, units(speed)));
            }
            other => panic!("{other:?}"),
        }
        Diagnostic::decode_episode1(&s.snapshot().canonical_bytes(), DiagnosticLimits::default())
            .unwrap();
        settle(&mut s, 400);
        assert_eq!(floor(&s, 5), units(dest), "{kind:?}");
    }
    // raiseFloor without a lower neighbour ceiling stops at its own ceiling.
    let mut l = room(13);
    set_heights(&mut l, 5, 32, 100);
    let mut s = sim(&l);
    assert!(
        s.start_movers(|m| m.do_floor(4, FloorKind::RaiseFloor))
            .unwrap()
    );
    match movers(&s).as_slice() {
        [SectorThinker::Floor(f)] => assert_eq!(f.floordestheight, units(100)),
        other => panic!("{other:?}"),
    }
}

// ---- Plats -----------------------------------------------------------------

fn lift_room() -> Lumps {
    // Lift room 7: sector 3 (tag 1) between sectors 1 (floor 2) and 2 (floor
    // 64); line 12 (front 2) re-specialled 20.
    let mut l = room(7);
    l.clear_things();
    set_heights(&mut l, 3, 2, 192);
    set_floor_flat(&mut l, 2, b"FLAT20\0\0");
    set_sector_special(&mut l, 3, 5, 1);
    set_line(&mut l, 12, 20, 1);
    set_front_texture(&mut l, 12, b"SW1COMM\0");
    set_line(&mut l, 0, 0, 0);
    thing(&mut l, 256, 64, 90, 1);
    l
}

/// `raiseToNearestAndChange`: the sector takes the floor flat of the line's front
/// sector and special 0 at the start (DSIM39 `floorpic`), rises at 1/2 unit to
/// the next higher neighbour floor and is removed there; plat `type` 3, `low` 0.
#[test]
fn raise_to_nearest_and_change_copies_the_front_flat_and_clears_the_special() {
    let mut s = sim(&lift_room());
    assert_eq!(special(&s, 3).special, 5);
    assert!(
        s.start_movers(|m| m.do_plat(12, PlatType::RaiseToNearestAndChange))
            .unwrap()
    );
    assert_eq!(special(&s, 3).floorpic, *b"FLAT20\0\0");
    assert_eq!(special(&s, 3).special, 0);
    match movers(&s).as_slice() {
        [SectorThinker::Plat(p)] => {
            assert_eq!(
                (p.kind, p.status, p.low, p.high),
                (
                    PlatType::RaiseToNearestAndChange,
                    PlatStatus::Up,
                    Fixed::ZERO,
                    units(64)
                )
            );
        }
        other => panic!("{other:?}"),
    }
    let capture = s.snapshot().canonical_bytes();
    Diagnostic::decode_episode1(&capture, DiagnosticLimits::default()).unwrap();
    let fields: Vec<_> = s
        .snapshot()
        .fields()
        .map(|f| (f.path().to_string(), f.value().to_vec()))
        .collect();
    assert!(
        fields
            .iter()
            .any(|(p, v)| p == "sector.3.floorpic" && v == b"FLAT20\0\0")
    );
    settle(&mut s, 200);
    assert_eq!(floor(&s, 3), units(64));
}

/// A blocked `raiseToNearestAndChange` rise (a barrel that does not fit under the
/// ceiling) refuses the tic with `PlatBlocked` (decision H7), state unchanged.
#[test]
fn blocked_raise_to_nearest_and_change_is_plat_blocked() {
    let mut l = lift_room();
    set_heights(&mut l, 3, 2, 46);
    thing(&mut l, 256, 320, 0, 2035);
    let mut s = sim(&l);
    assert!(
        s.start_movers(|m| m.do_plat(12, PlatType::RaiseToNearestAndChange))
            .unwrap()
    );
    for tic in 0..40 {
        let before = s.snapshot().canonical_bytes();
        match s.advance(zero()) {
            Ok(()) => continue,
            Err(e) => {
                assert_eq!(
                    e,
                    SimError::MapScale(MapScaleError::PlatBlocked),
                    "tic {tic}"
                );
                assert_eq!(s.snapshot().canonical_bytes(), before);
                return;
            }
        }
    }
    panic!("the blocked plat was not refused");
}

// ---- Stairs ----------------------------------------------------------------

/// The floors room with sector 0 tagged 9 for a stair line (42, W1 8).
fn stair_room() -> Lumps {
    let mut l = room(13);
    set_sector_special(&mut l, 0, 0, 9);
    set_line(&mut l, 42, 8, 9);
    l
}
fn floors(s: &Simulation) -> Vec<(u16, FloorKind, i32, i32, bool)> {
    movers(s)
        .into_iter()
        .filter_map(|m| match m {
            SectorThinker::Floor(f) => Some((
                f.sector,
                f.kind,
                f.direction,
                f.floordestheight.bits() >> 16,
                f.stair,
            )),
            _ => None,
        })
        .collect()
}

/// `EV_BuildStairs(build8)`: from tagged sector 0 the walk takes the first
/// two-sided line of each step's list whose front is that step and whose back
/// has the first step's flat (0 → 5 → 6), each step 8 higher, rising at 1/4
/// unit with `type` 0; a busy sector on the way is skipped but still adds 8.
#[test]
fn stairs_walk_the_front_sides_and_count_skipped_busy_steps() {
    let mut s = sim(&stair_room());
    assert!(s.start_movers(|m| m.build_stairs(42)).unwrap());
    assert_eq!(
        floors(&s),
        vec![
            (0, FloorKind::LowerFloor, 1, 8, true),
            (5, FloorKind::LowerFloor, 1, 16, true),
            (6, FloorKind::LowerFloor, 1, 24, true),
        ]
    );
    let speed = match movers(&s)[0] {
        SectorThinker::Floor(f) => f.speed,
        _ => unreachable!(),
    };
    assert_eq!(speed, Fixed::from_bits(16384));
    Diagnostic::decode_episode1(&s.snapshot().canonical_bytes(), DiagnosticLimits::default())
        .unwrap();
    // A flat change stops the walk.
    let mut l = stair_room();
    set_floor_flat(&mut l, 5, b"OTHER\0\0\0");
    let mut s = sim(&l);
    assert!(s.start_movers(|m| m.build_stairs(42)).unwrap());
    assert_eq!(
        floors(&s),
        vec![
            (0, FloorKind::LowerFloor, 1, 8, true),
            (6, FloorKind::LowerFloor, 1, 16, true),
        ]
    );
    // Sector 5 busy (turboLower from line 4): skipped, its 8 still counted.
    let mut s = sim(&stair_room());
    assert!(
        s.start_movers(|m| m.do_floor(4, FloorKind::TurboLower))
            .unwrap()
    );
    assert!(s.start_movers(|m| m.build_stairs(42)).unwrap());
    assert_eq!(
        floors(&s)[1..],
        [
            (0, FloorKind::LowerFloor, 1, 8, true),
            (6, FloorKind::LowerFloor, 1, 24, true),
        ]
    );
}

/// A stair step that does not fit on its first `P_ChangeSector` (a barrel under a
/// low ceiling) refuses the tic with `StairBlocked` (decision H7).
#[test]
fn blocked_stair_step_is_stair_blocked() {
    let mut l = stair_room();
    set_heights(&mut l, 0, 0, 44);
    thing(&mut l, 200, 64, 0, 2035);
    thing(&mut l, 1000, 64, 0, 1);
    let mut s = sim(&l);
    assert!(s.start_movers(|m| m.build_stairs(42)).unwrap());
    for _ in 0..40 {
        let before = s.snapshot().canonical_bytes();
        match s.advance(zero()) {
            Ok(()) => continue,
            Err(e) => {
                assert_eq!(e, SimError::MapScale(MapScaleError::StairBlocked));
                assert_eq!(s.snapshot().canonical_bytes(), before);
                return;
            }
        }
    }
    panic!("the blocked stair step was not refused");
}

// ---- Donut -----------------------------------------------------------------

/// Flips line `line` to face the other way, keeping the geometry: its endpoints
/// and its two sidedefs swap, and every seg of it takes the other side.
fn flip_line(l: &mut Lumps, line: usize) {
    let v = lump(l, "LINEDEFS");
    let at = 14 * line;
    let (a, b) = (v[at..at + 2].to_vec(), v[at + 2..at + 4].to_vec());
    v[at..at + 2].copy_from_slice(&b);
    v[at + 2..at + 4].copy_from_slice(&a);
    let (f, k) = (v[at + 10..at + 12].to_vec(), v[at + 12..at + 14].to_vec());
    v[at + 10..at + 12].copy_from_slice(&k);
    v[at + 12..at + 14].copy_from_slice(&f);
    let segs = lump(l, "SEGS");
    for seg in segs.chunks_mut(12) {
        if usize::from(u16::from_le_bytes([seg[6], seg[7]])) == line {
            seg[8] ^= 1;
        }
    }
}
/// The lift room rearranged as a donut: `s1` = sector 3 (tag 9), whose first
/// line (5) leads to `s2` = sector 1; `s2`'s first line (0, flipped to face
/// sector 0) has back sector 0 = `s3` (floor 32, flat `NUKAGE1`). Line 12 is the
/// S1 9 switch.
fn donut_room() -> Lumps {
    let mut l = room(7);
    flip_line(&mut l, 0);
    set_line(&mut l, 0, 0, 0);
    set_heights(&mut l, 0, 32, 192);
    set_floor_flat(&mut l, 0, b"NUKAGE1\0");
    set_sector_special(&mut l, 1, 7, 0);
    set_sector_special(&mut l, 3, 0, 9);
    set_line(&mut l, 12, 9, 9);
    set_front_texture(&mut l, 12, b"SW1COMM\0");
    thing(&mut l, 256, 64, 90, 1);
    l
}

/// `EV_DoDonut`: the `s2` ring (`donutRaise`, +1, 1/2 unit, `texture` = `s3`'s
/// flat, `newspecial` 0) then the `s1` hole (`lowerFloor`, −1), both to `s3`'s
/// floor; at its destination the ring takes the flat and special 0. A busy `s2`
/// refuses with `DonutBusy` (decision H8).
#[test]
fn donut_raises_the_ring_then_lowers_the_hole() {
    let mut s = sim(&donut_room());
    assert!(s.start_movers(|m| m.do_donut(12)).unwrap());
    match movers(&s).as_slice() {
        [SectorThinker::Floor(ring), SectorThinker::Floor(hole)] => {
            assert_eq!(
                (ring.sector, ring.kind, ring.direction),
                (1, FloorKind::DonutRaise, 1)
            );
            assert_eq!(ring.texture, *b"NUKAGE1\0");
            assert_eq!(
                (ring.floordestheight, ring.speed),
                (units(32), Fixed::from_bits(32768))
            );
            assert_eq!(
                (hole.sector, hole.kind, hole.direction),
                (3, FloorKind::LowerFloor, -1)
            );
            assert_eq!(hole.floordestheight, units(32));
        }
        other => panic!("{other:?}"),
    }
    let fields: Vec<_> = s
        .snapshot()
        .fields()
        .map(|f| f.path().to_string())
        .collect();
    assert!(fields.iter().any(|p| p.ends_with(".texture")));
    assert!(fields.iter().any(|p| p.ends_with(".newspecial")));
    Diagnostic::decode_episode1(&s.snapshot().canonical_bytes(), DiagnosticLimits::default())
        .unwrap();
    settle(&mut s, 300);
    assert_eq!(special(&s, 1).floorpic, *b"NUKAGE1\0");
    assert_eq!(special(&s, 1).special, 0);
    assert_eq!((floor(&s, 1), floor(&s, 3)), (units(32), units(32)));
    // DonutBusy: s2 (sector 1) holds a mover when the donut starts.
    let mut s = sim(&donut_room());
    let busy = SectorThinker::Floor(movers::FloorMove {
        sector: 1,
        kind: FloorKind::LowerFloorToLowest,
        floordestheight: Fixed::ZERO,
        direction: -1,
        speed: units(1),
        texture: [0; 8],
        newspecial: 0,
        stair: false,
    });
    s.actors.spawn_sector(busy).unwrap();
    let before = s.snapshot().canonical_bytes();
    assert_eq!(
        s.start_movers(|m| m.do_donut(12)),
        Err(SimError::MapScale(MapScaleError::DonutBusy))
    );
    assert_eq!(s.snapshot().canonical_bytes(), before);
}

// ---- Light and exits -------------------------------------------------------

/// W1 35 crossed by the player: every sector with the tag takes light 35, no
/// thinker; the line is cleared.
#[test]
fn light_trigger_sets_light_35_on_the_tagged_sectors() {
    let mut l = room(13);
    set_sector_special(&mut l, 10, 0, 9);
    set_sector_special(&mut l, 7, 0, 9);
    set_line(&mut l, 42, 35, 9);
    thing(&mut l, 900, 250, 0, 1);
    let mut s = sim(&l);
    let id = player(&s);
    let crossing = line_triggers::Crossing {
        line: 42,
        side: 0,
        special: 35,
    };
    let after = s
        .start_movers(|m| line_triggers::cross(m, id, line_triggers::Crosser::Player, crossing))
        .unwrap();
    assert_eq!(after, 0);
    assert_eq!((special(&s, 10).light, special(&s, 7).light), (35, 35));
    assert!(movers(&s).is_empty());
}

/// S1 51 used on a full tic: the switch clears, `level.secret_exit` and
/// `level.exit` are set, and the next tic is refused with `LevelExited`.
#[test]
fn secret_exit_sets_secret_exit_and_ends_the_level() {
    let mut l = room(6);
    set_line(&mut l, 10, 51, 0);
    // Room 6's exit switch (line 10) faces the player placed in front of it.
    let v = l.line_vertices(10);
    let (a, b) = (l.vertex(v[0]), l.vertex(v[1]));
    let mid = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
    let d = [b[0] - a[0], b[1] - a[1]];
    // Front (right) side normal of an axis-aligned line, 32 units out.
    let n = [d[1].signum() * 32, -d[0].signum() * 32];
    let angle = match (n[0].signum(), n[1].signum()) {
        (1, _) => 180,
        (-1, _) => 0,
        (_, 1) => 270,
        _ => 90,
    };
    thing(&mut l, mid[0] + n[0], mid[1] + n[1], angle, 1);
    let mut s = sim(&l);
    use_once(&mut s).unwrap();
    assert!(s.level_exit && s.level_secret_exit);
    assert_eq!(s.line_specials[10], 0);
    let fields: Vec<_> = s
        .snapshot()
        .fields()
        .map(|f| (f.path().to_string(), f.value().to_vec()))
        .collect();
    assert!(
        fields
            .iter()
            .any(|(p, v)| p == "level.secret_exit" && v == &[1])
    );
    Diagnostic::decode_episode1(&s.snapshot().canonical_bytes(), DiagnosticLimits::default())
        .unwrap();
    assert_eq!(s.advance(zero()), Err(SimError::LevelExited));
}
