#!/usr/bin/env python3
"""Writes docs/media/m6a2-melt-side-by-side.gif and m6a2-finale-side-by-side.gif (M6a part 2).

    m6a2_readme_visual.py [--work DIR] [--cargo-target DIR] [--check]

Both show the genuine linuxdoom-1.10 engine, the Rust finale-and-melt session and their per-pixel difference, three
panels as m5b_readme_visual.py draws them, on synthetic maps and our own art only (no IWAD): the melt is CHAIN's
level-to-intermission wipe (tic 126, screenblocks 11), the finale is FULLRET's crawl (narrow part 2 font, sampled
every 30th tic) and its stage-1 wipe at finale tic 1,571. Pictures:

* genuine: the M6a runner of tools/reference/m6a_build.py (strict, built under --work, sources untouched), run with
  --melt-clock unit as the goldens were, relinked with tools/media/m6a2_pixel_tap.c, an observer that writes screens[0]
  after each genuine F_Drawer, WI_Drawer and wipe_ScreenWipe call (the runner itself dumps no melt or finale pixels);
* Rust: crates/doom-game/examples/m6a2_melt_frames.rs, the session the gate m6a2_run drives.

Nothing is drawn that was not checked: every genuine tapped picture must hash to its committed digest (the G rows of
tests/reference/m6a_frames and m6a2_frames, the Y and V rows of tests/reference/m6a2_melt), every Rust picture of the
same names must equal the genuine one byte for byte (all tics and melt steps of the run, not only those drawn), and
palette numbers must agree (--runner-args --selftest-pixel 0 160 100 (as one quoted value) plants a fault that must stop it). On the first difference it exits and writes nothing; the difference panel is red where
pixels differ, so black means identical, and each caption counts them. Delays are whole hundredths of a second; the
file has no timestamp or path, so a rerun writes the same bytes. --check compares with the committed files (a local
check: it builds a C runner and runs cargo). --work must lie outside the repository.
"""
import argparse, hashlib, os, shutil, struct, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
for d in ('tools/reference', 'tools/media'):
    sys.path.insert(0, str(ROOT / d))
import m3h9c_build as base, m5c_build as C, m5c_reference as M5C, m6a_build, m6a_goldens as G, m6a2_goldens as G2  # noqa: E402
import m5b_readme_visual as B                                                                                 # noqa: E402

REF = ROOT / 'tests/reference'
W, H, PIXELS, SB = 320, 200, 64000, 11
SLOW, HOLD_CS = 2, 150                       # melt steps at half of 35 steps a second; the last picture stays
RUNNER_ARGS = []
sha = lambda b: hashlib.sha256(b).hexdigest()
# name, item, frames golden, melt golden, melt tic, sampled tics (picture before the melt), caption lines
JOBS = [
    dict(name='melt', item='CHAIN', frames='m6a_frames/CHAIN.sb11.frames', melts='m6a2_melt/CHAIN.sb11.melt', tic=126,
         before=[126], lead_cs=100, out='m6a2-melt-side-by-side.gif',
         what='CHAIN: LEVEL TO INTERMISSION MELT, GAMETIC 126, ONE STEP A FRAME, HALF SPEED'),
    dict(name='finale', item='FULLRET', frames='m6a2_frames/FULLRET.sb11.frames', melts='m6a2_melt/FULLRET.sb11.melt',
         tic=1891, before=list(range(321, 1891, 30)) + [1890], lead_cs=10, out='m6a2-finale-side-by-side.gif',
         what='FULLRET: CRAWL EVERY 30TH TIC, THEN THE STAGE-1 MELT (TIC 1571), HALF SPEED'),
]


def rows(path, kind):
    return [l.split(',')[1:] for l in (REF / path).read_text().splitlines() if l.startswith(kind + ',')]


def build(work, cargo_target):
    env = dict(os.environ, CARGO_TARGET_DIR=str(cargo_target))
    p = subprocess.run(['cargo', 'build', '-p', 'doom-game', '--release', '--locked', '--example', 'm6a2_melt_frames'],
                       cwd=ROOT, env=env, capture_output=True, text=True)
    if p.returncode:
        sys.exit(f'cargo build failed:\n{p.stderr[-2000:]}')
    so, sys.stdout = sys.stdout, open(os.devnull, 'w')           # the build prints its progress
    try:
        m6a_build.build('strict', work / 'build')
    finally:
        sys.stdout = so
    b = work / 'build/strict'
    flags = base.VARIANTS['strict']
    tap = b / 'obj/m6a2_pixel_tap.o'
    for cmd in ([*flags, '-I', str(b / 'src'), '-c', str(ROOT / 'tools/media/m6a2_pixel_tap.c'), '-o', str(tap)],
                [*flags, *m6a_build.LINK_FLAGS, '-Wl,--wrap=F_Drawer,--wrap=WI_Drawer,--wrap=wipe_ScreenWipe',
                 *[str(b / 'obj' / (f[:-2] + '.o')) for f in [*m6a_build.UNITS, 'layout_probe.c', m6a_build.STUBS_NAME,
                                                               m6a_build.DRIVER_NAME]], str(tap), *C.LIBS, '-o', str(b / 'tapped')]):
        p = subprocess.run(['cc', *cmd], capture_output=True, text=True)
        if p.returncode:
            sys.exit(f'tap build failed:\n{p.stderr[-2000:]}')
    return b / 'tapped', cargo_target / 'release/examples/m6a2_melt_frames'


def playpal(item):
    pal = None
    for w in item['wads']:
        d = w.read_bytes()
        n, o = struct.unpack('<ii', d[4:12])
        for i in range(n):
            p, s, name = struct.unpack('<ii8s', d[o + 16 * i:o + 16 * i + 16])
            if name.rstrip(b'\0') == b'PLAYPAL':
                pal = d[p:p + s]
    if pal is None or len(pal) != 14 * 768:
        sys.exit('no 14-palette PLAYPAL in the item WADs')
    return pal


def collect(job, work, tools):
    """Runs both engines on the job's item, checks everything, returns the shown pictures."""
    tapped, rust = tools
    item = next(i for i in G.items() + G2.items2() if i['name'] == job['item'])
    wads = [a for w in item['wads'] + [item['lmp_path']] for a in ('--wad', str(w))]
    out = work / job['name']
    shutil.rmtree(out, ignore_errors=True)
    (out / 'g').mkdir(parents=True), (out / 'r').mkdir()
    p = subprocess.run([str(tapped), *wads, '--demo', item['lump'], '--game-mode', item['mode'], '--no-dump',
                        *[x for k, v in M5C.RECOMMENDED.items() for x in (k, v)], '--render', 'every',
                        '--screenblocks', str(SB), '--melt-clock', 'unit', '--frames', str(out / 'frames.txt'),
                        '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt'),
                        '--view', str(out / 'view.txt'), *RUNNER_ARGS], capture_output=True, text=True, env=dict(G.ENV, M6A_TAP=str(out / 'g')))
    if p.returncode:
        sys.exit(f'the genuine runner failed: {p.stderr[-500:]}')
    p = subprocess.run([str(rust), *[a for w in item['wads'] for a in ('--wad', str(w))], '--demo', str(item['lmp_path']),
                        '--mode', item['mode'], '--screenblocks', str(SB), '--out', str(out / 'r')], capture_output=True, text=True)
    if p.returncode:
        sys.exit(f'the Rust frame dump failed: {p.stderr[-500:]}')
    rd = lambda side, n: (out / side / n).read_bytes()
    gold = {int(t): (s, int(pal)) for t, _g, s, pal, *_ in rows(job['frames'], 'G')}
    ys = {int(r[0]): r for r in rows(job['melts'], 'Y')}
    vs = {}
    for t, k, _tics, _done, s in rows(job['melts'], 'V'):
        vs[int(t), int(k)] = s
    rust_pal = {}
    for l in (out / 'r/index.txt').read_text().splitlines():
        f = l.split(',')
        rust_pal[f[0], int(f[1]), int(f[2]) if f[0] == 'M' else 0] = int(f[3] if f[0] == 'M' else f[2])
    n_files = n_diff = 0
    for f in sorted((out / 'g').iterdir()):                      # every tapped picture, equal to Rust and to the goldens
        kind, *a = f.stem.split('_')
        a = [int(x) for x in a]
        g, x = f.read_bytes(), rd('r', f.name)
        if len(g) != PIXELS or len(x) != PIXELS:
            sys.exit(f'{f.name}: not {PIXELS} bytes')
        n_diff += sum(p != q for p, q in zip(g, x))
        if g != x:
            i = next(k for k in range(PIXELS) if g[k] != x[k])
            sys.exit(f'first difference in {f.name}: pixel x={i % W} y={i // W}: genuine {g[i]}, Rust {x[i]}')
        want = {'d': lambda: gold[a[0]][0], 'm': lambda: vs[a[0], a[1]], 'ms': lambda: ys[a[0]][3], 'me': lambda: ys[a[0]][4]}[kind]()
        if sha(g) != want:
            sys.exit(f'{f.name}: the genuine picture does not match its committed digest')
        if kind == 'd' and rust_pal['D', a[0], 0] != gold[a[0]][1]:
            sys.exit(f'{f.name}: palette number differs from the genuine run')
        if kind == 'm' and rust_pal['M', a[0], a[1]] != int(ys[a[0]][5]):
            sys.exit(f'{f.name}: palette number differs from the genuine run')
        n_files += 1
    steps = sorted(k for t, k in vs if t == job['tic'])
    pal = playpal(item)
    shown = [(f'd_{t}', gold[t][1], 'sample') for t in job['before']] if job['name'] == 'finale' else \
        [(f'ms_{job["tic"]}', int(ys[job['tic']][5]), 'start')]
    shown += [(f'm_{job["tic"]}_{k}', int(ys[job['tic']][5]), f'{k}/{len(steps)}') for k in steps]
    return pal, [(rd('g', n + '.raw'), rd('r', n + '.raw'), pnum, label) for n, pnum, label in shown], n_files, n_diff, len(steps)


def compose(job, pal, pics):
    out = []
    for k, (c, r, pnum, label) in enumerate(pics):
        free = [i for i in range(255, -1, -1) if i not in set(c) | set(r)]
        table = bytearray(pal[pnum * 768:(pnum + 1) * 768])
        idx = {}
        for name, i in zip(B.UI, free):
            idx[name] = i
            table[3 * i:3 * i + 3] = bytes(B.UI[name])
        cv = bytearray([idx['bg']]) * (B.SCREEN_W * B.SCREEN_H)
        diff = bytes(idx['red'] if a != b else idx['black'] for a, b in zip(c, r))
        for p, img in enumerate((c, r, diff)):
            for y in range(H):
                at = (B.LABEL_H + y) * B.SCREEN_W + p * (W + B.GAP)
                cv[at:at + W] = img[y * W:(y + 1) * W]
        for p, s in enumerate(('GENUINE C ENGINE', 'RUST SESSION', 'DIFFERENCE')):
            B.text(cv, p * (W + B.GAP), 4, s, idx['white'])
        y = B.LABEL_H + H + 4
        n = sum(a != b for a, b in zip(c, r))
        B.text(cv, 0, y, job['what'], idx['white'])
        B.text(cv, 0, y + 22, f"PICTURE {label}: {n} OF {PIXELS} PIXELS DIFFER", idx['green'] if n == 0 else idx['red'])
        B.text(cv, 0, y + 46, 'ALL TICS AND MELT STEPS OF THE RUN COMPARED ON PALETTE INDICES. SYNTHETIC TEST MAP AND ART, NOT THE GAMES',
               idx['grey'], 1)
        out.append((bytes(table), bytes(cv), free[len(B.UI)]))
    return out


def delays(job, pics):
    """Lead pictures hold lead_cs; each melt step is SLOW/35 s, in whole hundredths tracking the running total."""
    d, k, prev = [], 0, 0
    for p in pics:
        if p[3] in ('start', 'sample'):
            d.append(job['lead_cs'])
        else:
            k += 1
            end = round(k * SLOW * 100 / 35)
            d.append(end - prev)
            prev = end
    d[-1] += HOLD_CS
    return d


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m6a2-readme-visual')
    ap.add_argument('--cargo-target', type=Path, default=None)
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--runner-args', default='', help='extra genuine runner switches, for example the self-test '
                    '--selftest-pixel 0 160 100, which must make the script fail')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents:
        sys.exit('--work must lie outside the repository')
    work.mkdir(parents=True, exist_ok=True)
    RUNNER_ARGS[:] = a.runner_args.split()
    tools = build(work, (a.cargo_target or work / 'target').resolve())
    results = []
    for job in JOBS:
        pal, pics, n_files, n_diff, steps = collect(job, work, tools)
        frames, ds = compose(job, pal, pics), delays(job, pics)
        gif, shown = B.write_gif(frames, ds)
        B.self_check(gif, shown, ds)
        secs = sum(ds) / 100
        if len(gif) >= 2_000_000 or secs >= 15:
            sys.exit(f'{job["out"]}: exceeds 2 MB or 15 s ({len(gif)} bytes, {secs} s)')
        print(f'{job["out"]}: {len(frames)} frames, {secs:.2f} s, {len(gif)} bytes; {steps} melt steps drawn; '
              f'{n_files} genuine pictures of the run compared, {n_diff} differing pixels')
        results.append((ROOT / 'docs/media' / job['out'], gif))
    for path, gif in results:
        if a.check:
            if not path.exists() or path.read_bytes() != gif:
                sys.exit(f'{path.relative_to(ROOT)} differs from the regenerated animation')
            print(f'{path.relative_to(ROOT)}: identical to the regenerated animation')
        else:
            path.write_bytes(gif)
            print(f'wrote {path}')


if __name__ == '__main__':
    main()
