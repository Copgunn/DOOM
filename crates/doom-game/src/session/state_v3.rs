//! `DSES` version 3: the saved state of a session of the wire 28 configuration
//! ([`super::finale_melt::Configuration::FinaleAndMeltWire28`], decision D2 of 2026-11-25,
//! recorded for the owner).
//!
//! **Versions 1 and 2 keep their bytes and rules** ([`super::state`], [`super::state_v2`]):
//! their readers refuse version 3 (`Version(3)`), this reader refuses both (`Version(1)`,
//! `Version(2)`), and each configuration saves only its own version
//! ([`DsesError::Configuration`] for another). Version 3 is version 2's layout field for field
//! ([`super::state_v2`]'s table), with three differences: the version is 3; the skill byte may be
//! 0 to 3 (wire 28 admits skills 0 and 1, ruling U12 (iii)); and the embedded level is a `DSIM`
//! version 44 capture (wire 28), read by `Diagnostic::decode_episode1_v6`, so a version 42 or 43
//! capture is refused. The embedded display and sound are `DDSP` version 4 and `DSND` version 3,
//! unchanged: wire 28 changes nothing they hold.
use super::Session;
use super::SessionData;
use super::state::DsesError;
use super::state_v2::{Layout, SessionStateV2};
use doom_sim::Simulation;
use std::sync::Arc;

/// The version written and read here.
pub const VERSION: u16 = 3;

/// A decoded `DSES` version 3 state: version 2's fields ([`SessionStateV2`]) under version 3's
/// rules (module documentation); its `level` is a DSIM44 capture.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionStateV3(pub SessionStateV2);

impl SessionStateV3 {
    /// The encoding: version 2's with the version 3.
    pub fn encode(&self) -> Vec<u8> {
        self.0.encode_as(Layout::V3)
    }

    /// Reads a version 3 state and checks it alone ([`Self::check`]), with version 2's bounds:
    /// every length bounded before anything is allocated, every field in its domain. Never
    /// panics.
    pub fn decode(bytes: &[u8]) -> Result<Self, DsesError> {
        SessionStateV2::decode_as(bytes, Layout::V3).map(Self)
    }

    /// Version 2's rules ([`SessionStateV2::check`]) with skills 0 to 3 and the embedded level
    /// read as DSIM44.
    pub fn check(&self) -> Result<(), DsesError> {
        self.0.check_as(Layout::V3)
    }
}

impl Session {
    /// The saved state (`DSES` version 3) between two calls of a wire 28 session, in a melt too.
    /// Refused: in another configuration ([`DsesError::Configuration`]), and after a rule-2
    /// refusal or with a planted fault ([`DsesError::NotSaveable`]).
    pub fn save_v3(&self) -> Result<SessionStateV3, DsesError> {
        self.save_as(Layout::V3).map(SessionStateV3)
    }

    /// A wire 28 session rebuilt from `state` over `data` and `level` (the level's simulation at
    /// the state's tic, whose capture must be the embedded one): [`Session::restore_v2`]'s checks
    /// under version 3's rules. On an error nothing is built.
    pub fn restore_v3(
        data: Arc<SessionData>,
        state: &SessionStateV3,
        level: Simulation,
    ) -> Result<Self, DsesError> {
        Self::restore_as(data, &state.0, level, Layout::V3)
    }
}
