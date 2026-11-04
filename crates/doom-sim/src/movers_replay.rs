//! A partial replay of the committed M3h8 C scenarios (plan steps 9–11): each case's
//! setup and per-op commands (`UC` rows) drive the Rust movers policy, and the
//! per-op sector heights and `specialdata` presence (`MH`), runtime line specials
//! (`ML`), use latch, player state and exit (`MZ`) and the crossings (`MX`/`MY`) are
//! compared with the C rows; [`full`] is the full replay of step 13 (thinker records,
//! buttons, the event stream and every DSIM35 capture against the independent
//! goldens). Expected values come only from the C rows and the independent goldens.
use crate::identity::OriginalState;
use crate::*;
use std::{collections::BTreeMap, sync::OnceLock};

// The frozen evidence decoder is shared with the integration-test readers.
#[allow(clippy::duplicate_mod)]
#[path = "../../../tests/support/evidence_pack.rs"]
mod packed;

type Rows = BTreeMap<(usize, usize), Vec<(String, Vec<i64>)>>;
/// Every trace row keyed by (case, op).
pub(crate) fn rows() -> &'static Rows {
    static DATA: OnceLock<Rows> = OnceLock::new();
    DATA.get_or_init(|| {
        let text =
            packed::unpack(include_bytes!("../../../tests/reference/m3h8_trace.dref")).unwrap();
        let text = std::str::from_utf8(&text).unwrap();
        let mut rows = Rows::new();
        for line in text.lines().skip(1) {
            let mut fields = line.split(',');
            let tag = fields.next().unwrap().to_string();
            let values: Vec<i64> = fields.map(|f| f.parse().unwrap()).collect();
            if values.len() < 2 {
                continue;
            }
            rows.entry((values[0] as usize, values[1] as usize))
                .or_default()
                .push((tag, values[2..].to_vec()));
        }
        rows
    })
}
fn op_rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    rows().get(&(which, op)).map_or(&[], Vec::as_slice)
}
fn row<'a>(rows: &'a [(String, Vec<i64>)], tag: &str) -> Option<&'a [i64]> {
    rows.iter()
        .find(|(t, _)| t == tag)
        .map(|(_, v)| v.as_slice())
}

#[derive(Clone, Debug)]
pub(crate) struct Case {
    pub which: usize,
    pub label: String,
    pub tics: usize,
    pub position: [i16; 2],
    pub seed: u8,
    pub hit_at: usize,
    pub hit_thing: usize,
    pub hit_amount: i32,
    pub attack_at: usize,
    pub attack_thing: usize,
    /// Controlled between-tic player target plus see-state entry (`chase_at`).
    pub chase_at: usize,
    pub chase_thing: usize,
    pub map: usize,
    pub clips: i32,
    pub ready: Weapon,
    pub owned: u16,
    pub reaction: u8,
    /// `script` 30: the declared controlled 200 health of M3g11 (player and body).
    pub script: u8,
}
pub(crate) fn cases() -> Vec<Case> {
    let mut lines = include_str!("../../../tests/reference/m3h8_cases.csv").lines();
    let header: Vec<_> = lines.next().unwrap().split(',').collect();
    lines
        .enumerate()
        .map(|(which, line)| {
            let v: BTreeMap<_, _> = header.iter().copied().zip(line.split(',')).collect();
            let n = |k: &str| v[k].parse::<i64>().unwrap();
            // A case that requested an exit or a reborn ends at that op.
            let last = rows()
                .range((which, 0)..(which + 1, 0))
                .filter(|(_, r)| r.iter().any(|(t, _)| t == "UC"))
                .map(|((_, op), _)| *op)
                .max()
                .unwrap();
            Case {
                which,
                label: v["label"].into(),
                tics: (n("tics") as usize).min(last),
                position: [n("px") as i16, n("py") as i16],
                seed: n("seed") as u8,
                hit_at: n("hit_at") as usize,
                hit_thing: n("hit_thing") as usize,
                hit_amount: n("hit_amount") as i32,
                attack_at: n("attack_at") as usize,
                attack_thing: n("attack_thing") as usize,
                chase_at: n("chase_at") as usize,
                chase_thing: n("chase_thing") as usize,
                map: n("map") as usize,
                clips: n("clips") as i32,
                ready: match n("ready") {
                    0 => Weapon::Fist,
                    1 => Weapon::Pistol,
                    2 => Weapon::Shotgun,
                    _ => Weapon::Chaingun,
                },
                owned: n("owned") as u16,
                reaction: n("reaction") as u8,
                script: n("script") as u8,
            }
        })
        .collect()
}
pub(crate) fn case(label: &str) -> Case {
    cases().into_iter().find(|c| c.label == label).unwrap()
}
/// The case's command at `op` (its `UC` row).
pub(crate) fn command(c: &Case, op: usize) -> [TicCommand; 4] {
    let v = row(op_rows(c.which, op), "UC").expect("UC row");
    let mut commands = [TicCommand::default(); 4];
    commands[0] = TicCommand {
        forward: v[0] as i8,
        side: v[1] as i8,
        angle_turn: v[2] as i16,
        buttons: v[3] as u8,
    };
    commands
}

/// The C driver's setup: admission, THINGs, the player body, weapons, then
/// `P_SpawnSpecials`, as in the M3h7 harness.
pub(crate) fn setup(c: &Case) -> (Simulation, ActorId, Vec<ActorId>) {
    // The committed room bytes, exactly as the C harness reads them.
    setup_map(c, crate::player_movers_tests::room(c.map).map())
}
pub(crate) fn setup_map(c: &Case, map: doom_map::Map) -> (Simulation, ActorId, Vec<ActorId>) {
    let mut s = Simulation::original_player_movers_map(
        map,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(c.seed, 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let things = s.spawn_map_things().unwrap().spawned;
    let body = s
        .spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
        .unwrap();
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: c.ready,
            owned: c.owned,
            ammo: [c.clips, 0, 0, 0],
        },
    )
    .unwrap();
    s.spawn_level_specials().unwrap();
    s.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)
        .unwrap();
    if c.script == 30 {
        s.original_player.as_mut().unwrap().health = 200;
        s.actors
            .get_mut(body)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .health = 200;
    }
    (s, body, things)
}
/// The declared between-tic inputs: the damage (thing `hit_thing`, the player when
/// the room has no THING), the attack entry and the see-state entry (`chase_at`:
/// `P_SetMobjState(seestate)` runs the state's `A_Chase` at once, outside any tic).
pub(crate) fn controlled(
    s: &mut Simulation,
    c: &Case,
    body: ActorId,
    things: &[ActorId],
    op: usize,
) {
    if op == c.hit_at {
        let target = things.get(c.hit_thing).copied().unwrap_or(body);
        let request = DamageRequest {
            target,
            inflictor: None,
            source: None,
            amount: c.hit_amount,
        };
        // P_DamageMobj through the shared checked transaction (the drop of a killed
        // former human is admitted there, as inside the tic).
        s.checked_demon_operation(|h| -> Result<(), SimError> {
            let plan = combat::DamagePlan::current_original(h, request)?;
            lifecycle::ActorAccess::reserve(&mut h.access, plan.spawn_count())?;
            h.apply_damage(plan);
            Ok(())
        })
        .and_then(|r| r)
        .unwrap_or_else(|e| panic!("{} hit: {e}", c.label));
    }
    if op == c.attack_at {
        let imp = things[c.attack_thing];
        s.set_actor_target(imp, Some(body)).unwrap();
        s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
            .unwrap();
    }
    if op == c.chase_at {
        // Every chase case declares a zombieman, whose see state is S_POSS_RUN1.
        let zombie = things[c.chase_thing];
        s.set_actor_target(zombie, Some(body)).unwrap();
        s.enter_original_state(zombie, OriginalState::S_POSS_RUN1)
            .unwrap();
    }
}

/// What a replay compared.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Compared {
    /// Ops whose `MH` and `ML` rows were compared.
    pub ops: usize,
    /// Crossings compared (`MX`/`MY` pairs).
    pub crossings: usize,
    /// `P_UseSpecialLine` and `PTR_UseTraverse` records compared (`MU`, `MI`).
    pub uses: usize,
    /// `P_Move` failure branches with `spechit` compared (`MP`).
    pub blocked: usize,
}
/// Replays `c` through op `upto`, comparing at every op the sector heights and
/// `specialdata` presence (`MH`), the runtime line specials (`ML`) and the
/// crossings of the op (`MX`/`MY`, in order).
pub(crate) fn replay(c: &Case, upto: usize) -> (Simulation, Compared) {
    replay_observed(c, upto, |_, _| {})
}
/// [`replay`], calling `observe` with the state after every op (op 0: the setup).
pub(crate) fn replay_observed(
    c: &Case,
    upto: usize,
    mut observe: impl FnMut(&Simulation, usize),
) -> (Simulation, Compared) {
    let (mut s, body, things) = setup(c);
    let mut compared = Compared::default();
    for op in 0..=upto.min(c.tics) {
        let pending = s.pending_events().len();
        if op != 0 {
            controlled(&mut s, c, body, &things, op);
            s.advance(command(c, op))
                .unwrap_or_else(|e| panic!("{} op{op}: {e}", c.label));
        }
        let events = if op == 0 {
            s.pending_events()
        } else {
            &s.events()[pending..]
        };
        let rows = op_rows(c.which, op);
        let mh = row(rows, "MH").unwrap_or_else(|| panic!("MH {} op{op}", c.label));
        assert_eq!(mh[0] as usize, s.sectors.len());
        for (i, sector) in s.sectors.iter().enumerate() {
            let v = &mh[1 + 3 * i..4 + 3 * i];
            assert_eq!(
                [
                    i64::from(sector.floor.bits()),
                    i64::from(sector.ceiling.bits())
                ],
                v[..2],
                "{} op{op} sector {i} heights",
                c.label
            );
            assert_eq!(
                movers::special_data(&s.actors, i as u16).is_some(),
                v[2] >= 0,
                "{} op{op} sector {i} specialdata",
                c.label
            );
        }
        let ml = row(rows, "ML").unwrap_or_else(|| panic!("ML {} op{op}", c.label));
        for pair in ml.chunks(2) {
            assert_eq!(
                i64::from(s.line_specials[pair[0] as usize]),
                pair[1],
                "{} op{op} line {}",
                c.label,
                pair[0]
            );
        }
        let expected: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "MX" || t == "MY")
            .collect();
        let got: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::Mover(MoverEvent::Crossed {
                    line,
                    side,
                    before,
                    after,
                    ..
                }) => Some((*line, *side, *before, *after)),
                _ => None,
            })
            .collect();
        assert_eq!(
            expected.len(),
            2 * got.len(),
            "{} op{op} crossings",
            c.label
        );
        for (pair, (line, side, before, after)) in expected.chunks(2).zip(&got) {
            let (mx, my) = (&pair[0].1, &pair[1].1);
            assert_eq!(
                [mx[1], mx[2], mx[3], my[2]],
                [
                    i64::from(*line),
                    i64::from(*side),
                    i64::from(*before),
                    i64::from(*after)
                ],
                "{} op{op} crossing",
                c.label
            );
            compared.crossings += 1;
        }
        // MZ: usedown, playerstate (2 = PST_REBORN), gameaction (6 = ga_completed),
        // secretexit.
        let mz = row(rows, "MZ").unwrap_or_else(|| panic!("MZ {} op{op}", c.label));
        let p = s.original_player.as_ref().unwrap();
        let state = if p.reborn {
            2
        } else {
            i64::from(p.state == PlayerBodyState::Dead)
        };
        assert_eq!(
            [
                i64::from(p.use_down),
                state,
                if s.level_exit { 6 } else { 0 },
                0
            ],
            mz[..4],
            "{} op{op} MZ",
            c.label
        );
        // MU and MI: every P_UseSpecialLine and PTR_UseTraverse of the op, in order.
        let mu: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "MU")
            .map(|(_, v)| [v[1], v[2], v[3], v[4], v[5]])
            .collect();
        let uses: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::Mover(MoverEvent::SpecialUse {
                    line,
                    side,
                    before,
                    result,
                    after,
                    ..
                }) => Some([
                    i64::from(*line),
                    i64::from(*side),
                    i64::from(*before),
                    i64::from(*result),
                    i64::from(*after),
                ]),
                _ => None,
            })
            .collect();
        assert_eq!(uses, mu, "{} op{op} uses", c.label);
        // MP: the P_Move failure branches with spechit, in order (numspechit).
        let mp: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "MP")
            .map(|(_, v)| v[2])
            .collect();
        let blocked: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::Mover(MoverEvent::SpechitBlocked { count, .. }) => Some(i64::from(*count)),
                _ => None,
            })
            .collect();
        assert_eq!(blocked, mp, "{} op{op} spechit-blocked moves", c.label);
        compared.blocked += blocked.len();
        let mi: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "MI")
            .map(|(_, v)| [v[0], v[1], v[4]])
            .collect();
        let intercepts: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::Mover(MoverEvent::UseIntercept {
                    line,
                    frac,
                    continues,
                }) => Some([
                    i64::from(*line),
                    i64::from(frac.bits()),
                    i64::from(*continues),
                ]),
                _ => None,
            })
            .collect();
        assert_eq!(intercepts, mi, "{} op{op} use intercepts", c.label);
        compared.uses += uses.len() + intercepts.len();
        compared.ops += 1;
        observe(&s, op);
    }
    (s, compared)
}

/// The full replay of plan step 13: every committed scenario through its last op
/// against the C rows and the independent DSIM35 goldens.
pub(crate) mod full {
    use super::*;
    use crate::snapshot::Snapshot;

    /// The committed full-byte fixtures (`m3h8_snapshot_contract.json`).
    pub(crate) const FIXTURES: &[(&str, &[u8])] = &[
        (
            "m3h8_state_0_20.bin",
            include_bytes!("../../../tests/reference/m3h8_state_0_20.bin"),
        ),
        (
            "m3h8_state_0_295.bin",
            include_bytes!("../../../tests/reference/m3h8_state_0_295.bin"),
        ),
        (
            "m3h8_state_0_82.bin",
            include_bytes!("../../../tests/reference/m3h8_state_0_82.bin"),
        ),
        (
            "m3h8_state_15_20.bin",
            include_bytes!("../../../tests/reference/m3h8_state_15_20.bin"),
        ),
        (
            "m3h8_state_16_31.bin",
            include_bytes!("../../../tests/reference/m3h8_state_16_31.bin"),
        ),
        (
            "m3h8_state_1_320.bin",
            include_bytes!("../../../tests/reference/m3h8_state_1_320.bin"),
        ),
        (
            "m3h8_state_22_23.bin",
            include_bytes!("../../../tests/reference/m3h8_state_22_23.bin"),
        ),
        (
            "m3h8_state_2_267.bin",
            include_bytes!("../../../tests/reference/m3h8_state_2_267.bin"),
        ),
        (
            "m3h8_state_3_10.bin",
            include_bytes!("../../../tests/reference/m3h8_state_3_10.bin"),
        ),
        (
            "m3h8_state_4_215.bin",
            include_bytes!("../../../tests/reference/m3h8_state_4_215.bin"),
        ),
        (
            "m3h8_state_4_216.bin",
            include_bytes!("../../../tests/reference/m3h8_state_4_216.bin"),
        ),
        (
            "m3h8_state_8_300.bin",
            include_bytes!("../../../tests/reference/m3h8_state_8_300.bin"),
        ),
        (
            "m3h8_state_9_3.bin",
            include_bytes!("../../../tests/reference/m3h8_state_9_3.bin"),
        ),
    ];
    type Frames = BTreeMap<(usize, usize), [Vec<Vec<u8>>; 2]>;
    /// The PBE1 frames of `m3h8_events.dref`, keyed by (case, op).
    fn frames() -> &'static Frames {
        static DATA: OnceLock<Frames> = OnceLock::new();
        DATA.get_or_init(|| {
            let bytes = packed::unpack(include_bytes!("../../../tests/reference/m3h8_events.dref"))
                .unwrap();
            let mut at = 0;
            let mut take = |n: usize| {
                let v = bytes[at..at + n].to_vec();
                at += n;
                v
            };
            assert_eq!(take(4), b"PBE1");
            let word = |v: Vec<u8>| u32::from_le_bytes(v.try_into().unwrap()) as usize;
            let mut frames = Frames::new();
            while at < bytes.len() {
                let mut take = |n: usize| {
                    let v = bytes[at..at + n].to_vec();
                    at += n;
                    v
                };
                let key = (word(take(4)), word(take(4)));
                let counts = [word(take(4)), word(take(4))];
                let mut frame = [Vec::new(), Vec::new()];
                for (family, count) in frame.iter_mut().zip(counts) {
                    for _ in 0..count {
                        let n = word(take(4));
                        family.push(take(n));
                    }
                }
                assert!(frames.insert(key, frame).is_none());
            }
            frames
        })
    }
    /// `m3h8_snapshots.csv`: (case, op) to (digest, fixture, bytes, fields).
    /// A capture's independent digest, fixture name, size and field count.
    type Capture = (u64, Option<&'static str>, usize, usize);
    fn captures() -> BTreeMap<(usize, usize), Capture> {
        include_str!("../../../tests/reference/m3h8_snapshots.csv")
            .lines()
            .skip(1)
            .map(|line| {
                let v: Vec<&'static str> = line.split(',').collect();
                (
                    (v[0].parse().unwrap(), v[1].parse().unwrap()),
                    (
                        u64::from_str_radix(v[3], 16).unwrap(),
                        (v[4] != "-").then_some(v[4]),
                        v[5].parse().unwrap(),
                        v[6].parse().unwrap(),
                    ),
                )
            })
            .collect()
    }

    /// Every count the full replay compared.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub(crate) struct Totals {
        pub cases: usize,
        pub ops: usize,
        pub event_frames: usize,
        pub event_records: usize,
        pub captures: usize,
        pub fixtures: usize,
        pub thinker_rows: usize,
        pub button_rows: usize,
        pub partial: Compared,
        pub digest: u64,
    }

    /// The ordered events of one op equal the independent PBE1 frame.
    fn assert_events(snap: &Snapshot, which: usize, op: usize) -> usize {
        let expected = &frames()[&(which, op)];
        let mut fields = Vec::new();
        for (prefix, events) in ["events.last", "events.pending"].into_iter().zip(expected) {
            fields.push((
                format!("{prefix}.count"),
                (events.len() as u64).to_le_bytes().to_vec(),
            ));
            fields.extend(
                events
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (format!("{prefix}.{i}"), e.clone())),
            );
        }
        let actual: Vec<_> = snap
            .fields()
            .filter(|f| f.path().starts_with("events."))
            .map(|f| (f.path().to_owned(), f.value().to_vec()))
            .collect();
        assert_eq!(actual, fields, "events case{which} op{op}");
        expected[0].len() + expected[1].len()
    }

    /// The thinker list (`TK`: kind, actor type or record fields) and the
    /// `buttonlist` (`MB`) after one op. Returns the compared row counts.
    fn assert_thinkers_and_buttons(
        s: &Simulation,
        which: usize,
        op: usize,
        rows: &[(String, Vec<i64>)],
    ) -> (usize, usize) {
        use crate::SectorThinker as T;
        let expected: Vec<_> = rows.iter().filter(|(t, _)| t == "TK").collect();
        let order: Vec<_> = s.actors.shared_order().collect();
        assert_eq!(order.len(), expected.len(), "TK count case{which} op{op}");
        for (index, (entry, (_, c))) in order.iter().zip(&expected).enumerate() {
            assert_eq!(c[0], index as i64);
            let got: Vec<i64> = match *entry {
                ThinkerId::Actor(id) => {
                    let a = s.actor(id).unwrap();
                    let pending = s.actor_availability(id) == ActorAvailability::PendingRemoval;
                    vec![
                        if pending { 9 } else { 0 },
                        i64::from(a.life().unwrap().actor_type.0),
                    ]
                }
                ThinkerId::Sector(id) => {
                    let slot = &s.actors.sector_slots[id.slot() as usize];
                    let entry = slot.entry.as_ref().unwrap();
                    assert_eq!(slot.generation, id.generation());
                    if entry.pending {
                        vec![8]
                    } else {
                        match entry.value {
                            T::Flash(t) => vec![
                                1,
                                t.sector.into(),
                                t.count.into(),
                                t.minlight.into(),
                                t.maxlight.into(),
                                t.maxtime.into(),
                                t.mintime.into(),
                            ],
                            T::Strobe(t) => vec![
                                2,
                                t.sector.into(),
                                t.count.into(),
                                t.minlight.into(),
                                t.maxlight.into(),
                                t.darktime.into(),
                                t.brighttime.into(),
                            ],
                            T::Glow(t) => vec![
                                3,
                                t.sector.into(),
                                -1,
                                t.minlight.into(),
                                t.maxlight.into(),
                                t.direction.into(),
                                0,
                            ],
                            T::Flicker(t) => vec![
                                4,
                                t.sector.into(),
                                t.count.into(),
                                t.minlight.into(),
                                t.maxlight.into(),
                                0,
                                0,
                            ],
                            // id, sector, type, direction, topheight, topcountdown,
                            // speed, topwait.
                            T::Door(d) => vec![
                                5,
                                d.sector.into(),
                                d.kind as i64,
                                d.direction.into(),
                                d.topheight.bits().into(),
                                d.topcountdown.into(),
                                match d.kind {
                                    movers::DoorKind::Normal => 2 << 16,
                                    movers::DoorKind::BlazeOpen => 8 << 16,
                                },
                                150,
                            ],
                            // id, sector, type, status, count, low, high, speed, wait,
                            // crush, tag, oldstatus.
                            T::Plat(p) => vec![
                                6,
                                p.sector.into(),
                                1,
                                p.status as i64,
                                p.count.into(),
                                p.low.bits().into(),
                                p.high.bits().into(),
                                4 << 16,
                                105,
                                0,
                                i64::from(s.world.map().sectors()[usize::from(p.sector)].tag),
                                0,
                            ],
                            // id, sector, type, direction, floordestheight, speed, crush,
                            // newspecial, texture.
                            T::Floor(f) => vec![
                                7,
                                f.sector.into(),
                                f.kind as i64,
                                -1,
                                f.floordestheight.bits().into(),
                                match f.kind {
                                    movers::FloorKind::LowerFloorToLowest => 1 << 16,
                                    movers::FloorKind::TurboLower => 4 << 16,
                                },
                                0,
                                0,
                                0,
                            ],
                        }
                    }
                }
            };
            // C: index, kind, id, then the kind's fields; the id (ident or
            // store index) is the C allocation's and is not compared.
            let mut c_fields = vec![c[1]];
            c_fields.extend_from_slice(&c[3..]);
            if matches!(c[1], 0 | 9) {
                assert_eq!(c.len(), 4, "TK actor row");
            }
            assert_eq!(got, c_fields, "TK case{which} op{op} entry{index}");
        }
        let mb: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "MB")
            .map(|(_, v)| [v[0], v[1], v[4], v[5]])
            .collect();
        let buttons: Vec<_> = s
            .buttons
            .iter()
            .enumerate()
            .filter_map(|(k, b)| {
                b.map(|b| {
                    [
                        k as i64,
                        i64::from(b.line),
                        i64::from(b.timer),
                        i64::from(b.sector),
                    ]
                })
            })
            .collect();
        assert_eq!(buttons, mb, "MB case{which} op{op}");
        (expected.len(), mb.len())
    }

    /// One committed capture: digest, size and field count from the independent
    /// index, full bytes for a fixture, and the explicit v35 reader's round trip.
    fn assert_capture(
        snap: &Snapshot,
        expected: (u64, Option<&str>, usize, usize),
        which: usize,
        op: usize,
    ) -> bool {
        let (digest, fixture, size, count) = expected;
        let limits = DiagnosticLimits::default();
        let bytes = snap.diagnostic_bytes(limits).unwrap();
        let full = fixture.map(|name| {
            let full = FIXTURES.iter().find(|(n, _)| *n == name).unwrap().1;
            let view = Diagnostic::decode_movers(full, limits).unwrap();
            for f in snap.fields() {
                assert_eq!(
                    Some(f.value()),
                    view.field(f.path()),
                    "fixture case{which} op{op} {}",
                    f.path()
                );
            }
            assert_eq!(bytes, full, "full bytes case{which} op{op}");
        });
        assert_eq!(snap.digest(), digest, "digest case{which} op{op}");
        assert_eq!(bytes.len(), size, "bytes case{which} op{op}");
        assert_eq!(snap.fields().count(), count, "fields case{which} op{op}");
        let view = Diagnostic::decode_movers(&bytes, limits)
            .unwrap_or_else(|e| panic!("decode case{which} op{op}: {e:?}"));
        assert_eq!(view.encode(limits).unwrap(), bytes);
        assert_eq!(
            Diagnostic::decode_specials(&bytes, limits).err(),
            Some(DiagnosticError::Version(35))
        );
        full.is_some()
    }

    /// Replays every scenario: the partial per-op comparisons, the thinker list
    /// and buttons, the event stream, every capture and every fixture.
    pub(crate) fn replay_all() -> Totals {
        let captures = captures();
        let mut t = Totals::default();
        for c in cases() {
            let mut failure = None;
            let (s, compared) = replay_observed(&c, c.tics, |s, op| {
                let snap = s.snapshot();
                let rows = op_rows(c.which, op);
                let (tk, mb) = assert_thinkers_and_buttons(s, c.which, op, rows);
                t.thinker_rows += tk;
                t.button_rows += mb;
                t.event_records += assert_events(&snap, c.which, op);
                t.event_frames += 1;
                if let Some(expected) = captures.get(&(c.which, op)) {
                    t.captures += 1;
                    t.fixtures += usize::from(assert_capture(&snap, *expected, c.which, op));
                } else if failure.is_none() && op == c.tics {
                    failure = Some(op);
                }
            });
            assert_eq!(failure, None, "{}: the last op is a capture", c.label);
            t.cases += 1;
            t.ops += compared.ops;
            t.partial.crossings += compared.crossings;
            t.partial.uses += compared.uses;
            t.partial.blocked += compared.blocked;
            t.digest ^= s.snapshot().digest().rotate_left(c.which as u32);
        }
        t
    }

    #[test]
    fn full_replay_matches_the_c_rows_and_the_independent_dsim35_goldens() {
        let t = replay_all();
        println!(
            "player_movers full replay: cases={} ops={} event_frames={} event_records={} captures={} fixtures={} thinker_rows={} button_rows={} crossings={} uses={} blocked={} digest={:016x}",
            t.cases,
            t.ops,
            t.event_frames,
            t.event_records,
            t.captures,
            t.fixtures,
            t.thinker_rows,
            t.button_rows,
            t.partial.crossings,
            t.partial.uses,
            t.partial.blocked,
            t.digest
        );
        // Every committed case, op, frame, capture, fixture and row was reached.
        let rows: usize = rows()
            .values()
            .flatten()
            .filter(|(tag, _)| tag == "TK")
            .count();
        let mb: usize = rows_named("MB");
        assert_eq!(t.cases, 33);
        assert_eq!(t.ops, frames().len());
        assert_eq!(t.event_frames, frames().len());
        assert_eq!(t.captures, captures().len());
        assert_eq!(t.fixtures, FIXTURES.len());
        assert_eq!(t.thinker_rows, rows);
        assert_eq!(t.button_rows, mb);
    }
    fn rows_named(tag: &str) -> usize {
        rows().values().flatten().filter(|(t, _)| t == tag).count()
    }
}
