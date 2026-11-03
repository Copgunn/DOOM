//! Original doors, lifts, floors, switches and the exit: the committed C scenarios
//! through the public API, and separate measurements.
#[path = "../../../tests/support/m3h8.rs"]
mod source;
mod older {
    include!("../../../tests/reference/m3h7_maps.rs");
}
use doom_sim::*;
use std::{
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
const WORKLOADS: [&str; 3] = ["doorway_patrol", "lift_cycle", "switch_cycle"];

/// Mover work of one tic's events: `T_MovePlane` steps, `P_ChangeSector` calls and
/// `PIT_ChangeSector` thing visits, crossings, `P_UseSpecialLine` calls and
/// `P_Move` failure branches with `spechit`, exits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Work {
    steps: usize,
    changes: usize,
    clips: usize,
    crossings: usize,
    uses: usize,
    blocked: usize,
    exits: usize,
}
impl Work {
    fn of_events(events: &[Event]) -> Self {
        let mut w = Self::default();
        for event in events {
            match event {
                Event::Mover(MoverEvent::PlaneStep { .. }) => w.steps += 1,
                Event::Mover(MoverEvent::SectorChanged { .. }) => w.changes += 1,
                Event::Mover(MoverEvent::ThingClipped { .. }) => w.clips += 1,
                Event::Mover(MoverEvent::Crossed { .. }) => w.crossings += 1,
                Event::Mover(MoverEvent::SpecialUse { .. }) => w.uses += 1,
                Event::Mover(MoverEvent::SpechitBlocked { .. }) => w.blocked += 1,
                Event::Mover(MoverEvent::Exit { .. }) => w.exits += 1,
                _ => {}
            }
        }
        w
    }
    /// The same counts from one op's C rows (`MV`, `MC` exits, `MT`, `MX`, `MU`,
    /// `MP`, and `MG` code 0: `G_ExitLevel`).
    fn of_rows(rows: &[(String, Vec<i64>)]) -> Self {
        let mut w = Self::default();
        for (tag, v) in rows {
            match tag.as_str() {
                "MV" => w.steps += 1,
                "MC" if v[0] == 1 => w.changes += 1,
                "MT" => w.clips += 1,
                "MX" => w.crossings += 1,
                "MU" => w.uses += 1,
                "MP" => w.blocked += 1,
                "MG" if v[0] == 0 => w.exits += 1,
                _ => {}
            }
        }
        w
    }
    fn add(&mut self, o: Self) {
        self.steps += o.steps;
        self.changes += o.changes;
        self.clips += o.clips;
        self.crossings += o.crossings;
        self.uses += o.uses;
        self.blocked += o.blocked;
        self.exits += o.exits;
    }
}

fn main() -> ResultHost<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let [flag, label, mode] = args.as_slice() {
        if flag == "--measure" {
            return measure(label, mode);
        }
    }
    if let [flag, label] = args.as_slice() {
        if flag == "--admit" {
            return admit(label);
        }
    }
    if !args.is_empty() {
        return Err(
            "use --measure doorway_patrol|lift_cycle|switch_cycle advance|step|capture, or --admit doorway|switchyard|lift|floors|dense|pools"
                .into(),
        );
    }
    let cases = source::cases();
    assert_eq!(cases.len(), 32);
    // Headline counts of the committed C rows over all 32 scenarios.
    let mut c_work = Work::default();
    for c in &cases {
        for op in 0..=c.tics {
            c_work.add(Work::of_rows(source::rows(c.which, op)));
        }
    }
    assert_eq!(
        c_work,
        Work {
            steps: 1742,
            changes: 1749,
            clips: 2032,
            crossings: 47,
            uses: 221,
            blocked: 202,
            exits: 2,
        }
    );
    // Every scenario whose declared inputs are public: all but the five `script` 30
    // scenarios (the controlled 200 health) and the two whose controlled damage
    // kills (`door_gibs_drop_bonus`, `dead_use_reborn`: the public `damage_actor`
    // refuses a kill branch), both crate-internal inputs replayed by the library
    // test `movers_replay::full` (`check_player_movers.py`). At every op the sector heights equal the
    // C `MH` row and the op's mover work equals its C rows; after the last op the
    // runtime line specials of the capture equal the C `ML` row.
    let (mut replayed, mut ops, mut digest) = (0, 0, 0xcbf2_9ce4_8422_2325_u64);
    let mut rust_work = Work::default();
    for c in cases.iter().filter(|c| c.script != 30 && c.hit_at == 0) {
        let (mut s, body, things) = source::setup(c);
        for op in 1..=c.tics {
            source::controlled(&mut s, c, body, &things, op)?;
            s.advance(source::command(c, op))?;
            let rows = source::rows(c.which, op);
            let work = Work::of_events(s.events());
            assert_eq!(work, Work::of_rows(rows), "{} op{op}", c.label);
            rust_work.add(work);
            let mh = source::row(rows, "MH").ok_or("MH row")?;
            assert_eq!(usize::try_from(mh[0])?, s.sectors().len());
            for (i, sector) in s.sectors().iter().enumerate() {
                assert_eq!(
                    [
                        i64::from(sector.floor.bits()),
                        i64::from(sector.ceiling.bits())
                    ],
                    mh[1 + 3 * i..3 + 3 * i],
                    "{} op{op} sector {i}",
                    c.label
                );
            }
            ops += 1;
        }
        let snapshot = s.snapshot();
        let ml = source::row(source::rows(c.which, c.tics), "ML").ok_or("ML row")?;
        for pair in ml.chunks(2) {
            let field = snapshot
                .fields()
                .find(|f| f.path() == format!("line.{}.special", pair[0]))
                .ok_or("line special field")?;
            assert_eq!(
                i64::from(i16::from_le_bytes(field.value().try_into()?)),
                pair[1],
                "{} line {}",
                c.label,
                pair[0]
            );
        }
        digest = (digest ^ snapshot.digest()).wrapping_mul(0x100_0000_01b3);
        replayed += 1;
    }
    assert_eq!((replayed, ops), (23, 4651));
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersMoversV1.wire_id(),
        19
    );
    assert_eq!(
        PLAYER_MOVERS_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1"
    );
    // The older policy rejects every committed movers room.
    for index in 0..source::MAP_BYTES.len() {
        assert!(
            Simulation::original_player_specials_map(source::case_map(index), source::options(6))
                .is_err()
        );
    }
    println!(
        "player_movers: C doorway, hall, switchyard, lift, floors and dense rooms, 32 scenarios: {} plane steps, {} P_ChangeSector calls, {} thing clips, {} crossings, {} uses, {} spechit-blocked moves, {} exits in the C rows; {replayed} public-input scenarios replayed over {ops} ops with matching heights, mover work and line specials ({} plane steps, {} P_ChangeSector calls); DSIM35 digest={digest:016x}",
        c_work.steps,
        c_work.changes,
        c_work.clips,
        c_work.crossings,
        c_work.uses,
        c_work.blocked,
        c_work.exits,
        rust_work.steps,
        rust_work.changes,
    );
    Ok(())
}

/// A workload: its setup and per-op commands. `doorway_patrol` is 1,200 tics of a
/// deterministic patrol (walking, turning, firing and `BT_USE`) in the doorway room
/// with a monster behind each door; `lift_cycle` and `switch_cycle` are the full
/// committed timelines of `lift_last_step_short` (rides down, up and the short last
/// step) and `switch_sr_door` (three SR uses with buttons and door cycles).
fn workload(label: &str) -> ResultHost<(Simulation, Vec<[TicCommand; 4]>)> {
    match label {
        "doorway_patrol" => {
            let mut c = source::case("door_use_wait_close");
            c.map = 1;
            let (s, _, _) = source::setup(&c);
            // The command stream is fixed in advance from an unmeasured run, so a
            // measured run and the work count replay the same inputs.
            let mut patrol = source::Patrol(0x2545_f491);
            let mut commands = Vec::with_capacity(1200);
            let mut probe = s.clone();
            for _ in 0..1200 {
                let dead = probe
                    .original_player(PlayerSlot::One)
                    .map(|p| p.health <= 0)
                    .unwrap_or(true);
                let command = patrol.command(dead);
                probe.advance(command)?;
                commands.push(command);
            }
            Ok((s, commands))
        }
        "lift_cycle" | "switch_cycle" => {
            let name = if label == "lift_cycle" {
                "lift_last_step_short"
            } else {
                "switch_sr_door"
            };
            let c = source::case(name);
            let (s, _, _) = source::setup(&c);
            Ok((s, (1..=c.tics).map(|op| source::command(&c, op)).collect()))
        }
        _ => Err("unknown workload".into()),
    }
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    if !WORKLOADS.contains(&label) {
        return Err("unknown workload".into());
    }
    let (mut sim, commands) = workload(label)?;
    // The recorded per-tic mover work, from an unmeasured run of the same inputs.
    let (mut total, mut peak_changes, mut peak_clips) = (Work::default(), 0, 0);
    let mut probe = sim.clone();
    for &command in &commands {
        probe.advance(command)?;
        let w = Work::of_events(probe.events());
        peak_changes = peak_changes.max(w.changes);
        peak_clips = peak_clips.max(w.clips);
        total.add(w);
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
        "player_movers label={label} mode={mode} calls={} warmup=0 peak_actors={peak} plane_steps={} change_sector={} change_sector_peak_tic={peak_changes} thing_clips={} thing_clips_peak_tic={peak_clips} crossings={} uses={} ns={} digest={:016x}",
        commands.len(),
        total.steps,
        total.changes,
        total.clips,
        total.crossings,
        total.uses,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}

/// Load time of a committed room under the movers policy: the parsed map is built
/// outside the timer; the admission (the Stage A rules plus the §7 mover rules: the
/// families, the reachable-height fixpoint and the `spechit` window) and the THING
/// and body spawns and `spawn_level_specials` inside it. `admit_ns` is the
/// admission alone. For `pools` (the M3h7 room, no mover) `specials_admit_ns` is the
/// same admission under the specials policy, which has no mover rules.
fn admit(label: &str) -> ResultHost<()> {
    let (map, position, specials) = match label {
        "pools" => {
            let wad = doom_wad::Wad::parse(older::MAP_BYTES[0], doom_wad::Limits::default())?;
            let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default())?;
            // The committed `pools_lights_soak` start (M3h7 map 0).
            (map.clone(), [768, -200], Some(map))
        }
        _ => {
            let name = match label {
                "doorway" => "door_use_wait_close",
                "switchyard" => "switch_sr_door",
                "lift" => "lift_player",
                "floors" => "floor_turbo_jump_up",
                "dense" => "dense_spechit_8",
                _ => return Err("unknown room".into()),
            };
            let c = source::case(name);
            (source::case_map(c.map), c.position, None)
        }
    };
    let (lines, sectors, cells) = (
        map.linedefs().len(),
        map.sectors().len(),
        map.blockmap().cell_ranges().len(),
    );
    let (result, time) = interval(|| -> Result<_, Box<dyn Error>> {
        let start = Instant::now();
        let mut sim = Simulation::original_player_movers_map(black_box(map), source::options(6))?;
        let admission = start.elapsed();
        sim.spawn_map_things()?;
        sim.spawn_player_body(
            PlayerSlot::One,
            position.map(doom_types::Fixed::from_map_units),
        )?;
        sim.spawn_level_specials()?;
        Ok((sim, admission))
    })?;
    let (sim, admission) = result?;
    // The specials policy's admission of the same room, after the movers one.
    let specials = match specials {
        Some(map) => {
            let start = Instant::now();
            let older =
                Simulation::original_player_specials_map(black_box(map), source::options(6))?;
            let elapsed = start.elapsed();
            drop(black_box(older));
            Some(elapsed)
        }
        None => None,
    };
    println!(
        "player_movers admit={label} linedefs={lines} sectors={sectors} cells={cells} ns={} admit_ns={} specials_admit_ns={} digest={:016x}",
        time.as_nanos(),
        admission.as_nanos(),
        specials.map_or(0, |d| d.as_nanos()),
        sim.snapshot().digest()
    );
    Ok(())
}
