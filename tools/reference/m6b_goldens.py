#!/usr/bin/env python3
"""M6b against the genuine engine: Nightmare's rules, the -respawn and -fast parameters, the process-global table edit
and the pause command, each watched in the genuine linuxdoom-1.10 runner on synthetic maps, with planted faults
(docs/M6B_REFERENCE_INVESTIGATION.md section 6). Expected values come from the genuine code, never from Rust output.

The runner is the M6b runner (tools/reference/m6b_build.py, contract tests/reference/m6b_runner_contract.json) in three
builds under --work: strict (the genuine text, -fwrapv: THE CLAIM), overflow and address (UBSan, ASan; run with
ASAN_OPTIONS=detect_leaks=0: leak detection disabled, not LeakSanitizer coverage). Per item of
tools/reference/m6b_wad.py ITEMS (the M5c recommended switches, game mode registered):

  plain     --render none with dumps, sound, session and M6b streams in the three builds: identical dumps and streams,
            a clean exit, no sanitizer report;
  rendered  the items of RENDER (the pause items, and the two whose intermission counts kills past 100 %) also in the strict build with --render every at each screen size of
            RENDER (10 and 11; 9 needs the border flat FLOOR7_2 and the border patches, which this WAD set does not
            have: the genuine engine stops with "W_GetNumForName: FLOOR7_2 not found!", observed): the frame stream (every level frame's digest, and the G rows of the intermission and finale
            frames) and the M6b stream, whose B rows carry the digest of every frame displayed while paused. The
            rendered run's dumps, sound and session streams must equal the plain run's.

Variants (VARIANTS; the altered genuine engine of the self-tests): strict builds with one rule of the original text
removed or changed by exact-once replacements made in the build directory (no file of the repository changes). Each
variant must differ from the strict run of each of its items; the first differing place (the capture, or the stream and
its line when every dump is equal) is written as a V row, so a later Rust test can show that a port behaving like the
variant is caught there. --selftest adds faults planted in the runner itself (an extra P_Random draw; the pause picture
one row lower), which must change the dumps and the paused frames at their place.

Tables (TABLES): runs of an item after earlier games of the same process (--prior-game), their T rows, and whether the
game rows of every dump (without the observer rows T, K and RM, whose serial numbers count the earlier loads) equal
those of the named run.

What is not here, and why: state digests over the wire 29 schema. Their shape waits on the owner's decisions N2, N4, N5
and N6 (what the wire admits and how the table and the pause are carried); the dumps they will be projected from are pinned
here by digest (the D column of the I rows).

Tic convention (owner): a session tic is the 0-based index of a G_Ticker; capture j is the state after j tics.

Outputs (written without --check; --check regenerates and compares, writing nothing):
  tests/reference/m6b_genuine.txt            DOOM-M6B-GENUINE-v1: I rows (per item: the header bytes, tics, captures, the
                                             end, digests of the dump set and of each stream), R rows (every pass of the
                                             respawn gate that could be resolved: respawned or blocked), T rows (the
                                             table runs), V rows (the variants)
  tests/reference/m6b_session/<ITEM>.session the session stream (DOOM-M6A-SESSION-v1) of the items of SESSION
  tests/reference/m6b_sound/<ITEM>.sound     the sound stream (m6a_goldens.golden_sound) of the items of SOUND
  tests/reference/m6b_frames/<ITEM>.sb<N>.frames  the frame stream's golden rows (m6a_goldens.golden_frames), RENDER
  tests/reference/m6b_pause/<ITEM>.sb<N>.txt      the M6b stream of the rendered run (T, MP, B and Z rows), RENDER
  tests/reference/m6b_results.json           DOOM-M6B-RESULTS-v1: builds, variants with their replacements, tool and
                                             input digests, per item the observed facts; `generated_with` is left out
                                             of --check

    m6b_goldens.py [--check] [--selftest] [--only ITEM[,ITEM]] [--work DIR] [--cc CC]
"""
import argparse, hashlib, json, math, os, re, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m3h9c_build as base          # noqa: E402  (read-only: revision, flags)
import m5c_reference as M5C         # noqa: E402  (read-only: RECOMMENDED)
import m6a_goldens as G6            # noqa: E402  (read-only: golden_frames, golden_sound)
import m6b_build as B               # noqa: E402
import m6b_wad as WAD               # noqa: E402
import level_snapshot as L          # noqa: E402  (read-only: Dump)
import wire27_goldens as W27        # noqa: E402  (read-only: UB, ASAN_NOTE)
from m3h9_geometry import enum_index  # noqa: E402

GENUINE, GENUINE_SCHEMA = ROOT / 'tests/reference/m6b_genuine.txt', 'DOOM-M6B-GENUINE-v1'
RESULTS, SCHEMA = ROOT / 'tests/reference/m6b_results.json', 'DOOM-M6B-RESULTS-v1'
DIRS = {k: ROOT / f'tests/reference/m6b_{k}' for k in ('session', 'sound', 'frames', 'pause')}
TOOLS = ['m6b_goldens.py', 'm6b_wad.py', 'm6b_build.py', 'm6b_session.c']
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
CLAIM = B.BUILDS
RENDER = {'RESP4': (10,), 'RESP3R': (10,), 'PAUSE': (10, 11), 'PAUSE0': (10,), 'PAUSEWI': (10, 11), 'PAUSEFIN': (10, 11)}   # the first size is the variants'
SESSION = ['RESP4', 'RESP3R', 'PAUSE', 'PAUSE0', 'PAUSEWI', 'PAUSEFIN']
SOUND = ['RESP4', 'RESP3R', 'BLOCK4', 'GIB4', 'PAUSE', 'PAUSE0', 'PAUSEWI', 'PAUSEFIN']
# The address build stops at the M6a-declared report (m6a_goldens.DECLARED_UB: st_stuff.c:1292, the key box index, at a
# level load after the first) on an item that loads a second level; m6a_goldens.py runs such items in a build that
# recovers from that one report. Here the address build of these items must show exactly that report and a prefix of the
# strict dumps; the overflow build runs them whole.
ADDRESS_STOPS = ['PAUSEWI']
address_stops = {}
# name: ({genuine file: [(old, new)]}, [items], what); every `old` occurs exactly once in the file.
VARIANTS = {
    'bit': ({'p_mobj.c': [('\tbit = 4;\n', '\tbit = 1<<(gameskill-1);\n')]}, ['BITS4'],
            'p_mobj.c:746: Nightmare\'s spawn bit computed like the other skills\' (8, the ambush flag)'),
    'clamp': ({'g_game.c': [('\tskill = sk_nightmare;\n', '\t;\n')]}, ['CLAMP5'],
              'g_game.c:1380: a skill above Nightmare not clamped'),
    'nodouble': ({'p_inter.c': [('\t|| gameskill == sk_nightmare)\n', '\t|| gameskill == sk_nightmare + 1)\n')]}, ['AMMO4'],
                 'p_inter.c:96: Nightmare no longer doubles P_GiveAmmo'),
    'react': ({'p_mobj.c': [('    if (gameskill != sk_nightmare)\n', '    if (gameskill != sk_nightmare + 1)\n')]}, ['REACT4', 'RESP4'],
              'p_mobj.c:503: reactiontime set from the table on Nightmare too'),
    'chase_dir': ({'p_enemy.c': [('\tif (gameskill != sk_nightmare && !fastparm)\n', '\tif (1)\n')]}, ['REACT4', 'REACT3F'],
                  'p_enemy.c:719: P_NewChaseDir after an attack on every skill'),
    'chase_count': ({'p_enemy.c': [('\tif (gameskill < sk_nightmare\n', '\tif (gameskill <= sk_nightmare\n'),
                                   ('\t    && !fastparm && actor->movecount)\n', '\t    && actor->movecount)\n')]}, ['REACT4', 'REACT3F'],
                    'p_enemy.c:738-739: the movecount test before a missile on every skill'),
    'fast_states': ({'g_game.c': [('\t    states[i].tics >>= 1; \n', '\t    ; \n')]}, ['FAST4', 'FAST3F'],
                    'g_game.c:1424: the thirteen state durations not halved'),
    'fast_speeds': ({'g_game.c': [('\tmobjinfo[MT_BRUISERSHOT].speed = 20*FRACUNIT; \n', '\tmobjinfo[MT_BRUISERSHOT].speed = 15*FRACUNIT; \n'),
                                  ('\tmobjinfo[MT_TROOPSHOT].speed = 20*FRACUNIT; \n', '\tmobjinfo[MT_TROOPSHOT].speed = 10*FRACUNIT; \n')]},
                    ['FAST4', 'FAST3F'], 'g_game.c:1425, 1427: the Baron and imp missile speeds not raised'),
    'respawn_flag': ({'g_game.c': [('    if (skill == sk_nightmare || respawnparm )\n', '    if (0)\n')]}, ['RESP4', 'RESP3R'],
                     'g_game.c:1416: respawnmonsters never set'),
    'timer': ({'p_mobj.c': [('\tif (mobj->movecount < 12*35)\n', '\tif (mobj->movecount < 13*35)\n')]}, ['RESP4', 'GIB4'],
              'p_mobj.c:461: the corpse timer one second longer'),
    'clock': ({'p_mobj.c': [('\tif ( leveltime&31 )\n', '\tif ( leveltime&63 )\n')]}, ['RESP4', 'BLOCK4'],
              'p_mobj.c:464: the gate every 64 tics'),
    'gate': ({'p_mobj.c': [('\tif (P_Random () > 4)\n', '\tif (P_Random () > 3)\n')]}, ['RESP4'],
             'p_mobj.c:467: the gate passes for 0 to 3 only'),
    'nocheck': ({'p_mobj.c': [('    if (!P_CheckPosition (mobj, x, y) ) \n', '    if (0) \n')]}, ['BLOCK4'],
                'p_mobj.c:370: no P_CheckPosition at the spawn point'),
    'fog_spot': ({'p_mobj.c': [('    mo = P_SpawnMobj (mobj->x,\n\t\t      mobj->y,\n', '    mo = P_SpawnMobj (x,\n\t\t      y,\n')]}, ['RESP4'],
                 'p_mobj.c:375-376: the first fog at the spawn point instead of the corpse'),
    'react18': ({'p_mobj.c': [('    mo->reactiontime = 18;\n', '    ;\n')]}, ['RESP4', 'RESP3R', 'GIB4'],
                'p_mobj.c:405: the respawned monster\'s reactiontime left as spawned'),
    'ambush': ({'p_mobj.c': [('\tmo->flags |= MF_AMBUSH;\n', '\t;\n')]}, ['RESP4', 'GIB4'],
               'p_mobj.c:403: the ambush flag not inherited'),
    'count': ({'p_mobj.c': [('    mo->spawnpoint = mobj->spawnpoint;\t\n', '    mo->spawnpoint = mobj->spawnpoint; totalkills++;\t\n')]},
              ['RESP4', 'RESP3R'], 'p_mobj.c:399: a respawned monster counted in totalkills'),
    'pause_run': ({'p_tick.c': [('    if (paused)\n', '    if (0)\n')]}, ['PAUSE', 'PAUSE0'], 'p_tick.c:135: P_Ticker runs while paused'),
    'pause_st': ({'g_game.c': [('\tST_Ticker (); \n', '\tif (!paused) ST_Ticker (); \n')]}, ['PAUSE'],
                 'g_game.c:731: ST_Ticker (and its M_Random draw) stopped while paused'),
    'pause_hu': ({'g_game.c': [('\tHU_Ticker ();            \n', '\tif (!paused) HU_Ticker ();            \n')]}, ['PAUSE'],
                 'g_game.c:733: HU_Ticker (the message timer) stopped while paused'),
    'pause_mask': ({'g_game.c': [('\t\tswitch (players[i].cmd.buttons & BT_SPECIALMASK) \n',
                                  '\t\tswitch (players[i].cmd.buttons == (BT_SPECIAL|BTS_PAUSE) ? BTS_PAUSE : 0) \n')]}, ['PAUSE'],
                   'g_game.c:704: only the byte 0x81 pauses'),
    'pause_btn': ({'p_user.c': [('    if (cmd->buttons & BT_SPECIAL)\n', '    if (0)\n')]}, ['PAUSE'],
                  'p_user.c:280: a special byte read as buttons when the level runs'),
    'pause_keep': ({'g_game.c': [('    sendpause = sendsave = paused = false; \n', '    sendpause = sendsave = false; \n')]}, ['PAUSEWI'],
                   'g_game.c:494: a level load keeps paused'),
    'pause_wi': ({'wi_stuff.c': [('\t    if (player->cmd.buttons & BT_ATTACK)\n',
                                  '\t    if ((player->cmd.buttons & BT_ATTACK) && !(player->cmd.buttons & BT_SPECIAL))\n')]}, ['PAUSEWI'],
                 'wi_stuff.c:1480: the intermission does not read a special byte as a fire press'),
}
# name: (item, [--prior-game values], the run whose game rows it must equal or None, what)
TABLES = {
    'T_3_3': ('FAST3', ['3,0,0'], 'FAST3', 'control: an earlier skill 3 game changes no game row'),
    'T_4_3': ('FAST3', ['4,0,0'], 'FAST3', 'Nightmare, then skill 3: the tables are put back (g_game.c:1429-1436)'),
    'T_4_4': ('FAST4', ['4,0,0'], 'FAST4', 'Nightmare twice: halved once (g_game.c:1421 is keyed on the previous skill)'),
    'T_4F_4': ('FAST4', ['4,0,1'], 'FAST4', 'Nightmare with -fast, then Nightmare: halved once'),
    'T_3F_3': ('FAST3', ['3,0,1'], None, 'skill 3 with -fast, then skill 3 without: the tables stay fast, fastparm is 0'),
    'T_3F_3F': ('FAST3F', ['3,0,1'], None, 'skill 3 with -fast twice: halved twice (run and pain durations 0)'),
    'T_4F4F_3': ('FAST3', ['4,0,1', '4,0,1'], None, 'Nightmare with -fast twice, then skill 3: halved twice, doubled once'),
}
MT = {enum_index('mobjtype_t', n): n[3:] for n in ('MT_PLAYER', 'MT_POSSESSED', 'MT_SHOTGUY', 'MT_TROOP', 'MT_SERGEANT', 'MT_SHADOWS',
                                                    'MT_BRUISER', 'MT_TFOG', 'MT_TROOPSHOT', 'MT_BRUISERSHOT')}
S_GIBS = enum_index('statenum_t', 'S_GIBS')
SARG = (enum_index('statenum_t', 'S_SARG_RUN1'), enum_index('statenum_t', 'S_SARG_PAIN2'))
MF_COUNTKILL, MF_AMBUSH = 0x400000, 0x20
RNDTABLE = [int(x) for x in re.findall(r'\d+', (lambda s: s[s.index('rndtable[256]'):s.index('};', s.index('rndtable[256]'))])(
    (base.SOURCE / 'm_random.c').read_text()))][1:]
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()
OBSERVER_ROWS = ('T,', 'K,', 'RM,')


# ------------------------------------------------------------------ runs -----------------------------------
def run(binary, name, out, sb=None, extra=(), keep=False):
    """One runner process: dict(code, stdout, stderr, marker, sound, session, m6b, frames, names, digests, game, dumps)."""
    if out.exists(): shutil.rmtree(out)
    (out / 'dumps').mkdir(parents=True)
    wads = [ROOT / w for w in WAD.wads(name)] + [ROOT / f'{WAD.SCRIPT_DIR}/{name}.lmp']
    cmd = [str(binary), *[x for w in wads for x in ('--wad', str(w))], '--demo', name, '--game-mode', 'registered',
           '--out', str(out / 'dumps'), *[x for k, v in M5C.RECOMMENDED.items() for x in (k, v)],
           *(['--render', 'every', '--screenblocks', str(sb), '--frames', str(out / 'frames.txt')] if sb else ['--render', 'none']),
           *extra, '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt'), '--m6b-out', str(out / 'm6b.txt')]
    try: p = subprocess.run(cmd, capture_output=True, text=True, env=ENV, timeout=300)
    except subprocess.TimeoutExpired: sys.exit(f'{name}: the runner did not end in 300 s')
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    names = ['load.txt'] + sorted(n for n in os.listdir(out / 'dumps') if n.startswith('tic_'))
    texts = [(out / 'dumps' / n).read_bytes() for n in names] if (out / 'dumps' / 'load.txt').exists() else []
    r = dict(code=p.returncode, stdout=p.stdout, stderr=p.stderr, marker=' end=marker ' in p.stdout, sound=rd('sound.txt'),
             session=rd('session.txt'), m6b=rd('m6b.txt'), frames=rd('frames.txt'), names=names if texts else [],
             digests=[sha(t) for t in texts],
             game=[sha(b''.join(l for l in t.splitlines(keepends=True) if not l.startswith(tuple(k.encode() for k in OBSERVER_ROWS))))
                   for t in texts], dumps=out / 'dumps')
    if not keep: shutil.rmtree(out)
    return r


def body(text):
    """A stream without its #config lines (they name the render mode and the screen size)."""
    return ''.join(l for l in text.splitlines(keepends=True) if not l.startswith('#config'))


def clean(r):
    return not r['code'] and r['marker'] and not W27.UB.search(r['stderr']) and 'AddressSanitizer' not in r['stderr']


def first_diff(a, b):
    """The first index at which two sequences differ, the shorter length when one is a prefix, None when equal."""
    for k, (x, y) in enumerate(zip(a, b)):
        if x != y: return k
    return None if len(a) == len(b) else min(len(a), len(b))


def place(strict, other):
    """Where a run first differs from the strict one: ('capture', j, the other's dump digest) or (stream, line, digest)."""
    k = first_diff(strict['digests'], other['digests'])
    if k is not None: return 'capture', k, other['digests'][k] if k < len(other['digests']) else '-'
    for stream in ('session', 'sound', 'frames', 'm6b'):
        a, b = body(strict[stream]).splitlines(), body(other[stream]).splitlines()
        k = first_diff(a, b)
        if k is not None: return stream, k + 1, sha(b[k]) if k < len(b) else '-'
    return None


# ------------------------------------------------------------------ observed facts (module text) -------------
def mobjs(d): return [m for k, m in d.thinkers if k == 'M']


def gates(dumps):
    """Every tic in which a corpse's respawn gate passed (P_Random () <= 4, p_mobj.c:467), when the tic's draws can be
    resolved: on a tic whose leveltime & 31 is 0, the corpses (MF_COUNTKILL, tics -1) whose movecount reaches 420 draw
    in thinker order; a passing corpse that is gone afterwards respawned (three more draws, p_mobj.c:375-398), one still
    there was blocked (P_CheckPosition, p_mobj.c:370). A tic is resolved when the corpses' draws so counted are all the
    tic's draws; other tics (a live monster also drew) are counted in `unresolved`."""
    out, unresolved = [], 0
    for j in range(len(dumps) - 1):
        a, b = dumps[j], dumps[j + 1]
        if a.h['leveltime'] & 31 or b.h['leveltime'] == a.h['leveltime']: continue
        el = [m for m in mobjs(a) if m['flags'] & MF_COUNTKILL and m['tics'] == -1 and m['movecount'] + 1 >= 12 * 35]
        if not el: continue
        k, rows, ok, after = a.h['prndindex'], [], True, mobjs(b)
        for m in el:
            k = (k + 1) & 255
            gone = not any(x['tics'] == -1 and (x['sp_x'], x['sp_y'], x['movecount']) == (m['sp_x'], m['sp_y'], m['movecount'] + 1)
                           for x in after)
            if RNDTABLE[k] <= 4:
                row = dict(tic=j, leveltime=a.h['leveltime'], type=MT.get(m['type'], m['type']), gibs=m['state'] == S_GIBS,
                           corpse=[m['x'], m['y']], radius=m['radius'], movecount=m['movecount'] + 1, spot=[m['sp_x'], m['sp_y']],
                           value=RNDTABLE[k], outcome='respawn' if gone else 'blocked')
                if gone:
                    k = (k + 3) & 255
                    new = after[-3:]
                    row.update(order=[MT.get(x['type'], x['type']) for x in new],
                               fogs=[[x['x'], x['y'], x['z'], x['tics']] for x in new[:2]],
                               monster=dict(x=new[2]['x'], y=new[2]['y'], z=new[2]['z'], angle=new[2]['angle'], tics=new[2]['tics'],
                                            reactiontime=new[2]['reactiontime'], ambush=bool(new[2]['flags'] & MF_AMBUSH),
                                            ceiling_minus_floor=new[2]['ceilingz'] - new[2]['floorz']))
                rows.append(row)
            elif gone: ok = False
        if k != b.h['prndindex'] or not ok: unresolved += 1; continue
        out += rows
    return out, unresolved


def respawns(dumps):
    """Every respawn, found by structure alone (whatever else drew in the tic): a corpse (MF_COUNTKILL, tics -1) that is
    gone a tic later while a thinker of its type with its spawn point and reactiontime 18 is new."""
    out = []
    for j in range(len(dumps) - 1):
        after = mobjs(dumps[j + 1])
        for m in mobjs(dumps[j]):
            if not (m['flags'] & MF_COUNTKILL and m['tics'] == -1): continue
            key = (m['type'], m['sp_x'], m['sp_y'])
            if any(x['tics'] == -1 and (x['type'], x['sp_x'], x['sp_y']) == key for x in after): continue
            new = [k for k, x in enumerate(after) if (x['type'], x['sp_x'], x['sp_y']) == key and x['reactiontime'] == 18]
            if len(new) != 1: continue
            k = new[0]
            out.append(dict(tic=j, leveltime=dumps[j].h['leveltime'], type=MT.get(m['type'], m['type']), movecount=m['movecount'] + 1,
                            corpse=[m['x'], m['y']], spot=[m['sp_x'], m['sp_y']],
                            thinkers_before_it=[MT.get(x['type'], x['type']) for x in after[max(k - 2, 0):k]],
                            thinkers_after_it=len(after) - 1 - k, monster_tics=after[k]['tics'],
                            ambush=bool(after[k]['flags'] & MF_AMBUSH), angle=after[k]['angle']))
    return out


def level_facts(dumps):
    """Kills against the level total, missile speeds, the demon state durations seen, each monster's first reactiontime
    and the first tic a missile exists."""
    f, speeds, sarg, react, first = {}, {}, set(), {}, {}
    for j, d in enumerate(dumps):
        for m in mobjs(d):
            t = MT.get(m['type'])
            if t in ('TROOPSHOT', 'BRUISERSHOT'):
                speeds.setdefault(t, set()).add(round(math.hypot(m['momx'], m['momy'], m['momz']) / 65536))
                first.setdefault(t, j - 1)
            if t in ('SERGEANT', 'SHADOWS') and SARG[0] <= m['state'] <= SARG[1]: sarg.add(m['tics'])
            if j == 0 and m['flags'] & MF_COUNTKILL: react[f"{t}@{m['sp_x']},{m['sp_y']}"] = m['reactiontime']
    last = dumps[-1]
    f.update(totalkills=last.h['totalkills'], killcount=last.players[0]['killcount'], spawned=sorted(react),
             reactiontime_at_load=sorted(set(react.values())), missile_speeds={k: sorted(v) for k, v in sorted(speeds.items())},
             first_missile_tic=dict(sorted(first.items())), demon_state_tics_seen=sorted(sarg))
    return f


def session_facts(r):
    """The paused intervals (from the G rows) with what moved in each, the pause-song calls, and the intermission's
    kill counter."""
    G = [l.split(',') for l in r['session'].splitlines() if l.startswith('G,')]
    col = dict(tic=1, gamestate=2, leveltime=6, gametic=7, prndindex=9, rndindex=10, paused=19)
    v = lambda row, k: int(row[col[k]])
    ivs, k = [], 0
    while k < len(G):
        if v(G[k], 'paused'):
            e = k
            while e + 1 < len(G) and v(G[e + 1], 'paused'): e += 1
            a, b = G[k], G[e]
            ivs.append(dict(first=v(a, 'tic'), last=v(b, 'tic'), gamestates=sorted({v(x, 'gamestate') for x in G[k:e + 1]}),
                            leveltime=[v(a, 'leveltime'), v(b, 'leveltime')], gametic=[v(a, 'gametic'), v(b, 'gametic')],
                            prndindex=[v(a, 'prndindex'), v(b, 'prndindex')], rndindex=[v(a, 'rndindex'), v(b, 'rndindex')]))
            k = e + 1
        else: k += 1
    f = dict(paused_intervals=ivs, song_calls=[l.split(',')[1:3] for l in r['m6b'].splitlines() if l.startswith('MP,')],
             paused_at_end=bool(G and v(G[-1], 'paused')))
    J = [l.split(',') for l in r['session'].splitlines() if l.startswith('J,')]
    I = [l.split(',') for l in r['session'].splitlines() if l.startswith('I,')]
    if J: f['intermission'] = dict(maxkills=int(I[0][6]), skills=int(I[0][13]), cnt_kills_max=max(int(x[9]) for x in J))
    return f


def tables_of(r):
    return [l.split(',')[1:] for l in r['m6b'].splitlines() if l.startswith('T,')]


# ------------------------------------------------------------------ records --------------------------------
def item_record(name, bins, work):
    fails = []
    runs = {b: run(bins[b], name, work / 'runs' / f'{name}_{b}', keep=b == 'strict') for b in CLAIM}
    s = runs['strict']
    if not clean(s): fails.append(f'{name}: strict exit {s["code"]}, marker {s["marker"]}: {s["stderr"][-300:]}')
    for b in CLAIM[1:]:
        r = runs[b]
        if b == 'address' and name in ADDRESS_STOPS:
            ub = W27.UB.findall(r['stderr'])
            n = len(r['digests'])
            if not r['code'] or len(ub) != 1 or G6.DECLARED_UB not in r['stderr'] or not n or r['digests'] != s['digests'][:n]:
                fails.append(f'{name}: address: not the one declared stop at {G6.DECLARED_UB_AT} after a prefix of the strict dumps')
            address_stops[name] = n
            continue
        if not clean(r) or any(r[k] != s[k] for k in ('digests', 'sound', 'session', 'm6b')):
            fails.append(f'{name}: {b} exit {r["code"]} or its dumps or streams differ from strict: {r["stderr"][-300:]}')
    dumps = [L.Dump(str(s['dumps'] / n)) for n in s['names']]
    g, unresolved = gates(dumps)
    facts = dict(level_facts(dumps), respawns=respawns(dumps), gates=g, gate_tics_unresolved=unresolved, **session_facts(s), tables=tables_of(s))
    rec = dict(name=name, tics=WAD.lump(name)[1], strict=s, facts=facts, fails=fails, rendered={})
    for sb in RENDER.get(name, ()):
        r = run(bins['strict'], name, work / 'runs' / f'{name}_sb{sb}', sb=sb)
        if not clean(r) or r['digests'] != s['digests'] or any(body(r[k]) != body(s[k]) for k in ('sound', 'session')):
            fails.append(f'{name}: the rendered run at screenblocks {sb} is not clean or its dumps, sound or session differ from the plain run')
        rec['rendered'][sb] = r
    shutil.rmtree(work / 'runs', ignore_errors=True)
    return rec


def variant_rows(bins, recs, work, only):
    out, fails = [], []
    for v, (_reps, names, _what) in VARIANTS.items():
        for name in names:
            if only and name not in only: continue
            sb = RENDER.get(name, (None,))[0]
            strict = recs[name]['rendered'][sb] if sb else recs[name]['strict']
            r = run(bins[v], name, work / 'runs' / f'{name}_{v}', sb=sb)
            if r['code'] not in (0,): fails.append(f'variant {v}: {name} exit {r["code"]}: {r["stderr"][-200:]}'); continue
            p = place(strict, r)
            if p is None: fails.append(f'variant {v}: {name} equals the genuine run'); continue
            out.append((v, name, *p))
    return out, fails


def table_rows(bins, recs, work, only):
    out, fails = [], []
    for t, (name, priors, same, what) in TABLES.items():
        if only and name not in only: continue
        r = run(bins['strict'], name, work / 'runs' / t, extra=[x for p in priors for x in ('--prior-game', p)])
        if not clean(r): fails.append(f'table {t}: exit {r["code"]}: {r["stderr"][-200:]}'); continue
        rows = tables_of(r)
        equal = {n: r['game'] == recs[n]['strict']['game'] for n in ('FAST3', 'FAST4', 'FAST3F') if n in recs}
        if same and not equal.get(same): fails.append(f'table {t}: the game rows differ from {same}')
        if not same and any(equal.values()): fails.append(f'table {t}: the game rows equal {[n for n, e in equal.items() if e]}')
        out.append(dict(name=t, item=name, priors=priors, what=what, tables=rows, equals=same or 'none',
                        game_sha256=sha(''.join(r['game']))))
    return out, fails


def genuine_text(recs, variants, tables):
    head = [f'#schema,{GENUINE_SCHEMA}',
            '#generator,tools/reference/m6b_goldens.py: the genuine M6b runner (strict build; overflow and address identical); '
            'never from Rust output',
            '#tics,a session tic is the 0-based index of a G_Ticker; capture j is the state after j tics',
            '#dumps,SHA-256 over the SHA-256 digests of every capture\'s dump text in order (load.txt, then tic_NNNNN.txt); '
            'game: the same over each dump without its T, K and RM rows',
            '#columns,I,item,source,skill,respawnparm,fastparm,tics,captures,end,dumps_sha256,game_sha256,session_sha256,sound_sha256,m6b_sha256',
            '#columns,R,item,tic,leveltime,type,gibs,corpse_x,corpse_y,radius,movecount,spot_x,spot_y,value,outcome,order',
            '#columns,T,run,item,priors,equals,game_sha256,when,skill,respawnparm,fastparm,respawnmonsters,sarg_tics,bruisershot_speed,headshot_speed,troopshot_speed',
            '#columns,V,variant,item,stream,place,variant_sha256']
    rows = []
    for name, r in recs.items():
        it, s = WAD.ITEMS[name], r['strict']
        data = WAD.lump(name)[0]
        end = 'complete' if len(s['digests']) == r['tics'] + 1 else f'level_tics:{len(s["digests"]) - 1}'
        rows.append(f"I,{name},{'E1M%d' % it['map'] if 'map' in it else it['base']},{data[1]},{data[5]},{data[6]},{r['tics']},"
                    f"{len(s['digests'])},{end},{sha(''.join(s['digests']))},{sha(''.join(s['game']))},{sha(s['session'])},"
                    f"{sha(G6.golden_sound(s['sound']))},{sha(s['m6b'])}")
        for g in r['facts']['gates']:
            rows.append(f"R,{name},{g['tic']},{g['leveltime']},{g['type']},{int(g['gibs'])},{g['corpse'][0]},{g['corpse'][1]},{g['radius']},"
                        f"{g['movecount']},{g['spot'][0]},{g['spot'][1]},{g['value']},{g['outcome']},{' '.join(g.get('order', ['-']))}")
    for t in tables:
        rows += [f"T,{t['name']},{t['item']},{' '.join(t['priors'])},{t['equals']},{t['game_sha256']},{','.join(row)}" for row in t['tables']]
    rows += [f'V,{v},{n},{stream},{k},{d}' for v, n, stream, k, d in variants]
    return '\n'.join(head + rows) + '\n'


def files_of(recs):
    out = {}
    for name, r in recs.items():
        if name in SESSION: out[DIRS['session'] / f'{name}.session'] = r['strict']['session']
        if name in SOUND: out[DIRS['sound'] / f'{name}.sound'] = G6.golden_sound(r['strict']['sound'])
        for sb, x in r['rendered'].items():
            out[DIRS['frames'] / f'{name}.sb{sb}.frames'] = G6.golden_frames(x['frames'])
            out[DIRS['pause'] / f'{name}.sb{sb}.txt'] = x['m6b']
    return out


def results(recs, variants, tables, cc):
    digest_of = lambda p: sha((ROOT / p).read_bytes())
    inputs = sorted({w for n in recs for w in WAD.wads(n)} | {f'{WAD.SCRIPT_DIR}/{n}.lmp' for n in recs})
    return dict(
        schema=SCHEMA, revision=base.REVISION,
        claim='every value is the genuine engine\'s strict build; the overflow and address builds gave identical dumps and streams',
        builds={b: base.VARIANTS[W27.BUILDS[b][0]] for b in CLAIM}, asan_options=W27.ASAN_NOTE,
        variants={v: dict(what=what, replacements={f: [dict(original=o, replacement=n) for o, n in reps] for f, reps in r.items()},
                          items=i) for v, (r, i, what) in VARIANTS.items()},
        variant_first_differences=[dict(variant=v, item=n, stream=s, place=k) for v, n, s, k, _d in variants],
        address_declared_stops=dict(report=G6.DECLARED_UB, captures_before_the_stop=dict(address_stops)),
        tables=tables, tools_sha256={t: digest_of(f'tools/reference/{t}') for t in TOOLS},
        inputs_sha256={p: digest_of(p) for p in inputs},
        waits_on_decisions='state digests over the wire 29 schema (DSIM45) are not generated: decisions N2, N4, N5 and N6',
        generated_with=dict(compiler=subprocess.run([cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0]),
        items={n: dict(what=WAD.ITEMS[n]['what'], tics=r['tics'], captures=len(r['strict']['digests']),
                       rendered_at=sorted(r['rendered']), observed=r['facts']) for n, r in recs.items()})


def selftest(bins, work, recs):
    out = []
    if 'RESP4' in recs:
        r = run(bins['strict'], 'RESP4', work / 'runs' / 'self_draw', extra=['--selftest-extra-draw', '700'])
        k = first_diff(recs['RESP4']['strict']['digests'], r['digests'])
        out.append(('an extra P_Random draw before session tic 700 of RESP4', f'first differing capture {k}', k == 701))
    if 'PAUSE' in recs:
        sb = RENDER['PAUSE'][0]
        r = run(bins['strict'], 'PAUSE', work / 'runs' / 'self_y', sb=sb, extra=['--selftest-pause-y'])
        s = recs['PAUSE']['rendered'][sb]
        first = recs['PAUSE']['facts']['paused_intervals'][0]['first']
        fr = lambda x: [l for l in G6.golden_frames(x['frames']).splitlines() if l.startswith('F,')]
        k = first_diff(fr(s), fr(r))
        out.append(('the pause picture one row lower in PAUSE', f'first differing frame row {k}, first paused tic {first}',
                    k == first and r['digests'] == s['digests'] and r['session'] == s['session']))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--only', help='authoring: these items only (nothing is written or compared)')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'm6b' / 'goldens')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    only = a.only.split(',') if a.only else None
    bins = {b: B.build(b, work / 'build', a.cc) for b in CLAIM}
    for v, (reps, names, _w) in VARIANTS.items():
        if not only or set(names) & set(only): bins[v] = B.variant(v, reps, work / 'build', a.cc)
    recs, fails = {}, []
    for name in WAD.ITEMS:
        if only and name not in only: continue
        r = recs[name] = item_record(name, bins, work)
        fails += r['fails']
        f = r['facts']
        print(f"{name}: {r['tics']} tics, {len(r['strict']['digests'])} captures, kills {f['killcount']}/{f['totalkills']}, "
              f"respawns {[x['tic'] for x in f['respawns']]}, gates {[(g['tic'], g['outcome']) for g in f['gates']]}, paused {[(i['first'], i['last']) for i in f['paused_intervals']]}")
    variants, f = variant_rows(bins, recs, work, only); fails += f
    for v, n, stream, k, _d in variants: print(f'variant {v}: {n} first differs at {stream} {k}')
    tables, f = table_rows(bins, recs, work, only); fails += f
    for t in tables: print(f"table {t['name']}: {t['item']} after {t['priors']}: {t['tables'][-1][4:]}, equals {t['equals']}")
    if a.selftest:
        for what, detail, ok in selftest(bins, work, recs):
            print(f'selftest {"pass" if ok else "FAIL"}: {what}: {detail}')
            if not ok: fails.append(f'selftest: {what}')
    shutil.rmtree(work / 'runs', ignore_errors=True)
    if fails: print('\n'.join(['FAILURES:'] + fails)); return 1
    if only: print('--only: nothing written or compared'); return 0
    texts = {GENUINE: genuine_text(recs, variants, tables), **files_of(recs)}
    res = results(recs, variants, tables, a.cc)
    print(f"compiler: {res['generated_with']['compiler']}; {W27.ASAN_NOTE}")
    if a.check:
        old = json.loads(RESULTS.read_text()) if RESULTS.exists() else {}
        old.pop('generated_with', None); res.pop('generated_with')
        bad = [str(p.relative_to(ROOT)) for p, t in texts.items() if not p.exists() or p.read_text() != t] + ([] if old == res else [RESULTS.name])
        stray = [str(p.relative_to(ROOT)) for d in DIRS.values() if d.exists() for p in d.iterdir() if p not in texts]
        if bad or stray: print(f'differs from the regenerated text: {", ".join(bad)}; not generated: {", ".join(stray)}'); return 1
        print(f'{len(texts)} golden files and {RESULTS.name} match ({len(recs)} items, {len(variants)} variant rows, {len(tables)} table runs)')
    else:
        for p, t in texts.items(): p.parent.mkdir(parents=True, exist_ok=True); p.write_text(t)
        RESULTS.write_text(json.dumps(res, indent=1) + '\n')
        print(f'wrote {len(texts)} golden files and {RESULTS.name}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
