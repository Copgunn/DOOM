//! Bounded original CPOS actions and immediate source refire handoffs.
use crate::{
    lifecycle::{ActorAccess, LifeHost},
    *,
};
/// Original refire short-circuit outcome, before any immediate see-state action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefireDecision {
    /// Random <40 returns early, preserving the natural attack loop.
    EarlyContinue,
    /// Null target, reached only after the random test.
    NoTarget,
    /// Target mobj health <=0, without a refire sight query.
    DeadTarget,
    /// Healthy target failed the original REJECT/BSP sight query.
    NotVisible,
    /// Healthy visible target preserves the natural attack loop.
    VisibleContinue,
}
impl RefireDecision {
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::EarlyContinue => 0,
            Self::NoTarget => 1,
            Self::DeadTarget => 2,
            Self::NotVisible => 3,
            Self::VisibleContinue => 4,
        }
    }
    pub(crate) const fn exits(self) -> bool {
        matches!(self, Self::NoTarget | Self::DeadTarget | Self::NotVisible)
    }
}
/// Ordered refire outcome; existing events carry facing, RNG, sight and state entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefireEvent {
    /// Native owner identity.
    pub actor: ActorId,
    /// Source short-circuit branch.
    pub decision: RefireDecision,
}
/// Keep short-circuit evaluation explicit; the controlled C boundary probe also
/// checks value 40, which never occurs in the original gameplay RNG table.
pub(crate) fn decide(
    roll: u8,
    health: impl FnOnce() -> Option<i32>,
    sight: impl FnOnce() -> SightResult,
) -> (RefireDecision, Option<SightResult>) {
    if roll < 40 {
        return (RefireDecision::EarlyContinue, None);
    }
    let Some(health) = health() else {
        return (RefireDecision::NoTarget, None);
    };
    if health <= 0 {
        return (RefireDecision::DeadTarget, None);
    }
    let result = sight();
    (
        if result.visible {
            RefireDecision::VisibleContinue
        } else {
            RefireDecision::NotVisible
        },
        Some(result),
    )
}
pub(crate) fn attack<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    crate::monsters::single_attack(h, id, true);
}
pub(crate) fn refire<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, work: &mut usize) {
    crate::monsters::face(h, id);
    let roll = crate::monsters::draw(h, id, 6);
    let a = *h.access.get(id).unwrap();
    let target = a.life.unwrap().target;
    let (decision, sight) = decide(
        roll,
        || target.map(|id| h.access.get(id).unwrap().life.unwrap().health),
        || {
            let t = *h.access.get(target.unwrap()).unwrap();
            crate::sight::query(h.world, h.sectors, a, t)
        },
    );
    if let Some(result) = sight {
        h.events.push(Event::Perception(PerceptionEvent::Sight {
            observer: id,
            target: target.unwrap(),
            result,
        }));
    }
    h.events.push(Event::Refire(RefireEvent {
        actor: id,
        decision,
    }));
    if decision.exits() {
        let see = h
            .registry
            .actor_type(a.life.unwrap().actor_type)
            .unwrap()
            .states()[1];
        h.enter_counted(id, see, work);
    }
}
impl Simulation {
    /// Explicit CPOS attack/refire fixtures; full original Chaingunner stays excluded.
    pub fn with_chaingunner_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(PerceptionError::WorkLimit.into());
        }
        if map.linedefs().iter().any(|l| l.special != 0)
            || map.sectors().iter().any(|s| s.special != 0)
        {
            return Err(PursuitError::Specials.into());
        }
        crate::sight::validate_map(&map)?;
        validate_registry(&registry)?;
        let mut sim = Self::with_combat_registry(map, limit, registry, options)?;
        sim.profile = Profile::Chaingunner;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    pub(crate) fn validate_chaingun_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        if ![37, 38, 135, 136].contains(&ty.0) {
            return Err(MonsterError::Definition.into());
        }
        let info = self
            .registry
            .as_deref()
            .unwrap()
            .actor_type(ty)
            .ok_or(MonsterError::Definition)?;
        if ty.0 == 136 && info.name != "FIXTURE_CHAINGUNNER_136"
            || ty.0 == 135 && info.name != "FIXTURE_DAMAGE_TARGET"
        {
            return Err(MonsterError::Definition.into());
        }
        self.registry
            .as_deref()
            .unwrap()
            .validate_type_chaingunner(ty, &self.sectors)?;
        Ok(())
    }
    pub(crate) fn chaingun_state_role(
        &self,
        ty: ActorTypeId,
        state: StateId,
    ) -> Result<(), SimError> {
        let valid = match ty.0 {
            136 => (149..=173).contains(&state.0) || [1, 2, 4, 5, 6, 7].contains(&state.0),
            135 => (149..=173).contains(&state.0),
            37 | 38 => (90..=96).contains(&state.0),
            _ => false,
        };
        if !valid {
            return Err(MonsterError::Definition.into());
        }
        Ok(())
    }
    /// Atomic original single CPosAttack, independently aimed at each invocation.
    pub fn chaingunner_attack(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.profile != Profile::Chaingunner {
            return Err(MonsterError::Profile.into());
        }
        let plan = self.prepare_monster_shot(id)?;
        let mut host = self.monster_host(plan.as_ref());
        host.events.push(Event::ActorState(ActorStateEvent::Action {
            actor: id,
            action: ActorAction::Original(lifecycle::source_action_id("A_CPosAttack")),
        }));
        attack(&mut host, id);
        Ok(())
    }
    /// Atomic original refire, including immediate see/Chase movement and actions.
    pub fn chaingunner_refire(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.profile != Profile::Chaingunner {
            return Err(MonsterError::Profile.into());
        }
        self.preflight_monster_callback(id)?;
        self.validate_entry(StateId(2))?;
        // Immutable positive CPOS durations prevent a nested Chase selection from
        // firing immediately. All sight/movement services are infallible after
        // the shared owner/map/roster/reference/arithmetic checks. Refire allocates
        // no actors. The existing bounded state host executes the entire handoff.
        let mut host = self.monster_host(None);
        host.events.push(Event::ActorState(ActorStateEvent::Action {
            actor: id,
            action: ActorAction::Original(lifecycle::source_action_id("A_CPosRefire")),
        }));
        refire(&mut host, id, &mut 0);
        Ok(())
    }
}
fn validate_registry(r: &ActorRegistry) -> Result<(), SimError> {
    let source = ActorRegistry::vanilla();
    if r.states() == source.states() && r.types() == source.types() {
        return Ok(());
    }
    if r.states().len() != source.states().len() || r.types().len() != source.types().len() {
        return Err(MonsterError::Definition.into());
    }
    for (i, s) in r.states().iter().enumerate() {
        let expected = match i {
            1 | 2 => ActorStateDef {
                name: if i == 1 {
                    "FIXTURE_CPOS_LOOK"
                } else {
                    "FIXTURE_CPOS_CHASE"
                }
                .into(),
                sprite: 38,
                frame: 0,
                tics: s.tics,
                action: ActorAction::Original(lifecycle::source_action_id(if i == 1 {
                    "A_Look"
                } else {
                    "A_Chase"
                })),
                next: StateId(i as u16),
                misc: [0; 2],
            },
            4..=7 => {
                let original = source
                    .states()
                    .iter()
                    .find(|s| s.name == format!("S_CPOS_ATK{}", i - 3))
                    .unwrap();
                let mut expected = original.clone();
                expected.name = format!("FIXTURE_CPOS_ATK{}", i - 3);
                expected.next = StateId(if i == 7 { 5 } else { i as u16 + 1 });
                expected
            }
            _ => source.states()[i].clone(),
        };
        if *s != expected || [1, 2].contains(&i) && !(1..=35).contains(&s.tics) {
            return Err(MonsterError::Definition.into());
        }
    }
    for (i, t) in r.types().iter().enumerate() {
        if ![135, 136].contains(&i) {
            if t != &source.types()[i] {
                return Err(MonsterError::Definition.into());
            }
            continue;
        }
        let owner = i == 136;
        let mut expected = source.types()[0].clone();
        expected.name = if owner {
            "FIXTURE_CHAINGUNNER_136"
        } else {
            "FIXTURE_DAMAGE_TARGET"
        }
        .into();
        expected.properties[0] = -1;
        let variable: &[usize] = if owner {
            &[4, 15, 16, 17, 20, 21]
        } else {
            &[2, 8, 16, 17, 21]
        };
        for &key in variable {
            expected.properties[key] = t.properties[key];
        }
        if owner {
            expected.properties[1] = 1;
            expected.properties[3] = 2;
            expected.properties[5] = 8;
            expected.properties[10] = 0;
            expected.properties[11] = 4;
        }
        let flags = if owner {
            2 | 32 | 64 | 128 | 1024 | 2048 | 2097152 | 33554432
        } else {
            2 | 4 | 32 | 64 | 1024 | 2048 | 262144 | 524288 | 33554432
        };
        if *t != expected
            || t.radius() < Fixed::ZERO
            || t.radius() > Fixed::from_map_units(32)
            || t.height() <= Fixed::ZERO
            || t.flags() & !flags != 0
            || owner && !(1..=16).contains(&t.speed())
            || !owner
                && (!(1..=1000).contains(&t.spawn_health())
                    || !(0..=256).contains(&t.pain_chance()))
        {
            return Err(MonsterError::Definition.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controlled_c_dependency_probe_covers_40_and_short_circuit_boundaries() {
        let rows = include_str!("../../../tests/reference/m3d3e_refire_boundary.csv");
        let mut count = 0;
        for row in rows.lines().skip(1) {
            let c = row
                .split(',')
                .map(|v| v.parse::<u32>().unwrap())
                .collect::<Vec<_>>();
            let mut target_checks = 0;
            let mut sight_checks = 0;
            let (decision, _) = decide(
                c[0] as u8,
                || {
                    target_checks += 1;
                    match c[1] {
                        0 => None,
                        1 => Some(0),
                        2 => Some(-1),
                        3 => Some(1),
                        _ => unreachable!(),
                    }
                },
                || {
                    sight_checks += 1;
                    SightResult {
                        visible: c[2] != 0,
                        reject_index: 0,
                        rejected: false,
                        eye: None,
                        slopes: None,
                        visits: Vec::new(),
                    }
                },
            );
            // Derive the expected branch from the original body's observed
            // dependency order, not a second handwritten threshold predicate.
            let expected = match c[7] {
                12 => RefireDecision::EarlyContinue,
                124 if c[1] == 0 => RefireDecision::NoTarget,
                124 => RefireDecision::DeadTarget,
                1234 => RefireDecision::NotVisible,
                123 => RefireDecision::VisibleContinue,
                _ => panic!("unexpected original dependency chronology"),
            };
            assert_eq!(decision, expected, "controlled C row {row}");
            assert_eq!(target_checks, u32::from(c[7] != 12));
            assert_eq!(sight_checks, c[5]);
            assert_eq!(u32::from(decision.exits()), c[6]);
            assert_eq!(&c[3..5], &[1, 1]);
            count += 1;
        }
        assert_eq!(count, 2048);
    }
}
