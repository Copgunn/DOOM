#!/usr/bin/env python3
"""Local-only: wire 29 against the genuine linuxdoom-1.10 engine on the 61 real runs, at the simulation level (M6b
phase 2 step 5; docs/M6B_REFERENCE_INVESTIGATION.md section 6.2 item 2; never in CI: it reads the commercial IWAD).

Kinds (--kind; each a demo of ours written under --work from the committed command recordings of
tests/reference/m4b_real_results.json, the E1M1-E1M9 scripts and DEMO1, tools/local/m5a_real.demo_path):
  skill4    the header's skill byte set to 4 (sk_nightmare);
  respawn   the respawnparm byte (5) set, the recording's own skill;
  fast      the fastparm byte (6) set, the recording's own skill;
  both      both bytes set, the recording's own skill;
  own       the recording unchanged, played under wire 29: the control, with Rust wire 28 played alongside (the example's
            --twin28: every capture DSIM44's but for the profile, context and the two DSIM45 fields);
  pause     (slot: pause commands inserted into a level; a later commit).
--recording FILE (repeatable, kind 'sessions'): the owner's doom-play recordings (ruling N8; read only), the game mode
from the file name's token; --selftest-recording N first makes one headlessly through doom-play's app
(wire29_session_real --record, skill 4 with both parameters, pauses at session tics 200/260 and 700/701).
Each demo is played to its last command: a player who now dies or misses the exit is still a valid input.

The session level (--session; always for sessions): crates/doom-game/examples/wire29_session_real.rs plays the
demo through Session::new_finale_and_melt_wire29, as doom-play does, and every session tic's G columns (SESSION_COLUMNS)
must equal the genuine session stream's, every level capture (paused tics included) the projected one.

Genuine: the M6b runner (tools/reference/m6b_build.py, read-only), strict build (the genuine text, -fwrapv), with
tools/reference/m6a_state.py's two exact-once state patches applied in memory (the dumps of the later loads), --render
none with dumps, session, sound and M6b streams. The M6b stream's T row of the demo's G_InitNew must name the skill and
parameters (wire29_goldens.check_table). Every level of the run (m6a_state.levels: a reborn reload and a carried load
are levels; a run is cut at the E1M8 victory) is projected by the independent DSIM45 encoder level_snapshot_wire29.py.
Rust: crates/doom-sim/examples/wire29_real.rs (release) plays the same demo under wire 29, takes only the genuine load
tics after the first (the intermission's length is the session's), and prints its levels and capture digests; every
capture after a level tic is compared with the genuine one, level by level. A Rust refusal is no failure: it is
counted by witness and by the fraction of the run's session tics it reached. Failures: a differing capture, differing
levels, a projection refusal, an unclean genuine run, a failed Rust run (reader, twin, load).

Per-run lines and logs go to --work/logs (IWAD-derived digests, scratch only); only counts are printed for the record.

    python3 tools/local/wire29_real.py --iwad PATH --work DIR --kind KIND[,KIND] [--only NAME,..] [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, functools, inspect, json, multiprocessing as mp, re, shutil, subprocess
import sys, threading, types
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m6a_state as ST                # noqa: E402  (read-only: the state patches, levels)
import m6a_build as B6A               # noqa: E402
import m6b_build as B6B               # noqa: E402  (read-only)
import m6b_goldens as G               # noqa: E402  (read-only: RECOMMENDED switches, ENV, tables_of)
import level_snapshot_m6a as S6       # noqa: E402
import level_snapshot_wire29 as E     # noqa: E402
import wire29_goldens as W29          # noqa: E402  (read-only: check_table)
import m6a_session_real as SR         # noqa: E402  (read-only: cut_at)
from m5a_real import runs_selected, demo_path   # noqa: E402

KINDS = {'skill4': (4, 0, 0), 'respawn': (None, 1, 0), 'fast': (None, 0, 1), 'both': (None, 1, 1), 'own': (None, 0, 0),
         'sessions': (None, None, None)}
PAUSE_SLOT = 'pause'          # the next commit's kind: pause commands inserted (the level rows equal the unpaused, shifted)
SESSION_COLUMNS = ('gamestate', 'map', 'leveltime', 'gametic', 'levelstarttic', 'prndindex', 'rndindex', 'paused')
# m6a_state.levels for runs with paused tics (in memory, the module file unchanged): a paused tic's dump repeats its
# leveltime, so the captures are those after the load dump and their session tics must be consecutive
_SRC = inspect.getsource(ST.levels).replace('def levels(', 'def levels_paused(')
for _old, _new in (('            if d.tic == 0: continue\n', '            if j == 0: continue\n'),
                   ('        if [c[0] for c in caps] != list(range(1, len(caps) + 1)):',
                    '        if caps and [c[1] for c in caps] != list(range(caps[0][1], caps[0][1] + len(caps))):')):
    assert _SRC.count(_old) == 1, _old
    _SRC = _SRC.replace(_old, _new)
exec(_SRC, ST.__dict__)
TL = threading.local()        # per genuine run: the parameters, the capture to record (diagnosis), what was recorded


def projector(mapdata, skill, map_number, sounds, start):
    """m6a_state.levels' projector, over DSIM45 (the declared override, as wire28_real.py's over DSIM44)."""
    pr = E.Projector(mapdata, skill, map_number, sounds, start, *TL.parameters)
    step, k, TL.made = pr.step, TL.made, TL.made + 1

    def recorded(d, following=None):
        out = step(d, following)
        if (k, d.tic) == TL.at: TL.seen['fields'] = out[0]
        return out
    pr.step = recorded
    found = pr.respawned

    def counted(d):
        out = found(d)
        TL.respawns += len(out)
        return out
    pr.respawned = counted
    return pr


SHIM = types.SimpleNamespace(**{k: getattr(S6, k) for k in dir(S6) if not k.startswith('__')})
SHIM.VERSION, SHIM.Projector = E.VERSION, projector


def build(work, cc):
    """The M6b strict runner with m6a_state's later-load dumps (module text)."""
    session = work / 'patched' / B6A.SESSION_C.name
    session.parent.mkdir(parents=True, exist_ok=True)
    session.write_bytes(ST.patched(B6A.text(B6A.SESSION_C), ST.SESSION_PATCHES).encode('latin-1'))
    plain_driver, plain_session = B6A.driver_text, B6A.SESSION_C
    B6A.driver_text, B6A.SESSION_C = (lambda: ST.patched(plain_driver(), [ST.DRIVER_PATCH])), session
    try: return B6B.build('strict', work / 'build', cc)
    finally: B6A.driver_text, B6A.SESSION_C = plain_driver, plain_session


def genuine(binary, item, out, respawn, fast, at=None, extra=(), paused=False):
    """dict(levels [(load, tic, map, kind, [(leveltime, digest)])], tics, end) or dict(fail); with `at` (level,
    leveltime) also the projected fields of that capture (diagnosis)."""
    if out.exists(): shutil.rmtree(out)
    (out / 'dumps' / 'loads').mkdir(parents=True)
    cmd = [str(binary), '--wad', str(item['wads'][0]), '--wad', str(item['lmp_path']), '--demo', item['lump'],
           '--game-mode', item['mode'], '--out', str(out / 'dumps'), *[x for kv in G.M5C.RECOMMENDED.items() for x in kv],
           '--render', 'none', '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt'),
           '--m6b-out', str(out / 'm6b.txt'), *extra]
    p = subprocess.run(cmd, capture_output=True, text=True, env=G.ENV)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    r = dict(session=rd('session.txt'), sound=rd('sound.txt'), m6b=rd('m6b.txt'), dumps=out / 'dumps')
    try:
        if p.returncode or ' end=marker ' not in p.stdout: return dict(fail=f'genuine exit {p.returncode}: {p.stderr[-300:]}')
        W29.check_table(item['name'], r, item['skill'], respawn, fast)
        gs = S6.rows(r['session'], 'G')
        victory = next((g['tic'] for g in gs if g['gamestate'] == 2), None)
        if victory is not None: r = dict(r, session=SR.cut_at(r['session'], victory), sound=SR.cut_at(r['sound'], victory))
        TL.made, TL.seen, TL.parameters, TL.at, TL.respawns = 0, {}, (respawn, fast), at, 0
        lv = (ST.levels_paused if paused else ST.levels)(r, item)
    except (ST.L.ProjectionError, SystemExit) as e:
        return dict(fail=f'genuine side: {e}')
    finally:
        shutil.rmtree(out, ignore_errors=True)
    return dict(levels=[(v['load'], max(v['tic'], 0), v['map'], v['kind'], [(c[0], c[2].hex(), c[1]) for c in v['captures']])
                        for v in lv], tics=len(gs), victory=victory, fields=TL.seen.get('fields'), respawns=TL.respawns,
                grows={g['tic']: tuple(g[c] for c in SESSION_COLUMNS) for g in gs})


def rust(example, item, sched, twin, at=None):
    """The example's levels [(load, tic, map, kind, [(leveltime, digest)])] and its end (kind, tic, detail); with `at`
    the fields of that capture as a third value."""
    p = subprocess.run([str(example), '--wad', str(item['wads'][0]), '--demo', str(item['lmp_path']), '--schedule',
                        str(sched), *(['--twin28'] if twin else []), *(['--fields', f'{at[0]},{at[1]}'] if at else [])],
                       capture_output=True, text=True)
    if p.returncode: return [], ('failed', -1, f'exit {p.returncode}: {p.stderr[-300:]}')
    lv, end = [], None
    for line in p.stdout.splitlines():
        f = line.split(',', 4)
        if f[0] == 'L': lv.append((int(f[1]), int(f[2]), int(f[3]), f[4], []))
        elif f[0] == 'C': lv[int(f[1])][4].append((int(f[2]), f[3]))
        elif f[0] == 'E': end = (f[1], int(f[2]), ','.join(f[3:]))
    fields = [tuple(l.split(',')[1:3]) for l in p.stdout.splitlines() if l.startswith('F,')]
    return (lv, end, fields) if at else (lv, end)


def compare(g, r):
    """(captures equal, first difference or None, (level, leveltime) of a differing capture or None): level by level,
    capture by capture, up to the shorter side."""
    equal, compare.at = 0, None
    for k, (gl, rl) in enumerate(zip(g, r)):
        if gl[1:4] != rl[1:4]: return equal, f'level {k}: genuine {gl[1:4]}, Rust {rl[1:4]}'
        for (gt, gd, *_), (rt, rd) in zip(gl[4], rl[4]):
            if (gt, gd) != (rt, rd):
                compare.at = (k, gt)
                return equal, f'level {k} ({gl[3]}, session tic {gl[1]}) capture {gt}/{rt} differs'
            equal += 1
        if len(rl[4]) > len(gl[4]): return equal, f'level {k}: Rust has {len(rl[4]) - len(gl[4])} captures more'
        if len(rl[4]) < len(gl[4]) and k + 1 < len(r): return equal, f'level {k}: Rust has fewer captures and goes on'
    if len(r) > len(g): return equal, f'Rust has {len(r) - len(g)} levels more'
    return equal, None


def session(a, item):
    """wire29_session_real's G rows [(tic, columns)], captures {tic: (digest, shift digest)} and end line."""
    p = subprocess.run([str(a.session_example), '--wad', str(item['wads'][0]), '--demo', str(item['lmp_path']), '--mode',
                        item['mode']], capture_output=True, text=True)
    rows = [l.split(',') for l in p.stdout.splitlines()]
    end = next((','.join(f[1:]) for f in rows if f[0] == 'E'), f'failed,exit {p.returncode}: {p.stderr[-300:]}')
    return ([(int(f[1]), tuple(map(int, f[2:]))) for f in rows if f[0] == 'G'],
            {int(f[1]): (f[2], f[3]) for f in rows if f[0] == 'C'}, end)


def session_check(g, s):
    """(session tics equal, captures equal, first difference or None) of a session run against the genuine one."""
    caps = {c[2]: c[1] for v in g['levels'] for c in v[4]}
    tics = ncaps = 0
    for t, cols in s[0]:
        if g['grows'].get(t) != cols: return tics, ncaps, f'session tic {t}: G {cols}, genuine {g["grows"].get(t)}'
        if t in caps:
            if s[1].get(t, ('-',))[0] != caps[t]: return tics, ncaps, f'session tic {t}: the capture differs'
            ncaps += 1
        elif t in s[1] and (g['victory'] is None or t < g['victory']): return tics, ncaps, f'session tic {t}: no genuine capture'
        tics += 1
    if s[2].startswith('failed'): return tics, ncaps, s[2]
    if not s[2].startswith('refused') and tics != g['tics']: return tics, ncaps, f'{tics} of {g["tics"]} genuine tics'
    return tics, ncaps, None


def one(kind, a, binary, example, work, kr, flip=None, extra=()):
    """One run of a kind; `flip` (a session tic: the Rust demo's command there changed) and `extra` (the genuine
    runner's switches) are the self-test's faults."""
    k, run = kr
    ST.S = SHIM
    skill, respawn, fast = KINDS[kind]
    d = work / 'demos' / kind; d.mkdir(parents=True, exist_ok=True)
    if kind == 'sessions':
        name, data, mode = f'R{k}_{run.stem.rsplit("-", 1)[-1]}', bytearray(run.read_bytes()), run.name
        mode = 'registered' if '-registered-' in mode else 'retail'
        respawn, fast = int(data[5] != 0), int(data[6] != 0)
    else:
        name, mode = f"{run['map']}_{Path(run['script']).stem.upper()}", 'retail'
        data = bytearray(Path(demo_path(run, d, a.iwad)).read_bytes())
        if skill is not None: data[1] = skill
        data[5], data[6] = respawn, fast
    lmp = d / f'W{k:02}.lmp'; lmp.write_bytes(bytes(data))
    item = dict(SR.item_for(name, lmp, mode, a.iwad, min(data[1], 4), data[3]))
    rec = dict(kind=kind, name=name, map=item['map'], skill=data[1])
    rec['session_tics'] = (len(data) - 14) // 4
    g = genuine(binary, item, work / 'g' / f'{kind}_{name}', respawn, fast, extra=extra, paused=kind == 'sessions')
    if 'fail' in g: return dict(rec, fail=g['fail'])
    sched = work / 'g' / f'{kind}_{name}.sched'
    sched.write_text(''.join(f'{v[1]},{v[2]}\n' for v in g['levels'][1:]))
    ritem = item
    if flip is not None:
        x = bytearray(data); x[13 + 4 * flip] = 0 if x[13 + 4 * flip] else 25
        ritem = dict(item, lmp_path=d / f'F{k:02}.lmp'); ritem['lmp_path'].write_bytes(bytes(x))
    r, end = rust(example, ritem, sched, kind == 'own')
    equal, diff = compare(g['levels'], r)
    gcaps = sum(len(v[4]) for v in g['levels'])
    rec.update(genuine_captures=gcaps, respawns=g['respawns'], genuine_levels=[f'{v[3]}@{v[1]}' for v in g['levels']], victory=g['victory'],
               rust_levels=[f'{v[3]}@{v[1]}' for v in r], equal=equal, diff=diff, end=end, first=compare.at)
    if end is None or end[0] == 'failed': rec['fail'] = f'Rust run failed: {end}'
    elif diff: rec['fail'] = f'differs: {diff}'
    elif end[0] == 'refused':
        rec['witness'] = end[2]
    elif equal != gcaps: rec['fail'] = f'{equal} of {gcaps} genuine captures compared without a refusal'
    if a.diagnose and compare.at:
        at = compare.at
        gf = genuine(binary, item, work / 'g' / f'{kind}_{name}_d', respawn, fast, at)['fields']
        sched.write_text(''.join(f'{v[1]},{v[2]}\n' for v in g['levels'][1:]))
        rf = dict(rust(example, item, sched, kind == 'own', at)[2])
        gd = {p: v.hex() for p, v in gf if not p.startswith('map.')}
        rec['fields'] = [(p, gd.get(p), rf.get(p)) for p in dict.fromkeys([*gd, *rf]) if gd.get(p) != rf.get(p)]
    sched.unlink(missing_ok=True)
    if a.session or kind == 'sessions':
        s = session(a, ritem)
        rec['session_equal'], rec['session_captures'], diff = session_check(g, s)
        rec['session_end'], rec['paused_tics'] = s[2], sum(c[-1] for _, c in s[0])
        if diff: return dict(rec, fail=f'session level: {diff}', session_first=diff)
    return rec


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', type=Path, required=True); ap.add_argument('--work', type=Path, required=True)
    ap.add_argument('--kind', required=True); ap.add_argument('--only'); ap.add_argument('--maps')
    ap.add_argument('--example', type=Path, default=ROOT / 'target/release/examples/wire29_real')
    ap.add_argument('--jobs', type=int, default=2); ap.add_argument('--cc', default='cc')
    ap.add_argument('--diagnose', action='store_true', help='at a differing capture, rerun both sides and log the fields')
    ap.add_argument('--selftest', action='store_true', help='faults on the first run of the first kind must be caught')
    ap.add_argument('--session', action='store_true', help='also the session level (always for sessions)')
    ap.add_argument('--session-example', type=Path, default=ROOT / 'target/release/examples/wire29_session_real')
    ap.add_argument('--recording', type=Path, action='append', default=[])
    ap.add_argument('--selftest-recording', type=int, metavar='TICS')
    a = ap.parse_args()
    kinds = a.kind.split(',')
    if PAUSE_SLOT in kinds: sys.exit('--kind pause: the next commit')
    if any(k not in KINDS for k in kinds): sys.exit(f'--kind: one of {", ".join(KINDS)}')
    if not a.iwad.is_file(): print(f'skip: no IWAD at {a.iwad} (local-only check)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    if not a.example.is_file(): sys.exit(f'build it first: cargo build --release -p doom-sim --example wire29_real')
    binary = build(work / 'runner', a.cc)
    only = {x.upper() for x in a.only.split(',')} if a.only else None
    runs = [r for r in runs_selected(a.maps.split(',') if a.maps else None, None)
            if not only or f"{r['map']}_{Path(r['script']).stem.upper()}" in only]
    if kinds == ['sessions']:
        runs = list(a.recording)
        if a.selftest_recording:
            own = work / 'E1M1-skill4-retail-selftest.lmp'
            p = subprocess.run([str(a.session_example), '--wad', str(a.iwad), '--mode', 'retail', '--record', str(own),
                                '--skill', '4', '--respawn', '--fast', '--tics', str(a.selftest_recording)])
            if p.returncode: sys.exit('the headless recording failed')
            runs.insert(0, own)
        if not runs: sys.exit('--kind sessions needs --recording or --selftest-recording')
    elif 'sessions' in kinds: sys.exit('--kind sessions goes alone')
    (work / 'logs').mkdir(parents=True, exist_ok=True)
    fails = []
    for kind in kinds:
        log = (work / 'logs' / f'{kind}.jsonl').open('a')
        recs = []
        with cf.ProcessPoolExecutor(max(1, min(a.jobs, 5)), mp_context=mp.get_context('fork')) as ex:
            for rec in ex.map(functools.partial(one, kind, a, binary, a.example, work), enumerate(runs)):
                recs.append(rec); log.write(json.dumps(rec) + '\n'); log.flush()
                at = f"{rec['end'][0]} at session tic {rec['end'][1]} of {rec['session_tics']}" if rec.get('end') else '-'
                print(f"{kind} {rec['name']}: {'FAIL ' + rec['fail'] if 'fail' in rec else 'ok'}; {rec.get('equal', 0)} of "
                      f"{rec.get('genuine_captures', '?')} captures equal; Rust {at}"
                      + (f"; witness {rec['witness']}" if 'witness' in rec else '')
                      + (f"; session level {rec['session_equal']} tics, {rec['session_captures']} captures equal, "
                         f"{rec['paused_tics']} paused, end {rec['session_end']}" if 'session_equal' in rec else ''), flush=True)
                for path, gv, rv in rec.get('fields', []): print(f'  field {path}: genuine {gv} Rust {rv}')
        ok = [r for r in recs if 'fail' not in r]
        done = [r for r in ok if 'witness' not in r]
        wit = Counter(r['witness'] for r in ok if 'witness' in r)
        print(f"{kind}: {len(recs)} runs, {sum(r.get('equal', 0) for r in recs)} captures compared equal, {len(done)} complete, "
              f"{sum(r.get('respawns', 0) for r in recs)} genuine respawns in {sum(1 for r in recs if r.get('respawns'))} runs, "
              f"{sum(wit.values())} end at a refusal, {len(recs) - len(ok)} fail")
        if any('session_equal' in r for r in recs):
            sw = Counter(r['session_end'].split(',')[2] for r in recs if r.get('session_end', '').startswith('refused'))
            print(f"  session level: {sum(r.get('session_equal', 0) for r in recs)} session tics and "
                  f"{sum(r.get('session_captures', 0) for r in recs)} captures equal, {sum(r.get('paused_tics', 0) for r in recs)}"
                  f" paused tics, {sum(sw.values())} end at a refusal {dict(sw)}")
        for w, n in wit.most_common():
            fr = [r['end'][1] / r['session_tics'] for r in ok if r.get('witness') == w]
            print(f"  {w}: {n} runs, at {min(fr):.0%} to {max(fr):.0%} of the run (median {sorted(fr)[len(fr) // 2]:.0%})")
        fails += [f"{r['kind']} {r['name']}: {r['fail']}" for r in recs if 'fail' in r]
    if a.selftest:
        # the harness can fail: a changed Rust command at session tic 100, and an extra genuine P_Random draw before
        # session tic 200, must each first differ in the first level at the capture after that tic or later
        for what, kw, t in (('a changed Rust command', dict(flip=100), 100),
                            ('an extra genuine P_Random draw', dict(extra=('--selftest-extra-draw', '200')), 200)):
            rec = one(kinds[0], a, binary, a.example, work, (0, runs[0]), **kw)
            ok = rec.get('first') is not None and rec['first'][0] == 0
            if a.session or kinds[0] == 'sessions':
                # a pause keeps leveltime still, so the session level's tic is the measure
                m = re.search(r'session tic (\d+)', rec.get('session_first') or '')
                ok = ok and m is not None and int(m[1]) >= t
            else:
                ok = ok and rec['first'][1] >= t + 1
            print(f"selftest {'pass' if ok else 'FAIL'}: {what} at session tic {t} ({kinds[0]} {rec['name']}): first "
                  f"difference at (level, leveltime) {rec.get('first')}; session level: {rec.get('session_first')}")
            if not ok: fails.append(f'selftest: {what}')
    print(f'{len(fails)} failures'); [print(f) for f in fails]
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
