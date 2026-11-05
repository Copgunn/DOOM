//! M3h9c level policy (wire 22, decision G4): its identity, the body spawned at
//! its THINGS record in lump order with the record's angle, the `damagecount`
//! decay of `P_PlayerThink` and `P_DeathThink` (turn toward the attacker, any
//! command, `BT_USE` reborn) and the DSIM38 reader. Rust-only policy tests: the
//! expected values come from the source reading (p_setup.c:313–346,
//! p_mobj.c:642–736, p_user.c:182–228 and 355–359), not from the C engine.
use crate::barrels_replay::{cases, room};
use crate::diagonal_room::Lumps;
use crate::*;

const LEVEL: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersLevelV1;
const BARRELS: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersBarrelsV1;

fn things(l: &mut Lumps) -> &mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == "THINGS").unwrap().1
}
fn record(x: i16, y: i16, angle: i16, kind: i16, options: i16) -> Vec<u8> {
    [x, y, angle, kind, options]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
/// `l` with a type-1 record at lump index `at` (clamped to the end).
fn with_start(mut l: Lumps, at: usize, position: [i16; 2], angle: i16) -> (Lumps, usize) {
    let t = things(&mut l);
    let at = at.min(t.len() / 10);
    let r = record(position[0], position[1], angle, 1, 7);
    t.splice(at * 10..at * 10, r);
    (l, at)
}
fn options(seed: u8) -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(seed, 0),
        respawn_monsters: false,
    }
}
fn level_setup(map: doom_map::Map, seed: u8) -> (Simulation, MapSpawnReport) {
    let mut s = Simulation::original_player_level_map(map, options(seed)).unwrap();
    let report = s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    (s, report)
}
fn zero() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
fn with(command: TicCommand) -> [TicCommand; 4] {
    let mut c = zero();
    c[0] = command;
    c
}
fn body(s: &Simulation) -> ActorId {
    s.original_player.unwrap().mo.unwrap()
}
fn angle(s: &Simulation, id: ActorId) -> u32 {
    s.actor(id).unwrap().angle()
}

#[test]
fn level_policy_identity_is_wire_22_with_its_own_context() {
    assert_eq!(LEVEL.wire_id(), 22);
    assert_eq!(BARRELS.wire_id(), 21);
    assert_eq!(LEVEL.context_schema(), PLAYER_LEVEL_CONTEXT_SCHEMA);
    assert_eq!(
        PLAYER_LEVEL_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1"
    );
    assert_ne!(LEVEL.context_schema(), BARRELS.context_schema());
    // A strict superset of the barrels policy; `has_level` is wire 22 alone.
    assert!(LEVEL.has_level() && LEVEL.has_barrels() && LEVEL.has_e1m1());
    assert!(LEVEL.has_movers() && LEVEL.has_specials() && LEVEL.has_world_pickups());
    assert!(!BARRELS.has_level());
    assert_eq!(player_map_scale::actor_slots(LEVEL), BARRELS_ACTOR_SLOTS);
}

/// Every M3h9b room with a type-1 record inserted mid-lump (at the case's
/// position, with a non-octant-zero angle) loads under wire 22: the body is spawned
/// at that lump position (every actor before it in the thinker list is a record
/// before it; the slot follows), with `ANG45 * (angle/45)`, no spawnpoint, and the
/// same draw count as the wire-21 setup (the body draws once in both).
#[test]
fn every_m3h9b_room_spawns_the_body_at_its_record_with_its_angle() {
    let mut seen = std::collections::BTreeSet::new();
    let mut checked = 0;
    for c in cases() {
        if !seen.insert(c.map) {
            continue;
        }
        let original = room(c.map);
        let count = original
            .lumps
            .iter()
            .find(|(n, _)| n == "THINGS")
            .unwrap()
            .1
            .len()
            / 10;
        for (at, angle) in [(count / 2, 135_i16), (0, -90), (count, 359)] {
            let (l, at) = with_start(
                Lumps {
                    lumps: original.lumps.clone(),
                },
                at,
                c.position,
                angle,
            );
            let (s, report) = level_setup(l.map(), c.seed);
            let id = report.player_body.expect("body spawned");
            assert_eq!(body(&s), id);
            assert_eq!(angle_of(&s, id), e1m1_rules::spawn_angle(angle));
            assert!(s.actor(id).unwrap().life().unwrap().spawnpoint.is_none());
            let order: Vec<_> = s.actor_order().collect();
            let position = order.iter().position(|a| *a == id).unwrap();
            let before = report
                .spawned
                .iter()
                .filter(|a| {
                    let p = s.actor(**a).unwrap().life().unwrap().spawnpoint.unwrap();
                    let lump = &l.lumps.iter().find(|(n, _)| n == "THINGS").unwrap().1;
                    let index = lump
                        .chunks_exact(10)
                        .position(|r| {
                            r == record(
                                p.position.x,
                                p.position.y,
                                p.angle,
                                p.kind,
                                p.options as i16,
                            )
                        })
                        .unwrap();
                    index < at
                })
                .count();
            assert_eq!(position, before, "{} at {at}", c.label);
            assert_eq!(id.slot() as usize, before);
            // Same RNG draws as the wire-21 setup of the original room.
            let mut old =
                Simulation::original_player_barrels_map(original.map(), options(c.seed)).unwrap();
            old.spawn_map_things().unwrap();
            old.spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
                .unwrap();
            assert_eq!(old.rng.indices(), s.rng.indices());
            // DSIM38 decodes; DSIM37 readers refuse it.
            let bytes = s.snapshot().canonical_bytes();
            let d = Diagnostic::decode_level(&bytes, DiagnosticLimits::default()).unwrap();
            assert_eq!(d.version(), 38);
            assert_eq!(d.encode(DiagnosticLimits::default()).unwrap(), bytes);
            assert_eq!(
                Diagnostic::decode_barrels(&bytes, DiagnosticLimits::default()).err(),
                Some(DiagnosticError::Version(38))
            );
            checked += 1;
        }
    }
    assert!(checked >= 3);
}
fn angle_of(s: &Simulation, id: ActorId) -> u32 {
    angle(s, id)
}

#[test]
fn level_without_one_type1_record_and_caller_bodies_are_refused() {
    let c = &cases()[0];
    let mut l = room(c.map);
    things(&mut l).clear();
    let mut s = Simulation::original_player_level_map(l.map(), options(0)).unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.spawn_map_things().err(),
        Some(SimError::PlayerBody(PlayerBodyError::Binding))
    );
    assert_eq!(s.snapshot(), before);
    assert_eq!(
        s.spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
            .err(),
        Some(SimError::PlayerBody(PlayerBodyError::Unsupported))
    );
    let (l, _) = with_start(l, 0, c.position, 0);
    let (mut l, _) = with_start(l, 1, c.position, 90);
    let _ = &mut l;
    let mut s = Simulation::original_player_level_map(l.map(), options(0)).unwrap();
    assert_eq!(
        s.spawn_map_things().err(),
        Some(SimError::PlayerBody(PlayerBodyError::Binding))
    );
}

/// The M3h9b case `fall_forward_draw` (room 5: a Zombieman and a barrel north of
/// the player position).
fn room5() -> movers_replay::Case {
    cases().into_iter().find(|c| c.map == 5).unwrap()
}
/// Room 5 with only the start and, optionally, one Zombieman `dy` units north.
fn lone_room(angle: i16, zombie: Option<i16>) -> (Simulation, Option<ActorId>) {
    let c = &room5();
    let mut l = room(c.map);
    things(&mut l).clear();
    let (mut l, _) = with_start(l, 0, c.position, angle);
    if let Some(dy) = zombie {
        // MTF_AMBUSH: the deaf Zombieman waits; it never sees a dead player.
        things(&mut l).extend(record(c.position[0], c.position[1] + dy, 0, 3004, 7 | 8));
    }
    let (s, report) = level_setup(l.map(), 0);
    (s, report.spawned.first().copied())
}
fn damage(s: &mut Simulation, source: Option<ActorId>, amount: i32) {
    let request = DamageRequest {
        target: body(s),
        inflictor: None,
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
    .unwrap();
}

#[test]
fn damagecount_decays_one_per_live_tic_under_wire_22_only() {
    let (mut s, _) = lone_room(0, None);
    damage(&mut s, None, 7);
    assert_eq!(s.original_player.unwrap().damage_count, 7);
    for expected in [6, 5, 4, 3, 2, 1, 0, 0, 0] {
        s.advance(zero()).unwrap();
        assert_eq!(s.original_player.unwrap().damage_count, expected);
        let bytes = s.snapshot().canonical_bytes();
        Diagnostic::decode_level(&bytes, DiagnosticLimits::default()).unwrap();
    }
    // The barrels policy keeps its frozen contract: no decay.
    let c = &cases()[0];
    let mut l = room(c.map);
    things(&mut l).clear();
    let mut old = Simulation::original_player_barrels_map(l.map(), options(0)).unwrap();
    old.spawn_map_things().unwrap();
    old.spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
        .unwrap();
    old.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    old.spawn_level_specials().unwrap();
    damage(&mut old, None, 7);
    for _ in 0..3 {
        old.advance(zero()).unwrap();
    }
    assert_eq!(old.original_player.unwrap().damage_count, 7);
}

/// p_user.c:200–224 with the attacker due north: `R_PointToAngle2` is
/// `ANG90 - 1` there (x 0, y > 0: `ANG90-1 - tantoangle[0]`). From angle 0 the
/// body steps `+ANG5` (delta below `ANG180`) for 18 tics, the 19th sees
/// `delta = 0x3fffffff - 18·ANG5 = 9 < ANG5` and sets the angle exactly; the
/// damage count only decays from then on. From `ANG180` it steps `−ANG5`.
#[test]
fn a_dead_player_turns_toward_its_attacker() {
    const NORTH: u32 = 0x3fff_ffff;
    let ang5 = crate::level::ANG5;
    assert_eq!(ang5, 59_652_323);
    for (start, step) in [(0_i16, 1_i64), (180, -1)] {
        let (mut s, zombie) = lone_room(start, Some(256));
        let zombie = zombie.unwrap();
        damage(&mut s, Some(zombie), 1000);
        let p = s.original_player.unwrap();
        assert_eq!(p.state, PlayerBodyState::Dead);
        assert_eq!(p.attacker, Some(zombie));
        assert_eq!(p.damage_count, 100);
        let id = body(&s);
        let mut expected = e1m1_rules::spawn_angle(start);
        let mut count = 100;
        let mut reached = false;
        for _ in 0..40 {
            let delta = NORTH.wrapping_sub(expected);
            if delta < ang5 || delta > ang5.wrapping_neg() {
                expected = NORTH;
                reached = true;
                count -= 1;
            } else {
                expected = if step > 0 {
                    expected.wrapping_add(ang5)
                } else {
                    expected.wrapping_sub(ang5)
                };
            }
            s.advance(zero()).unwrap();
            assert_eq!(angle(&s, id), expected);
            assert_eq!(s.original_player.unwrap().damage_count, count);
            let bytes = s.snapshot().canonical_bytes();
            Diagnostic::decode_level(&bytes, DiagnosticLimits::default()).unwrap();
        }
        assert!(reached);
        assert_eq!(angle(&s, id), NORTH);
    }
}

/// A dead player with no attacker (environment damage) only decays.
#[test]
fn a_dead_player_without_attacker_only_decays() {
    let (mut s, _) = lone_room(45, None);
    damage(&mut s, None, 1000);
    let id = body(&s);
    let a = angle(&s, id);
    for n in 1..=5 {
        s.advance(zero()).unwrap();
        assert_eq!(angle(&s, id), a);
        assert_eq!(s.original_player.unwrap().damage_count, 100 - n);
    }
}

/// p_user.c:227–228 and g_game.c:611–614: a dead player accepts any admitted
/// command and reads only `BT_USE`; `PST_REBORN` set in tic N is acted on by the
/// reborn loop at the start of tic N+1's `G_Ticker`, so tic N+1 is refused with
/// the state preserved. The barrels policy still refuses the extra fields.
#[test]
fn bt_use_while_dead_requests_a_reborn_and_the_next_tic_is_refused() {
    let (mut s, zombie) = lone_room(0, Some(256));
    damage(&mut s, zombie, 1000);
    let busy = TicCommand {
        forward: 50,
        side: -50,
        angle_turn: 0x0800,
        buttons: weapons::BT_ATTACK | weapons::BT_CHANGE | (2 << 3),
    };
    s.advance(with(busy)).unwrap();
    assert!(!s.original_player.unwrap().reborn);
    let id = body(&s);
    assert_eq!(s.actor(id).unwrap().momentum(), [Fixed::ZERO; 2]);
    let used = TicCommand {
        buttons: weapons::BT_USE | weapons::BT_ATTACK,
        forward: 10,
        ..busy
    };
    s.advance(with(used)).unwrap();
    assert!(s.original_player.unwrap().reborn);
    let before = s.snapshot();
    assert_eq!(s.advance(zero()), Err(SimError::PlayerReborn));
    assert_eq!(s.snapshot(), before);
    // Wire 21: the same dead player refuses the busy command.
    let c = &room5();
    let mut l = room(c.map);
    things(&mut l).clear();
    things(&mut l).extend(record(c.position[0], c.position[1] + 256, 0, 3004, 7 | 8));
    let mut old = Simulation::original_player_barrels_map(l.map(), options(0)).unwrap();
    let z = old.spawn_map_things().unwrap().spawned[0];
    old.spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
        .unwrap();
    old.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    old.spawn_level_specials().unwrap();
    damage(&mut old, Some(z), 1000);
    let before = old.snapshot();
    assert_eq!(
        old.advance(with(busy)),
        Err(SimError::PlayerBody(PlayerBodyError::Unsupported))
    );
    assert_eq!(old.snapshot(), before);
}

/// Re-encode `snap` with the DSIM version, `profile` and `context.schema`
/// replaced, keeping every other field.
fn relabel(snap: &Snapshot, version: u32, profile: OriginalExecutionProfile) -> Vec<u8> {
    let fields: Vec<(String, Vec<u8>)> = snap
        .fields()
        .map(|f| {
            let value = match f.path() {
                "profile" => profile.wire_id().to_le_bytes().to_vec(),
                "context.schema" => profile.context_schema().to_vec(),
                _ => f.value().to_vec(),
            };
            (f.path().to_string(), value)
        })
        .collect();
    let mut out = b"DSIM".to_vec();
    out.extend(version.to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for (path, value) in fields {
        out.extend((path.len() as u32).to_le_bytes());
        out.extend(path.as_bytes());
        out.extend((value.len() as u64).to_le_bytes());
        out.extend(value);
    }
    out
}

#[test]
fn decode_level_round_trip_and_refusals() {
    let limits = DiagnosticLimits::default();
    let c = &room5();
    let original = room(c.map);
    let count = original
        .lumps
        .iter()
        .find(|(n, _)| n == "THINGS")
        .unwrap()
        .1
        .len()
        / 10;
    assert!(count >= 2, "the room needs a THING after the start");
    let (l, _) = with_start(
        Lumps {
            lumps: original.lumps.clone(),
        },
        0,
        c.position,
        90,
    );
    let (s, _) = level_setup(l.map(), c.seed);
    let snap = s.snapshot();
    let bytes = snap.canonical_bytes();
    // The relabelling helper is the identity on a DSIM38 capture.
    assert_eq!(relabel(&snap, 38, LEVEL), bytes);
    let d = Diagnostic::decode_level(&bytes, limits).unwrap();
    assert_eq!(d.encode(limits).unwrap(), bytes);
    // Wrong version: every other reader refuses 38, this one refuses 37.
    let mut wrong = bytes.clone();
    wrong[4..8].copy_from_slice(&37_u32.to_le_bytes());
    assert_eq!(
        Diagnostic::decode_level(&wrong, limits).err(),
        Some(DiagnosticError::Version(37))
    );
    let barrels_bytes = relabel(&snap, 37, BARRELS);
    assert_eq!(
        Diagnostic::decode_level(&barrels_bytes, limits).err(),
        Some(DiagnosticError::Version(37))
    );
    for read in [
        Diagnostic::decode_barrels,
        Diagnostic::decode_e1m1,
        Diagnostic::decode,
    ] {
        assert!(matches!(
            read(&bytes, limits),
            Err(DiagnosticError::Version(38))
        ));
    }
    // The body after every THING (the wire-21 setup of the same lump, start
    // first) relabelled as DSIM38: not at its record position.
    let mut old = Simulation::original_player_barrels_map(l.map(), options(c.seed)).unwrap();
    old.spawn_map_things().unwrap();
    old.spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
        .unwrap();
    old.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    old.spawn_level_specials().unwrap();
    let moved = relabel(&old.snapshot(), 38, LEVEL);
    assert_eq!(
        Diagnostic::decode_level(&moved, limits).err(),
        Some(DiagnosticError::Identity)
    );
    // The wire-21 capture itself still decodes under its own reader.
    Diagnostic::decode_barrels(&old.snapshot().canonical_bytes(), limits).unwrap();
}
