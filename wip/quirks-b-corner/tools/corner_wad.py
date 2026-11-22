#!/usr/bin/env python3
"""Synthetic corner-trace scenes: one WAD (E1M1 slot) and one version-110 demo per scene.

Map framing, painter, sides and lump order are the frozen M4b/M4a generators' (imported
read-only from tools/reference; run with PYTHONDONTWRITEBYTECODE=1 so nothing is written into
the repository). Each map is one square room (0,0)-(2304,2304) (BLOCKMAP origin (0,0), 19x19
cells; walls of 2304 keep the wire-22 arithmetic certificate), the player start, and the scene's observable: an 8x8 void pillar, a barrel or a tiny
manual door. Coordinates come from the replica search (find_scenes.py); the genuine engine is
the evidence.

    corner_wad.py OUTDIR
"""
import struct, sys, json
from pathlib import Path
sys.dont_write_bytecode = True
sys.path.insert(0, '/home/copgunn/src/DOOM/tools/reference')
import m4b_wad as B          # noqa: E402  read-only
import m4a_wad as A          # noqa: E402  read-only
from m4a_wad import W        # noqa: E402

ROOM = (0, 0, 2304, 2304)
BARREL = 2035

# name: dict(start, angle (thing degrees), turn (demo byte on tic 0), kind, obj, action)
SCENES = {
    # hitscan, thing angle exactly 45/135/225/315; pillar in the first geometric cell the stalled walk never visits
    'H1PP': dict(start=(597, 596), angle=45, turn=0, pillar=(1978, 1978, 1986, 1986), action='fire'),
    'C1PP': dict(start=(596, 596), angle=45, turn=0, pillar=(1978, 1978, 1986, 1986), action='fire'),
    'H2MP': dict(start=(1709, 596), angle=135, turn=0, pillar=(317, 1978, 325, 1986), action='fire'),
    'C2MP': dict(start=(1708, 596), angle=135, turn=0, pillar=(317, 1978, 325, 1986), action='fire'),
    'H3MM': dict(start=(1694, 1695), angle=225, turn=0, pillar=(315, 315, 323, 323), action='fire'),
    'C3MM': dict(start=(1694, 1694), angle=225, turn=0, pillar=(315, 315, 323, 323), action='fire'),
    'H4PM': dict(start=(582, 1721), angle=315, turn=0, pillar=(1971, 324, 1979, 332), action='fire'),
    'C4PM': dict(start=(582, 1720), angle=315, turn=0, pillar=(1971, 324, 1979, 332), action='fire'),
    # start on the block line x=512 (nudged to 513 by p_maputl.c:777-778), angle 45 + turn 8 (fine 1280)
    'N1': dict(start=(512, 578), angle=45, turn=8, pillar=(1190, 1596, 1198, 1604), action='fire'),
    'C5N': dict(start=(513, 578), angle=45, turn=8, pillar=(1190, 1596, 1198, 1604), action='fire'),
    # autoaim: the aim trace (range 1024) stalls before the barrel's cell; angle 45 + turn 7 (fine 1248)
    'A1': dict(start=(1915, 1859), angle=45, turn=7, barrels=[(2093, 2112)], action='fire'),
    'C6A': dict(start=(1914, 1859), angle=45, turn=7, barrels=[(2093, 2112)], action='fire'),
    # repeated stall cell: a barrel crossed by the trace inside the stall cell (16,15)
    'D1': dict(start=(1915, 1859), angle=45, turn=7, barrels=[(2056, 2040)], action='fire'),
    # use: 64-unit trace stalls in (5,4); a manual door (special 1) whose west face lies in (5,5)
    'U1': dict(start=(621, 613), angle=45, turn=7, door=(656, 656, 672, 672), action='use'),
    'C7U': dict(start=(620, 613), angle=45, turn=7, door=(656, 656, 672, 672), action='use'),
    # small room (0,0)-(1536,1536) for wires 7-16 (vertices within 2048, wall norm <= 1536): stall at iteration 1
    'S1': dict(start=(308, 276), angle=45, turn=7, pillar=(425, 444, 433, 452), action='fire', room=(0, 0, 1536, 1536)),
    'C8S': dict(start=(307, 276), angle=45, turn=7, pillar=(425, 444, 433, 452), action='fire', room=(0, 0, 1536, 1536)),
    # the S1 scene in a 1024 room: the far-wall puff stays inside the +-1024 spawn envelope of wires 7-13
    'S2': dict(start=(308, 276), angle=45, turn=7, pillar=(425, 444, 433, 452), action='fire', room=(0, 0, 1024, 1024)),
    'C9S': dict(start=(307, 276), angle=45, turn=7, pillar=(425, 444, 433, 452), action='fire', room=(0, 0, 1024, 1024)),
    # the S1 scene in a 768 room: the far-wall puff stays inside the +-1008 actor envelope of wires 7-13
    'S3': dict(start=(308, 276), angle=45, turn=7, pillar=(425, 444, 433, 452), action='fire', room=(0, 0, 768, 768)),
    'C10S': dict(start=(307, 276), angle=45, turn=7, pillar=(425, 444, 433, 452), action='fire', room=(0, 0, 768, 768)),
}
# optional: three barrels crossed inside the stall cell (intercept overflow), run separately
OPTIONAL = {
    'D2': dict(start=(1915, 1859), angle=45, turn=7, barrels=[(2056, 2040)] * 3, action='fire'),
}

def plan(name, sc):
    p = B.Plan(name.lower())
    room = p.sector('ROOM', [sc.get('room', ROOM)])
    if 'door' in sc:
        door = p.sector('DOOR', [sc['door']], ceil=0, layer=1, door=True)
    if 'pillar' in sc:
        p.void(room, *sc['pillar'])
    if 'door' in sc:
        x0, y0, x1, y1 = sc['door']
        # special 1 (DR manual door, wire 22's set) is outside m4b_wad's PART1|{97} guard; append directly
        p.specials.append((room, door, (x0, (y0 + y1) // 2), 1, 0))
    p.thing(sc['start'][0], sc['start'][1], sc['angle'], B.PLAYER1)
    for bx, by in sc.get('barrels', []):
        p.thing(bx, by, 0, BARREL)
    return p

def wad_bytes(p):
    m = B.build_map(p)
    A.fix_sides(p, m)
    lumps = dict(m['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in p.things)
    flat = bytes(4096)
    wad = ([('PLAYPAL', W.playpal()), ('COLORMAP', W.colormap()), ('S_START', b''), ('S_END', b''), ('F_START', b'')] +
           [(f, flat) for f in W.FLATS] + [('F_END', b''), ('PNAMES', W.pnames()), ('TEXTURE1', W.texture1()),
           (W.PATCH, W.blank_patch(W.TEX_W, W.TEX_H)), ('E1M1', b'')] + [(n, lumps[n]) for n in A.MAP_LUMPS])
    data = bytearray(b'IWAD' + struct.pack('<ii', len(wad), 0))
    directory = b''
    for n, b in wad:
        directory += struct.pack('<ii8s', len(data), len(b), W.name8(n))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory), m

def demo(sc):
    """13-byte v110 header (skill 2, E1M1, single player) + 4 bytes/tic + 0x80 marker.
    tic 0: the turn byte; fire: ATTACK on tic 30 only (the pistol is up after 16 raise tics),
    then 40 idle tics; use: USE on tic 5 only, then 40 idle tics."""
    hdr = bytes([110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0])
    tics = []
    n = 71 if sc['action'] == 'fire' else 46
    for t in range(n):
        turn = sc['turn'] if t == 0 else 0
        buttons = 0
        if sc['action'] == 'fire' and t == 30: buttons = 1
        if sc['action'] == 'use' and t == 5: buttons = 2
        tics.append(bytes([0, 0, turn & 0xff, buttons]))
    return hdr + b''.join(tics) + bytes([0x80])

def main():
    out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
    allsc = dict(SCENES); allsc.update(OPTIONAL)
    info = {}
    for name, sc in allsc.items():
        p = plan(name, sc)
        data, m = wad_bytes(p)
        (out / f'{name}.wad').write_bytes(data)
        (out / f'{name}.lmp').write_bytes(demo(sc))
        info[name] = dict(sc, blocks=list(m['blocks'][:4]), lines=len(m['lines']), optional=name in OPTIONAL)
    (out / 'scenes.json').write_text(json.dumps(info, indent=1))
    print(len(info), 'scenes written to', out)

if __name__ == '__main__':
    main()
