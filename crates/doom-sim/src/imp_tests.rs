//! M3h2 Imp, fireball and mixed-roster behavior against compiled original-C rows.
//! Expected values never come from Rust output.
use super::*;
#[test]
fn imp_source_case_00_imp_melee_contact() {
    imp_evidence::verify_case(0);
}
#[test]
fn imp_source_case_01_imp_melee_boundary() {
    imp_evidence::verify_case(1);
}
#[test]
fn imp_source_case_02_imp_missile_medium() {
    imp_evidence::verify_case(2);
}
#[test]
fn imp_source_case_03_imp_missile_far() {
    imp_evidence::verify_case(3);
}
#[test]
fn imp_source_case_04_imp_varied_ranges() {
    imp_evidence::verify_case(4);
}
#[test]
fn imp_source_case_05_fireball_wall_miss() {
    imp_evidence::verify_case(5);
}
#[test]
fn imp_source_case_06_player_dodge() {
    imp_evidence::verify_case(6);
}
#[test]
fn imp_source_case_07_fireball_hits_former_human() {
    imp_evidence::verify_case(7);
}
#[test]
fn imp_source_case_08_fireball_hits_shotguy() {
    imp_evidence::verify_case(8);
}
#[test]
fn imp_source_case_09_fireball_hits_demon() {
    imp_evidence::verify_case(9);
}
#[test]
fn imp_source_case_10_fireball_hits_imp() {
    imp_evidence::verify_case(10);
}
#[test]
fn imp_source_case_11_fireball_spawn_blocked() {
    imp_evidence::verify_case(11);
}
#[test]
fn imp_source_case_12_fireball_passes_item() {
    imp_evidence::verify_case(12);
}
#[test]
fn imp_source_case_13_former_human_shoots_imp() {
    imp_evidence::verify_case(13);
}
#[test]
fn imp_source_case_14_imp_killed_pistol() {
    imp_evidence::verify_case(14);
}
#[test]
fn imp_source_case_15_imp_xdeath() {
    imp_evidence::verify_case(15);
}
#[test]
fn imp_source_case_16_imp_dies_with_fireball_flying() {
    imp_evidence::verify_case(16);
}
#[test]
fn imp_source_case_17_player_death_fireball() {
    imp_evidence::verify_case(17);
}
#[test]
fn imp_source_case_18_mixed_roster() {
    imp_evidence::verify_case(18);
}
#[test]
fn imp_source_case_19_roster_eight_imps() {
    imp_evidence::verify_case(19);
}
#[test]
fn imp_source_case_20_imps_pursuit() {
    imp_evidence::verify_case(20);
}

use crate::identity::{OriginalActorType, OriginalState};
use crate::lifecycle::source_sound_id;
use imp_evidence::{cases, commands, controlled, setup, setup_with_limit};

const IMP_DOOMED: i16 = 3001;
const FIREBALL: ActorTypeId = ActorTypeId(OriginalActorType::MT_TROOPSHOT.ordinal());

fn map_with_things(things: &[[i16; 5]]) -> doom_map::Map {
    // Test-input assembly only: case-0 geometry with replaced THINGS.
    let bytes = imp_evidence::MAP_BYTES[0];
    let u32at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let count = u32at(4);
    let directory = u32at(8);
    let mut output = vec![0; 12];
    output[..4].copy_from_slice(b"PWAD");
    let mut entries = Vec::new();
    for i in 0..count {
        let at = directory + i * 16;
        let name = &bytes[at + 8..at + 16];
        let offset = output.len();
        if name == b"THINGS\0\0" {
            for t in things {
                for v in t {
                    output.extend(v.to_le_bytes());
                }
            }
        } else {
            output.extend_from_slice(&bytes[u32at(at)..u32at(at) + u32at(at + 4)]);
        }
        entries.extend((offset as u32).to_le_bytes());
        entries.extend(((output.len() - offset) as u32).to_le_bytes());
        entries.extend_from_slice(name);
    }
    output[4..8].copy_from_slice(&(count as u32).to_le_bytes());
    let directory = output.len() as u32;
    output[8..12].copy_from_slice(&directory.to_le_bytes());
    output.extend(entries);
    let wad = doom_wad::Wad::parse(&output, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}

/// The failing tic must leave the complete canonical state, RNG and queues unchanged.
fn advance_rolls_back(s: &mut Simulation, command: [TicCommand; 4]) -> Result<(), SimError> {
    let before = s.clone();
    let result = s.advance(command);
    if result.is_err() {
        assert_eq!(s.snapshot(), before.snapshot(), "rollback snapshot");
        assert_eq!(s.rng(), before.rng(), "rollback rng");
        assert_eq!(s.events(), before.events(), "rollback events");
        assert_eq!(s.pending_events(), before.pending_events());
    }
    result
}

fn first_error(which: usize, limit: u32) -> Option<(usize, SimError)> {
    let c = &cases()[which];
    let (mut s, a, things) = setup_with_limit(c, limit);
    for op in 1..=c.tics {
        controlled(&mut s, c, a, &things, op).unwrap();
        if let Err(error) = advance_rolls_back(&mut s, commands(c, op)) {
            return Some((op, error));
        }
    }
    None
}

#[test]
fn fireball_capacity_error_rolls_back_whole_tic() {
    // imp_missile_far: Imp and player fill the store, so the first launch has
    // no slot. The source launch happens at op43 in the committed C trace.
    assert_eq!(
        first_error(3, 2),
        Some((43, SimError::Store(StoreError::Capacity)))
    );
}

#[test]
fn every_store_capacity_completes_or_rolls_back() {
    // Each limit either matches the full run or rejects one tic with full
    // rollback; none may panic, including a pass that retires and refills a slot.
    for which in [0, 7, 12, 18, 19] {
        let body_and_things = setup(&cases()[which]).2.len() as u32 + 1;
        for limit in body_and_things..=24 {
            first_error(which, limit);
        }
    }
}

fn live_fireballs(s: &Simulation) -> usize {
    s.actors.live_of_type(FIREBALL)
}

#[test]
fn live_projectile_cap_rejects_launch_with_full_rollback() {
    let c = &cases()[19];
    let (mut s, a, things) = setup(c);
    let imp = things[0];
    for _ in 0..MAX_LIVE_PROJECTILES {
        s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, imp, a.body))
            .unwrap()
            .unwrap();
    }
    assert_eq!(live_fireballs(&s), MAX_LIVE_PROJECTILES);
    // The direct service rejects before any RNG, event or actor change.
    let before = s.clone();
    assert_eq!(
        s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, imp, a.body))
            .unwrap(),
        Err(ProjectileError::WorkLimit.into())
    );
    assert_eq!(s.snapshot(), before.snapshot());
    assert_eq!(s.rng(), before.rng());
    // An actual A_TroopAttack in a tic discards that whole staged tic.
    let imp = things[1];
    s.set_actor_target(imp, Some(a.body)).unwrap();
    s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
        .unwrap();
    let error = (0..30)
        .find_map(|_| {
            advance_rolls_back(&mut s, [TicCommand::default(); 4])
                .err()
                .filter(|_| live_fireballs(&s) == MAX_LIVE_PROJECTILES)
        })
        .expect("a launch reaches the cap before any fireball lands");
    assert_eq!(error, ProjectileError::WorkLimit.into());
}

#[test]
fn slot_refilled_in_the_same_pass_is_a_recoverable_rejection() {
    // Thinker order: possessed, Imp, player, retiring puff, fireball. The full
    // store frees the puff's slot, then the fireball's kill spawns a drop into it,
    // so one pass makes more visits than the store has slots.
    let map = map_with_things(&[[-200, 0, 0, 3004, 7], [-260, 0, 0, IMP_DOOMED, 7]]);
    let mut s = Simulation::original_player_imps_map(map, 5, MapContextOptions::default()).unwrap();
    let things = s.spawn_map_things().unwrap().spawned;
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
    let health = s.actor(things[0]).unwrap().life().unwrap().health;
    s.damage_actor(DamageRequest {
        target: things[0],
        inflictor: None,
        source: None,
        amount: health - 1,
    })
    .unwrap();
    let puff = s
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::from_map_units(100), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, things[1], things[0]))
        .unwrap()
        .unwrap();
    let _ = body;
    for _ in 0..12 {
        let mut attempt = s.clone();
        attempt.mark_remove(puff).unwrap();
        let before = attempt.clone();
        match attempt.advance([TicCommand::default(); 4]) {
            Err(error) => {
                assert_eq!(error, SimError::Store(StoreError::VisitLimit));
                assert_eq!(attempt.snapshot(), before.snapshot());
                assert_eq!(attempt.rng(), before.rng());
                return;
            }
            Ok(()) => {
                if s.advance([TicCommand::default(); 4]).is_err() {
                    break;
                }
            }
        }
    }
    panic!("the fireball never reached the former human");
}

#[test]
fn fireball_explodes_on_same_species_without_damage_or_retarget() {
    // fireball_hits_imp: walk the committed scenario and find a same-species
    // block. PIT_CheckThing returns false before any draw or P_DamageMobj.
    let c = &cases()[10];
    let (mut s, a, things) = setup(c);
    let mut same_species = 0;
    for op in 1..=c.tics {
        let pending = s.pending_events().len();
        controlled(&mut s, c, a, &things, op).unwrap();
        let before = things
            .iter()
            .map(|id| s.actor(*id).unwrap().life().unwrap())
            .collect::<Vec<_>>();
        s.advance(commands(c, op)).unwrap();
        let events = &s.events()[pending..];
        for (i, e) in events.iter().enumerate() {
            let Event::SectorMovement { actor, attempt } = e else {
                continue;
            };
            let Some(blocker) = attempt.movement.blocked_by_actor else {
                continue;
            };
            let Some(index) = things.iter().position(|t| *t == blocker) else {
                continue;
            };
            let owner = s.actor(*actor).unwrap().life().unwrap();
            if owner.actor_type != FIREBALL || owner.target == Some(blocker) {
                continue;
            }
            same_species += 1;
            assert!(matches!(
                events.get(i + 1),
                Some(Event::Projectile(ProjectileEvent::Exploded(id))) if id == actor
            ));
            assert!(!events[..i].iter().any(
                |e| matches!(e, Event::Combat(CombatEvent::Damage(r)) if r.target == blocker)
            ));
            let after = s.actor(blocker).unwrap().life().unwrap();
            assert_eq!(after.health, before[index].health);
            assert_eq!(after.target, before[index].target);
        }
    }
    assert!(same_species > 0, "scenario reaches the same-species branch");
}

#[test]
fn imp_melee_choice_never_launches_inside_range() {
    // A_TroopAttack at contact: claw sound, damage and no missile in that action.
    let c = &cases()[0];
    let (mut s, a, things) = setup(c);
    let claw = source_sound_id("sfx_claw");
    let mut claws = 0;
    for op in 1..=c.tics {
        controlled(&mut s, c, a, &things, op).unwrap();
        s.advance(commands(c, op)).unwrap();
        let events = s.events();
        for (i, e) in events.iter().enumerate() {
            if matches!(e, Event::ActorState(ActorStateEvent::Sound { sound, .. }) if *sound == claw)
            {
                claws += 1;
                assert!(
                    events[i..]
                        .iter()
                        .all(|e| !matches!(e, Event::Projectile(ProjectileEvent::Launch(_))))
                );
            }
        }
    }
    assert!(claws > 0);
}

#[test]
fn fireball_public_controls_reject_atomically() {
    let c = &cases()[3];
    let (mut s, a, things) = setup(c);
    let fireball = s
        .checked_demon_operation(|h| {
            crate::projectiles::launch_fireball(h, things[0], a.body).unwrap();
            h.access.live_of_type(FIREBALL)
        })
        .map(|_| s.actors.order().last().unwrap())
        .unwrap();
    let before = s.clone();
    assert_eq!(
        s.spawn_type(FIREBALL, [Fixed::ZERO; 2], SpawnHeight::Floor),
        Err(ProjectileError::Controlled.into())
    );
    assert!(
        s.enter_original_state(fireball, OriginalState::S_TBALLX1)
            .is_err()
    );
    assert!(s.set_actor_target(fireball, None).is_err());
    assert!(s.set_actor_target(things[0], Some(fireball)).is_err());
    assert!(s.set_momentum(fireball, [Fixed::ZERO; 2]).is_err());
    assert!(s.set_vertical_momentum(fireball, Fixed::ZERO).is_err());
    assert!(
        s.damage_actor(DamageRequest {
            target: fireball,
            inflictor: None,
            source: None,
            amount: 1,
        })
        .is_err()
    );
    assert!(
        s.damage_actor(DamageRequest {
            target: things[0],
            inflictor: None,
            source: Some(fireball),
            amount: 1,
        })
        .is_err()
    );
    assert!(
        s.damage_actor(DamageRequest {
            target: a.body,
            inflictor: Some(fireball),
            source: Some(things[0]),
            amount: 1,
        })
        .is_err()
    );
    assert_eq!(s.snapshot(), before.snapshot());
    assert_eq!(s.rng(), before.rng());
}

#[test]
fn removed_owner_rejects_flying_fireball_atomically() {
    let c = &cases()[3];
    let (mut s, a, things) = setup(c);
    s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, things[0], a.body))
        .unwrap()
        .unwrap();
    s.mark_remove(things[0]).unwrap();
    assert_eq!(
        advance_rolls_back(&mut s, commands(c, 1)),
        Err(ProjectileError::Owner.into())
    );
}

#[test]
fn stale_and_foreign_handles_reject_for_imps() {
    let c = &cases()[3];
    let (mut s, _, things) = setup(c);
    let imp = things[0];
    let stale = ActorId::from_parts(imp.slot(), imp.generation() + 1);
    let before = s.clone();
    for target in [stale, ActorId::from_parts(40, 1)] {
        assert!(
            s.damage_actor(DamageRequest {
                target,
                inflictor: None,
                source: None,
                amount: 10,
            })
            .is_err()
        );
        assert!(s.set_actor_target(imp, Some(target)).is_err());
        assert_eq!(s.snapshot(), before.snapshot());
    }
    let (other, ..) = setup(&cases()[0]);
    let mut left = GameInstance::new(before);
    let mut right = GameInstance::new(other);
    let id = DefinitionIdentity::Original(OriginalActorType::MT_TROOP);
    let reference = left
        .with_execution_context(|ctx| {
            let actor = ctx.admit_local(imp, id)?;
            ctx.export_actor(actor)
        })
        .unwrap();
    let snapshot = right.simulation().snapshot();
    assert!(
        right
            .with_execution_context(|ctx| {
                ctx.import_actor(&reference, id)?;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(right.simulation().snapshot(), snapshot);
}

#[test]
fn historical_policies_reject_imps_and_fireballs() {
    let thing = [[-300, 0, 0, IMP_DOOMED, 7]];
    let mut monsters = Simulation::original_player_monsters_map(
        map_with_things(&thing),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    let before = monsters.clone();
    assert_eq!(
        monsters.spawn_map_things().unwrap_err(),
        SimError::MapSpawn(MapSpawnError::UnsupportedType(IMP_DOOMED))
    );
    assert_eq!(monsters.snapshot(), before.snapshot());
    let mut bonuses = Simulation::original_player_bonuses_map(
        map_with_things(&thing),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    assert_eq!(
        bonuses.spawn_map_things().unwrap_err(),
        SimError::MapSpawn(MapSpawnError::UnsupportedType(IMP_DOOMED))
    );
    for profile in [
        OriginalExecutionProfile::PlayerMonstersV1,
        OriginalExecutionProfile::PlayerDemonBonusesV1,
        OriginalExecutionProfile::PlayerDemonArmorV1,
        OriginalExecutionProfile::DemonNormalV1,
    ] {
        let registry = OriginalExecutionRegistry::new(profile).unwrap();
        for ty in [OriginalActorType::MT_TROOP, OriginalActorType::MT_TROOPSHOT] {
            assert_eq!(
                registry.admit_definition(DefinitionIdentity::Original(ty)),
                Err(OriginalRegistryError::Unsupported(
                    DefinitionIdentity::Original(ty)
                ))
            );
        }
    }
}

#[test]
fn imp_policy_excludes_raise_and_other_monsters() {
    let registry =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersImpV1).unwrap();
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersImpV1.wire_id(),
        OriginalExecutionProfile::PlayerMonstersV1.wire_id() + 1
    );
    let imp = DefinitionIdentity::Original(OriginalActorType::MT_TROOP);
    let roots = OriginalActorType::MT_TROOP.states();
    for state in [
        roots.spawn,
        roots.see,
        roots.pain,
        roots.melee,
        roots.missile,
        roots.death,
        roots.extreme_death,
    ] {
        registry.admit_state(imp, state).unwrap();
    }
    assert_eq!(
        registry.admit_state(imp, roots.raise),
        Err(OriginalRegistryError::State(roots.raise.ordinal()))
    );
    let fireball = DefinitionIdentity::Original(OriginalActorType::MT_TROOPSHOT);
    registry
        .admit_state(fireball, OriginalState::S_TBALL1)
        .unwrap();
    registry
        .admit_state(fireball, OriginalState::S_TBALLX3)
        .unwrap();
    assert!(
        registry
            .admit_state(fireball, OriginalState::S_TROO_RUN1)
            .is_err()
    );
    for ty in [
        OriginalActorType::MT_VILE,
        OriginalActorType::MT_HEAD,
        OriginalActorType::MT_HEADSHOT,
        OriginalActorType::MT_PLASMA,
    ] {
        assert!(
            registry
                .admit_definition(DefinitionIdentity::Original(ty))
                .is_err()
        );
    }
}

#[test]
fn imp_direct_entry_rejects_death_and_pain_states() {
    let c = &cases()[3];
    let (mut s, _, things) = setup(c);
    let roots = OriginalActorType::MT_TROOP.states();
    for state in [roots.death, roots.extreme_death, roots.pain] {
        let before = s.clone();
        assert!(
            s.enter_original_state(things[0], state).is_err(),
            "{state:?}"
        );
        assert_eq!(s.snapshot(), before.snapshot());
    }
    s.enter_original_state(things[0], roots.see).unwrap();
}

#[test]
fn imp_roster_cap_rejects_ninth_monster_atomically() {
    let things: Vec<[i16; 5]> = (0..9)
        .map(|i| [-300 + 50 * (i % 3), -60 + 60 * (i / 3), 0, IMP_DOOMED, 7])
        .collect();
    let mut s = Simulation::original_player_imps_map(
        map_with_things(&things),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    let before = s.clone();
    assert_eq!(
        s.spawn_map_things().unwrap_err(),
        SimError::MapSpawn(MapSpawnError::WorkLimit)
    );
    assert_eq!(s.snapshot(), before.snapshot());
    assert_eq!(s.rng(), before.rng());
}

#[test]
fn dsim29_reader_is_explicit_and_rejects_malformed_frames() {
    let c = &cases()[19];
    let (mut s, a, things) = setup(c);
    for op in 1..=60 {
        controlled(&mut s, c, a, &things, op).unwrap();
        s.advance(commands(c, op)).unwrap();
        if live_fireballs(&s) > 0 {
            break;
        }
    }
    assert!(live_fireballs(&s) > 0);
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_imps(&bytes, limits).unwrap();
    assert_eq!(view.version(), 29);
    assert_eq!(view.encode(limits).unwrap(), bytes);
    assert!(Diagnostic::decode_monsters(&bytes, limits).is_err());
    assert!(Diagnostic::decode_bonuses(&bytes, limits).is_err());
    assert!(Diagnostic::decode(&bytes, limits).is_err());
    let patch = |path: &str, value: &[u8]| {
        let mut out = bytes.clone();
        let needle = [&(path.len() as u32).to_le_bytes()[..], path.as_bytes()].concat();
        let at = out.windows(needle.len()).position(|w| w == needle).unwrap() + needle.len();
        let len = u64::from_le_bytes(out[at..at + 8].try_into().unwrap()) as usize;
        assert_eq!(len, value.len());
        out[at + 8..at + 8 + len].copy_from_slice(value);
        out
    };
    for (path, value) in [
        ("level.total_kills", 9_i32.to_le_bytes().to_vec()),
        ("level.total_kills", 7_i32.to_le_bytes().to_vec()),
        (
            "level.single_player_kill_count",
            9_i32.to_le_bytes().to_vec(),
        ),
        ("profile", 12_u32.to_le_bytes().to_vec()),
    ] {
        assert!(
            Diagnostic::decode_imps(&patch(path, &value), limits).is_err(),
            "{path}"
        );
    }
    // An Imp or fireball in a v28 frame is not an admitted former-human identity.
    let mut v28 = bytes.clone();
    v28[4..8].copy_from_slice(&28_u32.to_le_bytes());
    assert!(Diagnostic::decode_monsters(&v28, limits).is_err());
    // Former-human bytes still decode only through their own reader.
    let old = monster_evidence::setup(&monster_evidence::cases()[0])
        .0
        .snapshot();
    let old = old.diagnostic_bytes(limits).unwrap();
    assert!(Diagnostic::decode_monsters(&old, limits).is_ok());
    assert!(Diagnostic::decode_imps(&old, limits).is_err());
}

#[test]
fn dsim29_reader_enforces_the_live_projectile_bound() {
    let c = &cases()[19];
    let (mut s, a, things) = setup(c);
    for _ in 0..MAX_LIVE_PROJECTILES {
        s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, things[0], a.body))
            .unwrap()
            .unwrap();
    }
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    assert!(Diagnostic::decode_imps(&bytes, limits).is_ok());
    // Retype the Imp's type record: one more live fireball than admitted.
    let imp_type = format!("actor.{}.type", things[1].slot());
    let needle = [
        &(imp_type.len() as u32).to_le_bytes()[..],
        imp_type.as_bytes(),
    ]
    .concat();
    let mut over = bytes.clone();
    let at = over
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap()
        + needle.len()
        + 8;
    over[at..at + 2].copy_from_slice(&FIREBALL.0.to_le_bytes());
    assert!(Diagnostic::decode_imps(&over, limits).is_err());
}

#[test]
fn dsim29_live_bound_does_not_depend_on_field_order() {
    // Review regression: duplicate-free frames may list fields in any order.
    let c = &cases()[19];
    let (mut s, a, things) = setup(c);
    for _ in 0..MAX_LIVE_PROJECTILES {
        s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, things[0], a.body))
            .unwrap()
            .unwrap();
    }
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    // Reverse the whole field list: the count must not change.
    let snapshot = s.snapshot();
    let mut reversed = bytes[..16].to_vec();
    for f in snapshot.fields().collect::<Vec<_>>().into_iter().rev() {
        reversed.extend((f.path().len() as u32).to_le_bytes());
        reversed.extend(f.path().as_bytes());
        reversed.extend((f.value().len() as u64).to_le_bytes());
        reversed.extend(f.value());
    }
    for frame in [&bytes, &reversed] {
        let view = Diagnostic::decode_imps(frame, limits).unwrap();
        assert!(view.validate_live_projectiles_at(16).is_ok());
        assert!(view.validate_live_projectiles_at(15).is_err());
    }
}

#[test]
fn fireball_position_query_uses_the_missile_contact_rule() {
    // Review regression: a fresh fireball overlaps its owner, which never blocks it.
    let c = &cases()[3];
    let (mut s, a, things) = setup(c);
    s.checked_demon_operation(|h| crate::projectiles::launch_fireball(h, things[0], a.body))
        .unwrap()
        .unwrap();
    let fireball = s.actors.order().last().unwrap();
    let position = s.actor(fireball).unwrap().position();
    assert!(
        s.check_multisector_position(fireball, position)
            .unwrap()
            .allowed()
    );
}

#[test]
fn imp_replay_digest_is_stable_across_runs() {
    let run = || {
        cases().iter().enumerate().fold(0, |acc, (which, c)| {
            let (mut s, a, things) = setup(c);
            for op in 1..=c.tics {
                controlled(&mut s, c, a, &things, op).unwrap();
                s.advance(commands(c, op)).unwrap();
            }
            acc ^ s.snapshot().digest().rotate_left(which as u32)
        })
    };
    assert_eq!(run(), run());
}
