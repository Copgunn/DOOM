#!/usr/bin/env python3
"""Generate version-110 demo lumps (.lmp) for the M3h9c E1M1 runs from text scripts.

A script is a text file; blank lines and text after '#' are ignored. Lines:

  idle N                               N tics of zero ticcmds
  cmd N FORWARD SIDE TURN BUTTONS      N tics of one ticcmd
  seed S chaos N [nouse]               N tics from a fixed-seed pseudo-random stream

FORWARD and SIDE are -50..50 (the admitted domain: larger values reach the turbo check,
g_game.c:672-679, and -128 would be the 0x80 demo marker, g_game.c:1488,1493). TURN is
the angleturn high byte, 0..255 or -128..-1 (G_ReadDemoTiccmd shifts the unsigned byte
<< 8, g_game.c:1501). BUTTONS is an integer or names joined by '|' or '+': ATTACK (1),
USE (2), CHANGE (4), W0..W3 (the weapon number << 3, d_event.h:75-88). Refused: bit 0x80
(BT_SPECIAL: pause or save game, g_game.c:702-722), bit 0x40, weapon numbers 4..7 (bit
0x20) and weapon bits without CHANGE.

`chaos` draws every field independently from splitmix64 seeded with S (state += 0x9E3779B97F4A7C15;
z = state; z = (z ^ z>>30) * 0xBF58476D1CE4E5B9; z = (z ^ z>>27) * 0x94D049BB133111EB;
z ^= z>>31, all mod 2^64), per tic in this order: forward = z % 101 - 50, side = z % 101
- 50, turn = z % 256, buttons = [0, ATTACK, USE, ATTACK|USE, CHANGE|W0, CHANGE|W1,
CHANGE|W2, CHANGE|W3][z % 8]. The generator cannot know on which tics the player is
dead, so chaos stays in this general domain on every tic. With the trailing word
`nouse` the stream is the same but USE is removed from every buttons entry (USE -> 0,
ATTACK|USE -> ATTACK): on a map whose start is near the exit switch a chaos run would
otherwise end at the first USE that reaches it, and USE while dead triggers the reborn
reload (P_DeathThink, p_user.c:227-228).

Output: the 13-byte header (110, skill 2, episode 1, map 1, deathmatch 0, respawn 0,
fast 0, nomonsters 0, consoleplayer 0, playeringame 1 0 0 0; g_game.c:1555-1566 and
1589-1606), four bytes per tic (forwardmove, sidemove, angleturn >> 8, buttons) and the
0x80 marker. The lump name is the upper-cased base name (W_AddFile, w_wad.c:172-180),
so the base name must be 1..8 characters. A script whose own base name is longer names
its output with a comment line `# output: NAME.lmp` (same directory), e.g.
synth_idle.txt -> SYNIDLE.lmp.

    e1m1_scripts.py SCRIPT.txt [-o OUT.lmp]      default OUT: the `# output:` name, else
                                                 SCRIPT with .lmp
    e1m1_scripts.py --check                      regenerate every *.txt under
                                                 tests/reference/e1m1_scripts/ and
                                                 tests/reference/m3h9c_scripts/ in memory and
                                                 compare with the committed .lmp next to it
"""
import argparse, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT_DIRS = [ROOT / 'tests/reference/e1m1_scripts', ROOT / 'tests/reference/m3h9c_scripts']
HEADER = bytes([110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0])
MARKER = 0x80
NAMES = {'ATTACK': 1, 'USE': 2, 'CHANGE': 4, 'W0': 0 << 3, 'W1': 1 << 3, 'W2': 2 << 3, 'W3': 3 << 3}
CHAOS_BUTTONS = [0, 1, 2, 3, 4 | (0 << 3), 4 | (1 << 3), 4 | (2 << 3), 4 | (3 << 3)]
CHAOS_BUTTONS_NOUSE = [b & ~2 for b in CHAOS_BUTTONS]
MASK = (1 << 64) - 1

class ScriptError(Exception): pass

def splitmix64(seed):
    state = seed & MASK
    while True:
        state = (state + 0x9E3779B97F4A7C15) & MASK
        z = state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
        yield z ^ (z >> 31)

def buttons_value(token):
    if token.lstrip('-').isdigit(): return int(token)
    total = 0
    for name in token.replace('+', '|').split('|'):
        if name.upper() not in NAMES: raise ScriptError(f'unknown button name {name!r}')
        total |= NAMES[name.upper()]
    return total

def checked(forward, side, turn, buttons):
    for name, v in (('forward', forward), ('side', side)):
        if not -50 <= v <= 50: raise ScriptError(f'{name} {v} outside -50..50')
    if not -128 <= turn <= 255: raise ScriptError(f'turn {turn} outside -128..255')
    if not 0 <= buttons <= 255: raise ScriptError(f'buttons {buttons} outside 0..255')
    if buttons & 0x80: raise ScriptError('buttons bit 0x80 (BT_SPECIAL) refused')
    if buttons & 0x40: raise ScriptError('buttons bit 0x40 refused')
    if buttons & 0x20: raise ScriptError('weapon numbers 4..7 refused (bit 0x20)')
    if buttons & 0x18 and not buttons & 4: raise ScriptError('weapon bits without CHANGE refused')
    return bytes([forward & 0xff, side & 0xff, turn & 0xff, buttons])

def ticcmds(lines):
    """Yield the 4-byte ticcmd of every tic of a script."""
    for number, raw in enumerate(lines, 1):
        words = raw.split('#', 1)[0].split()
        if not words: continue
        try:
            if words[0] == 'idle' and len(words) == 2:
                yield from [bytes(4)] * count(words[1])
            elif words[0] == 'cmd' and len(words) == 6:
                cmd = checked(int(words[2]), int(words[3]), int(words[4]), buttons_value(words[5]))
                yield from [cmd] * count(words[1])
            elif words[0] == 'seed' and words[2:3] == ['chaos'] and (len(words) == 4 or (len(words) == 5 and words[4] == 'nouse')):
                draw, table = splitmix64(int(words[1])), CHAOS_BUTTONS_NOUSE if len(words) == 5 else CHAOS_BUTTONS
                for _ in range(count(words[3])):
                    f, s, t, b = next(draw) % 101 - 50, next(draw) % 101 - 50, next(draw) % 256, table[next(draw) % 8]
                    yield checked(f, s, t, b)
            else:
                raise ScriptError('expected "idle N", "cmd N FORWARD SIDE TURN BUTTONS" or "seed S chaos N [nouse]"')
        except (ScriptError, ValueError) as e:
            raise ScriptError(f'line {number}: {e}') from None

def count(token):
    n = int(token)
    if n < 0: raise ScriptError('negative count')
    return n

def lump(script_text):
    cmds = list(ticcmds(script_text.splitlines()))
    return HEADER + b''.join(cmds) + bytes([MARKER]), len(cmds)

def lump_name(path):
    base = Path(path).name.split('.')[0]
    if not 1 <= len(base) <= 8: raise ScriptError(f'{path}: base name must be 1..8 characters (the lump name)')
    return base.upper()

def default_output(script):
    """The `# output: NAME.lmp` comment of a script (next to it), else SCRIPT.lmp."""
    for line in script.read_text().splitlines():
        words = line.split()
        if words[:2] == ['#', 'output:'] and len(words) == 3 and words[2].endswith('.lmp') and '/' not in words[2]:
            return script.with_name(words[2])
    return script.with_suffix('.lmp')

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('script', nargs='?', type=Path)
    ap.add_argument('-o', '--out', type=Path)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    try:
        if a.check:
            sources = [s for d in SCRIPT_DIRS for s in sorted(d.glob('*.txt'))]
            for d in SCRIPT_DIRS:
                if not any(d.glob('*.txt')): sys.exit(f'no scripts under {d.relative_to(ROOT)}')
            bad = []
            for src in sources:
                out = default_output(src); lump_name(out)
                data, tics = lump(src.read_text())
                if not out.exists() or out.read_bytes() != data: bad.append(out.name)
                else: print(f'{src.parent.name}/{src.name}: {tics} tics, {out.name} matches')
            if bad: sys.exit('differs or missing: ' + ', '.join(bad))
            return
        if not a.script: ap.error('a script or --check is required')
        out = a.out or default_output(a.script); lump_name(out)
        data, tics = lump(a.script.read_text())
        out.write_bytes(data)
        print(f'{out}: {tics} tics, {len(data)} bytes, lump {lump_name(out)}')
    except ScriptError as e:
        sys.exit(f'error: {e}')

if __name__ == '__main__':
    main()
