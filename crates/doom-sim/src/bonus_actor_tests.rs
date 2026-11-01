use super::*;

#[test]
fn bonus_actor_identity_flags_and_policy_isolation() {
    let registry =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonBonusesV1).unwrap();
    assert_eq!(OriginalExecutionProfile::PlayerDemonBonusesV1.wire_id(), 11);
    for (kind, ty, doomed, root, sprite) in [
        (PickupKind::HealthBonus, 45, 2014, 816, 60),
        (PickupKind::ArmorBonus, 46, 2015, 822, 61),
    ] {
        let original = kind.original_type();
        assert_eq!(original.ordinal(), ty);
        assert_eq!(original.descriptor().doomed_number, doomed);
        assert_eq!(original.descriptor().flags, 0x800001);
        assert_eq!(original.states().spawn.ordinal(), root);
        assert_eq!(
            original.states().spawn.descriptor().sprite.ordinal(),
            sprite
        );
        world_pickups::validate_definition(ActorTypeId(ty)).unwrap();
        for policy in [
            OriginalExecutionProfile::PlayerDemonPickupsV1,
            OriginalExecutionProfile::PlayerDemonArmorV1,
        ] {
            let old = OriginalExecutionRegistry::new(policy).unwrap();
            assert!(
                old.admit_definition(DefinitionIdentity::Original(original))
                    .is_err()
            );
        }
        let mut state = original.states().spawn;
        for frame in [0, 1, 2, 3, 2, 1] {
            registry
                .admit_state(DefinitionIdentity::Original(original), state)
                .unwrap();
            assert_eq!(state.descriptor().frame, frame);
            assert_eq!(state.descriptor().tics, 6);
            assert!(state.descriptor().action.is_none());
            state = state.descriptor().next;
        }
        assert_eq!(state, original.states().spawn);
    }
}

#[test]
fn independent_bonus_actor_clocks_and_spawn_rng() {
    for which in [1, 4, 7, 9, 10] {
        let cases = bonus_evidence::cases();
        let c = &cases[which];
        let (mut sim, _) = bonus_evidence::setup(c);
        let mut trace = bonus_evidence::SourceTrace::seeded(c.seed);
        for op in 0..=c.tics {
            let pending = sim.pending_events().len();
            if op > 0 {
                sim.advance(bonus_evidence::commands(c, op)).unwrap();
            }
            let events = if op == 0 {
                sim.pending_events()
            } else {
                &sim.events()[pending..]
            };
            let rows = bonus_evidence::rows(which, op);
            trace.check(&sim, which, op, events, rows);
            assert_eq!(sim.snapshot().version(), 27);
        }
    }
}

#[test]
fn malformed_bonus_actor_state_and_flags_reject_before_tic() {
    for which in [1, 4] {
        for mutation in 0..6 {
            let cases = bonus_evidence::cases();
            let (mut s, _) = bonus_evidence::setup(&cases[which]);
            let id = s
                .actor_order()
                .find(|id| {
                    s.actor(*id).unwrap().life().unwrap().actor_type.0
                        == if which == 1 { 45 } else { 46 }
                })
                .unwrap();
            let life = s.actors.get_mut(id).unwrap().life.as_mut().unwrap();
            match mutation {
                0 => life.flags &= !world_pickups::MF_COUNTITEM,
                1 => life.flags |= world_pickups::MF_COUNTKILL,
                2 => life.tics = 7,
                3 => life.tics = 0,
                4 => life.sprite = 55,
                _ => life.state = Some(StateId(if which == 1 { 822 } else { 816 })),
            }
            let before = s.snapshot();
            assert!(s.advance([TicCommand::default(); 4]).is_err());
            assert_eq!(s.snapshot(), before);
        }
    }
}
