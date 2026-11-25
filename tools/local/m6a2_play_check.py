#!/usr/bin/env python3
"""Local-only: sessions recorded by doom-play in its finale-and-melt configuration (M6a part 2, wire 27) checked against
the genuine engine through the finale and every melt. tools/local/m6a_play_check.py cuts a run at the finale's first
tic; this tool does not.

Each --recording is a session file doom-play wrote (the owner's: only read). --idle N also builds, per recording, a
demo of ours under --out: the recording followed by N idle tics, named <item>I and reported as our construction (a
recording ends one tic before a tic doom-play refused, so only an extension can show that tic running). The game mode
is the file name's token. The --wad files are those doom-play was given, the IWAD first; it is copied into --work under
the mode's linuxdoom-1.10 name and only read.

Per demo the genuine runner (tools/reference/m6a_state.py's state variant: the genuine text, strict, -fwrapv) runs at
screenblocks 10 with the load dumps and at 11, then at both sizes with --melt-clock unit for the melt rows, and the
genuine mixer runs on the sound stream. The goldens go under --out in the layout of tests/reference: the part 1 files
(session, sound, frames, mixer, m6a_state.txt), uncut, and the melt files m6a_melt/<item>.melt (screenblocks 10) and
m6a2_melt/<item>.sb11.melt. The level states are projected by the independent DSIM42 encoder up to the victory; the
Rust gate reads the session's DSIM43 captures under wire 26's names. Then:

    M6A_REFERENCE_DIR=OUT cargo test --release -p doom-game --test m6a2_session -- --ignored real_maps --nocapture

IWAD-derived output stays in scratch; nothing is written in the repository. Only counts, tics and pass/fail are printed.

    python3 tools/local/m6a2_play_check.py --wad IWAD [--wad PWAD ..] --recording FILE [--recording FILE ..] --out DIR
                                           [--idle N] [--work DIR] [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, json, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m6a_play_check as PC        # noqa: E402  (read-only: the mode token)
import m6a_session_real as SR      # noqa: E402  (read-only: the genuine builds, the item, the victory cut)

G, ST = SR.G, SR.ST
UNIT = ['--melt-clock', 'unit']


def stable(text):
    """The frame rows two genuine runs of one demo must share: no melt rows, and on a tic whose F row flags columns of
    undefined reads only that row's masked digest (observed 2026-11-24: the whole-picture digests of F, D and Q differ
    there between two runs of the same build on the same demo, and nowhere else)."""
    rows = [r.split(',') for r in G.without(text, ('V', 'Y')).splitlines()]
    flagged = {r[1] for r in rows if r[0] == 'F' and int(r[6]) > 0}
    return [r[:2] + r[3:] if r[0] == 'F' and r[1] in flagged else r
            for r in rows if not (r[0] in ('D', 'Q') and r[1] in flagged)]


def genuine(binary, mbin, item, work):
    """Every golden of one uncut run: dict(files {rel: text}, end, tics, victory, state rows, fails)."""
    name, fails = item['name'], []
    r10 = ST.run(binary, item, work / 'sb10')
    r11 = G.run(binary, item, work / 'sb11', 11)
    u10 = G.run(binary, item, work / 'u10', 10, extra=UNIT)
    u11 = G.run(binary, item, work / 'u11', 11, extra=UNIT)
    end = G.M5C.end_of(dict(r10, summary=G.A.SUMMARY.search(r10['stdout'])))
    for label, r in (('screenblocks 11', r11), ('unit clock 10', u10), ('unit clock 11', u11)):
        if r['end'] != end: fails.append(f'the {label} run ended {r["end"]}, the screenblocks 10 run {end}')
        if G.golden_sound(r['sound']) != G.golden_sound(r10['sound']): fails.append(f'the {label} sound stream differs')
    for sb, plain, unit in ((10, r10, u10), (11, r11, u11)):
        if stable(unit['frames']) != stable(plain['frames']):
            fails.append(f'the unit melt clock changed frame rows other than the melt rows at screenblocks {sb}')
    gs = G.table(r10['session'], 'G')
    victory = next((int(g['tic']) for g in gs if g['gamestate'] == '2'), None)
    cut = (lambda t: SR.cut_at(t, victory)) if victory is not None else (lambda t: t)
    try: lv = ST.levels(dict(r10, session=cut(r10['session']), sound=cut(r10['sound'])), item)
    except ST.L.ProjectionError as e:
        lv = []; fails.append(f'the independent encoder refused the state: {e}')
    lst = work / 'starts.txt'; lst.write_text(SR.m5c_starts.convert(r10['sound'], name))
    wads = [x for w in item['wads'] for x in ('--wad', str(w))]
    p = subprocess.run([str(mbin), *wads, '--starts', str(lst), '--schedule', SR.MR.GOLDEN_SCHEDULE, '--out',
                        str(work / 'mixer.txt')], capture_output=True, text=True, env=G.ENV, cwd=work)
    if p.returncode or SR.MR.SANITIZER.search(p.stderr): fails.append(f'mixer exit {p.returncode}')
    files = {f'm6a_session/{name}.session': r10['session'], f'm6a_sound/{name}.sound': G.golden_sound(r10['sound']),
             f'm6a_frames/{name}.sb10.frames': G.golden_frames(r10['frames']),
             f'm6a_frames/{name}.sb11.frames': G.golden_frames(r11['frames']),
             f'm6a_mixer/goldens/{name}.golden': (work / 'mixer.txt').read_text(),
             f'm6a_melt/{name}.melt': G.melt_rows(u10['frames'], u10['sound']),
             f'm6a2_melt/{name}.sb11.melt': G.melt_rows(u11['frames'], u11['sound'])}
    melts = sum(1 for l in files[f'm6a_melt/{name}.melt'].splitlines() if l.startswith('Y,'))
    shutil.rmtree(work, ignore_errors=True)
    return dict(files=files, end=end, tics=len(gs), victory=victory, state=ST.rows_of(name, lv), fails=fails,
                line=f'genuine end {end} after {len(gs)} session tics; victory at session tic {victory}; '
                     f'{sum(len(v["captures"]) for v in lv)} level states, {melts} melts')


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--wad', type=Path, action='append', required=True)
    ap.add_argument('--recording', type=Path, action='append', required=True)
    ap.add_argument('--idle', type=int, default=0)
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m6a2/play-check')
    ap.add_argument('--jobs', type=int, default=2); ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    out, work = a.out.resolve(), a.work.resolve()
    for d in (out, work):
        if d == ROOT or ROOT in d.parents: sys.exit('--out and --work must lie outside the repository')
        if d.exists(): sys.exit(f'{d} exists: each check is written into fresh directories')
    items = []
    for k, path in enumerate(a.recording):
        mode, data = PC.mode_of(path, None), path.read_bytes()
        if len(data) < 14 or data[0] != 110 or data[2] != 1 or data[-1] != 0x80: sys.exit(f'{path}: not a session demo')
        iwad = work / 'iwad' / mode / SR.IWAD_NAME[mode]
        if not iwad.exists():
            iwad.parent.mkdir(parents=True); shutil.copyfile(a.wad[0], iwad)
        demos = [(f'P{k:02}', data, f'recording {k} ({(len(data) - 14) // 4} tics)')]
        if a.idle:
            demos.append((f'P{k:02}I', data[:-1] + b'\0\0\0\0' * a.idle + b'\x80',
                          f'recording {k} + {a.idle} idle tics (our construction)'))
        for name, body, label in demos:
            lmp = out / 'demos' / f'{name}.lmp'
            lmp.parent.mkdir(parents=True, exist_ok=True); lmp.write_bytes(body)
            items.append(dict(SR.item_for(name, lmp, mode, iwad, data[1], data[3]),
                              wads=[iwad, *map(Path.resolve, a.wad[1:])], label=label))
    binary = ST.build(work / 'runner', a.cc)
    mbin = SR.mb.build('strict', work / 'mixer', a.cc, quiet=True)
    results, manifest, state, fails = {}, [], [], []
    with cf.ThreadPoolExecutor(max(1, min(a.jobs, 2))) as ex:
        for it, g in zip(items, ex.map(lambda it: genuine(binary, mbin, it, work / it['name']), items)):
            print(f"{it['name']}: {it['label']}: {g['line']}", flush=True)
            for rel, text in g['files'].items():
                (out / rel).parent.mkdir(parents=True, exist_ok=True); (out / rel).write_text(text)
            results[it['name']] = dict(end=g['end'], session_tics=g['tics'])
            manifest.append(dict(name=it['name'], lump=it['lump'], mode=it['mode'], wadset=it['name'], map=it['map'],
                                 skill=it['skill'], lmp=str(it['lmp_path'])))
            state += g['state']; fails += [f"{it['name']}: {f}" for f in g['fails']]
    sets = {it['name']: [str(w) for w in it['wads']] for it in items}
    (out / 'm6a_manifest.json').write_text(json.dumps(dict(schema='DOOM-M6A-MANIFEST-v1', wadsets=sets, items=manifest), indent=1))
    (out / 'm6a_results.json').write_text(json.dumps(dict(schema='DOOM-M6A-RESULTS-v1', items=results), indent=1))
    (out / 'm6a_state.txt').write_text('\n'.join(ST.results_text({}).rstrip('\n').split('\n') + state) + '\n')
    print(f'{len(items)} demos, {sum(r["session_tics"] for r in results.values())} session tics; {len(fails)} failures')
    for f in fails: print(f)
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
