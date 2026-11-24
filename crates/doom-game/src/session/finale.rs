//! The Doom 1 Episode 1 finale logic of f_finale.c (M6a note §1.8): what `F_StartFinale`
//! (f_finale.c:96-191) and `F_Ticker` (f_finale.c:207-249) do to `finalestage` and
//! `finalecount`.
//!
//! **Reached by no configuration.** The session still refuses the victory tic
//! ([`super::NotYet::Victory`]); these pure functions are what a later configuration will call,
//! proven against the genuine engine's committed session rows. The rest of `F_StartFinale`
//! belongs to the session when it is wired: `gameaction = ga_nothing`, `gamestate = GS_FINALE`,
//! `viewactive` and `automapactive` false (f_finale.c:98-101) and `S_ChangeMusic(mus_victor)`
//! (f_finale.c:114, a refusal when `D_VICTOR` is missing). `finaleflat` and `finaletext` are fixed
//! for Episode 1 (`FLOOR4_8` and `E1TEXT`, f_finale.c:118-121) and are the drawer's constants
//! ([`doom_render::f_finale::E1FLAT`], [`doom_render::f_finale::E1TEXT`]).
//!
//! The Doom 1 finale starts no sound and draws no random number (no `M_Random` or `P_Random`
//! call in f_finale.c's Doom 1 path; M6a note §1.8), reads no command (the skip test is Doom II
//! only, f_finale.c:212-227; `F_Responder` serves only Doom II's cast, f_finale.c:195-201) and
//! never ends by itself (quirk Q-f_finale-207).
use doom_render::f_finale::{E1TEXT, TEXTSPEED};

pub use doom_render::FinaleView;

/// `TEXTWAIT` (f_finale.c:57).
pub const TEXTWAIT: i32 = 250;

/// `strlen(finaletext) * TEXTSPEED + TEXTWAIT` for Episode 1 (f_finale.c:241): 440 * 3 + 250 =
/// 1570. Stage 1 begins on the `F_Ticker` call that makes `finalecount` exceed it, the 1,571st
/// finale tic counting the `G_Ticker` that ran `G_DoCompleted` as the first (f_finale.c:230, 241;
/// g_game.c:740-742).
pub const STAGE_THRESHOLD: i32 = E1TEXT.len() as i32 * TEXTSPEED + TEXTWAIT;

/// A refused finale tic. Nothing changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FinaleRefusal {
    /// `gameepisode` other than 1: episodes 2 to 4 are a later milestone, to be named by the
    /// owner.
    Episode(i32),
    /// `finalestage` other than 0 or 1 (stage 2 is Doom II's cast, f_finale.c:232-236).
    Stage(i32),
    /// `finalecount++` (f_finale.c:230) would overflow a 32-bit `int`, which the C leaves
    /// undefined (after about 707 days on the art screen).
    Count,
}

impl std::fmt::Display for FinaleRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "finale tic refused: {self:?}")
    }
}
impl std::error::Error for FinaleRefusal {}

/// What one `F_Ticker` call did beside the view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinaleTick {
    /// `wipegamestate = -1` (f_finale.c:245): the text gave way to the art screen on this tic,
    /// so this tic's display melts.
    pub wipe: bool,
}

/// `F_StartFinale` for a Doom 1 game mode (f_finale.c:96-191) as far as the finale state goes:
/// episode `episode` (only 1 is admitted), `finalestage` 0, `finalecount` 0 (f_finale.c:188-189).
pub fn start(episode: i32) -> Result<FinaleView, FinaleRefusal> {
    if episode != 1 {
        return Err(FinaleRefusal::Episode(episode));
    }
    Ok(FinaleView { stage: 0, count: 0 })
}

/// `F_Ticker` for Doom 1 Episode 1 (f_finale.c:207-249): `finalecount++`, then in stage 0, when
/// `finalecount > STAGE_THRESHOLD`, `finalecount = 0`, `finalestage = 1` and a forced wipe
/// (f_finale.c:241-248; the bunny music is episode 3's). On a refusal `v` is unchanged.
pub fn tick(v: &mut FinaleView) -> Result<FinaleTick, FinaleRefusal> {
    if !(0..=1).contains(&v.stage) {
        return Err(FinaleRefusal::Stage(v.stage));
    }
    let count = v.count.checked_add(1).ok_or(FinaleRefusal::Count)?;
    if v.stage == 0 && count > STAGE_THRESHOLD {
        *v = FinaleView { stage: 1, count: 0 };
        return Ok(FinaleTick { wipe: true });
    }
    v.count = count;
    Ok(FinaleTick { wipe: false })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// f_finale.c:230, 241-248: stage 1 on the 1,571st call, with the wipe; then the count runs
    /// on; refusals keep the view.
    #[test]
    fn stage_1_begins_on_the_1571st_tic() {
        assert_eq!(STAGE_THRESHOLD, 1570);
        let mut v = start(1).expect("episode 1");
        for k in 1..=1570 {
            assert_eq!(tick(&mut v), Ok(FinaleTick { wipe: false }));
            assert_eq!(v, FinaleView { stage: 0, count: k });
        }
        assert_eq!(tick(&mut v), Ok(FinaleTick { wipe: true }));
        assert_eq!(v, FinaleView { stage: 1, count: 0 });
        assert_eq!(tick(&mut v), Ok(FinaleTick { wipe: false }));
        assert_eq!(v, FinaleView { stage: 1, count: 1 });
        assert_eq!(start(2), Err(FinaleRefusal::Episode(2)));
        let mut v = FinaleView {
            stage: 1,
            count: i32::MAX,
        };
        assert_eq!(tick(&mut v), Err(FinaleRefusal::Count));
        assert_eq!(v.count, i32::MAX);
        let mut v = FinaleView { stage: 2, count: 0 };
        assert_eq!(tick(&mut v), Err(FinaleRefusal::Stage(2)));
        assert_eq!(v, FinaleView { stage: 2, count: 0 });
    }
}
