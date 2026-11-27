#!/usr/bin/env python3
"""Wire 29, the stale pointer after a respawn: a corpse that is still named when P_NightmareRespawn removes it
(p_mobj.c:408), watched in the genuine linuxdoom-1.10 runner. Expected values come from the genuine code and the
independent DSIM45 encoder level_snapshot_wire29.py, never from Rust output.

The item STALE (skill 3 with the -respawn byte, g_game.c:1416-1419; our own map, nothing from any IWAD): the player
stands on a 5-unit damage floor (sector special 7, p_spec.c:1043-1049) until 5 health is left, steps off and wakes
an imp on a ledge it cannot leave (p_map.c:470-472) with a pistol shot at a barrel beside it. The imp throws a
fireball (speed 10 on skill 3); while it flies the player's next shots kill the barrel, whose explosion
(p_enemy.c A_Explode) kills the imp. The fireball then kills the player: P_DamageMobj sets `player->attacker` to
the missile's `target`, the imp's corpse (p_inter.c:847, 875). The dead player never presses use, so P_DeathThink
reads `player->attacker->x` and `->y` on every tic (p_user.c:200-205). The corpse respawns (p_mobj.c:356-409) and
is freed when P_RunThinkers next reaches it (p_tick.c:107-112, Z_Free); from then on P_DeathThink reads a freed
zone block.

Runs, as m6b_goldens.py runs its items (the M6b runner of tools/reference/m6b_build.py, imported read-only; the M5c
recommended switches; ASAN_OPTIONS=detect_leaks=0: leak detection disabled, not LeakSanitizer coverage):
  strict, overflow, address   identical dumps, sound and session streams and a clean exit are required;
  observer                    the strict build with OBSERVER's exact-once additions to four genuine files (evidence
                              only; its dumps must equal the strict ones): it records the corpse each
                              P_NightmareRespawn removes, prints every read of such a pointer by P_DeathThink and
                              A_Chase with whether the zone block is then free (its memblock_t id no longer ZONEID,
                              z_zone.c:142-144), and every Z_Malloc whose block overlaps a removed corpse.

Output (written without --check; --check regenerates and compares, writing nothing):
  tests/reference/wire29_stale/stale.wad, STALE.txt, STALE.lmp   the map and the demo
  tests/reference/wire29_stale/stale_state.txt   DOOM-WIRE29-STALE-v1: the I row (as wire29_state.txt's), a C row per
                                                 capture (its state digest over DSIM45) and O rows (observed facts)

    wire29_stale.py [--check] [--selftest] [--author] [--work DIR] [--cc CC]
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
import m4b_scripts as S              # noqa: E402  (read-only: the script format)
import m5c_reference as M5C          # noqa: E402  (read-only: RECOMMENDED)
import m6a_state as ST               # noqa: E402  (read-only: Shifted, extra_lumps)
import m6a_wad as W                  # noqa: E402  (read-only: the room builder)
import m6b_build as B                # noqa: E402  (read-only)
import m6b_goldens as G              # noqa: E402  (read-only: clean, ENV)
import m6b_wad as WAD                # noqa: E402  (read-only: raw_split, SHARED)
import wire29_goldens as WG          # noqa: E402  (read-only: digest)

DIR = 'tests/reference/wire29_stale'
NAME, SKILL, RESPAWN = 'STALE', 3, 1
STATE, SCHEMA = ROOT / DIR / 'stale_state.txt', 'DOOM-WIRE29-STALE-v1'
PLAYER, IMP, BARREL = 1, 3001, 2035
ALL = 1 | 2 | 4
# The player at 5 health after 19 hits of the special 7 floor (leveltime 0, 32, .., 576); off it before 608; a pistol
# shot at the barrel (the noise wakes the imp, p_pspr.c P_NoiseAlert); shots after the imp's first fireball until
# the barrel dies; then idle (never BT_USE: no reborn, p_user.c:227-228) past the imp's respawn.
LINES = ['idle 586', 'cmd 14 25 0 0 0', 'idle 30', 'cmd 1 0 0 0 ATTACK', 'idle 236', 'cmd 1 0 0 0 ATTACK', 'idle 650']
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def room():
    p = W.Room('stale', wall='M5WALLA')
    p.sector('NUKE', [(-128, -64, 0, 64)], light=192, special=7, fpic='NUKAGE1', cpic='M5FLAT2')
    p.sector('HALL', [(0, -64, 640, 64)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('LEDGE', [(640, -64, 768, 16)], floor=48, ceil=176, light=192, fpic='M5FLAT3', cpic='M5FLAT2')
    p.sector('PEDESTAL', [(640, 16, 768, 64)], floor=96, ceil=176, light=192, fpic='M5FLAT3', cpic='M5FLAT2')
    p.thing(-64, 40, 0, PLAYER)
    p.thing(704, -24, 0, IMP, ALL)                        # facing east, away from the player; it cannot leave the ledge
    p.thing(704, 40, 0, BARREL, ALL)                      # on the pedestal, on the player's line of fire (y = 40)
    return p


def script_text():
    return (f'# Wire 29 item {NAME}: a dead player whose attacker is a corpse that respawns and is freed.\n'
            f'# Generated by tools/reference/wire29_stale.py; header bytes: skill {SKILL}, respawnparm {RESPAWN}, fastparm 0.\n'
            + '\n'.join(['map E1M1'] + LINES) + '\n')


def lump():
    plain, patches, count = WAD.raw_split(script_text())
    data, tics = S.lump(plain)
    assert tics == count and not patches
    data = bytearray(data)
    data[1], data[5] = SKILL, RESPAWN
    return bytes(data), tics


def files():
    return {f'{DIR}/stale.wad': W.iwad([('E1M1', room())]), f'{DIR}/{NAME}.txt': script_text().encode(),
            f'{DIR}/{NAME}.lmp': lump()[0]}


# ------------------------------------------------------------------ the observer ---------------------------
WATCH = ('/* wire29_stale.py observer (evidence only): corpses removed by P_NightmareRespawn and reads through them */\n'
         '#include <stdio.h>\n'
         'void* w29_lo[64]; void* w29_hi[64]; int w29_n;\n'
         'void w29_watch (mobj_t* p) { if (w29_n < 64) { w29_lo[w29_n] = p; w29_hi[w29_n] = p + 1; '
         'fprintf (stderr, "W29,watch,%d,%d\\n", leveltime, w29_n); w29_n++; } }\n'
         'void w29_read (const char* what, void* p) { int i; for (i = 0; i < w29_n; i++) if (p == w29_lo[i]) '
         'fprintf (stderr, "W29,read,%s,%d,%d,%d\\n", what, leveltime, i, ((memblock_t*)((byte*)p - sizeof (memblock_t)))->id != 0x1d4a11); }   /* ZONEID, z_zone.c:43 */\n')
OBSERVER = {
    'p_mobj.c': [('void\nP_NightmareRespawn (mobj_t* mobj)\n', WATCH + 'void\nP_NightmareRespawn (mobj_t* mobj)\n'),
                 ('    // remove the old monster,\n    P_RemoveMobj (mobj);\n',
                  '    // remove the old monster,\n    w29_watch (mobj);\n    P_RemoveMobj (mobj);\n')],
    'z_zone.c': [('    base->id = ZONEID;\n    \n    return (',
                  '    base->id = ZONEID;\n    { extern void* w29_lo[]; extern void* w29_hi[]; extern int w29_n; extern int leveltime; int i;\n'
                  '      for (i = 0; i < w29_n; i++) if ((byte*)w29_lo[i] < (byte*)base + base->size && (byte*)w29_hi[i] > (byte*)base)\n'
                  '        fprintf (stderr, "W29,reuse,%d,%d,%d,%d\\n", leveltime, i, size, tag); }\n    \n    return ('),
                 ('#include "z_zone.h"\n', '#include "z_zone.h"\n#include <stdio.h>\n')],
    'p_user.c': [('    if (player->attacker && player->attacker != player->mo)\n    {\n',
                  '    if (player->attacker && player->attacker != player->mo)\n    {\n'
                  '\t{ extern void w29_read (const char*, void*); w29_read ("attacker", player->attacker); }\n')],
    'p_enemy.c': [('void A_Chase (mobj_t*\tactor)\n{\n    int\t\tdelta;\n',
                   'void A_Chase (mobj_t*\tactor)\n{\n    int\t\tdelta;\n'
                   '    { extern void w29_read (const char*, void*); if (actor->target) w29_read ("chase", actor->target); }\n')],
}


def run(binary, out, wads, extra=()):
    """One runner process, as m6b_goldens.run (read-only) runs an item."""
    if out.exists(): shutil.rmtree(out)
    (out / 'dumps').mkdir(parents=True)
    cmd = [str(binary), *[x for w in wads for x in ('--wad', str(w))], '--demo', NAME, '--game-mode', 'registered',
           '--out', str(out / 'dumps'), *[x for k, v in M5C.RECOMMENDED.items() for x in (k, v)], '--render', 'none',
           *extra, '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt'), '--m6b-out', str(out / 'm6b.txt')]
    p = subprocess.run(cmd, capture_output=True, text=True, env=G.ENV, timeout=600)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    names = ['load.txt'] + sorted(n for n in os.listdir(out / 'dumps') if n.startswith('tic_'))
    return dict(code=p.returncode, stdout=p.stdout, stderr=p.stderr, marker=' end=marker ' in p.stdout, sound=rd('sound.txt'),
                session=rd('session.txt'), m6b=rd('m6b.txt'), names=names, dumps=out / 'dumps',
                digests=[sha((out / 'dumps' / n).read_bytes()) for n in names])


def project(r, wads):
    """[the DSIM45 state digest of every capture], as wire29_goldens.project (whose item paths are m6b_wad's)."""
    it = dict(name=NAME, wads=wads, lmp_path=wads[-1])
    loads = S6.rows(r['session'], 'L')
    kind, p, _h, _s = S6.start_from_rows(r['session'], r['sound'], 0, str(wads[0]))
    mapdata = L.MapData(str(wads[0]), 'E1M1', ST.extra_lumps(it))
    sounds = ST.Shifted(M5S.SoundStream(text=r['sound']), max(loads[0]['tic'], 0))
    proj = E.Projector(mapdata, SKILL, 1, sounds, (kind, dict(p)), RESPAWN, 0)
    caps, following = [], L.Dump(str(r['dumps'] / r['names'][0]))
    for j in range(len(r['names'])):
        d, following = following, (L.Dump(str(r['dumps'] / r['names'][j + 1])) if j + 1 < len(r['names']) else None)
        caps.append(WG.digest(proj.step(d, following)[0]))
    return caps


def observed(r):
    """The observer's rows: (kind, leveltime, corpse, ...) tuples."""
    return [tuple(l.split(',')[1:]) for l in r['stderr'].splitlines() if l.startswith('W29,')]


def facts(r, obs):
    """The scenario, from the strict dumps (captures; `dangling_captures`: captures whose `player->attacker` names no
    thinker, the dump's -2) and the observer rows (leveltimes; a pending read is of the removed corpse still in the
    thinker list, a freed read of its block after Z_Free)."""
    dumps = [L.Dump(str(r['dumps'] / n)) for n in r['names']]
    p = [d.players[0] for d in dumps]
    dead = next(j for j, x in enumerate(p) if x['playerstate'] == 1)
    watch = [int(o[1]) for o in obs if o[0] == 'watch']
    reads = [o for o in obs if o[0] == 'read']
    freed = [o for o in reads if o[4] == '1']
    return dict(player_dead_capture=dead, attacker_at_death=p[dead]['attacker'], killcount=p[-1]['killcount'],
                totalkills=dumps[-1].h['totalkills'], respawn_leveltime=watch, attacker_reads=len([o for o in reads if o[1] == 'attacker']),
                chase_reads=len([o for o in reads if o[1] == 'chase']), pending_reads=len(reads) - len(freed),
                freed_reads=len(freed), first_freed_read=int(freed[0][2]) if freed else -1,
                reuses=len([o for o in obs if o[0] == 'reuse']),
                dangling_captures=sum(1 for x in p if x['attacker'] == -2))


def evidence(work, cc):
    for path, data in files().items():
        (work / 'item' / path).parent.mkdir(parents=True, exist_ok=True)
        (work / 'item' / path).write_bytes(data)
    wads = [work / 'item' / DIR / 'stale.wad'] + [ROOT / w for w in WAD.SHARED] + [work / 'item' / DIR / f'{NAME}.lmp']
    reuse = os.environ.get('W29_REUSE_BUILDS') == '1'          # authoring only
    bins = {b: (work / 'build' / b / 'crunner') if reuse else B.build(b, work / 'build', cc) for b in B.BUILDS}
    bins['observer'] = (work / 'build' / 'observer' / 'crunner') if reuse else B.variant('observer', OBSERVER, work / 'build', cc)
    runs = {b: run(x, work / 'runs' / b, wads) for b, x in bins.items()}
    s, fails = runs['strict'], []
    for b, r in runs.items():
        if not G.clean(r) or r['digests'] != s['digests'] or any(r[k] != s[k] for k in ('sound', 'session')):
            fails.append(f'{b}: exit {r["code"]}, or its dumps or streams differ from strict: {r["stderr"][-400:]}')
    obs = observed(runs['observer'])
    return runs, wads, obs, fails, bins


def state_text(r, caps, f):
    tics = lump()[1]
    seq = sha(b''.join(bytes.fromhex(c) for c in caps))
    head = [f'#schema,{SCHEMA}',
            '#generator,tools/reference/wire29_stale.py: the genuine M6b runner (strict build; overflow, address and observer '
            'identical) and the independent DSIM45 encoder level_snapshot_wire29.py; never from Rust output',
            '#tics,capture j is the state after j session tics; O values are leveltimes (level tics) unless named capture',
            '#state,as tests/reference/wire29_state.txt',
            '#columns,I,item,skill,respawnparm,fastparm,tics,captures,dumps_sha256,sequence_sha256',
            '#columns,C,item,capture,state_sha256', '#columns,O,fact,value']
    rows = [f"I,{NAME},{SKILL},{RESPAWN},0,{tics},{len(caps)},{sha(''.join(r['digests']))},{seq}"]
    rows += [f'C,{NAME},{j},{d}' for j, d in enumerate(caps)]
    rows += [f'O,{k},{" ".join(map(str, v)) if isinstance(v, list) else v}' for k, v in f.items()]
    return '\n'.join(head + rows) + '\n'


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--author', action='store_true', help='authoring: print the scenario, write nothing')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'wire29' / 'stale')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    runs, wads, obs, fails, bins = evidence(work, a.cc)
    s = runs['strict']
    if a.author:
        for j, n in enumerate(s['names']):
            d = L.Dump(str(s['dumps'] / n)); p = d.players[0]
            ms = [(G.MT.get(m['type'], m['type']), m['health'], m['state'], m['tics']) for k, m in d.thinkers if k == 'M' and m['type'] != 0]
            if j % 20 == 0 or j < 3 or any(m[0] == 'TROOPSHOT' for m in ms) or p['playerstate']:
                print(j, d.h['leveltime'], p['health'], p['playerstate'], p['attacker'], ms)
        print('\n'.join(','.join(o) for o in obs[:40]), f'... {len(obs)} observer rows')
    if fails: print('\n'.join(['FAILURES:'] + fails)); return 1
    caps, f = project(s, wads), facts(s, obs)
    print(f'{NAME}: {len(caps)} captures (DSIM45); ' + ', '.join(f'{k} {v}' for k, v in f.items()))
    text = state_text(s, caps, f)
    if a.selftest:
        # The tool can fail: one extra P_Random draw planted in the runner (m6b_build --selftest-extra-draw) must make
        # the projected digests differ from these at the capture after it, and a golden with one digit changed must
        # not compare equal.
        at = f['player_dead_capture']
        r = run(bins['strict'], work / 'runs' / 'self', wads, ['--selftest-extra-draw', str(at)])
        k = next((j for j, (x, y) in enumerate(zip(caps, project(r, wads))) if x != y), None)
        bad = text.replace(caps[at], ('0' if caps[at][0] != '0' else '1') + caps[at][1:])
        ok = k == at + 1 and bad != text
        print(f'selftest {"pass" if ok else "FAIL"}: an extra draw before session tic {at} first differs at capture {k}; '
              f'a changed golden digest {"differs" if bad != text else "equals"}')
        if not ok: return 1
    if a.author: return 0
    out = {ROOT / p: d for p, d in files().items()}
    out[STATE] = text.encode()
    if a.check:
        bad = [str(p.relative_to(ROOT)) for p, d in out.items() if not p.exists() or p.read_bytes() != d]
        if bad: print('differs from the regenerated text: ' + ', '.join(bad)); return 1
        print(f'{len(out)} files match ({len(caps)} captures, {len(obs)} observer rows; {G.W27.ASAN_NOTE})')
    else:
        for p, d in out.items(): p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(d)
        print(f'wrote {len(out)} files')
    shutil.rmtree(work / 'runs', ignore_errors=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
