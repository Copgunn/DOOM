#!/usr/bin/env python3
"""Wire 28 to the victory on skills 0 and 1 (owner, 2026-11-25): the synthetic items FIN0 and FIN1 and their goldens,
from the genuine linuxdoom-1.10 engine only, never from Rust output. Both are FINRET's demo (m6a_scripts/FINALE.lmp:
E1M8, the exit floor, the victory, the retail finale to its stage-1 melt) with the skill (header byte 1, g_game.c:1596)
set to 0 or 1; FIN0 has 128 empty ticcmds inserted before session tic 263 (the script's `idle 100`): sk_baby's halved
floor damage (p_inter.c:799-800) delays the exit by 128 tics, and its finale then runs command for command as FINRET's.
The goldens are made as SKILL0's and SKILL1's by the same tools, imported read-only (m6a_goldens.item_record: every build,
both screen sizes; m6a_melt_steps; m6a2_goldens.melt11_record; m5c_mixer_reference.run_sessions; the DSIM44 rows of
wire28_goldens.OVERRIDES[0], whose state run must give the goldens' streams), under tests/reference/wire28_fin/ in the
layout of those sets, with results.json (per item the skill, tics, end, victory tic, records). --check compares.

    wire28_fin.py [--check] [--work DIR] [--cc CC]
"""
import argparse, hashlib, json, sys, types
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5c_mixer_build as MB, m5c_mixer_reference as MR, m5c_starts, level_snapshot_m6a as S6  # noqa: E402
import m6a_goldens as G, m6a_melt_steps as MS, m6a_state as ST, m6a2_goldens as G2, level_snapshot_wire28 as E  # noqa: E402

OUT = 'tests/reference/wire28_fin/'
FIN = {'FIN0': (0, 263, 128), 'FIN1': (1, 0, 0)}           # skill, insertion tic, empty ticcmds inserted
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def record(name, bins, work):
    """(record, files {path: text or bytes}, failures) of one item."""
    (skill, at, n), base = FIN[name], next(i for i in G.items() if i['name'] == 'FINRET')
    b = base['lmp_path'].read_bytes()
    b = b[:1] + bytes([skill]) + b[2:13 + 4 * at] + bytes(4 * n) + b[13 + 4 * at:]
    lmp = work / 'lmp' / f'{name}.lmp'                     # the file name is the demo's lump name
    lmp.parent.mkdir(parents=True, exist_ok=True); lmp.write_bytes(b)
    it = dict(base, name=name, lump=name, skill=skill, lmp_path=lmp, lmp_sha256=sha(b))
    rec, texts, fails = G.item_record(it, bins['m6a'], work / 'runs')
    files = {p.replace('tests/reference/', OUT): t for p, t in texts.items()}
    t = lambda p: files[f'{OUT}{p}'.replace('*', name)]
    fails += MS.check_item(name, t('m6a_melt/*.melt'), t('m6a_session/*.session'))[2]
    ref = dict(frames=t('m6a_frames/*.sb11.frames'), sound=t('m6a_sound/*.sound'), session=t('m6a_session/*.session'))
    rec['melt_sb11'], files[f'{OUT}m6a2_melt/{name}.sb11.melt'], f = G2.melt11_record(it, bins['m6a'], work / 'runs', ref)
    res, f2 = MR.run_sessions({name: m5c_starts.convert(ref['sound'], name)}, bins['mixer'], MR.VARIANTS, work, 2, work / 'lists')
    files[f'{OUT}m6a_mixer/goldens/{name}.golden'] = res[(name, 'strict', MR.GOLDEN_SCHEDULE)]['rows']
    r = ST.run(bins['state'], it, work / 'state_run')
    same = (r['session'], G.golden_sound(r['sound']), G.golden_frames(r['frames'])) == (ref['session'], ref['sound'], t('m6a_frames/*.sb10.frames'))
    fails += f + f2 + ([] if same else [f'{name}: the state run\'s streams differ from the goldens'])
    shim = types.SimpleNamespace(**{k: getattr(S6, k) for k in dir(S6) if not k.startswith('__')})
    shim.Projector, shim.VERSION, plain, ST.S = E.Projector, E.VERSION, ST.S, shim
    try: rec['state_rows'] = ST.rows_of(name, ST.levels(r, it))
    finally: ST.S = plain
    rec['victory_tic'] = next(int(g['tic']) for g in G.table(ref['session'], 'G') if g['gamestate'] == '2')
    files[f'{OUT}{name}.lmp'] = b
    rec['goldens'] = {p: sha(x) for p, x in files.items()}
    return dict(skill=skill, inserted=[at, n], **rec), files, fails


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'wire28' / 'fin')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    bins = dict(m6a=G.build_all(work, a.cc), state=ST.build(work / 'state', a.cc),
                mixer={v: MB.build(v, work / 'mixer', a.cc, quiet=True) for v in MR.VARIANTS})
    files, recs, fails = {}, {}, []
    for name in FIN:
        r, f, x = record(name, bins, work)
        recs[name] = r; files.update(f); fails += x
        print(f"{name}: skill {r['skill']}, {r['session_tics']} tics, end {r['end']}, victory on tic {r['victory_tic']}, "
              f"{r['wipes']} melts, {len(r['state_rows'])} state rows")
    if fails: print('\n'.join(['FAILURES:'] + fails)); return 1
    head = ['#schema,DOOM-WIRE28-SKILL-STATE-v1', '#generator,tools/reference/wire28_fin.py (m6a_state.py\'s state variant and the '
            'independent DSIM44 encoder level_snapshot_wire28.py; never from Rust output)', '#rows,FIN0 and FIN1']
    files[f'{OUT}wire28_skill_state.txt'] = '\n'.join(head + [l for l in ST.results_text({}).splitlines() if l.startswith(('#columns', '#points'))]
                                                       + [l for r in recs.values() for l in r.pop('state_rows')]) + '\n'
    files[f'{OUT}results.json'] = json.dumps(dict(schema='DOOM-WIRE28-FIN-RESULTS-v1', base='FINRET', items=recs), indent=1) + '\n'
    files = {p: x if isinstance(x, bytes) else x.encode() for p, x in files.items()}
    have = {str(p.relative_to(ROOT)) for p in (ROOT / OUT).rglob('*') if p.is_file()}
    if a.check:
        bad = sorted(p for p, x in files.items() if p not in have or (ROOT / p).read_bytes() != x) + sorted(have - set(files))
        print(f'differs or not generated: {bad}' if bad else f'{len(files)} files under {OUT} match')
        return int(bool(bad))
    for p, x in files.items(): (ROOT / p).parent.mkdir(parents=True, exist_ok=True); (ROOT / p).write_bytes(x)
    print(f'wrote {len(files)} files under {OUT}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
