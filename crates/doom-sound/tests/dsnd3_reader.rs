//! `DSND` version 3 (M6a part 2, the finale-and-melt sound session): its reader on hand-built
//! states. Rust-only tests of the format's own rules; the states are built by hand, so they need
//! no WAD. That every state a session saves decodes and restores is `m6a_chain_finale_melt.rs`.
//!
//! Version 3 has version 2's layout and rules (docs/STATE_FORMAT.md); what it changes is that a
//! pending display (offset 49, a melt possibly in progress: the last tic's buffer not yet mixed)
//! restores. Checked here: every truncation, a trailing byte and an oversize input refused; every
//! out-of-domain value of the header refused; the buffer count of a pending display; versions 2
//! and 3 refusing each other's bytes; the rules of states no session reaches.

use doom_sim::ActorId;
use doom_sound::chain::{ChainConfig, ChainState};
use doom_sound::chain_melt::{ChainSoundStateV3, VERSION_FINALE_MELT};
use doom_sound::chain_state::{ChainSoundState, ChainStateError, MAX_BYTES};
use doom_sound::detector::LayoutDetector;
use doom_sound::logic::{ChannelEntry, SoundOrigin};
use doom_sound::mixer::{MixChannelState, NUM_CHANNELS};
use doom_sound::schedule::{MAX_TIC, Schedule};
use doom_sound::sounds::{NUMSFX, SfxId};
use doom_sound::state::StateError;
use doom_sound::tables::STEPTABLE;

const TICS: u64 = 51;
const TEXTURES: u32 = 125;

/// A level state after 51 tics with one occupied channel (sound 1 at an actor, handle 100) and
/// one busy mixer channel of the same sound (as `dsnd2_reader.rs`'s base).
fn base() -> ChainSoundStateV3 {
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
    ChainSoundStateV3 {
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
            mixer,
        },
    }
}

/// The base with a pending display after tic `TICS - 1`.
fn pending() -> ChainSoundStateV3 {
    let mut s = base();
    s.state.melt = true;
    s.state.buffers = Schedule::buffers_before(TICS - 1);
    s
}

fn read(s: &ChainSoundStateV3) -> Result<ChainSoundStateV3, ChainStateError> {
    ChainSoundStateV3::decode(&s.encode())
}

fn forged(base: &[u8], at: usize, v: &[u8]) -> Result<ChainSoundStateV3, ChainStateError> {
    let mut b = base.to_vec();
    b[at..at + v.len()].copy_from_slice(v);
    ChainSoundStateV3::decode(&b)
}

/// Both bases decode to themselves, as version 3 with version 2's bytes after the version field;
/// versions 2 and 3 refuse each other; every truncation, a trailing byte and an oversize input
/// are refused, never a panic.
#[test]
fn version_3_round_trips_and_refuses_version_2() {
    for b in [base(), pending()] {
        let bytes = b.encode();
        assert_eq!(read(&b).as_ref(), Ok(&b));
        assert_eq!(&bytes[4..6], &VERSION_FINALE_MELT.to_le_bytes());
        let v2 = b.state.encode();
        assert_eq!(bytes[..4], v2[..4]);
        assert_eq!(bytes[6..], v2[6..]);
        assert_eq!(
            ChainSoundStateV3::decode(&v2),
            Err(ChainStateError::State(StateError::Version(2)))
        );
        assert_eq!(
            ChainSoundState::decode(&bytes),
            Err(ChainStateError::State(StateError::Version(3)))
        );
        for n in 0..bytes.len() {
            assert!(
                ChainSoundStateV3::decode(&bytes[..n]).is_err(),
                "prefix {n}"
            );
        }
        let mut long = bytes.clone();
        long.push(0);
        assert_eq!(
            ChainSoundStateV3::decode(&long),
            Err(ChainStateError::State(StateError::Trailing))
        );
    }
    assert_eq!(
        ChainSoundStateV3::decode(&vec![0; MAX_BYTES + 1]),
        Err(ChainStateError::State(StateError::TooLong))
    );
    let mut bad = base().encode();
    bad[0] = b'X';
    assert_eq!(
        ChainSoundStateV3::decode(&bad),
        Err(ChainStateError::State(StateError::Magic))
    );
    for v in [0u16, 1, 4, u16::MAX] {
        assert_eq!(
            forged(&base().encode(), 4, &v.to_le_bytes()),
            Err(ChainStateError::State(StateError::Version(v)))
        );
    }
}

/// Out-of-domain header values: the volume, the channel count, the texture count, the game
/// state, the map (a finale on a map other than 8), the level start and end, the melt byte, the
/// buffer count with and without a pending display, the tic limit.
#[test]
fn version_3_refuses_out_of_domain_values() {
    let b = base().encode();
    let p = pending().encode();
    assert_eq!(
        forged(&b, 22, &[128]),
        Err(ChainStateError::State(StateError::Config))
    );
    assert_eq!(
        forged(&b, 23, &[65]),
        Err(ChainStateError::State(StateError::Config))
    );
    assert_eq!(
        forged(&b, 24, &(65_537u32).to_le_bytes()),
        Err(ChainStateError::State(StateError::Detector))
    );
    assert_eq!(forged(&b, 28, &[3]), Err(ChainStateError::GameState(3)));
    for map in [0i32, 10, -1, i32::MAX] {
        assert_eq!(
            forged(&b, 29, &map.to_le_bytes()),
            Err(ChainStateError::Map(map))
        );
    }
    // A finale needs map 8 and a level end; a finale on map 8 with level start < end < tics reads.
    let mut fin = base();
    fin.state.state = ChainState::Finale;
    fin.state.gamemap = 1;
    fin.state.level_end = 20;
    assert_eq!(read(&fin), Err(ChainStateError::Map(1)));
    fin.state.gamemap = 8;
    fin.state.channels[0] = None;
    fin.state.mixer[0] = None;
    assert!(read(&fin).is_ok());
    fin.state.melt = true;
    fin.state.buffers = Schedule::buffers_before(TICS - 1);
    assert!(read(&fin).is_ok(), "a pending display in the finale");
    // Level start and end against the game state and the tics.
    assert_eq!(
        forged(&b, 41, &5u64.to_le_bytes()),
        Err(ChainStateError::LevelTics)
    );
    assert_eq!(
        forged(&b, 33, &TICS.to_le_bytes()),
        Err(ChainStateError::LevelTics)
    );
    // The melt byte: 0 or 1, and 1 only after a tic.
    for m in [2u8, 0x80, 0xff] {
        assert_eq!(forged(&b, 49, &[m]), Err(ChainStateError::Melt(m)));
    }
    let mut zero = base();
    zero.state.tics = 0;
    zero.state.buffers = 0;
    zero.state.channels[0] = None;
    zero.state.mixer[0] = None;
    zero.state.handlenums = 0;
    assert!(read(&zero).is_ok());
    zero.state.melt = true;
    assert_eq!(read(&zero), Err(ChainStateError::Melt(1)));
    // The buffer count: all the tics' buffers without a pending display, those of the tics
    // before the last with one (the last tic's buffer owed). The base's 51 tics are chosen with
    // a buffer due after tic 50, so the two counts differ.
    assert_ne!(
        Schedule::buffers_before(TICS),
        Schedule::buffers_before(TICS - 1)
    );
    for (bytes, good) in [
        (&b, Schedule::buffers_before(TICS)),
        (&p, Schedule::buffers_before(TICS - 1)),
    ] {
        for wrong in [good - 1, good + 1, u64::MAX] {
            assert!(matches!(
                forged(bytes, 14, &wrong.to_le_bytes()),
                Err(ChainStateError::State(StateError::Schedule(_)))
            ));
        }
    }
    // The tic count past the limit `gametic` can count.
    let mut late = zero.clone();
    late.state.melt = false;
    late.state.tics = MAX_TIC + 2;
    late.state.buffers = Schedule::buffers_before(late.state.tics);
    late.state.level_start = 0;
    assert!(matches!(
        read(&late),
        Err(ChainStateError::State(StateError::Schedule(_)))
    ));
    late.state.tics = MAX_TIC + 1;
    late.state.buffers = Schedule::buffers_before(late.state.tics);
    assert!(read(&late).is_ok(), "the last tic `gametic` can count");
}

/// The rules of version 2's reader for states no session reaches hold in version 3, with or
/// without a pending display: an actor origin of generation 0, a channel whose handle the last
/// `S_UpdateSounds` freed (the update of a pending display's tic ran with `gametic` = the tics run),
/// a channel or mixer channel of a sound never looked up; the encoder writes a state with two
/// disagreeing counts as bytes the reader refuses.
#[test]
fn version_3_refuses_states_no_session_reaches() {
    for make in [base, pending] {
        let mut s = make();
        s.state.channels[0].as_mut().unwrap().origin =
            SoundOrigin::Actor(ActorId::from_parts(3, 0));
        assert_eq!(
            read(&s),
            Err(ChainStateError::Unreachable(
                "an actor origin of generation 0"
            ))
        );
        // Handle 100 after 100 tics: freed by that tic's update (`gametic >= handle`,
        // s_sound.c:560, i_sound.c:517-521).
        let mut s = make();
        s.state.tics = 100;
        s.state.buffers = Schedule::buffers_before(if s.state.melt { 99 } else { 100 });
        assert_eq!(
            read(&s),
            Err(ChainStateError::Unreachable(
                "a channel with an expired handle"
            ))
        );
        let mut s = make();
        s.state.mixer[0] = None;
        s.state.looked_up = [false; NUMSFX];
        assert_eq!(
            read(&s),
            Err(ChainStateError::Unreachable(
                "a channel whose sound was never looked up"
            ))
        );
        let mut s = make();
        s.state.config.channels += 1;
        assert_eq!(s.encode()[23], u8::MAX);
        assert_eq!(read(&s), Err(ChainStateError::State(StateError::Config)));
        let mut s = make();
        s.state.detector = LayoutDetector::new(TEXTURES - 1).expect("detector");
        assert_eq!(s.encode()[24..28], u32::MAX.to_le_bytes());
        assert_eq!(read(&s), Err(ChainStateError::State(StateError::Detector)));
    }
}
