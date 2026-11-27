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

The item STALE2 (the same skill and byte; room2): a monster holds the corpse as its `target`. A zombieman A on a ledge
sees the player and shoots; its shots, aimed at the first shootable thing on the line, hit the imp B on a band
between them, so B's target becomes A (p_inter.c:904-911) and B kills A with fireballs while the player hides in an
alcove. B's next A_Chase finds the target not shootable, P_LookForPlayers fails (the player is out of sight;
it sets `target` only on success, p_enemy.c:553) and B returns to its spawn state with `target` still naming the corpse
(p_enemy.c:704-711); A_Look reads only the sector's soundtarget, which never gets the player's noise (two
ML_SOUNDBLOCK lines, p_enemy.c:141-147). The corpse respawns and is freed. The player then shoots a barrel X in his
room; its explosion kills a barrel Y in one blow, so Y's `target` stays NULL and Y's explosion damages B with no
source (A_Explode passes the barrel's target, p_enemy.c:1600; p_map.c:1227), which keeps B's target; B's pain state leads to A_Chase, which reads the
freed block's flags (p_enemy.c:704-705).

Runs, as m6b_goldens.py runs its items (the M6b runner of tools/reference/m6b_build.py, imported read-only; the M5c
recommended switches; ASAN_OPTIONS=detect_leaks=0: leak detection disabled, not LeakSanitizer coverage):
  strict, overflow, address   identical dumps, sound and session streams and a clean exit are required;
  observer                    the strict build with OBSERVER's exact-once additions to four genuine files (evidence
                              only; its dumps must equal the strict ones): it records the corpse each
                              P_NightmareRespawn removes, prints every read of such a pointer by P_DeathThink and
                              A_Chase with whether the zone block is then free (its memblock_t id no longer ZONEID,
                              z_zone.c:142-144) and the health and flags the block holds, and every Z_Malloc whose
                              block overlaps a removed corpse.

Output (written without --check; --check regenerates and compares, writing nothing):
  tests/reference/wire29_stale/stale.wad, STALE.txt, STALE.lmp      STALE's map and demo
  tests/reference/wire29_stale/stale2.wad, STALE2.txt, STALE2.lmp   STALE2's
  tests/reference/wire29_stale/stale_state.txt, stale2_state.txt    DOOM-WIRE29-STALE-v1: the I row (as
      wire29_state.txt's), a C row per capture (its state digest over DSIM45) and O rows (observed facts)

    wire29_stale.py [--check] [--selftest] [--author] [--only ITEM[,ITEM]] [--work DIR] [--cc CC]
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
SKILL, RESPAWN = 3, 1
SCHEMA = 'DOOM-WIRE29-STALE-v1'
PLAYER, ZOMBIE, IMP, BARREL = 1, 3004, 3001, 2035
ML_SOUNDBLOCK = 64                                        # doomdata.h; P_RecursiveSound, p_enemy.c:141-147
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


def room2():
    """STALE2: the player's room R (with the hiding alcove H to its north) | a strip M between two ML_SOUNDBLOCK lines
    (the player's noise reaches M with one block and stops at the second, p_enemy.c:118-125) | the hall, crossed by a
    band 48 high on which the imp B stands (it cannot step down, p_map.c:470-472) | a pit | a ledge 48 high with the
    zombieman A, close enough that its shots (spread up to 22 degrees, p_enemy.c A_PosAttack) hit B. A sees the player down the hall and its shots, aimed at the first shootable thing on the line
    (P_AimLineAttack), hit B; B fights A. The barrels X and Y stand in R south of M's opening, where no shot down the
    hall reaches them."""
    p = W.Room('stale2', wall='M5WALLA')
    r = p.sector('ROOM', [(-320, -128, -144, 64), (-320, 64, -192, 256)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    m = p.sector('STRIP', [(-144, -64, -128, 64)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    w = p.sector('WEST', [(-128, -64, -80, 64)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('BAND', [(-80, -64, -16, 64)], floor=48, light=192, fpic='M5FLAT3', cpic='M5FLAT2')
    p.sector('PIT', [(-16, -64, 16, 64)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('LEDGE', [(16, -64, 80, 64)], floor=48, light=192, fpic='M5FLAT3', cpic='M5FLAT2')
    p.style(r, m, flags=ML_SOUNDBLOCK)
    p.style(m, w, flags=ML_SOUNDBLOCK)
    p.thing(-232, 0, 0, PLAYER)
    p.thing(48, 0, 180, ZOMBIE, ALL)                      # A, facing the player
    p.thing(-48, 0, 0, IMP, ALL)                          # B, its back to the player
    p.thing(*X2, 0, BARREL, ALL)
    p.thing(*Y2, 0, BARREL, ALL)
    return p


def room3():
    """BOSS2, on the E1M8 slot (A_BossDeath acts on map 8 of episode 1, p_enemy.c:1629-1635): a hub H, and two
    arenas A1 and A2, separate rooms (no line joins them, so a Baron neither sees nor hears the player before he
    arrives). WR teleport lines (special 97) lead H to A1, A1 to A2 and A2 back to H; each destination is on a Baron,
    so every arrival on a live Baron telefrags it (PIT_StompThing, p_map.c), and on a corpse lands on it. The tag-666
    pillar P beside H stands 2024 above H's floor (inside the port's height bound of 1024 either way): lowerFloorToLowest
    (p_enemy.c:1730-1733) takes it down to H's floor at 1 unit a tic, so a second call comes while it still moves."""
    p = W.Room('boss2', wall='M5WALLC')
    h = p.sector('HUB', [(-256, -64, 128, 64)], floor=-1024, ceil=-896, light=192, tag=3, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('PILLAR', [(-128, 64, -64, 128)], floor=1000, ceil=1024, light=192, tag=666, fpic='M5FLAT3', cpic='M5FLAT2')
    t1 = p.sector('T1', [(128, -64, 256, 64)], floor=-1024, ceil=-896, light=192, fpic='M5FLAT7', cpic='M5FLAT2')
    a1 = p.sector('A1', [(512, -128, 768, 128)], light=160, tag=1, fpic='M5FLAT8', cpic='M5FLAT2')
    t2 = p.sector('T2', [(768, -64, 896, 64)], light=160, fpic='M5FLAT7', cpic='M5FLAT2')
    a2 = p.sector('A2', [(1152, -128, 1408, 128)], light=160, tag=2, fpic='M5FLAT8', cpic='M5FLAT2')
    t3 = p.sector('T3', [(1408, -64, 1536, 64)], light=160, fpic='M5FLAT7', cpic='M5FLAT2')
    p.special(h, t1, (128, 0), W.WR_TELEPORT, 1)
    p.special(a1, t2, (768, 0), W.WR_TELEPORT, 2)
    p.special(a2, t3, (1408, 0), W.WR_TELEPORT, 3)
    p.thing(-192, 0, 0, PLAYER)
    for x in (-192, 576, 1216):
        p.thing(x, 0, 0, W.TELEPORTMAN, ALL)
    for x in (576, 1216):
        p.thing(x, 0, 180, W.BARON, ALL)
    return p


LINES3 = ['cmd 150 25 0 0 0', 'idle 1250', 'cmd 150 25 0 0 0', 'idle 700', 'cmd 150 25 0 0 0', 'idle 300']
X2, Y2 = (-232, -100), (-180, -80)
LINES2 = ['idle 222', 'cmd 16 0 -40 0 0', 'idle 2920', 'cmd 1 0 0 -64 0', 'idle 10', 'cmd 1 0 0 0 ATTACK', 'idle 24',
          'cmd 1 0 0 0 ATTACK', 'idle 24', 'cmd 1 0 0 0 ATTACK', 'idle 200']
ITEMS = {
    'STALE': dict(wad='stale.wad', state='stale_state.txt', room=room, lines=LINES,
                  what='a dead player whose attacker is a corpse that respawns and is freed'),
    'STALE2': dict(wad='stale2.wad', state='stale2_state.txt', room=room2, lines=LINES2,
                   what='a monster whose target is a corpse that respawns and is freed'),
    'BOSS2': dict(wad='boss2.wad', state='boss2_state.txt', room=room3, lines=LINES3, map=8,
                  what='two Barons on E1M8 killed, respawned and killed again: A_BossDeath more than once'),
}


def script_text(name):
    return (f'# Wire 29 item {name}: {ITEMS[name]["what"]}.\n'
            f'# Generated by tools/reference/wire29_stale.py; header bytes: skill {SKILL}, respawnparm {RESPAWN}, fastparm 0.\n'
            + '\n'.join([f'map E1M{slot(name)}'] + ITEMS[name]['lines']) + '\n')


def slot(name):
    return ITEMS[name].get('map', 1)


def lump(name):
    plain, patches, count = WAD.raw_split(script_text(name))
    data, tics = S.lump(plain)
    assert tics == count and not patches
    data = bytearray(data)
    data[1], data[5] = SKILL, RESPAWN
    return bytes(data), tics


def files(name):
    it = ITEMS[name]
    return {f'{DIR}/{it["wad"]}': W.iwad([(f'E1M{slot(name)}', it['room']())]), f'{DIR}/{name}.txt': script_text(name).encode(),
            f'{DIR}/{name}.lmp': lump(name)[0]}


# ------------------------------------------------------------------ the observer ---------------------------
WATCH = ('/* wire29_stale.py observer (evidence only): corpses removed by P_NightmareRespawn and reads through them */\n'
         '#include <stdio.h>\n'
         'void* w29_lo[64]; void* w29_hi[64]; int w29_n;\n'
         'void w29_watch (mobj_t* p) { if (w29_n < 64) { w29_lo[w29_n] = p; w29_hi[w29_n] = p + 1; '
         'fprintf (stderr, "W29,watch,%d,%d\\n", leveltime, w29_n); w29_n++; } }\n'
         'void w29_read (const char* what, void* p) { int i; for (i = 0; i < w29_n; i++) if (p == w29_lo[i]) '
         'fprintf (stderr, "W29,read,%s,%d,%d,%d,%d,%d\\n", what, leveltime, i, ((memblock_t*)((byte*)p - sizeof (memblock_t)))->id != 0x1d4a11, '
         '((mobj_t*)p)->health, ((mobj_t*)p)->flags); }   /* ZONEID, z_zone.c:43 */\n')
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
                   '    { extern void w29_read (const char*, void*); if (actor->target) w29_read ("chase", actor->target); }\n'),
                  # BOSS2: every tag-666 EV_DoFloor of A_BossDeath on episode 1 and the sectors it started (p_floor.c rtn)
                  ('\t    junk.tag = 666;\n\t    EV_DoFloor (&junk, lowerFloorToLowest);\n',
                   '\t    junk.tag = 666;\n\t    { int w29_r = EV_DoFloor (&junk, lowerFloorToLowest); '
                   'fprintf (stderr, "W29,boss,%d,%d\\n", leveltime, w29_r); }\n')],
}


def run(name, binary, out, wads, extra=()):
    """One runner process, as m6b_goldens.run (read-only) runs an item."""
    if out.exists(): shutil.rmtree(out)
    (out / 'dumps').mkdir(parents=True)
    cmd = [str(binary), *[x for w in wads for x in ('--wad', str(w))], '--demo', name, '--game-mode', 'registered',
           '--out', str(out / 'dumps'), *[x for k, v in M5C.RECOMMENDED.items() for x in (k, v)], '--render', 'none',
           *extra, '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt'), '--m6b-out', str(out / 'm6b.txt')]
    p = subprocess.run(cmd, capture_output=True, text=True, env=G.ENV, timeout=600)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    names = ['load.txt'] + sorted(n for n in os.listdir(out / 'dumps') if n.startswith('tic_'))
    return dict(code=p.returncode, stdout=p.stdout, stderr=p.stderr, marker=' end=marker ' in p.stdout, sound=rd('sound.txt'),
                session=rd('session.txt'), m6b=rd('m6b.txt'), names=names, dumps=out / 'dumps',
                digests=[sha((out / 'dumps' / n).read_bytes()) for n in names])


def project(name, r, wads):
    """[the DSIM45 state digest of every capture], as wire29_goldens.project (whose item paths are m6b_wad's)."""
    it = dict(name=name, wads=wads[:-1], lmp_path=wads[-1])          # the WADs, then the demo lump
    loads = S6.rows(r['session'], 'L')
    kind, p, _h, _s = S6.start_from_rows(r['session'], r['sound'], 0, str(wads[0]))
    mapdata = L.MapData(str(wads[0]), f'E1M{slot(name)}', ST.extra_lumps(it))
    sounds = ST.Shifted(M5S.SoundStream(text=r['sound']), max(loads[0]['tic'], 0))
    proj = E.Projector(mapdata, SKILL, slot(name), sounds, (kind, dict(p)), RESPAWN, 0)
    caps, following = [], L.Dump(str(r['dumps'] / r['names'][0]))
    for j in range(len(r['names'])):
        d, following = following, (L.Dump(str(r['dumps'] / r['names'][j + 1])) if j + 1 < len(r['names']) else None)
        caps.append(WG.digest(proj.step(d, following)[0]))
    return caps


def observed(r):
    """The observer's rows: (kind, leveltime, corpse, ...) tuples."""
    return [tuple(l.split(',')[1:]) for l in r['stderr'].splitlines() if l.startswith('W29,')]


def mobjs(d): return [m for k, m in d.thinkers if k == 'M']


def facts(name, r, obs):
    """The scenario, from the strict dumps (captures) and the observer rows (leveltimes; a pending read is of the
    removed corpse still in the thinker list, a freed read of its block after Z_Free, with the health and flags the
    block then holds). STALE: `dangling_captures` counts captures whose `player->attacker` names no thinker (the
    dump's -2). STALE2: B is the imp; `b_asleep_capture` is the first capture after A's death with B back in its
    spawn state, `b_target_asleep` the dump index B's target names there."""
    dumps = [L.Dump(str(r['dumps'] / n)) for n in r['names']]
    p = [d.players[0] for d in dumps]
    watch = [int(o[1]) for o in obs if o[0] == 'watch']
    reads = [o for o in obs if o[0] == 'read']
    freed = [o for o in reads if o[4] == '1']
    common = dict(respawn_leveltime=watch, attacker_reads=len([o for o in reads if o[1] == 'attacker']),
                  chase_reads=len([o for o in reads if o[1] == 'chase']), pending_reads=len(reads) - len(freed),
                  freed_reads=len(freed), first_freed_read=int(freed[0][2]) if freed else -1,
                  reuses=len([o for o in obs if o[0] == 'reuse']))
    if name == 'BOSS2':
        # the tag-666 floor's height (map units) at each boss call's capture and at the end; kills
        boss = [o for o in obs if o[0] == 'boss']
        pillar = lambda d: next(x['floorheight'] for x in d.sectors if x['tag'] == 666) >> 16
        return dict(respawn_leveltime=watch, boss_calls=[int(o[1]) for o in boss], boss_started=[int(o[2]) for o in boss],
                    pillar_at_calls=[pillar(dumps[int(o[1]) + 1]) for o in boss], pillar_end=pillar(dumps[-1]),
                    killcount=p[-1]['killcount'], totalkills=dumps[-1].h['totalkills'], player_health_end=p[-1]['health'],
                    pending_reads=common['pending_reads'], freed_reads=common['freed_reads'], reuses=common['reuses'])
    if name == 'STALE':
        dead = next(j for j, x in enumerate(p) if x['playerstate'] == 1)
        f = dict(player_dead_capture=dead, attacker_at_death=p[dead]['attacker'], killcount=p[-1]['killcount'],
                 totalkills=dumps[-1].h['totalkills'])
        f.update(common)
        f['dangling_captures'] = sum(1 for x in p if x['attacker'] == -2)
        return f
    imp = lambda d: next(m for m in mobjs(d) if m['type'] == G.enum_index('mobjtype_t', 'MT_TROOP'))
    zombie_dead = next(j for j, d in enumerate(dumps) if any(
        m['type'] == G.enum_index('mobjtype_t', 'MT_POSSESSED') and m['health'] <= 0 for m in mobjs(d)))
    stand = (G.enum_index('statenum_t', 'S_TROO_STND'), G.enum_index('statenum_t', 'S_TROO_STND2'))
    asleep = next(j for j in range(zombie_dead, len(dumps)) if imp(dumps[j])['state'] in stand)
    f = dict(zombie_dead_capture=zombie_dead, b_asleep_capture=asleep, b_target_asleep=imp(dumps[asleep])['target'],
             killcount=p[-1]['killcount'], totalkills=dumps[-1].h['totalkills'], player_health_end=p[-1]['health'])
    f.update(common)
    if freed:
        k = int(freed[0][2]) + 1
        b = imp(dumps[k]) if k < len(dumps) else None
        f.update(first_freed_values=f'health {freed[0][5]} flags {freed[0][6]}',
                 b_after_first_freed_read=f'state {b["state"]} target {b["target"]} health {b["health"]}' if b else '-')
    return f


def evidence(name, work, cc, bins=None):
    for path, data in files(name).items():
        (work / 'item' / path).parent.mkdir(parents=True, exist_ok=True)
        (work / 'item' / path).write_bytes(data)
    wads = [work / 'item' / DIR / ITEMS[name]['wad']] + [ROOT / w for w in WAD.SHARED] + [work / 'item' / DIR / f'{name}.lmp']
    if bins is None:
        reuse = os.environ.get('W29_REUSE_BUILDS') == '1'          # authoring only
        bins = {b: (work / 'build' / b / 'crunner') if reuse else B.build(b, work / 'build', cc) for b in B.BUILDS}
        bins['observer'] = (work / 'build' / 'observer' / 'crunner') if reuse else B.variant('observer', OBSERVER, work / 'build', cc)
    runs = {b: run(name, x, work / 'runs' / f'{name}_{b}', wads) for b, x in bins.items()}
    s, fails = runs['strict'], []
    for b, r in runs.items():
        if not G.clean(r) or r['digests'] != s['digests'] or any(r[k] != s[k] for k in ('sound', 'session')):
            fails.append(f'{name} {b}: exit {r["code"]}, or its dumps or streams differ from strict: {r["stderr"][-400:]}')
    obs = observed(runs['observer'])
    return runs, wads, obs, fails, bins


def state_text(name, r, caps, f):
    tics = lump(name)[1]
    seq = sha(b''.join(bytes.fromhex(c) for c in caps))
    head = [f'#schema,{SCHEMA}',
            '#generator,tools/reference/wire29_stale.py: the genuine M6b runner (strict build; overflow, address and observer '
            'identical) and the independent DSIM45 encoder level_snapshot_wire29.py; never from Rust output',
            '#tics,capture j is the state after j session tics; O values are leveltimes (level tics) unless named capture',
            '#state,as tests/reference/wire29_state.txt',
            '#columns,I,item,skill,respawnparm,fastparm,tics,captures,dumps_sha256,sequence_sha256',
            '#columns,C,item,capture,state_sha256', '#columns,O,fact,value']
    rows = [f"I,{name},{SKILL},{RESPAWN},0,{tics},{len(caps)},{sha(''.join(r['digests']))},{seq}"]
    rows += [f'C,{name},{j},{d}' for j, d in enumerate(caps)]
    rows += [f'O,{k},{" ".join(map(str, v)) if isinstance(v, list) else v}' for k, v in f.items()]
    return '\n'.join(head + rows) + '\n'


def author(name, s, obs):
    for j, n in enumerate(s['names']):
        d = L.Dump(str(s['dumps'] / n)); p = d.players[0]
        ms = [(G.MT.get(m['type'], m['type']), m['health'], m['state'], m['tics'], m['target'], m['x'] >> 16, m['y'] >> 16)
              for m in mobjs(d) if m['type'] != 0]
        if j % 20 == 0 or j < 3 or p['playerstate'] or any(m[0] in ('TROOPSHOT',) for m in ms) or j > len(s['names']) - 260:
            print(j, d.h['leveltime'], p['health'], p['playerstate'], p['attacker'], ms)
    print('\n'.join(','.join(o) for o in obs[:40]), f'... {len(obs)} observer rows')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--author', action='store_true', help='authoring: print the scenario, write nothing')
    ap.add_argument('--only', help='authoring: these items only')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'wire29' / 'stale')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    out, bins, total = {}, None, 0
    for name in ITEMS:
        if a.only and name not in a.only.split(','): continue
        runs, wads, obs, fails, bins = evidence(name, work, a.cc, bins)
        s = runs['strict']
        if a.author: author(name, s, obs)
        if fails: print('\n'.join(['FAILURES:'] + fails)); return 1
        caps, f = project(name, s, wads), facts(name, s, obs)
        print(f'{name}: {len(caps)} captures (DSIM45); ' + ', '.join(f'{k} {v}' for k, v in f.items()))
        text = state_text(name, s, caps, f)
        if a.selftest:
            # The tool can fail: one extra P_Random draw planted in the runner (m6b_build --selftest-extra-draw) must
            # make the projected digests differ from these at the capture after it, and a golden with one digit
            # changed must not compare equal.
            at = f['respawn_leveltime'][0] if f['respawn_leveltime'] else len(caps) // 2
            r = run(name, bins['strict'], work / 'runs' / 'self', wads, ['--selftest-extra-draw', str(at)])
            k = next((j for j, (x, y) in enumerate(zip(caps, project(name, r, wads))) if x != y), None)
            bad = text.replace(caps[at], ('0' if caps[at][0] != '0' else '1') + caps[at][1:])
            ok = k == at + 1 and bad != text
            print(f'selftest {"pass" if ok else "FAIL"}: {name}: an extra draw before session tic {at} first differs at '
                  f'capture {k}; a changed golden digest {"differs" if bad != text else "equals"}')
            if not ok: return 1
        out.update({ROOT / p: d for p, d in files(name).items()})
        out[ROOT / DIR / ITEMS[name]['state']] = text.encode()
        total += len(caps)
    if a.author or a.only:
        print('--author/--only: nothing written or compared'); return 0
    if a.check:
        bad = [str(p.relative_to(ROOT)) for p, d in out.items() if not p.exists() or p.read_bytes() != d]
        if bad: print('differs from the regenerated text: ' + ', '.join(bad)); return 1
        print(f'{len(out)} files match ({len(ITEMS)} items, {total} captures; {G.W27.ASAN_NOTE})')
    else:
        for p, d in out.items(): p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(d)
        print(f'wrote {len(out)} files')
    shutil.rmtree(work / 'runs', ignore_errors=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
