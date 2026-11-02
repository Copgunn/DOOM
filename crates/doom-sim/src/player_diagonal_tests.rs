//! M3h5 non-axis (diagonal) walls: source replay of every committed scenario, the
//! diagonal admission (closed boundary, closed-cell BLOCKMAP, spawn parity), the
//! sliver quirks of `P_PointOnLineSide`/`P_BoxOnLineSide`, the escape witness and
//! the recoverable paths. Expected behavior comes from the original source reading
//! in M3H5_REFERENCE_INVESTIGATION.md, never from Rust output.
use super::*;
use crate::diagonal_room::Lumps;
use crate::geometry::LineGeometry;

// Source replay: every committed scenario against the compiled original rows,
// the independent DSIM32 captures and ordered events; step equals advance.
// `corner_jump` is the one refused legal source move: tic 1 fails with Escape.
#[test]
fn diagonal_source_case_00_slide_45() {
    diagonal_evidence::verify_case(0);
}
#[test]
fn diagonal_source_case_01_slide_22() {
    diagonal_evidence::verify_case(1);
}
#[test]
fn diagonal_source_case_02_slide_shallow() {
    diagonal_evidence::verify_case(2);
}
#[test]
fn diagonal_source_case_03_convex_corner() {
    diagonal_evidence::verify_case(3);
}
#[test]
fn diagonal_source_case_04_concave_corner() {
    diagonal_evidence::verify_case(4);
}
#[test]
fn diagonal_source_case_05_fireball_diagonal_wall() {
    diagonal_evidence::verify_case(5);
}
#[test]
fn diagonal_source_case_06_puff_diagonal_wall() {
    diagonal_evidence::verify_case(6);
}
#[test]
fn diagonal_source_case_07_sight_past_pillar() {
    diagonal_evidence::verify_case(7);
}
#[test]
fn diagonal_source_case_08_zombie_around_pillar() {
    diagonal_evidence::verify_case(8);
}
#[test]
fn diagonal_source_case_09_imp_around_pillar() {
    diagonal_evidence::verify_case(9);
}
#[test]
fn diagonal_source_case_10_corridor_pursuit() {
    diagonal_evidence::verify_case(10);
}
#[test]
fn diagonal_source_case_11_corridor_slide_east() {
    diagonal_evidence::verify_case(11);
}
#[test]
fn diagonal_source_case_12_corridor_slide_west() {
    diagonal_evidence::verify_case(12);
}
#[test]
fn diagonal_source_case_13_riser_slide_puff() {
    diagonal_evidence::verify_case(13);
}
#[test]
fn diagonal_source_case_14_window_slide_low() {
    diagonal_evidence::verify_case(14);
}
#[test]
fn diagonal_source_case_15_window_slide_high() {
    diagonal_evidence::verify_case(15);
}
#[test]
fn diagonal_source_case_16_window_upper_puff() {
    diagonal_evidence::verify_case(16);
}
#[test]
fn diagonal_source_case_17_autoaim_up_window() {
    diagonal_evidence::verify_case(17);
}
#[test]
fn diagonal_source_case_18_autoaim_down_window() {
    diagonal_evidence::verify_case(18);
}
#[test]
fn diagonal_source_case_19_noise_diagonal_window() {
    diagonal_evidence::verify_case(19);
}
#[test]
fn diagonal_source_case_20_corner_jump_is_refused_at_tic_1() {
    let (_, captures, _, _, compared) = diagonal_evidence::verify_case(20);
    // Only the spawn capture is compared; the push tic is refused and the 39
    // committed tics after it, with the player in the void, are never reached.
    assert_eq!((captures, compared), (1, 1));
}
#[test]
fn diagonal_source_case_21_corner_no_jump() {
    let (_, _, _, _, compared) = diagonal_evidence::verify_case(21);
    assert_eq!(compared, 41);
}

const WEST: i16 = -180;
const POSSESSED: i16 = 3004;
const BLOCKMONSTERS: u16 = 2;
fn options() -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(6, 0),
        respawn_monsters: false,
    }
}
fn committed(label: &str) -> Lumps {
    let cases = diagonal_evidence::cases();
    let c = cases.iter().find(|c| c.label == label).unwrap();
    Lumps::parse(diagonal_evidence::MAP_BYTES[c.map])
}
fn diagonal(lumps: &Lumps) -> Result<Simulation, OriginalRegistryError> {
    Simulation::original_player_diagonal_map(lumps.map(), options())
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

#[test]
fn diagonal_admits_every_committed_room_and_spawn() {
    // Every committed map loads under the policy and every case's spawns are
    // admitted; the older policies reject the same maps at their axis rule.
    for c in diagonal_evidence::cases() {
        let (s, _, things) = diagonal_evidence::setup(&c);
        assert!(s.actor_order().count() > things.len(), "{}", c.label);
        rejects(
            Simulation::original_player_heights_map(diagonal_evidence::case_map(c.map), options()),
            SimError::MapScale(MapScaleError::Geometry),
        );
    }
}

#[test]
fn diagonal_rejects_geometry_outside_the_proven_domain_at_load() {
    // C3: a wall whose front side faces away from its room.
    let mut wedge = committed("slide_45");
    let wall = wedge.find_line([-256, 768], [256, 256]);
    wedge.reverse_line(wall);
    rejects(
        diagonal(&wedge),
        SimError::MapScale(MapScaleError::Boundary),
    );
    // C2: a wall shortened to a new end leaves two odd vertices (no crossing).
    let mut wedge = committed("slide_45");
    let wall = wedge.find_line([-256, 768], [256, 256]);
    let end = wedge.push_vertex([0, 512]);
    wedge.set_line_vertex(wall, 1, end);
    rejects(
        diagonal(&wedge),
        SimError::MapScale(MapScaleError::Boundary),
    );
    // C1: the tip vertex moved so that both tip walls cross the horizontal wall.
    let mut wedge = committed("slide_45");
    let tip_wall = wedge.find_line([768, 256], [1024, 0]);
    let tip = wedge.line_vertices(tip_wall)[1];
    wedge.set_vertex(tip, [600, 400]);
    rejects(
        diagonal(&wedge),
        SimError::MapScale(MapScaleError::Boundary),
    );
    // C1: two one-sided walls overlapping along a line (the pillar's north face
    // stretched over the room's top wall).
    let mut pillar = committed("sight_past_pillar");
    let face = pillar.find_line([64, 128], [-64, 128]);
    let [a, b] = pillar.line_vertices(face);
    pillar.set_vertex(a, [512, 512]);
    pillar.set_vertex(b, [-512, 512]);
    rejects(
        diagonal(&pillar),
        SimError::MapScale(MapScaleError::Boundary),
    );
    // A degenerate line.
    let mut wedge = committed("slide_45");
    let wall = wedge.find_line([-256, 768], [256, 256]);
    let [a, _] = wedge.line_vertices(wall);
    wedge.set_line_vertex(wall, 1, a);
    rejects(
        diagonal(&wedge),
        SimError::MapScale(MapScaleError::Geometry),
    );
    // A forbidden flag bit on a diagonal wall, and an inconsistent two-sided pair.
    let mut wedge = committed("slide_45");
    let wall = wedge.find_line([-256, 768], [256, 256]);
    wedge.set_line_flags(wall, 1 | 1024);
    rejects(
        diagonal(&wedge),
        SimError::MapScale(MapScaleError::Geometry),
    );
    let mut step = committed("riser_slide_puff");
    let riser = step.find_line([-384, -384], [384, 384]);
    step.set_line_flags(riser, 1);
    rejects(diagonal(&step), SimError::MapScale(MapScaleError::Geometry));
    // Every admitted flag bit on a diagonal wall and on a diagonal two-sided line.
    let mut wedge = committed("slide_45");
    let wall = wedge.find_line([-256, 768], [256, 256]);
    wedge.set_line_flags(wall, HEIGHTS_LINE_FLAGS);
    diagonal(&wedge).unwrap();
    let mut step = committed("riser_slide_puff");
    let riser = step.find_line([-384, -384], [384, 384]);
    step.set_line_flags(riser, 4 | HEIGHTS_LINE_FLAGS);
    diagonal(&step).unwrap();
    // The closed-cell BLOCKMAP rule: a diagonal wall missing from a cell it passes.
    let mut wedge = committed("slide_45");
    let wall = wedge.find_line([-256, 768], [256, 256]);
    wedge.unlist_line(wall);
    rejects(
        diagonal(&wedge),
        SimError::MapScale(MapScaleError::Blockmap),
    );
    let mut corridor = committed("corridor_pursuit");
    let piece = corridor.find_line([-384, -288], [0, 96]);
    corridor.unlist_line(piece);
    rejects(
        diagonal(&corridor),
        SimError::MapScale(MapScaleError::Blockmap),
    );
    // Line 0 is listed by every cell's leading 0 (p_maputl.c P_BlockLinesIterator
    // walks the list from its first word): unlisting it changes nothing.
    let mut step = committed("riser_slide_puff");
    assert_eq!(step.find_line([-384, -384], [384, 384]), 0);
    step.unlist_line(0);
    diagonal(&step).unwrap();
    // Line and sector specials, a sky ceiling, heights beyond ±512.
    let mut wedge = committed("slide_45");
    wedge.set_line_special(0, 1);
    rejects(diagonal(&wedge), SimError::Pursuit(PursuitError::Specials));
    let mut wedge = committed("slide_45");
    wedge.set_sector(0, [0, 128], 9, b"TESTCEIL");
    rejects(diagonal(&wedge), SimError::Pursuit(PursuitError::Specials));
    let mut wedge = committed("slide_45");
    wedge.set_sector(0, [0, 128], 0, b"F_SKY1\0\0");
    rejects(diagonal(&wedge), SimError::Combat(CombatError::Sky));
    let mut wedge = committed("slide_45");
    wedge.set_sector(0, [0, 513], 0, b"TESTCEIL");
    rejects(
        diagonal(&wedge),
        SimError::Perception(PerceptionError::Arithmetic),
    );
    // The heights and map-scale policies keep their own rules on the same bytes.
    rejects(
        Simulation::original_player_heights_map(committed("slide_45").map(), options()),
        SimError::MapScale(MapScaleError::Geometry),
    );
    rejects(
        Simulation::original_player_map_scale_map(committed("slide_45").map(), options()),
        SimError::PlayerBody(PlayerBodyError::Unsupported),
    );
}

#[test]
fn diagonal_rejects_spawns_whose_box_meets_a_wall_or_whose_center_is_outside() {
    // A THING whose closed box touches the 45° wall, one in the void beyond it and
    // one outside the vertex rectangle: the batch rejects before any spawn.
    for thing in [
        [-100, 580, 0, POSSESSED, 7],
        [-100, 700, 0, POSSESSED, 7],
        [-1100, 0, 0, POSSESSED, 7],
    ] {
        let mut wedge = committed("slide_45");
        wedge.clear_things();
        wedge.add_thing(thing);
        let mut s = diagonal(&wedge).unwrap();
        let before = s.snapshot();
        assert_eq!(
            s.spawn_map_things().map(|_| ()),
            Err(SimError::Monster(MonsterError::Arithmetic)),
            "{thing:?}"
        );
        assert_eq!(s.snapshot(), before);
    }
    // The same positions for the player body and the controlled spawns.
    let mut wedge = committed("slide_45");
    wedge.clear_things();
    let mut s = diagonal(&wedge).unwrap();
    for position in [[-100, 580], [-100, 700], [-1100, 0]] {
        let before = s.snapshot();
        assert_eq!(
            s.spawn_player_body(PlayerSlot::One, position.map(Fixed::from_map_units))
                .map(|_| ()),
            Err(SimError::PlayerBody(PlayerBodyError::Binding)),
            "{position:?}"
        );
        assert_eq!(s.snapshot(), before);
    }
    // A box 20 units from the wall is admitted (the committed slide_45 start).
    arm(&mut s, [-100, 560]);
}

#[test]
fn sliver_reading_quirks_of_a_diagonal_wall() {
    // P_PointOnLineSide on the (1,1)-direction wall through the origin: a point
    // exactly on the line reads back (1); a point in front (right of the
    // direction, cross >= 1 unit²) reads front (0); a point in front by less than
    // 1/L units reads back because both products floor to the same value.
    let wall = LineGeometry::new(
        [Fixed::ZERO; 2],
        [Fixed::from_map_units(1), Fixed::from_map_units(1)],
    )
    .unwrap();
    let unit = |x: i32, y: i32| [Fixed::from_bits(x), Fixed::from_bits(y)];
    assert_eq!(wall.point_side(unit(65536, 65536)), 1);
    assert_eq!(wall.point_side(unit(2 * 65536, 0)), 0);
    // cross = dy_u·Δx − dx_u·Δy = 0.5 − 0 = 0.5 unit²: in front, reads back.
    assert_eq!(wall.point_side(unit(32768, 0)), 1);
    // A box whose top-left corner lies exactly on the wall and whose bottom-right
    // corner is in front is read as crossing (-1); the mirror box, corner on the
    // wall and the other corner behind, reads as one side (back).
    // The front side of the (1,1) wall is below-right. A box there whose top-left
    // corner (0,0) lies exactly on the wall: that corner reads back, the opposite
    // corner (16,-16) front, so the box is read as crossing (-1) and blocked. The
    // mirror box behind the wall with its bottom-right corner on it reads one side.
    let r = Fixed::from_map_units(8);
    assert_eq!(
        wall.box_side([Fixed::from_map_units(8), Fixed::from_map_units(-8)], r),
        -1
    );
    assert_eq!(
        wall.box_side([Fixed::from_map_units(-8), Fixed::from_map_units(8)], r),
        1
    );
    // From the back side a box may stop with its leading corner less than 1/L past
    // the line: a corner 0.25 unit² in front (perpendicular 0.18 units) still reads
    // back and the box is read as one side; at 1 unit² it reads front and crosses.
    let raw = |x: i32, y: i32| [Fixed::from_bits(x), Fixed::from_bits(y)];
    assert_eq!(wall.box_side(raw(-8 * 65536 + 16384, 8 * 65536), r), 1);
    assert_eq!(wall.box_side(raw(-7 * 65536, 8 * 65536), r), -1);
}

#[test]
fn boundary_parity_and_box_tests_are_exact() {
    let wedge = committed("slide_45");
    let s = diagonal(&wedge).unwrap();
    let b = s.boundary.as_deref().unwrap();
    assert_eq!(b.segments().len(), 12);
    let at = |x: i16, y: i16| [Fixed::from_map_units(x), Fixed::from_map_units(y)];
    // Inside the room, in the vertex rectangle's corner outside the 45° wall, and
    // on the far side of the tip.
    assert!(b.inside(at(0, 0)));
    assert!(!b.inside(at(200, 700)));
    assert!(!b.inside(at(1023, 100)));
    // A raw step across the 5:1 wall flips the parity.
    let on_wall = [
        Fixed::from_map_units(0),
        Fixed::from_bits(-(769 * 65536) - 39322),
    ];
    assert!(b.inside([on_wall[0], on_wall[1] + Fixed::from_bits(1)]));
    assert!(!b.inside([on_wall[0], on_wall[1] - Fixed::from_bits(1)]));
    // The box test is closed: touching the wall by one raw unit counts.
    let r = Fixed::from_map_units(16);
    assert!(!b.box_meets_wall(at(-100, 560), r));
    assert!(b.box_meets_wall(at(-100, 580), r));
    assert!(!b.box_meets_wall([at(-100, 580)[0], Fixed::from_bits(580 * 65536 - 1)], r));
}

#[test]
fn escape_witness_is_recoverable_and_older_policies_refuse_the_push() {
    // The committed corner room: a push of (30, 30) at (-31, -20) jumps the corner
    // in one projected slide; the tic is refused and every field is unchanged, then
    // the same state continues under an admitted push.
    let mut corner = committed("corner_jump");
    corner.clear_things();
    let mut s = diagonal(&corner).unwrap();
    let body = arm(&mut s, [-31, -20]);
    let push = [Fixed::from_map_units(30), Fixed::from_map_units(30)];
    s.set_momentum(body, push).unwrap();
    let before = s.snapshot();
    let events = s.events().len();
    assert_eq!(
        s.advance([TicCommand::default(); 4]),
        Err(SimError::MapScale(MapScaleError::Escape))
    );
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.events().len(), events);
    // A push the proof covers (within 30 per axis after projection) is admitted
    // and keeps the center inside: the same start with (24, 24).
    s.set_momentum(body, [Fixed::from_map_units(24); 2])
        .unwrap();
    s.advance([TicCommand::default(); 4]).unwrap();
    let a = s.actor(body).unwrap();
    assert!(s.boundary.as_deref().unwrap().inside(a.position()));
    // The push is a controlled input of this policy only.
    let heights = Simulation::original_player_heights_map(
        crate::heights_room::HeightsRoom::new(&[-512, 512], &[[0, 128]]).map(),
        options(),
    )
    .unwrap();
    let mut heights = heights;
    let body = arm(&mut heights, [0, 0]);
    assert_eq!(
        heights.set_momentum(body, push),
        Err(SimError::PlayerBody(PlayerBodyError::Unsupported))
    );
}

#[test]
fn diagonal_policy_identity_is_distinct_from_the_older_policies() {
    let profile = OriginalExecutionProfile::PlayerMonstersDiagonalV1;
    assert_eq!(profile.wire_id(), 16);
    assert_eq!(profile.context_schema(), PLAYER_DIAGONAL_CONTEXT_SCHEMA);
    assert_eq!(
        PLAYER_DIAGONAL_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-DIAGONAL-1-MEDIUM-v1"
    );
    assert!(profile.has_heights() && profile.has_diagonals() && profile.has_map_scale());
    assert!(!OriginalExecutionProfile::PlayerMonstersHeightsV1.has_diagonals());
    let wedge = committed("slide_45");
    let s = diagonal(&wedge).unwrap();
    let bytes = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let view = Diagnostic::decode_diagonal(&bytes, DiagnosticLimits::default()).unwrap();
    assert_eq!(view.field("profile"), Some(&16u32.to_le_bytes()[..]));
    assert_eq!(
        view.field("context.schema"),
        Some(PLAYER_DIAGONAL_CONTEXT_SCHEMA)
    );
    assert!(Diagnostic::decode_heights(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_map_scale(&bytes, DiagnosticLimits::default()).is_err());
    // A DSIM31 heights capture is not a DSIM32 one.
    let heights = Simulation::original_player_heights_map(
        crate::heights_room::HeightsRoom::new(&[-512, 512], &[[0, 128]]).map(),
        options(),
    )
    .unwrap();
    let bytes = heights
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    assert!(Diagnostic::decode_diagonal(&bytes, DiagnosticLimits::default()).is_err());
    Diagnostic::decode_heights(&bytes, DiagnosticLimits::default()).unwrap();
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
/// Invariant Ω of the M3h5 note after every tic: every XY mover (the player,
/// monsters, corpses, fireballs) has its center inside Ω and its open box meets no
/// wall; every other live actor has its center in the rectangle.
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
        // The open box: shrink the closed box by one raw unit per side.
        assert!(
            !boundary.box_meets_wall(a.position(), a.radius() - Fixed::from_bits(1)),
            "tic {op} type {} at {:?} meets a wall",
            l.actor_type.0,
            a.position().map(Fixed::bits)
        );
    }
}
fn soak(label: &str, things: &[[i16; 5]], start: [i16; 2], seed: u64) {
    let mut lumps = committed(label);
    lumps.clear_things();
    for t in things {
        lumps.add_thing(*t);
    }
    let mut s = diagonal(&lumps).unwrap();
    s.spawn_map_things().unwrap();
    arm(&mut s, start);
    for op in 1..=1200 {
        let before = s.tic();
        // A dead player admits only zero commands, as the body contract documents.
        let live = s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live;
        let commands = if live {
            scripted(seed, op)
        } else {
            [TicCommand::default(); 4]
        };
        s.advance(commands)
            .unwrap_or_else(|e| panic!("{label} seed {seed} tic {op}: {e}"));
        assert_eq!(s.tic(), before + 1);
        assert_wall_invariant(&s, op);
    }
}
#[test]
fn wedge_soak_keeps_the_wall_invariant_with_the_full_roster() {
    let roster = [
        [-900, 300, 0, 3001, 7],
        [-800, -300, 0, 3004, 7],
        [-700, 500, 0, 9, 7],
        [-600, -600, 0, 65, 7],
        [-400, 0, 0, 3002, 7],
        [600, -100, WEST, 3001, 7],
        [500, -300, WEST, 3004, 7],
        [-200, -700, 0, 3002, 7],
    ];
    for seed in [1, 2, 3, 4] {
        soak("slide_45", &roster, [-100, 560], seed);
    }
}
#[test]
fn pillar_soak_keeps_the_wall_invariant_with_the_full_roster() {
    let roster = [
        [300, 300, WEST, 3001, 7],
        [300, -300, WEST, 3004, 7],
        [-300, 300, 0, 9, 7],
        [-300, -300, 0, 65, 7],
        [400, 0, WEST, 3002, 7],
        [0, 400, 0, 3001, 7],
        [0, -400, 0, 3004, 7],
        [-400, 0, 0, 3002, 7],
    ];
    for seed in [1, 2, 3, 4] {
        soak("sight_past_pillar", &roster, [-300, 0], seed);
    }
}
#[test]
fn blockmonsters_and_soundblock_bits_load_on_diagonal_lines() {
    let mut step = committed("riser_slide_puff");
    let riser = step.find_line([-384, -384], [384, 384]);
    step.set_line_flags(riser, 4 | BLOCKMONSTERS | 64);
    diagonal(&step).unwrap();
}
