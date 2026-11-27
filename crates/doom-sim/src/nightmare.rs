//! Wire 29 (`PlayerMonstersEpisode1V7`, DSIM45): skill 4 and the `-respawn` and
//! `-fast` parameters (M6b rulings N1, N4 (a), N6 (a) and N10 of 2026-11-26;
//! docs/M6B_REFERENCE_INVESTIGATION.md §1 to §3).
//!
//! The parameters are canonical (DSIM45 `actors.parameters`); what the C derives
//! from them and the skill is derived here, never saved: `respawnmonsters`
//! (g_game.c:1416-1419) and the fast table, which under ruling N4 (a) is that of
//! a fresh process: fast iff the skill is Nightmare or `fastparm` is set
//! (g_game.c:1421-1428; the histories of earlier games are quirk-register rows).
//! The registry's records stay pristine at every wire: the table is applied where
//! the C reads the records it edits, a state's duration on entry
//! ([`table_tics`]) and a missile's speed at launch ([`missile_speed`]).
//!
//! The corpse respawn ([`corpse_think`], [`respawn`]) is `P_MobjThinker`'s
//! `tics == -1` branch and `P_NightmareRespawn` (p_mobj.c:356-409, 450-471;
//! docs/M6B_REFERENCE_INVESTIGATION.md §1.6).
use crate::identity::{OriginalActorType, OriginalState};
use crate::lifecycle::{ActorAccess, LifeHost};
use crate::{ActorId, ActorTypeId, SimError, Simulation, Skill, SpawnHeight, StaticWorld};
use doom_types::Fixed;
use std::ops::RangeInclusive;

/// The `-respawn` and `-fast` parameters (`respawnparm`, `fastparm`;
/// d_main.c:809-810; demo header bytes 5 and 6, g_game.c:1560-1561, 1600-1601),
/// beside the skill (ruling N1). Wire 29 alone; constant for the level.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GameParameters {
    /// `respawnparm`: corpses respawn on any skill (g_game.c:1416).
    pub respawn: bool,
    /// `fastparm`: the fast table (g_game.c:1421) and both `A_Chase` rules
    /// (p_enemy.c:719, 738-739) on any skill.
    pub fast: bool,
}

/// A rule of Nightmare or the parameters changed as the altered genuine engine
/// of the same name changes the C (`tools/reference/m6b_goldens.py` `VARIANTS`),
/// for the planted-fault tests of `nightmare_genuine_tests`. Only a test can
/// plant one ([`with_fault`]); outside `cfg(test)` [`planted`] is always false.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Fault {
    /// `bit`: Nightmare's THING bit `1<<(gameskill-1)`, 8 (p_mobj.c:746).
    Bit,
    /// `nodouble`: Nightmare does not double `P_GiveAmmo` (p_inter.c:96).
    NoDouble,
    /// `react`: the type's reaction time on Nightmare too (p_mobj.c:503), which
    /// the spawn witnesses then expect.
    React,
    /// `fast_states`: the thirteen durations not halved (g_game.c:1424).
    FastStates,
    /// `fast_speeds`: the Baron's and the imp's missiles at 15 and 10 units
    /// (g_game.c:1425, 1427).
    FastSpeeds,
    /// `chase_dir`: `P_NewChaseDir` after an attack on every skill (p_enemy.c:719).
    ChaseDir,
    /// `chase_count`: the `movecount` test before a missile on every skill
    /// (p_enemy.c:738-739).
    ChaseCount,
    /// `respawn_flag`: `respawnmonsters` never set (g_game.c:1416).
    RespawnFlag,
    /// `timer`: the corpse timer 13 seconds (p_mobj.c:461).
    Timer,
    /// `clock`: the gate every 64 tics (p_mobj.c:464).
    Clock,
    /// `gate`: the gate passes for 0 to 3 only (p_mobj.c:467).
    Gate,
    /// `nocheck`: no `P_CheckPosition` at the spawn point (p_mobj.c:370).
    NoCheck,
    /// `fog_spot`: the first fog at the spawn point (p_mobj.c:375-376).
    FogSpot,
    /// `react18`: the respawned monster's reaction time left as spawned
    /// (p_mobj.c:405).
    React18,
    /// `ambush`: the ambush flag not inherited (p_mobj.c:403).
    Ambush,
    /// `count`: a respawned monster counted in `totalkills` (p_mobj.c:399).
    Count,
}
#[cfg(test)]
thread_local! {
    static FAULT: std::cell::Cell<Option<Fault>> = const { std::cell::Cell::new(None) };
}
/// Whether a test planted `fault` on this thread (never outside `cfg(test)`).
pub(crate) fn planted(fault: Fault) -> bool {
    #[cfg(test)]
    {
        FAULT.with(|f| f.get() == Some(fault))
    }
    #[cfg(not(test))]
    {
        let _ = fault;
        false
    }
}
/// Runs `f` with `fault` planted on this thread.
#[cfg(test)]
pub(crate) fn with_fault<R>(fault: Fault, f: impl FnOnce() -> R) -> R {
    FAULT.with(|c| c.set(Some(fault)));
    let result = f();
    FAULT.with(|c| c.set(None));
    result
}

/// The thirteen states the fast table halves (g_game.c:1423-1424).
pub(crate) const FAST_STATES: RangeInclusive<u16> =
    OriginalState::S_SARG_RUN1.ordinal()..=OriginalState::S_SARG_PAIN2.ordinal();

/// Wire 29: the fast table is in force (ruling N4 (a)); false under every
/// other policy.
pub(crate) fn fast(world: &StaticWorld, skill: Skill) -> bool {
    world
        .parameters
        .is_some_and(|p| p.fast || skill == Skill::Nightmare)
}

/// Wire 29: `fastparm`, which `A_Chase`'s two tests name beside the skill
/// (p_enemy.c:719, 738-739); false under every other policy.
pub(crate) fn fastparm(world: &StaticWorld) -> bool {
    world.parameters.is_some_and(|p| p.fast)
}

/// A missile's speed in the table in force: `MT_BRUISERSHOT`, `MT_HEADSHOT` and
/// `MT_TROOPSHOT` move 20 units a tic when it is fast (g_game.c:1425-1427),
/// every type its defined `speed` otherwise.
pub(crate) fn missile_speed(world: &StaticWorld, skill: Skill, ty: ActorTypeId, speed: i32) -> i32 {
    use crate::identity::OriginalActorType as T;
    let fast_type = [T::MT_BRUISERSHOT, T::MT_HEADSHOT, T::MT_TROOPSHOT]
        .iter()
        .any(|t| t.ordinal() == ty.0);
    if fast_type
        && fast(world, skill)
        && !(planted(Fault::FastSpeeds) && ty.0 != T::MT_HEADSHOT.ordinal())
    {
        20 << 16
    } else {
        speed
    }
}

/// Wire 29: `respawnmonsters` (g_game.c:1416-1419); false under every other
/// policy.
pub(crate) fn respawn_monsters(world: &StaticWorld, skill: Skill) -> bool {
    world
        .parameters
        .is_some_and(|p| p.respawn || skill == Skill::Nightmare)
        && !planted(Fault::RespawnFlag)
}

/// `P_MobjThinker` of an actor whose `tics` is -1 (p_mobj.c:450-471): a
/// `MF_COUNTKILL` corpse counts its `movecount` on while `respawnmonsters` is
/// set, and from 12 seconds on (the count is not reset at death: what the chase
/// left, p_enemy.c:356, 764) draws its gate on every tic whose `leveltime` is a
/// multiple of 32; a draw of 4 or less calls [`respawn`]. Every other actor
/// returns, which is all wires 1 to 28 do.
pub(crate) fn corpse_think<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, leveltime: u64) {
    let life = h.access.get(id).and_then(|a| a.life).expect("typed actor");
    if life.flags & crate::world_pickups::MF_COUNTKILL == 0
        || !respawn_monsters(h.world, h.options.skill)
    {
        return;
    }
    // `mobj->movecount++` on an `int`: 2^31 tics away, refused rather than wrapped.
    let Some(count) = life.movecount.checked_add(1) else {
        h.fail_sight(crate::MapScaleError::Arithmetic.into());
        return;
    };
    let l = h.access.get_mut(id).and_then(|a| a.life.as_mut());
    l.expect("typed actor").movecount = count;
    let wait = if planted(Fault::Timer) {
        13 * 35
    } else {
        12 * 35
    };
    let clock = if planted(Fault::Clock) { 63 } else { 31 };
    let pass = if planted(Fault::Gate) { 3 } else { 4 };
    if count < wait || leveltime & clock != 0 || h.draw(id) > pass {
        return;
    }
    respawn(h, id);
}

/// `P_NightmareRespawn (corpse)` (p_mobj.c:356-409). `P_CheckPosition` is made
/// for the corpse as it is (its radius, 0 for gibs, and flags) at the spawn
/// point and moves nothing; with the corpse as `tmthing` (no `MF_PICKUP`, not a
/// missile) it only fails on a solid thing or a blocking line, and fills
/// `spechit`, whose ninth entry is refused as everywhere. On success: a fog with
/// `sfx_telept` at the corpse on its sector's floor, a second at the spawn point,
/// the monster at the spawn point (no fit test: the C has none), and the corpse
/// removed; the three spawns are at the tail and think in this tic.
pub(crate) fn respawn<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    use crate::multisector::MoveFailure;
    let corpse = *h.access.get(id).expect("typed actor");
    let life = corpse.life.expect("typed actor");
    // A map-placed monster always has its THING (p_mobj.c:771), a respawned one
    // its corpse's (399).
    let (Some(thing), Some(location)) = (life.spawnpoint, corpse.location) else {
        h.fail_sight(crate::PlayerBodyError::Invariant.into());
        return;
    };
    let spot = [thing.position.x, thing.position.y].map(Fixed::from_map_units);
    if !planted(Fault::NoCheck) {
        let things = {
            let access = &h.access;
            crate::movement::check_things(h.world, id, &corpse, spot, h.spatial, &|i| {
                access.get(i).copied()
            })
        };
        let attempt = crate::multisector::check_move_from_things(
            (h.world, h.sectors),
            &corpse,
            spot,
            things,
            false,
            false,
        );
        if let Err(e) = crate::line_triggers::spechit(h.sectors, &attempt, &[]) {
            h.fail_sight(e.into());
            return;
        }
        if matches!(attempt.failure, MoveFailure::Actor | MoveFailure::Line) {
            return;
        }
    }
    let reserved = h.access.reserve(3).map_err(SimError::from).and_then(|()| {
        h.spatial
            .order
            .try_reserve(3)
            .map_err(|_| crate::StoreError::Capacity.into())
    });
    if let Err(e) = reserved {
        h.fail_sight(e);
        return;
    }
    let fog_type = ActorTypeId(OriginalActorType::MT_TFOG.ordinal());
    let telept = crate::lifecycle::source_sound_id("sfx_telept");
    let floor = h.sectors[usize::from(location.sector)].floor;
    let first = if planted(Fault::FogSpot) {
        spot
    } else {
        corpse.position()
    };
    let fog = h.spawn(fog_type, first, SpawnHeight::Absolute(floor));
    h.sound(Some(fog), telept);
    let fog = h.spawn(fog_type, spot, SpawnHeight::Floor);
    h.sound(Some(fog), telept);
    let ceiling = h
        .registry
        .actor_type(life.actor_type)
        .expect("admitted type")
        .flags()
        & 256;
    let height = if ceiling != 0 {
        SpawnHeight::Ceiling
    } else {
        SpawnHeight::Floor
    };
    let mo = h.spawn(life.actor_type, spot, height);
    let a = h.access.get_mut(mo).expect("new typed actor");
    let l = a.life.as_mut().expect("typed actor");
    l.spawnpoint = Some(thing);
    // p_mobj.c:400 `ANG45 * (mthing->angle/45)`, modulo 2^32 as the strict build
    // computes it (the sanitizer builds' declared adapter, m6b_build.py).
    a.spec.angle = ((i32::from(thing.angle) / 45) as u32).wrapping_mul(0x2000_0000);
    if thing.options & 8 != 0 && !planted(Fault::Ambush) {
        l.flags |= 32;
    }
    if !planted(Fault::React18) {
        l.reaction_time = 18;
    }
    if planted(Fault::Count) {
        if let Some(stats) = h.statistics.as_deref_mut() {
            stats.total_kills += 1;
        }
    }
    h.mark(id);
}

/// `P_SpawnMobj`'s reaction time (p_mobj.c:503-504): the type's `defined` value,
/// left 0 on Nightmare (`nightmare`: wire 29 at skill 4).
pub(crate) fn spawn_reaction_time(nightmare: bool, defined: i32) -> i32 {
    if nightmare && !planted(Fault::React) {
        0
    } else {
        defined
    }
}

impl Simulation {
    /// Wire 29 at skill 4 (`PlayerRuntime::nightmare`).
    pub(crate) fn nightmare(&self) -> bool {
        self.original_player.as_ref().is_some_and(|p| p.nightmare)
    }
}

/// A state's duration in the table in force: `tics`, its defined value, halved
/// for the thirteen when `fast` (g_game.c:1423-1424, `tics >>= 1`).
pub(crate) fn table_tics(state: u16, tics: i32, fast: bool) -> i32 {
    if fast && FAST_STATES.contains(&state) && !planted(Fault::FastStates) {
        tics >> 1
    } else {
        tics
    }
}

/// The longest a state may last, of the table in force ([`table_tics`]).
pub(crate) fn state_tics(state: OriginalState, fast: bool) -> i32 {
    table_tics(state.ordinal(), state.descriptor().tics, fast)
}
