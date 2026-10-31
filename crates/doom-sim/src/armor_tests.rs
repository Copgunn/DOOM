use super::*;
#[test]
fn independent_original_armor_actors() {
    assert_ne!(armor_evidence::verify_range(0..8), 0);
}

#[test]
fn independent_original_armor_pickups() {
    assert_ne!(armor_evidence::verify_range(8..15), 0);
    assert_ne!(armor_evidence::verify_range(17..18), 0);
    assert_ne!(armor_evidence::verify_range(21..22), 0);
    assert_ne!(armor_evidence::verify_range(32..33), 0);
}
#[test]
fn independent_original_give_armor_components() {
    for line in include_str!("../../../tests/reference/m3g10_components.csv").lines() {
        let v: Vec<_> = line.split(',').collect();
        if v.get(1) != Some(&"G") {
            continue;
        }
        let n = |i: usize| v[i].parse::<i32>().unwrap();
        let mut armor = PlayerArmorState {
            kind: ArmorType::try_from(n(4) as u8).unwrap(),
            points: n(5),
        };
        assert_eq!(
            armor
                .grant(ArmorType::try_from(n(3) as u8).unwrap())
                .unwrap(),
            n(6) != 0,
            "{line}"
        );
        assert_eq!(
            armor,
            PlayerArmorState {
                kind: ArmorType::try_from(n(7) as u8).unwrap(),
                points: n(8)
            },
            "{line}"
        );
    }
}

#[test]
fn independent_original_armor_sustained_combat() {
    assert_ne!(
        armor_evidence::verify_range(0..armor_evidence::cases().len()),
        0
    );
}

fn component(kind: u8, points: i32, health: i32, seed: u8) -> (Simulation, armor_evidence::Actors) {
    // Explicit component input, as independently specified in the C component
    // table; integrated encounter tests obtain every armor point from the world.
    let (mut s, a) = armor_evidence::setup(&armor_evidence::cases()[33]);
    let p = s.original_player.as_mut().unwrap();
    p.armor = Some(PlayerArmorState {
        kind: ArmorType::try_from(kind).unwrap(),
        points,
    });
    p.health = health;
    s.actors
        .get_mut(a.body)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .health = health;
    s.rng = DoomRng::from_indices(seed, 0);
    (s, a)
}
fn damage(s: &mut Simulation, a: armor_evidence::Actors, raw: i32, physical: bool) -> DamageResult {
    s.damage_actor(DamageRequest {
        target: a.body,
        source: a.origin,
        inflictor: if physical { a.origin } else { None },
        amount: raw,
    })
    .unwrap()
}
#[test]
fn independent_original_armor_damage_components() {
    let mut cases = 0;
    for line in include_str!("../../../tests/reference/m3g10_components.csv").lines() {
        let v: Vec<_> = line.split(',').collect();
        if v.get(1) != Some(&"D") {
            continue;
        }
        let n = |i: usize| v[i].parse::<i64>().unwrap();
        let (mut s, a) = component(n(3) as u8, n(4) as i32, n(5) as i32, n(8) as u8);
        damage(&mut s, a, n(6) as i32, n(7) != 0);
        let p = s.original_player(PlayerSlot::One).unwrap();
        let armor = s.original_player_armor(PlayerSlot::One).unwrap();
        let actor = s.actor(a.body).unwrap();
        let l = actor.life().unwrap();
        let birth = |id: Option<ActorId>| {
            id.map_or(-1, |id| {
                if Some(id) == a.origin {
                    0
                } else if id == a.body {
                    1
                } else {
                    panic!("component birth")
                }
            })
        };
        let got = [
            i64::from(armor.kind.wire_id()),
            i64::from(armor.points),
            i64::from(p.health),
            i64::from(l.health),
            i64::from(p.damage_count),
            birth(p.attacker),
            i64::from(actor.momentum()[0].bits()),
            i64::from(actor.momentum()[1].bits()),
            i64::from(l.state.unwrap().0),
            i64::from(l.tics),
            i64::from(l.flags),
            i64::from(l.reaction_time),
            i64::from(l.threshold),
            birth(l.target),
            i64::from(s.rng.indices()[0]),
            i64::from(actor.height().bits()),
            i64::from(p.state == PlayerBodyState::Dead),
            i64::from(p.frags),
        ];
        let expected: Vec<_> = v[9..].iter().map(|n| n.parse::<i64>().unwrap()).collect();
        assert_eq!(got.as_slice(), expected, "{line}");
        cases += 1;
    }
    assert_eq!(cases, 40);
}
#[test]
fn independent_original_armor_repeated_and_ignored_damage() {
    let lines: Vec<Vec<i64>> = include_str!("../../../tests/reference/m3g10_components.csv")
        .lines()
        .filter(|l| l.starts_with("ACOMP,R,"))
        .map(|l| l.split(',').skip(2).map(|v| v.parse().unwrap()).collect())
        .collect();
    for kind in [1, 2] {
        let (mut s, a) = component(kind, 5, 100, 0);
        for row in lines.iter().filter(|r| r[0] == i64::from(kind)) {
            let before = s.original_player_armor(PlayerSlot::One).unwrap();
            let hp = s.original_player(PlayerSlot::One).unwrap().health;
            // Source repeat components have neither a source nor an inflictor.
            s.damage_actor(DamageRequest {
                target: a.body,
                source: None,
                inflictor: None,
                amount: 10,
            })
            .unwrap();
            let after = s.original_player_armor(PlayerSlot::One).unwrap();
            let got = [
                i64::from(before.kind.wire_id()),
                i64::from(before.points),
                i64::from(after.kind.wire_id()),
                i64::from(after.points),
                i64::from(hp),
                i64::from(s.original_player(PlayerSlot::One).unwrap().health),
                i64::from(s.actor(a.body).unwrap().life().unwrap().health),
                i64::from(s.rng.indices()[0]),
            ];
            assert_eq!(got.as_slice(), &row[2..]);
        }
        let (mut s, a) = component(kind, i32::from(kind) * 100, 100, 0);
        damage(&mut s, a, if kind == 1 { 150 } else { 200 }, false);
        let before = s.original_player(PlayerSlot::One).unwrap();
        let armor = s.original_player_armor(PlayerSlot::One).unwrap();
        let rng = s.rng;
        let actor = *s.actor(a.body).unwrap();
        assert!(damage(&mut s, a, 10, true).ignored);
        assert_eq!(s.original_player(PlayerSlot::One).unwrap(), before);
        assert_eq!(s.original_player_armor(PlayerSlot::One).unwrap(), armor);
        assert_eq!(s.rng, rng);
        assert_eq!(*s.actor(a.body).unwrap(), actor);
    }
}
#[test]
fn armor_reduces_health_damage_without_reducing_raw_physical_thrust() {
    let mut observed = Vec::new();
    for (kind, points, health) in [(0, 0, 40), (1, 100, 60), (2, 200, 70)] {
        let (mut s, a) = component(kind, points, 100, 0);
        damage(&mut s, a, 60, true);
        assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, health);
        observed.push(s.actor(a.body).unwrap().momentum());
    }
    assert!(observed.windows(2).all(|v| v[0] == v[1]));
}

fn fresh(which: usize) -> (Simulation, armor_evidence::Actors) {
    armor_evidence::setup(&armor_evidence::cases()[which])
}
fn rejected(s: &mut Simulation, commands: [TicCommand; 4]) {
    let before = s.snapshot();
    let captures = snapshot::capture_count();
    let roster = s.original_player_roster();
    assert!(s.advance(commands).is_err());
    assert_eq!(snapshot::capture_count(), captures);
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.original_player_roster(), roster);
    assert!(s.step(commands).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn armor_policy_and_original_definition_admission_are_separate() {
    use identity::*;
    let r = OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonArmorV1).unwrap();
    let old =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonPickupsV1).unwrap();
    for (k, ty, first, second) in [
        (
            PickupKind::GreenArmor,
            OriginalActorType::MT_MISC0,
            OriginalState::S_ARM1,
            OriginalState::S_ARM1A,
        ),
        (
            PickupKind::BlueArmor,
            OriginalActorType::MT_MISC1,
            OriginalState::S_ARM2,
            OriginalState::S_ARM2A,
        ),
    ] {
        let definition = DefinitionIdentity::Original(ty);
        assert_eq!(k.original_type(), ty);
        assert_eq!(
            r.admit_definition(definition).unwrap(),
            ActorTypeId(ty.ordinal())
        );
        for state in [first, second] {
            assert_eq!(
                r.admit_state(definition, state).unwrap(),
                StateId(state.ordinal())
            );
        }
        assert!(old.admit_definition(definition).is_err());
        assert!(
            r.admit_state(definition, OriginalState::S_SARG_RUN1)
                .is_err()
        );
    }
    for ty in [
        OriginalActorType::MT_MISC2,
        OriginalActorType::MT_MISC3,
        OriginalActorType::MT_SHADOWS,
    ] {
        assert!(
            r.admit_definition(DefinitionIdentity::Original(ty))
                .is_err()
        );
    }
}
#[test]
fn historical_pickup_policy_does_not_acquire_armor_capability() {
    let mut s = Simulation::original_player_pickups_map(
        armor_evidence::case_map(1),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    let before = s.snapshot();
    assert!(s.spawn_map_things().is_err());
    assert_eq!(s.snapshot(), before);
    assert!(s.original_player_armor(PlayerSlot::One).is_err());
}
#[test]
fn armor_map_batch_failure_preserves_rng_actor_order_and_statistics() {
    let c = &armor_evidence::cases()[4];
    let mut s = Simulation::original_player_armor_map(
        armor_evidence::case_map(c.map),
        8,
        MapContextOptions::default(),
    )
    .unwrap();
    let before = s.snapshot();
    assert!(s.spawn_map_things().is_err());
    assert_eq!(s.snapshot(), before);
    // Mutate a synthetic WAD's last THING, not immutable runtime map storage.
    let mut bytes = include_bytes!("../../../tests/reference/m3g10_geometry_4.bin").to_vec();
    let count = i32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let directory = i32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    for i in 0..count {
        let at = directory + 16 * i;
        if &bytes[at + 8..at + 16] == b"THINGS\0\0" {
            let offset = i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
            let length = i32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
            bytes[offset + length - 4..offset + length - 2].copy_from_slice(&2014i16.to_le_bytes());
        }
    }
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s =
        Simulation::original_player_armor_map(map, 64, MapContextOptions::default()).unwrap();
    let before = s.snapshot();
    assert!(s.spawn_map_things().is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn invalid_animated_armor_fields_reject_before_commands_or_random_draws() {
    for mode in 0..13 {
        let (mut s, a) = fresh(1);
        let id = s.actor_order().find(|id| *id != a.body).unwrap();
        let actor = s.actors.get_mut(id).unwrap();
        match mode {
            0 => actor.life.as_mut().unwrap().sprite = 0,
            1 => actor.life.as_mut().unwrap().frame = 1,
            2 => actor.life.as_mut().unwrap().tics = 0,
            3 => actor.life.as_mut().unwrap().tics = 7,
            4 => actor.life.as_mut().unwrap().state = Some(StateId(804)),
            5 => actor.life.as_mut().unwrap().flags |= 2,
            6 => actor.life.as_mut().unwrap().health = 999,
            7 => actor.life.as_mut().unwrap().target = Some(a.body),
            8 => actor.spec.momentum[0] = Fixed::ONE,
            9 => actor.z = Fixed::ONE,
            10 => actor.spec.radius = Fixed::ONE,
            11 => actor.life.as_mut().unwrap().spawnpoint = None,
            _ => actor.life.as_mut().unwrap().flags |= world_pickups::MF_DROPPED,
        }
        rejected(&mut s, [TicCommand::default(); 4]);
    }
}
#[test]
fn invalid_armor_owner_and_scalar_records_reject_atomically() {
    for mode in 0..10 {
        let (mut s, a) = fresh(8);
        let p = s.original_player.as_mut().unwrap();
        match mode {
            0 => p.armor = None,
            1 => p.armor.as_mut().unwrap().points = 1,
            2 => {
                p.armor = Some(PlayerArmorState {
                    kind: ArmorType::Green,
                    points: 101,
                })
            }
            3 => {
                p.armor = Some(PlayerArmorState {
                    kind: ArmorType::Blue,
                    points: 201,
                })
            }
            4 => {
                p.armor = Some(PlayerArmorState {
                    kind: ArmorType::Green,
                    points: 0,
                })
            }
            5 => {
                p.armor = Some(PlayerArmorState {
                    kind: ArmorType::Blue,
                    points: -1,
                })
            }
            6 => p.mo = None,
            7 => p.mo = Some(ActorId::from_parts(a.body.slot(), a.body.generation() + 1)),
            8 => p.health = 99,
            _ => {
                s.actors.mark_remove(a.body).unwrap();
            }
        }
        rejected(
            &mut s,
            armor_evidence::commands(&armor_evidence::cases()[8], 1),
        );
    }
    assert!(ArmorType::try_from(3).is_err());
    assert!(PlayerSlot::try_from(1).is_err());
}
#[test]
fn unsupported_armor_controls_and_dropped_definitions_reject() {
    for mode in 0..4 {
        let (mut s, _) = fresh(8);
        let mut c = [TicCommand::default(); 4];
        match mode {
            0 => c[0].buttons = 2,
            1 => c[1].forward = 1,
            2 => c[0].forward = 51,
            _ => c[0].side = 51,
        }
        rejected(&mut s, c);
    }
    let (mut s, _) = fresh(0);
    let before = s.snapshot();
    for kind in [PickupKind::GreenArmor, PickupKind::BlueArmor] {
        assert!(s.spawn_dropped_pickup(kind, [Fixed::ZERO; 2]).is_err());
        assert_eq!(s.snapshot(), before);
    }
}
#[test]
fn late_armor_collection_failure_discards_grant_removal_animation_and_commands() {
    for which in [8, 9, 13] {
        let (mut s, _) = fresh(which);
        s.original_player
            .as_mut()
            .unwrap()
            .pickups
            .as_mut()
            .unwrap()
            .bonus_count = 96;
        // Private, range-valid but causally artificial bonus state: source grant
        // and removal execute before the existing bounded bonus validation fails.
        rejected(
            &mut s,
            armor_evidence::commands(&armor_evidence::cases()[which], 1),
        );
        assert_eq!(
            s.original_player_armor(PlayerSlot::One).unwrap(),
            PlayerArmorState::default()
        );
    }
}
#[test]
fn late_damage_failure_discards_absorption_depletion_death_thrust_and_events() {
    for (kind, points, raw, physical) in [
        (1, 100, 60, true),
        (1, 20, 60, true),
        (2, 200, 200, false),
        (1, 5, 60, true),
    ] {
        let (mut s, a) = component(kind, points, 100, 0);
        let request = DamageRequest {
            target: a.body,
            inflictor: if physical { a.origin } else { None },
            source: a.origin,
            amount: raw,
        };
        let plan = s.plan_damage(request).unwrap();
        let before = s.snapshot();
        let armor = s.original_player_armor(PlayerSlot::One).unwrap();
        let result = s.checked_demon_operation(|host| {
            host.apply_damage(plan);
            assert_ne!(host.player.as_ref().unwrap().armor, Some(armor));
            host.fail_sight(PerceptionError::WorkLimit.into());
        });
        assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
        assert_eq!(s.snapshot(), before);
    }
}
#[test]
fn armor_cannot_make_an_invalid_raw_thrust_travel_envelope_safe() {
    let (mut s, a) = component(2, 200, 100, 0);
    let before = s.snapshot();
    assert!(
        s.damage_actor(DamageRequest {
            target: a.body,
            inflictor: a.origin,
            source: a.origin,
            amount: 1000
        })
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
}
#[test]
fn paused_armor_world_does_not_animate_collect_consume_rng_or_capture_implicitly() {
    let (mut s, _) = fresh(8);
    s.set_paused(true);
    let mut stepped = s.clone();
    let armor = s.original_player_armor(PlayerSlot::One).unwrap();
    let rng = s.rng;
    let captures = snapshot::capture_count();
    let c = armor_evidence::commands(&armor_evidence::cases()[8], 1);
    s.advance(c).unwrap();
    assert_eq!(snapshot::capture_count(), captures);
    assert_eq!(s.snapshot(), stepped.step(c).unwrap());
    assert_eq!(s.rng, rng);
    assert_eq!(s.original_player_armor(PlayerSlot::One).unwrap(), armor);
}
#[test]
fn armor_item_instance_references_do_not_cross_game_instances() {
    use identity::*;
    let (s, a) = fresh(1);
    let item = s.actor_order().find(|id| *id != a.body).unwrap();
    let mut left = GameInstance::new(s.clone());
    let mut right = GameInstance::new(s);
    let reference = left
        .with_execution_context(|ctx| {
            let item = ctx.admit_local(
                item,
                DefinitionIdentity::Original(OriginalActorType::MT_MISC0),
            )?;
            ctx.export_actor(item)
        })
        .unwrap();
    let before = right.simulation().snapshot();
    assert!(
        right
            .with_execution_context(|ctx| {
                ctx.import_actor(
                    &reference,
                    DefinitionIdentity::Original(OriginalActorType::MT_MISC0),
                )?;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(right.simulation().snapshot(), before);
}
fn replace_field(bytes: &mut [u8], name: &str, value: &[u8]) {
    let mut at = 16;
    while at < bytes.len() {
        let n = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        at += 4;
        let yes = &bytes[at..at + n] == name.as_bytes();
        at += n;
        let n = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
        at += 8;
        if yes {
            assert_eq!(n, value.len());
            bytes[at..at + n].copy_from_slice(value);
            return;
        }
        at += n;
    }
    panic!("missing test field {name}");
}
#[test]
fn armor_diagnostic_is_explicit_and_preserves_historical_readers() {
    let (s, _) = fresh(1);
    let bytes = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    assert_eq!(s.snapshot().version(), 26);
    assert!(Diagnostic::decode(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_pistol(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_weapons(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_pickups(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_armor(&bytes, DiagnosticLimits::default()).is_ok());
}
#[test]
fn malformed_armor_diagnostic_fields_and_events_reject_safely() {
    let (mut s, _) = fresh(8);
    s.advance(armor_evidence::commands(&armor_evidence::cases()[8], 1))
        .unwrap();
    let snap = s.snapshot();
    let bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
    for (name, value) in [
        ("profile", 9u32.to_le_bytes().to_vec()),
        ("original_player.0.armor.type", vec![3]),
        (
            "original_player.0.armor.points",
            (-1i32).to_le_bytes().to_vec(),
        ),
        (
            "original_player.0.armor.points",
            101i32.to_le_bytes().to_vec(),
        ),
    ] {
        let mut bad = bytes.clone();
        replace_field(&mut bad, name, &value);
        assert!(
            Diagnostic::decode_armor(&bad, DiagnosticLimits::default()).is_err(),
            "{name}"
        );
    }
    for f in snap
        .fields()
        .filter(|f| f.path().starts_with("events.last.") && f.value().starts_with(&[23, 7]))
    {
        for offset in [2, 3, 4, 9] {
            let mut v = f.value().to_vec();
            v[offset] = 255;
            let mut bad = bytes.clone();
            replace_field(&mut bad, f.path(), &v);
            assert!(Diagnostic::decode_armor(&bad, DiagnosticLimits::default()).is_err());
        }
    }
    let (mut s, a) = component(1, 100, 100, 0);
    damage(&mut s, a, 60, true);
    let snap = s.snapshot();
    let bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
    let f = snap
        .fields()
        .find(|f| f.path().starts_with("events.pending.") && f.value().first() == Some(&24))
        .unwrap();
    let mut v = f.value().to_vec();
    v[17] = 21;
    let mut bad = bytes.clone();
    replace_field(&mut bad, f.path(), &v);
    assert!(Diagnostic::decode_armor(&bad, DiagnosticLimits::default()).is_err());
    for n in [0, 4, 16, bytes.len() - 1] {
        assert!(Diagnostic::decode_armor(&bytes[..n], DiagnosticLimits::default()).is_err());
    }
}
#[test]
fn armor_retirement_and_effect_slot_reuse_do_not_redirect_old_item_identity() {
    let c = &armor_evidence::cases()[22];
    let (mut s, _) = fresh(22);
    let item = s
        .actor_order()
        .find(|id| {
            s.actor(*id).and_then(Actor::life).is_some_and(|l| {
                world_pickups::kind(l.actor_type).is_some_and(PickupKind::is_armor)
            })
        })
        .unwrap();
    for op in 1..=128 {
        s.advance(armor_evidence::commands(c, op)).unwrap();
    }
    assert!(s.actor(item).is_none());
    assert_eq!(
        s.actor_availability(item),
        ActorAvailability::StaleGeneration
    );
    let reused = s.actor_order().find(|id| id.slot() == item.slot()).unwrap();
    assert_ne!(item.generation(), reused.generation());
    let before = s.snapshot();
    assert!(s.set_actor_target(item, None).is_err());
    assert_eq!(s.snapshot(), before);
}
