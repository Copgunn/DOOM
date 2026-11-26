#!/usr/bin/env python3
"""Local-only: wire 28 (Session::new_finale_and_melt_wire28, the session doom-play now runs) against the genuine
linuxdoom-1.10 engine on the owner's IWAD through the whole session path (wire 28 step 4; never in CI: it reads the
commercial IWAD).

Sets (--set); every demo is a new game in retail mode:
  skill0, skill1  the 61 real runs of tests/reference/m4b_real_results.json (the committed E1M1-E1M9 scripts and DEMO1,
                  m5a_real.demo_path) with the header's skill byte set to 0 or 1, and M8WALK and M8WALK3 at that skill
                  followed by --finale-idle idle tics (E1M8 into the finale): our constructions, written under --work
  e1m7            the five E1M7 runs of the 61 with the skill byte set to 3, and 35 idle tics from the load on skill 3
                  (M7IDLE's header): our constructions
  own             the 61 runs unchanged (their own skills 2 and 3)
  sessions        each --recording (a session doom-play wrote; only read, through a symlink) and, with --idle N, our
                  construction of it followed by N idle tics
Genuine: tools/local/m6a2_play_check.py's genuine() (read-only) per demo: the M6a runner's state variant (strict, the
genuine text, -fwrapv) at screenblocks 10 with the load dumps and at 11, both again with --melt-clock unit, and the
genuine mixer. The level states are projected by the independent DSIM44 encoder (level_snapshot_wire28.py; the
in-process override of wire28_goldens.skill_rows) and, with --wire27, also by the DSIM42 one. Goldens go to --work/ref
in the layout of tests/reference (IWAD-derived, scratch only); a finished demo is not run again (--work/ref/done).
Rust: a scratch crate under --work/gate holds the part 2 gate (crates/doom-game/tests/m6a2_run/mod.rs) with the
exact-once substitutions of WIRE28 (the wire 28 session and demo reader, the DSIM44 capture digested as it is, and a
flipped command for the self-test), m6a/mod.rs and rows.rs unchanged by #[path]. It drives every demo at screenblocks
10 and 11 and compares on every tic the session rows (G, C, I, J, F), every frame (real-map level frames by part 1's
masked-digest rule), every sound row and melt step; after the run the mixer buffers, the end, and the whole DSIM44 state
after every level tic. --wire27 also runs the unchanged ignored test real_maps_finale_and_melt_local (the wire 27
session) on the same goldens with the DSIM42 state rows.
--selftest (the set's first demo): a flipped ticcmd at session tic 100 (Rust side) and an extra P_Random draw before
session tic 200 (the genuine runner's --selftest-extra-draw) must each be reported at or after that tic, the clean run
passing. The IWAD is read in place through a symlink named doomu.wad; only counts, tics, pass/fail, witness names and
digests are printed.

    python3 tools/local/wire28_real.py --iwad PATH --set skill0|skill1|e1m7|own|sessions --work DIR [--recording LMP ..]
                                       [--idle N] [--finale-idle N] [--wire27] [--selftest] [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, json, os, re, subprocess, sys, threading, types
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m6a2_play_check as P2          # noqa: E402  (read-only: genuine)
import m6a_session_real as SR         # noqa: E402  (read-only: item_for, the builds)
import level_snapshot_m6a as S6       # noqa: E402
import level_snapshot_wire28 as E28   # noqa: E402
from m5a_real import runs_selected, demo_path   # noqa: E402

ST, G = SR.ST, SR.G
TESTS = ROOT / 'crates/doom-game/tests'
WIRE28 = [   # (old, new): exact-once substitutions of m6a2_run/mod.rs, in the scratch copy only
    ('Session::new_finale_and_melt(m6a::data', 'Session::new_finale_and_melt_wire28(m6a::data'),
    ('SessionDemo::parse(&lmp)', 'SessionDemo::parse_v3(&lmp)'),
    ('    Ok(demo.commands)\n', '    let mut c = demo.commands;\n    if let Ok(k) = std::env::var("W28_FLIP") {\n'
     '        let x = &mut c[k.parse::<usize>().unwrap()];\n        x.forward = if x.forward == 0 { 25 } else { 0 };\n'
     '    }\n    Ok(c)\n'),
    ('    assert_eq!(s.version(), 43, "wire 27 captures are DSIM 43");\n    let fields = wire26_fields(s);\n',
     '    assert_eq!(s.version(), 44, "wire 28 captures are DSIM 44");\n    let fields: Vec<(String, Vec<u8>)> = '
     's.fields().map(|f| (f.path().to_owned(), f.value().to_vec())).collect();\n'),
    ('    body.extend(42_u32.to_le_bytes());\n', '    body.extend(44_u32.to_le_bytes());\n'),
]
CARGO = '''[package]
name = "wire28-real-gate"
version = "0.0.0"
edition = "2024"
publish = false
[workspace]
[dependencies]
doom-game = {{ path = "{r}/crates/doom-game" }}
doom-render = {{ path = "{r}/crates/doom-render" }}
doom-sim = {{ path = "{r}/crates/doom-sim" }}
doom-sound = {{ path = "{r}/crates/doom-sound" }}
doom-test-evidence = {{ path = "{r}/crates/doom-test-evidence" }}
serde_json = "=1.0.150"
'''
MAIN = '''#[path = "{t}/m6a/mod.rs"]
mod m6a;
#[path = "{r}/crates/doom-sound/tests/common/rows.rs"]
mod rows;
mod m6a2_run;
fn main() {{
    for item in m6a::real_items() {{
        let spec = m6a2_run::Spec {{ item, source: m6a2_run::Source::Part1 }};
        for sb in [10u8, 11] {{
            let o = m6a2_run::run(&spec, m6a2_run::Options::at(sb));
            let c = &o.counts;
            println!("{{}}", serde_json::json!({{"name": spec.item.name, "sb": sb, "tics": c.tics, "level": c.level_tics,
                "intermission": c.intermission_tics, "finale": c.finale_tics, "loads": c.loads, "melts": c.melts,
                "steps": c.melt_steps, "sound": c.sound_rows, "buffers": c.buffers, "states": c.state_captures,
                "levels": c.state_levels, "masked": c.masked_frames, "uncomparable": c.uncomparable_frames,
                "end": o.end.as_ref().map(|e| format!("{{}} {{:?}}", e.tic, e.witness)),
                "diff": o.first_difference, "not_driven": o.not_driven}}));
        }}
    }}
}}
'''
NOTHING, LOCK, DSIM42 = b'\0\0\0\0', threading.Lock(), {}
SHIM = types.SimpleNamespace(**{k: getattr(S6, k) for k in dir(S6) if not k.startswith('__')})
SHIM.Projector, SHIM.VERSION = E28.Projector, E28.VERSION
LEVELS, PLAIN_S, WANT42 = ST.levels, ST.S, [False]


def levels(r, item, *a, **k):
    """m6a_state.levels over DSIM44 (the declared override); with --wire27 first over DSIM42, kept aside."""
    with LOCK:
        if WANT42[0]:
            try: DSIM42[item['name']] = ST.rows_of(item['name'], LEVELS(r, item, *a, **k))
            except ST.L.ProjectionError as e: DSIM42[item['name']] = f'refused: {e}'
        ST.S = SHIM
        try: return LEVELS(r, item, *a, **k)
        finally: ST.S = PLAIN_S


def extra_args(args):
    """m6a_state.run and m6a_goldens.run with `args` appended (the genuine self-test), or restored (args None)."""
    if not hasattr(extra_args, 'plain'): extra_args.plain = (ST.run, G.run)
    st, gr = extra_args.plain
    ST.run, G.run = (st, gr) if args is None else (
        lambda b, i, o, extra=(): st(b, i, o, extra=(*extra, *args)),
        lambda b, i, o, sb, dumps=False, extra=(), mode=None: gr(b, i, o, sb, dumps, (*extra, *args), mode))


def demos(a, work, iwad):
    """[(item name, lump path, label)] of the set; our constructions are written under work/demos."""
    d, out = work / 'demos', []
    d.mkdir(parents=True, exist_ok=True)

    def put(stem, name, data, label):
        p = d / f'{stem}.lmp'
        if not p.is_symlink(): p.write_bytes(data)
        out.append((name, p, label))
    if a.set == 'sessions':
        for k, rec in enumerate(a.recording):
            data, p = rec.read_bytes(), d / f'R{k}.lmp'
            if len(data) < 14 or data[0] != 110 or data[-1] != 0x80 or (len(data) - 14) % 4: sys.exit(f'{rec}: not a session demo')
            if not p.is_symlink(): p.symlink_to(rec.resolve())
            stamp = rec.stem.rsplit('-', 1)[-1]
            out.append((f'R{k}_{stamp}', p, f"the owner's recording {stamp}"))
            if a.idle: put(f'X{k}', f'X{k}_{stamp}', data[:-1] + NOTHING * a.idle + b'\x80', f'{stamp} + {a.idle} idle tics (ours)')
        return out
    skill = {'skill0': 0, 'skill1': 1, 'e1m7': 3}.get(a.set)
    for k, r in enumerate(runs_selected({'E1M7'} if a.set == 'e1m7' else None, None)):
        name = f"{r['map']}_{Path(r['script']).stem.upper()}"
        data = bytearray(Path(demo_path(r, d, iwad)).read_bytes())
        if skill is None: put(f'D{k:02}', name, bytes(data), f'{name} (committed run)'); continue
        data[1] = skill
        put(f'D{k:02}', f'S{skill}_{name}', bytes(data), f'{name} on skill {skill} (ours)')
        if skill < 2 and r['script'] in ('M8WALK.lmp', 'M8WALK3.lmp'):
            put(f'F{k:02}', f'S{skill}_{name}_FIN', bytes(data[:-1]) + NOTHING * a.finale_idle + b'\x80',
                f'{name} on skill {skill} + {a.finale_idle} idle tics (ours)')
        if a.set == 'e1m7' and r['script'] == 'M7IDLE.lmp':
            put('I35', 'S3_E1M7_IDLE35', bytes(data[:13]) + NOTHING * 35 + b'\x80', 'E1M7 skill 3, 35 idle tics from the load (ours)')
    return out


def goldens(items, ref, work, binary, mbin, jobs, extra=None):
    """Runs the genuine side of every item not yet done; writes the reference layout of all items."""
    done = ref / 'done'; done.mkdir(parents=True, exist_ok=True)
    todo = [it for it in items if not (done / f"{it['name']}.json").exists()]
    extra_args(extra)
    try:
        with cf.ThreadPoolExecutor(max(1, min(jobs, 2))) as ex:
            for it, g in zip(todo, ex.map(lambda it: P2.genuine(binary, mbin, it, work / 'g' / it['name']), todo)):
                for rel, text in g['files'].items():
                    (ref / rel).parent.mkdir(parents=True, exist_ok=True); (ref / rel).write_text(text)
                rec = {k: g[k] for k in ('end', 'tics', 'victory', 'state', 'fails', 'line')}
                rec['state42'] = DSIM42.pop(it['name'], None)
                (done / f"{it['name']}.json").write_text(json.dumps(rec))
                print(f"{it['name']}: genuine: {g['line']}" + ''.join(f'; FAIL {f}' for f in g['fails']), flush=True)
    finally:
        extra_args(None)
    recs = {it['name']: json.loads((done / f"{it['name']}.json").read_text()) for it in items}
    manifest = [dict(name=it['name'], lump=it['lump'], mode='retail', wadset='iwad', map=it['map'], skill=it['skill'],
                     lmp=str(it['lmp_path'])) for it in items]
    (ref / 'm6a_manifest.json').write_text(json.dumps(dict(schema='DOOM-M6A-MANIFEST-v1', wadsets={'iwad': [
        str(items[0]['wads'][0])]}, items=manifest), indent=1))
    (ref / 'm6a_results.json').write_text(json.dumps(dict(schema='DOOM-M6A-RESULTS-v1', items={
        n: dict(end=r['end'], session_tics=r['tics']) for n, r in recs.items()}), indent=1))
    head = ST.results_text({}).rstrip('\n').split('\n')
    (ref / 'm6a_state.txt').write_text('\n'.join(head + [l for r in recs.values() for l in r['state']]) + '\n')
    if all(isinstance(r['state42'], list) for r in recs.values()):
        r27 = ref.parent / (ref.name + '27'); r27.mkdir(exist_ok=True)
        for p in ref.iterdir():
            if p.name not in ('m6a_state.txt', 'done') and not (r27 / p.name).is_symlink(): (r27 / p.name).symlink_to(p)
        (r27 / 'm6a_state.txt').write_text('\n'.join(head + [l for r in recs.values() for l in r['state42']]) + '\n')
    return recs


def gate_build(work):
    crate = work / 'gate'
    (crate / 'src').mkdir(parents=True, exist_ok=True)
    src = (TESTS / 'm6a2_run/mod.rs').read_text()
    for old, new in WIRE28:
        if src.count(old) != 1: sys.exit(f'a WIRE28 substitution does not apply once: {old[:60]!r}')
        src = src.replace(old, new)
    (crate / 'src/m6a2_run.rs').write_text(src)
    (crate / 'src/main.rs').write_text(MAIN.format(t=TESTS, r=ROOT))
    (crate / 'Cargo.toml').write_text(CARGO.format(r=ROOT))
    if not (crate / 'Cargo.lock').exists(): (crate / 'Cargo.lock').write_bytes((ROOT / 'Cargo.lock').read_bytes())
    target = Path(os.environ.get('CARGO_TARGET_DIR', crate / 'target'))
    p = subprocess.run(['cargo', 'build', '--release', '--offline', '--manifest-path', str(crate / 'Cargo.toml')],
                       capture_output=True, text=True, env=dict(os.environ, CARGO_TARGET_DIR=str(target)))
    if p.returncode: sys.stderr.write(p.stderr[-3000:]); sys.exit('gate build FAILED')
    return target / 'release/wire28-real-gate'


def gate(binary, ref, only=None, flip=None):
    env = dict(os.environ, M6A_REFERENCE_DIR=str(ref))
    if only: env['M6A_REAL_ONLY'] = only
    if flip is not None: env['W28_FLIP'] = str(flip)
    p = subprocess.run([str(binary)], capture_output=True, text=True, env=env)
    if p.returncode: sys.exit(f'gate exit {p.returncode}: {p.stderr[-1500:]}')
    return [json.loads(l) for l in p.stdout.splitlines() if l.startswith('{')]


def first_tic(o):
    m = re.search(r'tic (\d+)', o['diff'] or '')
    return int(m[1]) if m else None


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', type=Path, required=True); ap.add_argument('--work', type=Path, required=True)
    ap.add_argument('--set', choices=['skill0', 'skill1', 'e1m7', 'own', 'sessions'], required=True)
    ap.add_argument('--recording', type=Path, action='append', default=[])
    ap.add_argument('--idle', type=int, default=0); ap.add_argument('--finale-idle', type=int, default=2400)
    ap.add_argument('--wire27', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--jobs', type=int, default=2); ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    if not a.iwad.is_file(): print(f'skip: no IWAD at {a.iwad} (local-only check)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    iwad = work / 'iwad' / 'doomu.wad'
    iwad.parent.mkdir(parents=True, exist_ok=True)
    if not iwad.is_symlink(): iwad.symlink_to(a.iwad.resolve())
    WANT42[0] = a.wire27
    ST.levels = levels
    items, labels = [], {}
    for name, lmp, label in demos(a, work, iwad):
        data = lmp.read_bytes()
        items.append(SR.item_for(name, lmp, 'retail', iwad, data[1], data[3])); labels[name] = label
    binary = ST.build(work / 'runner', a.cc)
    mbin = SR.mb.build('strict', work / 'mixer', a.cc, quiet=True)
    recs = goldens(items, work / 'ref', work, binary, mbin, a.jobs)
    rust = gate_build(work)
    fails, per_map, refusals = [], defaultdict(lambda: [0, 0, 0, 0]), []
    for o in gate(rust, work / 'ref'):
        name, r = o['name'], recs[o['name']]
        ok = o['diff'] is None and o['not_driven'] is None
        t = o['tics'] if ok else (first_tic(o) if first_tic(o) is not None else o['tics'])
        refused = o['end'] is not None and not ok
        if refused: refusals.append(f"{name} sb{o['sb']}: Rust refused session tic {o['end']}; genuine {r['end']} after {r['tics']}")
        if not ok: fails.append(f"{name} sb{o['sb']}: {o['diff'] or o['not_driven']}")
        print(f"{name} sb{o['sb']}: {labels[name]}: {'pass' if ok else 'FAIL'}; {t} of {r['tics']} session tics compared "
              f"(level {o['level']}, intermission {o['intermission']}, finale {o['finale']}); victory at "
              f"{r['victory']}; genuine end {r['end']}; {o['loads']} loads, {o['melts']} melts of {o['steps']} steps, "
              f"{o['sound']} sound rows, {o['buffers']} buffers, {o['states']} states on {o['levels']} levels, "
              f"{o['masked']} masked frames, {o['uncomparable']} uncomparable; Rust end {o['end']}", flush=True)
        if o['sb'] == 10:
            m = per_map[next(it['map'] for it in items if it['name'] == name)]
            m[0] += 1; m[1] += t; m[2] += ok; m[3] += refused
    for k in sorted(per_map):
        print(f'E1M{k}: {per_map[k][0]} runs, {per_map[k][1]} session tics compared (screenblocks 10; 11 the same when '
              f'passing), {per_map[k][2]} pass, {per_map[k][3]} refused')
    print(f"total: {len(items)} demos, {sum(v[1] for v in per_map.values())} session tics compared, "
          f"{sum(v[2] for v in per_map.values())} pass, {len(refusals)} refusals (both sizes)")
    for f in refusals: print('refusal: ' + f)
    if a.wire27:
        r27 = work / 'ref27'
        if not r27.exists(): fails.append('--wire27: the DSIM42 projection refused an item; no wire 27 run')
        else:
            p = subprocess.run(['cargo', 'test', '--release', '--offline', '-p', 'doom-game', '--test', 'm6a2_session', '--',
                                '--ignored', '--exact', 'real_maps_finale_and_melt_local', '--nocapture'], cwd=ROOT,
                               capture_output=True, text=True, env=dict(os.environ, M6A_REFERENCE_DIR=str(r27)))
            runs = re.findall(r'^\S+ sb1[01]: genuine .*$', p.stderr, re.M)
            tics = sum(int(x) for x in re.findall(r'[{,] tics: (\d+)', '\n'.join(runs)))
            print(f'wire 27 (real_maps_finale_and_melt_local, unchanged): exit {p.returncode}; {len(runs)} runs; '
                  f'{tics} session tics compared (both sizes)')
            if p.returncode: fails.append(f'wire 27 gate exit {p.returncode}')
    if a.selftest:
        it = items[0]
        n, clean = it['name'], gate(rust, work / 'ref', only=it['name'])
        flip = gate(rust, work / 'ref', only=n, flip=100)
        ok = all(o['diff'] is None for o in clean) and all((first_tic(o) or -1) >= 100 for o in flip)
        print(f"selftest {'pass' if ok else 'FAIL'}: a flipped ticcmd at session tic 100 ({n}): reported at session tics "
              f"{[first_tic(o) for o in flip]} (sb10, sb11)")
        selfref = work / 'ref_self'
        goldens([dict(it, name=n)], selfref, work, binary, mbin, 1, extra=('--selftest-extra-draw', '200'))
        draw = gate(rust, selfref)
        ok2 = all((first_tic(o) or -1) >= 200 for o in draw)
        print(f"selftest {'pass' if ok2 else 'FAIL'}: an extra P_Random draw before session tic 200 (genuine side, {n}): "
              f"reported at session tics {[first_tic(o) for o in draw]} (sb10, sb11)")
        fails += [f'selftest {w}' for w, x in (('flip', ok), ('draw', ok2)) if not x]
    print(f'{len(items)} demos; {len(fails)} failures')
    for f in fails: print(f)
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
