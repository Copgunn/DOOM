#!/usr/bin/env python3
"""Whole simulation state of the M6a level-chain items from the genuine engine: DSIM42 digests (M6a Phase 2 step 8).

The goldens of tools/reference/m6a_goldens.py hold session, frame and sound rows but no per-level state. This tool
runs every level-chain item of tests/reference/m6a_manifest.json (CHAIN_ITEMS, then RULING_ITEMS: the items of the
owner's rulings of 2026-11-22, the turbo message across loads, intermission commands outside the level domain and the
stale attacker)
through the genuine M6a runner with
dumps, projects every dump with the INDEPENDENT DSIM42 encoder (tools/reference/level_snapshot_m6a.py, itself the
frozen DSIM41 projection of level_snapshot_m5c.py plus wire 26's start group) and writes digests only. Never from
Rust output.

The runner. The M6a runner (tools/reference/m6a_build.py) dumps every tic that ends in a level (tic_NNNNN.txt, NNNNN
the session tic) and the first load (load.txt), but not the later loads: a later load and its level's tic 0 share
one G_Ticker (g_game.c:617-650 then P_Ticker, 727-729), so no tic ends between them. The projection needs a load
dump to start each level's actor store (an actor freed in tic 0 still had its slot). This tool therefore builds a
STATE variant of the runner, in its own work directory, from the M6a runner's generated driver and
tools/reference/m6a_session.c with two exact-once patches made in memory (nothing on disk changes):
  - the driver keeps the dump directory (`m6a_state_dir`) after the first load's dump;
  - the P_SetupLevel observer (__wrap_P_SetupLevel, called by G_DoLoadLevel, g_game.c:484) writes, after every load
    but the first, the dump loads/load_NNNNN.txt (NNNNN the session tic of the load) of the state P_SetupLevel left,
    and then restores the observer's lists of freed and removed thinkers, so every tic dump is byte for byte the
    plain runner's (checked on every run against the committed dumps_per_load of m6a_results.json).
Every other stream of the run (session, sound, frames at screenblocks 10) must equal the committed golden.

The projection (declared rules, beyond those of level_snapshot_m6a.py):
  - one level is one load: its load dump (load.txt or loads/load_T.txt) and the tic dumps of the session tics from
    its load tic T to the tic before the next load; capture j (H.leveltime) is the state after level tic j - 1,
    whose genuine sound rows are those of session tic T + j - 1 (the stream is read with that offset);
  - the P_InitThinkers observer (m5c_driver.c __wrap_P_InitThinkers) records the previous level's thinkers as
    freed at the load (p_setup.c:622 Z_FreeTags the zone); serials only grow, so in a later level a K or RM row
    naming a serial below the level's first serial (the least serial of its load dump) is the previous level's and
    is dropped;
  - the start group: kind, P_Random index, health, armour, weapons, ammunition, the backpack, cheats and the
    latches from the session rows (level_snapshot_m6a.start_from_rows); bob, psprites[ps_weapon].sx and the null
    flash psprite's tics, sx and sy (which the C rows do not record) from the previous level's last dump's P row,
    which neither G_PlayerFinishLevel (g_game.c:779-792), the intermission nor P_SetupPsprites (p_pspr.c:831-842,
    states only) changes; the load dump must hold the same values (checked);
  - compared state of a capture: the DSIM42 fields without `map.*` and `events.*` in level_compare.stripped's
    framing ('DSIM', u32 42, u64 field count, then per field u32 path length, path, u64 value length, value),
    followed by the `events.sounds.*` fields in the same per-field framing; its SHA-256 is the capture's state
    digest. `events.last` and `events.pending` (the encoder writes both with count 0, as DSIM40, M5c note 4.4.4)
    are left out; `map.*` is digested once per level (capture 1) in the per-field framing;
  - a last load with no capture (the demo's end marker read in the load's own G_Ticker, after the gameaction
    loop: g_game.c:617-650, then 667 and 1493-1496) is no level of the comparison: no P_Ticker runs for it;
  - the stale attacker (owner's ruling of 2026-11-22): after an exit the C leaves players[0].attacker pointing into
    the freed level (G_PlayerFinishLevel, g_game.c:779-792, and P_SpawnPlayer, p_mobj.c:675-683, leave it), where
    the dump writes -2 (a pointer to no thinker of the dump, tidx); it is never read before it is overwritten or
    wiped (note section 1.2, the attacker row). A -2 in P[0].attacker is projected as NULL only when all four hold:
    (a) it is player 0's attacker (no other reference); (b) the level was entered by a carried load
    (start_from_rows kind 'carried'); (c) the previous level's last dump had a non-NULL attacker (not -1, before
    this rule); (d) the value has not changed since the load: in the load dump and in every dump of the level up to
    this one, P[0].attacker is -2, P[0].playerstate is PST_LIVE, every mobj whose player is player 0 (M and RR rows:
    players[0].mo and any earlier body a second player-1 start left) has threshold 0, and no mobj is an MT_VILE
    (owner's ruling of 2026-11-22: the body's threshold, not damagecount). (d) is read from the dump this way because
    the dump names no address. Every write of attacker is p_inter.c:874 (player = target->player, 798); after it, a
    killing hit returns at 888-892 through P_KillMobj, which sets PST_DEAD (706), and every other hit reaches 904-911,
    which sets target->threshold to BASETHRESHOLD (100, p_local.h:61) when it was 0, whatever the damage (also a hit
    halved to 0 damage on skill 0, sk_baby, at 799-800), unless the source is NULL (attacker written NULL, not -2),
    the target itself (attacker its own live body) or an MT_VILE (904-906). A player body's threshold starts at 0
    (P_SpawnMobj's memset, p_mobj.c:490-491; a new body every level) and only A_Look (p_enemy.c:608) and A_Chase
    (681-689), monster actions a player body never runs, lower it; and a dead player is live again only through
    G_PlayerReborn's wipe. MT_VILE is a Doom II thing (P_SpawnMapThing skips it outside commercial mode,
    p_setup.c:318-331); the rule refuses any level in which one is present. The history check (-2 since the load)
    is kept as well. Any other -2 is still refused by the encoder (level_snapshot.py:841). The loads it fired on are counted and printed, per
    item and in total;
  - the session points: every capture j >= 1 (the state after each level tic) is in its level's sequence digest
    (SHA-256 of the captures' state digests, 32 bytes each, in order); D rows name capture 1 (the load tic), every
    capture whose leveltime is a multiple of STRIDE, the last capture of each level (the tic before a boundary or
    the run's last level tic). Capture 0 of a later level is no session state (its tic 0 runs in the same tic).

Results: tests/reference/m6a_state.txt (DOOM-M6A-STATE-v1), written without --check, compared byte for byte with
--check (which writes nothing). Counts and digests only.

--selftest proves the digests can fail: a planted extra P_Random draw at session tic 300 of CHAIN changes the
sequence digest of exactly the level holding tic 300 and the later ones; a bob one unit off (lower, or 1 for 0) in
the carried start groups changes every capture digest of the first carried level; and the previous level's K rows
left in (the serial rule off) are refused by the projection. The stale-attacker rule, on CHAIN's dumps changed in
memory (the first carried level's previous level ends with the player's own body as attacker): with P[0].attacker -2
in every dump of the carried level the rule fires on that load only and the level's captures equal NULL's; refused
with "a dangling reference": a -2 appearing at level tic 10 (mid-level), a -2 sector soundtarget at the load (a),
no attacker at the previous level's end (c), a threshold of 100 on players[0].mo at level tic 10 with
damagecount 0 and a live attacker at level tic 10 that is -2 again at 11 (d), and the same -2 attacker on REBHELD's reborn reload (b); on the carried load dump itself, condition (d) fails with a
threshold on another body of player 0, a dead player or an MT_VILE present; and with P[0].attacker the player's own
body at CHAIN's carried load (a live thinker: the rule does not fire) the load capture differs from the committed
D row, which the unchanged projection equals (the gate compares the field).

    m6a_state.py [--check] [--selftest] [--only ITEM[,ITEM]] [--work DIR] [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, hashlib, json, os, shutil, struct, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m3h9c_dump as dump      # noqa: E402
import level_snapshot as L     # noqa: E402
import level_snapshot_m5c as M5C   # noqa: E402
import level_snapshot_m6a as S     # noqa: E402
import level_compare as K      # noqa: E402
import m6a_build as B          # noqa: E402
import m6a_goldens as G        # noqa: E402  (read-only: items, the recommended switches, golden_* and segments)
from m3h9_geometry import enum_index   # noqa: E402

RESULTS = ROOT / 'tests/reference/m6a_state.txt'
SCHEMA = 'DOOM-M6A-STATE-v1'
CHAIN_ITEMS = ['CHAIN', 'INTSTA', 'INTSTB', 'INTIDLE', 'INTHELD', 'INTFAST', 'REBHELD', 'REBLATE', 'SECRET', 'REBSECRT',
               'PROBE', 'USEAT130', 'USEAT131', 'USEAT132', 'RBXDEAD', 'NOINTER', 'NOE1M2', 'NOE1M2AT']
RULING_ITEMS = ['TURBO', 'INTCMD', 'ATTACK']     # ATTACK: the stale attacker (ruling of 2026-11-22)
ITEMS = CHAIN_ITEMS + RULING_ITEMS
STRIDE = 35
PST_LIVE = enum_index('playerstate_t', 'PST_LIVE', 'd_player.h')          # 0
MT_VILE = enum_index('mobjtype_t', 'MT_VILE', 'info.h')                   # 3
UNRECORDED = dict(bob='bob', weapon_x='ps0sx', flash_tics='ps1tics', flash_x='ps1sx', flash_y='ps1sy')
sha = lambda b: hashlib.sha256(b).hexdigest()

DRIVER_PATCH = ('    if (!nodump) dump (out, "load.txt", -1);\n',
                '    if (!nodump) dump (out, "load.txt", -1);\n'
                '    m6a_state_dir = nodump ? NULL : out;      /* M6a state variant: the later loads\' dumps */\n')
SESSION_PATCHES = [
    ('void __real_P_SetupLevel (int episode, int map, int playermask, skill_t skill);\n',
     '/* M6a state variant (tools/reference/m6a_state.py): the dump of a later load, then the observer\'s freed and\n'
     ' * removed lists restored, so the next tic dump is the plain runner\'s byte for byte. */\n'
     'static char* m6a_state_dir;\n'
     'static void m6a_state_load_dump (void)\n'
     '{\n'
     '    char name[64]; int nf = nfreed, nr = nremoved, fs = freeseq;\n'
     '    snprintf (name, sizeof name, "loads/load_%05d.txt", snd_tic);\n'
     '    dump (m6a_state_dir, name, snd_tic);\n'
     '    nfreed = nf; nremoved = nr; freeseq = fs;\n'
     '}\n'
     'void __real_P_SetupLevel (int episode, int map, int playermask, skill_t skill);\n'),
    ('    m6a_loads++;\n',
     '    m6a_loads++;\n'
     '    if (m6a_loads > 1 && m6a_state_dir) m6a_state_load_dump ();   /* M6a state variant */\n'),
]


def patched(text, patches):
    for old, new in patches:
        if text.count(old) != 1: sys.exit(f'state patch does not apply exactly once ({text.count(old)}): {old[:60]!r}')
        text = text.replace(old, new)
    return text


def build(work, cc):
    """The state variant of the strict M6a runner (module text), built under `work`."""
    work.mkdir(parents=True, exist_ok=True)
    session = work / 'patched' / B.SESSION_C.name
    session.parent.mkdir(exist_ok=True)
    session.write_bytes(patched(B.text(B.SESSION_C), SESSION_PATCHES).encode('latin-1'))
    plain_driver, plain_session = B.driver_text, B.SESSION_C
    B.driver_text = lambda: patched(plain_driver(), [DRIVER_PATCH])
    B.SESSION_C = session
    try:
        with open(os.devnull, 'w') as q:
            so, sys.stdout = sys.stdout, q
            try: return B.build('strict', work / 'build', cc)
            finally: sys.stdout = so
    finally:
        B.driver_text, B.SESSION_C = plain_driver, plain_session


def run(binary, item, out, extra=()):
    """One state-variant run at screenblocks 10 with every stream (m6a_goldens.run's command plus the load dumps)."""
    if out.exists(): shutil.rmtree(out)
    (out / 'dumps' / 'loads').mkdir(parents=True)
    cmd = [str(binary), *[x for w in item['wads'] + [item['lmp_path']] for x in ('--wad', str(w))], '--demo', item['lump'],
           '--game-mode', item['mode'], '--out', str(out / 'dumps'),
           *[x for k, v in G.M5C.RECOMMENDED.items() for x in (k, v)], '--render', 'every', '--screenblocks', '10', *extra,
           '--frames', str(out / 'frames.txt'), '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt')]
    p = subprocess.run(cmd, capture_output=True, text=True, env=G.ENV)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    return dict(code=p.returncode, stdout=p.stdout, stderr=p.stderr, frames=rd('frames.txt'), sound=rd('sound.txt'),
                session=rd('session.txt'), dumps=out / 'dumps')


class Shifted:
    """A genuine sound stream read with an offset: rows(j) are the rows of session tic j + offset."""
    def __init__(self, base, offset):
        self.base, self.offset, self.last_tic = base, offset, base.last_tic - offset

    def rows(self, tic):
        return self.base.rows(tic + self.offset)


def extra_lumps(item):
    """The lump names after the first WAD (W_AddFile order) and the demo's (D_AddFile), for R_InitFlats."""
    names = []
    for w in item['wads'][1:]:
        data = Path(w).read_bytes()
        _, count, offset = struct.unpack_from('<4sii', data, 0)
        names += [struct.unpack_from('<8s', data, offset + 16 * k + 8)[0].split(b'\0')[0].decode('latin-1') for k in range(count)]
    return tuple(names) + L.demo_lumps(str(item['lmp_path']))


def framed(fields):
    return S.framed(fields)


def state_digest(fields):
    """The capture's state digest (module text)."""
    body, _ = K.stripped(S.VERSION, fields)
    return hashlib.sha256(body + framed([(p, v) for p, v in fields if p.startswith('events.sounds.')])).digest()


def drop_previous(d, first):
    """The dump without the K and RM rows of serials below `first` (module text)."""
    d.freed = [(q, s) for q, s in d.freed if s >= first]
    d.removed = {s: r for s, r in d.removed.items() if s >= first}
    if d.adds is not None: d.adds = {s: a for s, a in d.adds.items() if s >= first}
    return d


def p_row(d):
    return next(p for p in d.players if p['idx'] == 0)


class StaleAttacker:
    """The stale-attacker rule of one level (module text): armed by (a) to (c) at the load, disarmed for the rest of
    the level by the first dump that shows a change (d); `apply` projects P[0].attacker -2 as NULL while armed."""
    def __init__(self, kind, previous_raw):
        self.armed = kind == 'carried' and previous_raw is not None and previous_raw != -1
        self.fired = False

    def apply(self, d):
        p = p_row(d)
        d.raw_attacker = p['attacker']
        if not untouched(d): self.armed = False
        elif self.armed: p['attacker'] = -1; self.fired = True
        return d


def untouched(d):
    """Condition (d) on one dump (module text): P[0].attacker -2, P[0] PST_LIVE, threshold 0 on every mobj of
    player 0 (M and RR rows), no MT_VILE."""
    p = p_row(d)
    mobjs = [r for k, r in d.thinkers if k == 'M'] + list(d.pending.values())
    return (p['attacker'] == -2 and p['playerstate'] == PST_LIVE
            and all(r['threshold'] == 0 for r in mobjs if r['player'] == 0)
            and not any(r['type'] == MT_VILE for r in mobjs))


def levels(r, item, rule=True, bob_delta=0, plant=None):
    """Project every level of a run: [dict(load, tic, map, kind, captures [(leveltime, session tic, digest)], map,
    stale_attacker)]. `plant(k, j, last, dump)` (self-tests only) changes dump j of level k in memory first."""
    session, sound = r['session'], r['sound']
    stream = M5C.SoundStream(text=sound)
    loads = S.rows(session, 'L')
    skill = L.demo_skill(str(item['lmp_path']))
    lumps = extra_lumps(item)
    ddir = r['dumps']
    tics = sorted(int(n[4:9]) for n in os.listdir(ddir) if n.startswith('tic_'))
    out, previous = [], None
    for k, l in enumerate(loads):
        T = l['tic']; end = loads[k + 1]['tic'] if k + 1 < len(loads) else 1 << 30
        names = (['load.txt'] if k == 0 else [f'loads/load_{T:05}.txt']) + [f'tic_{t:05}.txt' for t in tics if max(T, 0) <= t < end]
        load = L.Dump(str(ddir / names[0]))
        first = min(s for s, _ in load.serials.values()) if k else None
        kind, p, _h, _s = S.start_from_rows(session, sound, k, str(item['wads'][0]))
        stale = StaleAttacker(kind, previous.raw_attacker if previous is not None else None)

        def dump_at(j):
            d = load if j == 0 else L.Dump(str(ddir / names[j]))
            if plant: plant(k, j, j == len(names) - 1, d)
            return stale.apply(drop_previous(d, first) if k and rule else d)
        if kind == 'carried':
            want = {f: p_row(previous)[c] for f, c in UNRECORDED.items()}
            if {f: p_row(load)[c] for f, c in UNRECORDED.items()} != want:
                raise L.ProjectionError('DumpDeficiency', 0, f'load {k}: the load dump does not hold the carried bob and psprites')
            p.update(want)
            if bob_delta: p['bob'] = p['bob'] - 1 if p['bob'] > 0 else 1
        mapdata = L.MapData(str(item['wads'][0]), f'E1M{l["map"]}', lumps)
        proj = S.Projector(mapdata, skill, l['map'], Shifted(stream, max(T, 0)), (kind, p))
        caps, mapg, following = [], None, dump_at(0)
        for j in range(len(names)):
            d, following = following, dump_at(j + 1) if j + 1 < len(names) else None
            fields, _ = proj.step(d, following)
            last = d
            if d.tic == 0: continue
            if mapg is None: mapg = sha(framed([x for x in fields if x[0].startswith('map.')]))
            caps.append((d.tic, d.h['tic'], state_digest(fields)))
        if [c[0] for c in caps] != list(range(1, len(caps) + 1)):
            raise L.ProjectionError('DumpDeficiency', 0, f'load {k}: level tics not consecutive')
        if not caps:
            # a load whose G_Ticker never completed: the demo's end marker is read after the gameaction loop
            # in the same G_Ticker, before any P_Ticker (g_game.c:617-650, then 667 and 1493-1496)
            if k != len(loads) - 1: raise L.ProjectionError('DumpDeficiency', 0, f'load {k}: no capture')
            break
        out.append(dict(load=k, tic=T, map=l['map'], kind=kind, captures=caps, mapg=mapg, stale_attacker=stale.fired))
        previous = last
    return out


def rows_of(name, lv):
    """The L and D rows of one item."""
    out = []
    for v in lv:
        caps = v['captures']
        seq = sha(b''.join(c[2] for c in caps))
        out.append(f'L,{name},{v["load"]},{v["tic"]},{v["map"]},{v["kind"]},{len(caps)},{v["mapg"] or "-"},{seq}')
        for j, (lt, tic, d) in enumerate(caps):
            why = [w for w, c in (('load', j == 0), ('stride', lt % STRIDE == 0), ('last', j == len(caps) - 1)) if c]
            if why: out.append(f'D,{name},{v["load"]},{tic},{lt},{"+".join(why)},{d.hex()}')
    return out


def check_run(r, item, results):
    """Failures if the state variant's run is not the plain runner's (the committed goldens and dumps_per_load)."""
    name, fails = item['name'], []
    want = lambda p: (ROOT / p).read_text()
    if r['session'] != want(f'tests/reference/m6a_session/{name}.session'): fails.append(f'{name}: session stream differs')
    if G.golden_sound(r['sound']) != want(f'tests/reference/m6a_sound/{name}.sound'): fails.append(f'{name}: sound stream differs')
    if G.golden_frames(r['frames']) != want(f'tests/reference/m6a_frames/{name}.sb10.frames'):
        fails.append(f'{name}: frame stream differs')
    snaps = [(n, sha((r['dumps'] / n).read_bytes())) for n in dump.snapshot_names(str(r['dumps']))]
    if G.segments(snaps, r['session']) != results['items'][name]['dumps_per_load']:
        fails.append(f'{name}: tic dumps differ from the committed dumps_per_load')
    return fails


def run_item(binary, item, work, results):
    name = item['name']
    r = run(binary, item, work / 'runs' / name)
    fails = check_run(r, item, results)
    lv = levels(r, item) if (r['dumps'] / 'load.txt').exists() else []
    shutil.rmtree(work / 'runs' / name)
    return name, rows_of(name, lv), fails, sum(len(v['captures']) for v in lv), len(lv), stale_fired(lv)


def stale_fired(lv):
    """The loads of a projected run on which the stale-attacker rule fired."""
    return sum(1 for v in lv if v['stale_attacker'])


def results_text(per_item):
    head = [f'#schema,{SCHEMA}',
            '#generator,tools/reference/m6a_state.py: the genuine M6a runner (state variant, strict) and the independent '
            'DSIM42 encoder tools/reference/level_snapshot_m6a.py; never from Rust output',
            '#state,SHA-256 of the DSIM42 capture without map.* and events.* (level_compare.stripped framing) followed by '
            'its events.sounds.* fields (per-field framing); map: SHA-256 of the map.* fields of capture 1',
            f'#points,L: every capture j >= 1 of a level in sequence_sha256 (SHA-256 of the 32-byte state digests in order); '
            f'D: capture 1 (load), every leveltime multiple of {STRIDE} (stride) and the level\'s last capture (last)',
            '#columns,L,item,load,tic,map,kind,captures,map_sha256,sequence_sha256',
            '#columns,D,item,load,tic,leveltime,why,state_sha256']
    return '\n'.join(head + [l for name in ITEMS for l in per_item.get(name, [])]) + '\n'


def selftest(binary, items_by_name, work, results):
    out = []
    it = items_by_name['CHAIN']
    base = run(binary, it, work / 'self_base')
    lv = levels(base, it)
    seqs = [sha(b''.join(c[2] for c in v['captures'])) for v in lv]
    r = run(binary, it, work / 'self_draw', extra=['--selftest-extra-draw', '300'])
    got = [sha(b''.join(c[2] for c in v['captures'])) for v in levels(r, it)]
    at = max(k for k, v in enumerate(lv) if v['tic'] <= 300)
    differ = [k for k, (a, b) in enumerate(zip(seqs, got)) if a != b]
    ok = differ == list(range(at, len(lv)))
    out.append(('extra P_Random draw at tic 300', f'levels {differ} differ (want {at}..{len(lv) - 1})', ok))
    bent = levels(base, it, bob_delta=1)
    k = next(k for k, v in enumerate(lv) if v['kind'] == 'carried')
    diff = [j for j, v in enumerate(bent) if [c[2] for c in v['captures']] != [c[2] for c in lv[j]['captures']]]
    every = all(a[2] != b[2] for a, b in zip(bent[k]['captures'], lv[k]['captures']))
    out.append(('carried bob one unit off in the start group', f'levels {diff} differ, every capture of level {k}: {every}', k in diff and every))
    try:
        levels(base, it, rule=False); refused = None
    except L.ProjectionError as e: refused = f'{e.kind} at tic {e.tic}: {e.text}'
    out.append(('previous level\'s K rows kept', f'refused: {refused}', refused is not None))
    out += attacker_selftest(binary, items_by_name, work, base, lv)
    for d in ('self_base', 'self_draw'): shutil.rmtree(work / d, ignore_errors=True)
    return out


def attacker_selftest(binary, items_by_name, work, base, lv):
    """The stale-attacker rule (module text) on dumps changed in memory: it fires on a carried load whose previous
    level ended with a non-NULL attacker and gives that level the captures of NULL; a dangling attacker appearing
    mid-level is refused; and so is a dangling pointer that fails any one of the four conditions."""
    out, it = [], items_by_name['CHAIN']
    K = next(i for i, v in enumerate(lv) if v['kind'] == 'carried'); KL = lv[K]['load']

    def planter(prev_live=True, dangling_from=0, extra=None, level=KL, body=False):
        # as the C would hold it: the stale value (or, with `body`, the player's own body) from the load (or
        # `dangling_from`) up to the level's first write of attacker (p_inter.c:874-875: the genuine dump's first P[0]
        # with attacker not NULL, damagecount raised, a threshold on a body of player 0 or the player dead)
        quiet = [True]

        def plant(k, j, last, d):
            p = p_row(d)
            if k == level - 1 and last and prev_live: p['attacker'] = p['mo']    # a live thinker of that dump
            if k == level:
                quiet[0] = quiet[0] and p['attacker'] == -1 and p['damagecount'] == 0 and p['playerstate'] == PST_LIVE \
                    and all(r['threshold'] == 0 for x, r in d.thinkers if x == 'M' and r['player'] == 0)
                if j >= dangling_from and quiet[0]: p['attacker'] = p['mo'] if body else -2
                if extra: extra(j, d)
        return plant

    def refusal(r, item, plant):
        try:
            levels(r, item, plant=plant); return None
        except L.ProjectionError as e: return e

    try:
        got = levels(base, it, plant=planter()); fired = [v['load'] for v in got if v['stale_attacker']]
        same = [c[2] for c in got[K]['captures']] == [c[2] for c in lv[K]['captures']]
        out.append(('stale attacker at a carried load (all four conditions)',
                    f'fired on loads {fired} (want [{KL}]), level {K} captures equal to NULL\'s: {same}', fired == [KL] and same))
    except L.ProjectionError as e:
        out.append(('stale attacker at a carried load (all four conditions)', f'refused: {e.kind} at tic {e.tic}: {e.text}', False))

    def refused_case(what, r, item, plant, tic):
        e = refusal(r, item, plant)
        ok = e is not None and e.text == 'a dangling reference' and e.tic == tic
        out.append((what, 'not refused' if e is None else f'refused: {e.kind} at tic {e.tic}: {e.text} (want tic {tic})', ok))

    def hit(j, d):
        # a sourced hit on players[0].mo halved to 0 damage (skill 0, p_inter.c:799-800): damagecount stays 0 and
        # p_inter.c:904-911 sets the body's threshold
        if j == 10: next(r for x, r in d.thinkers if x == 'M' and r['idx'] == p_row(d)['mo'])['threshold'] = 100
    def write(j, d):
        if j == 10: p_row(d)['attacker'] = p_row(d)['mo']
    def other(j, d):
        if j == 0: d.sectors[0]['soundtarget'] = -2
    refused_case('dangling attacker appearing mid-level (level tic 10)', base, it, planter(dangling_from=10), 10)
    refused_case('(a) a dangling pointer that is not player 0\'s attacker (sector 0 soundtarget)', base, it,
                 planter(extra=other), 0)
    refused_case('(c) the previous level ended with a NULL attacker', base, it, planter(prev_live=False), 0)
    refused_case('(d) changed since the load: threshold 100 on players[0].mo at level tic 10, damagecount 0', base, it,
                 planter(extra=hit), 10)
    refused_case('(d) changed since the load: a live attacker at level tic 10, dangling again at 11', base, it,
                 planter(extra=write), 11)
    # (d) on one genuine dump directly (the carried load's, with attacker -2 and the previous level's end live), where
    # a projection would object to the change before the rule is reached: each change must stop the rule firing
    T = lv[K]['tic']

    def rule_case(what, change):
        d = L.Dump(str(base['dumps'] / f'loads/load_{T:05}.txt'))
        p_row(d)['attacker'] = -2
        held = untouched(d)
        s = StaleAttacker('carried', p_row(d)['mo'])
        change(d); s.apply(d)
        out.append((what, f'(d) held on the unchanged dump: {held}; fired after the change: {s.fired}',
                    held and not s.fired and p_row(d)['attacker'] == -2))

    def second_body(d):
        # a second player-1 start leaves the earlier body with player 0; a hit on it writes players[0].attacker
        # (p_inter.c:798, 874) and sets that body's threshold (904-911), not players[0].mo's
        r = next(r for x, r in d.thinkers if x == 'M' and r['player'] < 0)
        r['player'], r['threshold'] = 0, 100
    def dead(d): p_row(d)['playerstate'] = PST_LIVE + 1                     # PST_DEAD (d_player.h:56-58)
    def vile(d): next(r for x, r in d.thinkers if x == 'M' and r['player'] < 0)['type'] = MT_VILE
    rule_case('(d) threshold 100 on another body of player 0 (an earlier player-1 start)', second_body)
    rule_case('(d) player 0 dead (a killing hit returns before p_inter.c:904)', dead)
    rule_case('(d) an MT_VILE present (exempt from the threshold at p_inter.c:904-906)', vile)

    # the gate compares the field: CHAIN's carried load with P[0].attacker the player's own body (a live thinker, so
    # the rule must not fire) from the load to the first genuine write; the load capture must differ from the
    # committed D row, which the unchanged projection (and Rust) equals
    committed = {(f[1], int(f[2]), int(f[4])): f[6] for f in (l.split(',') for l in RESULTS.read_text().splitlines())
                 if f[0] == 'D'}
    want = committed[('CHAIN', KL, 1)]
    try:
        got = levels(base, it, plant=planter(body=True)); fired = [v['load'] for v in got if v['stale_attacker']]
        planted, plain = got[K]['captures'][0][2].hex(), lv[K]['captures'][0][2].hex()
        out.append(('genuine-side fault: attacker = the player\'s own body at CHAIN\'s carried load',
                    f'fired on {fired}; load capture {"differs from" if planted != want else "EQUALS"} the committed row; '
                    f'unchanged projection equals it: {plain == want}', not fired and planted != want and plain == want))
    except L.ProjectionError as e:
        out.append(('genuine-side fault: attacker = the player\'s own body at CHAIN\'s carried load',
                    f'refused: {e.kind} at tic {e.tic}: {e.text}', False))
    reb = items_by_name['REBHELD']
    r = run(binary, reb, work / 'self_rebheld')
    rl = levels(r, reb)
    R = next(i for i, v in enumerate(rl) if v['kind'] == 'reborn')
    refused_case('(b) not a carried load (REBHELD\'s reborn reload)', r, reb, planter(level=rl[R]['load']), 0)
    shutil.rmtree(work / 'self_rebheld', ignore_errors=True)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--only'); ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'doom-m6a-state')
    ap.add_argument('--jobs', type=int, default=2); ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    binary = build(work, a.cc)
    results = json.loads(G.RESULTS.read_text())
    items = {i['name']: i for i in G.items() if i['name'] in ITEMS}
    want = [n for n in ITEMS if not a.only or n in {x.upper() for x in a.only.split(',')}]
    per, fails, total = {}, [], [0, 0, 0]
    with cf.ThreadPoolExecutor(max_workers=max(1, min(a.jobs, 2))) as ex:
        for name, rows, f, caps, nlv, fired in ex.map(lambda n: run_item(binary, items[n], work, results), want):
            per[name] = rows; fails += f; total[0] += caps; total[1] += nlv; total[2] += fired
            print(f'{name}: {nlv} levels, {caps} captures, {sum(r.startswith("D,") for r in rows)} D rows, '
                  f'stale-attacker rule fired on {fired} loads' + ('' if not f else '; ' + '; '.join(f)))
    if a.selftest:
        for what, detail, ok in selftest(binary, items, work, results):
            print(f'selftest {"pass" if ok else "FAIL"}: {what}: {detail}')
            if not ok: fails.append(f'selftest: {what}')
    print(f'{len(want)} items, {total[1]} levels, {total[0]} captures projected; stale-attacker rule fired on '
          f'{total[2]} loads')
    if fails:
        print('\n'.join(['FAILURES:'] + fails)); return 1
    if a.only:
        print('--only: the results file is neither written nor compared'); return 0
    text = results_text(per)
    if a.check:
        if not RESULTS.exists() or RESULTS.read_text() != text:
            print(f'{RESULTS.relative_to(ROOT)} differs from the regenerated digests'); return 1
        print(f'{RESULTS.relative_to(ROOT)} matches ({total[1]} levels, {total[0]} captures)')
    else:
        RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
