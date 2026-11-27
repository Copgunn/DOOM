//! The session's sound across levels: doom-sound's [`ChainSound`] (M6a step 6, `DSND` version
//! 2) driven by the session in the C's order (M6a note §1.1, §1.6, §4.6; the order of the calls
//! is [`ChainSound`]'s module documentation). This module only adapts the simulation to it:
//!
//! - **a load tic** (`G_DoWorldDone` or `G_DoReborn`'s `G_DoLoadLevel` → `P_SetupLevel` →
//!   `S_Start`, p_setup.c:607): wire 26 records the load-time start (a carried chainsaw's
//!   `sfx_sawup`, p_pspr.c:145-146) both in its start group and at the head of tic 0's records
//!   (doom-sim's `load_sound_records`, DSIM42), so the head is handed to [`ChainSound`] as the
//!   load's records and the rest as the tic's; a head that is not the start group's records is a
//!   broken seam and refused;
//! - **a level tic**: the simulation's records, its listener and its origins;
//! - **an intermission tic**: the starts [`super::intermission`] requested, all with no origin,
//!   heard against the finished level's listener and origins (it stays valid memory until the
//!   next load's `Z_FreeTags`, p_setup.c:618; U8 quirk 9). Each start draws `M_Random` exactly once
//!   (s_sound.c:327-346), which is checked.
//!
//! An earlier interim of this module ran the M5c session directly and kept its state as `DSND`
//! version 1 bytes; it was replaced by [`ChainSound`] when step 6 was merged, so `DSES` v1 embeds
//! `DSND` version 2 only.
use doom_sim::Simulation;
use doom_sound::chain::{ChainRefusal, ChainSound, ChainTic, IntermissionSound, LevelLoad};
use doom_sound::session::{SimulationPositions, simulation_listener};
use std::fmt;

pub use doom_sound::music::{Music, MusicLumps};

/// Why the session's sound refused. Nothing changed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SoundWitness {
    /// doom-sound's session refused (a missing music lump included: the genuine engine's
    /// `W_GetNumForName` stop, s_sound.c:671-676, w_wad.c:399-409; U10 (i)).
    Chain(ChainRefusal),
    /// The intermission starts did not draw one `M_Random` value each: a broken seam, refused.
    Draws {
        /// The starts.
        starts: usize,
        /// The draws the sound session made.
        draws: u16,
    },
    /// The level (or the finished level) gives no listener (no player body).
    NoListener,
    /// Tic 0's records do not begin with the load's records (a broken seam with wire 26).
    LoadRecords,
}

impl fmt::Display for SoundWitness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chain(e) => write!(f, "{e}"),
            Self::Draws { starts, draws } => write!(
                f,
                "{starts} intermission sound starts made {draws} random draws"
            ),
            Self::NoListener => write!(f, "the level has no listener"),
            Self::LoadRecords => write!(f, "tic 0's records do not begin with the load's"),
        }
    }
}

impl From<ChainRefusal> for SoundWitness {
    fn from(e: ChainRefusal) -> Self {
        Self::Chain(e)
    }
}

/// Whether `S_ChangeMusic(target)` (s_sound.c:649-686) would stop the genuine engine now: it
/// looks nothing up when `target` is playing (665-666), and stops when the lump is missing.
pub fn music_lookup_fails(sound: &ChainSound, target: Music) -> bool {
    target != sound.music_playing() && !sound.music_lumps().present(target)
}

/// One level tic of `sim` (which has just completed it); with `load` the map loaded in this tic
/// (module documentation). With `owed` (wire 29) the tic is the level's tic 0 after a paused load
/// tic, whose records begin with the load's, given to the sound on that load tic. A paused tic
/// has no records (`Simulation::paused_tic`), a paused load tic only the load's. Transactional
/// (doom-sound's rule).
pub fn level_tic(
    sound: &mut ChainSound,
    load: Option<u8>,
    sim: &Simulation,
    rndindex: u8,
    owed: bool,
) -> Result<ChainTic, SoundWitness> {
    let listener = simulation_listener(sim).ok_or(SoundWitness::NoListener)?;
    let records = sim.sound_records();
    let head = sim.load_sound_records();
    let stripped = || {
        records
            .get(head.len()..)
            .filter(|_| records.starts_with(head))
    };
    let (load, tic_records) = match load {
        None if owed => (None, stripped().ok_or(SoundWitness::LoadRecords)?),
        None => (None, records),
        Some(map) => {
            // A paused load tic: the level's tic 0 has not run (its records are empty).
            let paused = sim.tic() == 0;
            let rest = if paused && records.is_empty() {
                records
            } else {
                stripped().ok_or(SoundWitness::LoadRecords)?
            };
            let load = LevelLoad {
                map: i32::from(map),
                records: head,
            };
            (Some(load), rest)
        }
    };
    Ok(sound.level_tic(
        load,
        tic_records,
        rndindex,
        &listener,
        &SimulationPositions(sim),
    )?)
}

/// One finale tic ([`ChainSound::finale_tic`]): on the first (from a level) the `D_VICTOR` lookup
/// of `F_StartFinale` (f_finale.c:114); no start (the Episode 1 finale starts no sound), heard
/// against the finished level `frozen`'s listener and origins. Transactional.
pub fn finale_tic(
    sound: &mut ChainSound,
    rndindex: u8,
    frozen: &Simulation,
) -> Result<ChainTic, SoundWitness> {
    let listener = simulation_listener(frozen).ok_or(SoundWitness::NoListener)?;
    Ok(sound.finale_tic(rndindex, &listener, &SimulationPositions(frozen))?)
}

/// One intermission tic: the starts in call order with the index before the first, and the
/// finished level `frozen` for the listener and the origins. Transactional.
pub fn intermission_tic(
    sound: &mut ChainSound,
    starts: &[IntermissionSound],
    rndindex: u8,
    frozen: &Simulation,
) -> Result<ChainTic, SoundWitness> {
    let listener = simulation_listener(frozen).ok_or(SoundWitness::NoListener)?;
    let mut next = sound.clone();
    let t = next.intermission_tic(starts, rndindex, &listener, &SimulationPositions(frozen))?;
    if usize::from(t.sound.draws) != starts.len() {
        return Err(SoundWitness::Draws {
            starts: starts.len(),
            draws: t.sound.draws,
        });
    }
    *sound = next;
    Ok(t)
}
