//! M6b wire 29: the original's pause in the sound session across levels (rulings N5 and N6;
//! docs/M6B_REFERENCE_INVESTIGATION.md §4.3), saved as `DSND` version 4 ([`ChainSoundStateV4`]).
//!
//! A pause session ([`ChainSound::new_pause`]) is the finale-and-melt session
//! ([`ChainMode::FinaleAndMelt`]) with what `G_Ticker`'s pause does to the sound code:
//!
//! - **Music only.** The toggle calls `S_PauseSound` or `S_ResumeSound` (g_game.c:704-709), which
//!   call `I_PauseSong` or `I_ResumeSong` under the `mus_paused` flag (s_sound.c:497-513), both
//!   empty in linuxdoom (i_sound.c:848-858). `I_RegisterSong` returns 1 (i_sound.c:875-881), so
//!   every call names handle 1. The port has no music: each call is a [`MusicCall`] of the tic
//!   ([`ChainSound::music_calls`]), and `mus_paused` is state. `S_Start` clears `mus_paused`
//!   without a call (s_sound.c:214), and the level's `G_DoLoadLevel` clears the game's `paused`
//!   without one (g_game.c:494); a music change while `mus_paused` is set calls `I_ResumeSong`
//!   once more and leaves the flag (`S_StopMusic`, s_sound.c:689-696), which the intermission's
//!   first `WI_Ticker` reaches after a pausing command on the completion tic (wi_stuff.c:1508-1515;
//!   the toggle comes before the game-state switch, g_game.c:698-748).
//! - **Effects play on.** `S_UpdateSounds` and the mixer run every frame with no test of
//!   `paused` (d_main.c:392-405): a paused tic is an ordinary tic of the sound code with no
//!   records (the simulation makes none), so channels play on and end during a pause, and nothing
//!   new starts in a paused level; the intermission's and the finale's sounds start as always
//!   (`WI_Ticker` and `F_Ticker` are not behind the pause, g_game.c:736-742).
//! - **Level tics.** A paused level tic is a session tic the level did not run (p_tick.c:135-136),
//!   so the level's tics are the session tics since its load less its paused tics, which the state
//!   counts (`DSND` version 3's rule, the session tics since the level's start, no longer holds).
//!
//! The session tells the sound what the tic's toggle did before the tic ([`ChainSound::pause_tic`]);
//! a tic without it is a tic with no toggle in an unpaused level.
//!
//! **`DSND` version 4** is version 3's layout ([`crate::chain_state`]) with the version 4 and nine
//! bytes inserted at offset 67, before the channel tail: u8 `mus_paused` (0 or 1) and u64 the
//! level's paused tics (at most the session tics of the level: in a level the tics run less the
//! level start, outside a level the level end less the level start). Version 3's reader refuses
//! version 4 (`Version(4)`) and this reader every other version.
use std::sync::Arc;

use doom_sim::Simulation;

use crate::chain::{ChainConfig, ChainRefusal, ChainSound};
use crate::chain_melt::{ChainMode, ChainSaveRefusal};
use crate::chain_state::{ChainSoundState, ChainStateError};
use crate::lumps::SoundSet;
use crate::music::MusicLumps;
use crate::state::StateError;

/// `DSND` version 4: the saved state of a pause session.
pub const VERSION_PAUSE: u16 = 4;
/// The song handle of every music call: `I_RegisterSong` returns 1 (i_sound.c:875-881).
pub const MUSIC_HANDLE: i32 = 1;
/// Where version 4's fields begin: the end of version 3's fixed part.
const AT: usize = 67;
/// Their length.
const LEN: usize = 9;

/// A call of the music interface the pause makes (each with [`MUSIC_HANDLE`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MusicCall {
    /// `I_PauseSong` (s_sound.c:501).
    Pause,
    /// `I_ResumeSong` (s_sound.c:510, or `S_StopMusic`'s, s_sound.c:692).
    Resume,
}

/// What `G_Ticker`'s pause did in a tic (g_game.c:704-709), given before the tic.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct TicPause {
    /// A command of the tic toggled `paused`.
    pub toggle: bool,
    /// `paused` after the toggle (in a level: the level's tic is not run).
    pub paused: bool,
}

/// The fields `DSND` version 4 adds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct PauseFields {
    /// `mus_paused` (s_sound.c:497-513).
    pub mus_paused: bool,
    /// The paused tics of the current (or, outside a level, the finished) level since its load.
    pub level_paused: u64,
}

/// The pause part of a pause session.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Pause {
    fields: PauseFields,
    next: TicPause,
    calls: Vec<MusicCall>,
}

/// Which tic a pause session commits (the C order of its music calls, module documentation).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PauseTic {
    /// A level tic; `load`: `S_Start` ran in it.
    Level { load: bool },
    /// The first intermission tic, whose `WI_Ticker` changes the music after the toggle.
    EnterIntermission { music: bool },
    /// The first finale tic, whose `F_StartFinale` changes the music before the toggle.
    EnterFinale { music: bool },
    /// Any other tic outside a level.
    Other,
}

impl Pause {
    fn toggle(&mut self) {
        let f = &mut self.fields;
        match (self.next.toggle, self.next.paused, f.mus_paused) {
            (true, true, false) => {
                self.calls.push(MusicCall::Pause);
                f.mus_paused = true;
            }
            (true, false, true) => {
                self.calls.push(MusicCall::Resume);
                f.mus_paused = false;
            }
            _ => {}
        }
    }

    fn stop_music(&mut self, changed: bool) {
        if changed && self.fields.mus_paused {
            self.calls.push(MusicCall::Resume);
        }
    }

    /// The music calls and the counters of a committed tic.
    pub(crate) fn commit(&mut self, tic: PauseTic) {
        self.calls.clear();
        match tic {
            PauseTic::Level { load } => {
                if load {
                    self.fields.mus_paused = false;
                    self.fields.level_paused = 0;
                }
                self.toggle();
                self.fields.level_paused += u64::from(self.next.paused);
            }
            PauseTic::EnterIntermission { music } => {
                self.toggle();
                self.stop_music(music);
            }
            PauseTic::EnterFinale { music } => {
                self.stop_music(music);
                self.toggle();
            }
            PauseTic::Other => self.toggle(),
        }
        self.next = TicPause::default();
    }
}

#[cfg(test)]
#[path = "chain_pause_tests.rs"]
mod tests;

/// The saved state of a pause session, `DSND` version 4 (module documentation).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainSoundStateV4 {
    /// The contents, as version 2 names them (`level_start` is the session tic of the level's
    /// load, whether its tic 0 ran there or was paused).
    pub state: ChainSoundState,
    /// The fields version 4 adds.
    pub pause: PauseFields,
}

impl ChainSoundStateV4 {
    /// The `DSND` version 4 bytes (as version 2's encoder, a state no session holds encodes to
    /// bytes the reader refuses).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = self.state.encode_version(VERSION_PAUSE);
        let mut extra = vec![u8::from(self.pause.mus_paused)];
        extra.extend_from_slice(&self.pause.level_paused.to_le_bytes());
        out.splice(AT..AT, extra);
        out
    }

    /// Reads `DSND` version 4 bytes with every check of version 2's reader and version 4's rules
    /// ([`ChainStateError::Pause`]). Never panics.
    pub fn decode(bytes: &[u8]) -> Result<Self, ChainStateError> {
        if bytes.len() < AT + LEN {
            return Err(StateError::Truncated.into());
        }
        let mut rest = bytes.to_vec();
        let extra: Vec<u8> = rest.drain(AT..AT + LEN).collect();
        let state = ChainSoundState::decode_version(&rest, VERSION_PAUSE)?;
        let mus_paused = match extra[0] {
            0 => false,
            1 => true,
            _ => return Err(ChainStateError::Pause("mus_paused")),
        };
        let level_paused = u64::from_le_bytes(extra[1..].try_into().expect("8 bytes"));
        let s = Self {
            state,
            pause: PauseFields {
                mus_paused,
                level_paused,
            },
        };
        if s.level_tics().is_none() {
            return Err(ChainStateError::Pause("level paused tics"));
        }
        Ok(s)
    }

    /// The tics the level simulation has run: [`ChainSoundState::level_tics`] less the level's
    /// paused tics (None when they exceed them).
    pub fn level_tics(&self) -> Option<u64> {
        self.state.level_tics().checked_sub(self.pause.level_paused)
    }
}

impl ChainSound {
    /// A new game on map `map` as a pause session: [`ChainMode::FinaleAndMelt`] with the pause
    /// (module documentation).
    pub fn new_pause(
        set: Arc<SoundSet>,
        music: MusicLumps,
        config: ChainConfig,
        map: i32,
    ) -> Result<Self, ChainRefusal> {
        let mut s = Self::new_with(set, music, config, map, ChainMode::FinaleAndMelt)?;
        s.pause = Some(Pause::default());
        Ok(s)
    }

    /// Whether this is a pause session.
    pub fn is_pause(&self) -> bool {
        self.pause.is_some()
    }

    /// What the next tic's toggle does (a pause session only; `None` takes it back, as after a
    /// refused tic). Refused with [`ChainRefusal::NotPause`] otherwise, with nothing changed.
    pub fn pause_tic(&mut self, next: Option<TicPause>) -> Result<(), ChainRefusal> {
        let p = self.pause.as_mut().ok_or(ChainRefusal::NotPause)?;
        p.next = next.unwrap_or_default();
        Ok(())
    }

    /// The music calls of the last tic, in order (empty outside a pause session).
    pub fn music_calls(&self) -> &[MusicCall] {
        self.pause.as_ref().map_or(&[], |p| &p.calls)
    }

    /// `mus_paused` and the level's paused tics (None outside a pause session).
    pub fn pause_fields(&self) -> Option<PauseFields> {
        self.pause.as_ref().map(|p| p.fields)
    }

    /// The pause part of a committed tic (nothing outside a pause session).
    pub(crate) fn commit_pause(&mut self, tic: PauseTic) {
        if let Some(p) = self.pause.as_mut() {
            p.commit(tic);
        }
    }

    /// The saved state of a pause session, `DSND` version 4, with the reason for a refusal named.
    pub fn save_v4_checked(&self) -> Result<ChainSoundStateV4, ChainSaveRefusal> {
        match (&self.pause, self.fault) {
            (None, _) => Err(ChainSaveRefusal::NotPauseSession),
            (Some(_), Some(_)) => Err(ChainSaveRefusal::PlantedFault),
            (Some(p), None) => Ok(ChainSoundStateV4 {
                state: self.save(),
                pause: p.fields,
            }),
        }
    }

    /// A pause session rebuilt from a version 4 state, with every check of
    /// [`ChainSound::restore_v3`] against the level's tics ([`ChainSoundStateV4::level_tics`]).
    pub fn restore_v4(
        set: Arc<SoundSet>,
        music: MusicLumps,
        state: &ChainSoundStateV4,
        sim: &Simulation,
        expected: ChainConfig,
    ) -> Result<Self, ChainStateError> {
        let mut s = Self::restore_mode(
            set,
            music,
            &state.state,
            sim,
            expected,
            ChainMode::FinaleAndMelt,
            state.pause.level_paused,
        )?;
        s.pause = Some(Pause {
            fields: state.pause,
            ..Pause::default()
        });
        Ok(s)
    }
}
