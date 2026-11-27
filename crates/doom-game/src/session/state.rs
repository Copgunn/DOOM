//! `DSES` version 1: the saved state of a session (M6a ruling U2 = B1), for restore-and-continue
//! at any session tic of the level chain: in a level, on its load tic, and in the intermission.
//!
//! It holds the session group (what exists only between levels in the C, M6a note §4.1) and
//! embeds the three states that already exist: the level's capture (`DSIM` version 42, wire 26),
//! the session display (`DDSP` version 3) and the session's sound (`DSND` version 2), as `DDSP`
//! embeds its renderer state. doom-sim has no restore from its capture (M5a to M5c restore next
//! to a clone of the simulation), so [`Session::restore`] takes the level's simulation (in the
//! intermission the finished level's, which stays alive read-only) and requires its capture to
//! be the embedded one byte for byte: the capture is identity evidence, not a source.
//!
//! Layout, little endian:
//!
//! | Field | Encoding | Meaning |
//! | --- | --- | --- |
//! | magic | 4 bytes `DSES` | |
//! | version | u16 = 1 | |
//! | mode, mode source | u8 (0 registered, 1 retail), u8 (0 probed, 1 forced) | the declared game mode (ruling U4) |
//! | start map, skill, screenblocks | u8, u8, u8 | the settings: 1..=9, 2..=3, 10..=11 |
//! | `gametic` | u64 | session tics run (d_net.c:745-746) |
//! | `gamestate` | u8: 0 level, 1 intermission, 2 finale | doomdef.h:127-133; 2 is refused (not built yet) |
//! | `gamemap` | u8, 1..=9 | the current or finished level's map |
//! | `gameaction` | u8: 0, 6 or 8 | after the last tic (d_event.h:54-63): 6 only in a level that requested an exit, 8 only in the intermission after `G_WorldDone` |
//! | `secretexit`, `didsecret` | u8, u8 (0 or 1) | `secretexit` as the last completed level left it (g_game.c:1000; on an exit tic the pending exit is in the level capture and [`Session::secretexit`] reports it, this byte follows at the completion); the console player's `didsecret` (d_player.h:164) |
//! | `wminfo` | i32 `epsd`, u8 `didsecret`, 10 x i32 (`last`, `next`, `maxkills`, `maxitems`, `maxsecret`, `partime`, `skills`, `sitems`, `ssecret`, `stime`) | d_player.h:173-211 |
//! | wi_stuff.c statics | 12 x i32 (`state`, `acceleratestage`, `cnt`, `bcnt`, `firstrefresh`, `sp_state`, the five counts, `cnt_pause`), u8 `snl_pointeron`, 10 x (i32 `ctr`, i32 `nexttic`) | wi_stuff.c:140-175, 301-336, 749, 1316 |
//! | carried player | u8: 0 none (in a level), 1 alive, 2 dead at the exit | |
//! | carried, 1 | i32 health, u8 armour type, i32 armour points, u16 weapons, u8 ready weapon, 4 x i32 ammo, 4 x i32 maxima, u8 backpack, i32 cheats, u8 `attackdown`, u8 `usedown`, i32 bob, i32 weapon x, i32 flash tics, 2 x i32 flash x, y | doom-sim's `CarriedStart` as the exit tic left it (g_game.c:779-792), without its `P_Random` index, which is the finished level's |
//! | carried, 1 or 2 | u8 `attackdown`, u8 `usedown`, i32 `viewheight`, i32 `deltaviewheight` | the latches as the intermission rewrote them (wi_stuff.c:1476-1497) and the exit tic's view |
//! | finale | u8: 0 none, 1 followed by i32 `finalestage`, i32 `finalecount` | room for the finale (f_finale.c:52-54); 1 is refused (not built yet) |
//! | level | u32 length n, n bytes | the level's `DSIM` version 42 capture (`canonical_bytes`) |
//! | display | u32 length n, n bytes | `DDSP` version 3 |
//! | sound | u8 kind = 2, u32 length n, n bytes | `DSND` version 2 (the kind names the embedded format; 2 is the only one) |
//!
//! Nothing follows. [`SessionState::check`] lists what a decoded state must keep, and
//! [`Session::restore`] what it must share with the level. Two things cannot be tied exactly and
//! are declared limits (docs/STATE_FORMAT.md): the carried view heights (outside a level neither
//! embedded state holds the finished player's view; `viewheight` is only bounded above) and the
//! latches after the first intermission tic (rewritten from commands the state does not hold).
//! A melt in progress is the display's and the sound's (`DDSP` v3 and `DSND` v2
//! each have its field, refused there until the melt pictures exist); input held during a melt
//! belongs to the app's event layer (M5b S4, S5), not to the session.
use super::intermission::{
    Anim, Intermission, Latches, NUM_ANIMS, PARS_E1, WbPlayer, WbStart, next_map,
};
use super::{
    CarriedPlayer, DeclaredMode, MapLoadError, ModeSource, Session, SessionData, SessionMode,
    SessionSettings, skill_of,
};
use doom_render::{
    Display, DisplayConfig, Frame, GameState, SessionDisplayState, SessionRestoreError,
    StateDecodeError,
};
use doom_sim::{
    ArmorType, CarriedStart, Diagnostic, DiagnosticLimits, LevelExit, PlayerArmorState, Simulation,
    Weapon,
};
use doom_sound::chain::{ChainConfig, ChainSound};
use doom_sound::chain_state::{ChainSoundState, ChainStateError};
use doom_types::Fixed;
use std::fmt;
use std::sync::Arc;

/// The magic.
pub const MAGIC: &[u8; 4] = b"DSES";
/// The version written and read here.
pub const VERSION: u16 = 1;
/// The sound slot kind of `DSND` version 2.
pub const SOUND_DSND2: u8 = 2;
/// The largest encoding read (bounds the whole before an embedded reader runs).
pub const MAX_BYTES: usize = 64 << 20;
/// `VIEWHEIGHT` (p_local.h:45), 41 units.
pub(super) const VIEWHEIGHT: i32 = 41 << 16;

/// Why a `DSES` state was refused. Nothing is built or changed.
#[derive(Debug)]
pub enum DsesError {
    /// Wrong magic.
    Magic,
    /// Unsupported version.
    Version(u16),
    /// The bytes end early.
    Truncated,
    /// Bytes after the last field.
    Trailing,
    /// Larger than [`MAX_BYTES`].
    TooLarge,
    /// A field out of its domain (the field's name).
    Value(&'static str),
    /// Fields that cannot hold together (the rule's name).
    Inconsistent(&'static str),
    /// The embedded level capture is not a valid `DSIM` version 42 capture.
    Level(String),
    /// The embedded display state could not be decoded.
    Display(StateDecodeError),
    /// The embedded sound state could not be decoded.
    Sound(ChainStateError),
    /// The sound slot kind is not one this build reads.
    SoundKind(u8),
    /// Not built yet: a state in the finale (after the owner's hand test).
    NotYetFinale,
    /// Restore: the session data's declared game mode is not the saved one.
    Mode,
    /// Restore: the simulation's capture is not the embedded one.
    LevelMismatch,
    /// Restore: the map could not be loaded.
    Map(MapLoadError),
    /// Decode or restore: the display refused its state (decode applies the display's own
    /// `check`; restore also checks it against the level).
    DisplayRestore(SessionRestoreError),
    /// Restore: the sound refused its state.
    SoundRestore(ChainStateError),
    /// Save: the session ended through a rule-2 refusal, or holds a planted fault.
    NotSaveable,
    /// Save: the session's configuration saves only the other version (owner's ruling 3 of
    /// 2026-11-23: a [`super::finale_melt::Configuration::FinaleAndMelt`] session saves as `DSES`
    /// version 2 only, a frozen one as version 1 only, a
    /// [`super::finale_melt::Configuration::FinaleAndMeltWire28`] one as version 3 only).
    /// Nothing is written.
    Configuration(super::finale_melt::Configuration),
    /// `DSES` version 2: the embedded display's melt in progress and the embedded sound's pending
    /// display disagree (a session mixes the owed buffer exactly when its melt ends, so a saved
    /// state has a pending display exactly when it has a melt in progress).
    MeltAndPendingDisplay,
    /// `DSES` version 3 only (owner's ruling 2 of 2026-11-26): the skill byte and the embedded
    /// level's skill (`actors.options[0]` of its DSIM44 capture) disagree. Versions 1 and 2 make
    /// no such check; their acceptance is frozen.
    SkillAndLevel {
        /// The state's skill byte.
        skill: u8,
        /// The embedded level's skill byte, if the capture names one.
        level: Option<u8>,
    },
    /// `DSES` version 4: the display slot kind is not one this build reads.
    DisplayKind(u8),
    /// Save, `DSES` version 4, **temporary (M6b step 7)**: the session has shown a frame while
    /// paused, which neither its picture (`M_PAUSE`, not drawn yet) nor the embedded `DDSP`
    /// version 4 and `DSND` version 3 (whose tic rules a paused level tic breaks) can hold.
    /// Nothing is written.
    PausedNotYet,
}

impl fmt::Display for DsesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DSES: {self:?}")
    }
}

impl std::error::Error for DsesError {}

/// A decoded `DSES` version 1 state.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionState {
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
    /// `secretexit`.
    pub secretexit: bool,
    /// `didsecret`.
    pub didsecret: bool,
    /// `wminfo`.
    pub wminfo: WbStart,
    /// The wi_stuff.c statics.
    pub wi: Intermission,
    /// The carried player (outside a level); a living one's `P_Random` index is read as 0.
    pub carried: Option<CarriedPlayer>,
    /// The level's `DSIM` version 42 capture.
    pub level: Vec<u8>,
    /// `DDSP` version 3.
    pub display: SessionDisplayState,
    /// `DSND` version 2.
    pub sound: ChainSoundState,
}

pub(super) struct Writer(pub(super) Vec<u8>);

impl Writer {
    pub(super) fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub(super) fn b(&mut self, v: bool) {
        self.0.push(u8::from(v));
    }
    pub(super) fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(super) fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(super) fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(super) fn blob(&mut self, v: &[u8]) {
        let n = u32::try_from(v.len()).expect("an embedded state below 4 GiB");
        self.0.extend_from_slice(&n.to_le_bytes());
        self.0.extend_from_slice(v);
    }
}

pub(super) struct Reader<'a>(pub(super) &'a [u8]);

impl<'a> Reader<'a> {
    pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8], DsesError> {
        if self.0.len() < n {
            return Err(DsesError::Truncated);
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    pub(super) fn u8(&mut self) -> Result<u8, DsesError> {
        Ok(self.take(1)?[0])
    }
    pub(super) fn b(&mut self, name: &'static str) -> Result<bool, DsesError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DsesError::Value(name)),
        }
    }
    fn arr<const N: usize>(&mut self) -> Result<[u8; N], DsesError> {
        Ok(self.take(N)?.try_into().expect("N bytes"))
    }
    pub(super) fn u16(&mut self) -> Result<u16, DsesError> {
        Ok(u16::from_le_bytes(self.arr()?))
    }
    pub(super) fn i32(&mut self) -> Result<i32, DsesError> {
        Ok(i32::from_le_bytes(self.arr()?))
    }
    fn fixed(&mut self) -> Result<Fixed, DsesError> {
        Ok(Fixed::from_bits(self.i32()?))
    }
    pub(super) fn u64(&mut self) -> Result<u64, DsesError> {
        Ok(u64::from_le_bytes(self.arr()?))
    }
    pub(super) fn blob(&mut self) -> Result<&'a [u8], DsesError> {
        let n = u32::from_le_bytes(self.arr()?) as usize;
        self.take(n)
    }
}

/// `weapontype_t` (d_items.h) as doom-sim's [`Weapon`].
fn weapon_of(v: u8) -> Result<Weapon, DsesError> {
    Ok(match v {
        0 => Weapon::Fist,
        1 => Weapon::Pistol,
        2 => Weapon::Shotgun,
        3 => Weapon::Chaingun,
        4 => Weapon::Rocket,
        5 => Weapon::Plasma,
        6 => Weapon::Bfg,
        7 => Weapon::Chainsaw,
        8 => Weapon::SuperShotgun,
        _ => return Err(DsesError::Value("ready weapon")),
    })
}

fn armor_of(v: u8) -> Result<ArmorType, DsesError> {
    Ok(match v {
        0 => ArmorType::None,
        1 => ArmorType::Green,
        2 => ArmorType::Blue,
        _ => return Err(DsesError::Value("armour type")),
    })
}

pub(super) fn game_state_byte(g: GameState) -> u8 {
    match g {
        GameState::Level => 0,
        GameState::Intermission => 1,
        GameState::Finale => 2,
    }
}

pub(super) fn wminfo_fields(m: &WbStart) -> [i32; 10] {
    [
        m.last,
        m.next,
        m.maxkills,
        m.maxitems,
        m.maxsecret,
        m.partime,
        m.plyr.skills,
        m.plyr.sitems,
        m.plyr.ssecret,
        m.plyr.stime,
    ]
}

pub(super) fn wi_fields(i: &Intermission) -> [i32; 12] {
    [
        i.state,
        i.acceleratestage,
        i.cnt,
        i.bcnt,
        i.firstrefresh,
        i.sp_state,
        i.cnt_kills,
        i.cnt_items,
        i.cnt_secret,
        i.cnt_time,
        i.cnt_par,
        i.cnt_pause,
    ]
}

pub(super) fn write_start(w: &mut Writer, s: &CarriedStart) {
    w.i32(s.health);
    w.u8(s.armor.kind.wire_id());
    w.i32(s.armor.points);
    w.u16(s.weapons_owned);
    w.u8(s.ready_weapon as u8);
    s.ammo.iter().chain(&s.max_ammo).for_each(|v| w.i32(*v));
    w.b(s.backpack);
    w.i32(s.cheats);
    w.b(s.attack_down);
    w.b(s.use_down);
    for v in [s.bob, s.weapon_x] {
        w.i32(v.bits());
    }
    w.i32(s.flash_tics);
    for v in s.flash_position {
        w.i32(v.bits());
    }
}

pub(super) fn read_start(r: &mut Reader<'_>) -> Result<CarriedStart, DsesError> {
    let health = r.i32()?;
    let kind = armor_of(r.u8()?)?;
    let points = r.i32()?;
    let weapons_owned = r.u16()?;
    let ready_weapon = weapon_of(r.u8()?)?;
    let mut ammo = [0; 4];
    let mut max_ammo = [0; 4];
    for x in ammo.iter_mut().chain(&mut max_ammo) {
        *x = r.i32()?;
    }
    Ok(CarriedStart {
        health,
        armor: PlayerArmorState { kind, points },
        weapons_owned,
        ready_weapon,
        ammo,
        max_ammo,
        backpack: r.b("backpack")?,
        cheats: r.i32()?,
        attack_down: r.b("attackdown")?,
        use_down: r.b("usedown")?,
        bob: r.fixed()?,
        weapon_x: r.fixed()?,
        flash_tics: r.i32()?,
        flash_position: [r.fixed()?, r.fixed()?],
        p_random: 0,
    })
}

impl SessionState {
    /// The encoding (module documentation).
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer(Vec::new());
        w.0.extend_from_slice(MAGIC);
        w.u16(VERSION);
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
        w.u8(0); // no finale (not built yet)
        w.blob(&self.level);
        w.blob(&self.display.encode());
        w.u8(SOUND_DSND2);
        w.blob(&self.sound.encode());
        w.0
    }

    /// Reads a version 1 state and checks it alone ([`Self::check`]).
    pub fn decode(bytes: &[u8]) -> Result<Self, DsesError> {
        if bytes.len() > MAX_BYTES {
            return Err(DsesError::TooLarge);
        }
        let mut r = Reader(bytes);
        if r.take(4)? != MAGIC {
            return Err(DsesError::Magic);
        }
        let version = r.u16()?;
        if version != VERSION {
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
        let epsd = r.i32()?;
        let wdid = r.b("wminfo didsecret")?;
        let mut v = [0i32; 10];
        for x in &mut v {
            *x = r.i32()?;
        }
        let wminfo = WbStart {
            epsd,
            didsecret: wdid,
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
        };
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
        let wi = Intermission {
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
        };
        let carried = match r.u8()? {
            0 => None,
            k @ (1 | 2) => {
                let start = if k == 1 {
                    Some(read_start(&mut r)?)
                } else {
                    None
                };
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
        };
        match r.u8()? {
            0 => {}
            1 => return Err(DsesError::NotYetFinale),
            _ => return Err(DsesError::Value("finale")),
        }
        let level = r.blob()?.to_vec();
        let display = SessionDisplayState::decode(r.blob()?).map_err(DsesError::Display)?;
        let kind = r.u8()?;
        if kind != SOUND_DSND2 {
            return Err(DsesError::SoundKind(kind));
        }
        let sound = ChainSoundState::decode(r.blob()?).map_err(DsesError::Sound)?;
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
            level,
            display,
            sound,
        };
        state.check()?;
        Ok(state)
    }

    /// The rules a state keeps without its session data (each error names its rule): the field
    /// domains; the embedded level capture's map, tic and start `gametic` against the outer
    /// state; the display's own `check`, no wipe due after a whole tic, and its entry tic against
    /// the sound's level tics; outside a level `wminfo` as `G_DoCompleted` wrote it, the
    /// wi_stuff.c statics as an intermission in progress keeps them
    /// ([`Intermission::reachable`]), `bcnt` against the session tics, `gameaction` 8 exactly at
    /// the intermission's end, and the carried player against the status bar's last health; in a
    /// level the statics of a fresh process or of a finished intermission.
    pub fn check(&self) -> Result<(), DsesError> {
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
        value(skill_of(self.settings.skill).is_ok(), "skill")?;
        value(
            (10..=11).contains(&self.settings.screenblocks),
            "screenblocks",
        )?;
        value((1..=9).contains(&self.map), "gamemap")?;
        value(matches!(self.gameaction, 0 | 6 | 8), "gameaction")?;
        if self.gamestate == GameState::Finale {
            return Err(DsesError::NotYetFinale);
        }
        self.wi.check().map_err(DsesError::Value)?;
        let in_level = self.gamestate == GameState::Level;
        // The embedded level capture, read by its own reader and tied to the outer state: its
        // map, its tic count and the `gametic` of its load are the session's.
        let level = Diagnostic::decode_episode1_v4(&self.level, DiagnosticLimits::default())
            .map_err(|e| DsesError::Level(format!("{e:?}")))?;
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
            level_u64("tic") == Some(self.sound.level_tics()),
            "the level capture's tic",
        )?;
        rule(
            level_u64("level.start.gametic") == Some(self.sound.level_start),
            "the level's start gametic and levelstarttic",
        )?;
        // The display's own rules (a melt in progress, the wipe flag, a level scene's tic, carried
        // taint, `st_oldhealth`), which a restore applies, hold for a decoded state as well.
        self.display.check().map_err(DsesError::DisplayRestore)?;
        // A session state is saved after a whole tic: its frame is drawn, so no wipe is due
        // (d_main.c:300), except before the first tic of all.
        rule(
            self.display.wipe == (self.gametic == 0),
            "display wipe after a whole tic",
        )?;
        // The display entered its state, and the sound its level or its end, in the same tic.
        rule(
            self.display.entered
                == if in_level {
                    self.sound.level_start
                } else {
                    self.sound.level_end
                },
            "display entered and the sound's level tics",
        )?;
        rule(
            self.display.scene.game_state() == self.gamestate,
            "display game state",
        )?;
        rule(self.display.tic == self.gametic, "display tic")?;
        rule(
            self.display.screenblocks == self.settings.screenblocks,
            "display screenblocks",
        )?;
        rule(
            self.sound.state.byte() == game_state_byte(self.gamestate),
            "sound game state",
        )?;
        rule(self.sound.tics == self.gametic, "sound tics")?;
        rule(self.sound.gamemap == i32::from(self.map), "sound map")?;
        rule(in_level == self.carried.is_none(), "carried player")?;
        rule(
            match self.gameaction {
                6 => in_level,
                8 => !in_level,
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
        if !in_level {
            // What G_DoCompleted and WI_initVariables wrote for the finished map
            // (g_game.c:1039-1131, wi_stuff.c:1824-1831).
            let m = &self.wminfo;
            // `last` is the file's value: `+ 1` only where it cannot leave `int`.
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
            // The wi_stuff.c statics are an intermission's in progress over this `wminfo`
            // (counts the tally can multiply, counters on its steps, animation tics ahead).
            self.wi.reachable(m).map_err(DsesError::Inconsistent)?;
            // `bcnt` counts this intermission's tics (wi_stuff.c:1506), the session tics since
            // the level ended.
            rule(
                u64::try_from(self.wi.bcnt).ok() == self.gametic.checked_sub(self.sound.level_end),
                "bcnt and the intermission's tics",
            )?;
            // `G_WorldDone` runs exactly when the last count reaches 0 (wi_stuff.c:737-747).
            rule(
                (self.gameaction == 8) == (self.wi.state == -1 && self.wi.cnt == 0),
                "gameaction 8 and the intermission's end",
            )?;
            let carried = self.carried.as_ref();
            // `st_oldhealth` is the finished level's last health (st_stuff.c:994): the carried
            // player's, or a dead player's 0.
            rule(
                carried.is_some_and(|c| {
                    self.display.status.oldhealth == c.start.map_or(0, |s| s.health)
                }),
                "carried health and st_oldhealth",
            )?;
            // `viewheight` never exceeds VIEWHEIGHT (p_user.c:112-116, 190-195; a step up only
            // lowers it, p_mobj.c:310-315). Nothing outside a level holds the finished player's
            // view heights, so their exact values cannot be checked (module documentation).
            rule(
                carried.is_some_and(|c| c.viewheight <= VIEWHEIGHT),
                "carried viewheight",
            )?;
        } else {
            // In a level the statics are a fresh process's, or the last intermission's as its
            // final tic left them (never read again before `WI_Start` rewrites them).
            rule(
                self.wi == Intermission::default() || (self.wi.state == -1 && self.wi.cnt == 0),
                "the last intermission's statics",
            )?;
        }
        Ok(())
    }
}

impl Session {
    /// The saved state (`DSES` version 1) after the last tic. A session that ended through a
    /// rule-2 refusal, or holds a planted fault, cannot be saved.
    pub fn save(&self) -> Result<SessionState, DsesError> {
        if self.config != super::finale_melt::Configuration::RefusesFinaleAndMelt {
            return Err(DsesError::Configuration(self.config));
        }
        if self.ended.is_some() || self.fault.is_some() {
            return Err(DsesError::NotSaveable);
        }
        let display = self.display.session_state().ok_or(DsesError::NotSaveable)?;
        let mut carried = self.carried;
        if let Some(CarriedPlayer { start: Some(s), .. }) = carried.as_mut() {
            s.p_random = 0;
        }
        Ok(SessionState {
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
            level: self.level.snapshot().canonical_bytes(),
            display,
            sound: self.sound.save(),
        })
    }

    /// A session rebuilt from `state` over `data` and `level`: the level's simulation at the
    /// state's tic (in the intermission the finished level's after its last tic), whose capture
    /// must be the embedded one. Checked besides [`SessionState::check`]: the declared mode
    /// against the data's, the capture, the map, `gameaction` and `secretexit` against the
    /// level's exit request, outside a level the carried player field for field and `wminfo`'s
    /// maxima, counts and time against the finished level's ([`Simulation::level_finish`]), the
    /// display's and the sound's own restores against the level, and the level's start `gametic`
    /// (`DSIM` 42 `level.start.gametic`) against the sound's `levelstarttic`. On an error nothing
    /// is built.
    pub fn restore(
        data: Arc<SessionData>,
        state: &SessionState,
        level: Simulation,
    ) -> Result<Self, DsesError> {
        state.check()?;
        if data.mode != state.mode {
            return Err(DsesError::Mode);
        }
        if level.snapshot().canonical_bytes() != state.level {
            return Err(DsesError::LevelMismatch);
        }
        let skill = skill_of(state.settings.skill).map_err(|_| DsesError::Value("skill"))?;
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
            let dead = level
                .level_finish()
                .map_err(|_| DsesError::Inconsistent("the finished level's player"))?
                .player_dead;
            rule(
                state
                    .carried
                    .as_ref()
                    .is_some_and(|c| c.start.is_none() == dead),
                "carried player and the level's",
            )?;
            // What `G_PlayerFinishLevel` left and `G_DoCompleted` counted is the finished
            // level's: the carried player field for field (the saved one has no `P_Random`
            // index; its latches are the exit tic's, the intermission rewrites its own copy),
            // and `wminfo`'s totals, counts and time (g_game.c:1113-1131).
            let finish = level
                .level_finish()
                .map_err(|_| DsesError::Inconsistent("the finished level's player"))?;
            let left = finish.carried.map(|c| CarriedStart { p_random: 0, ..c });
            let saved = state.carried.and_then(|c| c.start);
            rule(left == saved, "carried player and the finished level's")?;
            let one = |v: i32| if v == 0 { 1 } else { v };
            let m = &state.wminfo;
            rule(
                [m.maxkills, m.maxitems, m.maxsecret] == finish.totals.map(one)
                    && [m.plyr.skills, m.plyr.sitems, m.plyr.ssecret] == finish.counts
                    && m.plyr.stime == i32::try_from(finish.level_time).unwrap_or(i32::MAX),
                "wminfo and the finished level's counts",
            )?;
        }
        let cfg = DisplayConfig {
            screenblocks: state.settings.screenblocks,
        };
        let mut display = Display::new_session(data.res.clone(), lvl.clone(), cfg)
            .map_err(|_| DsesError::Value("display"))?;
        let scene_level = in_level.then(|| (lvl.clone(), &level));
        display
            .restore_session(state.display.clone(), scene_level)
            .map_err(DsesError::DisplayRestore)?;
        let sound = ChainSound::restore(
            data.sound_set.clone(),
            data.music,
            &state.sound,
            &level,
            ChainConfig::fresh(data.textures),
        )
        .map_err(DsesError::SoundRestore)?;
        // `levelstarttic = gametic` at the load (g_game.c:470), which wire 26 keeps for the turbo
        // test (owner's ruling 1 of 2026-11-22).
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
            config: super::finale_melt::Configuration::RefusesFinaleAndMelt,
            finale: None,
            parameters: doom_sim::GameParameters::default(),
            paused: false,
            shown_paused: false,
            head_owed: false,
            data,
        })
    }
}
