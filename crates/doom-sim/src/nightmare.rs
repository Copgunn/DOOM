//! Wire 29 (`PlayerMonstersEpisode1V7`, DSIM45): skill 4 and the `-respawn` and
//! `-fast` parameters (M6b rulings N1, N4 (a), N6 (a) and N10 of 2026-11-26;
//! docs/M6B_REFERENCE_INVESTIGATION.md §1 to §3).
//!
//! The parameters are canonical (DSIM45 `actors.parameters`); what the C derives
//! from them and the skill is derived here, never saved: `respawnmonsters`
//! (g_game.c:1416-1419) and the fast table, which under ruling N4 (a) is that of
//! a fresh process: fast iff the skill is Nightmare or `fastparm` is set
//! (g_game.c:1421-1428; the histories of earlier games are quirk-register rows).
//!
//! **Temporary refusals** ([`Wire29Pending`], M6b phase 2 step 1). The rules of
//! Nightmare and the parameters that are not built yet refuse the tic, with the
//! state before it kept, wherever wire 28's rules would otherwise run on: each
//! variant names the step that replaces it with the original's behaviour.
use crate::identity::OriginalState;
use crate::{Simulation, Skill, StaticWorld};
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
    /// A state of the fast table entered while it is fast (g_game.c:1423-1424:
    /// `S_SARG_RUN1` to `S_SARG_PAIN2`, info.c:613-625). Step 3.
    FastStates,
    /// A missile of the fast table spawned while it is fast (g_game.c:1425-1427:
    /// `MT_BRUISERSHOT`, `MT_HEADSHOT`, `MT_TROOPSHOT` at 20 units). Step 3.
    FastMissile,
    /// `A_Chase` with `fastparm` below Nightmare where it differs from the skill
    /// test alone: after an attack (p_enemy.c:719) or before a missile with
    /// `movecount` left (p_enemy.c:738-739). Step 3.
    FastChase,
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

/// Wire 29: `fastparm` below Nightmare, where `A_Chase`'s two tests
/// (p_enemy.c:719, 738-739) differ from wire 28's test of the skill alone.
pub(crate) fn chase_fastparm(world: &StaticWorld, skill: Skill) -> bool {
    world
        .parameters
        .is_some_and(|p| p.fast && skill != Skill::Nightmare)
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

/// The longest a state may last, of the table in force: the thirteen halved
/// when `fast` (g_game.c:1423-1424, `tics >>= 1`), every other state as defined.
pub(crate) fn state_tics(state: OriginalState, fast: bool) -> i32 {
    let tics = state.descriptor().tics;
    if fast && FAST_STATES.contains(&state.ordinal()) {
        tics >> 1
    } else {
        tics
    }
}
