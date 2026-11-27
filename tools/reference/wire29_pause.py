#!/usr/bin/env python3
"""Wire 29, the pause in the session: genuine session streams for the session's comparison, and two items the M6b
note left as "read, not run" (docs/M6B_REFERENCE_INVESTIGATION.md §4.2). Expected values come from the genuine
linuxdoom-1.10 runner and the independent DSIM45 encoder level_snapshot_wire29.py, never from Rust output.

Streams (strict build of the M6b runner, tools/reference/m6b_build.py, read-only, run as m6b_goldens.run runs an
item): the session stream (DOOM-M6A-SESSION-v1) of every M6b item whose stream tests/reference/m6b_session/ does not
hold; each must hash to the session_sha256 of the item's I row in the frozen tests/reference/m6b_genuine.txt, where
m6b_goldens.py has shown the overflow and address builds identical.

New items (the strict, overflow and address builds identical in dumps, sound and session streams, with a clean exit;
ASAN_OPTIONS=detect_leaks=0: leak detection disabled, not LeakSanitizer coverage; LOADP's address build, which loads a
second level, must instead stop at exactly the M6a-declared report st_stuff.c:1292 after a prefix of the strict dumps,
as m6b_goldens.py's ADDRESS_STOPS rule for PAUSEWI). Each is a committed demo edited by
the byte edits its text file lists (session tics):
  DEADSPEC  tests/reference/m6a_scripts/REBLATE.lmp (E1M2 of the M6a episode set; the player dies on the 20-damage
            floor at session tic 160): 0x81 on a dead tic pauses (g_game.c:704-708); 0x81 again unpauses, and on that
            tic P_DeathThink reads the raw byte (p_user.c:227, 258-262), which has no BT_USE: no reborn; 0x83 a few
            tics later toggles nothing (low bits 11) and has BT_USE: the reborn (p_user.c:227-228), loaded next tic.
  LOADP     tests/reference/m6a_scripts/CHAIN.lmp (PAUSEWI's base): PAUSEWI's three pause commands in the
            intermission, then 0x81 on the load tic of E1M2: G_DoLoadLevel clears paused (g_game.c:494) and the
            command sets it again (698-708), so the new level's tic 0 does not run while the carried chainsaw's
            sfx_sawup (p_pspr.c:145-146) has started inside P_SetupLevel; 0x81 four tics later runs tic 0.
  TURBOP    tests/reference/m6b_scripts/PAUSE.lmp (skill 3): a forward move of 60 (above TURBOTHRESHOLD 0x32,
            g_game.c:672) on session tic 256 inside the long pause (gametic 256: gametic & 31 == 0 and
            (gametic >> 5) & 3 == 0 = consoleplayer) and on tic 160 (gametic 160: (160 >> 5) & 3 == 1, no message):
            the message is set while paused and HU_Ticker, which runs while paused (g_game.c:733), shows it. Also run
            rendered at screenblocks 10 for the HUD's state (the frame stream's P and X rows).

Output (written without --check; --check regenerates and compares, writing nothing):
  tests/reference/wire29_session/<ITEM>.session          the session stream of each M6b item without one
  tests/reference/wire29_session/<NEW>.lmp, <NEW>.txt     each new item's demo and its edits
  tests/reference/wire29_session/<NEW>.session            its session stream
  tests/reference/wire29_session/<new>_state.txt          DOOM-WIRE29-PAUSE-v1: the I row (as wire29_stale's), a C row
      per capture of the first level (its state digest over DSIM45), O rows (observed facts) and, for TURBOP, H rows
      (tic, message_on of the frame stream's P row) and X rows (tic, the message shown)

    wire29_pause.py [--check] [--selftest] [--only ITEM[,ITEM]] [--work DIR] [--cc CC]
"""
import argparse, hashlib, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import level_snapshot as L           # noqa: E402
import level_snapshot_m5c as M5S     # noqa: E402
import level_snapshot_m6a as S6      # noqa: E402
import level_snapshot_wire29 as E    # noqa: E402
import m5c_reference as M5C          # noqa: E402  (read-only: RECOMMENDED)
import m6a_state as ST               # noqa: E402  (read-only: Shifted, extra_lumps)
import m6b_build as B                # noqa: E402  (read-only)
import m6b_goldens as G              # noqa: E402  (read-only: run, clean, ENV, GENUINE, DIRS)
import m6b_wad as WAD                # noqa: E402  (read-only)
import wire29_goldens as WG          # noqa: E402  (read-only: digest)

DIR = 'tests/reference/wire29_session'
SCHEMA = 'DOOM-WIRE29-PAUSE-v1'
PAUSE, SPECIAL_USE = 0x81, 0x83
NEW = {
    'DEADSPEC': dict(base='tests/reference/m6a_scripts/REBLATE.lmp', wads=WAD.EPISODE,
                     edits=[('buttons', 170, PAUSE), ('buttons', 180, PAUSE), ('buttons', 185, SPECIAL_USE), ('cut', 260)],
                     what='special bytes on a dead player\'s tics: 0x81 pauses and unpauses (no reborn), 0x83 asks for '
                          'the reborn'),
    'LOADP': dict(base='tests/reference/m6a_scripts/CHAIN.lmp', wads=WAD.EPISODE, address_stop=True,
                  edits=[('buttons', 127, PAUSE), ('buttons', 129, PAUSE), ('buttons', 135, PAUSE),
                         ('buttons', 142, PAUSE), ('buttons', 146, PAUSE), ('cut', 170)],
                  what='PAUSEWI\'s pauses in the intermission, then 0x81 on the load tic of E1M2 (G_DoLoadLevel '
                       'clears paused, the command sets it: the level\'s tic 0 does not run, while the carried '
                       'chainsaw\'s sfx_sawup starts at the load) and 0x81 four tics later (tic 0 runs)'),
    'TURBOP': dict(base='tests/reference/m6b_scripts/PAUSE.lmp', wads=WAD.WADSET, sb=10,
                   edits=[('forward', 160, 60), ('forward', 256, 60)],
                   what='the turbo message during a pause: forward 60 on gametic 256 (the message) and 160 (none)'),
}
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def pinned_items():
    """The M6b items without a committed session stream, with their pinned session digests."""
    out = {}
    for line in G.GENUINE.read_text().splitlines():
        f = line.split(',')
        if f[0] == 'I' and not (G.DIRS['session'] / f'{f[1]}.session').exists(): out[f[1]] = f[11]
    return out


def lump(name):
    """(the edited demo, its tic count)."""
    it = NEW[name]
    data = bytearray((ROOT / it['base']).read_bytes())
    assert data[-1] == 0x80 and (len(data) - 14) % 4 == 0
    for e in it['edits']:
        tics = (len(data) - 14) // 4
        assert e[1] < tics + (e[0] == 'cut'), (name, e)
        if e[0] == 'cut': data = data[:13 + 4 * e[1]] + b'\x80'
        else: data[13 + 4 * e[1] + {'forward': 0, 'buttons': 3}[e[0]]] = e[2]
    return bytes(data), (len(data) - 14) // 4


def text(name):
    it = NEW[name]
    return (f'# Wire 29 item {name}: {it["what"]}.\n# Generated by tools/reference/wire29_pause.py from {it["base"]} by the '
            'byte edits below (session tics; the header kept).\n'
            + '\n'.join(' '.join(str(x) if not isinstance(x, int) or k < 2 or e[0] != 'buttons' else hex(x)
                                 for k, x in enumerate(e)) for e in it['edits']) + '\n')


def run(binary, name, out, wads, sb=None, extra=()):
    """One runner process (m6b_goldens.run's switches) of the demo lump NAME in the last file of `wads`."""
    if out.exists(): shutil.rmtree(out)
    (out / 'dumps').mkdir(parents=True)
    cmd = [str(binary), *[x for w in wads for x in ('--wad', str(w))], '--demo', name, '--game-mode', 'registered',
           '--out', str(out / 'dumps'), *[x for k, v in M5C.RECOMMENDED.items() for x in (k, v)],
           *(['--render', 'every', '--screenblocks', str(sb), '--frames', str(out / 'frames.txt')] if sb else ['--render', 'none']),
           *extra, '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt'), '--m6b-out', str(out / 'm6b.txt')]
    p = subprocess.run(cmd, capture_output=True, text=True, env=G.ENV, timeout=600)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    names = ['load.txt'] + sorted(n for n in os.listdir(out / 'dumps') if n.startswith('tic_'))
    return dict(code=p.returncode, stdout=p.stdout, stderr=p.stderr, marker=' end=marker ' in p.stdout, sound=rd('sound.txt'),
                session=rd('session.txt'), frames=rd('frames.txt'), names=names, dumps=out / 'dumps',
                digests=[sha((out / 'dumps' / n).read_bytes()) for n in names])


def project(r, wads, header):
    """The DSIM45 state digest of every capture of the first level (wire29_goldens.project's projection)."""
    it = dict(wads=wads[:-1], lmp_path=wads[-1])
    loads = S6.rows(r['session'], 'L')
    kind, p, _h, _s = S6.start_from_rows(r['session'], r['sound'], 0, str(wads[0]))
    mapdata = L.MapData(str(wads[0]), f"E1M{loads[0]['map']}", ST.extra_lumps(it))
    sounds = ST.Shifted(M5S.SoundStream(text=r['sound']), max(loads[0]['tic'], 0))
    skill, respawn, fast = min(header[1], 4), header[5], header[6]
    proj = E.Projector(mapdata, skill, loads[0]['map'], sounds, (kind, dict(p)), respawn, fast)
    names = r['names']
    # the first level ends at the next load's tic (a reborn's dumps follow without a gap) or at the first gap
    end = min([x['tic'] for x in loads[1:]] + [next((j for j, n in enumerate(names[1:]) if int(n[4:9]) != j), len(names) - 1)])
    caps, following = [], L.Dump(str(r['dumps'] / names[0]))
    for j in range(end + 1):
        d, following = following, (L.Dump(str(r['dumps'] / names[j + 1])) if j < end else None)
        caps.append(WG.digest(proj.step(d, following)[0]))
    return caps


def facts(name, r):
    """What the session stream shows: the paused intervals, the loads, the dead tics; TURBOP: the message state."""
    rows = S6.rows(r['session'], 'G')
    paused = [g['tic'] for g in rows if g['paused']]
    dead = [c['tic'] for c in S6.rows(r['session'], 'C') if c['playerstate'] != 0]
    f = dict(paused_tics=f'{paused[0]}-{paused[-1]} ({len(paused)})' if paused else '-',
             loads=' '.join(f"{x['tic']}:E1M{x['map']}" for x in S6.rows(r['session'], 'L')),
             dead_tics=f'{dead[0]}-{dead[-1]} ({len(dead)})' if dead else '-')
    return f


def hud_rows(frames):
    """The frame stream's P rows (tic, message_on) and X rows (tic, text)."""
    h = [f"H,{l.split(',')[1]},{l.split(',')[5]}" for l in frames.splitlines() if l.startswith('P,')]
    x = [f"X,{l.split(',', 3)[1]},{l.split(',', 3)[3]}" for l in frames.splitlines() if l.startswith('X,')]
    return h + x


def state_text(name, r, caps, f, extra):
    data, tics = lump(name)
    seq = sha(b''.join(bytes.fromhex(c) for c in caps))
    head = [f'#schema,{SCHEMA}',
            '#generator,tools/reference/wire29_pause.py: the genuine M6b runner (strict build; overflow and address '
            'identical) and the independent DSIM45 encoder level_snapshot_wire29.py; never from Rust output',
            '#tics,capture j is the state after j session tics, paused tics included; C rows: the first level',
            '#state,as tests/reference/wire29_state.txt',
            '#columns,I,item,skill,respawnparm,fastparm,tics,captures,dumps_sha256,sequence_sha256',
            '#columns,C,item,capture,state_sha256', '#columns,O,fact,value',
            '#columns,H,tic,message_on (the frame stream P row at screenblocks 10)', '#columns,X,tic,message shown']
    rows = [f"I,{name},{data[1]},{data[5]},{data[6]},{tics},{len(caps)},{sha(''.join(r['digests']))},{seq}"]
    rows += [f'C,{name},{j},{d}' for j, d in enumerate(caps)]
    rows += [f'O,{k},{v}' for k, v in f.items()]
    return '\n'.join(head + rows + extra) + '\n'


def new_item(name, bins, work, selftest):
    """The new item's files, or exit on a failure."""
    it = NEW[name]
    data, _ = lump(name)
    (work / 'item').mkdir(parents=True, exist_ok=True)
    (work / 'item' / f'{name}.lmp').write_bytes(data)
    wads = [ROOT / w for w in it['wads']] + [work / 'item' / f'{name}.lmp']
    runs = {b: run(x, name, work / 'runs' / f'{name}_{b}', wads) for b, x in bins.items()}
    s = runs['strict']
    for b, r in runs.items():
        if b == 'address' and it.get('address_stop'):
            # m6b_goldens.ADDRESS_STOPS's rule: exactly the M6a-declared report (st_stuff.c:1292) at the second load,
            # after a prefix of the strict dumps
            k = len(r['digests'])
            if not r['code'] or len(G.W27.UB.findall(r['stderr'])) != 1 or G.G6.DECLARED_UB not in r['stderr'] \
                    or not k or r['digests'] != s['digests'][:k]:
                sys.exit(f'{name} address: not the one declared stop at {G.G6.DECLARED_UB_AT}')
            continue
        if not G.clean(r) or r['digests'] != s['digests'] or any(r[k] != s[k] for k in ('sound', 'session')):
            sys.exit(f'{name} {b}: exit {r["code"]}, or its dumps or streams differ from strict: {r["stderr"][-400:]}')
    caps, f = project(s, wads, data), facts(name, s)
    extra = []
    if it.get('sb'):
        rr = run(bins['strict'], name, work / 'runs' / f'{name}_sb', wads, sb=it['sb'])
        if not G.clean(rr) or rr['digests'] != s['digests']: sys.exit(f'{name}: the rendered run differs')
        extra = hud_rows(rr['frames'])
    print(f'{name}: {len(caps)} captures (DSIM45); ' + ', '.join(f'{k} {v}' for k, v in f.items()))
    if selftest:
        # An extra P_Random draw planted in the runner before the middle of the level must change the capture after it.
        at = len(caps) // 2
        r = run(bins['strict'], name, work / 'runs' / 'self', wads, extra=['--selftest-extra-draw', str(at)])
        k = next((j for j, (x, y) in enumerate(zip(caps, project(r, wads, data))) if x != y), None)
        print(f'selftest {"pass" if k == at + 1 else "FAIL"}: {name}: an extra draw before session tic {at} first differs '
              f'at capture {k}')
        if k != at + 1: sys.exit(1)
    return {ROOT / DIR / f'{name}.lmp': data, ROOT / DIR / f'{name}.txt': text(name).encode(),
            ROOT / DIR / f'{name}.session': s['session'].encode(),
            ROOT / DIR / f'{name.lower()}_state.txt': state_text(name, s, caps, f, extra).encode()}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--only', help='authoring: these items only (nothing is written or compared)')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'wire29' / 'pause')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    only = a.only.split(',') if a.only else None
    bins = {b: B.build(b, work / 'build', a.cc) for b in B.BUILDS}
    out = {}
    for name, want in pinned_items().items():
        if only and name not in only: continue
        r = G.run(bins['strict'], name, work / 'runs' / name)
        if not G.clean(r) or sha(r['session']) != want: sys.exit(f'{name}: the session stream is not the pinned one')
        out[ROOT / DIR / f'{name}.session'] = r['session'].encode()
        print(f'{name}: session stream pinned ({r["session"].count(chr(10) + "G,")} G rows)')
    if a.selftest and out:
        bad = sha(out[next(iter(out))].replace(b'\nG,1,', b'\nG,1,9', 1))
        ok = bad not in pinned_items().values()
        print(f'selftest {"pass" if ok else "FAIL"}: a changed G row no longer hashes to a pinned digest')
        if not ok: return 1
    for name in NEW:
        if only and name not in only: continue
        out.update(new_item(name, bins, work, a.selftest))
    if only: print('--only: nothing written or compared'); return 0
    if a.check:
        bad = [str(p.relative_to(ROOT)) for p, d in out.items() if not p.exists() or p.read_bytes() != d]
        if bad: print('differs from the regenerated text: ' + ', '.join(bad)); return 1
        print(f'{len(out)} files match ({len(pinned_items())} pinned streams, {len(NEW)} new items; {G.W27.ASAN_NOTE})')
    else:
        for p, d in out.items(): p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(d)
        print(f'wrote {len(out)} files')
    shutil.rmtree(work / 'runs', ignore_errors=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
