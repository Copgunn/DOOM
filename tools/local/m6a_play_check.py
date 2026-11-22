#!/usr/bin/env python3
"""Local-only: sessions recorded by doom-play checked against the genuine engine (M6a Phase 2 step 9; owner's ruling
U14 (3): the owner's own sessions, each checked with the local session tool and reported with its counts).

Each --demo is a session file doom-play wrote, E1M<map>-skill<n>-<retail|registered>-<time>.lmp: its game mode is the
file name's token (ruling U4; --mode, when given, must agree with it, as doom-game's replay_mode rules), its start map
and skill are the demo header's. The --wad files are the ones doom-play was given, in its order (--iwad first, then
every --wad). The first is copied into --work under the name linuxdoom-1.10 gives the mode (doomu.wad retail,
doom.wad registered); the given file is only read. Per demo, tools/local/m6a_session_real.py's `genuine` runs the
genuine M6a runner at screenblocks 10 (every stream and the load dumps) and 11 and the genuine mixer, and the goldens go
under --out in the layout of tests/reference, which the Rust gates read:

    M6A_REFERENCE_DIR=OUT cargo test --release -p doom-game --test m6a_session --test m6a_state -- --ignored real_maps

A session that reaches the E1M8 victory is compared up to the finale's first tic (the finale is not built yet).
IWAD-derived output stays in scratch; nothing is written in the repository.

    python3 tools/local/m6a_play_check.py --wad IWAD [--wad PWAD ..] --demo FILE [--demo FILE ..] --out DIR
                                          [--mode retail|registered] [--work DIR] [--jobs N]
"""
import argparse, concurrent.futures as cf, json, re, shutil, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m6a_session_real as SR      # noqa: E402

TOKEN = re.compile(r'^E1M[1-9]-skill[0-9]-(retail|registered)-')


def mode_of(path, flag):
    """The token's mode; a flag must agree with it, and a name without a token needs the flag."""
    m = TOKEN.match(path.name)
    token = m.group(1) if m else None
    if token and flag and token != flag: sys.exit(f'{path.name}: --mode {flag} disagrees with the file name ({token})')
    if not (token or flag): sys.exit(f'{path.name}: no game mode token in the file name and no --mode')
    return token or flag


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--wad', type=Path, action='append', required=True)
    ap.add_argument('--demo', type=Path, action='append', required=True)
    ap.add_argument('--mode', choices=['registered', 'retail'])
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m6a/play-check')
    ap.add_argument('--jobs', type=int, default=2); ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    out, work = a.out.resolve(), a.work.resolve()
    for d in (out, work):
        if d == ROOT or ROOT in d.parents: sys.exit('--out and --work must lie outside the repository')
        if d.exists(): sys.exit(f'{d} exists: each check is written into fresh directories')
    items = []
    for k, path in enumerate(a.demo):
        mode, data = mode_of(path, a.mode), path.read_bytes()
        if len(data) < 14 or data[0] != 110 or data[2] != 1 or data[-1] != 0x80: sys.exit(f'{path}: not a session demo')
        iwad = work / 'iwad' / mode / SR.IWAD_NAME[mode]
        if not iwad.exists():
            iwad.parent.mkdir(parents=True); shutil.copyfile(a.wad[0], iwad)
        lmp = out / 'demos' / f'P{k:02}.lmp'
        lmp.parent.mkdir(parents=True, exist_ok=True); lmp.write_bytes(data)
        items.append(dict(SR.item_for(f'P{k:02}', lmp, mode, iwad, data[1], data[3]), wads=[iwad, *map(Path.resolve, a.wad[1:])],
                          source=path.name))
    binary = SR.ST.build(work / 'runner', a.cc)
    mbin = SR.mb.build('strict', work / 'mixer', a.cc, quiet=True)
    results, manifest, state, fails = {}, [], [], []
    with cf.ThreadPoolExecutor(max(1, min(a.jobs, 2))) as ex:
        for it, g in zip(items, ex.map(lambda it: SR.genuine(binary, mbin, it, work / it['name']), items)):
            print(f"{it['source']}: {g['line']}", flush=True)
            for rel, text in g['files'].items():
                (out / rel).parent.mkdir(parents=True, exist_ok=True); (out / rel).write_text(text)
            results[it['name']] = dict(end=g['end'], session_tics=g['tics'])
            manifest.append(dict(name=it['name'], lump=it['lump'], mode=it['mode'], wadset=it['name'], map=it['map'],
                                 skill=it['skill'], lmp=str(it['lmp_path'])))
            state += g['state']; fails += [f"{it['source']}: {f}" for f in g['fails']]
    sets = {it['name']: [str(w) for w in it['wads']] for it in items}
    (out / 'm6a_manifest.json').write_text(json.dumps(dict(schema='DOOM-M6A-MANIFEST-v1', wadsets=sets, items=manifest), indent=1))
    (out / 'm6a_results.json').write_text(json.dumps(dict(schema='DOOM-M6A-RESULTS-v1', items=results), indent=1))
    (out / 'm6a_state.txt').write_text('\n'.join(SR.ST.results_text({}).rstrip('\n').split('\n') + state) + '\n')
    print(f'{len(items)} sessions, {sum(r["session_tics"] for r in results.values())} session tics; {len(fails)} failures')
    for f in fails: print(f)
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
