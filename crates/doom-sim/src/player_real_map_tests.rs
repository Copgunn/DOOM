//! M3h6 real-map scale: source replay of every committed scenario in rooms of
//! classic-map size, the real-map admission (format bounds, the arithmetic and
//! slide-window certificates, starts as records), the intercept witness, the
//! hitscan buffer regression and the recoverable paths. Expected behavior comes
//! from the original source reading in M3H6_REFERENCE_INVESTIGATION.md, never
//! from Rust output.
use super::*;
use crate::diagonal_room::Lumps;
use crate::player_map_scale::validate_slide_window;
use crate::{diagonal_evidence, noise, real_map_evidence};

// Source replay: every committed scenario against the compiled original rows,
// the independent DSIM33 captures and ordered events; step equals advance.
#[test]
fn real_map_source_case_00_town_patrol() {
    real_map_evidence::verify_case(0);
}
#[test]
fn real_map_source_case_01_town_fight() {
    real_map_evidence::verify_case(1);
}
#[test]
fn real_map_source_case_02_town_sight_far() {
    real_map_evidence::verify_case(2);
}
#[test]
fn real_map_source_case_03_town_slide_chamfer() {
    real_map_evidence::verify_case(3);
}
#[test]
fn real_map_source_case_04_town_window_shot() {
    real_map_evidence::verify_case(4);
}
#[test]
fn real_map_source_case_05_town_fireball() {
    real_map_evidence::verify_case(5);
}
#[test]
fn real_map_source_case_06_gallery_shot() {
    real_map_evidence::verify_case(6);
}
#[test]
fn real_map_source_case_07_gallery_aim_pickups() {
    real_map_evidence::verify_case(7);
}
#[test]
fn real_map_source_case_08_gallery_monster_shot() {
    real_map_evidence::verify_case(8);
}
#[test]
fn real_map_source_case_09_gallery_shotgun() {
    real_map_evidence::verify_case(9);
}
#[test]
fn real_map_source_case_10_extent_shot_long_wall() {
    real_map_evidence::verify_case(10);
}
#[test]
fn real_map_source_case_11_extent_sight_diagonal() {
    real_map_evidence::verify_case(11);
}
#[test]
fn real_map_source_case_12_extent_slide_long_wall() {
    real_map_evidence::verify_case(12);
}
#[test]
fn real_map_source_case_13_extent_fireball_far() {
    real_map_evidence::verify_case(13);
}
#[test]
fn real_map_source_case_14_dense_slide() {
    real_map_evidence::verify_case(14);
}
#[test]
fn real_map_source_case_15_dense_slide_steep() {
    real_map_evidence::verify_case(15);
}
#[test]
fn real_map_source_case_16_dense_shot() {
    real_map_evidence::verify_case(16);
}
#[test]
fn real_map_source_case_17_wide_pursuit() {
    real_map_evidence::verify_case(17);
}
#[test]
fn real_map_source_case_18_wide_sight() {
    real_map_evidence::verify_case(18);
}
#[test]
fn real_map_source_case_19_wide_shot_corridor() {
    real_map_evidence::verify_case(19);
}

const POSSESSED: i16 = 3004;
const CHAINGUY: i16 = 65;
const FLOOR_LAMP: i16 = 2028;
fn options() -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(6, 0),
        respawn_monsters: false,
    }
}
fn committed(label: &str) -> Lumps {
    let cases = real_map_evidence::cases();
    let c = cases.iter().find(|c| c.label == label).unwrap();
    Lumps::parse(real_map_evidence::MAP_BYTES[c.map])
}
fn wedge() -> Lumps {
    let cases = diagonal_evidence::cases();
    let c = cases.iter().find(|c| c.label == "slide_45").unwrap();
    Lumps::parse(diagonal_evidence::MAP_BYTES[c.map])
}
fn witness() -> Lumps {
    Lumps::parse(include_bytes!("../../../tests/reference/m3h6_witness.bin"))
}
fn real_map(lumps: &Lumps) -> Result<Simulation, OriginalRegistryError> {
    Simulation::original_player_real_map_map(lumps.map(), options())
}
fn rejects(result: Result<Simulation, OriginalRegistryError>, expected: SimError) {
    match result {
        Err(OriginalRegistryError::Runtime(error)) => assert_eq!(error, expected),
        Err(other) => panic!("unexpected error {other:?}, expected {expected:?}"),
        Ok(_) => panic!("admitted, expected {expected:?}"),
    }
}
fn arm(s: &mut Simulation, position: [i16; 2]) -> ActorId {
    let body = s
        .spawn_player_body(PlayerSlot::One, position.map(Fixed::from_map_units))
        .unwrap();
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: Weapon::Pistol,
            owned: 3,
            ammo: [50, 0, 0, 0],
        },
    )
    .unwrap();
    body
}
fn attack(turn: i16) -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0] = TicCommand {
        forward: 0,
        side: 0,
        angle_turn: turn,
        buttons: BT_ATTACK,
    };
    c
}

#[test]
fn real_map_admits_every_committed_room_and_the_older_rooms() {
    // Every committed M3h6 map loads under the policy and its spawns are admitted;
    // the diagonal policy rejects every one of them (too many lines, a line norm
    // beyond 1,536 or a span beyond ±2,048).
    for c in real_map_evidence::cases() {
        let (s, _, things) = real_map_evidence::setup(&c);
        assert!(s.actor_order().count() > things.len(), "{}", c.label);
        assert!(
            Simulation::original_player_diagonal_map(real_map_evidence::case_map(c.map), options())
                .is_err(),
            "{}",
            c.label
        );
    }
    // The policy is a superset: every committed M3h5 room loads unchanged.
    for c in diagonal_evidence::cases() {
        Simulation::original_player_real_map_map(diagonal_evidence::case_map(c.map), options())
            .unwrap_or_else(|e| panic!("{}: {e:?}", c.label));
    }
}

#[test]
fn arithmetic_certificate_is_computed_from_the_map_and_refuses_failing_maps() {
    let extent = committed("extent_shot_long_wall");
    let (certificate, clauses) = ArithmeticCertificate::evaluate(&extent.map()).unwrap();
    assert_eq!(clauses, [true; 8]);
    assert_eq!(certificate.span, [2400, 1800]);
    assert_eq!(certificate.norm1, 3850);
    assert_eq!(certificate.norm2_squared, 2200 * 2200 + 1650 * 1650);
    assert_eq!(certificate.component, 2400);
    assert_eq!(certificate.height, 128);
    // The two partitions of the automatic builder lie on the diagonal lines.
    assert!(certificate.node_norm == 3850 || certificate.node_norm == 3600);
    let verdict = |lumps: &Lumps| ArithmeticCertificate::evaluate(&lumps.map()).unwrap().1;
    // (i): the long wall stretched to norm 3,854; every other clause still holds.
    let mut extent = committed("extent_shot_long_wall");
    let wall = extent.find_line([2200, 1650], [0, 0]);
    let [tip, _] = extent.line_vertices(wall);
    extent.set_vertex(tip, [2204, 1650]);
    assert_eq!(
        verdict(&extent),
        [false, true, true, true, true, true, true, true]
    );
    rejects(
        real_map(&extent),
        SimError::MapScale(MapScaleError::Arithmetic),
    );
    assert_eq!(
        ArithmeticCertificate::derive(&extent.map()),
        Err(SimError::MapScale(MapScaleError::Arithmetic))
    );
    // (ii): the rectangle stretched to 2,800 units so that diag · L₂ passes 2²³
    // while (i) holds; the unmoved BLOCKMAP no longer covers it, so (v) fails too.
    let mut extent = committed("extent_shot_long_wall");
    let top = extent.find_line([0, 600], [0, 1800]);
    let [_, corner] = extent.line_vertices(top);
    extent.set_vertex(corner, [0, 2800]);
    assert_eq!(
        verdict(&extent),
        [true, false, true, true, false, true, true, true]
    );
    rejects(
        real_map(&extent),
        SimError::MapScale(MapScaleError::Arithmetic),
    );
    // (iv): the wide square's far corner moved to x = 8,300. The span sum stays
    // below 32,768 (iii) and the 1,132-unit edge keeps (i), but the 11,222-unit
    // diagonal fails (ii) as well and the grid is left behind (v).
    let mut wide = committed("wide_pursuit");
    let far = wide.find_line([7168, 7552], [7552, 7552]);
    let [_, corner] = wide.line_vertices(far);
    wide.set_vertex(corner, [8300, 7552]);
    assert_eq!(
        verdict(&wide),
        [true, false, true, false, false, true, true, true]
    );
    rejects(
        real_map(&wide),
        SimError::MapScale(MapScaleError::Arithmetic),
    );
    // (viii) alone: the whole extent room and its grid shifted to x = 28,500, where
    // a 2,177-unit reach from the far edge leaves the `short` range.
    let mut extent = committed("extent_shot_long_wall");
    for v in 0..6 {
        let [x, y] = extent.vertex(v);
        extent.set_vertex(v, [x + 28_500, y]);
    }
    extent.shift_nodes(28_500);
    extent.set_blockmap_origin([28_500, 0]);
    assert_eq!(
        verdict(&extent),
        [true, true, true, true, true, true, true, false]
    );
    rejects(
        real_map(&extent),
        SimError::MapScale(MapScaleError::Arithmetic),
    );
    // The same shift to x = 28,000 keeps every clause (30,400 + 2,177 < 32,768).
    let mut extent = committed("extent_shot_long_wall");
    for v in 0..6 {
        let [x, y] = extent.vertex(v);
        extent.set_vertex(v, [x + 28_000, y]);
    }
    extent.shift_nodes(28_000);
    extent.set_blockmap_origin([28_000, 0]);
    assert_eq!(verdict(&extent), [true; 8]);
    // (vii): heights beyond ±512 fail the certificate under this policy; the
    // diagonal policy names the same bytes by its own check.
    let mut town = committed("town_patrol");
    town.set_sector(0, [0, 513], 0, b"TESTCEIL");
    rejects(
        real_map(&town),
        SimError::MapScale(MapScaleError::Arithmetic),
    );
    let mut wedge = wedge();
    wedge.set_sector(0, [0, 513], 0, b"TESTCEIL");
    rejects(
        real_map(&wedge),
        SimError::MapScale(MapScaleError::Arithmetic),
    );
    rejects(
        Simulation::original_player_diagonal_map(wedge.map(), options()),
        SimError::Perception(PerceptionError::Arithmetic),
    );
}

#[test]
fn slide_window_certificate_bounds_every_committed_room_and_refuses_an_overfilled_one() {
    let mut largest = 0;
    for c in real_map_evidence::cases() {
        let lines = slide_window_lines(&real_map_evidence::case_map(c.map));
        assert!(lines <= MAX_INTERCEPTS, "{}: {lines}", c.label);
        largest = largest.max(lines);
    }
    assert_eq!(largest, 123);
    assert_eq!(slide_window_lines(&committed("dense_slide").map()), 123);
    assert_eq!(slide_window_lines(&committed("gallery_shot").map()), 26);
    // Extra listings keep the closed-cell rule but overfill one window: every
    // gallery line listed in cell 0.
    let mut gallery = committed("gallery_shot");
    let all: Vec<u16> = (0..gallery.line_count() as u16).collect();
    gallery.list_lines_in_cell(0, &all);
    assert_eq!(slide_window_lines(&gallery.map()), 222);
    rejects(real_map(&gallery), SimError::MapScale(MapScaleError::Lines));
    // 128 distinct lines are admitted: list exactly enough to reach the bound.
    let mut gallery = committed("gallery_shot");
    let present = gallery.cell_lines(0);
    let extra: Vec<u16> = (0..222u16)
        .filter(|l| !present.contains(l))
        .take(MAX_INTERCEPTS - slide_window_lines(&committed("gallery_shot").map()))
        .collect();
    gallery.list_lines_in_cell(0, &extra);
    assert!(slide_window_lines(&gallery.map()) <= MAX_INTERCEPTS);
    real_map(&gallery).unwrap();
}

#[test]
fn real_map_keeps_the_boundary_blockmap_special_and_sky_rules() {
    // C3 on an M3h5 room under the real-map policy.
    let mut wedge = wedge();
    let wall = wedge.find_line([-256, 768], [256, 256]);
    wedge.reverse_line(wall);
    rejects(
        real_map(&wedge),
        SimError::MapScale(MapScaleError::Boundary),
    );
    // The closed-cell rule on a town chamfer.
    let mut town = committed("town_patrol");
    let chamfer = town.find_line([240, 256], [288, 208]);
    town.unlist_line(chamfer);
    rejects(real_map(&town), SimError::MapScale(MapScaleError::Blockmap));
    // A degenerate line.
    let mut town = committed("town_patrol");
    let chamfer = town.find_line([240, 256], [288, 208]);
    let [a, _] = town.line_vertices(chamfer);
    town.set_line_vertex(chamfer, 1, a);
    rejects(real_map(&town), SimError::MapScale(MapScaleError::Geometry));
    // Line and sector specials and a sky ceiling.
    let mut town = committed("town_patrol");
    town.set_line_special(0, 1);
    rejects(real_map(&town), SimError::Pursuit(PursuitError::Specials));
    let mut town = committed("town_patrol");
    town.set_sector(0, [0, 128], 9, b"TESTCEIL");
    rejects(real_map(&town), SimError::Pursuit(PursuitError::Specials));
    let mut town = committed("town_patrol");
    town.set_sector(0, [0, 128], 0, b"F_SKY1\0\0");
    rejects(real_map(&town), SimError::Combat(CombatError::Sky));
    // A short REJECT is the parser's refusal: the one source read past a lump.
    let mut town = committed("town_patrol");
    town.truncate_reject(100);
    let error = town.try_map().expect_err("short REJECT rejected");
    assert!(format!("{error:?}").contains("RejectTooShort"), "{error:?}");
}

#[test]
fn player_starts_are_records_and_unadmitted_types_still_reject() {
    let mut town = committed("town_patrol");
    town.clear_things();
    for kind in 1..=4 {
        town.add_thing([64 + 40 * kind, 144, 0, kind, 7]);
    }
    for i in 0..11 {
        town.add_thing([640 + 40 * i, 144, 0, 11, 7]);
    }
    town.add_thing([1024, 144, 0, POSSESSED, 7]);
    let mut s = real_map(&town).unwrap();
    let report = s.spawn_map_things().unwrap();
    assert_eq!(report.player_starts.len(), 4);
    assert_eq!(
        report
            .player_starts
            .iter()
            .map(|t| t.kind)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    // P_SpawnMapThing keeps at most ten deathmatch starts (p_mobj.c:718–725).
    assert_eq!(report.deathmatch_starts.len(), 10);
    assert_eq!(report.spawned.len(), 1);
    // The diagonal policy still rejects a start before any spawn.
    let mut wedge = wedge();
    wedge.clear_things();
    wedge.add_thing([0, 0, 0, 1, 7]);
    let mut d = Simulation::original_player_diagonal_map(wedge.map(), options()).unwrap();
    assert_eq!(
        d.spawn_map_things().map(|_| ()),
        Err(SimError::MapSpawn(MapSpawnError::StartOrInvalidType(1)))
    );
    // An unadmitted doomednum rejects the whole batch with unchanged state.
    let mut town = committed("town_patrol");
    town.clear_things();
    town.add_thing([64, 144, 0, 1, 7]);
    town.add_thing([1024, 144, 0, FLOOR_LAMP, 7]);
    let mut s = real_map(&town).unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.spawn_map_things().map(|_| ()),
        Err(SimError::MapSpawn(MapSpawnError::UnsupportedType(
            FLOOR_LAMP
        )))
    );
    assert_eq!(s.snapshot(), before);
}

#[test]
fn hitscans_fire_in_a_map_above_128_lines() {
    // Regression for the line-indexed seen[128] buffer: the town's corridor lines
    // carry indices above 128 and sit in the cells a shot from the first room
    // visits; before the fix every such shot failed with InterceptLimit.
    let cases = real_map_evidence::cases();
    let c = cases.iter().find(|c| c.label == "town_sight_far").unwrap();
    assert!(real_map_evidence::case_map(c.map).linedefs().len() > 128);
    let (mut s, _, _) = real_map_evidence::setup(c);
    let before = s
        .original_player_weapons(PlayerSlot::One)
        .unwrap()
        .unwrap()
        .bullets;
    for op in 1..=30 {
        s.advance(real_map_evidence::commands(c, op))
            .unwrap_or_else(|e| panic!("tic {op}: {e}"));
    }
    let after = s
        .original_player_weapons(PlayerSlot::One)
        .unwrap()
        .unwrap()
        .bullets;
    assert!(after < before, "the pistol fired ({before} -> {after})");
}

#[test]
fn intercept_witness_refuses_the_tic_and_keeps_the_state() {
    // The Rust-only witness hall: a 2,048-unit shot east from (-1000, 100)
    // straddles 130 teeth lines, so P_LineAttack would write the 129th intercept.
    // The autoaim traces (1,024 units) stay below the bound; the shot's tic fails
    // with Intercepts through both advance and step, byte for byte unchanged.
    let hall = witness();
    assert_eq!(hall.map().linedefs().len(), 200);
    assert!(slide_window_lines(&hall.map()) <= MAX_INTERCEPTS);
    let mut s = real_map(&hall).unwrap();
    arm(&mut s, [-1000, 100]);
    let mut stepped = s.clone();
    let mut refused = None;
    for op in 1..=40 {
        let before = s.snapshot();
        let result = s.advance(attack(0));
        let step = stepped.step(attack(0)).map(|_| ());
        assert_eq!(result, step, "tic {op}");
        if let Err(error) = result {
            assert_eq!(error, SimError::MapScale(MapScaleError::Intercepts));
            assert_eq!(s.snapshot(), before);
            assert_eq!(stepped.snapshot(), before);
            refused = Some(op);
            break;
        }
    }
    // The tic holds three 1,024-unit aim traces (P_BulletSlope) and the 2,048-unit
    // shot; an aim trace reaches at most 34 teeth (69 intercepts), so the shot is
    // the trace that writes the 129th. The refusal comes after the ready states.
    let refused = refused.expect("the long shot reached the witness");
    assert!(refused > 1, "aim and ready states precede the shot");
    // Turned north, the same state fires across the 256-unit hall.
    s.advance(attack(16384)).unwrap();
    let before = s
        .original_player_weapons(PlayerSlot::One)
        .unwrap()
        .unwrap()
        .bullets;
    for _ in 0..40 {
        s.advance(attack(0)).unwrap();
    }
    assert!(
        s.original_player_weapons(PlayerSlot::One)
            .unwrap()
            .unwrap()
            .bullets
            < before
    );
    // The same hall under the diagonal policy is refused at load (200 lines).
    rejects(
        Simulation::original_player_diagonal_map(hall.map(), options()),
        SimError::MapScale(MapScaleError::Lines),
    );
}

/// Rust-only internal equivalence: the prepared per-sector lists give the same
/// flood as the linedef scan on the same world. The source evidence for the flood
/// itself is the C `N` rows replayed by the scenarios.
#[test]
fn noise_flood_with_sector_lists_matches_the_linedef_scan() {
    let mut s = real_map(&committed("town_fight")).unwrap();
    let lists = s.world.sector_lines.take().expect("prepared lists");
    assert_eq!(lists.len(), s.world.map().sectors().len());
    assert_eq!(
        lists.iter().map(Vec::len).sum::<usize>(),
        s.world
            .map()
            .linedefs()
            .iter()
            .map(|l| 1 + usize::from(l.sides[1].is_some()))
            .sum::<usize>()
    );
    let scanned: Vec<_> = (0..s.sectors.len() as u16)
        .map(|start| noise::flood(&s.world, s.sector_view(), start))
        .collect();
    s.world.sector_lines = Some(lists);
    for (start, expected) in scanned.iter().enumerate() {
        assert_eq!(
            &noise::flood(&s.world, s.sector_view(), start as u16),
            expected,
            "sector {start}"
        );
    }
}

#[test]
fn real_map_policy_identity_is_distinct_from_the_older_policies() {
    let profile = OriginalExecutionProfile::PlayerMonstersRealMapV1;
    assert_eq!(profile.wire_id(), 17);
    assert_eq!(profile.context_schema(), PLAYER_REAL_MAP_CONTEXT_SCHEMA);
    assert_eq!(
        PLAYER_REAL_MAP_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-REALMAP-1-MEDIUM-v1"
    );
    assert!(
        profile.has_real_map()
            && profile.has_diagonals()
            && profile.has_heights()
            && profile.has_map_scale()
    );
    assert!(!OriginalExecutionProfile::PlayerMonstersDiagonalV1.has_real_map());
    assert_eq!(REAL_MAP_FORMAT_RECORDS, 32_767);
    let s = real_map(&committed("town_patrol")).unwrap();
    let bytes = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let view = Diagnostic::decode_real_map(&bytes, DiagnosticLimits::default()).unwrap();
    assert_eq!(view.field("profile"), Some(&17u32.to_le_bytes()[..]));
    assert_eq!(
        view.field("context.schema"),
        Some(PLAYER_REAL_MAP_CONTEXT_SCHEMA)
    );
    assert_eq!(view.field("sectors.count"), Some(&74u32.to_le_bytes()[..]));
    assert!(Diagnostic::decode_diagonal(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_heights(&bytes, DiagnosticLimits::default()).is_err());
    // A DSIM32 diagonal capture is not a DSIM33 one.
    let diagonal = Simulation::original_player_diagonal_map(wedge().map(), options()).unwrap();
    let bytes = diagonal
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    assert!(Diagnostic::decode_real_map(&bytes, DiagnosticLimits::default()).is_err());
    Diagnostic::decode_diagonal(&bytes, DiagnosticLimits::default()).unwrap();
}

/// The M3h3 scripted command sequence over a seed: runs, strafes and turns.
fn scripted(seed: u64, op: usize) -> [TicCommand; 4] {
    let mut x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(op as u64);
    x ^= x >> 29;
    let mut c = [TicCommand::default(); 4];
    c[0] = TicCommand {
        forward: if x & 3 == 0 { 0 } else { 50 },
        side: match (x >> 2) & 3 {
            0 => -40,
            1 => 40,
            _ => 0,
        },
        angle_turn: match (x >> 4) & 7 {
            0 => 1024,
            1 => -1024,
            2 => 4096,
            _ => 0,
        },
        buttons: u8::from((x >> 7) & 7 == 0),
    };
    c
}
/// Invariant Ω of the M3h5 note after every tic, unchanged by the real-map policy.
fn assert_wall_invariant(s: &Simulation, op: usize) {
    let boundary = s.boundary.as_deref().unwrap();
    let room = s.room().unwrap();
    for id in s.actor_order() {
        let a = s.actor(id).unwrap();
        let Some(l) = a.life() else { continue };
        assert!(
            room.contains(a.position()),
            "tic {op} type {}",
            l.actor_type.0
        );
        if original_registry::is_impact(l.actor_type) || l.flags & 1 != 0 {
            continue;
        }
        assert!(
            boundary.inside(a.position()),
            "tic {op} type {} at {:?} outside",
            l.actor_type.0,
            a.position().map(Fixed::bits)
        );
        assert!(
            !boundary.box_meets_wall(a.position(), a.radius() - Fixed::from_bits(1)),
            "tic {op} type {} at {:?} meets a wall",
            l.actor_type.0,
            a.position().map(Fixed::bits)
        );
    }
}
fn soak(label: &str, seed: u64) -> usize {
    let lumps = committed(label);
    let mut s = real_map(&lumps).unwrap();
    s.spawn_map_things().unwrap();
    let cases = real_map_evidence::cases();
    let c = cases.iter().find(|c| c.label == label).unwrap();
    arm(&mut s, c.position);
    let mut refused = 0;
    for op in 1..=1200 {
        let before = s.tic();
        let live = s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live;
        let commands = if live {
            scripted(seed, op)
        } else {
            [TicCommand::default(); 4]
        };
        match s.advance(commands) {
            Ok(()) => assert_eq!(s.tic(), before + 1),
            // The two refused legal source behaviors keep the state and the tic.
            Err(SimError::MapScale(MapScaleError::Escape | MapScaleError::Intercepts)) => {
                refused += 1;
                assert_eq!(s.tic(), before);
            }
            Err(e) => panic!("{label} seed {seed} tic {op}: {e}"),
        }
        assert_wall_invariant(&s, op);
    }
    refused
}
#[test]
fn town_soak_keeps_the_wall_invariant_with_the_full_roster_and_pickups() {
    let mut refused = 0;
    for seed in [1, 2, 3, 4] {
        refused += soak("town_fight", seed);
    }
    // No committed room reaches either witness under the scripted inputs.
    assert_eq!(refused, 0);
}
#[test]
fn wide_soak_keeps_the_wall_invariant_along_the_corridor() {
    for seed in [1, 2] {
        assert_eq!(soak("wide_pursuit", seed), 0);
    }
}

/// Local, opt-in: the certificate, the boundary rules, the closed-cell rule and the
/// slide window over every map of the separately authorized IWADs, as counts only,
/// plus the admission time of the E1M1 geometry with its specials and sky stripped
/// in memory (never written). Nothing but the slot and counts is printed.
#[test]
#[ignore = "requires separately authorized assets via DOOM_IWAD_DIR"]
fn private_iwad_real_map_counts() {
    use std::time::Instant;
    let Some(directory) = std::env::var_os("DOOM_IWAD_DIR").filter(|v| !v.is_empty()) else {
        println!("DOOM_IWAD_DIR unset; nothing to count");
        return;
    };
    let (mut maps, mut unparsed, mut certificate, mut boundary, mut blockmap, mut window, mut all) =
        (0, 0, 0, 0, 0, 0, 0);
    let mut e1m1 = None;
    for entry in std::fs::read_dir(directory).expect("asset directory") {
        let path = entry.expect("asset entry").path();
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wad"))
        {
            continue;
        }
        let bytes = std::fs::read(&path).expect("asset bytes");
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("archive");
        for (index, lump) in wad.lumps().iter().enumerate() {
            if !doom_map::is_map_marker(lump.name()) {
                continue;
            }
            let name = lump.name();
            let end = name.as_bytes().iter().position(|b| *b == 0).unwrap_or(8);
            let slot = std::str::from_utf8(&name.as_bytes()[..end]).unwrap();
            maps += 1;
            // The parser's own rules (seg sides, subsector sectors, BSP) come first;
            // a map it refuses is counted, never named beyond this count.
            let Ok(map) = doom_map::Map::from_wad(&wad, slot, doom_map::MapLimits::default())
            else {
                unparsed += 1;
                continue;
            };
            let c = ArithmeticCertificate::derive(&map).is_ok();
            let b = Boundary::derive(&map).is_ok();
            let k = RoomBounds::derive_real_map(&map).is_ok();
            let w = slide_window_lines(&map) <= MAX_INTERCEPTS;
            certificate += usize::from(c);
            boundary += usize::from(b);
            blockmap += usize::from(k);
            window += usize::from(w);
            all += usize::from(c && b && k && w);
            if slot == "E1M1" && index + 10 < wad.lumps().len() {
                let lumps: Vec<(String, Vec<u8>)> = (1..=10)
                    .map(|j| {
                        let l = &wad.lumps()[index + j];
                        let n = l.name();
                        let e = n.as_bytes().iter().position(|b| *b == 0).unwrap_or(8);
                        (
                            std::str::from_utf8(&n.as_bytes()[..e]).unwrap().to_string(),
                            wad.data(index + j).unwrap().to_vec(),
                        )
                    })
                    .collect();
                e1m1 = Some((map, lumps));
            }
        }
    }
    println!(
        "iwad maps {maps}: parser refusals {unparsed}, certificate {certificate}, boundary {boundary}, closed-cell blockmap {blockmap}, slide window {window}, all {all}"
    );
    assert!(maps > 0);
    let Some((map, lumps)) = e1m1 else {
        println!("no E1M1 slot; admission time not measured");
        return;
    };
    let start = Instant::now();
    ArithmeticCertificate::derive(&map).unwrap();
    Boundary::derive(&map).unwrap();
    RoomBounds::derive_real_map(&map).unwrap();
    validate_slide_window(&map).unwrap();
    let geometry = start.elapsed();
    // Specials and sky stripped in memory only; the result is never written.
    let mut lumps = Lumps {
        lumps: std::iter::once(("MAP01".to_string(), Vec::new()))
            .chain(lumps)
            .collect(),
    };
    for line in 0..lumps.line_count() {
        lumps.set_line_special(line, 0);
    }
    for sector in 0..map.sectors().len() {
        let heights = map.sectors()[sector].heights;
        lumps.set_sector(sector, heights, 0, b"TESTCEIL");
    }
    let stripped = lumps.map();
    let start = Instant::now();
    let s = Simulation::original_player_real_map_map(stripped, options()).unwrap();
    let admission = start.elapsed();
    println!(
        "e1m1 geometry checks {} us, full admission {} us, sight budget {}",
        geometry.as_micros(),
        admission.as_micros(),
        s.world.sight_work_budget
    );
}

#[test]
fn pushes_to_items_are_refused_and_pushes_to_monsters_admitted() {
    // Found by the fuzz campaign: a pickup pushed by the controlled momentum input
    // would carry momentum no source path gives an item, which the DSIM33 item
    // witness and the tic preflight refuse. The real-map policy refuses the push
    // itself with unchanged state; a monster push is admitted as before.
    let hall = committed("gallery_monster_shot");
    let mut s = real_map(&hall).unwrap();
    let things = s.spawn_map_things().unwrap().spawned;
    arm(&mut s, [900, 100]);
    let item = things
        .iter()
        .copied()
        .find(|id| s.actor(*id).unwrap().life().unwrap().flags & 1 != 0)
        .unwrap();
    let zombie = things
        .iter()
        .copied()
        .find(|id| s.actor(*id).unwrap().life().unwrap().flags & 1 == 0)
        .unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.set_momentum(item, [Fixed::from_map_units(24); 2]),
        Err(SimError::PlayerBody(PlayerBodyError::Unsupported))
    );
    assert_eq!(s.snapshot(), before);
    s.set_momentum(item, [Fixed::ZERO; 2]).unwrap();
    s.set_momentum(zombie, [Fixed::from_map_units(24); 2])
        .unwrap();
    for _ in 0..8 {
        s.advance([TicCommand::default(); 4]).unwrap();
        let bytes = s
            .snapshot()
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap();
        Diagnostic::decode_real_map(&bytes, DiagnosticLimits::default()).unwrap();
    }
}

#[test]
fn dsim33_armor_events_above_100_points_decode() {
    // Found by review: the reader's version→policy ladder for armor events stopped
    // at 32, so a DSIM33 capture whose green armor exceeded 100 points after armor
    // bonuses (legal under every bonus policy) was refused with Identity.
    let mut hall = committed("gallery_shot");
    hall.clear_things();
    hall.add_thing([-950, 0, 0, 2018, 7]);
    hall.add_thing([-900, 0, 0, 2015, 7]);
    hall.add_thing([-850, 0, 0, 2015, 7]);
    hall.add_thing([-600, 0, -180, CHAINGUY, 7]);
    let mut s = real_map(&hall).unwrap();
    s.spawn_map_things().unwrap();
    arm(&mut s, [-1000, 0]);
    let mut run = [TicCommand::default(); 4];
    run[0].forward = 50;
    for _ in 0..40 {
        s.advance(run).unwrap();
    }
    let armor = s.original_player_armor(PlayerSlot::One).unwrap();
    assert!(
        armor.points > 100,
        "green armor plus two bonuses: {armor:?}"
    );
    // The chaingunner ahead fires bursts at the armored player; every capture,
    // including the tic that carries the armor event, must decode through the
    // v33 reader.
    let mut absorbed = false;
    for _ in 0..600 {
        s.advance([TicCommand::default(); 4]).unwrap();
        let bytes = s
            .snapshot()
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap();
        let view = Diagnostic::decode_real_map(&bytes, DiagnosticLimits::default())
            .unwrap_or_else(|e| panic!("tic {}: {e:?}", s.tic()));
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        if s.original_player_armor(PlayerSlot::One).unwrap().points < armor.points {
            absorbed = true;
            break;
        }
    }
    assert!(
        absorbed,
        "the zombie's shots never reached the armored player"
    );
}
