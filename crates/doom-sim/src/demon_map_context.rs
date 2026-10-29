//! Bounded source level totals and the unbound single-player death-credit branch.
use crate::*;

/// Admitted signed source counters. Available map kills and credited deaths are
/// distinct; items/secrets remain zero because their gameplay is unsupported.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LevelStatistics {
    /// Original `totalkills`, incremented by admitted map placements only.
    pub total_kills: i32,
    /// Original `totalitems`, initialized but no item placement is admitted.
    pub total_items: i32,
    /// Original `totalsecret`, initialized but specials are excluded.
    pub total_secrets: i32,
    /// Original `players[0].killcount` fallback for unbound/absent sources.
    /// This does not implement player-pointer attribution, points or frags.
    pub single_player_kill_count: i32,
}
/// Explicit game context at level construction. P_SetupLevel retains RNG state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MapContextOptions {
    /// True rejects: Nightmare respawning is outside this profile.
    pub respawn_monsters: bool,
    /// Existing owned streams; default starts a fresh game, restart retains them.
    pub initial_rng: DoomRng,
}
/// Source-ordered counter changes; diagnostic observations, not extra state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatisticsEvent {
    /// Map placement contributed one available kill after spawn timer RNG.
    MapKillTotal {
        /// New map actor.
        actor: ActorId,
        /// Updated available map kills.
        total: i32,
    },
    /// Unbound single-player death credit after corpse initialization.
    SinglePlayerKill {
        /// Killed actor.
        actor: ActorId,
        /// Updated fallback kill counter.
        total: i32,
    },
}
impl ActorRegistry {
    /// Profile17 preserves original Demon flags. Native identity/state aliases
    /// and disabled raise remain explicit adaptations, not full MT_SERGEANT.
    pub fn demon_map_context_fixture() -> Self {
        let fixture = Self::demon_stand_run_fixture();
        let mut types = fixture.types().to_vec();
        types[136].name = "FIXTURE_DEMON_MAP_CONTEXT_136".into();
        types[136].properties[21] = 4194310;
        Self::new(fixture.states().to_vec(), types).expect("source-derived context fixture")
    }
}
impl Simulation {
    /// Empty profile17. Call the existing atomic `spawn_map_things` before
    /// installing explicit unbound targets. No complete level loader or players.
    pub fn with_demon_map_context_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, SimError> {
        if context.respawn_monsters {
            return Err(MapSpawnError::Respawn.into());
        }
        if map.things().len() > MAX_DEMON_MAP_THINGS {
            return Err(MapSpawnError::WorkLimit.into());
        }
        let mut sim = Self::with_demon_lifecycle_profile(
            map,
            limit,
            registry,
            options,
            Profile::DemonMapContext,
        )?;
        sim.statistics = Some(LevelStatistics::default());
        sim.rng = context.initial_rng;
        Ok(sim)
    }
    /// Copy of the admitted counters; absent in historical profiles.
    pub const fn level_statistics(&self) -> Option<LevelStatistics> {
        self.statistics
    }
    /// Fresh bounded setup of the same map, retaining source RNG continuity.
    /// Returns a new instance; this instance is unchanged even on failure.
    /// Handles are scoped to their simulation. Targets/roster are reinstalled
    /// explicitly; complete player starts and game reload are unsupported.
    pub fn restart_demon_map_context(&self) -> Result<Self, SimError> {
        if self.profile != Profile::DemonMapContext {
            return Err(MapSpawnError::Context.into());
        }
        let mut fresh = Self::with_demon_map_context_registry(
            self.world.map().clone(),
            self.actors.max_slots,
            self.registry.as_deref().unwrap().clone(),
            self.actor_options,
            MapContextOptions {
                initial_rng: self.rng,
                respawn_monsters: false,
            },
        )?;
        fresh.spawn_map_things()?;
        Ok(fresh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> Simulation {
        let bytes = include_bytes!("../../../tests/reference/m3d3i_map_1.bin");
        let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
        let map = Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
        Simulation::with_demon_map_context_registry(
            map,
            64,
            ActorRegistry::demon_map_context_fixture(),
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .unwrap()
    }
    #[test]
    fn batch_and_death_statistics_are_distinct_and_corpse_is_persistent() {
        let mut s = empty();
        assert_eq!(s.level_statistics(), Some(LevelStatistics::default()));
        let ids = s.spawn_map_things().unwrap().spawned;
        assert_eq!(ids.len(), 2);
        assert_eq!(s.level_statistics().unwrap().total_kills, 2);
        let before = s.rng();
        for id in ids {
            assert_eq!(s.actor(id).unwrap().life().unwrap().flags, 4194310);
            s.damage_actor(DamageRequest {
                target: id,
                source: None,
                inflictor: None,
                amount: 150,
            })
            .unwrap();
        }
        assert_ne!(s.rng(), before);
        assert_eq!(s.level_statistics().unwrap().total_kills, 2);
        assert_eq!(s.level_statistics().unwrap().single_player_kill_count, 2);
        for _ in 0..1000 {
            s.step([TicCommand::default(); 4]).unwrap();
        }
        let rng = s.rng();
        let id = ActorId::from_parts(0, 1);
        let corpse = *s.actor(id).unwrap();
        for _ in 0..100 {
            s.step([TicCommand::default(); 4]).unwrap();
        }
        assert_eq!(s.rng(), rng);
        assert_eq!(*s.actor(id).unwrap(), corpse);
        s.damage_actor(DamageRequest {
            target: id,
            source: None,
            inflictor: None,
            amount: 150,
        })
        .unwrap();
        assert_eq!(s.level_statistics().unwrap().single_player_kill_count, 2);
    }
    #[test]
    fn both_statistics_overflows_reject_without_canonical_mutation() {
        let mut s = empty();
        s.statistics.as_mut().unwrap().total_kills = i32::MAX;
        let before = s.snapshot();
        assert_eq!(
            s.spawn_map_things(),
            Err(MapSpawnError::StatisticsOverflow.into())
        );
        assert_eq!(s.snapshot(), before);
        s.statistics = Some(LevelStatistics::default());
        let id = s.spawn_map_things().unwrap().spawned[0];
        s.statistics.as_mut().unwrap().single_player_kill_count = i32::MAX;
        let before = s.snapshot();
        assert_eq!(
            s.damage_actor(DamageRequest {
                target: id,
                source: None,
                inflictor: None,
                amount: 150
            }),
            Err(MapSpawnError::StatisticsOverflow.into())
        );
        assert_eq!(s.snapshot(), before);
    }
    #[test]
    fn reset_reuses_rng_and_constructor_without_changing_old_instance() {
        let mut s = empty();
        s.spawn_map_things().unwrap();
        s.gameplay_random();
        let old = s.snapshot();
        let seed = s.rng();
        let fresh = s.restart_demon_map_context().unwrap();
        assert_eq!(s.snapshot(), old);
        assert_eq!(fresh.tic(), 0);
        assert_eq!(fresh.level_statistics().unwrap().total_kills, 2);
        assert_eq!(
            fresh.level_statistics().unwrap().single_player_kill_count,
            0
        );
        let mut rng = seed;
        for _ in 0..4 {
            rng.p_random();
        }
        assert_eq!(fresh.rng(), rng);
    }
}
