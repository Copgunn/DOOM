#!/usr/bin/env python3
"""Bounded public level-policy (wire 22) operation inputs for fuzz_targets/player_level.rs; no Rust-generated expectations."""
import argparse
from pathlib import Path

# Input layout (see the target's module documentation):
#   byte 0  one of the 26 M3h9b scenario starts (room, player position), the rows
#           of tests/reference/m3h9b_cases.csv;
#   byte 1  0 for the case's own seed, inventory and reaction time, otherwise the
#           RNG index of a start with a full chaingun inventory;
#   byte 2  the lump index of the inserted type-1 THINGS record (% (records + 1));
#   bytes 3-4  that record's angle, little-endian i16;
#   byte 5  the number of nine-byte records that follow (% 41);
#   then the records: b0 operation (b0 % 16), b1 slot or selector, b2 forward,
#   b3 side, b4-5 angle_turn, b6-7 candidate handle, b8 buttons; damage, kills,
#   pushes and momz read their operands from b3-b5 and b8 (see the target);
#   then a version-110 demo (13 header bytes, 4-byte ticcmds, the 0x80 marker)
#   whose commands op 8 plays.
STARTS = 26
RNGS = [0, 6]
HEADER = 6
RECORDS = 40
DEMO_TICS = 256
MAX_LEN = HEADER + 9 * RECORDS + 13 + 4 * DEMO_TICS + 1
DEMO_HEADER = bytes([110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0])
# (lump index byte, angle) pairs cycled over the seeds: the record first, mid-lump
# and last; octant-zero, non-octant and negative angles.
STARTS_AT = [(0, 0), (3, 135), (255, -90), (1, 359), (2, 44), (255, 270)]
BT_ATTACK, BT_USE, BT_CHANGE = 1, 2, 4


def tic(forward=0, side=0, turn=0, buttons=0, slot=0):
    t = turn.to_bytes(2, 'little', signed=True)
    return bytes([0, slot, forward & 0xff, side & 0xff, t[0], t[1], 0, 1, buttons])


def run(tics, forward=0, side=0, turn=0, buttons=0):
    """A long run (op 4) of `tics` (1..256) tics of one held slot-One command; b8
    bit 0x40 keeps BT_USE once the player is dead (the reborn request)."""
    assert 1 <= tics <= 256
    t = turn.to_bytes(2, 'little', signed=True)
    return bytes([4, tics - 1, forward & 0xff, side & 0xff, t[0], t[1], 0, 1, buttons])


def damage(target, amount, source=0, inflictor=0, index=0, who=0):
    """Op 5: target 0 body, 1 monster, 2 decoration, 3 pickup, 4 barrel, 5 candidate;
    source/inflictor 0 none, 1 monster, 2 barrel, 3 body (`who` indexes them)."""
    a = amount.to_bytes(2, 'little', signed=True)
    return bytes([5, target, index, source | inflictor << 2, who, a[0], 0, 1, a[1]])


def kill(source=1, who=0, inflictor=False, extra=0):
    """Op 6: 1,000 + `extra` damage to the body; source 0 none, 1 monster `who`,
    2 the body itself; a monster inflictor when `inflictor`."""
    return bytes([6, 0, 0, source | (4 if inflictor else 0), who, extra, 0, 1, 0])


def play(tics):
    """Op 8: play the demo tail's next `tics` (1..256) commands."""
    assert 1 <= tics <= 256
    return bytes([8, tics - 1, 0, 0, 0, 0, 0, 1, 0])


def op(code, b1=0, b2=0, b3=0, b4=0, b5=0, b8=0, handle=(0, 1)):
    return bytes([code, b1, b2, b3, b4, b5, handle[0], handle[1], b8])


def demo(commands):
    """A version-110 demo of (forward, side, turn high byte, buttons) commands."""
    assert len(commands) <= DEMO_TICS
    out = bytearray(DEMO_HEADER)
    for f, s, t, b in commands:
        out += bytes([f & 0xff, s & 0xff, t & 0xff, b])
    out.append(0x80)
    return bytes(out)


IDLE, FIRE = run(256), run(256, buttons=BT_ATTACK)
WALK = [(25, 0, 0, 0)] * 32 + [(0, 0, 4, BT_ATTACK)] * 32 + [(-25, 10, 0, BT_USE)] * 16 + [(0, 0, 0, 0)] * 48
BUSY = [(50, -50, 8, BT_ATTACK | BT_CHANGE | (2 << 3))] * 8 + [(10, 0, 0, BT_ATTACK)] * 8
REBORN = BUSY + [(0, 0, 0, BT_USE)] + [(0, 0, 0, 0)] * 7
ODD = [(100, 0, 0, 0), (0, 0, 0, 0x40), (-127, 127, 0x80, 0x80), (0, 0, 0, 0)] * 4


def patterns():
    """(records, demo tail) pairs."""
    out = [([], b''), ([IDLE], b''), ([FIRE, IDLE], b''), ([run(128, 25, 0, 0, 1), IDLE], b''),
           ([run(64, 25), run(200, buttons=BT_USE)], b''), ([run(200, 0, 25, 1024, 1), run(120)], b'')]
    # Damage to the live player, then the one-per-tic decay; repeated damage.
    for amount, source in [(7, 0), (30, 1), (60, 3)]:
        out.append(([damage(0, amount, source, source), run(120)], b''))
    out.append(([damage(0, 5), tic(), damage(0, 9, 1, 1), run(16), damage(0, 40), run(64)], b''))
    # Kill the player from each source kind, then P_DeathThink: the turn toward a
    # monster attacker, the self and the sourceless death (decay only).
    for source, who, inf in [(1, 0, False), (1, 1, True), (1, 2, False), (0, 0, False), (2, 0, False)]:
        out.append(([kill(source, who, inf), run(64), IDLE], b''))
    # A dead player takes any admitted command; BT_USE then requests the reborn,
    # whose next tic is refused (state kept).
    out.append(([kill(), tic(50, -50, 2048, BT_ATTACK | BT_CHANGE | (2 << 3)), tic(10, 0, 0, BT_ATTACK), run(32, 25, 0, 512, 1),
                 tic(0, 0, 0, BT_USE), run(16)], b''))
    out.append(([kill(1, 1), run(48, 0, 0, 0, BT_USE | 0x40), run(8)], b''))
    # The attacker removed after the death: the next death turn is refused.
    out.append(([kill(1, 0), run(4), op(12, 2), run(16)], b''))
    out.append(([op(12, 2), kill(1, 0), op(12, 1), run(16)], b''))
    # Pause around a death turn.
    out.append(([kill(1, 0), run(3), op(7, 1), run(8), op(7, 0), run(32)], b''))
    # Monsters hunting the player (retarget at the body), so damage and death come
    # from play; then pushes and vertical momentum on the body.
    out.append(([op(10, 0, 0), op(10, 1, 0), op(10, 2, 0), FIRE, IDLE, IDLE], b''))
    out.append(([op(13, 0, 0, 20, 0, 5, 0), run(32), op(9, 0, 0, 0, 0, 8, 0), run(64), kill(1, 0), op(13, 0, 0, 30, 0, 0, 0), run(64)], b''))
    # The spawns the level policy refuses.
    out.append(([op(11, 0, 0, 0, 0, 0), op(11, 1), op(11, 2, 0x40, 0, 0, 0), op(12, 3), run(32)], b''))
    # Demo playback: a walk, a dead player's busy commands then BT_USE, commands
    # outside the admitted domain, an empty demo and a malformed tail.
    out.append(([play(128), IDLE], demo(WALK)))
    out.append(([kill(1, 0), play(16), play(16), run(8)], demo(REBORN)))
    out.append(([play(16), kill(), play(16)], demo(ODD + ODD)))
    out.append(([play(4), IDLE], demo([])))
    out.append(([play(8)], demo(WALK)[:-3]))
    out.append(([play(200), play(56), FIRE], demo([(25, 0, (i * 7) & 0xff, i & 3) for i in range(DEMO_TICS)])))
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    seeds = []
    k = 0
    for start in range(STARTS):
        for rng in RNGS:
            for records, tail in patterns():
                at, angle = STARTS_AT[k % len(STARTS_AT)]
                k += 1
                header = bytes([start, rng, at]) + angle.to_bytes(2, 'little', signed=True) + bytes([len(records)])
                assert len(records) <= RECORDS
                seeds.append(header + b''.join(records) + tail)
    assert all(len(s) <= MAX_LEN for s in seeds)
    for i, data in enumerate(seeds):
        (args.directory / f'level-{i:04d}').write_bytes(data)
    print(f'{len(seeds)} level-policy, dead-player and demo operation seeds (max_len {MAX_LEN})')


if __name__ == '__main__':
    main()
