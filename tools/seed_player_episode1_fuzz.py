#!/usr/bin/env python3
"""Bounded Episode 1 (wire 23) command-stream inputs for fuzz_targets/player_episode1.rs; no Rust-generated expectations."""
import argparse
from pathlib import Path

# Input layout (see the target's module documentation):
#   byte 0  the WAD (% 9), in the order of WADS below;
#   byte 1  0 for the default context (as the committed scripts run), otherwise the
#           gameplay RNG index of the start;
#   byte 2  the low byte of every command's angleturn;
#   byte 3  the capture stride, 16 << (byte 3 % 4) admitted tics between DSIM39 round trips;
#   then four-byte records: forward, side, angleturn high byte, buttons. Buttons bits
#   0x40/0x80 are the record's repeat count (none 1, 0x40 4, 0x80 16, both 64 tics);
#   the low six bits are sent as they are (1 ATTACK, 2 USE, 4 CHANGE, weapon << 3).
#
# Mapping from the committed scripts (tests/reference/m4a_scripts/*.lmp, version-110
# demos: 13 header bytes, 4-byte ticcmds forward/side/angleturn-high/buttons, the 0x80
# marker): the header and the marker are dropped and each ticcmd is a record as it
# stands (the scripts' buttons never use 0x40/0x80); runs of an identical ticcmd are
# folded into repeat records (64, 16, 4, 1), which replays the same command sequence.
# Only the first 600 tics fit the target's budget, so longer scripts are cut there.
# The WAD of each script is the one its .txt source names (tests/reference/m4a_manifest.json).
WADS = ['doors', 'floors', 'plats', 'stairs', 'exits', 'weapons', 'items', 'spectre', 'scale']
SCRIPTS = {
    'DOORS1': 'doors', 'DOORS2': 'doors', 'DOORS3': 'doors',
    'FLOORS1': 'floors', 'FLOORS2': 'floors', 'FLOORS3': 'floors',
    'PLATS1': 'plats', 'PLATS2': 'plats',
    'STAIRS1': 'stairs', 'STAIRS2': 'stairs', 'DONUT1': 'stairs',
    'EXIT1': 'exits', 'EXIT2': 'exits', 'SECT11': 'exits', 'EXDIE': 'exits',
    'SAW1': 'weapons', 'ROCKET1': 'weapons', 'ROCKDIE': 'weapons',
    'ITEMS1': 'items', 'TIMERS': 'items', 'SUIT1': 'items', 'SUITDIE': 'items', 'INVIS1': 'items',
    'TURBO1': 'items', 'TARGET1': 'items',
    'SPECT1': 'spectre', 'SPECT2': 'spectre', 'SPECTD': 'spectre',
    'SCALE1': 'scale', 'SCALE2': 'scale',
}
HEADER = 4
TIC_BUDGET = 600
MAX_LEN = HEADER + 4 * TIC_BUDGET
DEMO_HEADER = bytes([110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0])
BT_ATTACK, BT_USE, BT_CHANGE = 1, 2, 4
REPEATS = [(64, 0xc0), (16, 0x80), (4, 0x40), (1, 0x00)]
ROOT = Path(__file__).resolve().parent.parent / 'tests' / 'reference' / 'm4a_scripts'


def demo_commands(name):
    """The (forward, side, turn high byte, buttons) ticcmds of a committed script."""
    data = (ROOT / f'{name}.lmp').read_bytes()
    assert data[:len(DEMO_HEADER)] == DEMO_HEADER, name
    assert data[-1] == 0x80 and (len(data) - len(DEMO_HEADER) - 1) % 4 == 0, name
    body = data[len(DEMO_HEADER):-1]
    commands = [tuple(body[i:i + 4]) for i in range(0, len(body), 4)]
    assert all(c[3] & 0xc0 == 0 for c in commands), name
    return commands


def records(commands):
    """Folds a ticcmd list (at most TIC_BUDGET tics) into repeat records."""
    commands = commands[:TIC_BUDGET]
    out = bytearray()
    i = 0
    while i < len(commands):
        c = commands[i]
        run = 1
        while i + run < len(commands) and commands[i + run] == c:
            run += 1
        i += run
        for size, bits in REPEATS:
            while run >= size:
                out += bytes([c[0] & 0xff, c[1] & 0xff, c[2] & 0xff, (c[3] & 0x3f) | bits])
                run -= size
    return bytes(out)


def held(tics, forward=0, side=0, turn=0, buttons=0):
    """`tics` copies of one command (turn is the angleturn high byte)."""
    return [(forward & 0xff, side & 0xff, turn & 0xff, buttons)] * tics


def turn(byte):
    """One tic turning by `byte` × 1.40625 degrees (the scripts' turn byte)."""
    return held(1, turn=byte)


def patterns():
    """Synthetic command lists aimed at every WAD's mechanisms from its start."""
    out = [
        held(600),                                                        # idle: monsters and timers
        held(600, buttons=BT_ATTACK),                                     # fire in place
        held(40, 50) + held(560, buttons=BT_USE),                         # walk, then USE every tic
        held(120, 50, 0, 0, BT_USE),                                      # walk using
        sum((turn(64) + held(30, 50, 0, 0, BT_USE) + held(8, buttons=BT_USE)
             for _ in range(12)), []),                                    # square walks with uses
        sum((turn(t) + held(25, 50, 25, 0, BT_ATTACK | BT_USE) for t in (32, 96, 160, 224)), []) * 4,
        held(200, 25, 0, 4, BT_USE),                                      # a slow circle using
        held(300, -50, 50, 2, BT_ATTACK),                                 # backward strafe circle
        # every weapon change 0-7 (4-7 outside this step's command domain), each fired
        sum((held(1, buttons=BT_CHANGE | (w << 3)) + held(30, buttons=BT_ATTACK)
             for w in range(8)), []),
        sum((held(1, 50, 0, 0, BT_ATTACK | BT_CHANGE | (w << 3)) + held(15, 40, 0, 0, BT_USE)
             for w in (7, 3, 6, 2, 5, 1, 4, 0)), []),
        # movement outside ±50 (refused at this step), then inside
        held(20, 100) + held(20, 50) + held(20, -127, 127) + held(20, -50, 50) + held(20, 127, -100, 8),
        # weapon bits without BT_CHANGE (refused), interleaved with admitted tics
        [(0, 0, 0, 0x38), (25, 0, 0, 0), (0, 0, 0, 0x18 | BT_USE), (25, 0, 0, BT_USE)] * 50,
        # a run into a wall and around; turns of every high byte
        [(50, 0, t, BT_USE) for t in range(256)] + held(100, 50, 0, 0, BT_USE),
        # sprint and use toward each quarter
        sum((turn(q) + held(60, 50, 0, 0, BT_USE) + held(10) for q in (0, 64, 128, 192)), []) * 2,
    ]
    return out


def header(wad, rng=0, low=0, stride=3):
    return bytes([wad, rng, low, stride])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    seeds = []
    # Each committed script on its WAD: the faithful replay, a second RNG, a nonzero
    # angleturn low byte and the shortest stride; then a faster cut (every 2nd tic dropped,
    # reaching later mechanisms within the budget) and the script after a 30-tic wait.
    for name in sorted(SCRIPTS):
        wad = WADS.index(SCRIPTS[name])
        commands = demo_commands(name)
        body = records(commands)
        seeds.append(header(wad) + body)
        seeds.append(header(wad, rng=6) + body)
        seeds.append(header(wad, low=0x80, stride=0) + body)
        seeds.append(header(wad, rng=200, low=0x5a, stride=1) + body)
        seeds.append(header(wad) + records(commands[::2]))
        seeds.append(header(wad, stride=2) + records(held(30) + commands))
        if len(commands) > TIC_BUDGET:
            # the remaining part of a long script from the start, with the walk prefix kept
            seeds.append(header(wad) + records(commands[:60] + commands[TIC_BUDGET:]))
    # Every synthetic pattern on every WAD with two starts.
    for wad in range(len(WADS)):
        for i, commands in enumerate(patterns()):
            for rng in (0, 6):
                seeds.append(header(wad, rng=rng, low=(i * 37) & 0xff, stride=i) + records(commands))
    # Header-only and odd-length inputs.
    for wad in range(len(WADS)):
        seeds.append(header(wad))
        seeds.append(header(wad) + bytes([50, 0, 0]))
    assert all(len(s) <= MAX_LEN for s in seeds)
    assert len(set(seeds)) == len(seeds)
    for i, data in enumerate(seeds):
        (args.directory / f'episode1-{i:04d}').write_bytes(data)
    print(f'{len(seeds)} Episode 1 command-stream seeds (max_len {MAX_LEN})')


if __name__ == '__main__':
    main()
