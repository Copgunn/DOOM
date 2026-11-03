//! M3h8 transactions and soaks (plan step 14): a tic refused in the middle of a door
//! movement, a lift ride, a floor move, a use and at the `SpecialCross` witness
//! leaves every byte of the state as it was, the exit and reborn refusals preserve
//! it, and 1,200-tic soaks with four seeds on the doorway, switchyard, lift and
//! floors rooms keep V′, the item witness, `step == advance` and the DSIM35 round
//! trip. Rust-only safety tests: the refusals inside a tic are injected by the
//! test-only `planes::inject_fault` (after a plane step's write and its
//! `P_ChangeSector`, or after a use's line action), never by Rust output.
use crate::movers_replay::{Case, case, command, controlled, setup, setup_map};
use crate::planes::{FaultSite, fault_armed, inject_fault};
use crate::v_prime_tests::{Commands, assert_v_prime};
use crate::*;

/// Everything a refused tic must leave as it was: the canonical capture bytes, the
/// RNG, the sector and runtime line-special tables, the buttons, the exit flag, the
/// player, the whole thinker store (actors and movers, in order), the event log and
/// the pending events, and the tic.
#[derive(Debug, PartialEq)]
struct Witness {
    bytes: Vec<u8>,
    rng: doom_types::DoomRng,
    sectors: Vec<SectorState>,
    line_specials: Vec<i16>,
    buttons: String,
    level_exit: bool,
    player: String,
    thinkers: String,
    thinker_order: Vec<ThinkerId>,
    events: Vec<Event>,
    pending: Vec<Event>,
    tic: u64,
}
fn witness(s: &Simulation) -> Witness {
    Witness {
        bytes: s
            .snapshot()
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap(),
        rng: s.rng,
        sectors: s.sectors.clone(),
        line_specials: s.line_specials.clone(),
        buttons: format!("{:?}", s.buttons),
        level_exit: s.level_exit,
        player: format!("{:?}", s.original_player),
        thinkers: format!("{:?}", s.actors),
        thinker_order: s.shared_thinker_order().collect(),
        events: s.events().to_vec(),
        pending: s.pending_events().to_vec(),
        tic: s.tic(),
    }
}

/// Replays `c` and, at every op whose tic visits `site` (found on an unfaulted
/// copy), refuses that tic at the first and at the last visit: the error is the
/// injected one, the fault was consumed inside the tic, the state is byte-identical,
/// and the same tic then runs exactly as on the unfaulted copy. Returns the refused
/// tics.
fn refuse_at_every_visit(c: &Case, site: FaultSite, every: usize) -> usize {
    let (mut s, body, things) = setup(c);
    let mut refused = 0;
    for op in 1..=c.tics {
        controlled(&mut s, c, body, &things, op);
        let mut reference = s.clone();
        reference.advance(command(c, op)).unwrap();
        // The tic's events (the log holds the last tic's, pending events first).
        let visits = reference
            .events()
            .iter()
            .filter(|e| match site {
                FaultSite::PlaneStep => {
                    matches!(e, Event::Mover(MoverEvent::PlaneStep { direction, .. }) if *direction != 0)
                }
                FaultSite::SpecialUse => matches!(e, Event::Mover(MoverEvent::SpecialUse { .. })),
            })
            .count();
        if visits > 0 && op % every == 0 {
            for nth in [1, visits] {
                let before = witness(&s);
                let mut faulted = s.clone();
                inject_fault(Some((site, nth)));
                let result = faulted.advance(command(c, op));
                let consumed = !fault_armed();
                inject_fault(None);
                assert_eq!(
                    result,
                    Err(SimError::Store(StoreError::Capacity)),
                    "{} op{op} {site:?} {nth}",
                    c.label
                );
                assert!(consumed, "{} op{op}", c.label);
                assert_eq!(witness(&faulted), before, "{} op{op} {site:?}", c.label);
                faulted.advance(command(c, op)).unwrap();
                assert_eq!(witness(&faulted), witness(&reference), "{} op{op}", c.label);
                refused += 1;
            }
        }
        s = reference;
    }
    refused
}

/// A refused tic in the middle of a door movement (opening, waiting doors closing,
/// a closing door reversing on the player, a closing door crunching a corpse and
/// removing a drop), a lift ride with player and monster riders, the turbo and
/// lowest floors and both blazeOpen doors: at sampled ops of every committed
/// scenario that moves a plane, at the first and the last plane step of the tic,
/// after the step has written its height and run `P_ChangeSector` (clipping things,
/// touching pickups, gibbing corpses).
#[test]
fn a_tic_refused_inside_a_plane_step_preserves_every_byte() {
    for (label, every) in [
        ("door_use_wait_close", 3),
        ("door_reverse_on_player", 5),
        ("door_gibs_drop_bonus", 2),
        ("door_stuck_shootable", 5),
        ("door_monster_back_side", 7),
        ("lift_player", 2),
        ("lift_monster_rider", 2),
        ("lift_reverse_at_top", 3),
        ("floor_turbo_jump_up", 1),
        ("floor_lower_to_lowest", 1),
        ("blaze_reversed", 1),
    ] {
        let refused = refuse_at_every_visit(&case(label), FaultSite::PlaneStep, every);
        assert!(refused >= 2, "{label}: {refused}");
    }
}

/// A refused tic during a use, after the line's action ran (a DR door started or
/// reversed, an SR door started with its switch sound and button, the exit switch
/// with the exit requested, a monster's door use from the back side): the door,
/// the button, the cleared exit special and `level.exit` are all undone, and the
/// tic then runs exactly as unfaulted.
#[test]
fn a_tic_refused_during_a_use_preserves_every_byte() {
    for label in [
        "door_use_wait_close",
        "door_reuse_closes",
        "switch_sr_door",
        "switch_exit",
        "switch_exit_after_button",
        "use_held_latch",
        "use_back_side",
        "door_monster_back_side",
        "door_monster_bump",
    ] {
        let refused = refuse_at_every_visit(&case(label), FaultSite::SpecialUse, 1);
        assert!(refused > 0, "{label}");
    }
    // The exit tic refused: no exit, the S1 special still 11, no button.
    let c = case("switch_exit");
    let (mut s, body, things) = setup(&c);
    for op in 1..=c.tics {
        controlled(&mut s, &c, body, &things, op);
        let mut reference = s.clone();
        reference.advance(command(&c, op)).unwrap();
        if reference.level_exit {
            inject_fault(Some((FaultSite::SpecialUse, 1)));
            let result = s.advance(command(&c, op));
            inject_fault(None);
            assert!(result.is_err());
            assert!(!s.level_exit);
            assert_eq!(s.line_specials[10], 11);
            assert!(s.buttons.iter().all(Option::is_none));
            return;
        }
        s = reference;
    }
    panic!("switch_exit reaches its exit");
}

/// The `SpecialCross` witness on the Rust-only nine-line lane (rule 6 bypassed by
/// the test switch): the refused tic leaves every byte, the thinkers, the events
/// and the RNG as they were, and refuses again on retry.
#[test]
fn the_special_cross_witness_preserves_every_byte() {
    let mut lumps = crate::diagonal_room::Lumps::parse(include_bytes!(
        "../../../tests/reference/m3h8_dense9.bin"
    ));
    lumps.add_thing([400, 64, -180, 3001, 7]);
    let c = case("dense_imp_fireballs");
    let (mut s, body, things) =
        crate::mover_rules::with_window_rule_bypassed(|| setup_map(&c, lumps.map()));
    for op in 1..=c.tics {
        controlled(&mut s, &c, body, &things, op);
        let before = witness(&s);
        if let Err(error) = s.advance(command(&c, op)) {
            assert_eq!(error, SimError::MapScale(MapScaleError::SpecialCross));
            assert_eq!(witness(&s), before);
            assert_eq!(s.step(command(&c, op)), Err(error));
            assert_eq!(witness(&s), before);
            return;
        }
    }
    panic!("the lane reaches a ninth entry");
}

/// The exit (`switch_exit`) and the reborn request (`dead_use_reborn`): every later
/// `advance` and `step`, with any command, is refused with the whole state kept.
#[test]
fn exit_and_reborn_refusals_preserve_every_byte() {
    for (label, error) in [
        ("switch_exit", SimError::LevelExited),
        ("dead_use_reborn", SimError::PlayerReborn),
    ] {
        let c = case(label);
        let (mut s, _) = crate::movers_replay::replay(&c, c.tics);
        let before = witness(&s);
        let walk = [
            TicCommand {
                forward: 25,
                side: 0,
                angle_turn: 0,
                buttons: 2,
            },
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ];
        for commands in [command(&c, c.tics), [TicCommand::default(); 4], walk] {
            assert_eq!(s.advance(commands), Err(error), "{label}");
            assert_eq!(s.step(commands), Err(error), "{label}");
            assert_eq!(witness(&s), before, "{label}");
        }
    }
}

/// 1,200 tics with four seeds on the doorway (empty, back-side monster, gibs and
/// stuck populations), switchyard, lift and floors rooms with a command stream that
/// walks, turns, fires and uses (`BT_USE` about one tic in nine): V′ and the item
/// witness after every tic; every 10th tic `step` returns the capture `advance`
/// reaches; every 50th tic the DSIM35 capture decodes with `decode_movers` and
/// re-encodes to the same bytes. An exit ends a run (its refusals are asserted
/// above); a dead player gets the empty command, so no reborn is requested.
#[test]
fn soaks_keep_v_prime_step_equality_and_the_dsim35_round_trip() {
    let limits = DiagnosticLimits::default();
    let rooms = [
        "door_use_wait_close",
        "door_monster_back_side",
        "door_gibs_drop_bonus",
        "door_stuck_shootable",
        "switch_sr_door",
        "lift_player",
        "floor_turbo_jump_up",
        "floor_turbo_lower",
        "floor_lower_to_lowest",
        "blaze_open",
    ];
    let (mut uses, mut movers, mut exits, mut round_trips) = (0, 0, 0, 0);
    for label in rooms {
        let base = case(label);
        for seed in [3u8, 90, 171, 250] {
            let mut c = base.clone();
            c.seed = seed;
            let (mut s, _, _) = setup_map(&c, crate::player_movers_tests::room(c.map).map());
            let mut commands = Commands(0x85eb_ca6b ^ (u32::from(seed) << 12) ^ c.map as u32);
            for op in 1..=1200 {
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
                assert_v_prime(&s, label, op);
                movers += s
                    .shared_thinker_order()
                    .filter(|t| t.sector().is_some())
                    .count();
                if op % 50 == 0 {
                    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
                    let view = Diagnostic::decode_movers(&bytes, limits)
                        .unwrap_or_else(|e| panic!("{label} seed {seed} op{op}: {e:?}"));
                    assert_eq!(view.encode(limits).unwrap(), bytes);
                    round_trips += 1;
                }
                if s.level_exit {
                    exits += 1;
                    assert_eq!(s.advance(command), Err(SimError::LevelExited));
                    break;
                }
            }
        }
    }
    assert!(
        uses > 1000 && movers > 0 && round_trips > 0,
        "{uses} {movers}"
    );
    println!(
        "movers soaks: {uses} use commands, {movers} mover-tics, {exits} exits, {round_trips} round trips"
    );
}
