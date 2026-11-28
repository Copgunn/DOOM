//! `DSES` version 4: the saved state of a session of the wire 29 configuration
//! ([`super::finale_melt::Configuration::FinaleAndMeltWire29`], M6b rulings N1, N5, N6 and N10).
//!
//! **Versions 1 to 3 keep their bytes and rules** ([`super::state`], [`super::state_v2`],
//! [`super::state_v3`]): their readers refuse version 4 (`Version(4)`), this reader refuses
//! them, and each configuration saves only its own version ([`DsesError::Configuration`] for
//! another). Version 4 is version 2's layout ([`super::state_v2`]'s table) with these differences
//! and nothing else (offsets from the start, little endian):
//!
//! | Field | Version 3 | Version 4 |
//! | --- | --- | --- |
//! | version | u16 = 3 at 4 | u16 = 4 at 4 |
//! | skill | u8 at 9: 0..=3 | u8 at 9: 0..=4 |
//! | `respawnparm`, `fastparm`, `paused` | absent | 3 x u8 (0 or 1) at 11, 12, 13; every later field 3 bytes later |
//! | level | `DSIM` 44 | `DSIM` 45 (`Diagnostic::decode_episode1_v7`) |
//! | display | u32 length, `DDSP` 4 | u8 display kind = 5, then u32 length and `DDSP` 5 (a pause display's state) |
//! | sound | u8 kind = 3, `DSND` 3 | u8 kind = 4, then u32 length and `DSND` 4 (a pause session's state) |
//!
//! The two kind bytes are the slots of the embedded formats; this build writes and reads display
//! kind 5 and sound kind 4 only (any other is [`DsesError::DisplayKind`] or
//! [`DsesError::SoundKind`]). [`SessionStateV2`] holds the embedded states' `DDSP` 4 and `DSND` 3
//! parts, [`V4Fields`] the fields the two new versions add.
//!
//! Rules (besides version 3's, [`SessionStateV2::check`], with the display checked by `DDSP` 5's
//! rules): the parameters equal the level's `actors.parameters`; the level's paused tics,
//! `level.gametic_lead` - `level.start.gametic`, are session tics the simulation did not run, so
//! the capture's tic plus them equals the session tics of the level and they equal the sound's
//! paused tics (and in a level the display's); a paused level has `gameaction` 0 and at least one
//! paused tic; `paused` equals the display's and the sound's `mus_paused`.
use super::Session;
use super::SessionData;
use super::state::DsesError;
use super::state_v2::{Layout, SessionStateV2, V4Fields};
use doom_sim::Simulation;
use std::sync::Arc;

/// The version written and read here.
pub const VERSION: u16 = 4;
/// The display slot kind of `DDSP` version 5.
pub const DISPLAY_DDSP5: u8 = 5;
/// The sound slot kind of `DSND` version 4.
pub const SOUND_DSND4: u8 = 4;

/// A decoded `DSES` version 4 state: version 2's fields ([`SessionStateV2`], its `level` a DSIM45
/// capture) and the fields version 4 adds.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionStateV4 {
    /// Version 2's fields.
    pub base: SessionStateV2,
    /// The parameters and `paused`.
    pub v4: V4Fields,
}

impl SessionStateV4 {
    /// The encoding (module documentation).
    pub fn encode(&self) -> Vec<u8> {
        self.base.encode_with(Layout::V4, Some(self.v4))
    }

    /// Reads a version 4 state and checks it alone ([`Self::check`]), with version 2's bounds.
    /// Never panics.
    pub fn decode(bytes: &[u8]) -> Result<Self, DsesError> {
        let (base, v4) = SessionStateV2::decode_with(bytes, Layout::V4)?;
        let v4 = v4.ok_or(DsesError::Inconsistent("version 4 fields"))?;
        Ok(Self { base, v4 })
    }

    /// Version 3's rules with skills 0 to 4, the level read as DSIM45 and version 4's rules
    /// (module documentation).
    pub fn check(&self) -> Result<(), DsesError> {
        self.base.check_with(Layout::V4, Some(self.v4))
    }
}

impl Session {
    /// The saved state (`DSES` version 4) between two calls of a wire 29 session, in a melt and
    /// a pause too. Refused: in another configuration ([`DsesError::Configuration`]), and after a
    /// rule-2 refusal or with a planted fault ([`DsesError::NotSaveable`]).
    pub fn save_v4(&self) -> Result<SessionStateV4, DsesError> {
        let (base, pause) = self.save_parts(Layout::V4)?;
        let (display, sound) = pause.ok_or(DsesError::NotSaveable)?;
        Ok(SessionStateV4 {
            base,
            v4: V4Fields {
                parameters: self.parameters,
                paused: self.paused,
                display,
                sound,
            },
        })
    }

    /// A wire 29 session rebuilt from `state` over `data` and `level` (the level's simulation at
    /// the state's tic, whose capture must be the embedded one): [`Session::restore_v2`]'s checks
    /// under version 4's rules. On an error nothing is built.
    pub fn restore_v4(
        data: Arc<SessionData>,
        state: &SessionStateV4,
        level: Simulation,
    ) -> Result<Self, DsesError> {
        Self::restore_with(data, &state.base, level, Layout::V4, Some(state.v4))
    }
}
