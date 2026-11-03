//! Use, switches, the exit and the reborn request (M3h8 plan step 10). Expected
//! values come from the C rows of the committed scenarios (through the partial
//! replay) and the source reading of the M3h8 note §2.1, §2.3 and §2.8, never from
//! Rust output.
use super::*;
use crate::movers_replay::{Compared, case, command, replay};
use crate::*;

fn sfx(name: &str) -> u16 {
    crate::lifecycle::source_sound_id(name)
}
fn run(label: &str) -> (Simulation, Compared) {
    let c = case(label);
    replay(&c, c.tics)
}

/// Every use scenario that needs no monster door use matches the C rows at every op:
/// sector heights and `specialdata`, line specials, crossings, every
/// `P_UseSpecialLine` and `PTR_UseTraverse` record, `usedown`, the player state and
/// the exit.
#[test]
fn use_scenarios_match_the_c_rows() {
    for (label, uses) in [
        // A DR use opens, the door waits and closes; reused while rising, waiting,
        // closing; reversed onto the player.
        ("door_use_wait_close", 2),
        ("door_reuse_closes", 6),
        ("door_reverse_on_player", 2),
        ("door_blocks_sight_and_sound", 2),
        // SR 63 three times (the second while its door is active: nothing).
        ("switch_sr_door", 6),
        ("switch_exit", 2),
        ("switch_exit_after_button", 4),
        ("use_held_latch", 2),
        ("use_noway_wall", 1),
        ("use_noway_closed_door", 1),
        ("use_back_side", 2),
        ("dead_use_reborn", 0),
        ("blaze_reversed", 4),
        ("use_walkover_line", 2),
    ] {
        let (_, compared) = run(label);
        assert_eq!(compared.uses, uses, "{label}");
        assert_eq!(compared.ops, case(label).tics + 1, "{label}");
    }
}
/// `door_gibs_drop_bonus` and `door_stuck_shootable` match the C rows until the
/// first tic whose end state needs V′ (a gibbed corpse pushed below its floor, case
/// (e); a shootable thing stuck under a closed door, case (d)), which the movers
/// policy admits only from plan step 12: until then that tic is refused with the
/// M3h4 invariant.
#[test]
fn door_scenarios_match_the_c_rows_until_v_prime_is_needed() {
    for (label, refused, uses) in [
        ("door_gibs_drop_bonus", 216, 2),
        ("door_stuck_shootable", 6, 2),
    ] {
        let c = case(label);
        let (mut s, compared) = replay(&c, refused - 1);
        assert_eq!((compared.ops, compared.uses), (refused, uses), "{label}");
        assert_eq!(
            s.advance(command(&c, refused)),
            Err(SimError::MapScale(MapScaleError::Vertical)),
            "{label}"
        );
    }
}

/// `use_held_latch`: `BT_USE` held from tic 1 (usedown starts true, G_PlayerReborn)
/// does nothing until released; the use at 40 opens the SR door.
#[test]
fn a_use_held_from_level_start_waits_for_its_release() {
    let c = case("use_held_latch");
    let (s, _) = replay(&c, 30);
    assert!(s.original_player.as_ref().unwrap().use_down);
    assert!(
        s.actors
            .sector_slots
            .iter()
            .all(|slot| slot.entry.is_none())
    );
}

/// The exit (S1 11): the switch clears the line before the exit-sound test, so the
/// sound is `sfx_swtchn`, global (no button active); `G_ExitLevel` follows the sound
/// and precedes the use record; the tic completes, and the next advance is refused
/// with every byte preserved. `switch_exit_after_button`: with the SR button in slot
/// 0, the exit switch's sound plays at that button's sector.
#[test]
fn the_exit_completes_its_tic_and_refuses_the_next() {
    let c = case("switch_exit");
    let (mut s, _) = replay(&c, c.tics);
    assert!(s.level_exit);
    let events = s.events().to_vec();
    let at = |f: &dyn Fn(&Event) -> bool| events.iter().position(f).unwrap();
    let sound = at(&|e| {
        matches!(e, Event::ActorState(lifecycle::ActorStateEvent::Sound { actor: None, sound })
            if *sound == sfx("sfx_swtchn"))
    });
    let exit = at(&|e| matches!(e, Event::Mover(MoverEvent::Exit { secret: false })));
    let used = at(&|e| {
        matches!(
            e,
            Event::Mover(MoverEvent::SpecialUse {
                before: 11,
                after: 0,
                result: true,
                ..
            })
        )
    });
    assert!(sound < exit && exit < used);
    let snapshot = s.snapshot();
    let tic = s.tic();
    assert_eq!(s.advance(command(&c, c.tics)), Err(SimError::LevelExited));
    assert_eq!(s.step(command(&c, c.tics)), Err(SimError::LevelExited));
    assert_eq!((s.snapshot(), s.tic()), (snapshot, tic));

    let (s, _) = run("switch_exit_after_button");
    assert!(s.level_exit);
    assert!(s.events().iter().any(|e| matches!(
        e,
        Event::Mover(MoverEvent::SectorSound {
            origin: SoundOrigin::Sector,
            sound,
            ..
        }) if *sound == sfx("sfx_swtchn")
    )));
}

/// `BT_USE` while dead (N3): `P_DeathThink` sets `PST_REBORN` (encoded as the
/// player state 2), the tic completes with the reborn record last in the players
/// phase, and the next advance is refused with every byte preserved.
#[test]
fn a_dead_use_requests_a_reborn_and_refuses_the_next_tic() {
    let c = case("dead_use_reborn");
    let (mut s, _) = replay(&c, c.tics);
    let p = s.original_player.as_ref().unwrap();
    assert!(p.reborn && p.state == PlayerBodyState::Dead);
    let events = s.events();
    let reborn = events
        .iter()
        .position(|e| matches!(e, Event::Mover(MoverEvent::Reborn { slot: 0 })))
        .unwrap();
    assert!(matches!(events[reborn + 1], Event::Phase(Phase::Thinkers)));
    let state = s
        .snapshot()
        .fields()
        .find(|f| f.path() == "original_player.0.state")
        .unwrap()
        .value()
        .to_vec();
    assert_eq!(state, [2]);
    let snapshot = s.snapshot();
    assert_eq!(s.advance(command(&c, c.tics)), Err(SimError::PlayerReborn));
    assert_eq!(s.snapshot(), snapshot);
}

/// SR 63 while its door is active creates nothing: no switch change, no sound, no
/// button; a reusable switch's first use plays its sound globally, then starts its
/// button in slot 0, which counts down 35 tics and restores with `sfx_swtchn` at the
/// address of slot 0's sound origin.
#[test]
fn sr_switch_buttons_follow_the_source() {
    let c = case("switch_sr_door");
    let (s, _) = replay(&c, 3);
    let button = s.buttons[0].unwrap();
    // P_UpdateSpecials counted it down once at the end of the same tic.
    assert_eq!((button.line, button.timer), (1, 34));
    assert!(s.events().iter().any(|e| matches!(
        e,
        Event::ActorState(lifecycle::ActorStateEvent::Sound { actor: None, sound })
            if *sound == sfx("sfx_swtchn")
    )));
    // The button finishes at the end of op 37 (35 P_UpdateSpecials countdowns).
    let (s, _) = replay(&c, 36);
    assert_eq!(s.buttons[0].map(|b| b.timer), Some(1));
    let (s, _) = replay(&c, 37);
    assert_eq!(s.buttons[0], None);
    let specials = s
        .events()
        .iter()
        .position(|e| *e == Event::Phase(Phase::Specials))
        .unwrap();
    assert_eq!(
        s.events()[specials + 1],
        Event::Mover(MoverEvent::SectorSound {
            origin: SoundOrigin::Button,
            index: 0,
            sound: sfx("sfx_swtchn"),
        })
    );
    // The use at 60 happens while the door of the use at 3 is still active (it
    // waits 150 tics at the top): no door, no sound, no button.
    let (s, _) = replay(&c, 60);
    assert_eq!(s.buttons, [None; crate::mover_rules::MAX_BUTTONS]);
    assert!(!s.events().iter().any(|e| matches!(
        e,
        Event::ActorState(lifecycle::ActorStateEvent::Sound { actor: None, .. })
            | Event::Mover(MoverEvent::SectorSound { .. })
    )));
}

/// A button is never started twice for one line, and the switch sound uses slot 0's
/// origin whatever line it belongs to.
#[test]
fn buttons_are_deduplicated_per_line() {
    let s = crate::player_movers_tests::movers(&crate::player_movers_tests::room(6)).unwrap();
    let world = &s.world;
    let mut buttons: Buttons = [None; MAX_BUTTONS];
    let mut events = Vec::new();
    change_switch_texture(world, &mut events, &mut buttons, 1, 63, true).unwrap();
    change_switch_texture(world, &mut events, &mut buttons, 1, 63, true).unwrap();
    assert_eq!(buttons.iter().flatten().count(), 1);
    let sector = buttons[0].unwrap().sector;
    assert!(matches!(
        events.as_slice(),
        [
            Event::ActorState(lifecycle::ActorStateEvent::Sound { actor: None, .. }),
            Event::Mover(MoverEvent::SectorSound { origin: SoundOrigin::Sector, index, .. }),
        ] if *index == sector
    ));
    // A non-reusable switch clears its special before the exit-sound test.
    events.clear();
    let after = change_switch_texture(world, &mut events, &mut buttons, 10, 11, false).unwrap();
    assert_eq!(after, 0);
    assert!(matches!(
        events.as_slice(),
        [Event::Mover(MoverEvent::SectorSound { sound, .. })] if *sound == sfx("sfx_swtchn")
    ));
}

/// Older policies keep rejecting `BT_USE` as an unsupported button.
#[test]
fn the_specials_policy_still_rejects_use() {
    let c = crate::specials_evidence::cases().remove(0);
    let (mut s, _, _) = crate::specials_evidence::setup(&c);
    let mut commands = [TicCommand::default(); 4];
    commands[0].buttons = BT_USE;
    assert_eq!(s.advance(commands), Err(SimError::UnsupportedButtons));
}

/// Monster door use (plan step 11), against the C rows at every op: every `P_Move`
/// failure branch with `spechit` (`MP`, its count), every monster `P_UseSpecialLine`
/// (`MU`), and the doors, heights and crossings that follow.
/// `door_monster_back_side`: a monster inside the door sector presses against the DR
/// line from its back and opens it (side 0 is always passed), then keeps pressing
/// against the waiting door (each press returns true, so P_TryWalk draws movecount).
/// `door_monster_bump`: a monster opens a closed door from its front.
/// `lift_monster_cross`: failed moves against the WR 88 lines reset `movedir` though
/// the use returns false. `dense_imp_fireballs`: one such reset in the dense lane.
#[test]
fn monster_door_use_matches_the_c_rows() {
    for (label, blocked) in [
        ("door_monster_back_side", 110),
        ("door_monster_bump", 10),
        ("lift_monster_cross", 73),
        ("dense_imp_fireballs", 1),
    ] {
        let (_, compared) = run(label);
        assert_eq!(compared.blocked, blocked, "{label}");
        assert_eq!(compared.ops, case(label).tics + 1, "{label}");
    }
}

/// `EV_VerticalDoor` by a monster: a closed door opens; a rising or waiting door is
/// left alone ("bad guys never close doors"); a closing door goes back up with no
/// sound. Every DR use by a monster returns true; an 88 use returns false; a
/// back-side use returns false.
#[test]
fn a_monster_uses_only_dr_doors_and_never_closes_them() {
    let (mut s, _, player) = crate::planes::tests::setup(0, [-200, 0]);
    let use_door = |s: &mut Simulation, side: u8, special: i16| {
        s.start_movers(|m| {
            use_special_line(
                m,
                None,
                player,
                User::Monster,
                4,
                side,
                special,
                &mut UseOutcome::default(),
            )
        })
        .unwrap()
    };
    // `doorway` line 4 is a DR line on door sector 2.
    assert!(use_door(&mut s, 0, 1));
    let door = |s: &Simulation| match movers::special_data(&s.actors, 2)
        .and_then(|id| s.sector_thinker(id).copied())
    {
        Some(SectorThinker::Door(d)) => d,
        other => panic!("{other:?}"),
    };
    assert_eq!(door(&s).direction, 1);
    // Rising: a monster's use leaves it rising.
    assert!(use_door(&mut s, 0, 1));
    assert_eq!(door(&s).direction, 1);
    // Closing: a monster's use sends it up with no sound.
    s.start_movers(|m| {
        if let Some(id) = movers::special_data(m.store, 2) {
            if let Some(SectorThinker::Door(d)) = m.store.sector_get_mut(id) {
                d.direction = -1;
            }
        }
        Ok(())
    })
    .unwrap();
    s.pending.clear();
    assert!(use_door(&mut s, 0, 1));
    assert_eq!(door(&s).direction, 1);
    assert!(
        !s.pending
            .iter()
            .any(|e| matches!(e, Event::Mover(MoverEvent::SectorSound { .. })))
    );
    // Waiting: left waiting.
    s.start_movers(|m| {
        if let Some(id) = movers::special_data(m.store, 2) {
            if let Some(SectorThinker::Door(d)) = m.store.sector_get_mut(id) {
                d.direction = 0;
            }
        }
        Ok(())
    })
    .unwrap();
    assert!(use_door(&mut s, 0, 1));
    assert_eq!(door(&s).direction, 0);
    // Back side, and a special a monster cannot use.
    assert!(!use_door(&mut s, 1, 1));
    assert!(!use_door(&mut s, 0, 88));
}
