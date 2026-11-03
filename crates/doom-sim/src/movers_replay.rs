//! A partial replay of the committed M3h8 C scenarios (plan steps 9–11): each case's
//! setup and per-op commands (`UC` rows) drive the Rust movers policy, and the
//! per-op sector heights and `specialdata` presence (`MH`), runtime line specials
//! (`ML`), use latch, player state and exit (`MZ`) and the crossings (`MX`/`MY`) are
//! compared with the C rows. Step 13 builds the full replay with every row, DSIM35
//! bytes and the independent captures. Expected values come only from the C rows.
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
    pub map: usize,
    pub clips: i32,
    pub ready: Weapon,
    pub owned: u16,
    pub reaction: u8,
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
    // The committed room with the C harness's declared switch textures.
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
    (s, body, things)
}
/// The declared between-tic inputs: the damage (thing `hit_thing`, the player when
/// the room has no THING) and the attack entry.
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
