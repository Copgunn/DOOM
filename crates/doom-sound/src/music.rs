//! The music lookups of the Episode 1 path, as presence checks (M6a ruling U10 (i); M6a note
//! §1.7).
//!
//! linuxdoom-1.10 plays no music that any game code reads back (`I_RegisterSong` and
//! `I_PlaySong` do nothing the game observes, i_sound.c:841-881), but `S_ChangeMusic`
//! (s_sound.c:649-686) looks the lump `d_<name>` up with `W_GetNumForName` the first time a music
//! is changed to while another is playing, and that lookup ends in `I_Error` when the lump is
//! missing (s_sound.c:671-676, w_wad.c:399-409). A music already playing is never looked up again
//! (s_sound.c:665-666), and one found once keeps its lump number (`music->lumpnum`, s_sound.c:671);
//! since the WADs never change in a session, a lookup fails exactly when the lump is missing.
//!
//! The changes on the path: the level's music in `S_Start` at every load (s_sound.c:235-245,
//! `mus_e1m1 + (gameepisode-1)*9 + gamemap-1` for episode 1), the intermission's on its first
//! tic (`WI_Ticker` at `bcnt == 1`, wi_stuff.c:1508-1515) and the finale's in `F_StartFinale`
//! (f_finale.c:114). This module only says which of the eleven lumps the loaded WADs hold; the
//! session ([`crate::chain`]) refuses at the moment the genuine engine would stop. Music itself is
//! a later milestone, with no reference in linuxdoom-1.10.

use doom_render::LumpDirectory;

/// One music of the Episode 1 path (`musicenum_t`, sounds.h:100-133; names, sounds.c:38-70).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Music {
    /// `mus_e1m1` + map − 1 (`D_E1M1` ... `D_E1M9`): the level's music for map 1..=9 (made by
    /// [`Music::level`]; another number names no lump of the path and is never present).
    Level(u8),
    /// `mus_inter` (`D_INTER`).
    Inter,
    /// `mus_victor` (`D_VICTOR`).
    Victor,
}

impl Music {
    /// The level music of Episode 1 map `map` (s_sound.c:236-237 with `gameepisode` 1), or `None`
    /// outside 1..=9.
    pub fn level(map: i32) -> Option<Self> {
        u8::try_from(map)
            .ok()
            .filter(|m| (1..=9).contains(m))
            .map(Self::Level)
    }

    /// The lump name `W_GetNumForName` is given: `"d_" + name` (s_sound.c:673-674).
    pub fn lump_name(self) -> String {
        match self {
            Self::Level(m) => format!("d_e1m{m}"),
            Self::Inter => "d_inter".to_string(),
            Self::Victor => "d_victor".to_string(),
        }
    }

    /// 0..=8 for the nine level musics, 9 for `D_INTER`, 10 for `D_VICTOR`; `None` for a level
    /// number outside 1..=9 (which [`Self::level`] never makes).
    fn slot(self) -> Option<usize> {
        match self {
            Self::Level(m @ 1..=9) => Some(usize::from(m - 1)),
            Self::Level(_) => None,
            Self::Inter => Some(9),
            Self::Victor => Some(10),
        }
    }
}

impl std::fmt::Display for Music {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.lump_name().to_ascii_uppercase())
    }
}

/// Which of the eleven music lumps of the Episode 1 path the loaded WADs hold.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct MusicLumps {
    present: [bool; 11],
}

impl MusicLumps {
    /// The lookups of `W_CheckNumForName` (upper-cased, last match wins; w_wad.c) over `dir`.
    pub fn from_directory(dir: &LumpDirectory) -> Self {
        let mut present = [false; 11];
        for (i, p) in present.iter_mut().enumerate() {
            let m = match i {
                9 => Music::Inter,
                10 => Music::Victor,
                _ => Music::Level(i as u8 + 1),
            };
            *p = dir.check_num_for_name(m.lump_name().as_bytes()).is_some();
        }
        Self { present }
    }

    /// Every lump present (every IWAD of Episode 1 holds all eleven).
    pub fn all() -> Self {
        Self {
            present: [true; 11],
        }
    }

    /// The same set without `music` (test and evidence use: the variant WADs of M6a's refusal
    /// witnesses each lack one lump).
    pub fn without(mut self, music: Music) -> Self {
        if let Some(i) = music.slot() {
            self.present[i] = false;
        }
        self
    }

    /// Whether `music`'s lump is present (false for a level number outside 1..=9).
    pub fn present(&self, music: Music) -> bool {
        music.slot().is_some_and(|i| self.present[i])
    }
}
