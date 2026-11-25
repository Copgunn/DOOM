//! Mutation-survivor tests for `chain_melt.rs`, from rules in its doc comments: a buffer is owed
//! while a display is pending after a tic that had one due; none outside a melt.
mod common;
use common::{directory, texture_count};
use doom_sim::ActorId;
use doom_sound::chain::{ChainConfig, ChainSound};
use doom_sound::chain_melt::{ChainMode, ChainSaveRefusal};
use doom_sound::logic::{Listener, OriginPositions};
use doom_sound::lumps::{SoundCaps, SoundSet};
use doom_sound::music::MusicLumps;
use doom_sound::schedule::Schedule;
use std::sync::Arc;

struct Nowhere;
impl OriginPositions for Nowhere {
    fn actor_xy(&self, _: ActorId) -> Option<[i32; 2]> {
        None
    }
    fn sector_xy(&self, _: u16) -> Option<[i32; 2]> {
        None
    }
    fn button_xy(&self, _: u8) -> Option<[i32; 2]> {
        None
    }
}

/// Kills `ChainSaveRefusal::fmt` -> `Ok(Default::default())` (the text names the refusal).
#[test]
fn the_save_refusal_displays_its_name() {
    let text = ChainSaveRefusal::PlantedFault.to_string();
    assert_eq!(text, "sound save refused: PlantedFault");
}

/// Kills `ChainSoundStateV3::level_tics` -> 0 and -> 1 (7 level tics run, every tic pending),
/// and `ChainSound::buffer_owed` -> true: after each tic the buffer is owed exactly when that
/// tic had one due (the schedule's count moved), and both cases occur in the 7 tics.
#[test]
fn level_tics_and_the_owed_buffer_follow_the_tics_run() {
    let wads = [
        "m6a_wads/episode.wad",
        "m5b_gfx.wad",
        "m5c_sfx.wad",
        "m6a_wads/m6a_gfx.wad",
        "m6a_wads/m6a_music.wad",
    ];
    let dir = directory(&wads);
    let set = Arc::new(SoundSet::load(&dir, SoundCaps::default()).expect("sound set"));
    let config = ChainConfig::fresh(texture_count(&dir));
    let music = MusicLumps::from_directory(&dir);
    let mut c = ChainSound::new_with(set, music, config, 1, ChainMode::FinaleAndMelt).expect("new");
    let at = Listener {
        actor: ActorId::from_parts(1, 0),
        xy: [0, 0],
        angle: 0,
    };
    let mut seen = [false; 2];
    for tic in 1..=7u64 {
        c.level_tic(None, &[], 0, &at, &Nowhere).expect("tic");
        let owed = Schedule::buffers_before(tic) != Schedule::buffers_before(tic - 1);
        assert_eq!(c.buffer_owed(), owed, "tic {tic}");
        seen[usize::from(owed)] = true;
        assert_eq!(c.save_v3().expect("v3").level_tics(), tic);
        c.mix_after_display().expect("display done");
        assert!(!c.buffer_owed());
    }
    assert_eq!(seen, [true, true]);
}
