//! The version-110 demo (`.lmp`) format of linuxdoom-1.10, restricted to the one
//! game the level policy runs: medium skill, episode 1 map 1, single player.
//!
//! Layout (`G_DoPlayDemo`, g_game.c:1582–1620; `G_BeginRecording`, 1549–1567):
//! thirteen header bytes (version, skill, episode, map, deathmatch, respawn,
//! fast, nomonsters, consoleplayer, then `playeringame[0..4]`), then one 4-byte
//! ticcmd per tic and player in the game (`G_ReadDemoTiccmd`, 1491–1503):
//! `forwardmove` as a signed byte, `sidemove` as a signed byte, `angleturn` as
//! its high byte (`(unsigned char)byte << 8`), `buttons` as an unsigned byte.
//! The stream ends at `DEMOMARKER` 0x80 (1488), tested on the byte where the next
//! `forwardmove` would be (1493) before any field is read.
use crate::TicCommand;
use std::{error::Error, fmt};

/// g_game.c:1488 `DEMOMARKER`.
pub const DEMO_MARKER: u8 = 0x80;
/// doomdef.h `VERSION` 110, the first header byte (g_game.c:1589).
pub const DEMO_VERSION: u8 = 110;
/// The header this reader admits and [`write_demo`] writes: version 110, skill 2
/// (`sk_medium`), episode 1, map 1, deathmatch 0, respawn 0, fast 0,
/// nomonsters 0, consoleplayer 0, `playeringame` `[1, 0, 0, 0]`.
pub const DEMO_HEADER: [u8; 13] = [DEMO_VERSION, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0];

/// A parsed demo: one command per tic for player 1.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Demo {
    /// Player 1's ticcmds in tic order, before the marker.
    pub commands: Vec<TicCommand>,
}

/// Why [`parse_demo`] or [`write_demo`] refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoError {
    /// Fewer than the thirteen header bytes.
    Truncated,
    /// Header byte `index` (0 version, 1 skill, 2 episode, 3 map, 4 deathmatch,
    /// 5 respawn, 6 fast, 7 nomonsters, 8 consoleplayer, 9–12 `playeringame`)
    /// differs from [`DEMO_HEADER`].
    Header {
        /// The header byte's offset.
        index: usize,
        /// The byte found.
        value: u8,
    },
    /// The stream ended inside a ticcmd, or without the marker.
    UnterminatedCommands,
    /// Bytes follow the marker.
    TrailingBytes,
    /// [`write_demo`]: tic `tic`'s `forward` is −128, whose byte 0x80 a reader
    /// would take for the marker.
    MarkerForward {
        /// The tic's index.
        tic: usize,
    },
    /// [`write_demo`]: tic `tic`'s buttons carry bit 0x40 or `BT_SPECIAL` 0x80,
    /// outside the admitted command domain.
    Buttons {
        /// The tic's index.
        tic: usize,
    },
    /// [`write_demo`]: tic `tic`'s `angle_turn` has a nonzero low byte, which the
    /// format cannot carry (only `angleturn >> 8` is stored).
    AngleTurn {
        /// The tic's index.
        tic: usize,
    },
}
impl fmt::Display for DemoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "demo: {self:?}")
    }
}
impl Error for DemoError {}

/// Read a version-110 demo with exactly [`DEMO_HEADER`], then player 1's ticcmds
/// up to the marker, as `G_ReadDemoTiccmd` reads them (g_game.c:1491–1503):
/// a `forwardmove` byte of 0x80 is the marker, any other value starts a command.
/// Nothing may follow the marker.
pub fn parse_demo(bytes: &[u8]) -> Result<Demo, DemoError> {
    let header = bytes.get(..DEMO_HEADER.len()).ok_or(DemoError::Truncated)?;
    if let Some((index, &value)) = header
        .iter()
        .enumerate()
        .find(|(i, b)| **b != DEMO_HEADER[*i])
    {
        return Err(DemoError::Header { index, value });
    }
    let mut rest = &bytes[DEMO_HEADER.len()..];
    let mut commands = Vec::new();
    loop {
        match rest {
            [DEMO_MARKER, tail @ ..] => {
                return if tail.is_empty() {
                    Ok(Demo { commands })
                } else {
                    Err(DemoError::TrailingBytes)
                };
            }
            [forward, side, turn, buttons, tail @ ..] => {
                commands.push(TicCommand {
                    forward: *forward as i8,
                    side: *side as i8,
                    angle_turn: (u16::from(*turn) << 8) as i16,
                    buttons: *buttons,
                });
                rest = tail;
            }
            _ => return Err(DemoError::UnterminatedCommands),
        }
    }
}

/// Write `commands` as a version-110 demo with [`DEMO_HEADER`] and the marker.
/// Refuses a `forward` of −128 (its byte is the marker), buttons with bit 0x40
/// or 0x80, and an `angle_turn` with a nonzero low byte (not representable).
pub fn write_demo(commands: &[TicCommand]) -> Result<Vec<u8>, DemoError> {
    let mut out = Vec::with_capacity(DEMO_HEADER.len() + 4 * commands.len() + 1);
    out.extend_from_slice(&DEMO_HEADER);
    for (tic, c) in commands.iter().enumerate() {
        if c.forward == i8::MIN {
            return Err(DemoError::MarkerForward { tic });
        }
        if c.buttons & 0xc0 != 0 {
            return Err(DemoError::Buttons { tic });
        }
        if c.angle_turn & 0xff != 0 {
            return Err(DemoError::AngleTurn { tic });
        }
        let [turn, _] = c.angle_turn.to_be_bytes();
        out.extend([c.forward as u8, c.side as u8, turn, c.buttons]);
    }
    out.push(DEMO_MARKER);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(forward: i8, side: i8, angle_turn: i16, buttons: u8) -> TicCommand {
        TicCommand {
            forward,
            side,
            angle_turn,
            buttons,
        }
    }

    #[test]
    fn round_trip_and_field_mapping() {
        let commands = vec![
            cmd(0, 0, 0, 0),
            cmd(50, -50, 0x0100, 1),
            cmd(-127, 127, -256, 2 | 4 | (3 << 3)),
            cmd(25, 0, i16::MIN, 0),
            cmd(0, 24, 0x7f00, 0x3f),
        ];
        let bytes = write_demo(&commands).unwrap();
        assert_eq!(bytes.len(), 13 + 4 * commands.len() + 1);
        assert_eq!(&bytes[..13], &DEMO_HEADER);
        assert_eq!(&bytes[17..21], &[50, 0xce, 0x01, 1]);
        assert_eq!(bytes[21 + 2], 0xff);
        assert_eq!(*bytes.last().unwrap(), DEMO_MARKER);
        assert_eq!(parse_demo(&bytes).unwrap().commands, commands);
        // Empty demo: header and marker.
        let empty = write_demo(&[]).unwrap();
        assert_eq!(empty.len(), 14);
        assert!(parse_demo(&empty).unwrap().commands.is_empty());
    }

    #[test]
    fn each_header_byte_is_checked() {
        let good = write_demo(&[cmd(1, 0, 0, 0)]).unwrap();
        for index in 0..13 {
            let mut bad = good.clone();
            bad[index] ^= 0x01;
            assert_eq!(
                parse_demo(&bad),
                Err(DemoError::Header {
                    index,
                    value: bad[index]
                })
            );
        }
        assert_eq!(parse_demo(&good[..12]), Err(DemoError::Truncated));
    }

    #[test]
    fn marker_and_stream_errors() {
        let mut bytes = write_demo(&[cmd(1, 2, 0, 0)]).unwrap();
        // The marker ends the stream where a forwardmove would be read.
        let mut early = DEMO_HEADER.to_vec();
        early.push(DEMO_MARKER);
        assert!(parse_demo(&early).unwrap().commands.is_empty());
        // A 0x80 side, turn or buttons byte is data, not a marker.
        let mut inner = DEMO_HEADER.to_vec();
        inner.extend([0, 0x80, 0x80, 0, DEMO_MARKER]);
        assert_eq!(
            parse_demo(&inner).unwrap().commands,
            vec![cmd(0, -128, i16::MIN, 0)]
        );
        bytes.pop();
        assert_eq!(parse_demo(&bytes), Err(DemoError::UnterminatedCommands));
        bytes.push(1);
        assert_eq!(parse_demo(&bytes), Err(DemoError::UnterminatedCommands));
        bytes.pop();
        bytes.extend([DEMO_MARKER, 0]);
        assert_eq!(parse_demo(&bytes), Err(DemoError::TrailingBytes));
    }

    #[test]
    fn writer_refusals() {
        assert_eq!(
            write_demo(&[cmd(0, 0, 0, 0), cmd(-128, 0, 0, 0)]),
            Err(DemoError::MarkerForward { tic: 1 })
        );
        assert_eq!(
            write_demo(&[cmd(0, 0, 0, 0x40)]),
            Err(DemoError::Buttons { tic: 0 })
        );
        assert_eq!(
            write_demo(&[cmd(0, 0, 0, 0x80)]),
            Err(DemoError::Buttons { tic: 0 })
        );
        assert_eq!(
            write_demo(&[cmd(0, 0, 1, 0)]),
            Err(DemoError::AngleTurn { tic: 0 })
        );
        assert!(write_demo(&[cmd(-127, -128, 0, 0x3f)]).is_ok());
    }
}
