//! Original two-sided scrolling walls, static decorations and the spawn filter
//! under `PlayerMonstersE1M1V1`: the committed C scenarios through the public API,
//! and separate measurements. The rooms are the synthetic M3h9 rooms of the C
//! evidence (`scrollhall`, `scrollblock`, `dense8`, `gallery`, `liftroom`,
//! `census`), not the real E1M1: the policy is named for the map it prepares.
#[path = "../../../tests/support/m3h9.rs"]
mod source;
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
const WORKLOADS: [&str; 4] = [
    "gallery_patrol",
    "liftroom_ride",
    "dense8_cross",
    "scrollhall_fight",
];

/// Mover and line work of one tic's events: `T_MovePlane` steps, `P_ChangeSector`
/// calls and `PIT_ChangeSector` thing visits, crossings, `P_UseSpecialLine` calls
/// and `P_Move` failure branches with `spechit`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Work {
    steps: usize,
    changes: usize,
    clips: usize,
    crossings: usize,
    uses: usize,
    blocked: usize,
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
                _ => {}
            }
        }
        w
    }
    /// The same counts from one op's C rows (`MV`, `MC` code 1, `MT`, `MX`, `MU`,
    /// `MP`).
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
            "use --measure gallery_patrol|liftroom_ride|dense8_cross|scrollhall_fight advance|step|capture"
                .into(),
        );
    }
    let cases = source::cases();
    assert_eq!(source::all_cases().len(), 20);
    assert_eq!(cases.len(), 18);
    // Headline counts of the committed C rows over the 18 medium scenarios: mover
    // work, and the M3h9 observers (`DT`: PIT_CheckThing on a non-shootable thing;
    // `XT`: an aim or shoot trace skipping one).
    let (mut c_work, mut checks, mut skips) = (Work::default(), 0, 0);
    for c in &cases {
        for op in 0..=c.tics {
            let rows = source::rows(c.which, op);
            c_work.add(Work::of_rows(rows));
            checks += rows.iter().filter(|(t, _)| t == "DT").count();
            skips += rows.iter().filter(|(t, _)| t == "XT").count();
        }
    }
    assert_eq!(
        c_work,
        Work {
            steps: 95,
            changes: 95,
            clips: 222,
            crossings: 11,
            uses: 15,
            blocked: 10,
        }
    );
    assert_eq!((checks, skips), (205, 5));
    // The alcove scenario is refused at load (the closed-box spawn rule) with the
    // state unchanged.
    for label in source::REFUSED_AT_LOAD {
        let c = source::case(label);
        let mut s =
            Simulation::original_player_e1m1_map(source::case_map(c.map), source::options(c.seed))?;
        let before = s.snapshot();
        assert!(s.spawn_map_things().is_err());
        assert_eq!(s.snapshot(), before);
    }
    // Every loadable scenario whose declared inputs are public: all but the
    // `script` 30 ones (the controlled 200 health, crate-internal, replayed by the
    // library test `e1m1_replay`; `check_player_e1m1.py`). At every op the sector
    // heights equal the C `MH` row and the op's work its C rows; at every committed
    // capture the digest equals the independent DSIM36 index, and the capture
    // decodes under `decode_e1m1` alone; after the last op the runtime line
    // specials equal the C `ML` row.
    let digests = source::capture_digests();
    let limits = DiagnosticLimits::default();
    let (mut replayed, mut ops, mut captures, mut digest) = (0, 0, 0, 0xcbf2_9ce4_8422_2325_u64);
    let mut rust_work = Work::default();
    for c in source::loadable_cases().iter().filter(|c| c.script != 30) {
        let (mut s, body, things) = source::setup(c)?;
        for op in 0..=c.tics {
            if op != 0 {
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
            if let Some(expected) = digests.get(&(c.which, op)) {
                let snapshot = s.snapshot();
                assert_eq!(snapshot.digest(), *expected, "{} op{op} digest", c.label);
                let bytes = snapshot.diagnostic_bytes(limits)?;
                Diagnostic::decode_e1m1(&bytes, limits)
                    .map_err(|e| format!("{} op{op}: {e:?}", c.label))?;
                assert!(Diagnostic::decode_movers(&bytes, limits).is_err());
                captures += 1;
            }
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
    assert_eq!((replayed, ops, captures), (11, 604, 160));
    assert_eq!(OriginalExecutionProfile::PlayerMonstersE1M1V1.wire_id(), 20);
    assert_eq!(
        PLAYER_E1M1_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-E1M1-1-MEDIUM-v1"
    );
    // The movers policy refuses every committed room: the two-sided 48 rooms at
    // admission (rule 1), the rooms with decorations at the THING spawn
    // (`UnsupportedType`).
    for index in 0..source::MAP_BYTES.len() {
        let older =
            Simulation::original_player_movers_map(source::case_map(index), source::options(6));
        assert!(older.map_or(true, |mut s| s.spawn_map_things().is_err()));
    }
    println!(
        "player_e1m1: C scrollhall, scrollblock, dense8, gallery, liftroom and census rooms (synthetic, not the real E1M1), 18 medium scenarios: {} plane steps, {} P_ChangeSector calls, {} thing clips, {} crossings, {} uses, {} spechit-blocked moves, {checks} PIT_CheckThing non-shootable tests, {skips} trace skips in the C rows; {replayed} public-input scenarios replayed over {ops} ops with matching heights, work and line specials and {captures} captures matching the independent DSIM36 digests ({} plane steps, {} thing clips); DSIM36 digest={digest:016x}",
        c_work.steps,
        c_work.changes,
        c_work.clips,
        c_work.crossings,
        c_work.uses,
        c_work.blocked,
        rust_work.steps,
        rust_work.clips,
    );
    Ok(())
}

/// A workload: its setup and per-op commands. `gallery_patrol` is 1,200 tics of a
/// deterministic patrol (walking, turning, firing and `BT_USE`) in the gallery with
/// its ten decorations and a Zombieman; `scrollhall_fight` the same patrol in the
/// scroller hall with an Imp in the strip; `liftroom_ride` and `dense8_cross` are
/// the full committed timelines of `decorations_ride_lift` (a lamp and a corpse
/// ride the lift down and up) and `dense_spechit_8` (eight two-sided scrollers
/// crossed in one window).
fn workload(label: &str) -> ResultHost<(Simulation, Vec<[TicCommand; 4]>)> {
    match label {
        "gallery_patrol" | "scrollhall_fight" => {
            let name = if label == "gallery_patrol" {
                "monster_path_around_pillar"
            } else {
                "fireball_through_scroller"
            };
            let (s, _, _) = source::setup(&source::case(name))?;
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
        "liftroom_ride" | "dense8_cross" => {
            let name = if label == "liftroom_ride" {
                "decorations_ride_lift"
            } else {
                "dense_spechit_8"
            };
            let c = source::case(name);
            let (s, _, _) = source::setup(&c)?;
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
    // The recorded per-tic work, from an unmeasured run of the same inputs.
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
        "player_e1m1 label={label} mode={mode} calls={} warmup=0 peak_actors={peak} plane_steps={} change_sector={} change_sector_peak_tic={peak_changes} thing_clips={} thing_clips_peak_tic={peak_clips} crossings={} uses={} blocked={} ns={} digest={:016x}",
        commands.len(),
        total.steps,
        total.changes,
        total.clips,
        total.crossings,
        total.uses,
        total.blocked,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
