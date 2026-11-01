//! M3h3 wall-aware travel: reproduced M3h2 stalls on the historical policies and
//! the real-map-scale policy's rejection-free play. Expected behavior comes from the
//! original source reading in M3H3_REFERENCE_INVESTIGATION.md, never from Rust output.
use super::*;
use crate::identity::{OriginalActorType, OriginalState};

/// Test-input assembly only: a flat one-sector room of axis-aligned one-sided
/// `ML_BLOCKING` walls with the sector on their right, optional interior pillars,
/// walls split into pieces of at most `segment` units, and a cell-exact BLOCKMAP.
pub(crate) struct Room {
    pub min: [i16; 2],
    pub max: [i16; 2],
    pub pillars: Vec<[i16; 4]>,
    pub segment: i16,
    pub things: Vec<[i16; 5]>,
    /// BLOCKMAP origin; the grid is sized to cover the vertex rectangle.
    pub origin: [i16; 2],
    /// Leave this wall piece out entirely (a gap in the boundary).
    pub omit: Option<usize>,
    /// Keep this wall but list it in no BLOCKMAP cell (an incomplete BLOCKMAP).
    pub unlisted: Option<usize>,
}
impl Room {
    pub fn rectangle(min: [i16; 2], max: [i16; 2]) -> Self {
        Self {
            min,
            max,
            pillars: Vec::new(),
            segment: 1024,
            things: Vec::new(),
            origin: min,
            omit: None,
            unlisted: None,
        }
    }
    fn walls(&self) -> Vec<[i16; 4]> {
        let mut out = Vec::new();
        let [x0, y0] = self.min;
        let [x1, y1] = self.max;
        // Clockwise perimeter keeps the room on each wall's right (front) side.
        let mut push = |a: [i16; 2], b: [i16; 2]| {
            let length = (i32::from(b[0]) - i32::from(a[0])).abs()
                + (i32::from(b[1]) - i32::from(a[1])).abs();
            let pieces = ((length + i32::from(self.segment) - 1) / i32::from(self.segment)).max(1);
            for k in 0..pieces {
                let at = |p: i16, q: i16, k: i32| {
                    (i32::from(p) + (i32::from(q) - i32::from(p)) * k / pieces) as i16
                };
                out.push([
                    at(a[0], b[0], k),
                    at(a[1], b[1], k),
                    at(a[0], b[0], k + 1),
                    at(a[1], b[1], k + 1),
                ]);
            }
        };
        push([x0, y0], [x0, y1]);
        push([x0, y1], [x1, y1]);
        push([x1, y1], [x1, y0]);
        push([x1, y0], [x0, y0]);
        for &[px0, py0, px1, py1] in &self.pillars {
            // Counterclockwise pillar keeps the room outside, on the right.
            push([px0, py0], [px1, py0]);
            push([px1, py0], [px1, py1]);
            push([px1, py1], [px0, py1]);
            push([px0, py1], [px0, py0]);
        }
        out
    }
    pub fn wad(&self) -> Vec<u8> {
        let mut walls = self.walls();
        if let Some(i) = self.omit {
            walls.remove(i);
        }
        let mut vertices: Vec<[i16; 2]> = Vec::new();
        let mut vertex = |p: [i16; 2]| -> i16 {
            if let Some(i) = vertices.iter().position(|v| *v == p) {
                i as i16
            } else {
                vertices.push(p);
                (vertices.len() - 1) as i16
            }
        };
        let lines: Vec<[i16; 2]> = walls
            .iter()
            .map(|w| [vertex([w[0], w[1]]), vertex([w[2], w[3]])])
            .collect();
        let words = |v: &[i16]| v.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>();
        let name = |s: &str| {
            let mut b = s.as_bytes().to_vec();
            b.resize(8, 0);
            b
        };
        let mut linedefs = Vec::new();
        let mut sidedefs = Vec::new();
        let mut segs = Vec::new();
        for (i, l) in lines.iter().enumerate() {
            linedefs.extend(words(&[l[0], l[1], 1, 0, 0, i as i16, -1]));
            sidedefs.extend(words(&[0, 0]));
            sidedefs.extend(name("-"));
            sidedefs.extend(name("-"));
            sidedefs.extend(name("TESTWALL"));
            sidedefs.extend(words(&[0]));
            segs.extend(words(&[l[0], l[1], 0, i as i16, 0, 0]));
        }
        let mut sectors = words(&[0, 128]);
        sectors.extend(name("TESTFLR0"));
        sectors.extend(name("TESTCEIL"));
        sectors.extend(words(&[160, 0, 0]));
        let [ox, oy] = self.origin;
        let width = ((i32::from(self.max[0]) - i32::from(ox)) / 128 + 1) as i16;
        let height = ((i32::from(self.max[1]) - i32::from(oy)) / 128 + 1) as i16;
        let cells = usize::from(width as u16) * usize::from(height as u16);
        let mut lists: Vec<u16> = Vec::new();
        let mut offsets = Vec::with_capacity(cells);
        for cell in 0..cells {
            let cx = i32::from(ox) + (cell % usize::from(width as u16)) as i32 * 128;
            let cy = i32::from(oy) + (cell / usize::from(width as u16)) as i32 * 128;
            offsets.push((4 + cells + lists.len()) as u16);
            lists.push(0);
            for (i, w) in walls.iter().enumerate() {
                let (lx0, lx1) = (w[0].min(w[2]) as i32, w[0].max(w[2]) as i32);
                let (ly0, ly1) = (w[1].min(w[3]) as i32, w[1].max(w[3]) as i32);
                if lx1 >= cx
                    && lx0 <= cx + 128
                    && ly1 >= cy
                    && ly0 <= cy + 128
                    && i != 0
                    && self.unlisted != Some(i)
                {
                    lists.push(i as u16);
                }
            }
            lists.push(u16::MAX);
        }
        let mut blockmap = words(&[ox, oy, width, height]);
        for o in offsets.iter().chain(lists.iter()) {
            blockmap.extend(o.to_le_bytes());
        }
        let lumps: Vec<(&str, Vec<u8>)> = vec![
            ("MAP01", Vec::new()),
            (
                "THINGS",
                self.things.iter().flat_map(|t| words(t)).collect(),
            ),
            ("LINEDEFS", linedefs),
            ("SIDEDEFS", sidedefs),
            ("VERTEXES", vertices.iter().flat_map(|v| words(v)).collect()),
            ("SEGS", segs),
            ("SSECTORS", words(&[lines.len() as i16, 0])),
            ("NODES", Vec::new()),
            ("SECTORS", sectors),
            ("REJECT", vec![0]),
            ("BLOCKMAP", blockmap),
        ];
        let mut data = b"PWAD".to_vec();
        data.extend((lumps.len() as u32).to_le_bytes());
        data.extend(0_u32.to_le_bytes());
        let mut directory = Vec::new();
        for (n, b) in &lumps {
            directory.extend((data.len() as u32).to_le_bytes());
            directory.extend((b.len() as u32).to_le_bytes());
            directory.extend(name(n));
            data.extend(b);
        }
        let at = (data.len() as u32).to_le_bytes();
        data[8..12].copy_from_slice(&at);
        data.extend(directory);
        data
    }
    pub fn map(&self) -> doom_map::Map {
        let bytes = self.wad();
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
        doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
    }
}

const IMP: i16 = 3001;
const SHOTGUY: i16 = 9;

/// The M3h2 stall room: admitted by the historical policies (vertices within
/// 2,048, player within 994) yet larger than their 994-unit actor margin.
fn stall_room(things: &[[i16; 5]]) -> doom_map::Map {
    let mut room = Room::rectangle([-1000, -1000], [1200, 1000]);
    room.things = things.to_vec();
    room.map()
}
fn historical(policy: OriginalExecutionProfile, map: doom_map::Map) -> Simulation {
    let registry = OriginalExecutionRegistry::new(policy).unwrap();
    Simulation::with_original_registry(
        map,
        64,
        registry,
        ActorOptions::default(),
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
/// Run tics until the first rejection; then show that the same rejection recurs
/// for 64 further zero-command tics with the complete state unchanged. Returns the
/// rejected tic, the error and whether any earlier tic launched a fireball.
fn first_rejection(
    s: &mut Simulation,
    mut command: impl FnMut(&Simulation, usize) -> [TicCommand; 4],
    limit: usize,
) -> (usize, SimError, bool) {
    let mut launched = false;
    for op in 1..=limit {
        launched |= s
            .events()
            .iter()
            .any(|e| matches!(e, Event::Projectile(ProjectileEvent::Launch(_))));
        let commands = command(s, op);
        let before = s.clone();
        if let Err(error) = s.advance(commands) {
            assert_eq!(s.snapshot(), before.snapshot(), "rollback at tic {op}");
            assert_eq!(s.rng(), before.rng());
            assert_eq!(s.events(), before.events());
            for _ in 0..64 {
                assert_eq!(s.advance([TicCommand::default(); 4]), Err(error));
                assert_eq!(s.snapshot(), before.snapshot());
            }
            return (op, error, launched);
        }
    }
    panic!("no rejection within {limit} tics");
}

#[test]
fn stall_imp_policy_rejects_every_tic_after_a_fireball_hit_near_the_wall() {
    // M3h2 open limit 1: `|position| + 160M/3 <= 994` fails for any fireball hit on a
    // player 16 units from the east wall, and rollback restores the same hit.
    let mut s = historical(
        OriginalExecutionProfile::PlayerMonstersImpV1,
        stall_room(&[[800, 0, 0, IMP, 7]]),
    );
    let imp = s.spawn_map_things().unwrap().spawned[0];
    let body = arm(&mut s, [984, 0]);
    s.set_actor_target(imp, Some(body)).unwrap();
    s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
        .unwrap();
    let (op, error, launched) = first_rejection(&mut s, |_, _| [TicCommand::default(); 4], 200);
    assert_eq!(error, SimError::PlayerBody(PlayerBodyError::Unsupported));
    assert!(
        launched,
        "the Imp launched before the rejection at tic {op}"
    );
    assert!(
        (17..=60).contains(&op),
        "fireball flight to the player: tic {op}"
    );
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, 100);
}

#[test]
fn stall_monsters_policy_rejects_every_tic_after_a_hitscan_hit_near_the_wall() {
    // M3h2 open limit 1, base-master variant: three shotgun pellets against the same
    // player position reserve 20 units of travel each, past 994, with no Imp present.
    let mut s = historical(
        OriginalExecutionProfile::PlayerMonstersV1,
        stall_room(&[[936, 0, 0, SHOTGUY, 7]]),
    );
    // A_SPosAttack has no direct-entry state, so the shotgunner wakes through its
    // own A_Look, waits out its reaction time and fires at point-blank range.
    s.spawn_map_things().unwrap();
    arm(&mut s, [984, 0]);
    let (op, error, launched) = first_rejection(&mut s, |_, _| [TicCommand::default(); 4], 200);
    assert_eq!(error, SimError::PlayerBody(PlayerBodyError::Unsupported));
    assert!(!launched);
    assert!(
        op > 10,
        "look, reaction time and S_SPOS_ATK1 precede the volley: tic {op}"
    );
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, 100);
}

#[test]
fn stall_imp_policy_rejects_every_tic_once_a_fireball_passes_994() {
    // M3h2 open limit 2, confirmed: the room is admitted to 1,200 but every actor must
    // stay within 994 before each tic, so a missed fireball freezes the world.
    let mut s = historical(
        OriginalExecutionProfile::PlayerMonstersImpV1,
        stall_room(&[[-900, 100, 0, IMP, 7]]),
    );
    let imp = s.spawn_map_things().unwrap().spawned[0];
    // Far from the east wall, so the five strafing tics and the following friction
    // slide stay inside the historical travel envelope while the fireball flies on.
    let body = arm(&mut s, [-600, 0]);
    s.set_actor_target(imp, Some(body)).unwrap();
    s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
        .unwrap();
    let mut dodge_from = None;
    let (op, error, launched) = first_rejection(
        &mut s,
        |s, op| {
            if dodge_from.is_none()
                && s.events()
                    .iter()
                    .any(|e| matches!(e, Event::Projectile(ProjectileEvent::Launch(_))))
            {
                dodge_from = Some(op);
            }
            let mut c = [TicCommand::default(); 4];
            if dodge_from.is_some_and(|from| op < from + 5) {
                c[0].side = 50;
            }
            c
        },
        400,
    );
    assert!(launched);
    assert_eq!(error, SimError::Monster(MonsterError::Arithmetic));
    // The Imp keeps attacking during the long flight; only the missed fireball's
    // position, not any hit, causes this rejection.
    assert!(s.original_player(PlayerSlot::One).unwrap().health > 0);
    assert!(
        op > 150,
        "the missed fireball needs about 190 tics to reach x > 994: tic {op}"
    );
    let fireball = s
        .actor_order()
        .map(|id| *s.actor(id).unwrap())
        .find(|a| {
            a.life()
                .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_TROOPSHOT.ordinal())
        })
        .expect("the missed fireball is still flying");
    assert!(
        fireball.position()[0] > Fixed::from_map_units(994),
        "fireball x {} at tic {op}",
        fireball.position()[0].bits() / 65536
    );
    assert!(fireball.position()[0] < Fixed::from_map_units(1200));
}

// Source replay: every committed scenario against the compiled original rows,
// the independent DSIM30 captures and ordered events; step equals advance.
#[test]
fn map_scale_source_case_00_stall_fireball_hit_near_wall() {
    map_scale_evidence::verify_case(0);
}
#[test]
fn map_scale_source_case_01_stall_hitscan_near_wall() {
    map_scale_evidence::verify_case(1);
}
#[test]
fn map_scale_source_case_02_stall_fireball_past_994() {
    map_scale_evidence::verify_case(2);
}
#[test]
fn map_scale_source_case_03_corner_fireball_pinch() {
    map_scale_evidence::verify_case(3);
}
#[test]
fn map_scale_source_case_04_demon_corner_bites() {
    map_scale_evidence::verify_case(4);
}
#[test]
fn map_scale_source_case_05_wall_slide_under_fire() {
    map_scale_evidence::verify_case(5);
}
#[test]
fn map_scale_source_case_06_monster_knockback_at_wall() {
    map_scale_evidence::verify_case(6);
}
#[test]
fn map_scale_source_case_07_pillar_pinch() {
    map_scale_evidence::verify_case(7);
}
#[test]
fn map_scale_source_case_08_long_fireball_flight() {
    map_scale_evidence::verify_case(8);
}
#[test]
fn map_scale_source_case_09_full_room_traversal() {
    map_scale_evidence::verify_case(9);
}
#[test]
fn map_scale_source_case_10_eight_imps_far_room() {
    map_scale_evidence::verify_case(10);
}
#[test]
fn map_scale_source_case_11_mixed_roster_corner_fight() {
    map_scale_evidence::verify_case(11);
}
#[test]
fn map_scale_source_case_12_drops_far_from_origin() {
    map_scale_evidence::verify_case(12);
}
#[test]
fn map_scale_source_case_13_hitscan_far_from_origin() {
    map_scale_evidence::verify_case(13);
}

// Rust-only policy tests for PlayerMonstersMapScaleV1: the reproduced stalls
// progressing, map admission, the new recoverable paths, caps and readers.

fn map_scale(map: doom_map::Map) -> Simulation {
    Simulation::original_player_map_scale_map(
        map,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(6, 0),
            respawn_monsters: false,
        },
    )
    .unwrap()
}
fn stall_room_scale(things: &[[i16; 5]]) -> Simulation {
    map_scale(stall_room(things))
}
/// Every tic must succeed and advance the counter by exactly one.
fn progresses(
    s: &mut Simulation,
    mut command: impl FnMut(&Simulation, usize) -> [TicCommand; 4],
    tics: usize,
) {
    for op in 1..=tics {
        let before = s.tic();
        let commands = command(s, op);
        s.advance(commands)
            .unwrap_or_else(|e| panic!("tic {op} rejected under the map-scale policy: {e}"));
        assert_eq!(s.tic(), before + 1);
    }
}
fn fireball_count(s: &Simulation) -> usize {
    s.actor_order()
        .filter(|id| {
            s.actor(*id)
                .unwrap()
                .life()
                .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_TROOPSHOT.ordinal())
        })
        .count()
}

#[test]
fn map_scale_fireball_hit_near_the_wall_progresses() {
    // The M3h2 stall 3.1 under the new policy: the hit pushes the body into the
    // wall and the world keeps running, as the source does.
    let mut s = stall_room_scale(&[[800, 0, 0, IMP, 7]]);
    let imp = s.spawn_map_things().unwrap().spawned[0];
    let body = arm(&mut s, [984, 0]);
    s.set_actor_target(imp, Some(body)).unwrap();
    s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
        .unwrap();
    progresses(&mut s, |_, _| [TicCommand::default(); 4], 400);
    let p = s.original_player(PlayerSlot::One).unwrap();
    assert!(p.health < 100, "the fireball hit landed");
    // Pushed east by the hits, at most to the wall at 1,200 (center 1,184).
    let a = s.actor(body).unwrap();
    assert!(a.position()[0] <= Fixed::from_map_units(1184));
    assert!(a.position()[0] > Fixed::from_map_units(984));
}

#[test]
fn map_scale_hitscan_hit_near_the_wall_progresses() {
    let mut s = stall_room_scale(&[[936, 0, 0, SHOTGUY, 7]]);
    s.spawn_map_things().unwrap();
    arm(&mut s, [984, 0]);
    progresses(&mut s, |_, _| [TicCommand::default(); 4], 300);
    assert!(s.original_player(PlayerSlot::One).unwrap().health < 100);
}

#[test]
fn map_scale_fireball_past_994_progresses_to_the_far_wall() {
    let mut s = stall_room_scale(&[[-900, 100, 0, IMP, 7]]);
    let imp = s.spawn_map_things().unwrap().spawned[0];
    let body = arm(&mut s, [-600, 0]);
    s.set_actor_target(imp, Some(body)).unwrap();
    s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
        .unwrap();
    let mut dodge_from = None;
    let mut farthest = 0;
    for op in 1..=400 {
        if dodge_from.is_none()
            && s.events()
                .iter()
                .any(|e| matches!(e, Event::Projectile(ProjectileEvent::Launch(_))))
        {
            dodge_from = Some(op);
        }
        let mut c = [TicCommand::default(); 4];
        if dodge_from.is_some_and(|from| op < from + 5) {
            c[0].side = 50;
        }
        s.advance(c).unwrap_or_else(|e| panic!("tic {op}: {e}"));
        for id in s.actor_order() {
            let a = s.actor(id).unwrap();
            if a.life()
                .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_TROOPSHOT.ordinal())
            {
                farthest = farthest.max(a.position()[0].bits() / 65536);
            }
        }
    }
    assert!(
        farthest > 994 && farthest <= 1194,
        "the missed fireball flew to the east wall: x {farthest}"
    );
}

#[test]
fn map_scale_rejects_geometry_outside_the_proven_domain_at_load() {
    let options = MapContextOptions::default();
    let make = |room: &Room| Simulation::original_player_map_scale_map(room.map(), options);
    // A full-extent room is admitted; one vertex beyond 2,048 is not.
    make(&Room::rectangle([-2048, -2048], [2048, 2048])).unwrap();
    assert!(matches!(
        make(&Room::rectangle([-2048, -2048], [2049, 2048])),
        Err(OriginalRegistryError::Runtime(SimError::Geometry(
            GeometryError::CoordinateOverflow
        )))
    ));
    // A gap in the bounding walls, and a wall listed in no BLOCKMAP cell.
    let mut gap = Room::rectangle([-1000, -1000], [1000, 1000]);
    gap.segment = 500;
    gap.omit = Some(5);
    assert!(matches!(
        make(&gap),
        Err(OriginalRegistryError::Runtime(SimError::MapScale(
            MapScaleError::Walls
        )))
    ));
    let mut unlisted = Room::rectangle([-1000, -1000], [1000, 1000]);
    unlisted.segment = 500;
    unlisted.unlisted = Some(5);
    assert!(matches!(
        make(&unlisted),
        Err(OriginalRegistryError::Runtime(SimError::MapScale(
            MapScaleError::Blockmap
        )))
    ));
    // The historical Imp policy never checked either; it admits both maps.
    for room in [&gap, &unlisted] {
        historical(OriginalExecutionProfile::PlayerMonstersImpV1, room.map());
    }
    // A BLOCKMAP that does not cover the rectangle.
    let mut short = Room::rectangle([-1000, -1000], [1000, 1000]);
    short.origin = [-900, -1000];
    assert!(matches!(
        make(&short),
        Err(OriginalRegistryError::Runtime(SimError::MapScale(
            MapScaleError::Blockmap
        )))
    ));
    // Too many linedefs for one hitscan: 20 pillars of four walls plus the room.
    let mut many = Room::rectangle([-2048, -2048], [2048, 2048]);
    many.pillars = (0..20)
        .map(|i| {
            let x = -1800 + 150 * (i % 10);
            let y = if i < 10 { -1000 } else { 1000 };
            [x, y, x + 50, y + 50]
        })
        .collect();
    assert_eq!(many.map().linedefs().len(), 16 + 80);
    assert!(matches!(
        make(&many),
        Err(OriginalRegistryError::Runtime(SimError::MapScale(
            MapScaleError::Lines
        )))
    ));
    many.pillars.pop();
    make(&many).unwrap();
    // A non-axis wall stays excluded by the inherited grounded-geometry check.
    let diagonal = {
        let room = Room::rectangle([-1000, -1000], [1000, 1000]);
        let mut bytes = room.wad();
        let directory = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        for i in 0..count {
            let at = directory + 16 * i;
            if &bytes[at + 8..at + 16] == b"VERTEXES" {
                let start = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
                bytes[start..start + 2].copy_from_slice(&(-900_i16).to_le_bytes());
            }
        }
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
        doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
    };
    assert!(matches!(
        Simulation::original_player_map_scale_map(diagonal, options),
        Err(OriginalRegistryError::Runtime(SimError::PlayerBody(
            PlayerBodyError::Unsupported
        )))
    ));
    // The slot budget is part of the proof: any other capacity is refused.
    let registry =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersMapScaleV1).unwrap();
    assert_eq!(
        Simulation::with_original_registry(
            Room::rectangle([-1000, -1000], [1000, 1000]).map(),
            64,
            registry,
            ActorOptions::default(),
            options,
        )
        .err(),
        Some(SimError::Store(StoreError::InvalidLimit))
    );
}

#[test]
fn map_scale_rejects_spawns_whose_box_crosses_a_wall() {
    // A THING 19 units from the east wall (radius 20) could leave the room in
    // one step; the batch rejects it before any spawn.
    let mut s = stall_room_scale(&[[1181, 0, 0, IMP, 7], [800, 0, 0, IMP, 7]]);
    let before = s.snapshot();
    assert_eq!(
        s.spawn_map_things().unwrap_err(),
        SimError::Monster(MonsterError::Arithmetic)
    );
    assert_eq!(s.snapshot(), before);
    let mut s = stall_room_scale(&[[1180, 0, 0, IMP, 7]]);
    s.spawn_map_things().unwrap();
    // The player body: touching the wall is admitted, crossing it is not.
    let before = s.snapshot();
    assert!(
        s.spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(1185), Fixed::ZERO])
            .is_err()
    );
    assert_eq!(s.snapshot(), before);
    arm(&mut s, [1184, 0]);
    progresses(&mut s, |_, _| [TicCommand::default(); 4], 60);
}

#[test]
fn historical_policies_keep_their_coordinate_margins() {
    // The same THING is beyond the Imp policy's 978-unit spawn margin and inside
    // the map-scale room.
    let map = || stall_room(&[[1100, 0, 0, IMP, 7]]);
    let mut old = historical(OriginalExecutionProfile::PlayerMonstersImpV1, map());
    let before = old.snapshot();
    assert_eq!(
        old.spawn_map_things().unwrap_err(),
        SimError::Monster(MonsterError::Arithmetic)
    );
    assert_eq!(old.snapshot(), before);
    let mut new = map_scale(map());
    new.spawn_map_things().unwrap();
    // The historical player spawn margin (994) is unchanged as well.
    let mut old = historical(
        OriginalExecutionProfile::PlayerMonstersImpV1,
        stall_room(&[]),
    );
    assert!(
        old.spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(1000), Fixed::ZERO])
            .is_err()
    );
    let mut new = stall_room_scale(&[]);
    new.spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(1000), Fixed::ZERO])
        .unwrap();
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersMapScaleV1.wire_id(),
        OriginalExecutionProfile::PlayerMonstersImpV1.wire_id() + 1
    );
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersMapScaleV1.context_schema(),
        PLAYER_MAP_SCALE_CONTEXT_SCHEMA
    );
    for profile in [
        OriginalExecutionProfile::PlayerMonstersImpV1,
        OriginalExecutionProfile::PlayerMonstersV1,
    ] {
        assert_ne!(profile.context_schema(), PLAYER_MAP_SCALE_CONTEXT_SCHEMA);
    }
}

#[test]
fn momentum_bound_is_a_recoverable_rejection_with_full_rollback() {
    // In-world impulses stay far below 4,096 units; an in-crate momentum at the
    // bound shows the clamp path (a tic runs) and the rejection path (one more
    // controlled hit, rolled back in full).
    let mut s = stall_room_scale(&[[0, 60, 0, 3002, 7]]);
    let demon = s.spawn_map_things().unwrap().spawned[0];
    let body = arm(&mut s, [0, 0]);
    let at_bound = [
        Fixed::from_map_units(MAP_SCALE_MOMENTUM_UNITS as i16),
        Fixed::ZERO,
    ];
    s.actors.get_mut(body).unwrap().spec.momentum = at_bound;
    s.advance([TicCommand::default(); 4]).unwrap();
    let a = s.actor(body).unwrap();
    assert!(
        a.momentum()[0] < Fixed::from_map_units(30),
        "clamped by P_XYMovement"
    );
    assert!(a.position()[0] <= Fixed::from_map_units(30));
    s.actors.get_mut(body).unwrap().spec.momentum = at_bound;
    let before = s.clone();
    assert_eq!(
        s.damage_actor(DamageRequest {
            target: body,
            inflictor: Some(demon),
            source: Some(demon),
            amount: 1,
        }),
        Err(SimError::MapScale(MapScaleError::Momentum))
    );
    assert_eq!(s.snapshot(), before.snapshot());
    assert_eq!(s.rng(), before.rng());
    s.actors.get_mut(body).unwrap().spec.momentum = [
        Fixed::from_map_units(MAP_SCALE_MOMENTUM_UNITS as i16 + 1),
        Fixed::ZERO,
    ];
    let before = s.clone();
    assert_eq!(
        s.advance([TicCommand::default(); 4]),
        Err(SimError::MapScale(MapScaleError::Momentum))
    );
    assert_eq!(s.snapshot(), before.snapshot());
}

#[test]
fn position_outside_the_room_is_a_recoverable_rejection() {
    // Unreachable by the wall invariant; an in-crate corruption shows the witness
    // check still rolls the tic back instead of continuing with unproven arithmetic.
    let mut s = stall_room_scale(&[[800, 0, 0, IMP, 7]]);
    let imp = s.spawn_map_things().unwrap().spawned[0];
    arm(&mut s, [0, 0]);
    s.advance([TicCommand::default(); 4]).unwrap();
    s.actors.get_mut(imp).unwrap().spec.position[0] = Fixed::from_map_units(1300);
    let before = s.clone();
    assert_eq!(
        s.advance([TicCommand::default(); 4]),
        Err(SimError::Monster(MonsterError::Arithmetic))
    );
    assert_eq!(s.snapshot(), before.snapshot());
}

#[test]
fn map_scale_fireball_cap_and_slots_reject_with_full_rollback() {
    let mut s = map_scale({
        let mut room = Room::rectangle([-2048, -2048], [2048, 2048]);
        room.things = vec![[-1900, 0, 0, IMP, 7]];
        room.map()
    });
    let imp = s.spawn_map_things().unwrap().spawned[0];
    let body = arm(&mut s, [1900, 0]);
    for _ in 0..MAP_SCALE_MAX_LIVE_PROJECTILES {
        s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, imp, body))
            .unwrap()
            .unwrap();
    }
    assert_eq!(fireball_count(&s), MAP_SCALE_MAX_LIVE_PROJECTILES);
    let before = s.clone();
    assert_eq!(
        s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, imp, body))
            .unwrap(),
        Err(ProjectileError::WorkLimit.into())
    );
    assert_eq!(s.snapshot(), before.snapshot());
    assert_eq!(s.rng(), before.rng());
    // The 210 live actors advance; the DSIM30 reader admits the frame and its
    // 208 fireballs, and the v29 reader does not.
    s.advance([TicCommand::default(); 4]).unwrap();
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_map_scale(&bytes, limits).unwrap();
    assert_eq!(view.version(), 30);
    assert_eq!(view.encode(limits).unwrap(), bytes);
    assert!(Diagnostic::decode_imps(&bytes, limits).is_err());
    assert_eq!(
        view.validate_live_projectiles_at(MAP_SCALE_MAX_LIVE_PROJECTILES - 1),
        Err(DiagnosticError::Limit)
    );
}

#[test]
fn dsim30_reader_is_explicit_and_bounds_slots() {
    let mut s = stall_room_scale(&[[800, 0, 0, IMP, 7]]);
    s.spawn_map_things().unwrap();
    arm(&mut s, [984, 0]);
    s.advance([TicCommand::default(); 4]).unwrap();
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_map_scale(&bytes, limits).unwrap();
    assert_eq!(view.version(), 30);
    assert_eq!(view.encode(limits).unwrap(), bytes);
    for reader in [
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
        Diagnostic::decode_map_scale(
            &patch(
                "store.max_slots",
                &(MAP_SCALE_ACTOR_SLOTS + 1).to_le_bytes()
            ),
            limits
        )
        .unwrap_err(),
        DiagnosticError::Limit
    );
    assert!(
        Diagnostic::decode_map_scale(&patch("profile", &13_u32.to_le_bytes()), limits).is_err()
    );
    assert!(
        Diagnostic::decode_map_scale(&patch("level.total_kills", &9_i32.to_le_bytes()), limits)
            .is_err()
    );
    // A v29 frame never admits the map-scale slot count: the historical reader is unchanged.
    let mut old = historical(
        OriginalExecutionProfile::PlayerMonstersImpV1,
        stall_room(&[[800, 0, 0, IMP, 7]]),
    );
    old.spawn_map_things().unwrap();
    arm(&mut old, [0, 0]);
    let old_bytes = old.snapshot().diagnostic_bytes(limits).unwrap();
    assert!(Diagnostic::decode_imps(&old_bytes, limits).is_ok());
    assert!(Diagnostic::decode_map_scale(&old_bytes, limits).is_err());
    let patch_old = |value: u32| {
        let mut out = old_bytes.clone();
        let path = "store.max_slots";
        let needle = [&(path.len() as u32).to_le_bytes()[..], path.as_bytes()].concat();
        let at = out.windows(needle.len()).position(|w| w == needle).unwrap() + needle.len() + 8;
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        out
    };
    assert_eq!(
        Diagnostic::decode_imps(&patch_old(MAP_SCALE_ACTOR_SLOTS), limits).unwrap_err(),
        DiagnosticError::Limit
    );
    // A v30 frame relabeled 29 is not an admitted Imp-policy frame.
    let mut relabeled = bytes.clone();
    relabeled[4..8].copy_from_slice(&29_u32.to_le_bytes());
    assert!(Diagnostic::decode_imps(&relabeled, limits).is_err());
}

#[test]
fn stale_and_foreign_handles_reject_for_map_scale() {
    let mut s = stall_room_scale(&[[800, 0, 0, IMP, 7]]);
    let imp = s.spawn_map_things().unwrap().spawned[0];
    arm(&mut s, [0, 0]);
    let stale = ActorId::from_parts(imp.slot(), imp.generation() + 1);
    let before = s.clone();
    for target in [
        stale,
        ActorId::from_parts(200, 1),
        ActorId::from_parts(300, 1),
    ] {
        assert!(
            s.damage_actor(DamageRequest {
                target,
                inflictor: None,
                source: None,
                amount: 10,
            })
            .is_err()
        );
        assert!(s.set_actor_target(imp, Some(target)).is_err());
        assert!(s.mark_remove(target).is_err());
        assert_eq!(s.snapshot(), before.snapshot());
    }
    let other = stall_room_scale(&[[800, 0, 0, IMP, 7]]);
    let mut left = GameInstance::new(before);
    let mut right = GameInstance::new(other);
    let id = DefinitionIdentity::Original(OriginalActorType::MT_TROOP);
    let reference = left
        .with_execution_context(|ctx| {
            let actor = ctx.admit_local(imp, id)?;
            ctx.export_actor(actor)
        })
        .unwrap();
    let snapshot = right.simulation().snapshot();
    assert!(
        right
            .with_execution_context(|ctx| {
                ctx.import_actor(&reference, id)?;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(right.simulation().snapshot(), snapshot);
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

#[test]
fn full_budget_soak_never_rejects_a_valid_tic() {
    // Sixteen placed items, eight monsters (five Imps, three former humans whose
    // deaths add drops), a firing, running player and the full 4,096-unit room.
    let mut room = Room::rectangle([-2048, -2048], [2048, 2048]);
    let mut things = Vec::new();
    for i in 0..16 {
        let kind = [2007, 2048, 2008, 2049, 2011, 2012, 2014, 2015][i % 8];
        things.push([
            -1800 + 200 * (i as i16 % 8),
            -1900 + 300 * (i as i16 / 8),
            0,
            kind,
            7,
        ]);
    }
    for (i, kind) in [IMP, IMP, IMP, IMP, IMP, 3004, SHOTGUY, 65]
        .iter()
        .enumerate()
    {
        things.push([
            -1700 + 400 * (i as i16 % 4),
            1200 + 500 * (i as i16 / 4),
            -90,
            *kind,
            7,
        ]);
    }
    room.things = things;
    let map = room.map();
    let mut peak_fireballs = 0;
    let mut peak_actors = 0;
    let mut kills = 0;
    for seed in 0..4_u64 {
        let mut s = Simulation::original_player_map_scale_map(
            map.clone(),
            MapContextOptions {
                initial_rng: DoomRng::from_indices((seed * 61 % 256) as u8, 0),
                respawn_monsters: false,
            },
        )
        .unwrap();
        s.spawn_map_things().unwrap();
        let body = s
            .spawn_player_body(PlayerSlot::One, [Fixed::ZERO, Fixed::from_map_units(-1500)])
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
        s.actors.get_mut(body).unwrap().spec.angle = 0x4000_0000;
        progresses(&mut s, |_, op| scripted(seed, op), 1200);
        peak_fireballs = peak_fireballs.max(fireball_count(&s));
        peak_actors = peak_actors.max(s.actor_order().count());
        kills = kills.max(s.level_statistics().unwrap().single_player_kill_count);
    }
    assert!(peak_actors <= 241, "live actors {peak_actors}");
    assert!(peak_fireballs <= MAP_SCALE_MAX_LIVE_PROJECTILES);
    println!("soak peak_actors={peak_actors} peak_fireballs={peak_fireballs} kills={kills}");
}

#[test]
fn spawn_near_the_coordinate_minimum_cannot_wrap_into_the_room() {
    // Review regression: a THING at -32,768 (bits i32::MIN) once passed the box test
    // through wrapping fixed-point subtraction and then stalled every tic.
    let mut s = stall_room_scale(&[[-32768, 0, 0, IMP, 7], [800, 0, 0, IMP, 7]]);
    let before = s.snapshot();
    assert!(s.spawn_map_things().is_err());
    assert_eq!(s.snapshot(), before);
    let mut s = stall_room_scale(&[[800, 0, 0, IMP, 7]]);
    s.spawn_map_things().unwrap();
    let before = s.snapshot();
    assert!(
        s.spawn_player_body(PlayerSlot::One, [Fixed::from_bits(i32::MIN), Fixed::ZERO])
            .is_err()
    );
    assert!(
        s.spawn_player_body(
            PlayerSlot::One,
            [Fixed::ZERO, Fixed::from_bits(i32::MIN + 65536)]
        )
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
    arm(&mut s, [0, 0]);
    progresses(&mut s, |_, _| [TicCommand::default(); 4], 30);
}

#[test]
fn derived_budgets_follow_the_source_state_tables() {
    // The fireball bound and slot budget are derived, not measured: the shortest
    // Imp launch cycle is its attack state cut short by pain (PAIN, PAIN2, RUN1
    // then ATK1 and ATK2, the missile check passing on JUSTHIT), and a fireball
    // holds a slot for the launch pass, the longest chord of the largest room at
    // 10 units per pass, its three explosion states and one retirement visit.
    let tics = |s: OriginalState| s.descriptor().tics as usize;
    let imp = OriginalActorType::MT_TROOP.states();
    let fireball = OriginalActorType::MT_TROOPSHOT.states();
    assert_eq!(imp.pain, OriginalState::S_TROO_PAIN);
    assert_eq!(imp.missile, OriginalState::S_TROO_ATK1);
    let pain = tics(OriginalState::S_TROO_PAIN) + tics(OriginalState::S_TROO_PAIN2);
    let run = tics(OriginalState::S_TROO_RUN1);
    let attack = tics(OriginalState::S_TROO_ATK1) + tics(OriginalState::S_TROO_ATK2);
    let cycle = pain + run + attack;
    assert_eq!(cycle, 23);
    let speed = OriginalActorType::MT_TROOPSHOT.descriptor().speed / 65536;
    let radius = OriginalActorType::MT_TROOPSHOT.descriptor().radius / 65536;
    let side = 2 * i64::from(MAP_SCALE_EXTENT_UNITS) - 2 * i64::from(radius);
    let chord = ((2 * side * side) as f64).sqrt().ceil() as usize;
    let flight = chord.div_ceil(speed as usize);
    let explosion =
        tics(fireball.death) + tics(OriginalState::S_TBALLX2) + tics(OriginalState::S_TBALLX3);
    let lifetime = 1 + flight + explosion + 1;
    assert_eq!(lifetime, 598);
    let per_imp = lifetime.div_ceil(cycle);
    assert_eq!(per_imp, 26);
    assert_eq!(
        MAP_SCALE_MAX_LIVE_PROJECTILES,
        MAX_PLAYER_MONSTERS * per_imp
    );
    // Slots: player, monsters, items, drops, fireballs and impacts for the worst
    // roster (eight Imps), plus one pass of spawns, stays below the budget.
    let live = 1 + MAX_PLAYER_MONSTERS + MAX_WORLD_PICKUPS + MAP_SCALE_MAX_LIVE_PROJECTILES + 8;
    let spawns = MAX_PLAYER_MONSTERS + 7;
    assert_eq!(live, 241);
    assert!(live + spawns <= MAP_SCALE_ACTOR_SLOTS as usize - 64);
}
