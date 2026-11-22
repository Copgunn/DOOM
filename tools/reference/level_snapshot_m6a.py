#!/usr/bin/env python3
"""Independent DSIM42 projection (profile `episode1v4`): wire 26's start group from the genuine engine's rows.

DSIM42 is the canonical schema of wire 26 (`PlayerMonstersEpisode1V4`, M6a ruling U2 = B1;
docs/M6A_REFERENCE_INVESTIGATION.md section 4.3): wire 25's tic semantics unchanged plus a level start
that is not always a new game. It is the DSIM41 projection of level_snapshot_m5c.py, which this module
imports as a library and never edits, with exactly these differences:

- the version word is 42, `profile` is 26 and `context.schema` is `PLAYER-MONSTERS-EPISODE1-4-v1`;
- the start group after the last `events.sounds.*` field (`events.sounds.pending.count` in a demo run):

    level.start.kind          u8   0 new game, 1 carried, 2 reborn
    level.start.p_random      u8   prndindex when the load begins
    level.start.health        i32  player->health (also the new mobj's, p_mobj.c:673)
    level.start.armor_points  i32
    level.start.armor_type    u8
    level.start.weapons_owned u16  bit i = weaponowned[i]
    level.start.ready_weapon  u8   weapontype_t
    level.start.ammo.0..3     i32  ammotype_t order
    level.start.max_ammo.0..3 i32
    level.start.backpack      u8
    level.start.cheats        i32  always 0 (the simulation models no cheat)
    level.start.attack_down   u8
    level.start.use_down      u8
    level.start.bob           i32  raw 16.16
    level.start.weapon_x      i32  psprites[ps_weapon].sx
    level.start.flash_tics    i32  psprites[ps_flash].tics (state NULL)
    level.start.flash_x       i32  psprites[ps_flash].sx
    level.start.flash_y       i32  psprites[ps_flash].sy
    level.start.sounds.count  u64
    level.start.sounds.<i>         a DSIM41 sound record each

What the fields hold (linuxdoom-1.10):

- a new game (G_InitNew, g_game.c:1414 M_ClearRandom; P_SpawnPlayer -> G_PlayerReborn, g_game.c:815-831):
  index 0, health 100, no armour, fist and pistol owned, the pistol ready, 50 bullets, the maxammo table
  (p_inter.c:58), no backpack, both latches set, bob and the psprite coordinates 0 (the memset);
- a reborn player (G_DoReborn, g_game.c:924-932, or a player dead at the load, 477-480): the same player
  with the carried index (no M_ClearRandom outside G_InitNew);
- a carried player: what G_PlayerFinishLevel (779-792), G_DoLoadLevel (477-482), P_SetupLevel
  (p_setup.c:594-604), P_SpawnPlayer (p_mobj.c:642-700) and P_SetupPsprites (p_pspr.c:831-842) leave;
- the load's sound records: exactly one, `sfx_sawup` at the player's mobj without a placement
  (P_BringUpWeapon, p_pspr.c:145-146; an origin that is the listener is not placed, s_sound.c:302),
  when a carried ready weapon is the chainsaw, none otherwise (a fresh player's is the pistol).

From genuine rows (`start_from_rows`): the session stream (DOOM-M6A-SESSION-v1) gives each load's L row
(map, skill, prndindex_before) and the C row of the tic before the load (the last intermission tic,
whose latches WI_checkForAccelerate wrote, wi_stuff.c:1476-1497; a player dead or reborn there starts
reborn). The C rows do not carry bob or the psprite coordinates and flash count: they are written 0 and
listed as `unrecorded` in the results; a comparison with the genuine dumps of later levels supplies
them (not committed). The load's sawup is the leading S row of the load's G_Ticker with sfx_sawup and
okind player (no other S_StartSound call is reached by a load: P_SetupLevel's S_Start only stops
channels, and a weapon brought up at the load is still rising at tic 0, so no tic-0 call names
sfx_sawup); `--check` requires that row exactly when a carried ready weapon is the chainsaw. The
player's handle is (the number of things P_LoadThings spawns before the player's record, generation 1):
in lump order, stopping at the first Doom II type (p_setup.c:317-337), a type-11 record or a start
record 2-4 spawns nothing, a record with options & 16 or without the skill bit spawns nothing
(p_mobj.c:708-751), every other record spawns one mobj.

Dumps (`Projector`, for the later genuine comparison; not run here, no M6a dump set is committed): the
DSIM41 projection of each level's dumps with the header above and the start group appended. One
declared rule: the C's `secretexit` (g_game.c:999) is a session global, rewritten at every exit
(1004, 1014-1016) and read only between an exit and the next load (1071, 1087, 1151, 1160), so a level
after a secret exit holds the previous value; the simulation's `level.secret_exit` is this level's
request, so a dump whose gameaction is not ga_completed projects secretexit 0 (the frozen projection
would refuse it as 'secretexit without the exit request').

CLI:
    level_snapshot_m6a.py [--check | --write] [--selftest]
        the start group of every load of every committed M6a item (tests/reference/m6a_manifest.json,
        m6a_session, m6a_sound), compared with or written to tests/reference/m6a_dsim42_starts.txt
    level_snapshot_m6a.py DUMP_DIR --wad WAD --demo LMP --sound STREAM --session SESSION --load K [--out DIR]
        one level's dumps projected (local; nothing committed from an IWAD run)
"""
import argparse, hashlib, json, struct, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
import level_snapshot_m5c as M5C
from m3h9_geometry import enum_index

n = L.n
ProjectionError = L.ProjectionError
VERSION = 42
POLICY = 26
CONTEXT = b'PLAYER-MONSTERS-EPISODE1-4-v1'
EPISODE1V4 = L.Profile('episode1v4', VERSION, POLICY, CONTEXT, 1024, L.EPISODE1_MESSAGES, True)
START = 'level.start.'
RESULTS = ROOT / 'tests/reference/m6a_dsim42_starts.txt'
RESULTS_SCHEMA = 'DOOM-M6A-DSIM42-STARTS-v1'
MANIFEST = ROOT / 'tests/reference/m6a_manifest.json'
MAXAMMO = (200, 50, 300, 50)                                    # p_inter.c:58
MAXBOB = 0x100000                                               # p_local.h MAXBOB
MAX_FLASH_TICS = 7                                              # S_PISTOLFLASH, info.c:153
WEAPONS = (0, 1, 2, 3, 4, 7)                                    # the Episode 1 policy's weapons
WP_CHAINSAW = 7                                                 # doomdef.h weapontype_t
SFX_SAWUP = enum_index('sfxenum_t', 'sfx_sawup', 'sounds.h')
DOOM2_ONLY = (68, 64, 88, 89, 69, 67, 71, 65, 66, 84)           # p_setup.c:322-331
KINDS = {'new': 0, 'carried': 1, 'reborn': 2}
UNRECORDED = ('bob', 'weapon_x', 'flash_tics', 'flash_x', 'flash_y')


def fresh(p_random):
    """G_PlayerReborn's player (g_game.c:815-831) with `p_random`."""
    return dict(health=100, armor_points=0, armor_type=0, owned=3, ready=1, ammo=[50, 0, 0, 0],
                max_ammo=list(MAXAMMO), backpack=0, cheats=0, attack_down=1, use_down=1, bob=0, weapon_x=0,
                flash_tics=0, flash_x=0, flash_y=0, p_random=p_random)


def check_carried(p, where):
    """The C's bounds on a carried player (and the simulation's domain); ProjectionError otherwise."""
    def bad(text): raise ProjectionError('Unadmitted', where, f'a carried {text}')
    if not 1 <= p['health'] <= 200: bad('health')                      # alive; p_inter.c:385-386, 402-403
    if not 0 <= p['armor_points'] <= 200 or p['armor_type'] not in (0, 1, 2) \
            or (p['armor_type'] == 0) != (p['armor_points'] == 0): bad('armour')      # p_inter.c:258-263, 393-396, 861-867
    mx = [m << p['backpack'] for m in MAXAMMO]
    if p['backpack'] not in (0, 1) or p['max_ammo'] != mx: bad('maxammo')   # p_inter.c:589-595
    owned = p['owned']
    if owned & 3 != 3 or owned & ~sum(1 << w for w in WEAPONS) or p['ready'] not in WEAPONS \
            or not owned & (1 << p['ready']): bad('weapon set')               # g_game.c:826-827
    if any(not 0 <= a <= m for a, m in zip(p['ammo'], mx)): bad('ammunition')   # p_inter.c:106-107
    if p['cheats']: bad('cheat flag (not modelled)')
    if p['attack_down'] not in (0, 1) or p['use_down'] not in (0, 1): bad('latch')
    if not 0 <= p['bob'] <= MAXBOB: bad('bob')
    if not -15 * 65536 <= p['weapon_x'] <= 17 * 65536 or not -15 * 65536 <= p['flash_x'] <= 17 * 65536 \
            or not 0 <= p['flash_y'] <= 128 * 65536 or not 0 <= p['flash_tics'] <= MAX_FLASH_TICS: bad('psprite')
    if not 0 <= p['p_random'] <= 255: bad('P_Random index')


def sawup_record(handle):
    """The DSIM41 start record of P_BringUpWeapon's sfx_sawup at the player's mobj, unplaced."""
    return b'\1' + n('H', SFX_SAWUP) + b'\1' + struct.pack('<IQ', *handle) + b'\0'


def start_fields(kind, p, handle):
    """The DSIM42 start group of a load: `kind` in KINDS, `p` the player dict, `handle` the player's
    (slot, generation)."""
    if kind == 'new': assert p == fresh(0), 'a new game holds the fresh player and index 0'
    elif kind == 'reborn': assert p == fresh(p['p_random']), 'a reborn start holds the fresh player'
    else: check_carried(p, 'load')
    saw = kind == 'carried' and p['ready'] == WP_CHAINSAW
    i = lambda v: n('i', v)
    f = [(START + 'kind', n('B', KINDS[kind])), (START + 'p_random', n('B', p['p_random'])),
         (START + 'health', i(p['health'])), (START + 'armor_points', i(p['armor_points'])),
         (START + 'armor_type', n('B', p['armor_type'])), (START + 'weapons_owned', n('H', p['owned'])),
         (START + 'ready_weapon', n('B', p['ready']))]
    f += [(f'{START}ammo.{k}', i(v)) for k, v in enumerate(p['ammo'])]
    f += [(f'{START}max_ammo.{k}', i(v)) for k, v in enumerate(p['max_ammo'])]
    f += [(START + 'backpack', n('B', p['backpack'])), (START + 'cheats', i(p['cheats'])),
          (START + 'attack_down', n('B', p['attack_down'])), (START + 'use_down', n('B', p['use_down'])),
          (START + 'bob', i(p['bob'])), (START + 'weapon_x', i(p['weapon_x'])),
          (START + 'flash_tics', i(p['flash_tics'])), (START + 'flash_x', i(p['flash_x'])),
          (START + 'flash_y', i(p['flash_y'])), (START + 'sounds.count', n('Q', int(saw)))]
    if saw: f.append((START + 'sounds.0', sawup_record(handle)))
    return f


def framed(fields):
    """The fields in DSIM framing without the header (u32 path length, path, u64 value length, value)."""
    out = bytearray()
    for path, value in fields:
        out += n('I', len(path)) + path.encode() + n('Q', len(value)) + value
    return bytes(out)


def player_slot(things, skill):
    """The player's actor slot after P_LoadThings (see the module text): the mobjs spawned before the
    type-1 record. `things` is the THINGS lump, `skill` 0-4."""
    bit = 1 if skill <= 1 else 2 if skill == 2 else 4                      # p_mobj.c:747-751
    slot = 0
    for k in range(len(things) // 10):
        _x, _y, _angle, kind, options = struct.unpack_from('<hhhhh', things, 10 * k)
        if kind in DOOM2_ONLY: break                                       # p_setup.c:317-337
        if kind == 1: return slot
        if kind in (2, 3, 4, 11) or options & 16 or not options & bit: continue
        slot += 1
    raise ProjectionError('Unadmitted', 'load', 'no player start before the THINGS end')


def map_things(wad, map_number):
    """The THINGS lump of E1M<map_number> in `wad`."""
    data = Path(wad).read_bytes()
    count, offset = struct.unpack_from('<ii', data, 4)
    names = [struct.unpack_from('<8s', data, offset + 16 * k + 8)[0].split(b'\0')[0] for k in range(count)]
    at = names.index(f'E1M{map_number}'.encode())
    pos, size = struct.unpack_from('<ii', data, offset + 16 * (at + 1))
    assert names[at + 1] == b'THINGS'
    return data[pos:pos + size]


def rows(text, kind):
    """The rows of `kind` of a stream as name -> int (or str) maps, by its #columns line."""
    cols = None
    out = []
    for line in text.splitlines():
        f = line.split(',')
        if f[0] == '#columns' and f[1] == kind: cols = f[2:]
        elif f[0] == kind and cols and len(f) == len(cols) + 1:
            out.append({c: (int(v) if v.lstrip('-').isdigit() else v) for c, v in zip(cols, f[1:])})
    return out


def carried_from(c, p_random):
    """The carried player the C row of the tic before a load records (UNRECORDED fields 0)."""
    return dict(health=c['health'], armor_points=c['armorpoints'], armor_type=c['armortype'], owned=c['weaponowned'],
                ready=c['readyweapon'], ammo=[c[f'ammo{k}'] for k in range(4)],
                max_ammo=[c[f'maxammo{k}'] for k in range(4)], backpack=c['backpack'], cheats=c['cheats'],
                attack_down=c['attackdown'], use_down=c['usedown'], bob=0, weapon_x=0, flash_tics=0, flash_x=0,
                flash_y=0, p_random=p_random)


def start_from_rows(session, sound, load_index, wad):
    """(kind, player, handle, load sawup rows) of load `load_index` from the genuine session and sound
    streams, `wad` holding the maps."""
    loads = rows(session, 'L')
    l = loads[load_index]
    tic, p_random = l['tic'], l['prndindex_before']
    if load_index == 0:
        if p_random != 0: raise ProjectionError('Unadmitted', tic, 'a new game not at index 0')
        kind, p = 'new', fresh(0)
    else:
        c = next(r for r in rows(session, 'C') if r['tic'] == tic - 1)
        if c['playerstate'] == 0: kind, p = 'carried', carried_from(c, p_random)
        else: kind, p = 'reborn', fresh(p_random)
    handle = (player_slot(map_things(wad, l['map']), l['skill']), 1)
    # the leading S rows of the load's G_Ticker that are sfx_sawup at the player (seq order)
    at = sorted((r for r in rows(sound, 'S') if r['tic'] == tic), key=lambda r: r['seq'])
    saws = 0
    for r in at:
        if r['sfx'] == SFX_SAWUP and r['okind'] == 'player': saws += 1
        else: break
    return kind, p, handle, saws


def items():
    man = json.loads(MANIFEST.read_text())
    out = []
    for it in man['items']:
        session = ROOT / f'tests/reference/m6a_session/{it["name"]}.session'
        if session.exists():
            out.append((it['name'], ROOT / man['wadsets'][it['wadset']][0], session,
                        ROOT / f'tests/reference/m6a_sound/{it["name"]}.sound'))
    return out


def results():
    """The results text: one line per load of every committed M6a item."""
    lines = [f'#schema,{RESULTS_SCHEMA}',
             '#generator,tools/reference/level_snapshot_m6a.py (from the genuine session and sound goldens)',
             f'#unrecorded,{" ".join(UNRECORDED)} (not in the genuine C rows; written 0)',
             '#columns,item,load,tic,map,skill,kind,p_random,player_slot,load_sawups,start_group_hex']
    for name, wad, session_path, sound_path in items():
        session, sound = session_path.read_text(), sound_path.read_text()
        for k, l in enumerate(rows(session, 'L')):
            kind, p, handle, saws = start_from_rows(session, sound, k, wad)
            fields = start_fields(kind, p, handle)
            want = int(kind == 'carried' and p['ready'] == WP_CHAINSAW)
            if saws != want:
                raise ProjectionError('Unadmitted', l['tic'], f'{name}: {saws} load sawup rows, expected {want}')
            lines.append(f'{name},{k},{l["tic"]},{l["map"]},{l["skill"]},{kind},{p["p_random"]},{handle[0]},{saws},'
                         f'{framed(fields).hex()}')
    return '\n'.join(lines) + '\n'


class Projector(M5C.Projector):
    """The DSIM42 projector of one level's dumps: the DSIM41 projection with the DSIM42 header, the
    secretexit rule of the module text and the start group `start` = (kind, player) appended after the
    last `events.sounds` field; the player's handle comes from the projection's store."""
    def __init__(self, mapdata, skill, map_number, sounds, start):
        super().__init__(mapdata, skill, map_number, sounds)
        self.profile = EPISODE1V4
        self.start = start

    def step(self, dump, following=None):
        h = dump.h
        if h.get('secretexit') and h['gameaction'] != L.GA_COMPLETED: h['secretexit'] = 0
        fields, report = super().step(dump, following)
        player = next((e for e in self.entries if e.cls == 'actor' and e.row.get('player') == 0), None)
        if player is None: raise ProjectionError('DumpDeficiency', dump.tic, 'no player mobj')
        k = 1 + max(i for i, (p, _) in enumerate(fields) if p.startswith('events.sounds.'))
        kind, p = self.start
        return fields[:k] + start_fields(kind, p, self.actors.handles[player.serial]) + fields[k:], report


def compare41(f42, f41):
    """Raise AssertionError unless the DSIM42 fields are the DSIM41 fields plus the start group, with
    profile 26 and the wire-26 context."""
    rest = [(p, v) for p, v in f42 if not p.startswith(START) and p not in ('profile', 'context.schema')]
    assert rest == [(p, v) for p, v in f41 if p not in ('profile', 'context.schema')]
    d42, d41 = dict(f42), dict(f41)
    assert (d41['profile'], d42['profile']) == (n('I', 25), n('I', 26))
    assert (d41['context.schema'], d42['context.schema']) == (M5C.CONTEXT, CONTEXT)


def selftest():
    """The encoder's own refusals and the check's sensitivity."""
    checks = 0
    base = dict(fresh(9), health=150, armor_points=200, armor_type=1, owned=0b10001111, ready=7,
                ammo=[400, 7, 0, 3], max_ammo=[400, 100, 600, 100], backpack=1, attack_down=0, flash_tics=7)
    f = start_fields('carried', base, (3, 1))
    assert len(f) == 26 and f[-1] == (START + 'sounds.0', sawup_record((3, 1))); checks += 1
    assert len(start_fields('carried', dict(base, ready=1), (3, 1))) == 25; checks += 1
    for change in [dict(health=0), dict(health=201), dict(armor_type=0), dict(armor_points=201), dict(owned=0b10001101),
                   dict(ready=5), dict(ammo=[401, 7, 0, 3]), dict(max_ammo=list(MAXAMMO)), dict(cheats=1),
                   dict(bob=MAXBOB + 1), dict(flash_tics=8), dict(weapon_x=18 * 65536)]:
        try: start_fields('carried', dict(base, **change), (3, 1))
        except ProjectionError: checks += 1
        else: raise AssertionError(f'not refused: {change}')
    for kind, p in [('new', fresh(1)), ('reborn', dict(fresh(4), health=99))]:
        try: start_fields(kind, p, (0, 1))
        except AssertionError: checks += 1
        else: raise AssertionError(f'not refused: {kind}')
    things = b''.join(struct.pack('<hhhhh', 0, 0, 0, t, o) for t, o in [(11, 7), (2, 7), (3001, 16 | 7), (3001, 4),
                                                                         (3001, 2), (2014, 7), (1, 7), (3001, 7)])
    assert player_slot(things, 2) == 2 and player_slot(things, 3) == 2 and player_slot(things, 1) == 1; checks += 1
    text = results()
    lines = text.splitlines()
    k = next(i for i, l in enumerate(lines) if l.startswith('CHAIN,2,'))
    bent = lines[:k] + [lines[k][:-1] + ('0' if lines[k][-1] != '0' else '1')] + lines[k + 1:]
    assert '\n'.join(bent) + '\n' != text; checks += 1
    print(f'selftest passed: {checks} checks')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('dump', nargs='?')
    ap.add_argument('--check', action='store_true'); ap.add_argument('--write', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--wad'); ap.add_argument('--demo'); ap.add_argument('--sound'); ap.add_argument('--session')
    ap.add_argument('--load', type=int); ap.add_argument('--out')
    a = ap.parse_args()
    if a.dump:
        skill, _ = M5C.M.demo_header(a.demo)
        session, sound = Path(a.session).read_text(), Path(a.sound).read_text()
        l = rows(session, 'L')[a.load]
        kind, p, _h, _s = start_from_rows(session, sound, a.load, a.wad)
        mapdata = L.MapData(a.wad, f'E1M{l["map"]}', L.demo_lumps(a.demo))
        proj = Projector(mapdata, skill, l['map'], M5C.SoundStream(text=sound), (kind, p))
        count, seq = 0, hashlib.sha256()
        try:
            for dump, following in M5C.snapshots(a.dump):
                fields, _ = proj.step(dump, following)
                data = L.encode(fields, VERSION); count += 1; seq.update(hashlib.sha256(data).digest())
                if a.out: Path(a.out, f'tic_{dump.tic:05}.bin').write_bytes(data)
        except ProjectionError as err:
            print(f'projection stopped: {err.kind} at tic {err.tic}: {err.text}; projected {count}'); return 1
        print(f'projected {count} snapshots; sequence sha256 {seq.hexdigest()}')
        return 0
    if a.selftest: selftest()
    text = results()
    loads = sum(1 for l in text.splitlines() if not l.startswith('#'))
    if a.write:
        RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)} ({loads} loads)')
    elif a.check:
        if not RESULTS.exists() or RESULTS.read_text() != text:
            print(f'{RESULTS.relative_to(ROOT)} differs from the regenerated start groups'); return 1
        print(f'{RESULTS.relative_to(ROOT)} matches ({loads} loads)')
    return 0


if __name__ == '__main__':
    sys.exit(main())
