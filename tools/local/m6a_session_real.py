#!/usr/bin/env python3
"""Local-only: the M6a session against the genuine engine on the owner's IWAD (M6a Phase 2 step 8; never in CI: it
reads the commercial IWAD, which never enters the repository).

Two sets of runs, each in a declared game mode on a scratch copy of the IWAD named for it (ruling U4: doom.wad
registered, doomu.wad retail; the owner's file is only read, by the copy):

  single   the 61 real runs of tests/reference/m4b_real_results.json (the committed E1M1-E1M9 scripts and DEMO1,
           tools/local/m5a_real.py demo_path) as sessions started at the run's map: the session's first level is
           wire 26's new game, and a run that dies or exits goes on into the reborn or the intermission as far as
           its commands go;
  chains   sessions started at maps 1-9 (U14) that cross a boundary: the map's committed walk to its exit (a new
           game, so the exit is reached), an intermission skipped by use presses, then the next map's committed walk
           from the tic its load runs (found by a first genuine run) and idle tics; E1M2 starts with its death script,
           so the chain goes through the reborn reload first; E1M9 returns to E1M4 (the secret level's next map,
           g_game.c:1089-1096); E1M8 goes on into the finale. Spliced demos desynchronise after the first boundary
           (the carried player and P_Random index, note section 3.3) and stay valid comparison input. The walks are
           spliced as committed: their forward moves above TURBOTHRESHOLD (the E1M4, E1M6 and E1M7 walks use 100)
           stay, and after a load the turbo message follows the session's gametic as the C's does (g_game.c:671-679;
           the owner's ruling 1 of 2026-11-22).

Per run the genuine M6a runner (tools/reference/m6a_state.py's state variant, strict) at screenblocks 10 with dumps,
frames, sound and session streams and at screenblocks 11 with frames and sound (its sound stream must equal the
screenblocks-10 one), the genuine mixer (tools/reference/m5c_mixer_build.py, strict, ticbuffers) on the start list of
the sound stream, and the DSIM42 state digests of every level tic (m6a_state.levels; a run whose dumps the
independent encoder refuses fails and gets no state rows, its other goldens kept); the screenblocks-11 run must end
as the screenblocks-10 one does. A run that reaches the E1M8 victory is cut at the finale's first tic (the session's
typed NotYet(Victory) refusal there): every stream and the mixer's start list end before it. Everything is written
under --out in the layout of tests/reference (m6a_manifest.json, m6a_results.json, m6a_session/, m6a_sound/,
m6a_frames/, m6a_mixer/goldens/, m6a_state.txt): IWAD-derived, scratch only; --out and --work must not exist (each
set is written fresh), and the manifest and results are rewritten after every finished run, so a stopped set leaves
an index of the runs it finished. The Rust gates then read it:

    M6A_REFERENCE_DIR=OUT cargo test --release -p doom-game --test m6a_session --test m6a_state -- --ignored real_maps

Prints one line per run (counts) and writes nothing in the repository.

    python3 tools/local/m6a_session_real.py --set single|chains --mode registered|retail --out DIR
                                            [--iwad-dir DIR] [--work DIR] [--jobs N] [--only NAME,..]
"""
import argparse, concurrent.futures as cf, json, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m6a_state as ST             # noqa: E402
import m6a_goldens as G            # noqa: E402
import m5c_mixer_build as mb       # noqa: E402
import m5c_mixer_reference as MR   # noqa: E402
import m5c_starts                  # noqa: E402
from m5a_real import runs_selected, demo_path   # noqa: E402

IWAD_NAME = {'registered': 'doom.wad', 'retail': 'doomu.wad'}
HEADER = 13
WALKS = {1: 'e1m1_scripts/WALK.lmp', 2: 'e1m2_scripts/M2WALK.lmp', 3: 'e1m3_scripts/M3WALK.lmp',
         4: 'e1m4_scripts/M4WALK.lmp', 5: 'e1m5_scripts/M5WALK.lmp', 6: 'e1m6_scripts/M6WALK.lmp',
         7: 'e1m7_scripts/M7WALK.lmp', 8: 'e1m8_scripts/M8WALK.lmp', 9: 'e1m9_scripts/M9WALK.lmp'}
# (name, start map, skill, first demo, maps whose walks follow at each later load)
CHAINS = [('CH1', 1, 2, WALKS[1], [2]), ('CH2', 2, 2, 'e1m2_scripts/M2DIE.lmp', [2, 3]), ('CH3', 3, 2, WALKS[3], [4]),
          ('CH4', 4, 2, WALKS[4], [5]), ('CH5', 5, 3, 'e1m5_scripts/M5WALK3.lmp', [6]), ('CH6', 6, 2, WALKS[6], [7]),
          ('CH7', 7, 2, WALKS[7], [8]), ('CH8', 8, 2, WALKS[8], []), ('CH9', 9, 2, WALKS[9], [4])]
NOTHING = b'\0\0\0\0'
USE = b'\0\0\0\2'
INTERMISSION = NOTHING * 4 + (USE + NOTHING) * 30          # presses on every other tic until the next load
TAIL = NOTHING * 140


def commands(lmp):
    data = Path(lmp).read_bytes()
    assert data[0] == 110 and data[-1] == 0x80 and (len(data) - HEADER - 1) % 4 == 0, lmp
    return data[:HEADER], data[HEADER:-1]


def demo(header, map_number, skill, body):
    h = bytearray(header); h[1], h[2], h[3] = skill, 1, map_number
    return bytes(h) + body + b'\x80'


def item_for(name, lmp_path, mode, iwad, skill, map_number):
    return dict(name=name, lump=lmp_path.stem.upper(), mode=mode, wads=[iwad], lmp_path=lmp_path, skill=skill, map=map_number)


def plain_run(binary, item, work):
    """A genuine run with the session stream only (render none, no dumps): its L rows."""
    if work.exists(): shutil.rmtree(work)
    work.mkdir(parents=True)
    cmd = [str(binary), '--wad', str(item['wads'][0]), '--wad', str(item['lmp_path']), '--demo', item['lump'], '--game-mode',
           item['mode'], '--no-dump', *[x for k, v in G.M5C.RECOMMENDED.items() for x in (k, v)], '--render', 'none',
           '--session-out', str(work / 'session.txt')]
    subprocess.run(cmd, capture_output=True, text=True, env=G.ENV)
    text = (work / 'session.txt').read_text() if (work / 'session.txt').exists() else ''
    shutil.rmtree(work)
    return ST.S.rows(text, 'L'), ST.S.rows(text, 'G')


def build_chain(binary, spec, mode, iwad, demos, work):
    """The spliced demo of a chain (module text), built by genuine runs that find each load's tic."""
    name, map_number, skill, first, follow = spec
    header, body = commands(ROOT / 'tests/reference' / first)
    path = demos / f'{name}.lmp'
    for m in follow:
        path.write_bytes(demo(header, map_number, skill, body + INTERMISSION + TAIL))
        loads, _g = plain_run(binary, item_for(name, path, mode, iwad, skill, map_number), work / 'plain')
        later = [l['tic'] for l in loads if l['tic'] >= len(body) // 4]
        if not later:
            print(f'{name}: no load after tic {len(body) // 4}; the chain stops there'); break
        cut = later[0]
        _h, walk = commands(ROOT / 'tests/reference' / WALKS[m])
        body = (body + INTERMISSION + TAIL)[:4 * cut] + walk
    path.write_bytes(demo(header, map_number, skill, body + INTERMISSION + TAIL))
    return path


def cut_at(text, tic):
    """A stream without the rows of tics >= `tic` and without its end rows (Z, E)."""
    out = []
    for l in text.splitlines(keepends=True):
        f = l.split(',')
        if l.startswith('#') or f[0] == 'N': out.append(l); continue
        if f[0] in ('Z', 'E'): continue
        if len(f) > 1 and f[1].lstrip('-').isdigit() and int(f[1]) >= tic: continue
        out.append(l)
    return ''.join(out)


def genuine(binary, mbin, item, work):
    """Every golden of one run: dict(files {rel: text}, results, state rows, line)."""
    name = item['name']
    r10 = ST.run(binary, item, work / 'sb10')
    r11 = G.run(binary, dict(item, wads=item['wads']), work / 'sb11', 11)
    fails = []
    if r11['sound'] != r10['sound']: fails.append('the screenblocks 11 sound stream differs from 10')
    gs = G.table(r10['session'], 'G')
    victory = next((int(g['tic']) for g in gs if g['gamestate'] == '2'), None)
    end = G.M5C.end_of(dict(r10, summary=G.A.SUMMARY.search(r10['stdout'])))
    if r11['end'] != end: fails.append(f'the screenblocks 11 run ended {r11["end"]}, the screenblocks 10 run {end}')
    session, sound, f10, f11 = r10['session'], r10['sound'], r10['frames'], r11['frames']
    if victory is not None:
        session, sound, f10, f11 = (cut_at(t, victory) for t in (session, sound, f10, f11))
        res_end, tics = 'victory', victory
    else:
        res_end, tics = ('marker' if end == 'marker' else end), len(gs)
    try:
        lv = ST.levels(dict(r10, session=session, sound=sound), item) if (r10['dumps'] / 'load.txt').exists() else []
    except ST.L.ProjectionError as e:
        # the run fails and has no state rows (so the Rust state gate fails on it too); its other goldens are kept
        lv = []; fails.append(f'the independent encoder refused the state: {e}')
    starts = m5c_starts.convert(sound, name)
    lst = work / 'starts.txt'; lst.write_text(starts)
    p = subprocess.run([str(mbin), '--wad', str(item['wads'][0]), '--starts', str(lst), '--schedule', MR.GOLDEN_SCHEDULE,
                        '--out', str(work / 'mixer.txt')], capture_output=True, text=True, env=G.ENV, cwd=work)
    if p.returncode or MR.SANITIZER.search(p.stderr): fails.append(f'mixer exit {p.returncode}')
    mixer = (work / 'mixer.txt').read_text()
    files = {f'm6a_session/{name}.session': session, f'm6a_sound/{name}.sound': G.golden_sound(sound),
             f'm6a_frames/{name}.sb10.frames': G.golden_frames(f10), f'm6a_frames/{name}.sb11.frames': G.golden_frames(f11),
             f'm6a_mixer/goldens/{name}.golden': mixer}
    shutil.rmtree(work, ignore_errors=True)
    caps = sum(len(v['captures']) for v in lv)
    line = (f'{name}: {item["mode"]} E1M{item["map"]} skill {item["skill"]}: genuine end {end}, compared to {res_end} after '
            f'{tics} tics; {len(G.rows(session, "L"))} loads, {len(lv)} levels, {caps} level tics, '
            f'{len(G.rows(f10, "F"))} level and {len(G.rows(f10, "G"))} other frames, {len(G.rows(sound, "S"))} sound starts, '
            f'{sum(1 for l in mixer.splitlines() if l.startswith("b "))} buffers, stale-attacker rule fired on '
            f'{ST.stale_fired(lv)} loads' + ('' if not fails else '; ' + '; '.join(fails)))
    return dict(files=files, end=res_end, tics=tics, state=ST.rows_of(name, lv), line=line, fails=fails,
                fired=ST.stale_fired(lv))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--set', choices=['single', 'chains'], required=True)
    ap.add_argument('--mode', choices=['registered', 'retail'], required=True)
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--iwad-dir', type=Path, default=Path.home() / 'scratch/m6a/s1/iwad')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m6a/session-real')
    ap.add_argument('--jobs', type=int, default=2); ap.add_argument('--only'); ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    iwad = (a.iwad_dir / IWAD_NAME[a.mode]).resolve()
    if not iwad.is_file(): print(f'skip: no IWAD copy at {iwad}'); return 0
    out, work = a.out.resolve(), a.work.resolve()
    for d in (out, work):
        if d == ROOT or ROOT in d.parents: sys.exit('--out and --work must lie outside the repository')
        if d.exists(): sys.exit(f'{d} exists: each set is written into fresh directories')
    demos = out / 'demos'; demos.mkdir(parents=True)
    binary = ST.build(work / 'runner', a.cc)
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try: mbin = mb.build('strict', work / 'mixer', a.cc, quiet=True)
        finally: sys.stdout = so
    only = {x.upper() for x in a.only.split(',')} if a.only else None
    items = []
    if a.set == 'single':
        for k, r in enumerate(runs_selected(None, None)):
            name = f"{r['map']}_{Path(r['script']).stem.upper()}"
            if only and name not in only: continue
            src = demo_path(r, work, iwad)
            p = demos / f'S{k:02}.lmp'; p.write_bytes(Path(src).read_bytes())
            items.append(dict(item_for(name, p, a.mode, iwad, r['skill'], int(r['map'][3])), name=name))
    else:
        for spec in CHAINS:
            if only and spec[0] not in only: continue
            p = build_chain(binary, spec, a.mode, iwad, demos, work)
            items.append(item_for(spec[0], p, a.mode, iwad, spec[2], spec[1]))
    jobs = max(1, min(a.jobs, 2))
    results, manifest, state, fails, fired = {}, [], [], [], 0

    def index():
        (out / 'm6a_manifest.json').write_text(json.dumps(dict(schema='DOOM-M6A-MANIFEST-v1', wadsets={'iwad': [str(iwad)]},
                                                                 items=manifest), indent=1))
        (out / 'm6a_results.json').write_text(json.dumps(dict(schema='DOOM-M6A-RESULTS-v1', items=results), indent=1))
        head = ST.results_text({}).rstrip('\n').split('\n')
        (out / 'm6a_state.txt').write_text('\n'.join(head + state) + '\n')
    with cf.ThreadPoolExecutor(jobs) as ex:
        for it, g in zip(items, ex.map(lambda it: genuine(binary, mbin, it, work / it['name']), items)):
            print(g['line'], flush=True)
            for rel, text in g['files'].items():
                (out / rel).parent.mkdir(parents=True, exist_ok=True); (out / rel).write_text(text)
            results[it['name']] = dict(end=g['end'], session_tics=g['tics'])
            manifest.append(dict(name=it['name'], lump=it['lump'], mode=a.mode, wadset='iwad', map=it['map'],
                                 skill=it['skill'], lmp=str(it['lmp_path'])))
            state += g['state']; fails += [f"{it['name']}: {f}" for f in g['fails']]; fired += g['fired']
            index()
    index()
    print(f'{len(items)} runs, {sum(r["session_tics"] for r in results.values())} session tics compared; {len(fails)} failures; '
          f'stale-attacker rule fired on {fired} loads')
    for f in fails: print(f)
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
