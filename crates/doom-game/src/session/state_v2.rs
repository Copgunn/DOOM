//! `DSES` version 2: the saved state of a session of the finale-and-melt configuration
//! ([`super::finale_melt::Configuration::FinaleAndMelt`], owner's ruling V3 of 2026-11-23), for
//! restore-and-continue at any point between two calls: in a level, on its load tic, in the
//! intermission, in both finale stages, and in the middle of a melt.
//!
//! **Version 1 keeps its bytes and rules** ([`super::state`]): its reader refuses version 2
//! (`Version(2)`), this reader refuses version 1 (`Version(1)`), a frozen session saves only
//! version 1 and a finale-and-melt session only version 2 ([`DsesError::Configuration`] for the
//! other). Version 2 has version 1's layout field for field, with three differences: the version
//! is 2; the game state may be the finale, with the finale field set; and the embedded states are
//! `DSIM` version 43 (wire 27, where version 1 embeds version 42 of wire 26; redefined on
//! 2026-11-24 before version 2 was frozen, by the owner's ruling), `DDSP` version 4 and `DSND`
//! version 3 (sound kind 3). Little endian:
//!
//! | Field | Encoding | Meaning |
//! | --- | --- | --- |
//! | magic, version | 4 bytes `DSES`, u16 = 2 | |
//! | mode, mode source | u8 (0 registered, 1 retail), u8 (0 probed, 1 forced) | ruling U4 |
//! | start map, skill, screenblocks | u8, u8, u8 | 1..=9, 2..=3, 10..=11 |
//! | `gametic` | u64 | session tics run |
//! | `gamestate` | u8: 0 level, 1 intermission, 2 finale | doomdef.h:127-133 |
//! | `gamemap`, `gameaction` | u8 1..=9, u8 0, 6 or 8 | as version 1; 0 in the finale |
//! | `secretexit`, `didsecret` | u8, u8 | as version 1 |
//! | `wminfo`, wi_stuff.c statics | as version 1 | in a level and in the finale, the last intermission's (or a fresh process's) |
//! | carried player | as version 1: 0 none (in a level), 1 alive, 2 dead at the exit | present in the intermission and in the finale |
//! | finale | u8: 0 none, 1 followed by i32 `finalestage`, i32 `finalecount` | set exactly in the finale (f_finale.c:52-54) |
//! | level | u32 length n, n bytes | `DSIM` version 43 (wire 27; a version 42 capture is refused) |
//! | display | u32 length n, n bytes | `DDSP` version 4 (a melt in progress included) |
//! | sound | u8 kind = 3, u32 length n, n bytes | `DSND` version 3 (a pending display included) |
//!
//! Nothing follows. [`SessionStateV2::check`] lists what a decoded state must keep (version 1's
//! rules where they apply, and those of the finale and the melt), and [`Session::restore_v2`]
//! what it must share with the level.
use super::finale::STAGE_THRESHOLD;
use super::finale_melt::Configuration;
use super::intermission::{
    Anim, Intermission, Latches, NUM_ANIMS, PARS_E1, WbPlayer, WbStart, next_map,
};
use super::state::{
    DsesError, MAGIC, MAX_BYTES, Reader, VIEWHEIGHT, Writer, game_state_byte, read_start,
    wi_fields, wminfo_fields, write_start,
};
use super::{
    CarriedPlayer, DeclaredMode, ModeSource, Session, SessionData, SessionMode, SessionSettings,
};
use doom_render::{Display, DisplayConfig, FinaleView, Frame, GameState, SessionDisplayStateV4};
use doom_sim::{CarriedStart, Diagnostic, DiagnosticLimits, LevelExit, Simulation};
use doom_sound::chain::{ChainConfig, ChainSound};
use doom_sound::chain_melt::ChainSoundStateV3;
use std::sync::Arc;

/// The version written and read here.
pub const VERSION: u16 = 2;
/// The sound slot kind of `DSND` version 3.
pub const SOUND_DSND3: u8 = 3;

/// The two versions sharing this layout: version 2 (wire 27, DSIM43) and version 3 (wire 28,
/// DSIM44, [`super::state_v3`]). Each names its version, its configuration (whose skills the
/// settings may hold) and the reader of its embedded level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Layout {
    /// `DSES` version 2.
    V2,
    /// `DSES` version 3.
    V3,
}

impl Layout {
    fn version(self) -> u16 {
        match self {
            Self::V2 => VERSION,
            Self::V3 => super::state_v3::VERSION,
        }
    }

    fn config(self) -> Configuration {
        match self {
            Self::V2 => Configuration::FinaleAndMelt,
            Self::V3 => Configuration::FinaleAndMeltWire28,
        }
    }

    /// The embedded level's reader: DSIM43 alone for version 2, DSIM44 alone for version 3.
    fn level(self, bytes: &[u8]) -> Result<Diagnostic<'_>, DsesError> {
        let limits = DiagnosticLimits::default();
        match self {
            Self::V2 => Diagnostic::decode_episode1_v5(bytes, limits),
            Self::V3 => Diagnostic::decode_episode1_v6(bytes, limits),
        }
        .map_err(|e| DsesError::Level(format!("{e:?}")))
    }
}

/// A decoded `DSES` version 2 state.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionStateV2 {
    /// The declared game mode.
    pub mode: DeclaredMode,
    /// The settings.
    pub settings: SessionSettings,
    /// `gametic`.
    pub gametic: u64,
    /// `gamestate`.
    pub gamestate: GameState,
    /// `gamemap`.
    pub map: u8,
    /// `gameaction` (0, 6 or 8).
    pub gameaction: u8,
    /// `secretexit` (as version 1: the last completed exit's).
    pub secretexit: bool,
    /// `didsecret`.
    pub didsecret: bool,
    /// `wminfo`.
    pub wminfo: WbStart,
    /// The wi_stuff.c statics.
    pub wi: Intermission,
    /// The carried player (in the intermission and the finale); a living one's `P_Random` index
    /// is read as 0.
    pub carried: Option<CarriedPlayer>,
    /// `finalestage` and `finalecount` (the finale only).
    pub finale: Option<FinaleView>,
    /// The level's `DSIM` version 43 capture (wire 27).
    pub level: Vec<u8>,
    /// `DDSP` version 4.
    pub display: SessionDisplayStateV4,
    /// `DSND` version 3.
    pub sound: ChainSoundStateV3,
}

fn read_wminfo(r: &mut Reader<'_>) -> Result<WbStart, DsesError> {
    let epsd = r.i32()?;
    let didsecret = r.b("wminfo didsecret")?;
    let mut v = [0i32; 10];
    for x in &mut v {
        *x = r.i32()?;
    }
    Ok(WbStart {
        epsd,
        didsecret,
        last: v[0],
        next: v[1],
        maxkills: v[2],
        maxitems: v[3],
        maxsecret: v[4],
        partime: v[5],
        plyr: WbPlayer {
            skills: v[6],
            sitems: v[7],
            ssecret: v[8],
            stime: v[9],
        },
    })
}

fn read_wi(r: &mut Reader<'_>) -> Result<Intermission, DsesError> {
    let mut s = [0i32; 12];
    for x in &mut s {
        *x = r.i32()?;
    }
    let snl_pointeron = r.b("snl_pointeron")?;
    let mut anims = [Anim::default(); NUM_ANIMS];
    for a in &mut anims {
        a.ctr = r.i32()?;
        a.nexttic = r.i32()?;
    }
    Ok(Intermission {
        state: s[0],
        acceleratestage: s[1],
        cnt: s[2],
        bcnt: s[3],
        firstrefresh: s[4],
        sp_state: s[5],
        cnt_kills: s[6],
        cnt_items: s[7],
        cnt_secret: s[8],
        cnt_time: s[9],
        cnt_par: s[10],
        cnt_pause: s[11],
        snl_pointeron,
        anims,
    })
}

fn read_carried(r: &mut Reader<'_>) -> Result<Option<CarriedPlayer>, DsesError> {
    Ok(match r.u8()? {
        0 => None,
        k @ (1 | 2) => {
            let start = if k == 1 { Some(read_start(r)?) } else { None };
            Some(CarriedPlayer {
                start,
                latches: Latches {
                    attackdown: r.b("latch attackdown")?,
                    usedown: r.b("latch usedown")?,
                },
                viewheight: r.i32()?,
                deltaviewheight: r.i32()?,
            })
        }
        _ => return Err(DsesError::Value("carried")),
    })
}

impl SessionStateV2 {
    /// The encoding (module documentation).
    pub fn encode(&self) -> Vec<u8> {
        self.encode_as(Layout::V2)
    }

    /// The encoding under `layout`'s version number.
    pub(super) fn encode_as(&self, layout: Layout) -> Vec<u8> {
        let mut w = Writer(Vec::new());
        w.0.extend_from_slice(MAGIC);
        w.u16(layout.version());
        w.u8(u8::from(self.mode.mode == SessionMode::Retail));
        w.u8(u8::from(self.mode.source == ModeSource::Forced));
        w.u8(self.settings.map);
        w.u8(self.settings.skill);
        w.u8(self.settings.screenblocks);
        w.u64(self.gametic);
        w.u8(game_state_byte(self.gamestate));
        w.u8(self.map);
        w.u8(self.gameaction);
        w.b(self.secretexit);
        w.b(self.didsecret);
        w.i32(self.wminfo.epsd);
        w.b(self.wminfo.didsecret);
        wminfo_fields(&self.wminfo).iter().for_each(|v| w.i32(*v));
        wi_fields(&self.wi).iter().for_each(|v| w.i32(*v));
        w.b(self.wi.snl_pointeron);
        for a in &self.wi.anims {
            w.i32(a.ctr);
            w.i32(a.nexttic);
        }
        match &self.carried {
            None => w.u8(0),
            Some(c) => {
                match &c.start {
                    None => w.u8(2),
                    Some(s) => {
                        w.u8(1);
                        write_start(&mut w, s);
                    }
                }
                w.b(c.latches.attackdown);
                w.b(c.latches.usedown);
                w.i32(c.viewheight);
                w.i32(c.deltaviewheight);
            }
        }
        match &self.finale {
            None => w.u8(0),
            Some(f) => {
                w.u8(1);
                w.i32(f.stage);
                w.i32(f.count);
            }
        }
        w.blob(&self.level);
        w.blob(&self.display.encode());
        w.u8(SOUND_DSND3);
        w.blob(&self.sound.encode());
        w.0
    }

    /// Reads a version 2 state and checks it alone ([`Self::check`]): every length is bounded by
    /// the bytes left before anything is allocated, every field is checked in its domain, and the
    /// embedded states are read by their own readers. Never panics.
    pub fn decode(bytes: &[u8]) -> Result<Self, DsesError> {
        Self::decode_as(bytes, Layout::V2)
    }

    /// [`Self::decode`] for `layout`'s version (any other is refused) and checked by its rules.
    pub(super) fn decode_as(bytes: &[u8], layout: Layout) -> Result<Self, DsesError> {
        if bytes.len() > MAX_BYTES {
            return Err(DsesError::TooLarge);
        }
        let mut r = Reader(bytes);
        if r.take(4)? != MAGIC {
            return Err(DsesError::Magic);
        }
        let version = r.u16()?;
        if version != layout.version() {
            return Err(DsesError::Version(version));
        }
        let mode = DeclaredMode {
            mode: if r.b("mode")? {
                SessionMode::Retail
            } else {
                SessionMode::Registered
            },
            source: if r.b("mode source")? {
                ModeSource::Forced
            } else {
                ModeSource::Probed
            },
        };
        let settings = SessionSettings {
            map: r.u8()?,
            skill: r.u8()?,
            screenblocks: r.u8()?,
        };
        let gametic = r.u64()?;
        let gamestate = match r.u8()? {
            0 => GameState::Level,
            1 => GameState::Intermission,
            2 => GameState::Finale,
            _ => return Err(DsesError::Value("gamestate")),
        };
        let map = r.u8()?;
        let gameaction = r.u8()?;
        let secretexit = r.b("secretexit")?;
        let didsecret = r.b("didsecret")?;
        let wminfo = read_wminfo(&mut r)?;
        let wi = read_wi(&mut r)?;
        let carried = read_carried(&mut r)?;
        let finale = match r.u8()? {
            0 => None,
            1 => Some(FinaleView {
                stage: r.i32()?,
                count: r.i32()?,
            }),
            _ => return Err(DsesError::Value("finale")),
        };
        let level = r.blob()?.to_vec();
        let display = SessionDisplayStateV4::decode(r.blob()?).map_err(DsesError::Display)?;
        let kind = r.u8()?;
        if kind != SOUND_DSND3 {
            return Err(DsesError::SoundKind(kind));
        }
        let sound = ChainSoundStateV3::decode(r.blob()?).map_err(DsesError::Sound)?;
        if !r.0.is_empty() {
            return Err(DsesError::Trailing);
        }
        let state = Self {
            mode,
            settings,
            gametic,
            gamestate,
            map,
            gameaction,
            secretexit,
            didsecret,
            wminfo,
            wi,
            carried,
            finale,
            level,
            display,
            sound,
        };
        state.check_as(layout)?;
        Ok(state)
    }

    /// The rules a state keeps without its session data (each error names its rule):
    ///
    /// - version 1's rules, where they apply: the field domains; the embedded level capture's
    ///   map, tic and start `gametic` against the outer state; the display's own `check` (here
    ///   `DDSP` version 4's, a melt in progress included), no wipe due after a whole tic, its tic,
    ///   game state and screen size; the sound's game state, tics and map; in the intermission
    ///   `wminfo`, the wi_stuff.c statics, `bcnt`, `gameaction` 8 and the carried player as
    ///   version 1 checks them; in a level the statics of a fresh process or of a finished
    ///   intermission;
    /// - **the melt**: the display's melt in progress and the sound's pending display agree
    ///   ([`DsesError::MeltAndPendingDisplay`]: the session mixes the owed buffer exactly when the
    ///   melt ends), and the display's game mode is the declared one;
    /// - **the display's entry tic**: in a level the level's start, in the intermission the tic it
    ///   was entered, in the finale the victory tic in stage 0 and finale tic 1,571's in stage 1
    ///   (`begin_finale_art`, f_finale.c:245);
    /// - **the finale** (set exactly in the finale): map 8 (g_game.c:1033-1038), `gameaction` 0,
    ///   the carried player present with the status bar's last health and a `viewheight` at most
    ///   `VIEWHEIGHT`, and `finalestage` and `finalecount` those of the session tics since the
    ///   victory tic (f_finale.c:230, 241-248): stage 0 with `finalecount` = `gametic` - the
    ///   victory tic, 1..=1570; stage 1 with `finalecount` = `gametic` - the victory tic - 1,571,
    ///   at least 0;
    /// - **outside the intermission** (a level, the finale): the wi_stuff.c statics are a fresh
    ///   process's with a fresh `wminfo`, or a finished intermission's whose `next` led to the
    ///   current map (`G_DoWorldDone` loads `wminfo.next + 1`, g_game.c:1176; a reborn keeps it).
    pub fn check(&self) -> Result<(), DsesError> {
        self.check_as(Layout::V2)
    }

    /// [`Self::check`] under `layout`: its skills and its embedded level's reader.
    pub(super) fn check_as(&self, layout: Layout) -> Result<(), DsesError> {
        let value = |ok: bool, name| {
            if ok {
                Ok(())
            } else {
                Err(DsesError::Value(name))
            }
        };
        let rule = |ok: bool, name| {
            if ok {
                Ok(())
            } else {
                Err(DsesError::Inconsistent(name))
            }
        };
        value((1..=9).contains(&self.settings.map), "start map")?;
        value(layout.config().skill(self.settings.skill).is_ok(), "skill")?;
        value(
            (10..=11).contains(&self.settings.screenblocks),
            "screenblocks",
        )?;
        value((1..=9).contains(&self.map), "gamemap")?;
        value(matches!(self.gameaction, 0 | 6 | 8), "gameaction")?;
        if let Some(f) = &self.finale {
            value((0..=1).contains(&f.stage), "finalestage")?;
            value(f.count >= 0, "finalecount")?;
        }
        self.wi.check().map_err(DsesError::Value)?;
        let in_level = self.gamestate == GameState::Level;
        let sound = &self.sound.state;
        let level = layout.level(&self.level)?;
        // Version 3 alone: every level is set up on the one `gameskill` (g_game.c:484, 1450).
        let level_skill = level
            .field("actors.options")
            .and_then(|o| o.first().copied());
        if layout == Layout::V3 && level_skill != Some(self.settings.skill) {
            return Err(DsesError::SkillAndLevel {
                skill: self.settings.skill,
                level: level_skill,
            });
        }
        let level_u64 = |path: &str| {
            level
                .field(path)
                .and_then(|v| <[u8; 8]>::try_from(v).ok())
                .map(u64::from_le_bytes)
        };
        rule(
            level.field("level.map") == Some(&[self.map][..]),
            "the level capture's map",
        )?;
        rule(
            level_u64("tic") == Some(sound.level_tics()),
            "the level capture's tic",
        )?;
        rule(
            level_u64("level.start.gametic") == Some(sound.level_start),
            "the level's start gametic and levelstarttic",
        )?;
        self.display.check().map_err(DsesError::DisplayRestore)?;
        rule(
            self.display.mode == self.mode.mode.game_mode(),
            "display game mode",
        )?;
        let d = &self.display.session;
        rule(
            d.wipe == (self.gametic == 0),
            "display wipe after a whole tic",
        )?;
        if sound.melt != self.display.melt.is_some() {
            return Err(DsesError::MeltAndPendingDisplay);
        }
        rule(d.scene.game_state() == self.gamestate, "display game state")?;
        rule(d.tic == self.gametic, "display tic")?;
        rule(
            d.screenblocks == self.settings.screenblocks,
            "display screenblocks",
        )?;
        rule(
            sound.state.byte() == game_state_byte(self.gamestate),
            "sound game state",
        )?;
        rule(sound.tics == self.gametic, "sound tics")?;
        rule(sound.gamemap == i32::from(self.map), "sound map")?;
        rule(in_level == self.carried.is_none(), "carried player")?;
        rule(
            self.finale.is_some() == (self.gamestate == GameState::Finale),
            "finale and gamestate",
        )?;
        rule(
            match self.gameaction {
                6 => in_level,
                8 => self.gamestate == GameState::Intermission,
                _ => true,
            },
            "gameaction and gamestate",
        )?;
        if let Some(c) = &self.carried {
            if let Some(s) = &c.start {
                s.validate()
                    .map_err(|_| DsesError::Value("carried start"))?;
            }
        }
        // The display entered its state in the tic the sound's level tics say (finale stage 1:
        // the forced wipe of finale tic 1,571, STAGE_THRESHOLD session tics after the victory).
        let entered = match (self.gamestate, &self.finale) {
            (GameState::Level, _) => Some(sound.level_start),
            (GameState::Intermission, _) => Some(sound.level_end),
            (GameState::Finale, Some(f)) if f.stage == 1 => sound
                .level_end
                .checked_add(u64::from(STAGE_THRESHOLD.unsigned_abs())),
            (GameState::Finale, _) => Some(sound.level_end),
        };
        rule(
            Some(d.entered) == entered,
            "display entered and the sound's level tics",
        )?;
        // The carried player outside a level: `st_oldhealth` is the finished level's last health
        // (st_stuff.c:994), and `viewheight` never exceeds VIEWHEIGHT (version 1's rules).
        if !in_level {
            let carried = self.carried.as_ref();
            rule(
                carried.is_some_and(|c| d.status.oldhealth == c.start.map_or(0, |s| s.health)),
                "carried health and st_oldhealth",
            )?;
            rule(
                carried.is_some_and(|c| c.viewheight <= VIEWHEIGHT),
                "carried viewheight",
            )?;
        }
        match self.gamestate {
            GameState::Intermission => self.check_intermission(),
            GameState::Finale => {
                rule(self.map == 8, "the finale follows map 8")?;
                rule(self.gameaction == 0, "gameaction in the finale")?;
                let f = self
                    .finale
                    .as_ref()
                    .ok_or(DsesError::Inconsistent("finale and gamestate"))?;
                // finalecount from the session tics since the victory tic (f_finale.c:230,
                // 241-248): the victory tic leaves count 1.
                let since = self.gametic.checked_sub(sound.level_end);
                let count = u64::from(f.count.unsigned_abs());
                let threshold = u64::from(STAGE_THRESHOLD.unsigned_abs());
                let ok = match (f.stage, since) {
                    (0, Some(n)) => (1..=threshold).contains(&count) && n == count,
                    (1, Some(n)) => count.checked_add(threshold + 1) == Some(n),
                    _ => false,
                };
                rule(ok, "finalecount and the finale's tics")?;
                self.check_outside_intermission()
            }
            GameState::Level => self.check_outside_intermission(),
        }
    }

    /// In a level or the finale: the statics of a fresh process with a fresh `wminfo`, or of a
    /// finished intermission (`NoState` with its count run out, wi_stuff.c:737-747) whose `next`
    /// led to the current map.
    fn check_outside_intermission(&self) -> Result<(), DsesError> {
        let fresh = self.wi == Intermission::default();
        let finished = self.wi.state == -1 && self.wi.cnt == 0;
        let ok = if fresh {
            self.wminfo == WbStart::default()
        } else {
            finished && self.wminfo.next.checked_add(1) == Some(i32::from(self.map))
        };
        if ok {
            Ok(())
        } else {
            Err(DsesError::Inconsistent("the last intermission's statics"))
        }
    }

    /// Version 1's intermission rules (`SessionState::check`).
    fn check_intermission(&self) -> Result<(), DsesError> {
        let rule = |ok: bool, name| {
            if ok {
                Ok(())
            } else {
                Err(DsesError::Inconsistent(name))
            }
        };
        let m = &self.wminfo;
        let last = m
            .last
            .checked_add(1)
            .and_then(|v| u8::try_from(v).ok())
            .unwrap_or(0);
        rule(m.epsd == 0 && last == self.map, "wminfo map")?;
        rule((1..=9).contains(&last) && last != 8, "wminfo last")?;
        rule(m.next == next_map(last, self.secretexit), "wminfo next")?;
        rule(
            m.partime == 35 * PARS_E1[usize::from(last) - 1],
            "wminfo partime",
        )?;
        rule(
            m.maxkills >= 1 && m.maxitems >= 1 && m.maxsecret >= 1,
            "wminfo maxima",
        )?;
        rule(
            [m.plyr.skills, m.plyr.sitems, m.plyr.ssecret, m.plyr.stime]
                .iter()
                .all(|v| *v >= 0),
            "wminfo counts",
        )?;
        rule(self.map != 9 || m.didsecret, "didsecret after map 9")?;
        self.wi.reachable(m).map_err(DsesError::Inconsistent)?;
        rule(
            u64::try_from(self.wi.bcnt).ok()
                == self.gametic.checked_sub(self.sound.state.level_end),
            "bcnt and the intermission's tics",
        )?;
        rule(
            (self.gameaction == 8) == (self.wi.state == -1 && self.wi.cnt == 0),
            "gameaction 8 and the intermission's end",
        )
    }
}

impl Session {
    /// The saved state (`DSES` version 2) between two calls of a finale-and-melt session, in a
    /// melt too. Refused: in the frozen configuration ([`DsesError::Configuration`], whose save is
    /// [`Session::save`]), and after a rule-2 refusal or with a planted fault
    /// ([`DsesError::NotSaveable`]).
    pub fn save_v2(&self) -> Result<SessionStateV2, DsesError> {
        self.save_as(Layout::V2)
    }

    /// [`Self::save_v2`] for `layout`'s configuration alone.
    pub(super) fn save_as(&self, layout: Layout) -> Result<SessionStateV2, DsesError> {
        if self.config != layout.config() {
            return Err(DsesError::Configuration(self.config));
        }
        if self.ended.is_some() || self.fault.is_some() {
            return Err(DsesError::NotSaveable);
        }
        let display = self
            .display
            .session_state_v4_checked()
            .map_err(|_| DsesError::NotSaveable)?;
        let sound = self
            .sound
            .save_v3_checked()
            .map_err(|_| DsesError::NotSaveable)?;
        let mut carried = self.carried;
        if let Some(CarriedPlayer { start: Some(s), .. }) = carried.as_mut() {
            s.p_random = 0;
        }
        Ok(SessionStateV2 {
            mode: self.data.mode,
            settings: self.settings,
            gametic: self.gametic,
            gamestate: self.gamestate(),
            map: self.map,
            gameaction: self.gameaction(),
            secretexit: self.secretexit,
            didsecret: self.didsecret,
            wminfo: self.wminfo,
            wi: self.wi,
            carried,
            finale: self.finale,
            level: self.level.snapshot().canonical_bytes(),
            display,
            sound,
        })
    }

    /// A finale-and-melt session rebuilt from `state` over `data` and `level` (the level's
    /// simulation at the state's tic; in the intermission and the finale the finished level's
    /// after its last tic), whose capture must be the embedded one: [`Session::restore`]'s checks
    /// for version 2 (the finale's carried player against the finished level's as in the
    /// intermission; `wminfo` against it in the intermission only, since map 8 writes none,
    /// g_game.c:1033-1038), the display's version 4 restore (its melt in progress and game mode
    /// included) and the sound's version 3 restore (its pending display included). On an error
    /// nothing is built.
    pub fn restore_v2(
        data: Arc<SessionData>,
        state: &SessionStateV2,
        level: Simulation,
    ) -> Result<Self, DsesError> {
        Self::restore_as(data, state, level, Layout::V2)
    }

    /// [`Self::restore_v2`] under `layout`: its rules and its configuration.
    pub(super) fn restore_as(
        data: Arc<SessionData>,
        state: &SessionStateV2,
        level: Simulation,
        layout: Layout,
    ) -> Result<Self, DsesError> {
        state.check_as(layout)?;
        if data.mode != state.mode {
            return Err(DsesError::Mode);
        }
        if level.snapshot().canonical_bytes() != state.level {
            return Err(DsesError::LevelMismatch);
        }
        let skill = layout
            .config()
            .skill(state.settings.skill)
            .map_err(|_| DsesError::Value("skill"))?;
        let (_, lvl) = data.map(state.map).map_err(DsesError::Map)?;
        let in_level = state.gamestate == GameState::Level;
        let exit = level.level_exit_request();
        let rule = |ok: bool, name| {
            if ok {
                Ok(())
            } else {
                Err(DsesError::Inconsistent(name))
            }
        };
        rule(
            (state.gameaction == 6) == (in_level && exit.is_some()),
            "gameaction and the level's exit",
        )?;
        if !in_level {
            rule(exit.is_some(), "a finished level without an exit")?;
            rule(
                (exit == Some(LevelExit::Secret)) == state.secretexit,
                "secretexit and the level's exit",
            )?;
            let finish = level
                .level_finish()
                .map_err(|_| DsesError::Inconsistent("the finished level's player"))?;
            rule(
                state
                    .carried
                    .as_ref()
                    .is_some_and(|c| c.start.is_none() == finish.player_dead),
                "carried player and the level's",
            )?;
            let left = finish.carried.map(|c| CarriedStart { p_random: 0, ..c });
            let saved = state.carried.and_then(|c| c.start);
            rule(left == saved, "carried player and the finished level's")?;
            if state.gamestate == GameState::Intermission {
                let one = |v: i32| if v == 0 { 1 } else { v };
                let m = &state.wminfo;
                rule(
                    [m.maxkills, m.maxitems, m.maxsecret] == finish.totals.map(one)
                        && [m.plyr.skills, m.plyr.sitems, m.plyr.ssecret] == finish.counts
                        && m.plyr.stime == i32::try_from(finish.level_time).unwrap_or(i32::MAX),
                    "wminfo and the finished level's counts",
                )?;
            }
        }
        let cfg = DisplayConfig {
            screenblocks: state.settings.screenblocks,
        };
        let mut display = Display::new_session_with(
            data.res.clone(),
            lvl.clone(),
            cfg,
            layout.config().display_config(data.mode.mode.game_mode()),
        )
        .map_err(|_| DsesError::Value("display"))?;
        let scene_level = in_level.then(|| (lvl.clone(), &level));
        display
            .restore_session_v4(state.display.clone(), scene_level)
            .map_err(DsesError::DisplayRestore)?;
        let sound = ChainSound::restore_v3(
            data.sound_set.clone(),
            data.music,
            &state.sound,
            &level,
            ChainConfig::fresh(data.textures),
        )
        .map_err(DsesError::SoundRestore)?;
        rule(
            level.level_start_gametic() == Some(sound.level_start()),
            "the level's start gametic and levelstarttic",
        )?;
        let mut carried = state.carried;
        if let Some(CarriedPlayer { start: Some(s), .. }) = carried.as_mut() {
            // Neither index is reset between levels (m_random.c:69-72 has one caller,
            // G_InitNew): the carried index is the finished level's.
            s.p_random = level.rng().indices()[0];
        }
        Ok(Self {
            settings: state.settings,
            skill,
            gametic: state.gametic,
            map: state.map,
            secretexit: state.secretexit,
            didsecret: state.didsecret,
            wminfo: state.wminfo,
            wi: state.wi,
            world_done: state.gameaction == 8,
            carried,
            level,
            display,
            sound,
            frame: Frame::new(),
            ended: None,
            fault: None,
            config: layout.config(),
            finale: state.finale,
            data,
        })
    }
}
