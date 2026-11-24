#!/usr/bin/env python3
"""Local-only: doom-play sessions under wires 26 and 27 against the genuine strict linuxdoom-1.10 build on the owner's
IWAD, state compared after every tic (wire 27 real-map check; never in CI: it reads the commercial IWAD).

Each --recording is a session doom-play wrote (the owner's: read only, never changed, moved or copied); each
--construction is a demo of ours (for example a recording extended with idle tics; --idle N builds one per recording
under --work, labelled as ours). Every demo is one new-game level (its header's map and skill) in retail mode.

Per demo:
  genuine   tools/reference/wire27_goldens.py's strict build (the genuine text, -fwrapv; its own build(), imported
            read-only) runs the demo with --game-mode retail, --render none and the M5c recommended switches; the dumps
            are projected by the independent DSIM43 encoder (wire27_goldens.project, level_snapshot_wire27.py), one
            state digest per capture (capture j = the state after j tics). The session stream's first G row in the
            finale (gamestate 2) is the level's end.
  witness   the same demo in the witness build: every P_InterceptVector call classified by wire27_goldens.classify
            (rows that more than one linedef matches are classified against each candidate and must agree); counts per
            caller and case, split at the recording's length (--split), and the wire-27 end the witness predicts.
  rust      a scratch crate under --work (doom-sim by path, the repository unchanged) runs the demo under wire 26
            (original_player_episode1_v4_level) and wire 27 (..._v5_level), NewGame, and prints the digest of every
            capture as tests wire27_intercept.rs computes it (a DSIM42 capture renamed to DSIM43).
  compare   every capture both sides have must be equal; a Rust refusal at session tic N keeps capture N (refusals
            other than LevelExited are reported with the genuine witness's classification of that tic); LevelExited
            must come at the tic the genuine level ended.

Tic convention (owner): a session tic is the 0-based index of a tic, the number of tics completed before it.
--selftest proves the comparison can fail, on the first demo: a flipped ticcmd at session tic 100 (Rust side, in
memory) and an extra P_Random draw before session tic N-10 of its N tics (the genuine runner's --selftest-extra-draw;
late, so that the desynchronised run reaches no reborn load) must each be detected at their first affected capture.

The IWAD is used in place: --work gets a symlink to it named doomu.wad (the runner's retail name check) and a symlink to
each recording; nothing derived from the IWAD or the sessions is printed but counts, tics and pass/fail.

    python3 tools/local/wire27_session_real.py --iwad PATH [--recording LMP ..] [--construction LMP ..] [--idle N]
                                               --work DIR [--selftest] [--cc CC]
"""
import argparse, os, re, shutil, subprocess, sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import wire27_goldens as W          # noqa: E402  (read-only: build, project, classify, map_lines, RECOMMENDED)
import level_snapshot_m6a as S6     # noqa: E402

CARGO = '''[package]
name = "wire27-session-real"
version = "0.0.0"
edition = "2024"
publish = false
[workspace]
[dependencies]
doom-sim = {{ path = "{r}/crates/doom-sim" }}
doom-map = {{ path = "{r}/crates/doom-map" }}
doom-wad = {{ path = "{r}/crates/doom-wad" }}
sha2 = "=0.10.9"
'''
MAIN = r'''use doom_sim::{demo::parse_demo_episode1_v2, *};
use sha2::{Digest, Sha256};
fn framed(out: &mut Vec<u8>, path: &str, value: &[u8]) {
    out.extend((path.len() as u32).to_le_bytes()); out.extend(path.as_bytes());
    out.extend((value.len() as u64).to_le_bytes()); out.extend(value);
}
// tests/wire27_intercept.rs `digest`: a DSIM42 capture renamed to DSIM43, map.* and events.* left out, then events.sounds.*
fn digest(s: &Snapshot) -> String {
    let rename = s.version() == 42;
    assert!(rename || s.version() == 43);
    let kept: Vec<_> = s.fields().filter(|f| !f.path().starts_with("map.") && !f.path().starts_with("events.")).collect();
    let mut body = b"DSIM".to_vec();
    body.extend(43_u32.to_le_bytes()); body.extend((kept.len() as u64).to_le_bytes());
    for f in &kept {
        let v = match (rename, f.path()) {
            (true, "profile") => 27_u32.to_le_bytes().to_vec(),
            (true, "context.schema") => PLAYER_EPISODE1_V5_CONTEXT_SCHEMA.to_vec(),
            _ => f.value().to_vec(),
        };
        framed(&mut body, f.path(), &v);
    }
    for f in s.fields().filter(|f| f.path().starts_with("events.sounds.")) { framed(&mut body, f.path(), f.value()); }
    Sha256::digest(&body).iter().map(|b| format!("{b:02x}")).collect()
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (wire, wad, lmp) = (a[1].as_str(), std::fs::read(&a[2]).unwrap(), std::fs::read(&a[3]).unwrap());
    let parsed = doom_wad::Wad::parse(&wad, doom_wad::Limits::default()).unwrap();
    let number = lmp[3];
    let map = doom_map::Map::from_wad(&parsed, &format!("E1M{number}"), doom_map::MapLimits::default()).unwrap();
    let (header, demo) = parse_demo_episode1_v2(&lmp, number).unwrap();
    let skill = [Skill::Baby, Skill::Easy, Skill::Medium, Skill::Hard, Skill::Nightmare][header.skill as usize];
    let build = if wire == "27" { Simulation::original_player_episode1_v5_level } else { Simulation::original_player_episode1_v4_level };
    let mut s = build(map, number, skill, MapContextOptions::default(), LevelStart::NewGame).unwrap();
    let mut commands = demo.commands;
    if let Some(k) = a.get(4) { let c = &mut commands[k.parse::<usize>().unwrap()]; c.forward = if c.forward == 0 { 25 } else { 0 }; }
    println!("C 0 {}", digest(&s.snapshot()));
    let none = TicCommand::default();
    for (t, c) in commands.iter().enumerate() {
        if let Err(e) = s.advance([*c, none, none, none]) { println!("R {t} {e:?} {}", digest(&s.snapshot())); return; }
        println!("C {} {}", t + 1, digest(&s.snapshot()));
    }
    println!("E {}", commands.len());
}
'''


def rust_build(work):
    crate = work / 'rust'
    (crate / 'src').mkdir(parents=True, exist_ok=True)
    (crate / 'Cargo.toml').write_text(CARGO.format(r=ROOT))
    (crate / 'src/main.rs').write_text(MAIN)
    if not (crate / 'Cargo.lock').exists(): shutil.copyfile(ROOT / 'Cargo.lock', crate / 'Cargo.lock')
    target = Path(os.environ.get('CARGO_TARGET_DIR', crate / 'target'))
    p = subprocess.run(['cargo', 'build', '--release', '--offline', '--manifest-path', str(crate / 'Cargo.toml')],
                       capture_output=True, text=True, env=dict(os.environ, CARGO_TARGET_DIR=str(target)))
    if p.returncode: sys.stderr.write(p.stderr[-3000:]); sys.exit('rust build FAILED')
    return target / 'release/wire27-session-real'


def rust_run(binary, wire, iwad, lmp, flip=None):
    p = subprocess.run([str(binary), str(wire), str(iwad), str(lmp)] + ([str(flip)] if flip is not None else []),
                       capture_output=True, text=True)
    caps, end = [], None
    for line in p.stdout.splitlines():
        f = line.split()
        if f[0] == 'C': assert int(f[1]) == len(caps), line; caps.append(f[2])
        elif f[0] == 'R': end = ('refused', int(f[1]), ' '.join(f[2:-1]), f[-1])
        elif f[0] == 'E': end = ('end', int(f[1]))
    if p.returncode or end is None: sys.exit(f'rust run FAILED: {p.stderr[-2000:]}')
    return caps, end


def genuine_run(binary, item, out, witness=False, extra=()):
    """W.run's command in retail mode (its own hard-codes registered), --render none, no frames."""
    if out.exists(): shutil.rmtree(out)
    (out / 'dumps').mkdir(parents=True)
    cmd = [str(binary), '--wad', str(item['wads'][0]), '--wad', str(item['lmp_path']), '--demo', item['name'],
           '--game-mode', 'retail', '--out', str(out / 'dumps'), *[x for k, v in W.M5C.RECOMMENDED.items() for x in (k, v)],
           '--render', 'none', *extra, '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt')]
    env = dict(W.ENV, WIRE27_WITNESS=str(out / 'witness.txt')) if witness else W.ENV
    p = subprocess.run(cmd, capture_output=True, text=True, env=env)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    return dict(code=p.returncode, stderr=p.stderr, marker=' end=marker ' in p.stdout, sound=rd('sound.txt'),
                session=rd('session.txt'), dumps=out / 'dumps', witness=out / 'witness.txt')


def genuine(binary, item, out, extra=()):
    r = genuine_run(binary, item, out, extra=extra)
    if r['code'] or W.UB.search(r['stderr']): sys.exit(f"{item['name']}: genuine exit {r['code']} {r['stderr'][-500:]}")
    caps, _mapg = W.project(r, item)
    victory = next((g['tic'] for g in S6.rows(r['session'], 'G') if g['gamestate'] == 2), None)
    shutil.rmtree(out, ignore_errors=True)
    return dict(caps=[d for _j, _t, d in caps], victory=victory, marker=r['marker'])


def classify_all(item, path, split):
    """Counts of the witness rows (module text); every row must classify."""
    lines = W.map_lines(item)
    index = {}
    for i, (_x, _y, dx, dy) in enumerate(lines): index.setdefault((dy >> 8, dx >> 8), []).append(i)
    sites, calls, counts, ends, bad, wraps = {}, Counter(), Counter(), [], 0, []
    with open(path) as fh:
        for row in fh:
            f = row.split()
            cand = sorted(set(index.get((int(f[10]), int(f[14])), []) + index.get((int(f[14]), int(f[10])), []))) \
                if f[3] == 'line' and f[8] in ('2', '4') else []
            try:
                c = W.classify(row, sites, [lines[h] for h in cand])
            except ValueError as e:
                if 'linedefs match' not in str(e) or len(cand) < 2: bad += 1; continue
                each = []
                for h in cand:
                    try: each.append(W.classify(row, sites, [lines[h]]))
                    except ValueError: pass
                if not each or any(dict(x, line=0) != dict(each[0], line=0) for x in each): bad += 1; continue
                c = each[0]
            part = 'recorded' if c['tic'] < split else 'extension'
            calls[(part, c['caller'])] += 1
            for case in c['cases']: counts[(part, c['caller'], case)] += 1
            if c['wrapped']: counts[(part, c['caller'], 'wrapped')] += 1; wraps.append((c['tic'], c['caller']))
            if c['branch'] == 'saturation': counts[(part, c['caller'], 'saturation_branch_any')] += 1
            if 'absmin' in c['cases'] or c['wrapped'] and ('saturation' in c['cases'] or c['caller'] in W.NOT_WRAPPED):
                ends.append((c['tic'], 'AbsMin' if 'absmin' in c['cases'] else 'Saturation' if 'saturation' in c['cases']
                             else 'checked caller ' + c['caller']))
    return dict(calls=calls, counts=counts, end27=ends[0] if ends else None, bad=bad, wraps=wraps)


def compare(g, caps, end):
    """(verdict, tics compared, level ended in both) of one Rust run against the genuine captures."""
    n = min(len(g['caps']), len(caps))
    diff = next((j for j in range(n) if g['caps'][j] != caps[j]), None)
    if diff is not None: return f'FAIL: first differing capture {diff} (session tic {diff - 1})', max(diff - 1, 0), False
    if end[0] == 'end':
        ok = len(caps) == len(g['caps']) == end[1] + 1 and g['victory'] is None
        return ('pass' if ok else f'FAIL: {len(caps)} Rust captures, {len(g["caps"])} genuine'), end[1], False
    _, t, kind, kept = end
    if t >= len(g['caps']) or kept != g['caps'][t]: return f'FAIL: refused at {t} ({kind}), kept state not capture {t}', t, False
    if kind == 'LevelExited':
        ok = g['victory'] == t and len(g['caps']) == t + 1
        return ('pass' if ok else f'FAIL: LevelExited at {t}, genuine level ended at {g["victory"]}'), t, ok
    return f'refused at session tic {t} ({kind})', t, False


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', type=Path, required=True)
    ap.add_argument('--recording', type=Path, action='append', default=[])
    ap.add_argument('--construction', type=Path, action='append', default=[])
    ap.add_argument('--idle', type=int, default=0); ap.add_argument('--work', type=Path, required=True)
    ap.add_argument('--selftest', action='store_true'); ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    if not a.iwad.is_file(): print(f'skip: no IWAD at {a.iwad} (local-only check)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    (work / 'demos').mkdir(parents=True, exist_ok=True)
    iwad = work / 'iwad' / 'doomu.wad'
    iwad.parent.mkdir(exist_ok=True)
    if not iwad.is_symlink(): iwad.symlink_to(a.iwad.resolve())
    demos = []      # (label, lump path, recorded length or None)
    for k, rec in enumerate(a.recording):
        data = rec.read_bytes()
        if len(data) < 14 or data[0] != 110 or data[-1] != 0x80 or (len(data) - 14) % 4: sys.exit(f'{rec}: not a session demo')
        link = work / 'demos' / f'R{k}.lmp'
        if not link.is_symlink(): link.symlink_to(rec.resolve())
        n = (len(data) - 14) // 4
        demos.append((f"owner's recording {rec.name}", link, n))
        if a.idle:
            ext = work / 'demos' / f'X{k}.lmp'
            ext.write_bytes(data[:-1] + b'\0' * 4 * a.idle + b'\x80')
            demos.append((f'our construction: {rec.name} + {a.idle} idle tics', ext, n))
    for k, c in enumerate(a.construction):
        link = work / 'demos' / f'C{k}.lmp'
        if not link.is_symlink(): link.symlink_to(c.resolve())
        demos.append((f'our construction {c.name}', link, None))
    strict, wit = (W.build(b, work / 'build', a.cc) for b in ('strict', 'witness'))
    rust = rust_build(work)
    fails = []
    for label, lmp, split in demos:
        item = dict(name=lmp.stem, map=lmp.read_bytes()[3], wads=[iwad], lmp_path=lmp)
        g = genuine(strict, item, work / 'run' / lmp.stem)
        wr = genuine_run(wit, item, work / 'run' / (lmp.stem + 'w'), witness=True)
        wf = classify_all(item, wr['witness'], split if split is not None else 1 << 40)
        shutil.rmtree(work / 'run' / (lmp.stem + 'w'), ignore_errors=True)
        if wf['bad']: fails.append(f'{label}: {wf["bad"]} witness rows did not classify')
        ended = f"genuine level ended at session tic {g['victory']}" if g['victory'] is not None else 'genuine level did not end'
        print(f'{label}: {len(g["caps"]) - 1} genuine level tics; {ended}; witness predicts wire 27 '
              f"{'complete' if wf['end27'] is None else 'refused:%d:%s' % wf['end27']}; {wf['bad']} unclassified rows")
        for wire in (26, 27):
            caps, end = rust_run(rust, wire, iwad, lmp)
            verdict, tics, lv = compare(g, caps, end)
            if verdict.startswith('FAIL'): fails.append(f'{label} wire {wire}: {verdict}')
            if verdict.startswith('refused'):
                hit = wf['end27'] is not None and wf['end27'][0] == end[1]
                verdict += f"; witness at that tic: {wf['end27'][1] if hit else 'no wire-27 refusal predicted'}"
            print(f'  wire {wire}: {tics} tics compared; {verdict}; level ended in both: {"yes" if lv else "no"}', flush=True)
        for (p, c), v in sorted(wf['calls'].items()): print(f'  calls {p} {c}: {v}')
        for (p, c, k), v in sorted(wf['counts'].items()): print(f'  witness {p} {c} {k}: {v}')
        total = lambda k, c=None: sum(v for (_p, cc, kk), v in wf['counts'].items() if kk == k and c in (None, cc))
        print(f"  summary: {len(wf['wraps'])} wrapping calls, first at session tic "
              f"{wf['wraps'][0][0] if wf['wraps'] else None}; shoot_thing wrapping {total('wrapped', 'shoot_thing')}; "
              f"wrapped saturation {total('saturation')}; absmin {total('absmin')}; saturation branch (any operands) "
              f"{total('saturation_branch_any')}")
    if a.selftest and demos:
        label, lmp, _ = demos[0]
        n = (len(lmp.read_bytes()) - 14) // 4
        item = dict(name=lmp.stem, map=lmp.read_bytes()[3], wads=[iwad], lmp_path=lmp)
        g = genuine(strict, item, work / 'run' / 'self')
        k = 100
        caps, end = rust_run(rust, 27, iwad, lmp, flip=k)
        v = compare(g, caps, end)[0]
        ok = v.startswith(f'FAIL: first differing capture {k + 1} ')
        print(f'selftest {"pass" if ok else "FAIL"}: a flipped ticcmd at session tic {k} ({label}): {v}')
        # late, so that the desynchronised run cannot reach a reborn load (a second load) before the demo ends
        d = n - 10
        gd = genuine(strict, item, work / 'run' / 'selfdraw', extra=['--selftest-extra-draw', str(d)])
        caps, end = rust_run(rust, 27, iwad, lmp)
        v2 = compare(gd, caps, end)[0]
        ok2 = v2.startswith(f'FAIL: first differing capture {d + 1} ')
        print(f'selftest {"pass" if ok2 else "FAIL"}: an extra P_Random draw before session tic {d} (genuine side): {v2}')
        fails += [f'selftest {w}' for w, o in (('flip', ok), ('draw', ok2)) if not o]
    print(f'{len(demos)} demos; {len(fails)} failures')
    for f in fails: print(f)
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
