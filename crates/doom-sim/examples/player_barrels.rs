//! Exploding barrels, `A_Explode` and `P_RadiusAttack`, 64 pickups and every THING
//! angle under `PlayerMonstersBarrelsV1`: the committed C scenarios through the
//! public API, and separate measurements. The rooms are the synthetic M3h9b rooms
//! of the C evidence (`range`, `cluster`, `player_range`, `monsters`, `walls`,
//! `door`, `fireball`, `dense64`, `angles`, `embedded`), not the real E1M1.
#[path = "../../../tests/support/m3h9.rs"]
mod m3h9;
#[path = "../../../tests/support/m3h9b.rs"]
mod source;
use doom_sim::{identity::*, *};
use std::{
    collections::BTreeMap,
    error::Error,
    hint::black_box,
    io::Write,
    time::{Duration, Instant},
};
type ResultHost<T> = Result<T, Box<dyn Error>>;
fn interval<T>(run: impl FnOnce() -> T) -> std::io::Result<(T, Duration)> {
    let mut output = std::io::stdout().lock();
    output.write_all(b"DOOM_ALLOC_BEGIN\n")?;
    output.flush()?;
    let start = Instant::now();
    let result = run();
    let elapsed = start.elapsed();
    output.write_all(b"DOOM_ALLOC_END\n")?;
    output.flush()?;
    Ok((result, elapsed))
}
const WORKLOADS: [&str; 5] = [
    "cluster_chain",
    "dense64_fight",
    "range_patrol",
    "liftroom_ride",
    "dense64_lift",
];
/// `A_Explode`'s index in the source action table (the C rows' `E` kind 5 value).
const A_EXPLODE: i64 = 19;
fn barrel_state(state: i64) -> Option<usize> {
    [
        OriginalState::S_BEXP,
        OriginalState::S_BEXP2,
        OriginalState::S_BEXP4,
        OriginalState::S_GIBS,
    ]
    .iter()
    .position(|s| i64::from(s.ordinal()) == state)
}
fn is_barrel(s: &Simulation, id: ActorId) -> bool {
    s.actor(id)
        .and_then(|a| a.life())
        .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_BARREL.ordinal())
}

/// Barrel and mover work of one tic's events: explosions (`A_Explode`), the
/// candidates that reached `P_CheckSight` (blocked and visible), explosion damage,
/// barrel deaths (`S_BEXP` entries), `P_ChangeSector` calls and
/// `PIT_ChangeSector` thing visits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Work {
    explosions: usize,
    sight_blocked: usize,
    damaged: usize,
    deaths: usize,
    changes: usize,
    clips: usize,
}
impl Work {
    fn of_events(s: &Simulation, events: &[Event]) -> Self {
        let mut w = Self::default();
        for event in events {
            match event {
                Event::ActorState(ActorStateEvent::Action {
                    action: ActorAction::Original(a),
                    ..
                }) if i64::from(a.0) == A_EXPLODE => w.explosions += 1,
                Event::Perception(PerceptionEvent::Sight { target, result, .. })
                    if is_barrel(s, *target) =>
                {
                    if result.visible {
                        w.damaged += 1;
                    } else {
                        w.sight_blocked += 1;
                    }
                }
                Event::ActorState(ActorStateEvent::Entered { state, .. })
                    if state.0 == OriginalState::S_BEXP.ordinal() =>
                {
                    w.deaths += 1
                }
                Event::Mover(MoverEvent::SectorChanged { .. }) => w.changes += 1,
                Event::Mover(MoverEvent::ThingClipped { .. }) => w.clips += 1,
                _ => {}
            }
        }
        w
    }
    /// The same counts from one op's C rows (`RA`, `RC` codes 3 and 4, `E` kind
    /// 4 into `S_BEXP`, `MC` code 1, `MT`).
    fn of_rows(rows: &[(String, Vec<i64>)]) -> Self {
        let mut w = Self::default();
        for (tag, v) in rows {
            match tag.as_str() {
                "RA" => w.explosions += 1,
                "RC" if v[3] == 3 => w.sight_blocked += 1,
                "RC" if v[3] == 4 => w.damaged += 1,
                "E" if v[0] == 4 && barrel_state(v[2]) == Some(0) => w.deaths += 1,
                "MC" if v[0] == 1 => w.changes += 1,
                "MT" => w.clips += 1,
                _ => {}
            }
        }
        w
    }
    fn add(&mut self, o: Self) {
        self.explosions += o.explosions;
        self.sight_blocked += o.sight_blocked;
        self.damaged += o.damaged;
        self.deaths += o.deaths;
        self.changes += o.changes;
        self.clips += o.clips;
    }
}

fn main() -> ResultHost<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let [flag, label, mode] = args.as_slice() {
        if flag == "--measure" {
            return measure(label, mode);
        }
    }
    if !args.is_empty() {
        return Err(
            "use --measure cluster_chain|dense64_fight|range_patrol|liftroom_ride|dense64_lift advance|step|capture"
                .into(),
        );
    }
    let cases = source::cases();
    assert_eq!(cases.len(), 26);
    // Headline counts of the committed C rows over the 26 scenarios: explosions
    // (`RA`), radius candidates by outcome (`RC`: not shootable, boss, out of range,
    // sight blocked, damaged), barrel deaths (`S_BEXP` entries) and crunched dying
    // barrels (`S_GIBS` entries of a barrel, `MT` code 1).
    let (mut explosions, mut codes, mut deaths, mut gibbed) = (0, [0usize; 5], 0, 0);
    for c in &cases {
        for op in 0..=c.tics {
            for (tag, v) in source::rows(c.which, op) {
                match tag.as_str() {
                    "RA" => explosions += 1,
                    "RC" => codes[usize::try_from(v[3])?] += 1,
                    "E" if v[0] == 4 && barrel_state(v[2]) == Some(0) => deaths += 1,
                    "MT" if v[1] == 1 => gibbed += 1,
                    _ => {}
                }
            }
        }
    }
    assert_eq!(
        (explosions, codes, deaths, gibbed),
        (27, [43, 0, 3, 1, 32], 28, 1)
    );
    // Every scenario whose declared inputs are public: all but the controlled
    // damage (`hit_*`) and the controlled 200 health (`script` 30), both
    // crate-internal and replayed by the library test `barrels_replay`
    // (`check_player_barrels.py`). At every op: the spawn rows (which pin the C
    // idents: map THINGs in lump order, the player body, later spawns in creation
    // order), every state entry and action (`A_Explode` among them), each
    // candidate reaching `P_CheckSight` (`RC` codes 3 and 4) against the sight
    // records whose target is the exploding barrel, each explosion's damage
    // against the `G` row after its code-4 candidate, the sector heights (`MH`)
    // and the op's work. At every committed capture the digest equals the
    // independent DSIM37 index and the capture decodes under `decode_barrels`
    // alone.
    let digests = source::capture_digests();
    let limits = DiagnosticLimits::default();
    let (mut replayed, mut ops, mut captures, mut digest) = (0, 0, 0, 0xcbf2_9ce4_8422_2325_u64);
    let (mut rust_work, mut c_work) = (Work::default(), Work::default());
    for c in cases.iter().filter(|c| c.public()) {
        let (mut s, body, things) = source::setup(c)?;
        let mut idents: BTreeMap<ActorId, i64> = things
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i64::try_from(i).unwrap()))
            .collect();
        idents.insert(body, i64::try_from(things.len())?);
        let mut next = i64::try_from(things.len())? + 1;
        for op in 0..=c.tics {
            let pending = s.pending_events().len();
            let events = if op == 0 {
                s.pending_events().to_vec()
            } else {
                source::controlled(&mut s, c, body, &things, op)?;
                s.advance(source::command(c, op))?;
                s.events()[pending..].to_vec()
            };
            let rows = source::rows(c.which, op);
            let at = |what: &str| format!("{} op{op} {what}", c.label);
            let mut spawned = Vec::new();
            for e in &events {
                if let Event::ActorState(ActorStateEvent::Initialized { actor, actor_type }) = e {
                    if op != 0 {
                        assert!(idents.insert(*actor, next).is_none());
                        next += 1;
                    }
                    spawned.push([idents[actor], i64::from(actor_type.0)]);
                }
            }
            let id = |a: &ActorId| idents[a];
            let kind = |k: i64| -> Vec<[i64; 2]> {
                rows.iter()
                    .filter(|(t, v)| t == "E" && v[0] == k)
                    .map(|(_, v)| [v[1], v[2]])
                    .collect()
            };
            assert_eq!(spawned, kind(1), "{}", at("spawns"));
            let entered: Vec<_> = events
                .iter()
                .filter_map(|e| match e {
                    Event::ActorState(ActorStateEvent::Entered { actor, state }) => {
                        Some([id(actor), i64::from(state.0)])
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(entered, kind(4), "{}", at("state entries"));
            let actions: Vec<_> = events
                .iter()
                .filter_map(|e| match e {
                    Event::ActorState(ActorStateEvent::Action {
                        actor,
                        action: ActorAction::Original(a),
                    }) => Some([id(actor), i64::from(a.0)]),
                    _ => None,
                })
                .collect();
            assert_eq!(actions, kind(5), "{}", at("actions"));
            // RC codes 3/4 as (candidate, spot, visible); the G row after code 4.
            let (mut spot, mut queried, mut damaged) = (-1, Vec::new(), Vec::new());
            for (k, (tag, v)) in rows.iter().enumerate() {
                match tag.as_str() {
                    "RA" => spot = v[0],
                    "RC" if v[3] >= 3 => {
                        queried.push([v[0], spot, i64::from(v[3] == 4)]);
                        if v[3] == 4 {
                            let g = rows[k + 1..]
                                .iter()
                                .find(|(t, g)| t == "G" && g[0] == 2)
                                .ok_or("G row")?;
                            damaged.push([g.1[1], g.1[2], g.1[3], g.1[4]]);
                        }
                    }
                    _ => {}
                }
            }
            let sight: Vec<_> = events
                .iter()
                .filter_map(|e| match e {
                    Event::Perception(PerceptionEvent::Sight {
                        observer,
                        target,
                        result,
                    }) if is_barrel(&s, *target) => {
                        Some([id(observer), id(target), i64::from(result.visible)])
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(sight, queried, "{}", at("explosion sight"));
            let damage: Vec<_> = events
                .iter()
                .filter_map(|e| match e {
                    Event::Combat(CombatEvent::Damage(r))
                        if r.inflictor.is_some_and(|i| is_barrel(&s, i)) =>
                    {
                        Some([
                            id(&r.target),
                            r.inflictor.as_ref().map_or(-1, id),
                            r.source.as_ref().map_or(-1, id),
                            i64::from(r.amount),
                        ])
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(damage, damaged, "{}", at("explosion damage"));
            if op != 0 {
                let work = Work::of_events(&s, &events);
                assert_eq!(work, Work::of_rows(rows), "{}", at("work"));
                rust_work.add(work);
                c_work.add(Work::of_rows(rows));
                let mh = source::row(rows, "MH").ok_or("MH row")?;
                assert_eq!(usize::try_from(mh[0])?, s.sectors().len());
                for (i, sector) in s.sectors().iter().enumerate() {
                    assert_eq!(
                        [
                            i64::from(sector.floor.bits()),
                            i64::from(sector.ceiling.bits())
                        ],
                        mh[1 + 3 * i..3 + 3 * i],
                        "{}",
                        at(&format!("sector {i}"))
                    );
                }
                ops += 1;
            }
            if let Some(expected) = digests.get(&(c.which, op)) {
                let snapshot = s.snapshot();
                assert_eq!(snapshot.digest(), *expected, "{}", at("digest"));
                let bytes = snapshot.diagnostic_bytes(limits)?;
                Diagnostic::decode_barrels(&bytes, limits).map_err(|e| at(&format!("{e:?}")))?;
                assert_eq!(
                    Diagnostic::decode_e1m1(&bytes, limits).err(),
                    Some(DiagnosticError::Version(37))
                );
                captures += 1;
            }
        }
        digest = (digest ^ s.snapshot().digest()).wrapping_mul(0x100_0000_01b3);
        replayed += 1;
    }
    assert_eq!(rust_work, c_work);
    assert_eq!((replayed, ops, captures), (19, 1621, 328));
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersBarrelsV1.wire_id(),
        21
    );
    assert_eq!(
        PLAYER_BARRELS_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-BARRELS-1-MEDIUM-v1"
    );
    // The E1M1 policy refuses every committed scenario room, at admission or at
    // the THING spawn: a barrel, more than 16 pickups, an angle octant 4..7 or the
    // closed-box spawn rule that S′ relaxes.
    let mut refused = 0;
    for c in &cases {
        let older =
            Simulation::original_player_e1m1_map(source::case_map(c.map), source::options(6));
        refused += usize::from(older.map_or(true, |mut s| s.spawn_map_things().is_err()));
    }
    assert_eq!(refused, 26);
    println!(
        "player_barrels: C range, cluster, player_range, monsters, walls, door, fireball, dense64, angles and embedded rooms (synthetic, not the real E1M1), 26 medium scenarios: {explosions} explosions, {} radius candidates ({} not shootable, {} boss, {} out of range, {} sight blocked, {} damaged), {deaths} barrel deaths, {gibbed} crunched in the C rows; {replayed} public-input scenarios replayed over {ops} ops with matching spawns, state entries, actions, explosion sight and damage, heights and work and {captures} captures matching the independent DSIM37 digests ({} explosions, {} sight blocked, {} damaged, {} barrel deaths, {} P_ChangeSector calls, {} thing clips); {refused} scenario rooms refused by the E1M1 policy; DSIM37 digest={digest:016x}",
        codes.iter().sum::<usize>(),
        codes[0],
        codes[1],
        codes[2],
        codes[3],
        codes[4],
        rust_work.explosions,
        rust_work.sight_blocked,
        rust_work.damaged,
        rust_work.deaths,
        rust_work.changes,
        rust_work.clips,
    );
    Ok(())
}

/// A workload: its setup and per-op commands.
///
/// - `cluster_chain`: the committed `chain_reaction` timeline (140 tics: the
///   first explosion kills four barrels, two more generations follow).
/// - `dense64_fight`: 1,200 tics in the `dense64` room (64 pickups, 16 barrels,
///   6 monsters, 27 decorations): the player walks to the barrel rows and fires
///   along them, then patrols (walking, turning, firing, `BT_USE`).
/// - `range_patrol`: 1,200 tics of the same patrol without `BT_ATTACK` in the
///   `range` hall with its barrel: no barrel damage (asserted).
/// - `liftroom_ride`: the committed M3h9 `decorations_ride_lift` timeline (a lamp
///   and a dead player ride the lift) under the barrels policy: the
///   `P_ChangeSector` work of a lift. No committed room has both a lift sector and
///   129 linked things (`dense64` is one sector).
/// - `dense64_lift`: the `P_ChangeSector` measurement at the linked-thing bound
///   `N` = 129 (M3h9b note §5.3), in a room built in memory: the committed M3h9
///   `liftroom` geometry with its THINGS lump replaced by [`dense_lift_things`]
///   (121 linked at spawn, every per-kind bound full; the 8 drops that make 129 come
///   only from kills, and nothing is killed here). The player walks back and forth
///   across the WR 88 line for 1,200 tics, so the lift runs cycle after cycle under
///   its riders. Not C evidence: a Rust-only work and timing observation.
fn workload(label: &str) -> ResultHost<(Simulation, Vec<[TicCommand; 4]>)> {
    match label {
        "cluster_chain" => {
            let c = source::case("chain_reaction");
            let (s, _, _) = source::setup(&c)?;
            Ok((s, (1..=c.tics).map(|op| source::command(&c, op)).collect()))
        }
        "dense64_fight" | "range_patrol" => {
            let fight = label == "dense64_fight";
            let name = if fight {
                "dense_pickups"
            } else {
                "barrel_wounded"
            };
            let (s, _, _) = source::setup(&source::case(name))?;
            // The command stream is fixed in advance from an unmeasured run, so a
            // measured run and the work count replay the same inputs.
            let mut patrol = source::Patrol(0x2545_f491);
            let mut commands = Vec::with_capacity(1200);
            let mut probe = s.clone();
            for tic in 0..1200 {
                let dead = probe
                    .original_player(PlayerSlot::One)
                    .map(|p| p.health <= 0)
                    .unwrap_or(true);
                let mut command = patrol.command(dead);
                if fight && tic < 120 && !dead {
                    // Turn south-east, walk to the barrel rows and fire along them.
                    command[0] = TicCommand {
                        forward: if tic < 16 {
                            0
                        } else if tic < 80 {
                            25
                        } else {
                            0
                        },
                        side: 0,
                        angle_turn: match tic {
                            0..8 => -1280,
                            80..88 => 2048,
                            _ => 0,
                        },
                        buttons: u8::from(tic >= 88),
                    };
                }
                if !fight {
                    command[0].buttons &= !1;
                }
                probe.advance(command)?;
                commands.push(command);
            }
            Ok((s, commands))
        }
        "liftroom_ride" => {
            let c = m3h9::case("decorations_ride_lift");
            let mut s = Simulation::original_player_barrels_map(
                m3h9::case_map(c.map),
                m3h9::options(c.seed),
            )?;
            s.spawn_map_things()?;
            s.spawn_player_body(
                PlayerSlot::One,
                c.position.map(doom_types::Fixed::from_map_units),
            )?;
            s.initialize_player_weapons(
                PlayerSlot::One,
                WeaponInventory {
                    ready: c.ready,
                    owned: c.owned,
                    ammo: [c.clips, 0, 0, 0],
                },
            )?;
            s.spawn_level_specials()?;
            s.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)?;
            Ok((s, (1..=c.tics).map(|op| m3h9::command(&c, op)).collect()))
        }
        "dense64_lift" => {
            let c = m3h9::case("decorations_ride_lift");
            let bytes = with_things(m3h9::MAP_BYTES[c.map], &dense_lift_things())?;
            let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default())?;
            let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default())?;
            let mut s = Simulation::original_player_barrels_map(map, m3h9::options(c.seed))?;
            let spawned = s.spawn_map_things()?.spawned;
            assert_eq!(spawned.len(), 120);
            s.spawn_player_body(
                PlayerSlot::One,
                [48, 64].map(doom_types::Fixed::from_map_units),
            )?;
            s.initialize_player_weapons(
                PlayerSlot::One,
                WeaponInventory {
                    ready: c.ready,
                    owned: c.owned,
                    ammo: [c.clips, 0, 0, 0],
                },
            )?;
            s.spawn_level_specials()?;
            assert_eq!(s.actor_order().count(), 121);
            // Turn north, then walk 24 tics north and 24 tics south, crossing the
            // WR 88 line (y = 128) at x = 48 twice per cycle; a crossing while the
            // lift moves or waits does nothing (`EV_DoPlat` skips a sector with
            // `specialdata`), so the lift restarts as soon as it is back up.
            let commands = (0..1200)
                .map(|tic| {
                    let mut commands = [TicCommand::default(); 4];
                    commands[0] = TicCommand {
                        forward: if tic == 0 {
                            0
                        } else if (tic - 1) % 48 < 24 {
                            25
                        } else {
                            -25
                        },
                        side: 0,
                        angle_turn: if tic == 0 { 16384 } else { 0 },
                        buttons: 0,
                    };
                    commands
                })
                .collect();
            Ok((s, commands))
        }
        _ => Err("unknown workload".into()),
    }
}
/// The THINGS of `dense64_lift` in the `liftroom` (sector 0 x in [0, 512], y in
/// [0, 128]; 1 y in [128, 256]; 2 the upper ledge y in [384, 640], floor 64; 3 the
/// lift x in [192, 320], y in [256, 384], floor 64, tag 1), all with skill bits 7:
/// 16 barrels in a 4 × 4 grid on the lift; 32 pickups on the lift and 32 in sector 1
/// (the `dense64` kinds: a shotgun, 4 clips, 4 shells, a stimpack, 3 medikits, 16
/// health and 29 armor bonuses, the two armors, a clip box, 3 shell boxes); 32
/// decorations (the seven E1M1 types) in sector 1; 8 monsters (4 Zombiemen, 4
/// Imps) on the ledge facing north, so the player south of them is behind their
/// field of view and never wakes them (he never fires). Everything stands in x
/// in [136, 376], inside the BLOCKMAP columns the lift's `blockbox` covers; the
/// player walks at x = 48, clear of every pickup.
fn dense_lift_things() -> Vec<[i16; 5]> {
    const ZOMBIE: i16 = 3004;
    const IMP: i16 = 3001;
    const BARREL: i16 = 2035;
    let pickups: Vec<i16> = [(2001, 1), (2007, 4), (2008, 4), (2011, 1), (2012, 3)]
        .into_iter()
        .chain([
            (2014, 16),
            (2015, 29),
            (2018, 1),
            (2019, 1),
            (2048, 1),
            (2049, 3),
        ])
        .flat_map(|(kind, n)| std::iter::repeat_n(kind, n))
        .collect();
    assert_eq!(pickups.len(), 64);
    let decorations: Vec<i16> = [
        (2028, 5),
        (48, 5),
        (35, 4),
        (24, 5),
        (15, 5),
        (10, 4),
        (12, 4),
    ]
    .into_iter()
    .flat_map(|(kind, n)| std::iter::repeat_n(kind, n))
    .collect();
    assert_eq!(decorations.len(), 32);
    let mut things = Vec::with_capacity(120);
    for i in 0..16 {
        things.push([208 + 32 * (i % 4), 272 + 32 * (i / 4), 0, BARREL, 7]);
    }
    for (i, kind) in (0i16..).zip(pickups) {
        things.push(if i < 32 {
            [200 + 16 * (i % 8), 264 + 16 * (i / 8), 0, kind, 7]
        } else {
            [152 + 32 * (i % 8), 136 + 32 * ((i - 32) / 8), 0, kind, 7]
        });
    }
    for (i, kind) in (0i16..).zip(decorations) {
        things.push([136 + 32 * (i % 8), 152 + 32 * (i / 8), 0, kind, 7]);
    }
    for i in 0..8 {
        let kind = if i % 2 == 0 { ZOMBIE } else { IMP };
        things.push([160 + 48 * (i % 4), 432 + 48 * (i / 4), 90, kind, 7]);
    }
    things
}
/// `wad` (a PWAD) with its THINGS lump replaced by `things` (ten-byte
/// `mapthing_t` records), every other lump and the directory order unchanged.
fn with_things(wad: &[u8], things: &[[i16; 5]]) -> ResultHost<Vec<u8>> {
    let word = |at: usize| -> ResultHost<usize> {
        let bytes = wad.get(at..at + 4).ok_or("short WAD")?;
        Ok(usize::try_from(u32::from_le_bytes(bytes.try_into()?))?)
    };
    let (count, directory) = (word(4)?, word(8)?);
    let replacement: Vec<u8> = things
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let mut out = wad.get(..4).ok_or("short WAD")?.to_vec();
    let mut data = Vec::new();
    let mut entries = Vec::new();
    for i in 0..count {
        let at = directory + 16 * i;
        let name = wad.get(at + 8..at + 16).ok_or("short directory")?;
        let lump = if name.starts_with(b"THINGS") {
            replacement.as_slice()
        } else {
            let (pos, size) = (word(at)?, word(at + 4)?);
            wad.get(pos..pos + size).ok_or("short lump")?
        };
        entries.push((12 + data.len(), lump.len(), name));
        data.extend_from_slice(lump);
    }
    out.extend_from_slice(&u32::try_from(count)?.to_le_bytes());
    out.extend_from_slice(&u32::try_from(12 + data.len())?.to_le_bytes());
    out.extend_from_slice(&data);
    for (pos, size, name) in entries {
        out.extend_from_slice(&u32::try_from(pos)?.to_le_bytes());
        out.extend_from_slice(&u32::try_from(size)?.to_le_bytes());
        out.extend_from_slice(name);
    }
    Ok(out)
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    if !WORKLOADS.contains(&label) {
        return Err("unknown workload".into());
    }
    let (mut sim, commands) = workload(label)?;
    let linked = sim.actor_order().count();
    // The recorded per-tic work, from an unmeasured run of the same inputs.
    let (mut total, mut peak_changes, mut peak_clips, mut peak_damaged) =
        (Work::default(), 0, 0, 0);
    let mut probe = sim.clone();
    for &command in &commands {
        let pending = probe.pending_events().len();
        probe.advance(command)?;
        let w = Work::of_events(&probe, &probe.events()[pending..]);
        peak_changes = peak_changes.max(w.changes);
        peak_clips = peak_clips.max(w.clips);
        peak_damaged = peak_damaged.max(w.damaged);
        total.add(w);
    }
    match label {
        "cluster_chain" => assert_eq!((total.explosions, total.deaths), (7, 7)),
        "dense64_fight" => assert!(total.explosions > 0, "the fight explodes barrels"),
        "range_patrol" => assert_eq!((total.explosions, total.deaths, total.damaged), (0, 0, 0)),
        "dense64_lift" => assert!(total.changes > 0 && total.clips > 0),
        _ => assert!(total.changes > 0),
    }
    let mut peak = 0;
    let (result, time) = interval(|| -> Result<(), SimError> {
        for &command in &commands {
            if mode == "advance" {
                sim.advance(command)?;
            } else if mode == "capture" {
                sim.advance(command)?;
                drop(black_box(sim.snapshot()));
            } else {
                drop(black_box(sim.step(command)?));
            }
            peak = peak.max(sim.actor_order().count());
        }
        Ok(())
    })?;
    result?;
    assert_eq!(sim.snapshot(), probe.snapshot());
    println!(
        "player_barrels label={label} mode={mode} calls={} warmup=0 linked_at_start={linked} peak_actors={peak} explosions={} sight_blocked={} damaged={} damaged_peak_tic={peak_damaged} barrel_deaths={} change_sector={} change_sector_peak_tic={peak_changes} thing_clips={} thing_clips_peak_tic={peak_clips} ns={} digest={:016x}",
        commands.len(),
        total.explosions,
        total.sight_blocked,
        total.damaged,
        total.deaths,
        total.changes,
        total.clips,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
