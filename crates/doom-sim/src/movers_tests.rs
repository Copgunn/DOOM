//! Rust-only tests of the doors, lifts and floors on the committed M3h8 rooms,
//! started directly through their `EV_*` bodies (use and crossing are later plan
//! steps). Expected values come from the original source reading (M3h8 note
//! §2.4–§2.6) and the C facts pinned by the evidence, never from Rust output.
use super::*;
use crate::planes::tests::setup;
use crate::*;

fn units(v: i16) -> Fixed {
    Fixed::from_map_units(v)
}
fn idle(s: &mut Simulation) -> Vec<Event> {
    s.advance([TicCommand::default(); 4]).unwrap();
    s.events.clone()
}
/// The sector-thinker entries of the shared order with their records and whether
/// they are marked removed.
fn movers(s: &Simulation) -> Vec<(SectorThinkerId, SectorThinker, bool)> {
    s.shared_thinker_order()
        .filter_map(ThinkerId::sector)
        .map(|id| {
            (
                id,
                *s.sector_thinker(id).unwrap(),
                s.actors.sector_is_pending(id).unwrap(),
            )
        })
        .collect()
}
fn door(s: &Simulation) -> Door {
    match movers(s).as_slice() {
        [(_, SectorThinker::Door(d), false)] => *d,
        other => panic!("{other:?}"),
    }
}
fn mover_events(events: &[Event]) -> Vec<MoverEvent> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Mover(m) => Some(*m),
            _ => None,
        })
        .collect()
}
fn sounds(events: &[Event]) -> Vec<u16> {
    mover_events(events)
        .into_iter()
        .filter_map(|e| match e {
            MoverEvent::SectorSound { sound, .. } => Some(sound),
            _ => None,
        })
        .collect()
}
fn sfx(name: &str) -> u16 {
    lifecycle::source_sound_id(name)
}
fn steps(events: &[Event]) -> Vec<(u16, MoveResult, Fixed)> {
    mover_events(events)
        .into_iter()
        .filter_map(|e| match e {
            MoverEvent::PlaneStep {
                sector,
                result,
                after,
                ..
            } => Some((sector, result, after)),
            _ => None,
        })
        .collect()
}

/// `door_use_wait_close` (room 0, DR line 4, door sector 2 closed at 0): the door
/// starts with `sfx_doropn` (EV_VerticalDoor plays it unconditionally), rises 2 per
/// tic to 124 (the neighbours' 128 − 4), waits 150 tics, turns down with
/// `sfx_dorcls`, is removed at its floor and unlinked by the next pass, which
/// grows its slot's generation.
#[test]
fn dr_door_opens_waits_closes_and_leaves_its_slot() {
    let (mut s, _, _) = setup(0, [-200, 0]);
    s.start_movers(|m| m.vertical_door(4, true)).unwrap();
    let start = mover_events(&s.pending);
    assert!(matches!(
        start.as_slice(),
        [
            MoverEvent::SectorSound { origin: SoundOrigin::Sector, index: 2, sound },
            MoverEvent::Started { kind: 5, sector: 2, .. },
            MoverEvent::Call {
                function: MoverFunction::VerticalDoor,
                line: 4,
                argument: 1,
                result: -1,
                created: 1,
            },
        ] if *sound == sfx("sfx_doropn")
    ));
    let (id, ..) = movers(&s)[0];
    assert_eq!((id.slot(), id.generation()), (0, 1));
    // The mover is the tail of the shared order.
    assert_eq!(s.shared_thinker_order().last(), Some(ThinkerId::Sector(id)));
    assert_eq!(
        door(&s),
        Door {
            sector: 2,
            kind: DoorKind::Normal,
            direction: 1,
            topheight: units(124),
            topcountdown: 0,
        }
    );
    // Created before the pass, it steps in the first tic.
    for tic in 1..=62 {
        let events = idle(&mut s);
        assert_eq!(steps(&events), vec![(2, MoveResult::Ok, units(2 * tic))]);
    }
    let events = idle(&mut s);
    assert_eq!(steps(&events), vec![(2, MoveResult::PastDest, units(124))]);
    assert_eq!((door(&s).direction, door(&s).topcountdown), (0, 150));
    for _ in 1..150 {
        let events = idle(&mut s);
        assert!(steps(&events).is_empty() && sounds(&events).is_empty());
    }
    let events = idle(&mut s);
    assert_eq!(sounds(&events), vec![sfx("sfx_dorcls")]);
    assert_eq!((door(&s).direction, door(&s).topcountdown), (-1, 0));
    for tic in 1..=62 {
        let events = idle(&mut s);
        assert_eq!(
            steps(&events),
            vec![(2, MoveResult::Ok, units(124 - 2 * tic))]
        );
    }
    let events = idle(&mut s);
    assert_eq!(steps(&events), vec![(2, MoveResult::PastDest, Fixed::ZERO)]);
    assert!(mover_events(&events).contains(&MoverEvent::Removed { mover: id }));
    // Removed but not unlinked: the slot and the order entry stay, specialdata is
    // already clear.
    assert!(matches!(
        movers(&s).as_slice(),
        [(_, SectorThinker::Door(_), true)]
    ));
    assert_eq!(special_data(&s.actors, 2), None);
    let events = idle(&mut s);
    assert!(mover_events(&events).contains(&MoverEvent::Unlinked { mover: id }));
    assert!(movers(&s).is_empty());
    assert_eq!(s.actors.sector_slots[0].generation, 2);
    assert_eq!(s.sectors[2].ceiling, Fixed::ZERO);
}

/// EV_VerticalDoor on a live door: a player's use of a rising or waiting door sends
/// it down at once, a monster's does nothing, and anyone's use of a closing door
/// sends it back up, all without a sound or a new door.
#[test]
fn dr_use_reverses_a_live_door_by_user() {
    let (mut s, _, _) = setup(0, [-200, 0]);
    s.start_movers(|m| m.vertical_door(4, true)).unwrap();
    for _ in 0..10 {
        idle(&mut s);
    }
    s.pending.clear();
    // Monster use of the rising door: nothing.
    s.start_movers(|m| m.vertical_door(15, false)).unwrap();
    assert_eq!(door(&s).direction, 1);
    // Player use of the rising door: down immediately.
    s.start_movers(|m| m.vertical_door(15, true)).unwrap();
    assert_eq!(door(&s).direction, -1);
    // Monster use of the closing door: back up.
    s.start_movers(|m| m.vertical_door(4, false)).unwrap();
    assert_eq!(door(&s).direction, 1);
    assert!(sounds(&s.pending).is_empty());
    assert!(mover_events(&s.pending).iter().all(|e| matches!(
        e,
        MoverEvent::Call {
            created: 0,
            result: -1,
            ..
        }
    )));
    assert_eq!(movers(&s).len(), 1);
}

/// `switch_sr_door` (room 6, SR line 1, tag 1 → sector 4 closed at 0): EV_DoDoor
/// creates a normal door with `sfx_doropn` (its top differs from the ceiling) and
/// returns 1; a second call while it is live creates nothing and returns 0.
#[test]
fn sr_door_starts_once_per_live_door() {
    let (mut s, _, _) = setup(6, [64, 330]);
    assert!(s.start_movers(|m| m.do_door(1, DoorKind::Normal)).unwrap());
    assert_eq!(sounds(&s.pending), vec![sfx("sfx_doropn")]);
    assert_eq!(door(&s).topheight, units(124));
    assert_eq!(door(&s).sector, 4);
    s.pending.clear();
    assert!(!s.start_movers(|m| m.do_door(1, DoorKind::Normal)).unwrap());
    assert!(matches!(
        mover_events(&s.pending).as_slice(),
        [MoverEvent::Call {
            function: MoverFunction::DoDoor,
            result: 0,
            created: 0,
            ..
        }]
    ));
}

/// `blaze_open` and `blaze_reversed` (room 13, W1 109 line 20 and DR line 22 on door
/// sector 11 at 0/0, top 124): the blazeOpen door rises 8 per tic with `sfx_bdopn`;
/// a player's DR use sends it down; at the floor it stays a live thinker reporting
/// `pastdest` every tic; a DR use sends it up again and it is removed at the top.
#[test]
fn blaze_open_door_reversed_by_a_dr_use_waits_at_its_floor() {
    let (mut s, _, _) = setup(13, [1100, 64]);
    assert!(
        s.start_movers(|m| m.do_door(20, DoorKind::BlazeOpen))
            .unwrap()
    );
    assert_eq!(sounds(&s.pending), vec![sfx("sfx_bdopn")]);
    for tic in 1..=3 {
        let events = idle(&mut s);
        assert_eq!(steps(&events), vec![(11, MoveResult::Ok, units(8 * tic))]);
    }
    s.start_movers(|m| m.vertical_door(22, true)).unwrap();
    assert_eq!(door(&s).direction, -1);
    // 24 → 16 → 8 → 0 are ordinary steps (0 − 8 < 0 only from 0 on).
    for tic in 1..=3 {
        let events = idle(&mut s);
        assert_eq!(
            steps(&events),
            vec![(11, MoveResult::Ok, units(24 - 8 * tic))]
        );
    }
    for _ in 0..5 {
        let events = idle(&mut s);
        assert_eq!(
            steps(&events),
            vec![(11, MoveResult::PastDest, Fixed::ZERO)]
        );
        assert_eq!(door(&s).kind, DoorKind::BlazeOpen);
    }
    s.start_movers(|m| m.vertical_door(22, true)).unwrap();
    assert_eq!(door(&s).direction, 1);
    for tic in 1..=15 {
        let events = idle(&mut s);
        assert_eq!(steps(&events), vec![(11, MoveResult::Ok, units(8 * tic))]);
    }
    let events = idle(&mut s);
    assert_eq!(steps(&events), vec![(11, MoveResult::PastDest, units(124))]);
    assert!(matches!(
        movers(&s).as_slice(),
        [(_, SectorThinker::Door(_), true)]
    ));
    assert!(sounds(&events).is_empty());
}

/// `floor_lower_to_lowest` and `floor_turbo_lower` (room 13): the 38 target 9 goes
/// from 48 to its lowest neighbour 0 at 1 per tic; the 36 target 7 goes from 128 to
/// its highest neighbour 64 plus 8 at 4 per tic. `sfx_stnmov` plays when
/// `leveltime & 7 == 0`; at `pastdest` the floor is removed, then `sfx_pstop`.
#[test]
fn floors_lower_with_stone_sounds_and_stop() {
    let (mut s, _, _) = setup(13, [1100, 64]);
    assert!(
        s.start_movers(|m| m.do_floor(16, FloorKind::LowerFloorToLowest))
            .unwrap()
    );
    assert!(
        s.start_movers(|m| m.do_floor(11, FloorKind::TurboLower))
            .unwrap()
    );
    let records: Vec<_> = movers(&s).into_iter().map(|m| m.1).collect();
    assert_eq!(
        records,
        vec![
            SectorThinker::Floor(FloorMove {
                sector: 9,
                kind: FloorKind::LowerFloorToLowest,
                floordestheight: Fixed::ZERO,
            }),
            SectorThinker::Floor(FloorMove {
                sector: 7,
                kind: FloorKind::TurboLower,
                floordestheight: units(72),
            }),
        ]
    );
    // No sound at the start.
    assert!(sounds(&s.pending).is_empty());
    let mut turbo_done = None;
    for tic in 1..=49u64 {
        let leveltime = s.tic;
        let events = idle(&mut s);
        let st = steps(&events);
        assert_eq!(st[0].0, 9);
        assert_eq!(st[0].2, units(48 - tic.min(48) as i16));
        let stnmov = sounds(&events)
            .iter()
            .filter(|v| **v == sfx("sfx_stnmov"))
            .count();
        let live = 1 + usize::from(turbo_done.is_none());
        assert_eq!(stnmov, if leveltime & 7 == 0 { live } else { 0 });
        if st.iter().any(|v| v.0 == 7 && v.1 == MoveResult::PastDest) {
            turbo_done = Some(tic);
            assert_eq!(s.sectors[7].floor, units(72));
            assert!(sounds(&events).ends_with(&[sfx("sfx_pstop")]));
        }
        if tic == 49 {
            assert_eq!(st[0].1, MoveResult::PastDest);
            assert!(
                mover_events(&events)
                    .iter()
                    .any(|e| matches!(e, MoverEvent::Removed { .. }))
            );
        }
    }
    // 128 → 72 in 14 ordinary steps; the 15th returns pastdest.
    assert_eq!(turbo_done, Some(15));
    assert_eq!(s.sectors[9].floor, Fixed::ZERO);
}

/// `lift_player` (room 7, WR 88 line 0, tag 1 → sector 3 at floor 64): the lift
/// goes down at 4 per tic to its lowest neighbour (`sfx_pstart` at the start,
/// `sfx_pstop` on arrival), waits 105 tics, goes up with `sfx_pstart` to `high`
/// (its floor at activation) and is removed there with `sfx_pstop`.
#[test]
fn lift_lowers_waits_rises_and_is_removed() {
    let (mut s, _, _) = setup(7, [64, 600]);
    assert!(s.start_movers(|m| m.do_plat(0)).unwrap());
    assert_eq!(sounds(&s.pending), vec![sfx("sfx_pstart")]);
    let SectorThinker::Plat(plat) = movers(&s)[0].1 else {
        panic!()
    };
    assert_eq!(plat.high, units(64));
    assert_eq!(plat.status, PlatStatus::Down);
    assert!(plat.low < units(64));
    assert_eq!(active_plats(&s.actors), 1);
    let low = plat.low;
    let mut tics = 0;
    loop {
        tics += 1;
        let events = idle(&mut s);
        if steps(&events)[0].1 == MoveResult::PastDest {
            assert_eq!(sounds(&events), vec![sfx("sfx_pstop")]);
            break;
        }
        assert!(tics < 64);
    }
    assert_eq!(s.sectors[3].floor, low);
    let SectorThinker::Plat(plat) = movers(&s)[0].1 else {
        panic!()
    };
    assert_eq!((plat.status, plat.count), (PlatStatus::Waiting, 105));
    for _ in 1..105 {
        assert!(sounds(&idle(&mut s)).is_empty());
    }
    let events = idle(&mut s);
    assert_eq!(sounds(&events), vec![sfx("sfx_pstart")]);
    loop {
        let events = idle(&mut s);
        let st = steps(&events);
        if st[0].1 == MoveResult::PastDest {
            assert_eq!(st[0].2, units(64));
            assert_eq!(sounds(&events), vec![sfx("sfx_pstop")]);
            let removed = mover_events(&events);
            let pstop = removed
                .iter()
                .position(|e| matches!(e, MoverEvent::SectorSound { .. }))
                .unwrap();
            let gone = removed
                .iter()
                .position(|e| matches!(e, MoverEvent::Removed { .. }))
                .unwrap();
            // T_PlatRaise plays sfx_pstop before P_RemoveActivePlat.
            assert!(pstop < gone);
            break;
        }
    }
    assert_eq!(active_plats(&s.actors), 0);
}

/// A sector whose mover removed itself can start a new mover before the old entry
/// is unlinked: the new one takes the lowest free slot (not the occupied one), the
/// sector then has two entries, and the next pass unlinks the old one first.
#[test]
fn replacement_mover_joins_beside_the_removed_entry() {
    let (mut s, _, _) = setup(13, [1100, 64]);
    assert!(
        s.start_movers(|m| m.do_door(20, DoorKind::BlazeOpen))
            .unwrap()
    );
    for _ in 0..16 {
        idle(&mut s);
    }
    let old = movers(&s);
    assert!(matches!(
        old.as_slice(),
        [(_, SectorThinker::Door(_), true)]
    ));
    s.start_movers(|m| m.vertical_door(22, true)).unwrap();
    let both = movers(&s);
    assert_eq!(both.len(), 2);
    assert_eq!(both[0].0, old[0].0);
    assert_eq!((both[1].0.slot(), both[1].0.generation()), (1, 1));
    assert!(both.iter().all(|m| m.1.mover_sector() == Some(11)));
    assert_eq!(special_data(&s.actors, 11), Some(both[1].0));
    let events = idle(&mut s);
    let order: Vec<_> = mover_events(&events)
        .into_iter()
        .filter(|e| {
            matches!(
                e,
                MoverEvent::Unlinked { .. } | MoverEvent::PlaneStep { .. }
            )
        })
        .collect();
    assert!(matches!(
        order.as_slice(),
        [MoverEvent::Unlinked { mover }, MoverEvent::PlaneStep { sector: 11, .. }]
            if *mover == old[0].0
    ));
    // The freed slot 0 is the next mover's, with a grown generation.
    assert!(
        s.start_movers(|m| m.do_floor(16, FloorKind::LowerFloorToLowest))
            .unwrap()
    );
    let ids: Vec<_> = movers(&s)
        .into_iter()
        .map(|m| (m.0.slot(), m.0.generation()))
        .collect();
    assert_eq!(ids, vec![(1, 1), (0, 2)]);
}

/// `door_reverse_on_player` with the pinned EV_VerticalDoor quirk (room 0, DR line 6,
/// door sector 4 already open at 124): the use plays `sfx_doropn` although the door
/// is at its top, the first step returns `pastdest` and the door waits 150 tics;
/// closing onto the player standing in the door, the step below 56 is refused
/// (`crushed`) and the door goes back up with `sfx_doropn`. `step` equals `advance`
/// on every tic.
#[test]
fn closing_door_reopens_on_the_player_and_step_equals_advance() {
    let (mut s, _, player) = setup(0, [32, -192]);
    assert_eq!(s.actor(player).unwrap().location().unwrap().sector, 4);
    s.start_movers(|m| m.vertical_door(6, true)).unwrap();
    assert_eq!(sounds(&s.pending), vec![sfx("sfx_doropn")]);
    let tic = |s: &mut Simulation| {
        let mut stepped = s.clone();
        let snapshot = stepped.step([TicCommand::default(); 4]).unwrap();
        let events = idle(s);
        assert_eq!(s.snapshot(), snapshot);
        events
    };
    let events = tic(&mut s);
    assert_eq!(steps(&events), vec![(4, MoveResult::PastDest, units(124))]);
    assert_eq!((door(&s).direction, door(&s).topcountdown), (0, 150));
    for _ in 0..150 {
        tic(&mut s);
    }
    assert_eq!(door(&s).direction, -1);
    for n in 1..=34 {
        let events = tic(&mut s);
        assert_eq!(
            steps(&events),
            vec![(4, MoveResult::Ok, units(124 - 2 * n))]
        );
    }
    let events = tic(&mut s);
    assert_eq!(steps(&events), vec![(4, MoveResult::Crushed, units(56))]);
    assert_eq!(sounds(&events), vec![sfx("sfx_doropn")]);
    assert_eq!(door(&s).direction, 1);
    let events = tic(&mut s);
    assert_eq!(steps(&events), vec![(4, MoveResult::Ok, units(58))]);
    let a = s.actor(player).unwrap();
    assert_eq!(
        (a.z, a.floor, a.ceiling),
        (Fixed::ZERO, Fixed::ZERO, units(58))
    );
}
