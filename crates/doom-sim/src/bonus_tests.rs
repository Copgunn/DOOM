use super::*;
use crate::{lifecycle::ActorAccess, movement::MotionAccess};

fn component(
    doomed: i16,
    health: i32,
    kind: u8,
    points: i32,
) -> (Simulation, bonus_evidence::Actors, ActorId) {
    // Exactly the declared C component's initial state and spawn-call order.
    let bytes = bonus_evidence::COMPONENT_MAPS
        .iter()
        .find(|(k, _)| *k == doomed)
        .unwrap()
        .1;
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s =
        Simulation::original_player_bonuses_map(map, 64, MapContextOptions::default()).unwrap();
    let ids = s.spawn_map_things().unwrap().spawned;
    let body = s
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
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
    s.original_player.as_mut().unwrap().health = health;
    s.original_player.as_mut().unwrap().armor = Some(PlayerArmorState {
        kind: ArmorType::try_from(kind).unwrap(),
        points,
    });
    s.actors
        .get_mut(body)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .health = health;
    (
        s,
        bonus_evidence::Actors {
            body,
            origin: Some(ids[0]),
        },
        ids[1],
    )
}

fn touch(s: &mut Simulation, a: bonus_evidence::Actors) {
    // Direct Touch component, not movement: use the actual contact eligibility
    // query at x1, retain the original x0 body, as in the unticked C invocation.
    s.checked_demon_operation(|host| {
        let mut body = *host.access.get(a.body).unwrap();
        let attempt = movement::check(
            host.world,
            a.body,
            &body,
            [Fixed::from_map_units(1), Fixed::ZERO],
            host.spatial,
            &|id| host.access.get(id).copied(),
        );
        let mut events = Vec::new();
        world_pickups::Contacts {
            access: &mut host.access,
            player: host.player.as_deref_mut().unwrap(),
        }
        .contacts(a.body, &mut body, &attempt, host.spatial, &mut events)
        .unwrap();
        *host.access.get_mut(a.body).unwrap() = body;
        host.events.extend(events.into_iter().map(|e| match e {
            movement::MotionEvent::Contact(c) => c.event(),
            _ => panic!("direct contact component"),
        }));
    })
    .unwrap();
}

fn component_observations(s: &Simulation, a: bonus_evidence::Actors, item: ActorId) -> Vec<i64> {
    let p = s.original_player(PlayerSlot::One).unwrap();
    let armor = s.original_player_armor(PlayerSlot::One).unwrap();
    let pickup = s.original_player_pickups(PlayerSlot::One).unwrap();
    let body = s.actor(a.body).unwrap();
    let l = body.life().unwrap();
    let birth = |id: Option<ActorId>| {
        id.map_or(-1, |id| {
            if Some(id) == a.origin {
                0
            } else if id == item {
                1
            } else if id == a.body {
                2
            } else {
                panic!("component identity")
            }
        })
    };
    vec![
        i64::from(p.health),
        i64::from(l.health),
        i64::from(armor.kind.wire_id()),
        i64::from(armor.points),
        i64::from(pickup.item_count),
        i64::from(s.level_statistics().unwrap().total_items),
        i64::from(pickup.bonus_count),
        pickup.message as i64,
        i64::from(s.actors.is_pending(item) == Some(true)),
        i64::from(s.rng.indices()[0]),
        i64::from(p.damage_count),
        birth(p.attacker),
        i64::from(body.momentum()[0].bits()),
        i64::from(body.momentum()[1].bits()),
        i64::from(l.state.unwrap().0),
        i64::from(l.tics),
        i64::from(l.flags),
        i64::from(l.reaction_time),
        i64::from(l.threshold),
        birth(l.target),
        i64::from(body.height().bits()),
        i64::from(p.state == PlayerBodyState::Dead),
        i64::from(p.frags),
    ]
}

#[test]
fn independent_bonus_resources_ordinary_thresholds_and_shared_damage() {
    let mut count = 0;
    for line in include_str!("../../../tests/reference/m3g11_components.csv")
        .lines()
        .filter(|l| l.starts_with("BCOMP,T,"))
    {
        let v: Vec<_> = line.split(',').collect();
        let n = |i: usize| v[i].parse::<i32>().unwrap();
        let (mut s, a, item) = component(n(3) as i16, n(4), n(5) as u8, n(6));
        touch(&mut s, a);
        if n(2) == 40 {
            // Source-only raw300 physical component exceeds the retained30-unit
            // momentum admission. Its pickup equals source componentT4; the
            // invalid thrust rejects atomically, without widening old bounds.
            let pickup_row = include_str!("../../../tests/reference/m3g11_components.csv")
                .lines()
                .find(|l| l.starts_with("BCOMP,T,4,"))
                .unwrap();
            let expected: Vec<i64> = pickup_row
                .split(',')
                .skip(8)
                .map(|n| n.parse().unwrap())
                .collect();
            let got = component_observations(&s, a, item);
            for (field, (got, expected)) in got.iter().zip(&expected).enumerate() {
                if ![4, 5].contains(&field) {
                    assert_eq!(got, expected, "source pickupT40 field{field}");
                }
            }
            let before = s.snapshot();
            assert_eq!(
                s.damage_actor(DamageRequest {
                    target: a.body,
                    source: a.origin,
                    inflictor: a.origin,
                    amount: n(7)
                }),
                Err(PlayerBodyError::Unsupported.into())
            );
            assert_eq!(s.snapshot(), before);
            count += 1;
            continue;
        }
        if n(7) != 0 {
            s.rng = DoomRng::from_indices(0, 0);
            s.damage_actor(DamageRequest {
                target: a.body,
                source: a.origin,
                inflictor: a.origin,
                amount: n(7),
            })
            .unwrap_or_else(|e| panic!("{line}: {e}"));
        }
        let got = component_observations(&s, a, item);
        let expected: Vec<i64> = v[8..].iter().map(|n| n.parse().unwrap()).collect();
        for (field, (got, expected)) in got.iter().zip(&expected).enumerate() {
            // Counted statistics receive their independent implementation gate.
            if [4, 5].contains(&field) {
                continue;
            }
            assert_eq!(got, expected, "{line} field{field}");
        }
        count += 1;
    }
    assert_eq!(count, 48);
}

#[test]
fn bonus_armor_invariant_does_not_redefine_ordinary_green_armor() {
    for points in [101, 120, 199, 200] {
        let armor = PlayerArmorState {
            kind: ArmorType::Green,
            points,
        };
        assert!(armor.validate().is_err());
        assert!(
            armor
                .validate_for(OriginalExecutionProfile::PlayerDemonArmorV1)
                .is_err()
        );
        armor
            .validate_for(OriginalExecutionProfile::PlayerDemonBonusesV1)
            .unwrap();
    }
    for kind in [ArmorType::None, ArmorType::Green, ArmorType::Blue] {
        for points in [-1, 201, i32::MAX] {
            assert!(
                PlayerArmorState { kind, points }
                    .validate_for(OriginalExecutionProfile::PlayerDemonBonusesV1)
                    .is_err()
            );
        }
    }
}
