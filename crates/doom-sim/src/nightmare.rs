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
//! **Temporary refusals** ([`Wire29Pending`], M6b phase 2 step 1). The rules of
//! Nightmare and the parameters that are not built yet refuse the tic, with the
//! state before it kept, wherever wire 28's rules would otherwise run on: each
//! variant names the step that replaces it with the original's behaviour.
use crate::identity::OriginalState;
use crate::{ActorTypeId, Simulation, Skill, StaticWorld};
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

/// A wire-29 rule that is not built yet (temporary, M6b phase 2 step 1): the
/// tic is refused with the state before it kept, where wire 28's rule would
/// differ from the original's.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Wire29Pending {
    /// `P_MobjThinker`'s `tics == -1` branch of a `MF_COUNTKILL` corpse while
    /// `respawnmonsters` is set (p_mobj.c:450-471). Step 4.
    Respawn,
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
