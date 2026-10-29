//! Asserted original-C combined gameplay. Optional timing is host tooling only.
#[path = "../../../tests/support/m3f3_clipping.rs"]
mod additional;
#[path = "../../../tests/support/m3f3.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
use std::{
    collections::BTreeSet,
    hint::black_box,
    time::{Duration, Instant},
};
fn expected(which: usize, op: usize) -> u64 {
    let row = include_str!("../../../tests/reference/m3f3_snapshots.csv")
        .lines()
        .skip(1)
        .find(|r| {
            let v: Vec<_> = r.split(',').collect();
            v[0].parse::<usize>().unwrap() == which && v[1].parse::<usize>().unwrap() == op
        })
        .unwrap();
    u64::from_str_radix(row.split(',').nth(3).unwrap(), 16).unwrap()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--measure") {
        measure(&args[2], args[3].parse().unwrap());
        return;
    }
    let cases = fixtures::cases();
    let owner = ActorId::from_parts(0, 1);
    let mut digest = 0;
    for label in ["expanded_sight_dag", "wide_map_lifecycle", "long_lifecycle"] {
        let which = cases.iter().position(|c| c.label == label).unwrap();
        let c = &cases[which];
        let (raw, report) = fixtures::map_only(c);
        let mut source = fixtures::SourceTrace::for_case(c);
        source.check_original_spawn(&raw, which);
        assert_eq!(raw.profile(), Profile::DemonExpandedMap);
        assert_eq!(report.spawned, [owner]);
        assert_eq!(
            raw.actor(owner).unwrap().life().unwrap().state,
            Some(StateId(1))
        );
        assert!(
            raw.pending_events()
                .iter()
                .all(|e| !matches!(e, Event::ActorState(ActorStateEvent::Action { .. })))
        );
        let mut a = fixtures::setup(c);
        let mut b = fixtures::setup(c);
        let mut trace = fixtures::SourceTrace::for_case(c);
        trace.check(&a, which, 0, a.pending_events());
        fixtures::assert_golden(&a, which, 0);
        let mut entered = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut expanded_sight = false;
        let mut pain_sound = None;
        let mut death_sound = None;
        let mut strikes = 0;
        let sounds = a
            .actor_registry()
            .unwrap()
            .actor_type(ActorTypeId(136))
            .unwrap()
            .sounds();
        for op in 1..=c.v("tics") as usize {
            let pending = a.pending_events().len();
            fixtures::advance(&mut a, c, op);
            fixtures::advance(&mut b, c, op);
            assert_eq!(a.snapshot(), b.snapshot());
            trace.check(&a, which, op, &a.events()[pending..]);
            fixtures::assert_golden(&a, which, op);
            visited.insert(a.actor(owner).unwrap().location().unwrap().sector);
            for event in a.events() {
                match event {
                    Event::ActorState(ActorStateEvent::Entered { actor, state })
                        if *actor == owner =>
                    {
                        entered.insert(state.0);
                    }
                    Event::ActorState(ActorStateEvent::Sound {
                        actor: Some(actor),
                        sound,
                    }) if *actor == owner => {
                        if *sound == sounds[2] {
                            pain_sound = Some(op);
                        }
                        if *sound == sounds[3] {
                            death_sound = Some(op);
                        }
                    }
                    Event::Monster(MonsterEvent::Random { actor, kind: 4, .. })
                        if *actor == owner =>
                    {
                        strikes += 1
                    }
                    Event::Perception(PerceptionEvent::Sight { result, .. })
                        if result.visits.len() > 8192 =>
                    {
                        expanded_sight = true
                    }
                    _ => (),
                }
            }
            if op == c.v("kill_tick") as usize {
                let a = a.actor(owner).unwrap();
                assert_eq!(a.height(), Fixed::from_map_units(14));
                assert_ne!(a.life().unwrap().flags & 2, 0);
            }
        }
        assert_eq!(a.snapshot().digest(), expected(which, c.v("tics") as usize));
        if label == "expanded_sight_dag" {
            assert!(expanded_sight);
            continue;
        }
        // Source speed can cross several one-unit sectors in one move.
        assert!(visited.len() > 8);
        assert!(visited.last().unwrap() - visited.first().unwrap() > 100);
        assert!(strikes > 0);
        let corpse = a.actor(owner).unwrap();
        assert_eq!(corpse.life().unwrap().state, Some(StateId(14)));
        assert_eq!(corpse.life().unwrap().tics, -1);
        assert_eq!(corpse.height(), Fixed::from_map_units(14));
        assert_eq!(corpse.life().unwrap().flags & 6, 0);
        assert_eq!(a.level_statistics().unwrap().total_kills, 1);
        assert_eq!(a.level_statistics().unwrap().single_player_kill_count, 1);
        if label == "long_lifecycle" {
            assert!(pain_sound.unwrap() > c.v("damage_tick") as usize);
        } else {
            // The independent wide-map trace has a nonlethal hit without pain.
            assert_eq!(pain_sound, None);
        }
        assert!(death_sound.unwrap() > c.v("kill_tick") as usize);
        if label == "wide_map_lifecycle" {
            assert_eq!(corpse.position()[0], Fixed::from_map_units(880));
        } else {
            assert!((2..=21).all(|state| entered.contains(&state)));
            digest = a.snapshot().digest();
        }
    }
    println!(
        "demon_expanded_map: 769 lines / 256 sectors; pursuit, melee, pain, corpse; DSIM-v20 {digest:016x}"
    );
}
fn construct(map: doom_map::Map, kind: u8) -> Simulation {
    match kind {
        17 => Simulation::with_demon_map_context_registry(
            map,
            64,
            ActorRegistry::demon_map_context_fixture(),
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .unwrap(),
        19 => Simulation::map_extent(map).unwrap(),
        20 => Simulation::with_demon_expanded_map_registry(
            map,
            64,
            ActorRegistry::demon_map_context_fixture(),
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .unwrap(),
        _ => panic!("unsupported measurement profile"),
    }
}
fn populated(c: &fixtures::Case, bytes: &[u8], kind: u8) -> Simulation {
    let mut s = construct(decoded(bytes), kind);
    if kind != 19 {
        s.spawn_map_things().unwrap();
        return fixtures::install_targets(s, c);
    }
    for thing in s.world().map().things().to_vec() {
        s.spawn(ActorSpec {
            position: [
                Fixed::from_map_units(thing.position.x),
                Fixed::from_map_units(thing.position.y),
            ],
            radius: Fixed::from_map_units(30),
            height: Fixed::from_map_units(56),
            ..Default::default()
        })
        .unwrap();
    }
    for i in 0..4 {
        s.spawn(ActorSpec {
            position: [
                Fixed::from_bits(c.v(&format!("p{i}x"))),
                Fixed::from_bits(c.v(&format!("p{i}y"))),
            ],
            radius: Fixed::from_map_units(16),
            height: Fixed::from_map_units(56),
            ..Default::default()
        })
        .unwrap();
    }
    s
}
fn decoded(bytes: &[u8]) -> doom_map::Map {
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
fn measure(label: &str, kind: u8) {
    let (c, bytes) = if label == "high_cell_demon" {
        let c = additional::cases()
            .into_iter()
            .find(|c| c.label == label)
            .unwrap();
        let bytes = additional::map_bytes(c.v("map") as usize);
        // Both adapters deliberately use the same declared source-input schema.
        (
            fixtures::Case {
                label: c.label,
                values: c.values,
            },
            bytes,
        )
    } else {
        let c = fixtures::cases()
            .into_iter()
            .find(|c| c.label == label)
            .unwrap();
        let bytes = fixtures::map_bytes(c.v("map") as usize);
        (c, bytes)
    };
    let mut parse = Duration::ZERO;
    let mut prepare = Duration::ZERO;
    let mut batch = Duration::ZERO;
    for _ in 0..100 {
        let start = Instant::now();
        let map = black_box(decoded(bytes));
        parse += start.elapsed();
        let start = Instant::now();
        let mut s = black_box(construct(map, kind));
        prepare += start.elapsed();
        if kind != 19 {
            let start = Instant::now();
            black_box(s.spawn_map_things().unwrap());
            batch += start.elapsed();
        }
    }
    let mut s = populated(&c, bytes, kind);
    let owner = ActorId::from_parts(0, 1);
    let target = ActorId::from_parts(s.world().map().things().len() as u32, 1);
    let metrics = match kind {
        19 => Some(s.map_extent_metrics().unwrap()),
        20 => Some(s.demon_expanded_map_metrics().unwrap()),
        _ => None,
    };
    let start = Instant::now();
    let mut candidates = 0;
    let point = s.actor(owner).unwrap().position();
    for _ in 0..1000 {
        let q = black_box(
            s.check_multisector_position(owner, [point[0] + Fixed::from_map_units(10), point[1]])
                .unwrap(),
        );
        candidates += q.movement.lines.candidates.len() + q.movement.actors.len();
    }
    let collision = start.elapsed();
    let start = Instant::now();
    let mut visits = 0;
    for _ in 0..1000 {
        visits += black_box(s.check_sight(owner, target).unwrap())
            .visits
            .len();
    }
    let sight = start.elapsed();
    let mut chase = Duration::ZERO;
    if kind != 19 {
        for _ in 0..100 {
            let mut fresh = populated(&c, bytes, kind);
            fresh.set_actor_target(owner, Some(target)).unwrap();
            let start = Instant::now();
            fresh.chase(owner).unwrap();
            chase += start.elapsed();
        }
    }
    let start = Instant::now();
    for op in 1..=100 {
        if kind == 19 {
            s.step([TicCommand::default(); 4]).unwrap();
        } else {
            fixtures::advance(&mut s, &c, op);
        }
    }
    let step = start.elapsed();
    let start = Instant::now();
    let mut snapshot_bytes = 0;
    for _ in 0..100 {
        snapshot_bytes += black_box(s.snapshot().canonical_bytes()).len();
    }
    let snapshot = start.elapsed();
    println!(
        "label={label} profile={kind} parse100_us={} prepare100_us={} spawn100_us={} collision1000_us={} candidates={candidates} sight1000_us={} visits={visits} chase100_us={} step100_us={} snapshot100_us={} snapshot_bytes={snapshot_bytes} lines={} sectors={} cells={} references={} prepared_bytes={}",
        parse.as_micros(),
        prepare.as_micros(),
        batch.as_micros(),
        collision.as_micros(),
        sight.as_micros(),
        chase.as_micros(),
        step.as_micros(),
        snapshot.as_micros(),
        s.world().map().linedefs().len(),
        s.sectors().len(),
        s.world().map().blockmap().cell_ranges().len(),
        metrics.map_or_else(|| "-".into(), |m| m.prepared_references.to_string()),
        metrics.map_or_else(|| "-".into(), |m| m.prepared_storage_bytes.to_string())
    );
}
