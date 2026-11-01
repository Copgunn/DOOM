//! M3h4 two-sided lines, static heights and vertical movement: source replay of
//! every committed scenario, the heights admission, the vertical invariant and the
//! recoverable paths. Expected behavior comes from the original source reading in
//! M3H4_REFERENCE_INVESTIGATION.md, never from Rust output.
use super::*;
use crate::heights_room::{Fault, HeightsRoom};
use crate::identity::OriginalState;

// Source replay: every committed scenario against the compiled original rows,
// the independent DSIM31 captures and ordered events; step equals advance.
#[test]
fn heights_source_case_00_stairs_up() {
    heights_evidence::verify_case(0);
}
#[test]
fn heights_source_case_01_stairs_down() {
    heights_evidence::verify_case(1);
}
#[test]
fn heights_source_case_02_too_high_step() {
    heights_evidence::verify_case(2);
}
#[test]
fn heights_source_case_03_imp_pursuit_stairs() {
    heights_evidence::verify_case(3);
}
#[test]
fn heights_source_case_04_demon_pursuit_stairs() {
    heights_evidence::verify_case(4);
}
#[test]
fn heights_source_case_05_monster_refuses_riser() {
    heights_evidence::verify_case(5);
}
#[test]
fn heights_source_case_06_autoaim_up_step() {
    heights_evidence::verify_case(6);
}
#[test]
fn heights_source_case_07_autoaim_down_step() {
    heights_evidence::verify_case(7);
}
#[test]
fn heights_source_case_08_drop_landing_oof() {
    heights_evidence::verify_case(8);
}
#[test]
fn heights_source_case_09_monster_refuses_ledge() {
    heights_evidence::verify_case(9);
}
#[test]
fn heights_source_case_10_corpse_pushed_off_ledge() {
    heights_evidence::verify_case(10);
}
#[test]
fn heights_source_case_11_fall_forward_from_below() {
    heights_evidence::verify_case(11);
}
#[test]
fn heights_source_case_12_fireball_down_ledge_floor() {
    heights_evidence::verify_case(12);
}
#[test]
fn heights_source_case_13_window_sight_pit_then_step() {
    heights_evidence::verify_case(13);
}
#[test]
fn heights_source_case_14_window_lower_wall_puff() {
    heights_evidence::verify_case(14);
}
#[test]
fn heights_source_case_15_window_upper_wall_puff() {
    heights_evidence::verify_case(15);
}
#[test]
fn heights_source_case_16_noise_through_window() {
    heights_evidence::verify_case(16);
}
#[test]
fn heights_source_case_17_noise_through_soundblock() {
    heights_evidence::verify_case(17);
}
#[test]
fn heights_source_case_18_reject_bit() {
    heights_evidence::verify_case(18);
}
#[test]
fn heights_source_case_19_closed_opening_blocks_all() {
    heights_evidence::verify_case(19);
}
#[test]
fn heights_source_case_20_fence_slide() {
    heights_evidence::verify_case(20);
}
#[test]
fn heights_source_case_21_fence_blocks_monsters_only() {
    heights_evidence::verify_case(21);
}
#[test]
fn heights_source_case_22_crawlspace_refusal() {
    heights_evidence::verify_case(22);
}
#[test]
fn heights_source_case_23_fireball_up_ceiling() {
    heights_evidence::verify_case(23);
}

// Rust-only policy tests for PlayerMonstersHeightsV1: admission, spawn fit, the
// vertical invariant and momz witnesses, recoverable paths, soaks and the reader.

const IMP: i16 = 3001;
const POSSESSED: i16 = 3004;
const SHOTGUY: i16 = 9;
const WEST: i16 = -180;
const STAIRS_SPANS: [i16; 6] = [-768, -512, -256, 0, 256, 1024];
const STAIRS_HEIGHTS: [[i16; 2]; 5] = [[97, 256], [72, 256], [48, 256], [24, 256], [0, 256]];
const LEDGE_SPANS: [i16; 4] = [-1024, 0, 64, 1024];
const LEDGE_HEIGHTS: [[i16; 2]; 3] = [[96, 256], [0, 256], [0, 100]];
const WINDOW_SPANS: [i16; 5] = [-1024, -64, 64, 512, 1024];
const WINDOW_HEIGHTS: [[i16; 2]; 4] = [[0, 128], [40, 88], [-64, 256], [64, 256]];
const CLOSED_SPANS: [i16; 4] = [-512, 0, 64, 512];
const CLOSED_HEIGHTS: [[i16; 2]; 3] = [[0, 256], [128, 128], [0, 256]];
const CRAWL_SPANS: [i16; 3] = [-512, 0, 512];
const CRAWL_HEIGHTS: [[i16; 2]; 2] = [[0, 128], [0, 32]];

fn stairs() -> HeightsRoom {
    HeightsRoom::new(&STAIRS_SPANS, &STAIRS_HEIGHTS)
}
fn ledge() -> HeightsRoom {
    HeightsRoom::new(&LEDGE_SPANS, &LEDGE_HEIGHTS)
}
fn window() -> HeightsRoom {
    HeightsRoom::new(&WINDOW_SPANS, &WINDOW_HEIGHTS)
}
fn heights(map: doom_map::Map) -> Simulation {
    Simulation::original_player_heights_map(
        map,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(6, 0),
            respawn_monsters: false,
        },
    )
    .unwrap()
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
/// Invariant V of the M3h4 note at a tic boundary: every actor that is not an
/// impact satisfies `floor <= z <= ceiling - height` with its own floor/ceiling.
fn assert_vertical_invariant(s: &Simulation, op: usize) {
    for id in s.actor_order() {
        let a = s.actor(id).unwrap();
        let Some(l) = a.life() else { continue };
        if original_registry::is_impact(l.actor_type) {
            continue;
        }
        assert!(
            a.floor <= a.z && a.z + a.height() <= a.ceiling,
            "tic {op} type {} z {} floor {} ceiling {} height {}",
            l.actor_type.0,
            a.z.bits(),
            a.floor.bits(),
            a.ceiling.bits(),
            a.height().bits()
        );
    }
}
/// Every tic must succeed, advance the counter by one and keep the invariant.
fn progresses(
    s: &mut Simulation,
    mut command: impl FnMut(&Simulation, usize) -> [TicCommand; 4],
    tics: usize,
) {
    for op in 1..=tics {
        let before = s.tic();
        let commands = command(s, op);
        s.advance(commands)
            .unwrap_or_else(|e| panic!("tic {op} rejected under the heights policy: {e}"));
        assert_eq!(s.tic(), before + 1);
        assert_vertical_invariant(s, op);
    }
}
fn rejects(result: Result<Simulation, OriginalRegistryError>, expected: SimError) {
    match result {
        Err(OriginalRegistryError::Runtime(error)) => assert_eq!(error, expected),
        Err(other) => panic!("unexpected error {other:?}, expected {expected:?}"),
        Ok(_) => panic!("admitted, expected {expected:?}"),
    }
}

#[test]
fn heights_room_mirrors_the_committed_geometry() {
    // The Rust builder and the Python builder produce identical bytes for the
    // committed rooms, so rejection variants built here share their geometry.
    let cases = heights_evidence::cases();
    let committed = |which: usize| heights_evidence::MAP_BYTES[cases[which].map];
    let stairs = stairs().with_things(&[[-400, 0, 0, IMP, 7]]);
    assert_eq!(stairs.wad(), committed(3));
    let sight = window().with_things(&[[300, 0, WEST, POSSESSED, 7], [700, 0, WEST, POSSESSED, 7]]);
    assert_eq!(sight.wad(), committed(13));
    let soundblock = window()
        .with_portals(&[(1, 64), (2, 64)])
        .with_things(&[[300, 0, WEST, POSSESSED, 7]]);
    assert_eq!(soundblock.wad(), committed(17));
    let mut rejected = window().with_things(&[[700, 0, WEST, POSSESSED, 7]]);
    rejected.reject = vec![[0, 3]];
    assert_eq!(rejected.wad(), committed(18));
    let fence = HeightsRoom::new(&[-512, 0, 512], &[[0, 256], [0, 256]])
        .with_portals(&[(1, 1)])
        .with_things(&[[300, 0, WEST, IMP, 7]]);
    assert_eq!(fence.wad(), committed(20));
}

#[test]
fn heights_rejects_geometry_outside_the_proven_domain_at_load() {
    let options = MapContextOptions::default();
    let make = |room: &HeightsRoom| Simulation::original_player_heights_map(room.map(), options);
    make(&stairs()).unwrap();
    make(&ledge()).unwrap();
    make(&window()).unwrap();
    make(&HeightsRoom::new(&CLOSED_SPANS, &CLOSED_HEIGHTS)).unwrap();
    make(&HeightsRoom::new(&CRAWL_SPANS, &CRAWL_HEIGHTS)).unwrap();
    // A diagonal two-sided line, a forbidden flag bit, an inconsistent two-sided
    // pair: the heights geometry check.
    let portal = stairs().map().linedefs().len() - 4;
    for fault in [
        Fault::DiagonalPortal(1),
        Fault::Flags(portal, 4 | 512),
        Fault::Flags(0, 1 | 1024),
        Fault::InconsistentTwoSided(portal),
    ] {
        rejects(
            make(&stairs().with_fault(fault)),
            SimError::MapScale(MapScaleError::Geometry),
        );
    }
    // Every admitted flag bit on a one-sided wall and on a two-sided line.
    make(&stairs().with_fault(Fault::Flags(0, HEIGHTS_LINE_FLAGS))).unwrap();
    make(&stairs().with_fault(Fault::Flags(portal, 4 | HEIGHTS_LINE_FLAGS))).unwrap();
    // A two-sided boundary line without a one-sided covering wall.
    rejects(
        make(&stairs().with_fault(Fault::TwoSidedBoundary)),
        SimError::MapScale(MapScaleError::Walls),
    );
    // A two-sided line listed in no BLOCKMAP cell.
    rejects(
        make(&stairs().with_fault(Fault::Unlisted(portal))),
        SimError::MapScale(MapScaleError::Blockmap),
    );
    // Line and sector specials, a sky ceiling, heights beyond ±512 and an inverted sector.
    rejects(
        make(&stairs().with_fault(Fault::LineSpecial(portal))),
        SimError::Pursuit(PursuitError::Specials),
    );
    rejects(
        make(&stairs().with_fault(Fault::SectorSpecial(2))),
        SimError::Pursuit(PursuitError::Specials),
    );
    rejects(
        make(&stairs().with_fault(Fault::Sky(2))),
        SimError::Combat(CombatError::Sky),
    );
    rejects(
        make(&stairs().with_fault(Fault::Heights(0, [97, 513]))),
        SimError::Perception(PerceptionError::Arithmetic),
    );
    assert!(make(&stairs().with_fault(Fault::Heights(2, [100, 50]))).is_err());
    // More linedefs than one hitscan admits: 40 sectors give 121 lines.
    let many = HeightsRoom::new(
        &(0..=40).map(|i| -1024 + 50 * i).collect::<Vec<_>>(),
        &[[0, 256]; 40],
    );
    assert_eq!(many.map().linedefs().len(), 121);
    rejects(make(&many), SimError::MapScale(MapScaleError::Lines));
    // The historical map-scale policy still admits only the flat single sector.
    rejects(
        Simulation::original_player_map_scale_map(stairs().map(), options),
        SimError::PlayerBody(PlayerBodyError::Unsupported),
    );
    // Closed-door sectors (floor == ceiling) load as static walls.
    make(&HeightsRoom::new(
        &[-512, 0, 64, 512],
        &[[0, 256], [64, 64], [0, 256]],
    ))
    .unwrap();
}

#[test]
fn heights_rejects_spawns_that_do_not_fit_their_sector() {
    // A THING in the 48-unit window and in a closed sector: the batch rejects it
    // before any spawn and the state is unchanged.
    for room in [
        window().with_things(&[[0, 0, 0, POSSESSED, 7], [300, 0, 0, POSSESSED, 7]]),
        HeightsRoom::new(&CLOSED_SPANS, &CLOSED_HEIGHTS).with_things(&[[32, 0, 0, POSSESSED, 7]]),
    ] {
        let mut s = heights(room.map());
        let before = s.snapshot();
        assert_eq!(
            s.spawn_map_things().unwrap_err(),
            SimError::MapScale(MapScaleError::Fit)
        );
        assert_eq!(s.snapshot(), before);
    }
    // The player body in the 32-unit crawlspace.
    let mut s = heights(HeightsRoom::new(&CRAWL_SPANS, &CRAWL_HEIGHTS).map());
    let before = s.snapshot();
    assert_eq!(
        s.spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(200), Fixed::ZERO])
            .unwrap_err(),
        SimError::MapScale(MapScaleError::Fit)
    );
    assert_eq!(s.snapshot(), before);
    arm(&mut s, [-200, 0]);
    // A dropped pickup into a closed sector; the same drop fits the 48-unit window.
    let mut s = heights(HeightsRoom::new(&CLOSED_SPANS, &CLOSED_HEIGHTS).map());
    let before = s.snapshot();
    assert_eq!(
        s.spawn_dropped_pickup(PickupKind::Clip, [Fixed::from_map_units(32), Fixed::ZERO])
            .unwrap_err(),
        SimError::MapScale(MapScaleError::Fit)
    );
    assert_eq!(s.snapshot(), before);
    let mut s = heights(window().map());
    s.spawn_dropped_pickup(PickupKind::Clip, [Fixed::ZERO, Fixed::ZERO])
        .unwrap();
    arm(&mut s, [-300, 0]);
    progresses(&mut s, |_, _| [TicCommand::default(); 4], 30);
}

#[test]
fn vertical_witnesses_are_recoverable_rejections_with_full_rollback() {
    // Unreachable by the vertical invariant; in-crate corruption shows the witness
    // checks roll the tic back instead of continuing with unproven arithmetic.
    let mut s = heights(stairs().with_things(&[[-400, 0, 0, POSSESSED, 7]]).map());
    let zombie = s.spawn_map_things().unwrap().spawned[0];
    arm(&mut s, [900, 0]);
    s.advance([TicCommand::default(); 4]).unwrap();
    for (z, momz) in [
        (Fixed::from_map_units(300), Fixed::ZERO),
        (Fixed::from_map_units(60), Fixed::ZERO),
        (
            Fixed::from_map_units(72),
            Fixed::from_map_units(HEIGHTS_MOMZ_UNITS as i16 + 1),
        ),
        (
            Fixed::from_map_units(72),
            -Fixed::from_map_units(HEIGHTS_MOMZ_UNITS as i16 + 1),
        ),
    ] {
        let mut corrupt = s.clone();
        let a = corrupt.actors.get_mut(zombie).unwrap();
        a.z = z;
        a.momz = momz;
        let before = corrupt.clone();
        assert_eq!(
            corrupt.advance([TicCommand::default(); 4]),
            Err(SimError::MapScale(MapScaleError::Vertical))
        );
        assert_eq!(corrupt.snapshot(), before.snapshot());
        assert_eq!(corrupt.rng(), before.rng());
    }
    // A monster off its floor with a momz inside the witness is admitted and falls.
    let a = s.actors.get_mut(zombie).unwrap();
    a.z = Fixed::from_map_units(120);
    progresses(&mut s, |_, _| [TicCommand::default(); 4], 20);
    assert_eq!(s.actor(zombie).unwrap().z, Fixed::from_map_units(72));
}

#[test]
fn fireball_chord_momz_beyond_the_historical_witness_is_admitted() {
    // P_SpawnMissile: momz = (dest.z - source.z) / (distance / speed). An Imp 968
    // units below a player 80 units away launches with momz 121, beyond the
    // historical 64-unit witness and inside the derived 1,024-unit one.
    let room = HeightsRoom::new(&[-512, 0, 512], &[[-512, 512], [456, 512]])
        .with_things(&[[-40, 0, 0, IMP, 7]]);
    let mut s = heights(room.map());
    let imp = s.spawn_map_things().unwrap().spawned[0];
    let body = arm(&mut s, [40, 0]);
    s.advance([TicCommand::default(); 4]).unwrap();
    s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, imp, body))
        .unwrap()
        .unwrap();
    let momentum = s
        .pending_events()
        .iter()
        .find_map(|e| match e {
            Event::Projectile(ProjectileEvent::Velocity { momentum, .. }) => Some(*momentum),
            _ => None,
        })
        .expect("launched");
    assert_eq!(momentum[2], Fixed::from_map_units(121));
    progresses(&mut s, |_, _| [TicCommand::default(); 4], 30);
}

#[test]
fn steep_fireball_half_step_explodes_below_its_floor_and_is_clipped() {
    // P_CheckMissileSpawn moves the new fireball by half its momentum before the
    // first P_TryMove. An Imp 20 units behind a 500-unit ledge edge aiming at a
    // player 35 units away launches with momz -166 (dist 3), so the half-step
    // leaves the fireball 51 units below its floor: the step rule refuses, it
    // explodes in place and its next P_ZMovement clips it to the floor.
    let room = HeightsRoom::new(&[-512, 0, 512], &[[-200, 500], [300, 500]])
        .with_things(&[[20, 0, WEST, IMP, 7]]);
    let mut s = heights(room.map());
    let imp = s.spawn_map_things().unwrap().spawned[0];
    let body = arm(&mut s, [-15, 0]);
    s.advance([TicCommand::default(); 4]).unwrap();
    // Controlled launch between tics: the blast is below its floor until the next
    // pass, which the witness exempts as it exempts the blast's XY location.
    s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, imp, body))
        .unwrap()
        .unwrap();
    let blast = s
        .actor_order()
        .find(|id| {
            s.actor(*id)
                .unwrap()
                .life()
                .is_some_and(|l| original_registry::is_fireball(l.actor_type))
        })
        .expect("exploded fireball");
    let a = s.actor(blast).unwrap();
    assert_eq!(a.momz, Fixed::ZERO, "exploded");
    assert!(a.z < a.floor - Fixed::from_map_units(24), "below its floor");
    progresses(&mut s, |_, _| [TicCommand::default(); 4], 2);
    assert_eq!(s.actor(blast).unwrap().z, Fixed::from_map_units(300));
    // The same launch inside a tic (A_TroopAttack) is clipped in the same pass.
    let mut s = heights(room.map());
    let imp = s.spawn_map_things().unwrap().spawned[0];
    let body = arm(&mut s, [-15, 0]);
    s.set_actor_target(imp, Some(body)).unwrap();
    s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
        .unwrap();
    progresses(&mut s, |_, _| [TicCommand::default(); 4], 40);
    assert!(
        s.events().iter().chain(s.pending_events()).count() > 0 || s.actor_order().count() > 2,
        "the launch happened"
    );
}

#[test]
fn heights_policy_identity_is_distinct_from_the_older_policies() {
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersHeightsV1.wire_id(),
        OriginalExecutionProfile::PlayerMonstersMapScaleV1.wire_id() + 1
    );
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersHeightsV1.context_schema(),
        PLAYER_HEIGHTS_CONTEXT_SCHEMA
    );
    for profile in [
        OriginalExecutionProfile::PlayerMonstersMapScaleV1,
        OriginalExecutionProfile::PlayerMonstersImpV1,
        OriginalExecutionProfile::PlayerMonstersV1,
    ] {
        assert_ne!(profile.context_schema(), PLAYER_HEIGHTS_CONTEXT_SCHEMA);
        assert!(!profile.has_heights());
    }
    assert!(OriginalExecutionProfile::PlayerMonstersHeightsV1.has_map_scale());
    assert_eq!(HEIGHTS_MOMZ_UNITS, 1024);
    assert_eq!(HEIGHTS_Z_UNITS, MAP_SCALE_EXTENT_UNITS);
    // The slot budget is part of the proof: any other capacity is refused.
    let registry =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersHeightsV1).unwrap();
    assert_eq!(
        Simulation::with_original_registry(
            stairs().map(),
            64,
            registry,
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .err(),
        Some(SimError::Store(StoreError::InvalidLimit))
    );
}

#[test]
fn dsim31_reader_is_explicit_and_checks_items_against_their_sector() {
    let mut s = heights(
        stairs()
            .with_things(&[[-400, 0, 0, POSSESSED, 7], [200, -100, 0, 2007, 7]])
            .map(),
    );
    let spawned = s.spawn_map_things().unwrap().spawned;
    let clip = spawned[1];
    arm(&mut s, [900, 0]);
    s.advance([TicCommand::default(); 4]).unwrap();
    assert_eq!(s.actor(clip).unwrap().floor, Fixed::from_map_units(24));
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_heights(&bytes, limits).unwrap();
    assert_eq!(view.version(), 31);
    assert_eq!(view.encode(limits).unwrap(), bytes);
    for reader in [
        Diagnostic::decode_map_scale,
        Diagnostic::decode_imps,
        Diagnostic::decode_monsters,
        Diagnostic::decode,
    ] {
        assert!(reader(&bytes, limits).is_err());
    }
    let patch = |path: &str, value: &[u8]| {
        let mut out = bytes.clone();
        let needle = [&(path.len() as u32).to_le_bytes()[..], path.as_bytes()].concat();
        let at = out.windows(needle.len()).position(|w| w == needle).unwrap() + needle.len();
        let len = u64::from_le_bytes(out[at..at + 8].try_into().unwrap()) as usize;
        assert_eq!(len, value.len());
        out[at + 8..at + 8 + len].copy_from_slice(value);
        out
    };
    assert_eq!(
        Diagnostic::decode_heights(
            &patch(
                "store.max_slots",
                &(MAP_SCALE_ACTOR_SLOTS + 1).to_le_bytes()
            ),
            limits
        )
        .unwrap_err(),
        DiagnosticError::Limit
    );
    assert!(Diagnostic::decode_heights(&patch("profile", &14_u32.to_le_bytes()), limits).is_err());
    // The item's floor and ceiling must be its own sector's: the flat 0/128 is rejected.
    let item = format!("actor.{}", clip.slot());
    assert!(
        Diagnostic::decode_heights(
            &patch(&format!("{item}.floor"), &0_i32.to_le_bytes()),
            limits
        )
        .is_err()
    );
    assert!(
        Diagnostic::decode_heights(
            &patch(&format!("{item}.ceiling"), &(128 * 65536_i32).to_le_bytes()),
            limits
        )
        .is_err()
    );
    // A v31 frame relabeled 30 is not an admitted map-scale frame, and a v30
    // frame is not a heights frame.
    let mut relabeled = bytes.clone();
    relabeled[4..8].copy_from_slice(&30_u32.to_le_bytes());
    assert!(Diagnostic::decode_map_scale(&relabeled, limits).is_err());
    let mut flat = map_scale_evidence::setup(&map_scale_evidence::cases()[0]).0;
    flat.advance([TicCommand::default(); 4]).unwrap();
    let flat_bytes = flat.snapshot().diagnostic_bytes(limits).unwrap();
    assert!(Diagnostic::decode_map_scale(&flat_bytes, limits).is_ok());
    assert!(Diagnostic::decode_heights(&flat_bytes, limits).is_err());
}

/// Deterministic command stream: runs, strafes, turns and sustained fire.
fn scripted(seed: u64, op: usize) -> [TicCommand; 4] {
    let mut x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(op as u64)
        .wrapping_mul(1442695040888963407);
    x ^= x >> 29;
    let phase = (op / 40) as u64 + seed;
    let mut c = [TicCommand::default(); 4];
    c[0] = TicCommand {
        forward: [50, 50, 0, -25, 50][(phase % 5) as usize],
        side: [0, 40, -40, 0, 24][((phase / 5) % 5) as usize],
        angle_turn: [0, 0, 1024, -2048, 512][((x >> 8) % 5) as usize],
        buttons: if (x >> 16) % 4 != 0 { 1 } else { 0 },
    };
    c
}
fn soak(room: HeightsRoom, start: [i16; 2], angle: u32) {
    let map = room.map();
    let (mut peak_actors, mut kills, mut landings, mut falls) = (0, 0, 0, 0);
    for seed in 0..4_u64 {
        let mut s = Simulation::original_player_heights_map(
            map.clone(),
            MapContextOptions {
                initial_rng: DoomRng::from_indices((seed * 61 % 256) as u8, 0),
                respawn_monsters: false,
            },
        )
        .unwrap();
        s.spawn_map_things().unwrap();
        let body = s
            .spawn_player_body(PlayerSlot::One, start.map(Fixed::from_map_units))
            .unwrap();
        s.initialize_player_weapons(
            PlayerSlot::One,
            WeaponInventory {
                ready: Weapon::Chaingun,
                owned: 15,
                ammo: [200, 50, 0, 0],
            },
        )
        .unwrap();
        s.actors.get_mut(body).unwrap().spec.angle = angle;
        for op in 1..=1200 {
            // A dead player admits only zero commands, as the body contract documents.
            let live = s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live;
            let commands = if live {
                scripted(seed, op)
            } else {
                [TicCommand::default(); 4]
            };
            s.advance(commands)
                .unwrap_or_else(|e| panic!("seed {seed} tic {op} rejected: {e}"));
            assert_vertical_invariant(&s, op);
            for e in s.events() {
                match e {
                    Event::ActorState(lifecycle::ActorStateEvent::Sound { sound: 34, .. }) => {
                        landings += 1
                    }
                    Event::Vertical { update, .. } if update.gravity_applied => falls += 1,
                    _ => {}
                }
            }
        }
        peak_actors = peak_actors.max(s.actor_order().count());
        kills = kills.max(s.level_statistics().unwrap().single_player_kill_count);
    }
    assert!(peak_actors <= 241, "live actors {peak_actors}");
    assert!(falls > 0, "the soak never left the floor");
    println!(
        "heights soak peak_actors={peak_actors} kills={kills} landings={landings} falls={falls}"
    );
}
#[test]
fn stairs_soak_keeps_the_vertical_invariant_with_the_full_budget() {
    // Sixteen items along the south side, eight monsters along the north side on
    // every tread, a firing, running player starting on the bottom step.
    let mut things = Vec::new();
    for i in 0..16 {
        let kind = [2007, 2048, 2008, 2049, 2011, 2012, 2014, 2015][i % 8];
        things.push([-700 + 100 * i as i16, -400, 0, kind, 7]);
    }
    for (i, kind) in [IMP, IMP, IMP, IMP, IMP, POSSESSED, SHOTGUY, 65]
        .iter()
        .enumerate()
    {
        things.push([-650 + 200 * i as i16, 300, WEST, *kind, 7]);
    }
    soak(stairs().with_things(&things), [900, -300], 0x8000_0000);
}
#[test]
fn ledge_soak_keeps_the_vertical_invariant_with_the_full_budget() {
    let mut things = Vec::new();
    for i in 0..16 {
        let kind = [2007, 2048, 2008, 2049, 2011, 2012, 2014, 2015][i % 8];
        things.push([-900 + 120 * i as i16, -400, 0, kind, 7]);
    }
    for (i, kind) in [IMP, IMP, IMP, IMP, IMP, POSSESSED, SHOTGUY, 65]
        .iter()
        .enumerate()
    {
        things.push([-900 + 250 * i as i16, 300, 0, *kind, 7]);
    }
    soak(ledge().with_things(&things), [-200, -300], 0);
}
