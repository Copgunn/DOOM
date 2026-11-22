//! The single-player intermission logic of wi_stuff.c for Episode 1 (M6a note §1.6, §4.4): the
//! update half of `WI_Start` and `WI_Ticker` (wi_stuff.c:1503-1536, 1796-1850), whose drawing half
//! is doom-render's `wi_stuff::draw` (it reads [`Intermission::view`]).
//!
//! Every wi_stuff.c static the single-player path writes is a field of [`Intermission`], which
//! lives as long as the session (they are file statics of one process: `snl_pointeron`,
//! wi_stuff.c:749, starts false and keeps its value from one intermission to the next, which gives
//! the first "you are here" frame of a session its hidden pointer, U8 quirk 4). The deathmatch
//! and network paths (`WI_initDeathmatchStats`, `WI_initNetgameStats` and their updates) are not
//! reachable in a single-player session.
//!
//! `M_Random` (m_random.c:63-67) is drawn here for the animated background in
//! [`Intermission::start`] and on entering "you are here" (`WI_initAnimatedBack`,
//! wi_stuff.c:500-528: ten `ANIM_ALWAYS` animations in episode 1, wi_stuff.c:226-238, one draw
//! each), and once for each sound start, in call order: every start here is
//! `S_StartSound(0, sfx)` with `sfx_pistol`, `sfx_barexp` or `sfx_sgcock`, none of which has a
//! link (sounds.c), so `S_StartSoundAtVolume` measures nothing (no origin, s_sound.c:300-324) and
//! draws exactly once for the pitch (s_sound.c:337-346) before choosing a channel. The starts
//! themselves are requests for the sound session, which draws the same values from the index
//! given in [`WiTic::sound_index`] (the session checks that it drew one per start). The animated
//! background is never drawn (wi_stuff.c:588), but its draws and counters run.
use doom_render::{IntermissionView, WiState};
use doom_sim::{BT_ATTACK, BT_USE};
use doom_sound::chain::IntermissionSound;
use doom_types::DoomRng;

/// `TICRATE` (doomdef.h:122).
const TICRATE: i32 = 35;
/// `SHOWNEXTLOCDELAY` in seconds (wi_stuff.c:300).
const SHOWNEXTLOCDELAY: i32 = 4;
/// `NUMANIMS[0]` (wi_stuff.c:263-268): the ten animations of episode 1's map.
pub const NUM_ANIMS: usize = 10;
/// `TICRATE/3`, the period of every episode 1 animation (wi_stuff.c:226-238).
const ANIM_PERIOD: i32 = TICRATE / 3;
/// `nanims` of every episode 1 animation (wi_stuff.c:226-238).
const ANIM_FRAMES: i32 = 3;

/// The par times of episode 1 in seconds (`pars[1][1..=9]`, g_game.c:978-984).
pub const PARS_E1: [i32; 9] = [30, 75, 120, 90, 165, 180, 180, 30, 165];

/// `wbplayerstruct_t` of the console player (d_player.h:173-185), as `G_DoCompleted` fills it
/// (g_game.c:1122-1131). `in` is 1, `frags` and `score` 0 in a single-player session.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WbPlayer {
    /// `skills`: the player's `killcount`.
    pub skills: i32,
    /// `sitems`: `itemcount`.
    pub sitems: i32,
    /// `ssecret`: `secretcount`.
    pub ssecret: i32,
    /// `stime`: `leveltime`.
    pub stime: i32,
}

/// `wbstartstruct_t` (d_player.h:187-211), the global `wminfo` (g_game.c:1064-1131) as far as a
/// single-player Episode 1 session writes it (`maxfrags` 0, `pnum` 0).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WbStart {
    /// `epsd` (0-based): 0.
    pub epsd: i32,
    /// `didsecret`: the console player's at completion (g_game.c:1064).
    pub didsecret: bool,
    /// `last` (0-based map just finished).
    pub last: i32,
    /// `next` (0-based map to enter, M6a note §1.4).
    pub next: i32,
    /// `maxkills` (`totalkills`); `WI_initVariables` writes 1 into a zero (wi_stuff.c:1824-1831).
    pub maxkills: i32,
    /// `maxitems` (`totalitems`), likewise.
    pub maxitems: i32,
    /// `maxsecret` (`totalsecret`), likewise.
    pub maxsecret: i32,
    /// `partime` in tics: `35*pars[1][map]` (g_game.c:1116-1119).
    pub partime: i32,
    /// `plyr[0]`.
    pub plyr: WbPlayer,
}

/// One animation of the episode 1 map (`anim_t`'s `ctr` and `nexttic`, wi_stuff.c:140-175).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Anim {
    /// `ctr`: the frame to draw next (-1 after `WI_initAnimatedBack`).
    pub ctr: i32,
    /// `nexttic`: the `bcnt` at which it advances.
    pub nexttic: i32,
}

/// The wi_stuff.c statics of the single-player path (wi_stuff.c:301-336, 749, 1316). A fresh
/// process has every one zero or false (C statics), including `state` (0 = `StatCount`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Intermission {
    /// `state` (`stateenum_t`: -1 `NoState`, 0 `StatCount`, 1 `ShowNextLoc`; wi_stuff.h:31-36).
    pub state: i32,
    /// `acceleratestage` (0 or 1).
    pub acceleratestage: i32,
    /// `cnt`.
    pub cnt: i32,
    /// `bcnt`.
    pub bcnt: i32,
    /// `firstrefresh` (written 1 by `WI_initVariables`, read nowhere).
    pub firstrefresh: i32,
    /// `sp_state` (wi_stuff.c:1316).
    pub sp_state: i32,
    /// `cnt_kills[0]`.
    pub cnt_kills: i32,
    /// `cnt_items[0]`.
    pub cnt_items: i32,
    /// `cnt_secret[0]`.
    pub cnt_secret: i32,
    /// `cnt_time`.
    pub cnt_time: i32,
    /// `cnt_par`.
    pub cnt_par: i32,
    /// `cnt_pause`.
    pub cnt_pause: i32,
    /// `snl_pointeron` (wi_stuff.c:749): written by the logic (768) and by the drawer (814).
    pub snl_pointeron: bool,
    /// The ten animations of episode 1.
    pub anims: [Anim; NUM_ANIMS],
}

/// What one `WI_Ticker` (and, on the first intermission tic, `WI_Start`) asked of the rest of the
/// session.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WiTic {
    /// The sound starts, in call order (`S_StartSound(0, sfx)`, all with no origin).
    pub starts: Vec<IntermissionSound>,
    /// The `M_Random` index just before the first start (the sound session draws their pitches
    /// from it; no other draw falls between two starts of one tic).
    pub sound_index: u8,
    /// `S_ChangeMusic(mus_inter, true)` at `bcnt == 1` (wi_stuff.c:1508-1515).
    pub music_inter: bool,
    /// `WI_updateNoState` reached `G_WorldDone` (wi_stuff.c:740-746).
    pub world_done: bool,
    /// `M_Random` draws made, sound starts included.
    pub draws: u16,
}

/// The two button latches of the player that `WI_checkForAccelerate` reads and rewrites
/// (`attackdown`, `usedown`; wi_stuff.c:1470-1498).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Latches {
    /// `attackdown`.
    pub attackdown: bool,
    /// `usedown`.
    pub usedown: bool,
}

impl Intermission {
    /// `WI_Start` (wi_stuff.c:1838-1850) single player: `WI_initVariables` (1796-1836) on the
    /// global `wminfo` (`wbs = &wminfo`), so a zero maximum becomes 1 in `wminfo` itself;
    /// `WI_loadData` is the display's (`Display::begin_intermission`); `WI_initStats`
    /// (1318-1328) with its `WI_initAnimatedBack` (ten draws at `bcnt` 0).
    pub fn start(&mut self, wbs: &mut WbStart, rng: &mut DoomRng) -> u16 {
        // WI_initVariables
        self.acceleratestage = 0;
        self.cnt = 0;
        self.bcnt = 0;
        self.firstrefresh = 1;
        if wbs.maxkills == 0 {
            wbs.maxkills = 1;
        }
        if wbs.maxitems == 0 {
            wbs.maxitems = 1;
        }
        if wbs.maxsecret == 0 {
            wbs.maxsecret = 1;
        }
        // (`gamemode != retail && epsd > 2` never holds for episode 1, wi_stuff.c:1833-1835.)
        // WI_initStats
        self.state = 0;
        self.acceleratestage = 0;
        self.sp_state = 1;
        self.cnt_kills = -1;
        self.cnt_items = -1;
        self.cnt_secret = -1;
        self.cnt_time = -1;
        self.cnt_par = -1;
        self.cnt_pause = TICRATE;
        self.init_animated_back(rng)
    }

    /// `WI_initAnimatedBack` (wi_stuff.c:500-528) for episode 1: every animation is
    /// `ANIM_ALWAYS`, so `ctr = -1` and `nexttic = bcnt + 1 + M_Random() % period`, in table order.
    fn init_animated_back(&mut self, rng: &mut DoomRng) -> u16 {
        for a in &mut self.anims {
            a.ctr = -1;
            a.nexttic = self.bcnt + 1 + i32::from(rng.m_random()) % ANIM_PERIOD;
        }
        NUM_ANIMS as u16
    }

    /// `WI_updateAnimatedBack` (wi_stuff.c:530-582) for episode 1 (`ANIM_ALWAYS` only, no draw).
    fn update_animated_back(&mut self) {
        for a in &mut self.anims {
            if self.bcnt == a.nexttic {
                a.ctr += 1;
                if a.ctr >= ANIM_FRAMES {
                    a.ctr = 0;
                }
                a.nexttic = self.bcnt + ANIM_PERIOD;
            }
        }
    }

    /// `WI_Ticker` (wi_stuff.c:1503-1536), single player, with the tic's command buttons and the
    /// player's latches (rewritten by `WI_checkForAccelerate`).
    pub fn ticker(
        &mut self,
        wbs: &WbStart,
        buttons: u8,
        latches: &mut Latches,
        rng: &mut DoomRng,
    ) -> WiTic {
        let mut out = WiTic {
            sound_index: rng.indices()[1],
            ..WiTic::default()
        };
        // bcnt++ (1506); the intermission music at bcnt == 1 (1508-1515).
        self.bcnt = self.bcnt.wrapping_add(1);
        out.music_inter = self.bcnt == 1;
        self.check_for_accelerate(buttons, latches);
        match self.state {
            0 => self.update_stats(wbs, rng, &mut out),
            1 => self.update_show_next_loc(rng, &mut out),
            _ => self.update_no_state(&mut out),
        }
        out
    }

    /// `WI_checkForAccelerate` (wi_stuff.c:1470-1498): a rising edge of `BT_ATTACK` or `BT_USE`
    /// against the latch sets `acceleratestage`; each latch becomes the button's state.
    fn check_for_accelerate(&mut self, buttons: u8, latches: &mut Latches) {
        if buttons & BT_ATTACK != 0 {
            if !latches.attackdown {
                self.acceleratestage = 1;
            }
            latches.attackdown = true;
        } else {
            latches.attackdown = false;
        }
        if buttons & BT_USE != 0 {
            if !latches.usedown {
                self.acceleratestage = 1;
            }
            latches.usedown = true;
        } else {
            latches.usedown = false;
        }
    }

    /// `S_StartSound(0, sfx)`: the request, and its one pitch draw (module documentation).
    fn start_sound(sfx: IntermissionSound, rng: &mut DoomRng, out: &mut WiTic) {
        if out.starts.is_empty() {
            out.sound_index = rng.indices()[1];
        }
        out.starts.push(sfx);
        rng.m_random();
        out.draws += 1;
    }

    /// `WI_updateStats` (wi_stuff.c:1330-1434).
    fn update_stats(&mut self, wbs: &WbStart, rng: &mut DoomRng, out: &mut WiTic) {
        self.update_animated_back();
        let p = wbs.plyr;
        let kills = p.skills.wrapping_mul(100) / wbs.maxkills;
        let items = p.sitems.wrapping_mul(100) / wbs.maxitems;
        let secret = p.ssecret.wrapping_mul(100) / wbs.maxsecret;
        let time = p.stime / TICRATE;
        let par = wbs.partime / TICRATE;
        if self.acceleratestage != 0 && self.sp_state != 10 {
            self.acceleratestage = 0;
            self.cnt_kills = kills;
            self.cnt_items = items;
            self.cnt_secret = secret;
            self.cnt_time = time;
            self.cnt_par = par;
            Self::start_sound(IntermissionSound::Barexp, rng, out);
            self.sp_state = 10;
        }
        let pistol = self.bcnt & 3 == 0;
        match self.sp_state {
            2 | 4 | 6 => {
                let (count, target) = match self.sp_state {
                    2 => (&mut self.cnt_kills, kills),
                    4 => (&mut self.cnt_items, items),
                    _ => (&mut self.cnt_secret, secret),
                };
                *count = count.wrapping_add(2);
                if pistol {
                    Self::start_sound(IntermissionSound::Pistol, rng, out);
                }
                if *count >= target {
                    *count = target;
                    Self::start_sound(IntermissionSound::Barexp, rng, out);
                    self.sp_state += 1;
                }
            }
            8 => {
                if pistol {
                    Self::start_sound(IntermissionSound::Pistol, rng, out);
                }
                self.cnt_time = self.cnt_time.wrapping_add(3);
                if self.cnt_time >= time {
                    self.cnt_time = time;
                }
                self.cnt_par = self.cnt_par.wrapping_add(3);
                if self.cnt_par >= par {
                    self.cnt_par = par;
                    if self.cnt_time >= time {
                        Self::start_sound(IntermissionSound::Barexp, rng, out);
                        self.sp_state += 1;
                    }
                }
            }
            10 => {
                if self.acceleratestage != 0 {
                    Self::start_sound(IntermissionSound::Sgcock, rng, out);
                    // `gamemode != commercial` (1419-1422).
                    self.init_show_next_loc(rng, out);
                }
            }
            s if s & 1 != 0 => {
                self.cnt_pause -= 1;
                if self.cnt_pause == 0 {
                    self.sp_state += 1;
                    self.cnt_pause = TICRATE;
                }
            }
            _ => {}
        }
    }

    /// `WI_initShowNextLoc` (wi_stuff.c:752-760).
    fn init_show_next_loc(&mut self, rng: &mut DoomRng, out: &mut WiTic) {
        self.state = 1;
        self.acceleratestage = 0;
        self.cnt = SHOWNEXTLOCDELAY * TICRATE;
        out.draws += self.init_animated_back(rng);
    }

    /// `WI_updateShowNextLoc` (wi_stuff.c:762-770).
    fn update_show_next_loc(&mut self, _rng: &mut DoomRng, _out: &mut WiTic) {
        self.update_animated_back();
        self.cnt -= 1;
        if self.cnt == 0 || self.acceleratestage != 0 {
            self.init_no_state();
        } else {
            self.snl_pointeron = (self.cnt & 31) < 20;
        }
    }

    /// `WI_initNoState` (wi_stuff.c:730-735).
    fn init_no_state(&mut self) {
        self.state = -1;
        self.acceleratestage = 0;
        self.cnt = 10;
    }

    /// `WI_updateNoState` (wi_stuff.c:737-747): `WI_End` (unloading only) and `G_WorldDone`
    /// when the count reaches 0.
    fn update_no_state(&mut self, out: &mut WiTic) {
        self.update_animated_back();
        self.cnt -= 1;
        if self.cnt == 0 {
            out.world_done = true;
        }
    }

    /// What `WI_Drawer` reads (doom-render's [`IntermissionView`]).
    pub fn view(&self, wbs: &WbStart) -> IntermissionView {
        IntermissionView {
            state: match self.state {
                -1 => WiState::NoState,
                0 => WiState::StatCount,
                _ => WiState::ShowNextLoc,
            },
            cnt_kills: self.cnt_kills,
            cnt_items: self.cnt_items,
            cnt_secret: self.cnt_secret,
            cnt_time: self.cnt_time,
            cnt_par: self.cnt_par,
            snl_pointeron: self.snl_pointeron,
            epsd: wbs.epsd,
            last: wbs.last,
            next: wbs.next,
            didsecret: wbs.didsecret,
        }
    }

    /// The bounds a saved intermission must keep (`DSES` v1): `state` -1..=1, `acceleratestage`
    /// 0 or 1, `sp_state` 0..=10, `cnt` 0..=140, `cnt_pause` 1..=35 (0 in a fresh process), every
    /// animation `ctr` -1..=2. Counts are not bounded beyond `i32` (they are the C's ints).
    pub fn check(&self) -> Result<(), &'static str> {
        let ok = [
            ((-1..=1).contains(&self.state), "state"),
            ((0..=1).contains(&self.acceleratestage), "acceleratestage"),
            ((0..=10).contains(&self.sp_state), "sp_state"),
            ((0..=140).contains(&self.cnt), "cnt"),
            ((0..=TICRATE).contains(&self.cnt_pause), "cnt_pause"),
            ((0..=1).contains(&self.firstrefresh), "firstrefresh"),
            (self.bcnt >= 0, "bcnt"),
            (
                self.anims
                    .iter()
                    .all(|a| (-1..ANIM_FRAMES).contains(&a.ctr)),
                "anim ctr",
            ),
        ];
        ok.iter().find(|(o, _)| !o).map_or(Ok(()), |(_, n)| Err(*n))
    }
}

/// `wminfo.next` for a completed episode 1 map that is not map 8 (g_game.c:1084-1109): the secret
/// exit goes to E1M9 (8); a normal exit from E1M9 returns to E1M4 (3); otherwise the next map.
pub fn next_map(gamemap: u8, secretexit: bool) -> i32 {
    if secretexit {
        8
    } else if gamemap == 9 {
        3
    } else {
        i32::from(gamemap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wbs(skills: i32, max: i32, stime: i32) -> WbStart {
        WbStart {
            maxkills: max,
            maxitems: max,
            maxsecret: max,
            partime: 35 * 30,
            plyr: WbPlayer {
                skills,
                sitems: skills,
                ssecret: skills,
                stime,
            },
            ..WbStart::default()
        }
    }

    /// The probe of the note §1.6 (observed with the genuine wi_stuff.c): with no press, state
    /// 10 is reached on intermission tic 175 + n2 + n4 + n6 + n8; for 50/75/50 % and 45 s against
    /// 30 s that is tic 281, with 26 pistols, 4 barexps and the 20 animation draws once the
    /// press comes. Here kills 1 of 2 etc.: 50 % for all three.
    #[test]
    fn the_tally_reaches_state_10_on_the_documented_tic() {
        let mut w = wbs(1, 2, 45 * 35);
        w.plyr.sitems = 3;
        w.maxitems = 4;
        let mut wi = Intermission::default();
        let mut rng = DoomRng::default();
        assert_eq!(wi.start(&mut w, &mut rng), 10);
        let mut latches = Latches::default();
        let mut tic = 0;
        let (mut pistols, mut barexps) = (0, 0);
        while wi.sp_state != 10 {
            tic += 1;
            let t = wi.ticker(&w, 0, &mut latches, &mut rng);
            pistols += t
                .starts
                .iter()
                .filter(|s| **s == IntermissionSound::Pistol)
                .count();
            barexps += t
                .starts
                .iter()
                .filter(|s| **s == IntermissionSound::Barexp)
                .count();
        }
        // n2 = ceil(51/2) = 26, n4 = ceil(76/2) = 38, n6 = 26, n8 = ceil(46/3) = 16: 175 + 106.
        assert_eq!(tic, 281);
        assert_eq!((pistols, barexps), (26, 4));
        // A press then: sgcock and ten more animation draws on the same tic.
        let t = wi.ticker(&w, BT_USE, &mut latches, &mut rng);
        assert_eq!(t.starts, vec![IntermissionSound::Sgcock]);
        assert_eq!(t.draws, 11);
        assert_eq!(wi.state, 1);
    }

    #[test]
    fn a_zero_maximum_becomes_one_and_a_held_button_does_not_accelerate() {
        let mut w = wbs(0, 0, 0);
        let mut wi = Intermission::default();
        let mut rng = DoomRng::default();
        wi.start(&mut w, &mut rng);
        assert_eq!((w.maxkills, w.maxitems, w.maxsecret), (1, 1, 1));
        let mut latches = Latches {
            attackdown: true,
            usedown: false,
        };
        wi.ticker(&w, BT_ATTACK, &mut latches, &mut rng);
        assert_eq!(wi.sp_state, 1, "held fire is no press");
        wi.ticker(&w, 0, &mut latches, &mut rng);
        assert!(!latches.attackdown);
        let t = wi.ticker(&w, BT_ATTACK, &mut latches, &mut rng);
        assert_eq!(
            (wi.sp_state, t.starts.clone()),
            (10, vec![IntermissionSound::Barexp])
        );
    }

    #[test]
    fn the_next_map_follows_g_docompleted() {
        assert_eq!(next_map(1, false), 1);
        assert_eq!(next_map(3, true), 8);
        assert_eq!(next_map(9, false), 3);
        assert_eq!(next_map(9, true), 8);
    }
}
