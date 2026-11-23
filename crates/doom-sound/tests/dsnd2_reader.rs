//! `DSND` version 2: what its reader refuses beyond version 1's shared checks, and the
//! encoder's rule for a hand-built state (M6a reader review, items 6 and 10). Rust-only tests of
//! the format's own rules; the states are built by hand, so they need no WAD. That every state a
//! session saves still decodes is `m6a_chain.rs` (every tic of four items round-trips).

use doom_sim::ActorId;
use doom_sound::chain::{ChainConfig, ChainState};
use doom_sound::chain_state::{ChainSoundState, ChainStateError};
use doom_sound::detector::LayoutDetector;
use doom_sound::logic::{ChannelEntry, SoundOrigin};
use doom_sound::mixer::{MixChannelState, NUM_CHANNELS};
use doom_sound::schedule::Schedule;
use doom_sound::sounds::{NUMSFX, SfxId};
use doom_sound::state::StateError;
use doom_sound::tables::STEPTABLE;

const TICS: u64 = 50;
const TEXTURES: u32 = 125;

/// A level state after 50 tics with one occupied channel (sound 1 at an actor, handle 100, so
/// still playing: 50 < 100) and one busy mixer channel of the same sound.
fn base() -> ChainSoundState {
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
    let mut mixer = [None; NUM_CHANNELS];
    mixer[0] = Some(MixChannelState {
        sfx,
        pos: 0,
        end: 512,
        step: STEPTABLE[128],
        frac: 0,
        start: 49,
        handle: 100,
        left: 100,
        right: 100,
    });
    ChainSoundState {
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
        mixer,
    }
}

fn read(s: &ChainSoundState) -> Result<ChainSoundState, ChainStateError> {
    ChainSoundState::decode(&s.encode())
}

fn unreachable(s: &ChainSoundState) -> &'static str {
    match read(s) {
        Err(ChainStateError::Unreachable(rule)) => rule,
        other => panic!("not refused as unreachable: {other:?}"),
    }
}

/// Item 10: an actor origin of generation 0, a labelled detector with no texture, an occupied
/// channel whose handle the last `S_UpdateSounds` would have freed, and a channel (logic or
/// mixer) playing a sound never looked up. Each differs from the accepted base in that one thing.
#[test]
fn version_2_refuses_states_no_session_reaches() {
    let b = base();
    assert_eq!(read(&b).as_ref(), Ok(&b), "the base decodes to itself");

    let mut s = base();
    s.channels[0].as_mut().expect("entry").origin = SoundOrigin::Actor(ActorId::from_parts(3, 0));
    assert_eq!(unreachable(&s), "an actor origin of generation 0");

    // Labelled with no texture: no candidate position exists for the detector to fire on.
    let mut s = base();
    s.config.textures = 0;
    s.detector = LayoutDetector::from_parts(0, Some(10), 1).expect("parts");
    assert_eq!(unreachable(&s), "a labelled detector with no texture");
    let mut s = base();
    s.detector = LayoutDetector::from_parts(TEXTURES, Some(10), 1).expect("parts");
    assert!(read(&s).is_ok(), "labelled with textures");
    let mut s = base();
    s.config.textures = 0;
    s.detector = LayoutDetector::new(0).expect("detector");
    assert!(read(&s).is_ok(), "no texture and no label");

    // `gametic >= handle` frees the channel in the update that ends the tic (s_sound.c:560): a
    // handle of at most the tics run cannot be in the table; one above them can.
    for (tics, ok) in [(99, true), (100, false), (101, false)] {
        let mut s = base();
        s.tics = tics;
        s.buffers = Schedule::buffers_before(tics);
        match read(&s) {
            Ok(_) => assert!(ok, "handle 100 after {tics} tics accepted"),
            Err(e) => {
                assert!(!ok, "handle 100 after {tics} tics: {e:?}");
                assert_eq!(
                    e,
                    ChainStateError::Unreachable("a channel with an expired handle")
                );
            }
        }
    }

    let mut s = base();
    s.mixer[0] = None;
    s.looked_up = [false; NUMSFX];
    assert_eq!(unreachable(&s), "a channel whose sound was never looked up");
    let mut s = base();
    s.channels[0] = None;
    s.looked_up = [false; NUMSFX];
    assert_eq!(
        unreachable(&s),
        "a mixer channel whose sound was never looked up"
    );
}

/// Item 6: the struct names the channel count and the texture count twice. A hand-built state
/// in which they disagree encodes to bytes the reader refuses, never to the state of one of the
/// two counts; a consistent state's bytes are what they were.
#[test]
fn a_state_with_two_counts_does_not_encode_to_another_state() {
    let b = base();
    let bytes = b.encode();
    assert_eq!(bytes[23], b.config.channels as u8);
    assert_eq!(bytes[24..28], TEXTURES.to_le_bytes());

    let mut s = base();
    s.config.channels += 1;
    assert_eq!(s.encode()[23], u8::MAX);
    assert_eq!(read(&s), Err(ChainStateError::State(StateError::Config)));
    let mut s = base();
    s.channels.pop();
    assert_eq!(read(&s), Err(ChainStateError::State(StateError::Config)));

    let mut s = base();
    s.detector = LayoutDetector::new(TEXTURES - 1).expect("detector");
    assert_eq!(s.encode()[24..28], u32::MAX.to_le_bytes());
    assert_eq!(read(&s), Err(ChainStateError::State(StateError::Detector)));
    let mut s = base();
    s.config.textures += 1;
    assert_eq!(read(&s), Err(ChainStateError::State(StateError::Detector)));
}
