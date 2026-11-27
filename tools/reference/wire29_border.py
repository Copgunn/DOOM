#!/usr/bin/env python3
"""Wire 29: the pause picture below screen size 10, where the view window is inset (M6b step 7; the owner's "read, not
run" item of docs/M6B_REFERENCE_INVESTIGATION.md section 4.4). Expected values come from the genuine linuxdoom-1.10
runner only, never from Rust output.

Below size 10 D_Display draws the border: R_FillBackScreen tiles the flat FLOOR7_2 over screens[1] and draws the patches
brdr_t, brdr_b, brdr_l, brdr_r, brdr_tl, brdr_tr, brdr_bl and brdr_br around the view window (r_draw.c:731-813),
R_DrawViewBorder copies it to screens[0] (d_main.c:279-293); the synthetic WAD set had none of them, so the genuine
engine stopped at "W_GetNumForName: FLOOR7_2 not found!" (observed in M6b phase 1). This tool writes OUR OWN border
(nothing from any IWAD) and runs item PAUSE with it:

  tests/reference/wire29_wads/border.wad  a PWAD: FLOOR7_2, a 64x64 flat of two palette indices in 8-pixel checks;
                                          brdr_t/b/l/r 8x8 patches of one colour each with a darker edge line, and the
                                          four corners 8x8 of a third colour
  runs     the strict build of the M6b runner (tools/reference/m6b_build.py, read-only), rendered at screen sizes 9, 5
           and 3 (SIZES) and at 10, the demo of PAUSE on its M6b WAD set with border.wad before the demo lump, with the
           raw frames (--frames-raw); at 10 the frame stream must equal the committed tests/reference/m6b_frames/
           PAUSE.sb10.frames (the border lumps change nothing at full width, r_draw.c:747-748) and every run's dumps,
           sound and session streams (without their #config lines) those of the plain run without border.wad;
  goldens  tests/reference/wire29_border/PAUSE.sb<N>.pause, the M6b stream of each small size (the B row of every
           paused tic with its frame digest, the MP rows), and tests/reference/wire29_border/border.txt
           (DOOM-WIRE29-BORDER-v1): per size an O row with where the picture was found in the first paused frame
           (the top-left screen pixel of M_PAUSE's box, found by matching every opaque pixel of the M_PAUSE patch of
           tests/reference/m6b_wads/m6b_gfx.wad in the raw frame: exactly one place must match) and the digest of that
           frame.

--selftest: the runner's --selftest-pause-y (the picture one row lower) at size 5 must move the found place one row
down and change the first paused frame's digest.

    wire29_border.py [--check] [--selftest] [--work DIR] [--cc CC]
"""
import argparse, hashlib, shutil, struct, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m6a_goldens as G6            # noqa: E402  (read-only: golden_frames)
import m6a_wad as W                 # noqa: E402  (read-only: wad, patch)
import m6b_build as B               # noqa: E402  (read-only)
import m6b_goldens as G             # noqa: E402  (read-only: clean, run)
import m6b_wad as WAD               # noqa: E402  (read-only)
import wire29_pause as P            # noqa: E402  (read-only: run)

BORDER = 'tests/reference/wire29_wads/border.wad'
OUT = ROOT / 'tests/reference/wire29_border'
SCHEMA = 'DOOM-WIRE29-BORDER-v1'
ITEM, SIZES = 'PAUSE', (9, 5, 3)
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def border_wad():
    flat = bytes(96 if (x // 8 + y // 8) % 2 else 104 for y in range(64) for x in range(64))
    side = lambda ink, edge, vertical: W.patch([[edge if (x if vertical else y) in (0, 7) else ink for x in range(8)]
                                                for y in range(8)])
    lumps = [('FLOOR7_2', flat), ('BRDR_T', side(112, 4, False)), ('BRDR_B', side(112, 4, False)),
             ('BRDR_L', side(120, 4, True)), ('BRDR_R', side(120, 4, True))]
    lumps += [(f'BRDR_{c}', W.patch([[200] * 8 for _ in range(8)])) for c in ('TL', 'TR', 'BL', 'BR')]
    return W.wad(b'PWAD', lumps)


def pause_patch():
    """(width, height, left, top, {(x, y): index}) of the committed M_PAUSE (r_defs.h patch_t)."""
    data = (ROOT / WAD.GFX).read_bytes()
    n, at = struct.unpack_from('<ii', data, 4)
    lumps = {data[at + 16 * k + 8:at + 16 * k + 16].rstrip(b'\0').decode(): struct.unpack_from('<ii', data, at + 16 * k)
             for k in range(n)}
    pos, size = lumps['M_PAUSE']
    p = data[pos:pos + size]
    w, h, left, top = struct.unpack_from('<hhhh', p)
    px = {}
    for x in range(w):
        o = struct.unpack_from('<i', p, 8 + 4 * x)[0]
        while p[o] != 0xff:
            for k in range(p[o + 1]): px[(x, p[o] + k)] = p[o + 3 + k]
            o += p[o + 1] + 4
    return w, h, left, top, px


def find(frame, patch):
    """Every screen place (x, y) at which the patch's opaque pixels all appear in a 320x200 frame."""
    w, h, _l, _t, px = patch
    (fx, fy), first = next(iter(px.items()))
    return [(x, y) for y in range(201 - h) for x in range(321 - w)
            if frame[(y + fy) * 320 + x + fx] == first and all(frame[(y + b) * 320 + x + a] == v for (a, b), v in px.items())]


def rendered(binary, sb, work, extra=()):
    wads = [ROOT / w for w in WAD.wads(ITEM)] + [work / 'border.wad', ROOT / f'{WAD.SCRIPT_DIR}/{ITEM}.lmp']
    raw = work / 'raw' / f'sb{sb}'
    if raw.exists(): shutil.rmtree(raw)
    raw.mkdir(parents=True)
    r = P.run(binary, ITEM, work / 'runs' / f'sb{sb}', wads, sb=sb, extra=['--frames-raw', str(raw), *extra])
    r['raw'] = raw
    return r


def observe(r, patch):
    """(the first paused tic, the place found, the frame's digest)."""
    tic = next(int(l.split(',')[1]) for l in r['m6b'].splitlines() if l.startswith('B,'))
    frame = (r['raw'] / f'frame_{tic:05d}.raw').read_bytes()
    return tic, find(frame, patch), sha(frame)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'wire29' / 'border')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    work.mkdir(parents=True, exist_ok=True)
    wad = border_wad()
    (work / 'border.wad').write_bytes(wad)
    binary = B.build('strict', work / 'build', a.cc)
    plain = G.run(binary, ITEM, work / 'runs' / 'plain')
    patch = pause_patch()
    out = {ROOT / BORDER: wad}
    rows = [f'#schema,{SCHEMA}', '#generator,tools/reference/wire29_border.py: the genuine M6b runner (strict build) with '
            'our own border lumps (tests/reference/wire29_wads/border.wad); never from Rust output',
            f'#patch,M_PAUSE,width,{patch[0]},height,{patch[1]},leftoffset,{patch[2]},topoffset,{patch[3]}',
            '#columns,O,item,screenblocks,first_paused_tic,picture_x,picture_y,frame_sha256']
    for sb in (10,) + SIZES:
        r = rendered(binary, sb, work)
        same = r['digests'] == plain['digests'] and all(G.body(r[k]) == G.body(plain[k]) for k in ('sound', 'session'))
        if not G.clean(r) or not same: sys.exit(f'sb {sb}: exit {r["code"]} or the streams differ from the plain run: {r["stderr"][-300:]}')
        tic, places, digest = observe(r, patch)
        if len(places) != 1: sys.exit(f'sb {sb}: M_PAUSE found at {places}')
        b = next(l.split(',')[-1] for l in r['m6b'].splitlines() if l.startswith(f'B,{tic},'))
        if b != digest: sys.exit(f'sb {sb}: the raw frame is not the B row\'s')
        if sb == 10:
            if G6.golden_frames(r['frames']) != (ROOT / f'tests/reference/m6b_frames/{ITEM}.sb10.frames').read_text():
                sys.exit('sb 10: the frame stream with border.wad differs from the committed one')
        else:
            out[OUT / f'{ITEM}.sb{sb}.pause'] = r['m6b'].encode()
        rows.append(f'O,{ITEM},{sb},{tic},{places[0][0]},{places[0][1]},{digest}')
        print(f'sb {sb}: {r["m6b"].count(chr(10) + "B,")} paused frames; the first, tic {tic}, has M_PAUSE at {places[0]}')
    out[OUT / 'border.txt'] = ('\n'.join(rows) + '\n').encode()
    if a.selftest:
        r = rendered(binary, 5, work, extra=['--selftest-pause-y'])
        tic, places, digest = observe(r, patch)
        want = next(l.split(',') for l in rows if l.startswith(f'O,{ITEM},5,'))
        ok = places == [(int(want[4]), int(want[5]) + 1)] and digest != want[6]
        print(f'selftest {"pass" if ok else "FAIL"}: the picture one row lower at size 5 is found at {places}, digest changed')
        if not ok: return 1
    if a.check:
        bad = [str(p.relative_to(ROOT)) for p, d in out.items() if not p.exists() or p.read_bytes() != d]
        if bad: print('differs from the regenerated text: ' + ', '.join(bad)); return 1
        print(f'{len(out)} files match ({len(SIZES)} small sizes and 10)')
    else:
        for p, d in out.items(): p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(d)
        print(f'wrote {len(out)} files')
    shutil.rmtree(work / 'runs', ignore_errors=True); shutil.rmtree(work / 'raw', ignore_errors=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
