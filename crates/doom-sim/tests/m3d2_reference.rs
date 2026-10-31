//! Original player/psprite/weapon/combat sequences, independent of Rust output.
#[path = "../../../tests/support/m3d2.rs"]
mod fixtures;
use doom_sim::*;
use std::collections::BTreeMap;
fn numbers(row: &str) -> Vec<i64> {
    row.split(',').skip(1).map(|x| x.parse().unwrap()).collect()
}
#[test]
fn complete_player_weapon_sequences_match_original_c() {
    let source = doom_test_evidence::text(
        "tests/reference/m3d2_trace.csv",
        include_bytes!("../../../tests/reference/m3d2_trace.csv.zst"),
        include_str!("../../../tests/reference/zstd_manifest.json"),
    )
    .expect("verified historical zstd evidence");
    let mut fields = 0;
    for (which, c) in fixtures::cases().iter().enumerate() {
        let mut sim = fixtures::setup(c);
        let commands = fixtures::commands(which);
        let mut identities = BTreeMap::new();
        let mut next = 0;
        let mut rows = BTreeMap::<usize, Vec<&str>>::new();
        for row in source
            .lines()
            .filter(|r| r.split(',').nth(1).unwrap().parse::<usize>().unwrap() == which)
        {
            rows.entry(row.split(',').nth(2).unwrap().parse().unwrap())
                .or_default()
                .push(row);
        }
        for tic in 0..=commands.len() {
            let pending = sim.pending_events().len();
            if tic > 0 {
                sim.step(commands[tic - 1])
                    .unwrap_or_else(|e| panic!("{} tic {tic}: {e}", c.label));
            }
            let events = if tic == 0 {
                sim.pending_events()
            } else {
                &sim.events()[pending..]
            };
            for e in events {
                if let Event::ActorState(ActorStateEvent::Initialized { actor, .. }) = e {
                    identities.insert(next, *actor);
                    next += 1;
                }
            }
            let current = &rows[&tic];
            let p = sim.weapon_player().unwrap();
            let row = current
                .iter()
                .find(|r| r.starts_with("P,"))
                .map(|r| numbers(r))
                .unwrap();
            let mut actual = vec![
                i64::from(p.slot),
                0,
                i64::from(p.health),
                p.ready as i64,
                p.pending.map_or(10, |w| w as i64),
                i64::from(p.owned),
            ];
            actual.extend(p.ammo.map(i64::from));
            actual.extend(p.max_ammo.map(i64::from));
            actual.extend([
                i64::from(p.attack_down),
                i64::from(p.refire),
                i64::from(p.bob.bits()),
                i64::from(p.extra_light),
            ]);
            for ps in p.psprites {
                actual.extend([
                    ps.state.map_or(-1, |s| i64::from(s.0)),
                    i64::from(ps.tics),
                    i64::from(ps.position[0].bits()),
                    i64::from(ps.position[1].bits()),
                ]);
            }
            actual.push(i64::from(sim.rng().indices()[0]));
            assert_eq!(actual, row[2..], "{} tic {tic} player", c.label);
            fields += actual.len();
            for row in current.iter().filter(|r| r.starts_with("A,")) {
                let a = numbers(row);
                let id = identities[&a[2]];
                let actual = sim.actor(id);
                assert_eq!(
                    actual.is_some(),
                    a[3] != 0,
                    "{} tic {tic} actor {} alive",
                    c.label,
                    a[2]
                );
                if let Some(v) = actual {
                    let l = v.life().unwrap();
                    let pos = v.position();
                    let m = v.momentum();
                    let z = v.constraints();
                    let target = l.target.map_or(-1, |id| {
                        *identities.iter().find(|(_, v)| **v == id).unwrap().0
                    });
                    let got = vec![
                        i64::from(l.actor_type.0),
                        i64::from(pos[0].bits()),
                        i64::from(pos[1].bits()),
                        i64::from(v.z().bits()),
                        i64::from(m[0].bits()),
                        i64::from(m[1].bits()),
                        i64::from(v.vertical_momentum().bits()),
                        i64::from(l.health),
                        l.state.map_or(-1, |s| i64::from(s.0)),
                        i64::from(l.tics),
                        i64::from(l.flags),
                        i64::from(v.height().bits()),
                        i64::from(v.radius().bits()),
                        i64::from(l.reaction_time),
                        i64::from(l.threshold),
                        target,
                        i64::from(v.lastlook()),
                        i64::from(z[0].bits()),
                        i64::from(z[1].bits()),
                        i64::from(v.location().unwrap().subsector),
                        i64::from(sim.rng().indices()[0]),
                    ];
                    assert_eq!(got, a[4..], "{} tic {tic} actor {}", c.label, a[2]);
                    fields += got.len();
                }
            }
            for row in current.iter().filter(|r| r.starts_with("G,")) {
                let a = numbers(row);
                if let Some(v) = sim.actor(identities[&a[2]]) {
                    assert_eq!(i64::from(v.angle()), a[3], "{} tic {tic} angle", c.label);
                }
            }
            for row in current.iter().filter(|r| r.starts_with("S,")) {
                let a = numbers(row);
                let id = sim.sound_targets()[a[2] as usize].map_or(-1, |id| {
                    *identities.iter().find(|(_, v)| **v == id).unwrap().0
                });
                assert_eq!(id, a[3], "{} tic {tic} noise", c.label);
            }
            let expected = current
                .iter()
                .filter(|r| r.starts_with("W,"))
                .map(|r| numbers(r)[2..].to_vec())
                .collect::<Vec<_>>();
            let actual = events
                .iter()
                .filter_map(|e| weapon_event(e, &identities))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{} tic {tic} weapon event order", c.label);
            let expected = current
                .iter()
                .filter(|r| r.starts_with("E,"))
                .map(|r| numbers(r))
                .filter(|r| r[2] != 8)
                .map(|r| r[2..].to_vec())
                .collect::<Vec<_>>();
            let actual = events
                .iter()
                .filter_map(|e| life_event(e, &identities))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{} tic {tic} lifecycle order", c.label);
            let combined = current
                .iter()
                .filter_map(|r| {
                    let n = numbers(r);
                    if r.starts_with("W,") {
                        Some((b'W', n[2..].to_vec()))
                    } else if r.starts_with("E,") && n[2] != 8 {
                        Some((b'E', n[2..].to_vec()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let actual = events
                .iter()
                .filter_map(|e| {
                    weapon_event(e, &identities)
                        .map(|v| (b'W', v))
                        .or_else(|| life_event(e, &identities).map(|v| (b'E', v)))
                })
                .collect::<Vec<_>>();
            assert_eq!(
                actual, combined,
                "{} tic {tic} combined source event order",
                c.label
            );
        }
    }
    println!("Compared {fields} C player/actor fields");
    assert_eq!(fields, 422_943);
}
fn ordinal(id: ActorId, m: &BTreeMap<i64, ActorId>) -> i64 {
    *m.iter().find(|(_, v)| **v == id).unwrap().0
}
fn weapon_event(e: &Event, m: &BTreeMap<i64, ActorId>) -> Option<Vec<i64>> {
    let Event::Weapon(w) = e else { return None };
    Some(match w {
        WeaponEvent::Entered {
            slot,
            position,
            state,
        } => vec![
            0,
            i64::from(*slot),
            i64::from(*position),
            i64::from(state.0),
        ],
        WeaponEvent::Action {
            slot,
            position,
            action,
        } => vec![
            1,
            i64::from(*slot),
            i64::from(*position),
            i64::from(action.0),
        ],
        WeaponEvent::Ammo {
            slot,
            ammo,
            before,
            after,
        } => vec![
            2,
            i64::from(*slot),
            i64::from(*ammo),
            i64::from(*before),
            i64::from(*after),
        ],
        WeaponEvent::Light { slot, value } => vec![3, i64::from(*slot), i64::from(*value)],
        WeaponEvent::Fire {
            slot,
            weapon,
            refire,
        } => vec![4, i64::from(*slot), *weapon as i64, i64::from(*refire)],
        WeaponEvent::Bullet {
            actor,
            angle,
            slope,
            damage,
        } => vec![
            5,
            ordinal(*actor, m),
            i64::from(*angle),
            i64::from(slope.bits()),
            i64::from(*damage),
        ],
        WeaponEvent::Noise {
            sector,
            blocks,
            target,
        } => vec![
            6,
            i64::from(*sector),
            i64::from(*blocks),
            ordinal(*target, m),
        ],
        WeaponEvent::Initialized { slot, actor } => vec![7, i64::from(*slot), ordinal(*actor, m)],
        WeaponEvent::Random { kind, value } => vec![8, i64::from(*kind), i64::from(*value)],
    })
}
fn life_event(e: &Event, m: &BTreeMap<i64, ActorId>) -> Option<Vec<i64>> {
    Some(match e {
        Event::Think(id) => vec![0, ordinal(*id, m), 0],
        Event::RemovalMarked(id) => vec![2, ordinal(*id, m), 0],
        Event::Retired(id) => vec![3, ordinal(*id, m), 0],
        Event::ActorState(e) => match e {
            ActorStateEvent::Initialized { actor, actor_type } => {
                vec![1, ordinal(*actor, m), i64::from(actor_type.0)]
            }
            ActorStateEvent::Entered { actor, state } => {
                vec![4, ordinal(*actor, m), i64::from(state.0)]
            }
            ActorStateEvent::Action {
                actor,
                action: ActorAction::Original(a),
            } => vec![5, ordinal(*actor, m), i64::from(a.0)],
            ActorStateEvent::Sound { actor, sound } => {
                vec![7, actor.map_or(-1, |id| ordinal(id, m)), i64::from(*sound)]
            }
            _ => return None,
        },
        _ => return None,
    })
}
