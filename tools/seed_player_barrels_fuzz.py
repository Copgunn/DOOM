#!/usr/bin/env python3
"""Bounded public barrel, radius-attack and decoration operation inputs for fuzz_targets/player_barrels.rs; no Rust-generated expectations."""
import argparse
from pathlib import Path

# The 26 M3h9b scenario starts of fuzz_targets/player_barrels.rs (byte 0, the rows of
# tests/reference/m3h9b_cases.csv): the range, cluster, player_range, monsters,
# walls, door, fireball, dense64, angles and embedded rooms with their player
# spawns. Byte 1 is 0 for the C driver's own setup of the case, otherwise the RNG
# index of a start with a full chaingun inventory.
STARTS = 26
RNGS = [0, 6]
# Records: b0 operation (b0 % 16), b1 slot or selector, b2 forward, b3 side, b4-5
# angle_turn, b6-7 candidate handle, b8 buttons; damage, pushes and momz read their
# operands from b3-b5 and b8 (see the target). At most 40 records (-max_len=362).
MAX_LEN = 2 + 9 * 40


def tic(forward=0, side=0, turn=0, buttons=0, slot=0):
    t = turn.to_bytes(2, 'little', signed=True)
    return bytes([0, slot, forward & 0xff, side & 0xff, t[0], t[1], 0, 1, buttons])


def run(tics, forward=0, side=0, turn=0, buttons=0):
    """A long run (op 4) of `tics` (1..256) tics of one held slot-One command."""
    assert 1 <= tics <= 256
    t = turn.to_bytes(2, 'little', signed=True)
    return bytes([4, tics - 1, forward & 0xff, side & 0xff, t[0], t[1], 0, 1, buttons])


def damage(target, amount, source=0, inflictor=0, index=0, who=0):
    """Op 5: target 0 body, 1 monster, 2 decoration, 3 pickup, 4 barrel, 5 candidate;
    source/inflictor 0 none, 1 monster, 2 barrel, 3 body (`who` indexes them)."""
    a = amount.to_bytes(2, 'little', signed=True)
    return bytes([5, target, index, source | inflictor << 2, who, a[0], 0, 1, a[1]])


def push(op, selector, index, x, y):
    """Op 8 (a barrel) or op 13 (0 body, 1 monster, 2 pickup, 3 decoration, 4 candidate)."""
    xb, yb = x.to_bytes(2, 'little', signed=True), y.to_bytes(2, 'little', signed=True)
    return bytes([op, selector, index, xb[0], xb[1], yb[0], 0, 1, yb[1]])


def momz(selector, index, units):
    """Op 9: 0 barrel, 1 body, 2 monster, 3 decoration, 4 candidate; momz in 16.16."""
    v = (units << 16).to_bytes(4, 'little', signed=True)
    return bytes([9, selector, index, v[0], v[1], v[2], 0, 1, v[3]])


IDLE, FIRE, USE = run(256), run(256, buttons=1), run(256, buttons=2)


def patterns():
    out = [b'']
    # Long horizons: standing, firing along the spawn angle, walking and firing,
    # turning while firing, using (doors, the lift), strafing.
    out += [IDLE, FIRE + IDLE, run(128, 25, 0, 0, 1) + IDLE, run(256, 0, 0, 2048, 1) + IDLE,
            run(64, 25) + USE, run(200, 0, 25, 0, 1) + run(120), run(96, -25, 0, 0, 1) + IDLE]
    out += [tic(buttons=1) * 24 + IDLE, tic(25, 0, 0, 1) * 16 + tic(0, 0, 1024, 3) * 16 + IDLE]
    # Wound then kill a barrel (with a monster inflictor and source, with none, with
    # a barrel inflictor), then 300+ tics so chains run their course.
    for src, inf in [(1, 1), (0, 0), (2, 2), (1, 0), (0, 1)]:
        for index in [0, 1, 5]:
            out.append(damage(4, 5, src, inf, index) + run(8) + damage(4, 30, src, inf, index) + IDLE + run(64))
    out.append(damage(4, 1000, 1, 1) + IDLE + run(100) + FIRE)
    # Push a barrel (in bound, at the bound, one past it, diagonally), then let it
    # slide; vertical momentum on a barrel (in bound, at it, past it, downwards).
    for x, y in [(24, 0), (64, 0), (65, 0), (-30, 10), (0, -64), (0, 0)]:
        out.append(run(4) + push(8, 0, 0, x, y) + run(128) + damage(4, 25, 1, 1) + IDLE)
    for units in [8, 64, 65, -8, -65]:
        out.append(run(4) + momz(0, 1, units) + run(96))
    # The refusal patterns: a target, a removal, a state entry and a death request on
    # a live barrel and on a dying one; direct spawns of 2035 and a decoration.
    for k in range(4):
        out.append(bytes([6, k, 0, k, 0, 0, 0, 1, 0]) + run(16) + damage(4, 30, 1, 1) + run(6) +
                   bytes([6, k, 0, k, 1, 0, 0, 1, 0]) + run(64))
    out.append(bytes([10, 7, 0, 0, 0, 0, 0, 1, 0]) + bytes([10, 7, 0x80, 0xfe, 0x40, 0x00, 0, 1, 1]) +
               bytes([10, 2, 0, 0, 0, 0, 0, 1, 0]) + run(32))
    # The stale bomb source: wound a barrel from a monster, remove that monster, kill
    # the barrel (the explosion tic is refused with the Invariant witness).
    out.append(damage(4, 5, 1, 1) + bytes([12, 1, 0, 0, 0, 0, 0, 1, 0]) + damage(4, 30) + run(32) + run(16, buttons=1))
    # Damage the body, a monster, a decoration and a pickup with and without an
    # inflictor; then keep firing.
    for target in [0, 1, 2, 3, 5]:
        for src, inf in [(1, 1), (0, 0), (3, 3)]:
            out.append(damage(target, 60, src, inf) + run(48, buttons=1) + run(48, buttons=2))
    # Pause, inherited decoration operations (retarget, removal, push, momz), and an
    # invalid command.
    out.append(bytes([7, 1, 0, 0, 0, 0, 0, 1, 0]) + run(32, buttons=1) + bytes([7, 0, 0, 0, 0, 0, 0, 1, 0]) + FIRE)
    out.append(bytes([11, 0, 0, 0, 0, 0, 0, 1, 0]) + bytes([11, 0x80, 0, 0, 0, 0, 0, 1, 0]) + bytes([11, 1, 3, 0, 0, 0, 0, 1, 0]) +
               bytes([12, 2, 0, 0, 0, 0, 0, 1, 0]) + bytes([12, 0, 0, 0, 0, 0, 9, 9, 0]) + push(13, 3, 0, 8, 0) + push(13, 3, 0, 0, 0) +
               push(13, 3, 0, 65, 0) + push(13, 1, 0, 20, 0) + momz(3, 0, 4) + momz(3, 0, 0) + momz(2, 0, 4) + run(64, buttons=1))
    out.append(tic(60) + tic(0, 0, 0, 0x80) + tic(0, 0, 0, 1, slot=2) + run(64, buttons=1))
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    seeds = [bytes([start, rng]) + p for start in range(STARTS) for rng in RNGS for p in patterns()]
    assert all(len(s) <= MAX_LEN for s in seeds)
    for i, data in enumerate(seeds):
        (args.directory / f'barrels-{i:04d}').write_bytes(data)
    print(f'{len(seeds)} barrel, radius-attack and decoration operation seeds (max_len {MAX_LEN})')


if __name__ == '__main__':
    main()
