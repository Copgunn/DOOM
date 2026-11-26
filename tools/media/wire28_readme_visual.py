#!/usr/bin/env python3
"""Writes docs/media/wire28-skills-side-by-side.gif (wire 28): the items SKILL0 and SKILL1 side by side.

    wire28_readme_visual.py [--work DIR] [--cargo-target DIR] [--check]

The same recorded commands (tests/reference/m6a_scripts/SKILL0.lmp and SKILL1.lmp, which differ only in the skill
byte) on our synthetic episode WAD and generated art (tests/reference/m6a_wads/episode.wad, m5b_gfx.wad, ...; no
IWAD), played by the Rust wire 28 session (crates/doom-game/examples/wire28_skill_frames.rs, screenblocks 10), every
4th tic at real speed. Panels: skill 0, skill 1, and their per-pixel difference (red where they differ).

Only the Rust frames are drawn; no genuine-engine run is made here. What backs them: every level frame of both runs
(348 each) must hash to the F row of the committed genuine stream tests/reference/m6a_frames/ITEM.sb10.frames, the
digests the gate `skills_0_and_1_against_every_genuine_stream` compares with the Rust session at every tic; the demos
must match the manifest's lmp_sha256; the first tic at which the two runs differ must be 32 (docs/validation/WIRE28.md
section 4). The palette is the PLAYPAL of the synthetic WADs. The file has no timestamp or path, so a rerun writes
the same bytes. --check compares with the committed file (a local check: it runs cargo). --work must be outside the
repository.
"""
import argparse, hashlib, json, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
for d in ('tools/reference', 'tools/media'):
    sys.path.insert(0, str(ROOT / d))
import m5b_readme_visual as B, m6a2_readme_visual as V   # noqa: E402

REF, OUT = ROOT / 'tests/reference', ROOT / 'docs/media/wire28-skills-side-by-side.gif'
W, H, SB, EVERY, HOLD_CS = 320, 200, 10, 4, 100
sha = lambda b: hashlib.sha256(b).hexdigest()


def frames(item, m, work, exe):
    """The item's level frames by tic, each checked against the committed genuine stream's F row."""
    wads = [ROOT / w for w in m['wadsets'][m['wadset']]]
    if not all(REF in w.parents for w in wads):
        sys.exit('a WAD of the item is not a committed synthetic one under tests/reference')
    lmp = ROOT / m['lmp']
    if sha(lmp.read_bytes()) != m['lmp_sha256']:
        sys.exit(f'{item}: the demo is not the manifest one')
    out = work / item
    p = subprocess.run([str(exe), *[a for w in wads for a in ('--wad', str(w))], '--demo', str(lmp), '--mode', m['mode'],
                        '--screenblocks', str(SB), '--out', str(out)], capture_output=True, text=True)
    if p.returncode:
        sys.exit(f'{item}: the Rust frame dump failed: {p.stderr[-500:]}')
    gold = {int(f[1]): f[2] for f in (l.split(',') for l in (REF / f'm6a_frames/{item}.sb{SB}.frames').read_text().splitlines())
            if f[0] == 'F'}
    pal = {int(t): int(n) for t, n in (l.split(',') for l in (out / 'index.txt').read_text().splitlines())}
    if sorted(pal) != sorted(gold):
        sys.exit(f'{item}: the level tics are not those of the genuine stream')
    pics = {t: (out / f't_{t}.raw').read_bytes() for t in gold}
    for t, s in gold.items():
        if sha(pics[t]) != s:
            sys.exit(f'{item} tic {t}: the Rust frame differs from the genuine stream digest')
    return pics, pal, wads


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/wire28-readme-visual')
    ap.add_argument('--cargo-target', type=Path, default=None)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents:
        sys.exit('--work must lie outside the repository')
    target = (a.cargo_target or work / 'target').resolve()
    p = subprocess.run(['cargo', 'build', '-p', 'doom-game', '--release', '--locked', '--example', 'wire28_skill_frames'],
                       cwd=ROOT, env={**__import__('os').environ, 'CARGO_TARGET_DIR': str(target)}, capture_output=True, text=True)
    if p.returncode:
        sys.exit(f'cargo build failed:\n{p.stderr[-2000:]}')
    exe = target / 'release/examples/wire28_skill_frames'
    items = {i['name']: i for i in json.loads((REF / 'm6a_manifest.json').read_text())['items']}
    m = json.loads((REF / 'm6a_manifest.json').read_text())
    (p0, pal0, wads), (p1, pal1, _) = [frames(n, {**items[n], 'wadsets': m['wadsets']}, work, exe) for n in ('SKILL0', 'SKILL1')]
    first = min(t for t in p0 if p0[t] != p1[t])
    if first != 32:
        sys.exit(f'the runs first differ at tic {first}, the record says 32')
    pal = V.playpal({'wads': wads})
    recs = []
    for t in sorted(p0):
        if t % EVERY == 0:
            n = sum(x != y for x, y in zip(p0[t], p1[t]))
            recs.append(dict(c=p0[t], r=p1[t], pal=pal0[t], tic=t, differing=n))
    out = []
    for r in recs:
        free = [i for i in range(255, -1, -1) if i not in set(r['c']) | set(r['r'])]
        table = bytearray(pal[r['pal'] * 768:(r['pal'] + 1) * 768])
        idx = {}
        for name, i in zip(B.UI, free):
            idx[name] = i
            table[3 * i:3 * i + 3] = bytes(B.UI[name])
        cv = bytearray([idx['bg']]) * (B.SCREEN_W * B.SCREEN_H)
        diff = bytes(idx['red'] if x != y else idx['black'] for x, y in zip(r['c'], r['r']))
        for k, img in enumerate((r['c'], r['r'], diff)):
            for y in range(H):
                at = (B.LABEL_H + y) * B.SCREEN_W + k * (W + B.GAP)
                cv[at:at + W] = img[y * W:(y + 1) * W]
        for k, s in enumerate(('SKILL 0: TOO YOUNG TO DIE', 'SKILL 1: NOT TOO ROUGH', 'DIFFERENCE')):
            B.text(cv, k * (W + B.GAP), 4, s, idx['white'], 1)
        y = B.LABEL_H + H + 4
        B.text(cv, 0, y, 'SAME COMMANDS ON A SYNTHETIC MAP: EASY SKILLS DIFFER FROM TIC 32', idx['white'])
        B.text(cv, 0, y + 22, f"TIC {r['tic']}: {r['differing']} OF {W * H} PIXELS DIFFER BETWEEN THE TWO RUNS", idx['green'])
        B.text(cv, 0, y + 46, 'RUST FRAMES, EACH EQUAL TO THE GENUINE ENGINE FRAME OF ITS RUN. EVERY 4TH TIC, REAL SPEED. SYNTHETIC ART',
               idx['grey'], 1)
        out.append((bytes(table), bytes(cv), free[len(B.UI)]))
    ends = [round((k + 1) * EVERY * 100 / 35) for k in range(len(out))]
    ds = [ends[0]] + [b - a for a, b in zip(ends, ends[1:])]
    ds[-1] += HOLD_CS
    gif, shown = B.write_gif(out, ds)
    B.self_check(gif, shown, ds)
    secs = sum(ds) / 100
    if len(gif) >= 2_000_000 or secs >= 15:
        sys.exit(f'exceeds 2 MB or 15 s ({len(gif)} bytes, {secs} s)')
    print(f'{OUT.name}: {len(out)} frames, {secs:.2f} s, {len(gif)} bytes, sha256 {sha(gif)}; first difference at tic {first}; '
          f'{len(p0) + len(p1)} level frames equal to the genuine stream digests')
    if a.check:
        if not OUT.exists() or OUT.read_bytes() != gif:
            sys.exit(f'{OUT.relative_to(ROOT)} differs from the regenerated animation')
        print(f'{OUT.relative_to(ROOT)}: identical to the regenerated animation')
    else:
        OUT.write_bytes(gif)
        print(f'wrote {OUT}')


if __name__ == '__main__':
    main()
