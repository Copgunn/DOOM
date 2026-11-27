//! M6a part 2: the finale-and-melt configuration of the sound session across levels
//! ([`ChainMode::FinaleAndMelt`], owner rulings V1 and V3 of 2026-11-23 and the melt ruling of
//! 2026-11-21), saved as `DSND` version 3 ([`ChainSoundStateV3`]).
//!
//! Everything the frozen configuration ([`ChainMode::RefusesMelt`], part 1) does per tic is done
//! the same way here: the level, load, intermission and finale tics, their records, their
//! `S_UpdateSounds` and their music lookups, `D_VICTOR` included (`S_ChangeMusic(mus_victor)` at
//! `F_StartFinale`, f_finale.c:114; s_sound.c:649-686; a missing lump is the genuine stop at the
//! finale's first tic, `W_GetNumForName`, w_wad.c:399-409). The Episode 1 finale starts no sound
//! (`F_Ticker`, f_finale.c:207-249, and `F_StartFinale`'s episode-1 branch call no sound function
//! but the music change). What this configuration adds is **the melt**, in the C order of a pass
//! of `D_DoomLoop` (d_main.c:374-405):
//!
//! 1. the tic (`G_Ticker`: its sound records, d_main.c:383 or `TryRunTics`), then
//!    `S_UpdateSounds` (d_main.c:392): one [`ChainSound::level_tic`],
//!    [`ChainSound::intermission_tic`] or [`ChainSound::finale_tic`] call, which here **leaves the
//!    tic's buffer owed** (if one is due under the declared schedule, [`crate::schedule`]) and the
//!    tic's display pending;
//! 2. `D_Display` (d_main.c:395), and inside it, when the display wipes, the melt loop
//!    (d_main.c:320-344), which calls no sound function: no `S_UpdateSounds`, no `I_UpdateSound`,
//!    no `I_SubmitSound`, and no tic runs, so no sound starts (sounds start only inside tics). One
//!    [`ChainSound::melt_step`] per melt step (one `wipe_ScreenWipe` call with `ticks = 1`, note
//!    §1.16.1), each **accepted and changing nothing**: the channels of the sound logic, the
//!    mixer's channel positions and the schedule stand where the tic left them;
//! 3. `I_UpdateSound` (d_main.c:397-400, compiled without `SNDSERV`) and `I_SubmitSound`
//!    (d_main.c:402-405, without `SNDINTR`): [`ChainSound::mix_after_display`] mixes the owed
//!    buffer, after the melt's last picture. Nothing touched the mixer in between, so it is the
//!    buffer the frozen configuration mixes inside the tic: **the sequence of mixed buffers does
//!    not depend on the melt** (note §1.16.4 option (a), the reference build's; the owner's
//!    ruling of 2026-11-21: nothing is mixed or submitted while the melt runs, nothing skipped).
//!
//! The call order is enforced: a tic while a display is pending is
//! [`ChainRefusal::DisplayPending`], and a melt step or a mix with none pending is
//! [`ChainRefusal::NoDisplayPending`]; each refusal changes nothing.
//!
//! **What a melt in progress adds to the state: nothing but the pending display.** The melt's
//! pictures, its column positions and its step count are the display's (`DDSP` version 4); the
//! sound code is not called during it, so the only sound state between a tic and the end of its
//! display is that the tic's buffer, if one was due, is not yet mixed. That is the one byte that
//! `DSND` version 2 already laid out for a melt in progress (its offset 49), which version 2's
//! restore refuses; version 3 has the same layout and restores it.
//!
//! **An extra `S_UpdateSounds` would change no channel.** Run again with the same `gametic`,
//! listener and origins, it frees nothing more (a channel is playing while `gametic < handle`,
//! i_sound.c:517-521, and the first update already freed the rest, s_sound.c:553-607), and the
//! parameters it computes go nowhere (`I_UpdateSoundParams` does nothing in linuxdoom,
//! i_sound.c:675-688). Its only trace is the call itself, which the genuine runner counts (the
//! `updates` total of its sound stream's Z row): [`ChainSound::updates`] counts the session's.

use std::sync::Arc;

use doom_sim::Simulation;

use crate::chain::{ChainConfig, ChainRefusal, ChainSound};
use crate::chain_state::{ChainSoundState, ChainStateError};
use crate::lumps::SoundSet;
use crate::mixer::Buffer;
use crate::music::{Music, MusicLumps};

/// `DSND` version 3: the saved state of [`ChainMode::FinaleAndMelt`].
pub const VERSION_FINALE_MELT: u16 = 3;

/// Which configuration a sound session across levels runs (owner ruling V1 of 2026-11-23: the new
/// behaviour in a second configuration beside the frozen one).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChainMode {
    /// M6a part 1, frozen: every tic mixes its due buffer in the call (the melt takes no time,
    /// decision T6 as extended), a melt step is [`crate::chain::NotYet::MeltSteps`], and the
    /// saved state is `DSND` version 2, whose restore refuses a melt in progress
    /// ([`crate::chain::NotYet::MeltInProgress`]). What [`ChainSound::new`] builds.
    RefusesMelt,
    /// M6a part 2: the finale-and-melt session (module documentation): a tic leaves its buffer
    /// owed until its display is done, melt steps are accepted and change nothing, and the saved
    /// state is `DSND` version 3, which restores a pending display.
    FinaleAndMelt,
}

/// A fault planted by the evidence self-tests, to prove that the comparisons with the genuine
/// goldens fail when the session is wrong. Never set by the game; a session holding one cannot be
/// saved as version 3 ([`ChainSound::save_v3`] gives `None`).
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChainFault {
    /// The first melt step of a display mixes the owed buffer (during the melt, not after it).
    MixInMelt,
    /// Every melt step runs `S_UpdateSounds` again (module documentation: its only trace is the
    /// call).
    UpdateInMelt,
    /// The `D_VICTOR` lookup made on the finale's second tic instead of its first.
    VictorLate,
}

/// Why a checked save of a sound session across levels was refused (owner's ruling 3 of
/// 2026-11-23: each configuration saves only in its own version). Nothing is changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChainSaveRefusal {
    /// A [`ChainMode::FinaleAndMelt`] session is saved as `DSND` version 3 only.
    FinaleAndMeltSavesVersion3,
    /// A [`ChainMode::RefusesMelt`] session is saved as `DSND` version 2 only.
    RefusesMeltSavesVersion2,
    /// The session holds a planted fault (evidence self-tests only).
    PlantedFault,
    /// A pause session (`crate::chain_pause`) is saved as `DSND` version 4 only.
    PauseSavesVersion4,
    /// `DSND` version 4 is the save of a pause session only.
    NotPauseSession,
}

impl std::fmt::Display for ChainSaveRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sound save refused: {self:?}")
    }
}

impl std::error::Error for ChainSaveRefusal {}

/// The saved state of a finale-and-melt sound session, `DSND` version 3 (docs/STATE_FORMAT.md):
/// the layout, meaning and rules of version 2 ([`ChainSoundState`]) with the version 3, read by
/// its own reader. `state.melt` is a pending display: the logic and the `S_UpdateSounds` of the
/// last tic done, its buffer (if one was due) not yet mixed, a melt possibly in progress.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainSoundStateV3 {
    /// The contents, as version 2 names them.
    pub state: ChainSoundState,
}

impl ChainSoundStateV3 {
    /// The `DSND` version 3 bytes. As version 2's encoder, a state no session holds encodes to
    /// bytes the reader refuses, never to another state.
    pub fn encode(&self) -> Vec<u8> {
        self.state.encode_version(VERSION_FINALE_MELT)
    }

    /// Reads `DSND` version 3 bytes with every check of version 2's reader (another version,
    /// version 2 included, is `StateError::Version`). Never panics.
    pub fn decode(bytes: &[u8]) -> Result<Self, ChainStateError> {
        ChainSoundState::decode_version(bytes, VERSION_FINALE_MELT).map(|state| Self { state })
    }

    /// The music this state has playing.
    pub fn music_playing(&self) -> Music {
        self.state.music_playing()
    }

    /// The tics the level simulation has run in this state ([`ChainSoundState::level_tics`]).
    pub fn level_tics(&self) -> u64 {
        self.state.level_tics()
    }
}

impl ChainSound {
    /// A new game on map `map` in configuration `mode`: [`ChainMode::RefusesMelt`] is exactly
    /// [`Self::new`]; [`ChainMode::FinaleAndMelt`] runs the melt (module documentation).
    pub fn new_with(
        set: Arc<SoundSet>,
        music: MusicLumps,
        config: ChainConfig,
        map: i32,
        mode: ChainMode,
    ) -> Result<Self, ChainRefusal> {
        let mut s = Self::new(set, music, config, map)?;
        s.mode = mode;
        Ok(s)
    }

    /// The configuration.
    pub fn mode(&self) -> ChainMode {
        self.mode
    }

    /// Whether the last tic's display is pending ([`ChainMode::FinaleAndMelt`] only: between a
    /// tic and [`Self::mix_after_display`], a melt possibly in progress).
    pub fn display_pending(&self) -> bool {
        self.pending
    }

    /// Whether the pending display owes the last tic's buffer (one was due after it). The
    /// schedule ([`Self::schedule`]) already counts it.
    pub fn buffer_owed(&self) -> bool {
        self.owed
    }

    /// The buffers actually mixed: the schedule's count less an owed buffer.
    pub fn buffers_mixed(&self) -> u64 {
        self.sound.schedule.buffers() - u64::from(self.owed)
    }

    /// The `S_UpdateSounds` calls the session has made (d_main.c:392): one per tic, none in a
    /// melt step. Not saved: a restored session counts one per tic it has run.
    pub fn updates(&self) -> u64 {
        self.updates
    }

    /// [`Self::melt_step`] in [`ChainMode::FinaleAndMelt`].
    pub(crate) fn melt_step_finale_and_melt(&mut self) -> Result<(), ChainRefusal> {
        if !self.pending {
            return Err(ChainRefusal::NoDisplayPending);
        }
        match self.fault {
            Some(ChainFault::UpdateInMelt) => self.updates = self.updates.saturating_add(1),
            Some(ChainFault::MixInMelt) if self.owed => {
                self.sound.mix_owed();
                self.owed = false;
            }
            _ => {}
        }
        Ok(())
    }

    /// After the display of the last tic, melt included (`I_UpdateSound` and `I_SubmitSound`,
    /// d_main.c:397-405): its buffer, if one was due, mixed now, and the display no longer
    /// pending. [`ChainRefusal::NoDisplayPending`] when none is pending (always in
    /// [`ChainMode::RefusesMelt`], which mixes inside the tic), with nothing changed.
    pub fn mix_after_display(&mut self) -> Result<Option<Buffer>, ChainRefusal> {
        if !self.pending {
            return Err(ChainRefusal::NoDisplayPending);
        }
        let buffer = self.owed.then(|| self.sound.mix_owed());
        self.pending = false;
        self.owed = false;
        Ok(buffer)
    }

    /// Plants a fault for the evidence self-tests (or removes it with `None`).
    #[doc(hidden)]
    pub fn plant_fault(&mut self, fault: Option<ChainFault>) {
        self.fault = fault;
    }

    /// The saved state of a [`ChainMode::FinaleAndMelt`] session, `DSND` version 3; `None` for a
    /// [`ChainMode::RefusesMelt`] session (it saves as version 2, [`Self::save`]), for a
    /// session holding a planted fault and for a pause session (`DSND` version 4,
    /// [`Self::save_v4_checked`]).
    pub fn save_v3(&self) -> Option<ChainSoundStateV3> {
        (self.mode == ChainMode::FinaleAndMelt && self.fault.is_none() && self.pause.is_none())
            .then(|| ChainSoundStateV3 { state: self.save() })
    }

    /// The `DSND` version 2 state, refused by type for a session of
    /// [`ChainMode::FinaleAndMelt`] (owner's ruling 3 of 2026-11-23: a session of the new
    /// configuration is saved only in the new versions; an old-version save from it is a typed
    /// refusal). For a [`ChainMode::RefusesMelt`] session it is exactly [`Self::save`].
    /// [`Self::save`] itself keeps its behaviour for every configuration: existing tests pin
    /// what it gives on a finale-and-melt session (a pending display seen through version 2).
    pub fn save_v2_checked(&self) -> Result<ChainSoundState, ChainSaveRefusal> {
        match self.mode {
            ChainMode::RefusesMelt => Ok(self.save()),
            ChainMode::FinaleAndMelt => Err(ChainSaveRefusal::FinaleAndMeltSavesVersion3),
        }
    }

    /// [`Self::save_v3`] with the reason for a refusal named.
    pub fn save_v3_checked(&self) -> Result<ChainSoundStateV3, ChainSaveRefusal> {
        if self.pause.is_some() {
            return Err(ChainSaveRefusal::PauseSavesVersion4);
        }
        match (self.mode, self.fault) {
            (ChainMode::RefusesMelt, _) => Err(ChainSaveRefusal::RefusesMeltSavesVersion2),
            (ChainMode::FinaleAndMelt, Some(_)) => Err(ChainSaveRefusal::PlantedFault),
            (ChainMode::FinaleAndMelt, None) => Ok(ChainSoundStateV3 { state: self.save() }),
        }
    }

    /// A [`ChainMode::FinaleAndMelt`] session rebuilt from a version 3 state, with every check of
    /// [`Self::restore`] (configuration, music lumps, sound set, simulation tics and origins); a
    /// pending display is restored as pending, its buffer owed when the last tic had one due, so
    /// the session continues with the display's melt steps and [`Self::mix_after_display`].
    pub fn restore_v3(
        set: Arc<SoundSet>,
        music: MusicLumps,
        state: &ChainSoundStateV3,
        sim: &Simulation,
        expected: ChainConfig,
    ) -> Result<Self, ChainStateError> {
        Self::restore_mode(
            set,
            music,
            &state.state,
            sim,
            expected,
            ChainMode::FinaleAndMelt,
            0,
        )
    }
}
