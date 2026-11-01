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
}
impl Room {
    pub fn rectangle(min: [i16; 2], max: [i16; 2]) -> Self {
        Self {
            min,
            max,
            pillars: Vec::new(),
            segment: 1024,
            things: Vec::new(),
            origin: [min[0] - 8, min[1] - 8],
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
        let walls = self.walls();
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
                if lx1 >= cx && lx0 <= cx + 128 && ly1 >= cy && ly0 <= cy + 128 && i != 0 {
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
