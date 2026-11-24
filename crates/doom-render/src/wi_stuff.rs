//! The single-player intermission drawer of wi_stuff.c for Episode 1 (M6a note §1.6, §4.4):
//! `WI_Drawer` (wi_stuff.c:1772-1793) with its single-player screens, the statistics
//! (`WI_drawStats`, wi_stuff.c:1436-1468) and "you are here" (`WI_drawShowNextLoc` and
//! `WI_drawNoState`, wi_stuff.c:771-816), and the graphics `WI_loadData` caches
//! (wi_stuff.c:1538-1706).
//!
//! The drawer reads only an explicit [`IntermissionView`] of the intermission state, which the
//! game layer's intermission logic owns and fills; it draws no random number (no `M_Random` or
//! `P_Random` call in any drawing function of wi_stuff.c) and writes exactly one piece of state,
//! `snl_pointeron`, which `WI_drawNoState` sets (wi_stuff.c:814): the new value is returned in
//! [`IntermissionFrame::snl_pointeron`] for the logic to keep.
//!
//! **The animated background is never drawn.** `WI_drawAnimatedBack` begins `if (commercial)
//! return;` (wi_stuff.c:588), and `commercial` is the enum constant 2 (doomdef.h:40-42), so it
//! always returns; its update and its `M_Random` draws (wi_stuff.c:523, 537) belong to the logic.
//!
//! **The background buffer.** `WI_loadData` draws the background into `screens[1]`
//! (wi_stuff.c:1557-1558) and every frame starts by copying `screens[1]` over `screens[0]`
//! (`WI_slamBackground`, wi_stuff.c:406-410). At screenblocks 10 and 11 (decision S14) nothing else
//! writes `screens[1]`: `R_FillBackScreen` returns when the view is 320 wide (r_draw.c:747-748),
//! and the save buffer that reuses it (g_game.c:1284) is not reached (saving is a later
//! milestone). `screens[1]` starts zeroed (`V_Init`, v_video.c:482-493, through `I_AllocLow`'s
//! `memset`, i_system.c:147-154), and drawing the same background lump again leaves the same
//! bytes, so the buffer is the background drawn over zeros at every intermission
//! ([`IntermissionGraphics::background`]).
//!
//! What the genuine code would do with memory it does not own is refused, never drawn
//! differently (decision S17): a post outside its lump or below the last row of its buffer
//! ([`DrawError`]), and a splat that does not fit at its node, where `WI_drawOnLnode` would read
//! the pointer after the one-element `&splat` (wi_stuff.c:469-487, 794, 798).
use crate::st_stuff::patch_named;
use crate::v_video::{Canvas, DrawError, Patch, Screen};
use crate::{ResourceError, Resources, SCREENHEIGHT, SCREENWIDTH};

/// `stateenum_t` (wi_stuff.h:31-36).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WiState {
    /// `NoState` (-1): "you are here" with the pointer always on (`WI_drawNoState`).
    NoState,
    /// `StatCount` (0): the statistics.
    StatCount,
    /// `ShowNextLoc` (1): "you are here" with the blinking pointer.
    ShowNextLoc,
}

/// Everything `WI_Drawer`'s single-player path reads (the intermission logic writes it):
/// the wi_stuff.c statics and the `wbstartstruct_t` fields of the episode. Nothing else of the
/// intermission state is read by the drawer: `sp_state`, `cnt_pause`, `bcnt`, `acceleratestage`
/// and the animation counters steer only the logic, since the animated background is never drawn
/// (wi_stuff.c:588).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntermissionView {
    /// `state` (wi_stuff.c:310).
    pub state: WiState,
    /// `cnt_kills[0]` (wi_stuff.c:326): -1 until counted (`WI_initStats`, wi_stuff.c:1323).
    pub cnt_kills: i32,
    /// `cnt_items[0]` (wi_stuff.c:327).
    pub cnt_items: i32,
    /// `cnt_secret[0]` (wi_stuff.c:328).
    pub cnt_secret: i32,
    /// `cnt_time` in seconds (wi_stuff.c:329).
    pub cnt_time: i32,
    /// `cnt_par` in seconds (wi_stuff.c:330).
    pub cnt_par: i32,
    /// `snl_pointeron` before this frame (wi_stuff.c:749): false in a fresh process, written by
    /// `WI_updateShowNextLoc` (wi_stuff.c:768) and set by `WI_drawNoState` (wi_stuff.c:814).
    pub snl_pointeron: bool,
    /// `wbs->epsd` (0-based); only Episode 1 (0) is admitted.
    pub epsd: i32,
    /// `wbs->last` (0-based map number just finished).
    pub last: i32,
    /// `wbs->next` (0-based map number to enter).
    pub next: i32,
    /// `wbs->didsecret`.
    pub didsecret: bool,
}

/// A refused intermission frame. Nothing changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntermissionRefusal {
    /// A 2D draw the genuine code would make outside its memory (decision S17).
    Draw(DrawError),
    /// `wbs->epsd` other than 0: only Episode 1 is admitted (episodes 2 to 4 are a later
    /// milestone, to be named by the owner).
    Episode(i32),
    /// `wbs->last` or `wbs->next` outside 0..=8: `lnames[]` holds nine entries
    /// (wi_stuff.c:1585-1591), and `lnodes[epsd][]` nine (wi_stuff.c:177-203).
    Map(&'static str),
    /// The splat does not fit at map node `node`: `WI_drawOnLnode` would then test
    /// `c[1]` (wi_stuff.c:485-487), the word after the static `splat`, which is not a patch the
    /// code owns (wi_stuff.c:348, 794, 798).
    SplatDoesNotFit {
        /// The map node (0-based).
        node: usize,
    },
}

impl From<DrawError> for IntermissionRefusal {
    fn from(e: DrawError) -> Self {
        Self::Draw(e)
    }
}

impl std::fmt::Display for IntermissionRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "intermission frame refused: {self:?}")
    }
}
impl std::error::Error for IntermissionRefusal {}

/// One drawn intermission frame beside its pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntermissionFrame {
    /// `snl_pointeron` after the frame: set by `WI_drawNoState` (wi_stuff.c:814), else as given.
    pub snl_pointeron: bool,
}

/// `lnodes[0]` (wi_stuff.c:179-190): the map positions of Episode 1.
const LNODES: [[i32; 2]; 9] = [
    [185, 164],
    [148, 143],
    [69, 122],
    [209, 102],
    [116, 89],
    [166, 55],
    [71, 56],
    [135, 29],
    [71, 24],
];
/// `WI_TITLEY` (wi_stuff.c:80).
const TITLEY: i32 = 2;
/// `SP_STATSX`, `SP_STATSY` (wi_stuff.c:84-85).
const SP_STATSX: i32 = 50;
const SP_STATSY: i32 = 50;
/// `SP_TIMEX`, `SP_TIMEY` (wi_stuff.c:87-88).
const SP_TIMEX: i32 = 16;
const SP_TIMEY: i32 = SCREENHEIGHT as i32 - 32;
/// The animation frames of Episode 1: ten animations of three frames each (wi_stuff.c:226-238).
const ANIMS: usize = 10;

/// The intermission graphics of Episode 1, resolved as `WI_loadData` resolves them
/// (wi_stuff.c:1538-1706, in its order; `W_CacheLumpName` is `W_GetNumForName`: the last match
/// wins and a missing name is `I_Error`, w_wad.c:399-409), and the background buffer.
///
/// Every one of the 79 names must exist, also those single player never draws. The pictures
/// the single-player drawer can draw are validated as patches (header and column directory in
/// the lump; posts when drawn), a conservative refusal where the genuine code would read past
/// the lump; the pictures only cached (the animation frames, `WIOSTS`, `WIFRGS`, `WIKILRS`,
/// `WIVCTMS`, `WIMSTT`, `STFST01`, `STFDEAD0`, `STPB0`-`3`, `WIBP1`-`4`) need only exist, as in
/// the C, which never reads them in single player.
#[derive(Clone, Debug)]
pub struct IntermissionGraphics {
    lnames: [Patch; 9],
    yah: [Patch; 2],
    splat: Patch,
    wiminus: Patch,
    num: [Patch; 10],
    percent: Patch,
    finished: Patch,
    entering: Patch,
    sp_secret: Patch,
    kills: Patch,
    items: Patch,
    colon: Patch,
    time: Patch,
    sucks: Patch,
    par: Patch,
    background: Vec<u8>,
}

/// A refused graphics load (`WI_Start`'s `WI_loadData`): the genuine engine stops at the first
/// missing name, at the intermission's first tic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntermissionLoadError {
    /// A name is missing or a drawn picture is not a valid patch.
    Resource(ResourceError),
    /// Drawing the background into `screens[1]` (wi_stuff.c:1558) would leave its memory.
    Background(DrawError),
    /// Only Episode 1 (`wbs->epsd` 0) is admitted.
    Episode(i32),
}

impl std::fmt::Display for IntermissionLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "intermission graphics refused: {self:?}")
    }
}
impl std::error::Error for IntermissionLoadError {}

impl IntermissionGraphics {
    /// `WI_loadData` for episode `epsd` (0-based; only 0 is admitted) of a Doom 1 game mode:
    /// `WIMAP0` (wi_stuff.c:1548; the retail `INTERPIC` switch is for episode 4 only,
    /// wi_stuff.c:1550-1554), drawn over the zeroed `screens[1]`, then every name in order.
    pub fn load(res: &Resources, epsd: i32) -> Result<Self, IntermissionLoadError> {
        if epsd != 0 {
            return Err(IntermissionLoadError::Episode(epsd));
        }
        let r = IntermissionLoadError::Resource;
        let p = |n: &str| patch_named(res, n).map_err(r);
        let exists = |n: &str| {
            res.directory()
                .get_num_for_name(n.as_bytes())
                .map(|_| ())
                .map_err(r)
        };
        let bg = p("WIMAP0")?;
        let mut background = vec![0u8; SCREENWIDTH * SCREENHEIGHT];
        // V_DrawPatch(0, 0, 1, bg) (wi_stuff.c:1558): screens[1] is a 320x200 buffer like
        // screens[0] (v_video.c:489-492), so the same drawer serves; its rows end at 200.
        let mut taint = vec![0u32; SCREENWIDTH * SCREENHEIGHT];
        let mut c = Canvas {
            res,
            fg: &mut background,
            taint: &mut taint,
            bg: &mut [],
            bg_defined: &mut [],
        };
        c.patch(0, 0, Screen::Fg, &bg)
            .map_err(IntermissionLoadError::Background)?;
        let lnames = (0..9)
            .map(|i| p(&format!("WILV0{i}")))
            .collect::<Result<Vec<_>, _>>()?;
        let yah = [p("WIURH0")?, p("WIURH1")?];
        let splat = p("WISPLAT")?;
        for j in 0..ANIMS {
            for i in 0..3 {
                exists(&format!("WIA0{j:02}{i:02}"))?;
            }
        }
        let wiminus = p("WIMINUS")?;
        let num = (0..10)
            .map(|i| p(&format!("WINUM{i}")))
            .collect::<Result<Vec<_>, _>>()?;
        let percent = p("WIPCNT")?;
        let finished = p("WIF")?;
        let entering = p("WIENTER")?;
        let kills = p("WIOSTK")?;
        exists("WIOSTS")?;
        let sp_secret = p("WISCRT2")?;
        // `french` is false (the French build was declined): WIOSTI (wi_stuff.c:1655-1663).
        let items = p("WIOSTI")?;
        exists("WIFRGS")?;
        let colon = p("WICOLON")?;
        let time = p("WITIME")?;
        let sucks = p("WISUCKS")?;
        let par = p("WIPAR")?;
        for n in ["WIKILRS", "WIVCTMS", "WIMSTT", "STFST01", "STFDEAD0"] {
            exists(n)?;
        }
        for i in 0..4 {
            exists(&format!("STPB{i}"))?;
            exists(&format!("WIBP{}", i + 1))?;
        }
        Ok(Self {
            lnames: lnames.try_into().expect("nine"),
            yah,
            splat,
            wiminus,
            num: num.try_into().expect("ten"),
            percent,
            finished,
            entering,
            sp_secret,
            kills,
            items,
            colon,
            time,
            sucks,
            par,
            background,
        })
    }

    /// `screens[1]` after `WI_loadData`: the background drawn over zeros (see the module
    /// documentation).
    pub fn background(&self) -> &[u8] {
        &self.background
    }
}

/// The drawing context of one frame: `screens[0]` and its (unused) taint.
struct Wi<'a> {
    c: Canvas<'a>,
    g: &'a IntermissionGraphics,
}

fn width(p: &Patch) -> i32 {
    i32::from(p.header.width)
}
fn height(p: &Patch) -> i32 {
    i32::from(p.header.height)
}

impl Wi<'_> {
    fn patch(&mut self, x: i32, y: i32, p: &Patch) -> Result<(), DrawError> {
        self.c.patch(x, y, Screen::Fg, p).map(|_| ())
    }

    /// `WI_drawLF` (wi_stuff.c:421-434): the finished level's name and "Finished!".
    fn draw_lf(&mut self, last: usize) -> Result<(), DrawError> {
        let name = self.g.lnames[last];
        let mut y = TITLEY;
        self.patch((SCREENWIDTH as i32 - width(&name)) / 2, y, &name)?;
        y += (5 * height(&name)) / 4;
        let f = self.g.finished;
        self.patch((SCREENWIDTH as i32 - width(&f)) / 2, y, &f)
    }

    /// `WI_drawEL` (wi_stuff.c:439-453): "Entering" and the next level's name.
    fn draw_el(&mut self, next: usize) -> Result<(), DrawError> {
        let e = self.g.entering;
        let mut y = TITLEY;
        self.patch((SCREENWIDTH as i32 - width(&e)) / 2, y, &e)?;
        let name = self.g.lnames[next];
        y += (5 * height(&name)) / 4;
        self.patch((SCREENWIDTH as i32 - width(&name)) / 2, y, &name)
    }

    /// `WI_drawOnLnode(n, c)` (wi_stuff.c:456-499): the first of `c` that fits strictly inside
    /// the screen at node `n`, else nothing (the C prints "Could not place patch"). `c` has the
    /// length the caller's array has; a one-element `c` that does not fit is refused (the C
    /// would test the word after it).
    fn draw_on_lnode(&mut self, n: usize, c: &[Patch]) -> Result<(), IntermissionRefusal> {
        let [x, y] = LNODES[n];
        let mut i = 0;
        loop {
            let Some(p) = c.get(i) else {
                return Err(IntermissionRefusal::SplatDoesNotFit { node: n });
            };
            let left = x - i32::from(p.header.leftoffset);
            let top = y - i32::from(p.header.topoffset);
            let right = left + width(p);
            let bottom = top + height(p);
            if left >= 0 && right < SCREENWIDTH as i32 && top >= 0 && bottom < SCREENHEIGHT as i32 {
                self.patch(x, y, p)?;
                return Ok(());
            }
            i += 1;
            if i == 2 {
                return Ok(());
            }
        }
    }

    /// `WI_drawNum` (wi_stuff.c:611-666): returns the new x. Single player never passes a
    /// negative number (both callers return first, wi_stuff.c:674-675, 697-698), but the
    /// minus sign is kept as written; arithmetic is 64-bit so no input wraps.
    fn draw_num(&mut self, mut x: i32, y: i32, n: i32, digits: i32) -> Result<i32, DrawError> {
        let fontwidth = width(&self.g.num[0]);
        let mut n = i64::from(n);
        let mut digits = digits;
        if digits < 0 {
            if n == 0 {
                digits = 1;
            } else {
                digits = 0;
                let mut temp = n;
                while temp != 0 {
                    temp /= 10;
                    digits += 1;
                }
            }
        }
        let neg = n < 0;
        if neg {
            n = -n;
        }
        if n == 1994 {
            return Ok(0);
        }
        while digits > 0 {
            digits -= 1;
            x -= fontwidth;
            let d = self.g.num[(n % 10) as usize];
            self.patch(x, y, &d)?;
            n /= 10;
        }
        if neg {
            x -= 8;
            let m = self.g.wiminus;
            self.patch(x, y, &m)?;
        }
        Ok(x)
    }

    /// `WI_drawPercent` (wi_stuff.c:668-679).
    fn draw_percent(&mut self, x: i32, y: i32, p: i32) -> Result<(), DrawError> {
        if p < 0 {
            return Ok(());
        }
        let pc = self.g.percent;
        self.patch(x, y, &pc)?;
        self.draw_num(x, y, p, -1).map(|_| ())
    }

    /// `WI_drawTime` (wi_stuff.c:687-721): minutes and seconds right-aligned at x, or "sucks"
    /// above 61*59 seconds.
    fn draw_time(&mut self, mut x: i32, y: i32, t: i32) -> Result<(), DrawError> {
        if t < 0 {
            return Ok(());
        }
        let colon = self.g.colon;
        if t <= 61 * 59 {
            let mut div = 1;
            loop {
                let n = (t / div) % 60;
                x = self.draw_num(x, y, n, 2)? - width(&colon);
                div *= 60;
                if div == 60 || t / div != 0 {
                    self.patch(x, y, &colon)?;
                }
                if t / div == 0 {
                    return Ok(());
                }
            }
        }
        let sucks = self.g.sucks;
        self.patch(x - width(&sucks), y, &sucks)
    }

    /// `WI_drawStats` (wi_stuff.c:1436-1468) after the slammed background.
    fn draw_stats(&mut self, v: &IntermissionView, last: usize) -> Result<(), DrawError> {
        let lh = (3 * height(&self.g.num[0])) / 2;
        self.draw_lf(last)?;
        let sw = SCREENWIDTH as i32;
        let g = self.g;
        self.patch(SP_STATSX, SP_STATSY, &g.kills)?;
        self.draw_percent(sw - SP_STATSX, SP_STATSY, v.cnt_kills)?;
        self.patch(SP_STATSX, SP_STATSY + lh, &g.items)?;
        self.draw_percent(sw - SP_STATSX, SP_STATSY + lh, v.cnt_items)?;
        self.patch(SP_STATSX, SP_STATSY + 2 * lh, &g.sp_secret)?;
        self.draw_percent(sw - SP_STATSX, SP_STATSY + 2 * lh, v.cnt_secret)?;
        self.patch(SP_TIMEX, SP_TIMEY, &g.time)?;
        self.draw_time(sw / 2 - SP_TIMEX, SP_TIMEY, v.cnt_time)?;
        // `wbs->epsd < 3` (wi_stuff.c:1462-1466): always true for Episode 1.
        self.patch(sw / 2 + SP_TIMEX, SP_TIMEY, &g.par)?;
        self.draw_time(sw - SP_TIMEX, SP_TIMEY, v.cnt_par)
    }

    /// `WI_drawShowNextLoc` (wi_stuff.c:771-810) after the slammed background, with the pointer
    /// state `pointer` (`snl_pointeron`).
    fn draw_show_next_loc(
        &mut self,
        v: &IntermissionView,
        next: usize,
        pointer: bool,
    ) -> Result<(), IntermissionRefusal> {
        // `gamemode != commercial` and `wbs->epsd <= 2` (wi_stuff.c:782-788).
        let last = if v.last == 8 { v.next - 1 } else { v.last };
        let splat = [self.g.splat];
        for i in 0..=last {
            self.draw_on_lnode(i as usize, &splat)?;
        }
        if v.didsecret {
            self.draw_on_lnode(8, &splat)?;
        }
        if pointer {
            let yah = self.g.yah;
            self.draw_on_lnode(next, &yah)?;
        }
        self.draw_el(next)?;
        Ok(())
    }
}

/// `WI_Drawer` (wi_stuff.c:1772-1793) for single player into `screen` (`screens[0]`, 320x200,
/// overwritten whole by `WI_slamBackground`, wi_stuff.c:406-410), with the graphics `g` loaded
/// from `res`. On a refusal `screen` is unspecified and nothing else changes.
pub fn draw(
    res: &Resources,
    g: &IntermissionGraphics,
    v: &IntermissionView,
    screen: &mut [u8],
) -> Result<IntermissionFrame, IntermissionRefusal> {
    if v.epsd != 0 {
        return Err(IntermissionRefusal::Episode(v.epsd));
    }
    let map = |m: i32, name| {
        usize::try_from(m)
            .ok()
            .filter(|m| *m < 9)
            .ok_or(IntermissionRefusal::Map(name))
    };
    let last = map(v.last, "last")?;
    let next = map(v.next, "next")?;
    // WI_slamBackground (wi_stuff.c:408); WI_drawAnimatedBack draws nothing (wi_stuff.c:588).
    screen.copy_from_slice(&g.background);
    let mut taint = vec![0u32; SCREENWIDTH * SCREENHEIGHT];
    let mut wi = Wi {
        c: Canvas {
            res,
            fg: screen,
            taint: &mut taint,
            bg: &mut [],
            bg_defined: &mut [],
        },
        g,
    };
    let pointer = match v.state {
        WiState::StatCount => {
            wi.draw_stats(v, last)?;
            v.snl_pointeron
        }
        WiState::ShowNextLoc => {
            wi.draw_show_next_loc(v, next, v.snl_pointeron)?;
            v.snl_pointeron
        }
        WiState::NoState => {
            // WI_drawNoState (wi_stuff.c:812-816): the pointer on, then ShowNextLoc's drawing.
            wi.draw_show_next_loc(v, next, true)?;
            true
        }
    };
    Ok(IntermissionFrame {
        snl_pointeron: pointer,
    })
}
