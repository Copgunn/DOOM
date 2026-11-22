#!/usr/bin/env python3
"""Goldens of the M6a session items from the genuine engine only (docs/M6A_REFERENCE_INVESTIGATION.md, Phase 2 step 2).

Every item of tests/reference/m6a_manifest.json (tools/reference/m6a_wad.py: the synthetic multi-map WADs, pictures,
music and scripts) is run by the genuine M6a runner (tools/reference/m6a_build.py, contract
tests/reference/m6a_runner_contract.json) in the item's declared game mode, with the M5c recommended switches
(m5c_reference.RECOMMENDED) and the runner's default melt clock (counter, M5c's declared clock). Never from Rust output.
Per item:

  strict    screenblocks 10 with dumps, frames, sound, session and view streams; screenblocks 11 with frames and sound
            (its sound stream must equal the screenblocks-10 one);
  variants  the overflow and address builds at screenblocks 10: every stream and the dump set identical to strict
            (address runs use ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so not LeakSanitizer coverage),
            and no sanitizer report;
  melt      strict at screenblocks 10 with --melt-clock unit (the runner's evidence switch): its melt rows (frame V and Y
            rows, sound W rows) go to a separate file that makes no claim (the declared melt schedule is settled by a
            later section of the note); every other row must equal the counter run's.
  modes     the item CHAIN is also run in the other game mode: every stream must equal the declared mode's apart from
            the lines that print the mode (the modes differ on the Episode 1 path only in the finale's art screen,
            f_finale.c:714-720, and the episode-2 switch textures, p_switch.c:113-119, which no synthetic map names).

Goldens (written without --check, compared byte for byte with --check, which writes nothing to the repository):
  tests/reference/m6a_session/<ITEM>.session       the session stream (DOOM-M6A-SESSION-v1), screenblocks 10
  tests/reference/m6a_sound/<ITEM>.sound           the sound stream (DOOM-M5C-SOUND-v1 rows plus the M6a LV rows)
                                                   without the melt rows W and with the totals' melt step count
                                                   masked (each melt's 320 draws stay in the R rows)
  tests/reference/m6a_frames/<ITEM>.sb<N>.frames   the frame stream's golden rows (m5c_reference.normalise: F, P, X, M,
                                                   U, N, O, E and Q) and the M6a G rows (a whole-frame digest of every
                                                   intermission and finale frame), without the melt rows V and Y
  tests/reference/m6a_melt/<ITEM>.melt             the unit-clock melt rows (frame V and Y, sound W, the melt step
                                                   total), labelled "no claim"
  tests/reference/m6a_results.json                 DOOM-M6A-RESULTS-v1: inputs and tools by digest, per item the counts,
                                                   the end, the dump set's digest per level load, the golden digests,
                                                   the variant and melt facts, and the U8 witnesses observed
--selftest proves the comparisons can fail: faults planted in the runner (a started sound's volume, an extra P_Random
draw, a flipped level pixel) must each make the matching golden comparison differ at its place, and a changed session
value and a changed whole-frame digest (planted in memory in a copy of the golden text) must be caught.

    m6a_goldens.py [--check] [--selftest] [--only ITEM[,ITEM]] [--work DIR] [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, hashlib, json, os, re, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m3h9c_build as base    # noqa: E402
import m3h9c_dump as dump     # noqa: E402
import m5c_reference as M5C   # noqa: E402  (read-only: normalise, end_of, RECOMMENDED)
import m5c_sfx                # noqa: E402  (read-only: the sfx table parsed from sounds.c)
import m5a_reference as A     # noqa: E402  (read-only: SUMMARY)
import m5c_build              # noqa: E402
import m6a_build              # noqa: E402
import m6a_wad as W           # noqa: E402

MANIFEST = ROOT / W.MANIFEST
RESULTS = ROOT / 'tests/reference/m6a_results.json'
SCHEMA = 'DOOM-M6A-RESULTS-v1'
DIRS = {k: ROOT / f'tests/reference/m6a_{k}' for k in ('session', 'sound', 'frames', 'melt')}
VARIANTS = ['strict', 'overflow', 'address', 'address_bounds']
# The genuine engine forms an out-of-bounds pointer at every level start with a weapon of no ammunition ready (the fist or
# the chainsaw, which a session carries): ST_createWidgets passes &plyr->ammo[weaponinfo[readyweapon].ammo] with
# am_noammo = 5 into int ammo[4] (st_stuff.c:1288-1294, d_player.h); ST_Ticker replaces it by &largeammo in the same
# G_Ticker, before any frame reads it (st_stuff.c:926-935). UBSan's bounds check stops the address build there. So the
# address build is compared through the last tic before that report (which must be exactly DECLARED_UB), and a fourth
# build, address_bounds, is the address build with only the bounds check recoverable (-fsanitize-recover=bounds,
# declared here, built through m6a_build with the frozen flag table extended in memory, not on disk): it must run to
# the strict end with identical streams, and every report it prints must be DECLARED_UB.
DECLARED_UB_AT = 'st_stuff.c:1292:16'
DECLARED_UB = DECLARED_UB_AT + ": runtime error: index 5 out of bounds for type 'int [4]'"
ADDRESS_BOUNDS_FLAGS = ['-fsanitize-recover=bounds']
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
MODE_CHECK = 'CHAIN'
TOOLS = ['m6a_goldens.py', 'm6a_wad.py', 'm6a_build.py', 'm6a_session.c', 'm6a_wi_access.c']
SFX = {name: i for i, name, _l in m5c_sfx.sfx_table()}
MELT_HEADER = ('#schema,DOOM-M6A-MELT-v1\n'
               '#claim,none: melt pictures under the runner\'s --melt-clock unit evidence switch; the declared melt schedule is '
               'settled by a later section of docs/M6A_REFERENCE_INVESTIGATION.md, and the original\'s in-between pictures '
               'depended on machine speed\n')
sha = lambda b: hashlib.sha256(b).hexdigest()


def items(only=None):
    man = json.loads(MANIFEST.read_text())
    out = []
    for it in man['items']:
        it = dict(it, wads=[ROOT / w for w in man['wadsets'][it['wadset']]], lmp_path=ROOT / it['lmp'])
        out.append(it)
    if only:
        want = {x.upper() for x in only.split(',')}
        out = [i for i in out if i['name'] in want]
    return out


def run(binary, item, out, sb, dumps=False, extra=(), mode=None):
    """One runner process: dict(code, end, stderr, stdout, frames, sound, session, view, snapshots[(name, sha)])."""
    if out.exists(): shutil.rmtree(out)
    out.mkdir(parents=True)
    cmd = [str(binary), *[x for w in item['wads'] + [item['lmp_path']] for x in ('--wad', str(w))], '--demo', item['lump'],
           '--game-mode', mode or item['mode'], *(['--out', str(out / 'dumps')] if dumps else ['--no-dump']),
           *[x for k, v in M5C.RECOMMENDED.items() for x in (k, v)], '--render', 'every', '--screenblocks', str(sb), *extra,
           '--frames', str(out / 'frames.txt'), '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt'),
           '--view', str(out / 'view.txt')]
    if dumps: (out / 'dumps').mkdir()
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    snaps = [(n, sha((out / 'dumps' / n).read_bytes())) for n in dump.snapshot_names(str(out / 'dumps'))] if dumps else []
    r = dict(code=p.returncode, summary=A.SUMMARY.search(p.stdout), stderr=p.stderr, stdout=p.stdout, frames=rd('frames.txt'),
             sound=rd('sound.txt'), session=rd('session.txt'), view=rd('view.txt'), snapshots=snaps)
    r['end'] = M5C.end_of(r)
    shutil.rmtree(out)
    return r


def rows(text, kind):
    return [l.split(',') for l in text.splitlines() if l.startswith(kind + ',')]


def cols(text, kind):
    for l in text.splitlines():
        f = l.split(',')
        if f[0] == '#columns' and f[1] == kind: return f[2:]
    raise KeyError(kind)


def table(text, kind):
    c = cols(text, kind)
    return [dict(zip(c, r[1:])) for r in rows(text, kind)]


def golden_frames(text):
    """The golden rows: m5c_reference.normalise's (melt rows V removed) and the M6a G rows, headers kept but those of V
    and Y."""
    full = M5C.normalise(text)[1]
    keep = set(M5C.GOLDEN_KINDS) - {'V'} | {'G'}
    out = []
    for l in full.splitlines(keepends=True):
        k = l.split(',', 1)[0]
        if l.startswith(('#columns,V,', '#columns,Y,')): continue
        if l.startswith('#') or k in keep: out.append(l)
    return ''.join(out)


def golden_sound(text):
    """The sound stream without its melt rows (W: the draws and steps of each melt, whose step count depends on the
    melt clock) and with the totals row's melt step count masked; the 320 draws of each melt stay in the R rows."""
    out = [re.sub(r',melt_steps,\d+', ',melt_steps,-', l) for l in text.splitlines(keepends=True)
           if not l.startswith(('W,', '#columns,W,'))]
    return ''.join(out)


def melt_rows(frames, sound):
    total = re.search(r',melt_steps,(\d+)', sound)
    return MELT_HEADER + f'#melt_steps,{total[1] if total else "-"}\n' + ''.join(l for l in frames.splitlines(keepends=True) if l.startswith(('#columns,V,', '#columns,Y,', 'V,', 'Y,'))) \
        + ''.join(l for l in sound.splitlines(keepends=True) if l.startswith(('#columns,W,', 'W,')))


def without(text, kinds):
    """`text` without the rows of `kinds` and the configuration lines (which print the clock and the mode)."""
    return ''.join(l for l in text.splitlines(keepends=True) if not l.startswith(tuple(k + ',' for k in kinds) + ('#config',)))


def segments(snaps, session):
    """The dump set's digest per level load: snapshots grouped by the load they follow (L rows of the session)."""
    loads = [int(r[1]) for r in rows(session, 'L')]          # an L row carries its load's tic (-1: before tic 0)
    out, cur, h = [], None, None
    for name, digest in snaps:
        k = 0 if name == 'load.txt' else max(i for i, t in enumerate(loads) if t <= int(name[4:9]))
        if k != cur:
            if h: out.append(dict(load=cur, snapshots=n, sha256=h.hexdigest()))
            cur, h, n = k, hashlib.sha256(), 0
        h.update(digest.encode()); n += 1
    if h: out.append(dict(load=cur, snapshots=n, sha256=h.hexdigest()))
    return out


def truncated(r, last):
    """The run's streams and dumps through tic `last`: header and N rows kept, end rows (Z, frame W) dropped, as
    m5c_reference.truncate cuts a stream."""
    def cut(t):
        out = []
        for l in t.splitlines(keepends=True):
            f = l.split(',')
            if l.startswith('#') or f[0] == 'N': out.append(l)
            elif len(f) > 1 and f[1].lstrip('-').isdigit() and f[0] != 'Z' and int(f[1]) <= last: out.append(l)
        return ''.join(out)
    snaps = [(n, d) for n, d in r['snapshots'] if n == 'load.txt' or int(n[4:9]) <= last]
    return dict(frames=cut(r['frames']), sound=cut(r['sound']), session=cut(r['session']), view=cut(r['view']), snapshots=snaps,
                end=r['end'])


def counts(r):
    s = r['session']
    melts = table(r['frames'], 'Y') if 'Y,' in r['frames'] else []
    return dict(end=r['end'], session_tics=len(rows(s, 'G')), loads=len(rows(s, 'L')), wipes=len(melts),
                level_frames=len(rows(r['frames'], 'F')), other_frames=len(rows(r['frames'], 'G')),
                sound_starts=len(rows(r['sound'], 'S')), dumps=len(r['snapshots']))


# ---------------------------------------------------------------- the U8 witnesses ------------------------
def session_tables(r):
    s = r['session']
    G = {int(g['tic']): g for g in table(s, 'G')}
    C = {int(g['tic']): g for g in table(s, 'C')}
    J = {int(g['tic']): g for g in table(s, 'J')}
    I = {int(g['tic']): g for g in table(s, 'I')}
    L = table(s, 'L')
    S = table(r['sound'], 'S')
    return G, C, J, I, L, S


def intermissions(G):
    """[(first tic, last tic)] of every intermission."""
    out, start = [], None
    for t in sorted(G):
        if G[t]['gamestate'] == '1' and start is None: start = t
        if G[t]['gamestate'] != '1' and start is not None: out.append((start, t - 1)); start = None
    if start is not None: out.append((start, max(G)))
    return out


def loads_after_first(L):
    return [int(l['tic']) for l in L[1:]]                 # an L row carries the tic of its load's G_Ticker


def lmp_buttons(item):
    """{tic: buttons} of the demo's ticcmds with a button (the version-110 layout, g_game.c:1489-1505)."""
    b = item['lmp_path'].read_bytes()[13:-1]
    return {t: b[4 * t + 3] for t in range(len(b) // 4) if b[4 * t + 3]}


def witness(name, r, buttons=None):
    """Observed facts for the U8 quirks this item reaches: {quirk: [fact strings]}; a fact that does not hold raises."""
    if 'G,' not in r['session']: return {}                                  # stopped before tic 0
    G, C, J, I, L, S = session_tables(r)
    ints = intermissions(G)
    f = {}
    add = lambda q, s: f.setdefault(str(q), []).append(s)

    def need(cond, what):
        if not cond: raise SystemExit(f'{name}: witness does not hold: {what}')
        return what

    exits = [t for t in sorted(G) if G[t]['gameaction'] == '6' and G[t]['gamestate'] == '0']   # ga_completed (d_event.h:54-63)
    for a, b in ints:                                                       # 2 and 3: the tally waits; draws without pictures
        acc = [t for t in range(a, b + 1) if J[t]['sp_state'] == '10' and (t == a or J[t - 1]['sp_state'] != '10')]
        wi = int(G[b]['wi_draws']) - int(G[a - 1]['wi_draws'])
        if b == max(G) and G[b]['gamestate'] == '1' and J[b]['state'] == '0' and J[b]['sp_state'] == '10':
            held = sum(1 for t in range(a, b + 1) if J[t]['sp_state'] == '10')
            add(2, need(held > 1, f'intermission from tic {a}: tally state 10 from tic {acc[0] if acc else "?"}, still waiting for a '
                                 f'press at the last tic {b} ({held} tics in state 10)'))
            digests = {x[3] for x in rows(r['frames'], 'G') if a <= int(x[1]) <= b and J[int(x[1])]['sp_state'] == '10'}
            ctr = {J[t]['anim_ctr'] for t in range(a, b + 1) if J[t]['sp_state'] == '10'}
            add(3, need(len(digests) == 1 and len(ctr) > 1,
                        f'intermission from tic {a}: one frame digest over the {held} state-10 tics while the animation '
                        f'counter takes {len(ctr)} values (WI_drawAnimatedBack returns, wi_stuff.c:588); {wi} intermission '
                        f'M_Random draws'))
        elif G[b]['gameaction'] == '8' or (b + 1 in G and G[b + 1]['gamestate'] == '0'):
            add(3, need(wi == 20, f'intermission {a}-{b}: 20 intermission M_Random draws (10 at WI_Start, 10 at the sgcock tic)'))
    for a, b in ints:                                                       # 2: a button held from the level
        for btn, key in (('use', 'usedown'), ('fire', 'attackdown')):
            if C[a - 1][key] != '1': continue
            held = next((t for t in range(a, b + 1) if C[t][key] != '1'), b + 1) - a
            jump = [t for t in range(a, b + 1) if J[t]['state'] == '0' and J[t]['sp_state'] == '10'
                    and (t == a or J[t - 1]['sp_state'] != '10') and (t > a and J[t - 1]['sp_state'] not in ('9',))]
            if held > 1:
                add(2, need(not jump or jump[0] >= a + held,
                            f'intermission from tic {a}: {btn} held from the level ({key} 1 at the exit), {held} tics with no skip; '
                            f'the tally jumped to state 10 first on tic {jump[0] if jump else "-"}, after the release'))
    for k, (a, b) in enumerate(ints):                                       # 4: the first ShowNextLoc frame's pointer
        snl = [t for t in range(a, b + 1) if J[t]['state'] == '1' and (t == a or J[t - 1]['state'] != '1')]
        if snl:
            t = snl[0]
            want = '0' if k == 0 else '1'
            add(4, need(J[t]['snl_pointeron'] == want,
                        f'intermission {k + 1} (tic {a}): snl_pointeron {J[t]["snl_pointeron"]} on the frame of the sgcock tic {t}'
                        + (' (the first of the process: hidden)' if k == 0 else ' (left set by the previous intermission: shown)')))
    marked = False
    for a, b in ints:                                                       # 5: the secret level's splat
        i = I[a]
        if i['last'] == '8' or i['next'] == '8' or int(i['didsecret']) or marked:
            add(5, f'intermission from tic {a}: last {int(i["last"]) + 1}, next {int(i["next"]) + 1}, didsecret {i["didsecret"]}'
                   + (' (unmarked again after the reborn)' if marked and not int(i['didsecret']) else ''))
        marked = marked or bool(int(i['didsecret']))
    for t in loads_after_first(L):                                          # 7 and 8: carried and dropped at each load
        c0, c = C[t - 1], C[t]
        pre_ints = [x for x in ints if x[1] == t - 1]
        if not pre_ints: continue                                           # a reborn reload: item 10
        ex = max(e for e in exits if e < t)
        ce = C[ex]
        carried = all(ce[k] == c[k] for k in ('armorpoints', 'armortype', 'weaponowned', 'ammo0', 'ammo1', 'ammo2', 'ammo3',
                                               'maxammo0', 'backpack', 'readyweapon')) and (ce['health'] == c['health'] or ce['playerstate'] == '1')
        if ce['playerstate'] == '1':
            add(7, need(c['playerstate'] == '0' and int(c['health']) > 0 and c['readyweapon'] == '1' and c['weaponowned'] == '3'
                        and c['ammo0'] == '50',
                        f'exit tic {ex} with the player dead (playerstate 1, health {ce["health"]}): the load at tic {t} starts a live '
                        f'player with the pistol and 50 bullets (G_PlayerReborn), health {c["health"]} after that tic\'s floor damage'))
            continue
        add(7, need(carried, f'exit tic {ex} -> load tic {t}: health {ce["health"]}, armor {ce["armorpoints"]}, weapons {ce["weaponowned"]}, '
                             f'ammo {ce["ammo0"]}/{ce["ammo1"]}/{ce["ammo2"]}/{ce["ammo3"]}, backpack {ce["backpack"]} carried'))
        if int(ce['cards']) or any(int(ce[f'pw{k}']) for k in range(6)):
            add(7, need(c['cards'] == '0' and all(c[f'pw{k}'] == '0' for k in range(6)),
                        f'exit tic {ex}: cards {ce["cards"]}, powers {",".join(ce[f"pw{k}"] for k in range(6))}; cards and powers 0 from the next tic'))
        if ce['pendingweapon'] != '10' and ce['pendingweapon'] != ce['readyweapon']:
            add(7, need(c0['pendingweapon'] == ce['pendingweapon'] and c['pendingweapon'] in ('10', ce['readyweapon'])
                        and c['readyweapon'] == ce['readyweapon'],
                        f'exit tic {ex}: weapon change to {ce["pendingweapon"]} pending (ready {ce["readyweapon"]}); load tic {t}: ready '
                        f'{c["readyweapon"]}, pending {c["pendingweapon"]} (cancelled by P_SetupPsprites)'))
        if ce['deltaviewheight'] != '0':
            add(7, f'exit tic {ex}: deltaviewheight {ce["deltaviewheight"]} (a landing); load tic {t}: {c["deltaviewheight"]}')
        if ce['readyweapon'] == '7':
            saw = [s for s in S if int(s['tic']) == t and s['sfx'] == str(SFX['sawup'])]
            add(8, need(saw, f'load tic {t} with the chainsaw ready: sfx_sawup started on the load tic'))
    for e in exits:                                                         # 9: level sounds at the exit tic
        started = [s for s in S if int(s['tic']) == e]
        if not (started and e + 1 in G and G[e + 1]['gamestate'] == '1'): continue
        nxt = min([int(l['tic']) for l in L if int(l['tic']) > e] + [max(G) + 1])
        K = table(r['sound'], 'K') if 'K,' in r['sound'] else []
        R = {int(x['tic']): x for x in table(r['sound'], 'R')}
        need(not [k for k in K if e <= int(k['tic']) < nxt and k['reason'] == 'level_start'], f'exit tic {e}: no channel stopped by the completion')
        cut = [k for k in K if e < int(k['tic']) < nxt and k['reason'] == 'same_origin']
        add(9, f'exit tic {e}: {", ".join("sfx " + s["sfx"] for s in started)} started on the exit tic; {R[e + 1]["occupied"]} channel(s) '
               f'still occupied after the first intermission tic {e + 1}'
               + ''.join(f'; the level\'s sfx {k["sfx"]} still playing on tic {k["tic"]}, cut by a tally sound (no origin, s_sound.c:471-483)'
                         for k in cut)
               + '; no channel stopped by the completion (S_Start stops them at the next load, s_sound.c:209-211); the mixer was not '
                 'run in this step')
    fin = [t for t in sorted(G) if G[t]['gamestate'] == '2']
    if fin:                                                                 # 1 and 11: E1M8 into the finale
        f0, f1 = fin[0], fin[-1]
        e = f0 - 1
        F = {int(x['tic']): x for x in table(r['session'], 'F')}
        add(1, need(G[e]['gamestate'] == '0' and G[e]['gameaction'] == '6' and G[e]['map'] == '8' and G[f0]['gamestate'] == '2'
                    and G[f0]['wi_draws'] == G[e]['wi_draws'],
                    f'E1M8 exit on tic {e}; tic {f0} is the finale (gamestate 2): no intermission, no intermission draws'))
        s1 = [t for t in fin if F[t]['finalestage'] == '1']
        if s1:
            pressed = sorted(t for t in (buttons or {}) if f0 <= t <= f1)
            add(11, need(s1[0] - f0 + 1 == 1571 and all(G[t]['gameaction'] == '0' and G[t]['gamestate'] == '2' for t in fin)
                         and F[f1]['finalestage'] == '1' and G[f1]['fin_draws'] == G[f0]['fin_draws'],
                         f'finale tics {f0}-{f1}: stage 1 from tic {s1[0]} (finale tic 1571), still stage 1 with no game action at the '
                         f'last tic; presses on tics {pressed} changed nothing; no finale M_Random draw'))
    for e in exits:                                                         # 12: a reborn and an exit in one tic
        if e + 1 not in G: continue
        nxt = 'the map reloaded (leveltime 1 on tic %d)' % (e + 1) if G[e + 1]['gamestate'] == '0' else 'the intermission'
        if C[e]['playerstate'] == '2':
            add(12, need(G[e + 1]['gamestate'] == '0' and G[e + 1]['map'] == G[e]['map'] and G[e + 1]['leveltime'] == '1',
                         f'tic {e}: the exit (gameaction 6) and the reborn (playerstate 2) in one tic; tic {e + 1}: {nxt}, no intermission'))
        elif C[e]['playerstate'] == '1':
            add(12, need(G[e + 1]['gamestate'] == '1', f'tic {e}: the exit taken by the dead player (playerstate 1); tic {e + 1}: {nxt}'))
    if name.startswith('USEAT') and not exits:
        t = int(L[1]['tic'])
        add(12, need(G[t]['gamestate'] == '0' and G[t]['leveltime'] == '1', f'no exit: the reborn reloaded the map on tic {t}'))
    for l in L[1:]:                                                         # 10: the reborn
        t = int(l['tic'])
        if not any(x[1] == t - 1 for x in ints) and G[t]['map'] == G[t - 1]['map']:
            dead = max(x for x in sorted(C) if x < t and C[x]['playerstate'] == '1' and (C[x - 1]['playerstate'] != '1'))
            reb = min(x for x in sorted(C) if x >= dead and C[x]['playerstate'] == '2')
            add(10, need(reb + 1 == t and l['prndindex_before'] == G[t - 1]['prndindex'],
                         f'death on tic {dead}, reborn requested on tic {reb}, the map reloaded on tic {t}; P_Random index '
                         f'{l["prndindex_before"]} -> {l["prndindex_after"]} and M_Random index {l["rndindex_before"]} -> '
                         f'{l["rndindex_after"]} across the load (not reset)'))
    return f


def item_record(item, bins, runs, other_mode=False):
    """Every run of one item: (record, golden texts {path: text}, failures)."""
    N = item['name']; fails = []
    s10 = run(bins['strict'], item, runs / f'{N}_s10', 10, dumps=True)
    s11 = run(bins['strict'], item, runs / f'{N}_s11', 11)
    unit = run(bins['strict'], item, runs / f'{N}_unit', 10, extra=['--melt-clock', 'unit'])
    var = {v: run(bins[v], item, runs / f'{N}_{v}', 10, dumps=True) for v in VARIANTS if v != 'strict'}
    ubs = lambda r: sorted({re.sub(r'^\S*/', '', l.strip()) for l in r['stderr'].splitlines() if 'runtime error' in l})
    rec = dict(mode=item['mode'], wadset=item['wadset'], lmp_sha256=item['lmp_sha256'], **counts(s10))
    if s10['end'] != s11['end'] or s10['sound'] != s11['sound']: fails.append(f'{N}: sound or end differs between screenblocks 10 and 11')
    rec['variants'] = {}
    for v, r in var.items():
        ref, got, note = s10, r, {}
        if v == 'address' and r['end'].startswith('sanitizer'):
            last = M5C.last_tic_of(r)                                  # stopped by DECLARED_UB: compared through its last tic
            if ubs(r) != [DECLARED_UB]: fails.append(f'{N}: address reports {ubs(r)}')
            ref, got = truncated(s10, last), truncated(r, last)
            note = dict(stopped_by=DECLARED_UB, compared_through_tic=last)
        if v == 'address_bounds':
            note = dict(bounds_reports=ubs(r))
            if any(u != DECLARED_UB for u in ubs(r)): fails.append(f'{N}: address_bounds reports {ubs(r)}')
            else:                                                       # the run's end without the declared report
                r['end'] = M5C.end_of(dict(r, stderr='\n'.join(l for l in r['stderr'].splitlines() if DECLARED_UB_AT not in l)))
        keys = ('frames', 'sound', 'session', 'view', 'snapshots') + (('end',) if not note.get('stopped_by') else ())
        diff = [k for k in keys if got[k] != ref[k]]
        if diff: fails.append(f'{N}: {v} differs from strict ({diff})')
        rec['variants'][v] = dict(identical_to_strict=not diff, **note)
    steps = lambda t: re.sub(r',melt_steps,\d+', ',melt_steps,-', t)       # the sound Z row's melt step total
    um = without(unit['frames'], ('V', 'Y')) == without(s10['frames'], ('V', 'Y')) and \
        steps(without(unit['sound'], ('W',))) == steps(without(s10['sound'], ('W',))) and \
        without(unit['session'], ()) == without(s10['session'], ())
    if not um: fails.append(f'{N}: the unit melt clock changed rows other than the melt rows')
    Y = table(unit['frames'], 'Y') if 'Y,' in unit['frames'] else []
    Vr = table(unit['frames'], 'V') if 'V,' in unit['frames'] else []
    Gs = {int(g['tic']): g for g in table(s10['session'], 'G')} if 'G,' in s10['session'] else {}
    Ls = {int(l['tic']) for l in table(s10['session'], 'L')} if 'L,' in s10['session'] else set()
    def kind(y):
        t = int(y['tic'])
        if t - 1 not in Gs: return 'session start'
        a, b = Gs[t - 1]['gamestate'], y['gamestate']
        return {('0', '1'): 'level to intermission', ('1', '0'): 'intermission to level', ('0', '2'): 'level to finale',
                ('2', '2'): 'finale text to art screen'}.get((a, b), 'reborn reload' if (a, b) == ('0', '0') and t in Ls else f'{a} to {b}')
    kinds = {}
    for y in Y:
        k = kinds.setdefault(kind(y), dict(melts=0, start_differs_from_end=0))
        k['melts'] += 1; k['start_differs_from_end'] += y['start_sha256'] != y['end_sha256']
    rec['melt'] = dict(claim='none', melts=len(Y), steps=[sum(1 for v in Vr if v['tic'] == y['tic']) for y in Y],
                       start_differs_from_end=sum(1 for y in Y if y['start_sha256'] != y['end_sha256']), kinds=kinds,
                       other_rows_equal_to_counter_clock=um)
    G2 = [g for g in table(s10['frames'], 'G')] if 'G,' in s10['frames'] else []
    rec['whole_frames_2d_undefined'] = sum(1 for g in G2 if any(int(g[k]) for k in ('skipped_patches', 'beyond_rows', 'lump_overrun')))
    rec['flagged_level_frames'] = sum(1 for x in table(s10['frames'], 'F') if int(x['columns']) > 0) if 'F,' in s10['frames'] else 0
    od = re.search(r'other_draws (\d+)', s10['stdout'])
    rec['other_draws'] = int(od[1]) if od else None
    if od and od[1] != '0': fails.append(f'{N}: other_draws {od[1]}')
    rec['dump_set_sha256'] = sha(''.join(d for _n, d in s10['snapshots']).encode())
    rec['dumps_per_load'] = segments(s10['snapshots'], s10['session'])
    texts = {f'tests/reference/m6a_session/{N}.session': s10['session'], f'tests/reference/m6a_sound/{N}.sound': golden_sound(s10['sound']),
             f'tests/reference/m6a_frames/{N}.sb10.frames': golden_frames(s10['frames']),
             f'tests/reference/m6a_frames/{N}.sb11.frames': golden_frames(s11['frames']),
             f'tests/reference/m6a_melt/{N}.melt': melt_rows(unit['frames'], unit['sound'])}
    rec['goldens'] = {p: sha(t.encode()) for p, t in texts.items()}
    try:
        rec['witnesses'] = witness(N, s10, lmp_buttons(item))
    except SystemExit as e:
        fails.append(str(e)); rec['witnesses'] = {}
    if other_mode:
        om = 'retail' if item['mode'] == 'registered' else 'registered'
        o = run(bins['strict'], item, runs / f'{N}_{om}', 10, dumps=True, mode=om)
        same = all(without(o[k], ()) == without(s10[k], ()) for k in ('frames', 'sound', 'session', 'view')) \
            and o['snapshots'] == s10['snapshots'] and o['end'] == s10['end']
        rec['other_mode'] = dict(mode=om, identical_but_the_mode_lines=same)
        if not same: fails.append(f'{N}: the {om} run differs from the {item["mode"]} run')
    for r, who in [(s10, 'strict'), (s11, 'strict sb11'), (unit, 'unit')] + [(r, v) for v, r in var.items()]:
        if r['end'].startswith(('sanitizer', 'abnormal')) and not (who == 'address' and rec['variants']['address'].get('stopped_by')):
            fails.append(f'{N}: {who} ended {r["end"]}')
    return rec, texts, fails


def build_all(work, cc):
    base.VARIANTS.setdefault('address_bounds', base.VARIANTS['address'] + ADDRESS_BOUNDS_FLAGS)
    m5c_build.DRIVER_FLAGS.setdefault('address_bounds', m5c_build.DRIVER_FLAGS['address'])
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try: return {v: m6a_build.build(v, work / 'build', cc) for v in VARIANTS}
        finally: sys.stdout = so


def first_diff(a, b):
    al, bl = a.splitlines(), b.splitlines()
    for i, (x, y) in enumerate(zip(al, bl)):
        if x != y: return i, x, y
    return (min(len(al), len(bl)), None, None) if len(al) != len(bl) else None


def selftest(bins, work):
    """Each planted fault must make the matching comparison differ at its place."""
    item = items('CHAIN')[0]
    gold = {k: (ROOT / f'tests/reference/m6a_{k}/CHAIN.{ext}').read_text() for k, ext in (('sound', 'sound'), ('session', 'session'))}
    gold['frames'] = (ROOT / 'tests/reference/m6a_frames/CHAIN.sb10.frames').read_text()
    res = json.loads(RESULTS.read_text())['items']['CHAIN']
    out = []
    r = run(bins['strict'], item, work / 'self_sound', 10, extra=['--selftest-sound', '200'])
    d = first_diff(gold['sound'], golden_sound(r['sound']))
    out.append(('started volume', f'--selftest-sound 200: first sound difference {d and d[1:]}',
                bool(d and d[1].startswith('S,') and int(d[1].split(',')[1]) >= 200)))
    r = run(bins['strict'], item, work / 'self_draw', 10, dumps=True, extra=['--selftest-extra-draw', '300'])
    segs = segments(r['snapshots'], r['session'])
    bad = [s['load'] for s, g in zip(segs, res['dumps_per_load']) if s != g]
    out.append(('extra P_Random draw', f'--selftest-extra-draw 300: dump digests differ from load {bad[:1]} on',
                sha(''.join(x for _n, x in r['snapshots']).encode()) != res['dump_set_sha256'] and bool(bad) and bad[0] == 2))
    r = run(bins['strict'], item, work / 'self_pixel', 10, extra=['--selftest-pixel', '420', '100', '100'])
    d = first_diff(gold['frames'], golden_frames(r['frames']))
    out.append(('flipped pixel', f'--selftest-pixel 420 100 100: first frame difference {d and d[1][:40]}',
                bool(d and d[1].startswith('F,420,'))))
    lines = gold['session'].splitlines(keepends=True)
    k = next(i for i, l in enumerate(lines) if l.startswith('C,500,'))
    f = lines[k].split(','); f[3] = str(int(f[3]) + 1); planted = ''.join(lines[:k] + [','.join(f)] + lines[k + 1:])
    d = first_diff(gold['session'], planted)
    out.append(('session value', f'armor of C,500 changed in a copy: first difference at line {d and d[0]}', bool(d and d[0] == k)))
    lines = gold['frames'].splitlines(keepends=True)
    k = next(i for i, l in enumerate(lines) if l.startswith('G,130,'))
    f = lines[k].split(','); f[3] = f[3][::-1]; planted = ''.join(lines[:k] + [','.join(f)] + lines[k + 1:])
    d = first_diff(gold['frames'], planted)
    out.append(('whole-frame digest', f'G,130 digest reversed in a copy: first difference at line {d and d[0]}', bool(d and d[0] == k)))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m6a-goldens')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--only')
    ap.add_argument('--jobs', type=int, default=1)
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    bins = build_all(work, a.cc)
    todo = items(a.only) if (a.check or a.only or not a.selftest) else []
    failures, results, texts = [], {}, {}
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        futs = [pool.submit(item_record, i, bins, work / 'runs', i['name'] == MODE_CHECK) for i in todo]
        for item, fut in zip(todo, futs):
            rec, tx, fails = fut.result()
            failures += fails; results[item['name']] = rec; texts.update(tx)
            print(f"{item['name']}: {rec['end']}; {rec['session_tics']} tics, {rec['loads']} loads, {rec['wipes']} wipes, "
                  f"{rec['level_frames']} level + {rec['other_frames']} other frames, {rec['sound_starts']} sound starts, "
                  f"{rec['dumps']} dumps; variants {'ok' if not fails else 'see failures'}; witnesses "
                  f"{sorted(rec['witnesses'], key=int)}", flush=True)
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    if todo and not a.only:
        man = json.loads(MANIFEST.read_text())
        wit = {}
        for n, rec in results.items():
            for q, facts in rec['witnesses'].items(): wit.setdefault(q, []).append(n)
        doc = dict(schema=SCHEMA, runner_contract_sha256=sha(m6a_build.CONTRACT.read_bytes()),
                   tools_sha256={f'tools/reference/{n}': sha(base.text(ROOT / 'tools/reference' / n).encode('latin-1')) for n in TOOLS},
                   manifest_sha256=sha(MANIFEST.read_bytes()),
                   inputs_sha256={f['file']: f['sha256'] for f in man['files']},
                   frozen_inputs_sha256={p: sha((ROOT / p).read_bytes()) for p in (W.GFX_M5B, W.SFX_M5C)},
                   switches=dict(M5C.RECOMMENDED, **{'--render': 'every', '--melt-clock': 'counter (melt files: unit)'}),
                   views=[10, 11], variants=VARIANTS, asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
                   claim='every golden is the genuine engine\'s stream (strict build), identical in the overflow and address builds; '
                         'the melt files make no claim',
                   totals=dict(items=len(results), session_tics=sum(r['session_tics'] for r in results.values()),
                               loads=sum(r['loads'] for r in results.values()), wipes=sum(r['wipes'] for r in results.values()),
                               level_frames=sum(r['level_frames'] for r in results.values()),
                               other_frames=sum(r['other_frames'] for r in results.values()),
                               sound_starts=sum(r['sound_starts'] for r in results.values()),
                               dumps=sum(r['dumps'] for r in results.values())),
                   witnessed_by={q: wit[q] for q in sorted(wit, key=int)},
                   melt_kinds={k: dict(melts=sum(r['melt']['kinds'].get(k, {}).get('melts', 0) for r in results.values()),
                                       start_differs_from_end=sum(r['melt']['kinds'].get(k, {}).get('start_differs_from_end', 0)
                                                                  for r in results.values()))
                               for k in sorted({k for r in results.values() for k in r['melt']['kinds']})},
                   items=results)
        texts[str(RESULTS.relative_to(ROOT))] = json.dumps(doc, indent=1) + '\n'
        if a.check:
            bad = [p for p, t in texts.items() if not (ROOT / p).exists() or (ROOT / p).read_text() != t]
            extra = [str(p.relative_to(ROOT)) for d in DIRS.values() if d.is_dir() for p in sorted(d.iterdir())
                     if str(p.relative_to(ROOT)) not in texts]
            if bad or extra:
                for p in bad:
                    o = work / 'regenerated' / p; o.parent.mkdir(parents=True, exist_ok=True); o.write_text(texts[p])
                sys.exit(f'differs: {bad}; not generated: {extra} (regenerated text under {work / "regenerated"})')
            print(f'{len(texts)} golden files match ({len(results)} items)')
        else:
            for d in DIRS.values():
                if d.is_dir():
                    for p in d.iterdir():
                        if str(p.relative_to(ROOT)) not in texts: p.unlink()
            for p, t in texts.items():
                (ROOT / p).parent.mkdir(parents=True, exist_ok=True); (ROOT / p).write_text(t)
            print(f'wrote {len(texts)} golden files ({len(results)} items)')
    if a.selftest:
        outcomes = selftest(bins, work)
        for what, detail, ok in outcomes: print(f"selftest {what}: {detail}: {'pass' if ok else 'FAIL'}")
        if not all(ok for _, _, ok in outcomes): sys.exit(1)


if __name__ == '__main__':
    main()
