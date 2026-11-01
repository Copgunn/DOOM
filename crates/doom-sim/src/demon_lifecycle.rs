//! Explicit normal-timing Demon fixture definitions and bounded lifecycle admission.
use crate::*;
impl ActorRegistry {
    /// Labelled Demon lifecycle fixture, not full original Demon/Spectre admission.
    /// Source Demon fields are copied, excluding scoring and resurrection; the
    /// stand/run loops and state aliases are explicit fixture adaptations.
    pub fn demon_lifecycle_fixture() -> Self {
        let source = Self::vanilla();
        let mut states = source.states().to_vec();
        for (id, name, tics, action) in [
            (1, "FIXTURE_SARG_LOOK", 10, "A_Look"),
            (2, "FIXTURE_SARG_CHASE", 2, "A_Chase"),
        ] {
            states[id] = ActorStateDef {
                name: name.into(),
                sprite: 39,
                frame: 0,
                tics,
                action: ActorAction::Original(lifecycle::source_action_id(action)),
                next: StateId(id as u16),
                misc: [0; 2],
            };
        }
        let aliases = [
            (4, "S_SARG_ATK1", 5),
            (5, "S_SARG_ATK2", 6),
            (6, "S_SARG_ATK3", 2),
            (7, "S_SARG_PAIN", 8),
            (8, "S_SARG_PAIN2", 2),
            (9, "S_SARG_DIE1", 10),
            (10, "S_SARG_DIE2", 11),
            (11, "S_SARG_DIE3", 12),
            (12, "S_SARG_DIE4", 13),
            (13, "S_SARG_DIE5", 14),
            (14, "S_SARG_DIE6", 0),
        ];
        for (id, name, next) in aliases {
            let mut state = source
                .states()
                .iter()
                .find(|s| s.name == name)
                .unwrap()
                .clone();
            state.name = format!("FIXTURE_{}", &name[2..]);
            state.next = StateId(next);
            states[id] = state;
        }
        let mut types = source.types().to_vec();
        let mut owner = types[12].clone();
        owner.name = "FIXTURE_DEMON_LIFECYCLE_136".into();
        for (key, value) in [
            (0, -1),
            (1, 1),
            (3, 2),
            (7, 7),
            (10, 4),
            (12, 9),
            (13, 0),
            (21, 6),
            (22, 0),
        ] {
            owner.properties[key] = value;
        }
        types[136] = owner;
        let mut target = types[0].clone();
        target.name = "FIXTURE_DAMAGE_TARGET".into();
        for (key, value) in [(0, -1), (2, 1000), (8, 0), (16, 16 * 65536), (21, 6)] {
            target.properties[key] = value;
        }
        types[135] = target;
        Self::new(states, types).expect("source-derived fixture definitions")
    }
}

impl Simulation {
    /// Profile 14: explicit normal SARG melee, pain, ordinary death and corpses.
    /// No player damage, scoring, resurrection or complete monster admission.
    pub fn with_demon_lifecycle_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        Self::with_demon_lifecycle_profile(map, limit, registry, options, Profile::DemonLifecycle)
    }
    pub(crate) fn with_demon_lifecycle_profile(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
        profile: Profile,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(PerceptionError::WorkLimit.into());
        }
        if options.skill == Skill::Nightmare {
            return Err(MonsterError::Definition.into());
        }
        if map.linedefs().iter().any(|l| l.special != 0)
            || map.sectors().iter().any(|s| s.special != 0)
        {
            return Err(PursuitError::Specials.into());
        }
        crate::sight::validate_map(&map)?;
        let expected = if profile.uses_map_statistics() {
            ActorRegistry::demon_map_context_fixture()
        } else if profile.uses_demon_stand_run() {
            ActorRegistry::demon_stand_run_fixture()
        } else {
            ActorRegistry::demon_lifecycle_fixture()
        };
        if registry.states() != expected.states() || registry.types() != expected.types() {
            return Err(MonsterError::Definition.into());
        }
        let mut sim = Self::with_combat_registry(map, limit, registry, options)?;
        sim.profile = profile;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    fn type_admitted(&self, ty: ActorTypeId) -> bool {
        let bits = self.admitted_types.get();
        let n = usize::from(ty.0);
        bits.get(n / 64)
            .is_some_and(|w| w & (1_u64 << (n % 64)) != 0)
    }
    fn remember_admitted(&self, ty: ActorTypeId) {
        let mut bits = self.admitted_types.get();
        let n = usize::from(ty.0);
        if let Some(w) = bits.get_mut(n / 64) {
            *w |= 1_u64 << (n % 64);
            self.admitted_types.set(bits);
        }
    }
    pub(crate) fn validate_demon_lifecycle_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        if self.profile == Profile::OriginalDemon {
            // Every input below is immutable for the life of this simulation: the
            // shared registry, the static sectors, the policy and the identity
            // tables. A type that passed once passes again, so remember it.
            if self.type_admitted(ty) {
                return Ok(());
            }
            original_registry::require_role_for(self.player_policy(), ty)
                .map_err(|_| MonsterError::Definition)?;
            if self.player_policy().has_world_pickups()
                && world_pickups::kind_for(self.player_policy(), ty).is_some()
            {
                let registry = self.registry.as_deref().ok_or(MonsterError::Definition)?;
                let original = identity::OriginalActorType::try_from(ty.0)
                    .map_err(|_| MonsterError::Definition)?;
                original
                    .resolve(registry)
                    .map_err(|_| MonsterError::Definition)?;
                original
                    .states()
                    .spawn
                    .resolve(registry)
                    .map_err(|_| MonsterError::Definition)?;
                world_pickups::validate_definition(ty)?;
                self.remember_admitted(ty);
                return Ok(());
            }
            self.registry
                .as_deref()
                .ok_or(MonsterError::Definition)?
                .validate_type_original(ty, self.dimension_sectors())?;
            self.remember_admitted(ty);
            return Ok(());
        }
        if ![37, 38, 135, 136].contains(&ty.0) {
            return Err(MonsterError::Definition.into());
        }
        self.registry
            .as_deref()
            .unwrap()
            .validate_type_demon_context(ty, &self.sectors, self.profile.uses_map_statistics())?;
        Ok(())
    }
    pub(crate) fn lifecycle_state_role(
        &self,
        ty: ActorTypeId,
        state: StateId,
    ) -> Result<(), SimError> {
        if self.profile == Profile::OriginalDemon {
            return original_registry::state_role_for(self.player_policy(), ty, state)
                .map_err(|_| MonsterError::Definition.into());
        }
        let valid = match ty.0 {
            136 if self.profile == Profile::DemonExpandedMap => {
                NativeDemonState::checked(state).is_some()
            }
            136 if self.profile.uses_demon_stand_run() => (1..=21).contains(&state.0),
            136 => [1, 2].contains(&state.0) || (4..=14).contains(&state.0),
            135 => (149..=173).contains(&state.0),
            37 | 38 => (90..=96).contains(&state.0),
            _ => false,
        };
        if !valid {
            return Err(MonsterError::Definition.into());
        }
        Ok(())
    }
    pub(crate) fn lifecycle_owner(&self, a: Actor) -> Result<(), SimError> {
        let l = a.life.unwrap();
        let state = l.state.ok_or(MonsterError::Definition)?;
        let (death_state, before_fall) = if self.profile == Profile::OriginalDemon {
            original_registry::death_phase(l.actor_type, state)
                .map_err(|_| MonsterError::Definition)?
        } else if self.profile == Profile::DemonExpandedMap {
            let native = NativeDemonState::checked(state).ok_or(MonsterError::Definition)?;
            (native.is_death(), native.before_fall())
        } else {
            ((9..=14).contains(&state.0), state.0 < 12)
        };
        let dead = l.health <= 0;
        if self.profile.uses_map_statistics() && l.flags & 4194304 == 0
            || dead != death_state
            || (l.flags & 1048576 != 0) != dead
            || (l.flags & 4 != 0) == dead
            || a.height() != Fixed::from_map_units(if dead { 14 } else { 56 })
            || dead && ((l.flags & 2 != 0) != before_fall || l.flags & 1024 == 0)
        {
            return Err(MonsterError::Definition.into());
        }
        Ok(())
    }
    pub(crate) fn preflight_lifecycle_combat(
        &self,
        ids: &[Option<ActorId>],
    ) -> Result<(), SimError> {
        self.preflight_monster_owner()?;
        for id in ids.iter().flatten() {
            self.monster_reference(*id)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "demon_lifecycle_tests.rs"]
mod tests;
