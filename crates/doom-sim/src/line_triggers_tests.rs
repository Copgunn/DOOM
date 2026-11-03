//! `spechit`, the crossing loop and `P_CrossSpecialLine` (M3h8 plan step 9). The
//! expected values come from the C rows of the committed scenarios (through the
//! partial replay) and the source reading of the M3h8 note §2.2 and §6, never from
//! Rust output.
use super::*;
use crate::movers_replay::{case, replay};
use crate::*;

/// Replays a crossing scenario through its last op and returns the comparison.
fn crossing_case(label: &str) -> crate::movers_replay::Compared {
    let c = case(label);
    let (_, compared) = replay(&c, c.tics);
    compared
}

/// `floor_turbo_jump_up`, `floor_turbo_lower`, `floor_lower_to_lowest` and
/// `blaze_open`: a player crossing a W1 line starts its mover inside the pass (it
/// steps later in the same pass), the line reads 0 from that op on, and every sector
/// height, `specialdata` and line special matches the C rows at every op.
#[test]
fn player_crossings_of_w1_lines_match_the_c_rows() {
    for (label, crossings) in [
        ("floor_turbo_jump_up", 1),
        ("floor_turbo_lower", 1),
        ("floor_lower_to_lowest", 1),
        ("blaze_open", 1),
    ] {
        let compared = crossing_case(label);
        assert_eq!(compared.crossings, crossings, "{label}");
        assert_eq!(compared.ops, case(label).tics + 1, "{label}");
    }
}

/// The WR 88 lift crossed by the player (`lift_player`: twice, the second after the
/// lift finished; `lift_retrigger_beside_removed`: the second while the removed
/// entry is not yet unlinked; `lift_last_step_short` and `lift_reverse_at_top`) and
/// by a monster (`lift_monster_cross`): the line keeps its special.
#[test]
fn lift_crossings_by_players_and_monsters_match_the_c_rows() {
    for (label, crossings) in [
        ("lift_player", 2),
        ("lift_retrigger_beside_removed", 2),
        ("lift_monster_rider", 1),
        ("lift_monster_cross", 1),
        ("lift_last_step_short", 3),
        ("lift_reverse_at_top", 1),
    ] {
        assert_eq!(crossing_case(label).crossings, crossings, "{label}");
    }
}

/// `dense_spechit_8`: eight WR 88 lines in one window; the player's moves cross
/// them in the reverse `spechit` order of each `P_TryMove` (two in op 12: line 5,
/// then line 1). `dense_imp_fireballs`: an imp's fireballs fill `spechit` and cross
/// lines with their special unchanged (`P_CrossSpecialLine` returns at once for
/// `MT_TROOPSHOT`), then the imp crosses them and lowers the lift.
#[test]
fn dense_window_and_fireball_crossings_match_the_c_rows() {
    assert_eq!(crossing_case("dense_spechit_8").crossings, 8);
    assert_eq!(crossing_case("dense_imp_fireballs").crossings, 18);
}

/// `P_CrossSpecialLine` by crosser and special: the player clears 36, 38 and 109
/// whatever their `EV_*` did and keeps 88; a monster clears nothing; 1, 63 and 11
/// have no crossing case.
#[test]
fn only_a_player_clears_the_w1_lines() {
    use crate::mover_rules::*;
    for special in [LINE_FLOOR_TURBO, LINE_FLOOR_LOWEST, LINE_DOOR_BLAZE] {
        assert!(clears(Crosser::Player, special));
        assert!(!clears(Crosser::Monster, special));
    }
    for special in [LINE_LIFT, LINE_DOOR_DR, LINE_DOOR_SR, LINE_EXIT, 48] {
        assert!(!clears(Crosser::Player, special));
        assert!(!clears(Crosser::Monster, special));
    }
}

/// A monster crossing a W1 floor line does nothing and keeps it, while a monster
/// crossing 88 lowers the lift (p_spec.c:520–534); a W1 line crossed by a player is
/// cleared even when its `EV_*` created nothing (its sector already moving).
#[test]
fn cross_special_line_by_crosser() {
    let (mut s, _, player) = crate::planes::tests::setup(13, [200, 64]);
    let monster = s.actors.order().find(|id| *id != player);
    let crossing = |line, special| Crossing {
        line,
        side: 0,
        special,
    };
    // `floors` line 4 is the W1 36 line tagging sector 5.
    let after = s
        .start_movers(|m| {
            cross(
                m,
                monster.unwrap_or(player),
                Crosser::Monster,
                crossing(4, 36),
            )
        })
        .unwrap();
    assert_eq!(after, 36);
    assert!(movers::special_data(&s.actors, 5).is_none());
    // The player starts the floor and clears the line; a second crossing (the sector
    // already moving) still clears it and creates nothing.
    for created in [1, 0] {
        let mut events = Vec::new();
        let after = s
            .start_movers(|m| {
                let r = cross(m, player, Crosser::Player, crossing(4, 36));
                events = m.events.clone();
                r
            })
            .unwrap();
        assert_eq!(after, 0);
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Mover(MoverEvent::Call { created: c, .. }) if *c == created
        )));
        assert!(matches!(
            events.last(),
            Some(Event::Mover(MoverEvent::Crossed {
                line: 4,
                side: 0,
                before: 36,
                after: 0,
                ..
            }))
        ));
    }
}

/// The note's §3/§15 risk: a position check that fails at a blocking line keeps the
/// openings visited before it (so a monster's door use after a partial pass sees
/// the same `spechit` as the source), and a thing block keeps none.
#[test]
fn a_line_failure_keeps_the_openings_before_the_blocker() {
    // `dense` (room 17): the player at the lane entrance, moving east into the
    // eight two-sided lift lines and then the east wall.
    let (s, _, player) = crate::planes::tests::setup(17, [200, 64]);
    let actor = *s.actor(player).unwrap();
    let view = s.sector_view();
    // Targets whose box straddles the lift lines and the lane's south wall.
    let mut found = None;
    'search: for y in 1..16 {
        for x in 230..280 {
            let attempt = crate::multisector::check_move(
                (&s.world, view),
                player,
                &actor,
                [Fixed::from_map_units(x), Fixed::from_map_units(y)],
                &s.spatial,
                &|id| s.actor(id).copied(),
                true,
            );
            if attempt.failure == crate::multisector::MoveFailure::Line
                && !attempt.openings.is_empty()
            {
                found = Some(attempt);
                break 'search;
            }
        }
    }
    let attempt = found.expect("a partial line pass");
    let blocker = attempt.movement.lines.blocked_by.unwrap();
    // The openings are exactly the two-sided candidates visited before the blocker.
    let position = attempt
        .movement
        .lines
        .candidates
        .iter()
        .position(|l| *l == blocker)
        .unwrap();
    for visit in &attempt.openings {
        assert!(attempt.movement.lines.candidates[..position].contains(&visit.line));
        assert!(s.world.map().linedefs()[usize::from(visit.line)].sides[1].is_some());
    }
}

/// `spechit` is the special subsequence of the openings, read through the runtime
/// specials (a cleared line drops out), empty for a view without line specials, and
/// its ninth entry is the `SpecialCross` overflow.
#[test]
fn spechit_is_the_special_subsequence_of_the_openings() {
    use crate::multisector::{Opening, OpeningVisit};
    let (s, _, player) = crate::planes::tests::setup(17, [200, 64]);
    let actor = *s.actor(player).unwrap();
    let mut attempt = crate::multisector::check_move(
        (&s.world, s.sector_view()),
        player,
        &actor,
        actor.position(),
        &s.spatial,
        &|id| s.actor(id).copied(),
        true,
    );
    let opening = Opening {
        top: Fixed::ZERO,
        bottom: Fixed::ZERO,
        range: Fixed::ZERO,
        low_floor: Fixed::ZERO,
    };
    let specials = s.line_specials.clone();
    let special_lines: Vec<u16> = (0..specials.len() as u16)
        .filter(|l| specials[usize::from(*l)] != 0)
        .collect();
    let plain = (0..specials.len() as u16)
        .find(|l| specials[usize::from(*l)] == 0)
        .unwrap();
    assert_eq!(special_lines.len(), 8);
    attempt.openings = std::iter::once(plain)
        .chain(special_lines.iter().copied())
        .map(|line| OpeningVisit { line, opening })
        .collect();
    let writes = crate::staging::StagedWrites::default();
    let view = writes.view(&s.sectors, &specials);
    let hit = spechit(view, &attempt, &[]).unwrap();
    assert_eq!(hit.lines(), special_lines.as_slice());
    let hit = spechit(view, &attempt, &special_lines[..1]).unwrap();
    assert_eq!(hit.lines(), &special_lines[1..]);
    // Without line specials (every older policy) nothing is collected.
    assert!(spechit(s.sector_view(), &attempt, &[]).unwrap().is_empty());
    // A ninth special opening overflows.
    attempt.openings.push(OpeningVisit {
        line: special_lines[0],
        opening,
    });
    assert_eq!(spechit(view, &attempt, &[]), Err(SpecialCrossOverflow));
}

/// The `SpecialCross` witness on the Rust-only nine-line lane: admitted only by a
/// test bypass of rule 6, the imp of `dense_imp_fireballs` (radius 20, the only
/// box wide enough to cover nine lines four units apart) walks into the lane and its
/// position check would store a ninth `spechit` entry; the tic is refused and every
/// table, the thinkers and the RNG are byte-identical before and after.
#[test]
fn the_ninth_spechit_entry_refuses_the_tic_with_the_state_preserved() {
    let mut lumps = crate::diagonal_room::Lumps::parse(include_bytes!(
        "../../../tests/reference/m3h8_dense9.bin"
    ));
    // MT_TROOP (3001) at (400, 64) facing west, every skill.
    lumps.add_thing([400, 64, -180, 3001, 7]);
    let c = case("dense_imp_fireballs");
    let (mut s, body, things) = crate::mover_rules::with_window_rule_bypassed(|| {
        crate::movers_replay::setup_map(&c, lumps.map())
    });
    // The committed case's commands and attack entry, until a tic is refused.
    let mut refused = None;
    for op in 1..=c.tics {
        crate::movers_replay::controlled(&mut s, &c, body, &things, op);
        let before = s.clone();
        let snapshot = s.snapshot();
        match s.advance(crate::movers_replay::command(&c, op)) {
            Ok(()) => {}
            Err(error) => {
                assert_eq!(error, SimError::MapScale(MapScaleError::SpecialCross));
                assert_eq!(s.snapshot(), snapshot);
                assert_eq!(s.sectors, before.sectors);
                assert_eq!(s.line_specials, before.line_specials);
                assert_eq!(s.rng, before.rng);
                assert_eq!(s.tic(), before.tic());
                assert_eq!(s.actor(body), before.actor(body));
                refused = Some(op);
                break;
            }
        }
    }
    assert!(refused.is_some(), "the lane reaches a ninth entry");
}

/// The sector-thinker records of the shared order (live and removed).
fn sector_thinkers(s: &Simulation) -> Vec<SectorThinker> {
    s.shared_thinker_order()
        .filter_map(ThinkerId::sector)
        .map(|id| *s.sector_thinker(id).unwrap())
        .collect()
}

/// Review finding 1 (DR): a between-tic see-state entry (`door_chase_entry`, the C
/// harness's `chase_at`) runs `A_Chase` at once, and its `P_Move` collects `spechit`
/// and takes the failure branch exactly as inside a tic (p_map.c:410, p_enemy.c:310–324
/// do not depend on how the move was started): the zombieman pressed against the
/// closed door D1 tries east (`MP`: movedir 0, one entry), `movedir` becomes
/// `DI_NODIR`, the DR line 2 is used from side 0 and returns true (`MU`), and a
/// normal door rising toward 124 is created before any tic runs (C `TK` after op 1).
#[test]
fn a_between_tic_chase_entry_uses_a_dr_line_from_its_blocked_move() {
    let c = case("door_chase_entry");
    let (mut s, body, things) = crate::movers_replay::setup(&c);
    let zombie = things[0];
    let pending = s.pending_events().len();
    crate::movers_replay::controlled(&mut s, &c, body, &things, 1);
    let events = &s.pending_events()[pending..];
    let blocked: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::Mover(MoverEvent::SpechitBlocked { actor, count }) => Some((*actor, *count)),
            _ => None,
        })
        .collect();
    assert_eq!(blocked, [(zombie, 1)]);
    let uses: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::Mover(MoverEvent::SpecialUse {
                user,
                line,
                side,
                before,
                result,
                after,
            }) => Some((*user, *line, *side, *before, *result, *after)),
            _ => None,
        })
        .collect();
    assert_eq!(uses, [(zombie, 2, 0, 1, true, 1)]);
    assert_eq!(s.actors.get(zombie).unwrap().life.unwrap().movedir, 8);
    match sector_thinkers(&s).as_slice() {
        [SectorThinker::Door(d)] => {
            assert_eq!(d.sector, 2);
            assert_eq!(d.kind, movers::DoorKind::Normal);
            assert_eq!(d.direction, 1);
            assert_eq!(d.topheight, Fixed::from_map_units(124));
        }
        other => panic!("{other:?}"),
    }
    // Created between tics: the door steps in the next tic, as in C (ceiling 2 after op 1).
    assert_eq!(s.sectors[2].ceiling, Fixed::ZERO);
    s.advance(crate::movers_replay::command(&c, 1)).unwrap();
    assert_eq!(s.sectors[2].ceiling, Fixed::from_map_units(2));
}

/// Review finding 1 (88): a between-tic see-state entry (`lift_chase_entry`) whose
/// first `P_TryWalk` north crosses the WR 88 line of the lower hall calls
/// `P_CrossSpecialLine` from the crossing loop (C `MX`/`MY` in op 1: line 0, old side
/// 0, special kept) and the lift (sector 3) starts down toward the hall floor 2.
#[test]
fn a_between_tic_chase_entry_crosses_a_lift_line() {
    let c = case("lift_chase_entry");
    let (mut s, body, things) = crate::movers_replay::setup(&c);
    let zombie = things[0];
    let pending = s.pending_events().len();
    crate::movers_replay::controlled(&mut s, &c, body, &things, 1);
    let crossings: Vec<_> = s.pending_events()[pending..]
        .iter()
        .filter_map(|e| match e {
            Event::Mover(MoverEvent::Crossed {
                actor,
                line,
                side,
                before,
                after,
            }) => Some((*actor, *line, *side, *before, *after)),
            _ => None,
        })
        .collect();
    assert_eq!(crossings, [(zombie, 0, 0, 88, 88)]);
    match sector_thinkers(&s).as_slice() {
        [SectorThinker::Plat(p)] => {
            assert_eq!(p.sector, 3);
            assert_eq!(p.status, movers::PlatStatus::Down);
            assert_eq!(p.low, Fixed::from_map_units(2));
            assert_eq!(p.high, Fixed::from_map_units(64));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(s.line_specials[0], 88);
}
