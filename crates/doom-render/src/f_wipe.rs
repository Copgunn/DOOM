//! The melt of f_wipe.c (M6a note §1.11, §1.16): `wipe_initMelt` (f_wipe.c:140-169) and one
//! `wipe_doMelt` call with `ticks = 1` (f_wipe.c:171-224), the declared schedule of §1.16.1 (one
//! melt step per tic).
//!
//! **Reached by no configuration.** The session display still refuses a melt in progress
//! (`NotYet::MeltPictures`); this module is the pure step that a later configuration will call,
//! proven against the genuine engine's per-step picture digests.
//!
//! The genuine melt works on three buffers: the start screen (`wipe_scr_start`, `screens[2]`,
//! f_wipe.c:243-244), the end screen (`wipe_scr_end`, `screens[3]`, f_wipe.c:255-256) and the
//! picture (`wipe_scr`, which is `screens[0]`, f_wipe.c:284), which `wipe_initMelt` sets to the
//! start screen (f_wipe.c:149). The start and end screens are transposed to column-major order
//! as 160 x 200 `short`s (`wipe_shittyColMajorXform`, f_wipe.c:50-70, 153-154), an access
//! optimisation: each `short` is the pixel pair (2i, 2i + 1) of one row, so column pair `i` of
//! row `r` is bytes `2i` and `2i + 1` of row `r` in the row-major screen, which is how this
//! module reads them. Every move copies whole pairs, never single pixels.
//!
//! Integer semantics as in the C: the column positions are `int`s, -15..=0 at the start (the
//! clamp of f_wipe.c:164-165), a waiting column (`y < 0`) counts up by one per tic, a moving one
//! advances by `y + 1` while `y < 16` and by 8 after (f_wipe.c:198), cut to reach exactly 200
//! (f_wipe.c:199), and `done` is true only for a call in which no column waited or moved
//! (f_wipe.c:184, 194, 217). The genuine code forms a pointer one past the picture when a column
//! reaches 200 (f_wipe.c:208-212, a zero-length copy; quirk Q-f_wipe-210); this module forms no
//! pointer and copies nothing there.
use crate::{SCREENHEIGHT, SCREENWIDTH};
use doom_types::DoomRng;

/// The column pairs `wipe_doMelt` moves: `width /= 2` (f_wipe.c:186).
pub const MELT_COLUMNS: usize = SCREENWIDTH / 2;

/// The `M_Random` draws of `wipe_initMelt`: one for each `i < width`, width 320 (f_wipe.c:159-166),
/// although only the first 160 offsets are used (quirk Q-f_wipe-159a).
pub const MELT_DRAWS: usize = SCREENWIDTH;

const PIXELS: usize = SCREENWIDTH * SCREENHEIGHT;
const HEIGHT: i32 = SCREENHEIGHT as i32;

/// `wipe_initMelt`'s column offsets (f_wipe.c:158-166) drawn from `M_Random` after index
/// `m_random_index`: `y[0] = -(M_Random() % 16)`, then `y[i] = y[i-1] + (M_Random() % 3) - 1`
/// clamped to 0 above and from -16 to -15, for all 320 `i`. Returns the 160 offsets
/// `wipe_doMelt` uses and the `M_Random` index after the 320 draws.
pub fn melt_offsets(m_random_index: u8) -> ([i32; MELT_COLUMNS], u8) {
    let mut rng = DoomRng::from_indices(0, m_random_index);
    let mut y = [0i32; MELT_COLUMNS];
    let mut prev = -(i32::from(rng.m_random()) % 16);
    y[0] = prev;
    for i in 1..MELT_DRAWS {
        let r = i32::from(rng.m_random()) % 3 - 1;
        let mut v = prev + r;
        if v > 0 {
            v = 0;
        } else if v == -16 {
            v = -15;
        }
        if let Some(slot) = y.get_mut(i) {
            *slot = v;
        }
        prev = v;
    }
    (y, rng.indices()[1])
}

/// A refused melt. Nothing is built.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeltError {
    /// The start screen, end screen is not 64,000 palette indices: `which` names it, `len` is
    /// its length.
    Screen {
        /// `"start"` or `"end"`.
        which: &'static str,
        /// The length given.
        len: usize,
    },
    /// An initial column offset outside -15..=0, which `wipe_initMelt` cannot produce
    /// (f_wipe.c:159-165).
    Offset {
        /// The column pair.
        column: usize,
        /// The offset given.
        value: i32,
    },
}

impl std::fmt::Display for MeltError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "melt refused: {self:?}")
    }
}
impl std::error::Error for MeltError {}

/// One melt: the start and end screens, the picture and the 160 column positions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Melt {
    y: [i32; MELT_COLUMNS],
    start: Vec<u8>,
    end: Vec<u8>,
    picture: Vec<u8>,
}

impl Melt {
    /// `wipe_initMelt` (f_wipe.c:140-169) for a melt from `start` to `end` (64,000 palette
    /// indices each, row-major) whose 320 `M_Random` draws follow index `m_random_index`. Returns
    /// the melt, whose picture is the start screen (f_wipe.c:149), and the `M_Random` index after
    /// the draws.
    pub fn new(start: &[u8], end: &[u8], m_random_index: u8) -> Result<(Self, u8), MeltError> {
        let (offsets, after) = melt_offsets(m_random_index);
        Ok((Self::from_offsets(start, end, offsets)?, after))
    }

    /// A melt from `start` to `end` with the initial column offsets `offsets`, each -15..=0 as
    /// `wipe_initMelt` makes them ([`melt_offsets`]).
    pub fn from_offsets(
        start: &[u8],
        end: &[u8],
        offsets: [i32; MELT_COLUMNS],
    ) -> Result<Self, MeltError> {
        for (which, s) in [("start", start), ("end", end)] {
            if s.len() != PIXELS {
                return Err(MeltError::Screen {
                    which,
                    len: s.len(),
                });
            }
        }
        if let Some((column, &value)) = offsets
            .iter()
            .enumerate()
            .find(|(_, v)| !(-15..=0).contains(*v))
        {
            return Err(MeltError::Offset { column, value });
        }
        Ok(Self {
            y: offsets,
            start: start.to_vec(),
            end: end.to_vec(),
            picture: start.to_vec(),
        })
    }

    /// One `wipe_doMelt` call with `ticks = 1` (f_wipe.c:184-222): every column pair waits,
    /// moves or stays, and the picture is updated. Returns `done`: true when no column waited or
    /// moved in this call, the first call after every column reached 200 (from then on every
    /// call is true and changes nothing).
    pub fn step(&mut self) -> bool {
        let mut done = true;
        for (i, y) in self.y.iter_mut().enumerate() {
            if *y < 0 {
                *y += 1;
                done = false;
            } else if *y < HEIGHT {
                let mut dy = if *y < 16 { *y + 1 } else { 8 };
                if *y + dy >= HEIGHT {
                    dy = HEIGHT - *y;
                }
                // f_wipe.c:200-207: end-screen rows y..y+dy of the pair to the same rows.
                for row in *y..*y + dy {
                    copy_pair(&mut self.picture, row, &self.end, row, i);
                }
                *y += dy;
                // f_wipe.c:209-216: start-screen rows 0..200-y of the pair to rows y.. .
                for k in 0..HEIGHT - *y {
                    copy_pair(&mut self.picture, *y + k, &self.start, k, i);
                }
                done = false;
            }
        }
        done
    }

    /// The picture (`screens[0]`, 64,000 palette indices, row-major).
    pub fn picture(&self) -> &[u8] {
        &self.picture
    }

    /// The 160 column positions (`y[]`, f_wipe.c:138): negative while waiting, then 0..=200.
    pub fn columns(&self) -> &[i32; MELT_COLUMNS] {
        &self.y
    }
}

/// Copies the `short` of column pair `pair` from row `src_row` of `src` to row `dst_row` of
/// `dst` (rows 0..200, guaranteed by [`Melt::step`]).
fn copy_pair(dst: &mut [u8], dst_row: i32, src: &[u8], src_row: i32, pair: usize) {
    let at = |row: i32| usize::try_from(row).unwrap_or(0) * SCREENWIDTH + 2 * pair;
    let (d, s) = (at(dst_row), at(src_row));
    dst[d..d + 2].copy_from_slice(&src[s..s + 2]);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// f_wipe.c:158-166 from rndtable (m_random.c): index 0's first draw is rndtable[1] = 8, so
    /// y[0] = -8; every offset is in -15..=0, neighbours differ by at most one, and the index
    /// moves by 320 mod 256.
    #[test]
    fn the_offsets_follow_wipe_init_melt() {
        let (y, after) = melt_offsets(0);
        assert_eq!(y[0], -8);
        assert_eq!(after, 64);
        for idx in 0..=255u8 {
            let (y, after) = melt_offsets(idx);
            assert_eq!(after, idx.wrapping_add(64));
            assert!(y.iter().all(|v| (-15..=0).contains(v)));
            assert!(y.windows(2).all(|w| (w[0] - w[1]).abs() <= 1));
        }
    }

    /// f_wipe.c:192-216 and §1.16.1: N = 28 + max(-y0) calls, the last one done, the last
    /// two pictures equal to the end screen; a column pair moves as one; after step k a
    /// moving column shows end rows above its position and the start screen pushed down.
    #[test]
    fn a_melt_ends_on_the_end_screen_after_28_plus_the_longest_wait() {
        let start: Vec<u8> = (0..PIXELS).map(|i| (i % 253) as u8).collect();
        let end: Vec<u8> = (0..PIXELS).map(|i| (i % 241) as u8 ^ 0x55).collect();
        let mut offsets = [0i32; MELT_COLUMNS];
        offsets[7] = -15;
        offsets[8] = -3;
        let mut m = Melt::from_offsets(&start, &end, offsets).expect("melt");
        assert_eq!(m.picture(), &start[..]);
        let mut steps = 0;
        let mut pictures = Vec::new();
        loop {
            steps += 1;
            let done = m.step();
            pictures.push(m.picture().to_vec());
            if steps == 1 {
                // Column 0 moved 1 row; column 7 still waits.
                assert_eq!(m.columns()[0], 1);
                assert_eq!(m.columns()[7], -14);
                assert_eq!(m.picture()[..2], end[..2]);
                assert_eq!(m.picture()[320..322], start[..2]);
                assert_eq!(m.picture()[14..16], start[14..16]);
            }
            if done {
                break;
            }
        }
        assert_eq!(steps, 28 + 15);
        let n = pictures.len();
        assert_eq!(pictures[n - 1], end);
        assert_eq!(pictures[n - 2], end);
        assert_ne!(pictures[n - 3], end);
        // Positions 1, 3, 7, 15, 31, then +8 to 199, then 200 (27 moves).
        let mut m = Melt::from_offsets(&start, &end, [0; MELT_COLUMNS]).expect("melt");
        let mut seen = Vec::new();
        while !m.step() {
            seen.push(m.columns()[0]);
        }
        let mut want = vec![1, 3, 7, 15, 31];
        want.extend((1..=21).map(|k| 31 + 8 * k));
        want.push(200);
        assert_eq!(seen, want);
    }

    #[test]
    fn bad_screens_and_offsets_are_refused() {
        let s = vec![0u8; PIXELS];
        assert_eq!(
            Melt::new(&s[1..], &s, 0).unwrap_err(),
            MeltError::Screen {
                which: "start",
                len: PIXELS - 1
            }
        );
        let mut o = [0i32; MELT_COLUMNS];
        o[3] = -16;
        assert_eq!(
            Melt::from_offsets(&s, &s, o).unwrap_err(),
            MeltError::Offset {
                column: 3,
                value: -16
            }
        );
    }
}
