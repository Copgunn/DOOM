//! `DSND` version 4 (M6b wire 29, the pause session's state): its reader on a hand-built state
//! (mutation survivors of step 9, `ChainSoundStateV4::decode`). Rust-only tests of the format's
//! own rules (`chain_pause.rs` module documentation: nine bytes at offset 67; "Never panics";
//! every other version refused); the state is built by hand, so it needs no WAD. That every
//! state a session saves decodes and restores is `doom-game`'s `wire29_frames.rs`.

use doom_sim::ActorId;
use doom_sound::chain::{ChainConfig, ChainState};
use doom_sound::chain_pause::{ChainSoundStateV4, PauseFields};
use doom_sound::chain_state::{ChainSoundState, ChainStateError};
use doom_sound::detector::LayoutDetector;
use doom_sound::logic::{ChannelEntry, SoundOrigin};
use doom_sound::mixer::NUM_CHANNELS;
use doom_sound::schedule::Schedule;
use doom_sound::sounds::{NUMSFX, SfxId};
use doom_sound::state::StateError;

const TICS: u64 = 51;
const TEXTURES: u32 = 125;

/// A level state after 51 tics with one occupied channel and 20 paused tics.
fn base() -> ChainSoundStateV4 {
    let config = ChainConfig::fresh(TEXTURES);
    let sfx = SfxId::new(1).expect("sound 1");
    let mut looked_up = [false; NUMSFX];
    looked_up[sfx.index()] = true;
    let mut channels = vec![None; config.channels];
    channels[0] = Some(ChannelEntry {
        sfx,
        origin: SoundOrigin::Actor(ActorId::from_parts(3, 1)),
        handle: 100,
    });
    ChainSoundStateV4 {
        state: ChainSoundState {
            config,
            tics: TICS,
            buffers: Schedule::buffers_before(TICS),
            state: ChainState::Level,
            gamemap: 1,
            level_start: 0,
            level_end: 0,
            melt: false,
            detector: LayoutDetector::new(TEXTURES).expect("detector"),
            looked_up,
            channels,
            handlenums: 101,
            mixer: [None; NUM_CHANNELS],
        },
        pause: PauseFields {
            mus_paused: true,
            level_paused: 20,
        },
    }
}

/// The state round-trips, and every truncation of its bytes is refused, never a panic: the
/// lengths just below the fixed part (offset 67) and its nine added bytes (76) are the ones a
/// wrong length test would let through to the slicing. A byte string shorter than 76 is
/// `Truncated`.
#[test]
fn version_4_round_trips_and_every_truncation_is_refused() {
    let s = base();
    let bytes = s.encode();
    assert_eq!(ChainSoundStateV4::decode(&bytes).as_ref(), Ok(&s));
    for n in 0..bytes.len() {
        let r = ChainSoundStateV4::decode(&bytes[..n]);
        assert!(r.is_err(), "{n} bytes");
        if n < 76 {
            assert_eq!(
                r,
                Err(ChainStateError::State(StateError::Truncated)),
                "{n} bytes"
            );
        }
    }
}

/// `mus_paused` is 0 or 1 and the level's paused tics at most the level's tics.
#[test]
fn version_4_refuses_a_forged_flag_and_too_many_paused_tics() {
    let bytes = base().encode();
    let mut b = bytes.clone();
    b[67] = 2;
    assert_eq!(
        ChainSoundStateV4::decode(&b),
        Err(ChainStateError::Pause("mus_paused"))
    );
    let mut b = bytes;
    b[68..76].copy_from_slice(&(TICS + 1).to_le_bytes());
    assert_eq!(
        ChainSoundStateV4::decode(&b),
        Err(ChainStateError::Pause("level paused tics"))
    );
    b[68..76].copy_from_slice(&TICS.to_le_bytes());
    assert!(
        ChainSoundStateV4::decode(&b).is_ok(),
        "all of a level's tics paused is a state"
    );
}
