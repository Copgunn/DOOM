//! The Doom 1 Episode 1 finale drawer of f_finale.c (M6a note §1.8): `F_Drawer`
//! (f_finale.c:700-736) for stage 0, the text crawl (`F_TextWrite`, f_finale.c:261-327), and
//! stage 1, the art screen (f_finale.c:712-721).
//!
//! **Reached only by the finale-and-melt session display** (`SessionConfig::FinaleAndMelt`,
//! `Display::draw_finale`); the frozen session display still refuses the finale
//! (`NotYet::FinaleDrawer`). Proven against the genuine engine's committed frames. It reads
//! only the explicit [`FinaleView`] (`finalestage` and `finalecount`, which the game layer's
//! finale logic owns) and the game mode, draws no random number (no `M_Random` or `P_Random`
//! call in f_finale.c's Doom 1 path) and writes nothing but `screens[0]`.
//!
//! **Persistent `screens[0]`.** Stage 0 overwrites the whole screen with the tiled flat
//! (f_finale.c:273-289) before drawing the text; stage 1 draws its patch over whatever
//! `screens[0]` holds (f_finale.c:716-720: a patch leaves its transparent pixels alone,
//! v_video.c:244-262), which in a session is the last text frame (the wipe ends with
//! `screens[0]` equal to the end screen, f_wipe.c:184-222). The caller passes the persistent
//! screen and its taint (decision R4) in and gets them back drawn.
//!
//! **What the genuine code would do with memory it does not own is refused, never drawn
//! differently** (decision S17): a flat shorter than the 4,096 bytes `F_TextWrite` copies, a
//! post outside its lump or below row 199 ([`DrawError`]), and a character whose font index is
//! `HU_FONTSIZE`, which `F_TextWrite`'s test `c > HU_FONTSIZE` lets through to read
//! `hu_font[63]`, one past the 63-entry array (f_finale.c:314, hu_stuff.c:90, hu_stuff.h:30-34;
//! the Episode 1 text has no such character, so this is never reached by [`draw`]). A name
//! `W_CacheLumpName` cannot find stops the genuine engine with
//! `W_GetNumForName: NAME not found!` (w_wad.c:399-409); here it is the typed refusal
//! [`DrawRefusal::Lump`]. In registered mode the art screen is `HELP2`, which the owner's
//! registered IWAD lacks (M6a note §1.5, §1.8), so the genuine engine stops on the first stage-1
//! frame; [`draw`] refuses that frame. On every refusal the screen and taint are unchanged.
use crate::r_data::ResourceError;
use crate::v_video::{Canvas, DrawError, Patch, Screen};
use crate::{GameMode, Resources, SCREENHEIGHT, SCREENWIDTH};

pub use crate::d_display::session::FinaleView;

/// `E1TEXT` (d_englsh.h:359-374; dstrings.h:37 includes the English strings, the French build
/// was declined), which `F_StartFinale` makes `finaletext` for Episode 1 (f_finale.c:118-121):
/// 440 characters, 69 of them outside the font (the spaces) and 15 newline characters (the text ends with one).
pub const E1TEXT: &[u8] = b"Once you beat the big badasses and\n\
clean out the moon base you're supposed\n\
to win, aren't you? Aren't you? Where's\n\
your fat reward and ticket home? What\n\
the hell is this? It's not supposed to\n\
end this way!\n\
\n\
It stinks like rotten meat, but looks\n\
like the lost Deimos base.  Looks like\n\
you're stuck on The Shores of Hell.\n\
The only way out is through.\n\
\n\
To continue the DOOM experience, play\n\
The Shores of Hell and its amazing\n\
sequel, Inferno!\n";

/// `finaleflat` for Episode 1 (f_finale.c:119).
pub const E1FLAT: &str = "FLOOR4_8";

/// `TEXTSPEED` (f_finale.c:56): tics per character of the crawl.
pub const TEXTSPEED: i32 = 3;

/// `HU_FONTSIZE` (hu_stuff.h:34): the 63 glyphs `'!'`..`'_'` of `hu_font` (hu_stuff.c:90).
const HU_FONTSIZE: i32 = 63;

/// The art screen of Episode 1's stage 1 (f_finale.c:714-720): `CREDIT` in retail mode, `HELP2`
/// in every other Doom 1 mode.
pub fn art_lump(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Retail => "CREDIT",
        GameMode::Registered | GameMode::Shareware => "HELP2",
    }
}

/// A refused finale frame. The screen and its taint are unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrawRefusal {
    /// `W_CacheLumpName` would stop the genuine engine (`W_GetNumForName: NAME not found!`,
    /// w_wad.c:399-409), or the art lump is not a patch whose header and column directory lie
    /// in the lump, or the flat is shorter than the 4,096 bytes `F_TextWrite` reads
    /// (f_finale.c:279-283).
    Lump(ResourceError),
    /// A 2D draw the genuine code would make outside its memory (decision S17).
    Draw(DrawError),
    /// `finalestage` other than 0 or 1: stage 2 is Doom II's cast (f_finale.c:702-706), never
    /// entered in Doom 1.
    Stage(i32),
    /// The font is not the 63 patches `HU_Init` loads (hu_stuff.c:404-410).
    Font(usize),
    /// A character whose font index is `HU_FONTSIZE` (`` '`' ``): `F_TextWrite`'s test
    /// `c > HU_FONTSIZE` (f_finale.c:314) lets it through to `hu_font[63]`, one past the array
    /// (hu_stuff.c:90). Not in the Episode 1 text.
    FontPastEnd,
    /// A byte outside ASCII: `toupper` of a negative `char` (f_finale.c:313) is not defined by
    /// the C standard. Not in the Episode 1 text.
    Character(u8),
    /// The screen or taint is not 320x200.
    Screen(usize),
}

impl From<DrawError> for DrawRefusal {
    fn from(e: DrawError) -> Self {
        Self::Draw(e)
    }
}

impl std::fmt::Display for DrawRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lump(ResourceError::LumpNotFound(name)) => {
                write!(f, "W_GetNumForName: {name} not found!")
            }
            e => write!(f, "finale frame refused: {e:?}"),
        }
    }
}
impl std::error::Error for DrawRefusal {}

/// `F_Drawer` (f_finale.c:700-736) for Doom 1 Episode 1 in game mode `mode`: stage 0 the text
/// crawl, stage 1 the art screen, into `screen` (`screens[0]`, 320x200) and its `taint`, which
/// hold the previous frame on entry. `font` is `hu_font` (the 63 patches of
/// [`crate::hu_stuff::load_font`]). On a refusal `screen` and `taint` are unchanged.
pub fn draw(
    res: &Resources,
    font: &[Patch],
    mode: GameMode,
    v: &FinaleView,
    screen: &mut [u8],
    taint: &mut [u32],
) -> Result<(), DrawRefusal> {
    const PIXELS: usize = SCREENWIDTH * SCREENHEIGHT;
    if screen.len() != PIXELS || taint.len() != PIXELS {
        return Err(DrawRefusal::Screen(screen.len().min(taint.len())));
    }
    // Drawn into copies and kept only on success, so that a refusal changes nothing.
    let (mut fg, mut t) = (screen.to_vec(), taint.to_vec());
    match v.stage {
        0 => text_write(res, font, E1TEXT, v.count, &mut fg, &mut t)?,
        1 => {
            // f_finale.c:714-720: W_CacheLumpName at every frame, then V_DrawPatch(0, 0, 0, ..).
            let name = art_lump(mode);
            let lump = res
                .directory()
                .get_num_for_name(name.as_bytes())
                .map_err(DrawRefusal::Lump)?;
            let p = Patch::load(res, lump)
                .ok_or(DrawRefusal::Lump(ResourceError::PatchBounds { lump }))?;
            canvas(res, &mut fg, &mut t).patch(0, 0, Screen::Fg, &p)?;
        }
        s => return Err(DrawRefusal::Stage(s)),
    }
    screen.copy_from_slice(&fg);
    taint.copy_from_slice(&t);
    Ok(())
}

fn canvas<'a>(res: &'a Resources, fg: &'a mut [u8], taint: &'a mut [u32]) -> Canvas<'a> {
    Canvas {
        res,
        fg,
        taint,
        bg: &mut [],
        bg_defined: &mut [],
    }
}

/// `F_TextWrite` (f_finale.c:261-327) with `finaletext` = `text` and `finalecount` = `count`.
fn text_write(
    res: &Resources,
    font: &[Patch],
    text: &[u8],
    count: i32,
    screen: &mut [u8],
    taint: &mut [u32],
) -> Result<(), DrawRefusal> {
    if font.len() != HU_FONTSIZE as usize {
        return Err(DrawRefusal::Font(font.len()));
    }
    // f_finale.c:274-289: the flat tiled, row y from row y & 63 of the flat, 64 bytes at a time
    // (SCREENWIDTH & 63 is 0, so the remainder copy at 284-288 never runs).
    let lump = res
        .directory()
        .get_num_for_name(E1FLAT.as_bytes())
        .map_err(DrawRefusal::Lump)?;
    let src = res.lump(lump);
    let flat = src
        .get(..64 * 64)
        .ok_or(DrawRefusal::Lump(ResourceError::LumpTooShort {
            name: E1FLAT.into(),
            needed: 64 * 64,
            length: src.len(),
        }))?;
    for (y, row) in screen.chunks_exact_mut(SCREENWIDTH).enumerate() {
        let from = &flat[(y & 63) << 6..((y & 63) << 6) + 64];
        for piece in row.chunks_exact_mut(64) {
            piece.copy_from_slice(from);
        }
    }
    taint.fill(0);
    let mut c = canvas(res, screen, taint);
    // f_finale.c:294-325.
    let (mut cx, mut cy) = (10i32, 10i32);
    // (finalecount - 10) / TEXTSPEED truncates toward zero, as C's division does; a count that
    // the subtraction would overflow is far below 10 and gives no character either way.
    let n = (count.saturating_sub(10) / TEXTSPEED).max(0);
    for &ch in text.iter().take(usize::try_from(n).unwrap_or(0)) {
        // f_finale.c:303-305: the string's NUL ends the crawl (the slice's end here).
        if ch == b'\n' {
            cx = 10;
            cy += 11;
            continue;
        }
        if !ch.is_ascii() {
            return Err(DrawRefusal::Character(ch));
        }
        // f_finale.c:313-318: toupper in the C locale, then the off-by-one range test.
        let c_idx = i32::from(ch.to_ascii_uppercase()) - i32::from(crate::hu_lib::FONTSTART);
        if !(0..=HU_FONTSIZE).contains(&c_idx) {
            cx += 4;
            continue;
        }
        let glyph = font
            .get(usize::try_from(c_idx).map_err(|_| DrawRefusal::FontPastEnd)?)
            .ok_or(DrawRefusal::FontPastEnd)?;
        // f_finale.c:320-324: one character too wide for the line ends the whole crawl.
        let w = i32::from(glyph.header.width);
        if cx + w > SCREENWIDTH as i32 {
            break;
        }
        c.patch(cx, cy, Screen::Fg, glyph)?;
        cx += w;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LumpDirectory;
    use std::path::Path;

    /// The committed synthetic finale set (`tests/reference/m6a_wads`, the manifest's `episode`
    /// WAD set; `m6a_gfx_nohelp2.wad` for `nohelp2`), with extra lumps appended.
    fn res(gfx: &str, extra: &[(&str, Vec<u8>)]) -> Resources {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
        let mut dir = LumpDirectory::new();
        for w in [
            "m6a_wads/episode.wad",
            "m5b_gfx.wad",
            "m5c_sfx.wad",
            gfx,
            "m6a_wads/m6a_music.wad",
        ] {
            let b = std::fs::read(root.join(w)).expect("WAD");
            let wad = doom_wad::Wad::parse(&b, doom_wad::Limits::default()).expect("WAD");
            dir.add_wad(&wad).expect("lumps");
        }
        for (n, b) in extra {
            dir.add_single_lump(n, b).expect("lump");
        }
        Resources::load(dir, GameMode::Registered).expect("resources")
    }

    fn screens() -> (Vec<u8>, Vec<u32>) {
        (vec![7u8; 64000], vec![9u32; 64000])
    }

    /// d_englsh.h:359-374: 440 characters, so stage 1 begins when `finalecount` exceeds
    /// 440 * 3 + 250 = 1570 (f_finale.c:241); 69 characters fall outside the font.
    #[test]
    fn the_episode_1_text_has_440_characters_and_69_gaps() {
        assert_eq!(E1TEXT.len(), 440);
        let gaps = E1TEXT
            .iter()
            .filter(|&&c| c != b'\n' && !(b'!'..=b'_').contains(&c.to_ascii_uppercase()))
            .count();
        assert_eq!(gaps, 69);
    }

    /// f_finale.c:714-720 and w_wad.c:399-409: registered mode without `HELP2` refuses the
    /// stage-1 frame with the genuine message and leaves the screen alone; retail draws
    /// `CREDIT`; stage 2 is refused.
    #[test]
    fn a_missing_art_lump_is_a_typed_refusal() {
        let r = res("m6a_wads/m6a_gfx_nohelp2.wad", &[]);
        let font = crate::hu_stuff::load_font(&r).expect("font");
        let (mut s, mut t) = screens();
        let v = FinaleView { stage: 1, count: 0 };
        let e = draw(&r, &font, GameMode::Registered, &v, &mut s, &mut t).unwrap_err();
        assert_eq!(
            e,
            DrawRefusal::Lump(ResourceError::LumpNotFound("HELP2".into()))
        );
        assert_eq!(e.to_string(), "W_GetNumForName: HELP2 not found!");
        assert_eq!((s, t), screens());
        let (mut s, mut t) = screens();
        assert_eq!(
            draw(&r, &font, GameMode::Retail, &v, &mut s, &mut t),
            Ok(())
        );
        let v = FinaleView { stage: 2, count: 0 };
        assert_eq!(
            draw(&r, &font, GameMode::Retail, &v, &mut s, &mut t),
            Err(DrawRefusal::Stage(2))
        );
    }

    /// f_finale.c:279-283: a flat shorter than 64x64 is refused (the C reads past it).
    #[test]
    fn a_short_flat_is_refused() {
        let r = res("m6a_wads/m6a_gfx.wad", &[("FLOOR4_8", vec![1u8; 4095])]);
        let font = crate::hu_stuff::load_font(&r).expect("font");
        let (mut s, mut t) = screens();
        let v = FinaleView { stage: 0, count: 0 };
        assert!(matches!(
            draw(&r, &font, GameMode::Retail, &v, &mut s, &mut t),
            Err(DrawRefusal::Lump(ResourceError::LumpTooShort {
                needed: 4096,
                length: 4095,
                ..
            }))
        ));
        assert_eq!((s, t), screens());
    }

    /// f_finale.c:274-325 on texts the Episode 1 path never draws: the tiled flat clears the
    /// taint; a space is a 4-pixel gap; a lower-case letter draws its capital; `{` is a gap;
    /// `` ` `` (index 63, past `hu_font`) is refused; a character that does not fit ends the
    /// whole crawl, the next line included; `(count - 10) / 3` characters are drawn.
    #[test]
    fn the_crawl_follows_f_textwrite() {
        let flat: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let r = res("m6a_wads/m6a_gfx.wad", &[("FLOOR4_8", flat.clone())]);
        let font = crate::hu_stuff::load_font(&r).expect("font");
        let run = |text: &[u8], count: i32| {
            let (mut s, mut t) = screens();
            text_write(&r, &font, text, count, &mut s, &mut t).map(|()| (s, t))
        };
        let (bare, taint) = run(b"", 1000).expect("flat");
        assert!(taint.iter().all(|&v| v == 0));
        for y in [0usize, 63, 64, 199] {
            for x in [0usize, 63, 64, 319] {
                assert_eq!(bare[y * 320 + x], flat[((y & 63) << 6) + (x & 63)]);
            }
        }
        let w = |c: u8| i32::from(font[usize::from(c - b'!')].header.width);
        // "a" and "A" draw the same; " A" is "A" moved 4 pixels; "{A" likewise.
        assert_eq!(run(b"a", 13), run(b"A", 13));
        let shifted = |prefix: &[u8]| {
            let mut t = prefix.to_vec();
            t.push(b'A');
            run(&t, 16).expect("drawn").0
        };
        assert_eq!(shifted(b" "), shifted(b"{"));
        assert_ne!(shifted(b" "), run(b"A", 13).expect("drawn").0);
        assert_eq!(run(b"`", 13), Err(DrawRefusal::FontPastEnd));
        assert_eq!(run(b"\x80", 13), Err(DrawRefusal::Character(0x80)));
        // (count - 10) / 3: 12 draws nothing, 13 one character, -20 nothing.
        assert_eq!(run(b"A", 12).expect("drawn").0, bare);
        assert_eq!(run(b"A", -20).expect("drawn").0, bare);
        assert_ne!(run(b"A", 13).expect("drawn").0, bare);
        // A line of capitals until one no longer fits ends everything after it.
        let n = usize::try_from((310 / w(b'M')) + 1).expect("count");
        let mut long = vec![b'M'; n];
        let fits = run(&long, 1000).expect("drawn").0;
        long.extend_from_slice(b"\nA");
        assert_eq!(run(&long, 1000).expect("drawn").0, fits);
    }
}
