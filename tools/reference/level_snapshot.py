#!/usr/bin/env python3
"""Independent DSIM38 projection of the M3h9c whole-engine runner's per-tic dumps.

The genuine-engine runner (tools/reference/m3h9c_driver.c, read by m3h9c_dump.py) writes
the complete gameplay state after the load (`load.txt`) and after every `G_Ticker`
(`tic_NNNNN.txt`). This module projects one such state to the bytes of a DSIM38 capture
(M3h9c note §5 and decisions G4/G5; docs/STATE_FORMAT.md "DSIM v38"): the DSIM37 layout
with version 38, profile 22 (`PlayerMonstersLevelV1`) and context
`PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1`, every field at its DSIM37 path, width and position.
No Rust output is read. The frozen encoder chain is reused by import and never edited:
`movers_snapshot.Store` (the lowest-free-slot rule with generation + 1 on unlink) for the
actor store and the sector-thinker store, `movers_snapshot.store_fields`/`button_fields`
and `specials_snapshot.sector_thinker_fields` for those records (with their assertions of
the implied mover values), `weapon_snapshot.weapon_fields` for the weapon block,
`m3f3_snapshot_goldens.map_fields` for `map.*` and `world.conservative_omissions` (fed a
PWAD of the ten lumps of the map slot read from the WAD itself), `specials_snapshot.decode`
as the field reader, and the frozen `n`/identity byte encoders. The synthetic harness rows
are not reconstructed: the facts come from the dump instead.

Field sources (dump row kind and column):

- `tic` = H.leveltime (the load dump is tic 0, `tic_NNNNN` is tic N + 1, as Rust's
  `tic_{N+1}.bin`); `rng.gameplay` = H.prndindex, `rng.miscellaneous` = H.rndindex;
  `level.total_*` = H totals; `level.single_player_kill_count` = P[0].killcount;
  `level.exit` = H.gameaction == ga_completed (6).
- `actors.options` = (H.gameskill, 0, 0).
- Actors: the M rows in thinker order (x, y, z, momentum, radius, height, floorz/ceilingz,
  angle, lastlook, flags, type, state (-1 -> 65535), tics, health, reactiontime, sprite,
  frame, target, threshold, movedir, movecount, player, subsector); `sector` is the
  subsector's sector (P_GroupLines: the sector of the subsector's first seg's sidedef,
  computed from the lumps; checked against M.sector whenever the actor is sector-linked);
  `spawnpoint` is present iff M.sp_type != 0 (P_SpawnMobj zeroes the record; P_SpawnPlayer
  sets none), `death_requested` and `retired` are 0.
- Sector thinkers: LL/LS/LG/LF (kinds 1-4) and DR/PL/FL (kinds 5-7) rows; a removed mover
  (R row of a mover identity) is kind 8 with the sector of its last record. CE rows,
  stasis plats and X rows are outside the layout and raise `Unadmitted`. Mover fields the
  genuine code has not written yet are projected as the schema's 0 (DSIM35 decision N6;
  no statement reads them before the first write). The driver's declared ZeroFill now
  zero-fills every zone block, so C reads 0 there too; for dumps of an earlier driver,
  whose genuine Z_Malloc left zone bytes there, the rule still applies: a
  door's `topcountdown` until it first waits (T_VerticalDoor sets it at the top), a
  plat's `count` until it first waits (T_PlatRaise, status 2) and its `oldstatus`
  (P_StopPlat only), a floor's `newspecial` and `texture` (change types only).
- Sectors: S rows (runtime floor/ceiling, light, special, soundtarget); `specialdata` is
  checked against the store (the live mover naming the sector) but, as in DSIM35, not
  encoded. `sector.{i}.actors` is the projected link order filtered by sector and checked
  equal to the dump's sector thing list (S thinglist order via M.sector/M.spos).
- `line.{i}.special` for every line whose LINEDEFS special is nonzero: L rows.
- Buttons: B rows; `soundorg` (not dumped) is the line's front sector from the lumps,
  which is what P_StartButton stores (the frozen `button_fields` asserts the same).
- `player.{i}`: `actor` is the 13 zero bytes of the frozen layout; the command is P.cmd_*.
- `original_player.0`: P[0] mo, health, playerstate, attacker, damagecount, frag0,
  weapons (readyweapon, pendingweapon, owned bitmask of wo*, ammo0, ammo1, attackdown,
  refire, bob, extralight, both psprites), bonuscount, itemcount, armortype/armorpoints,
  secretcount, usedown; `pickups.message` is the last message P_TouchSpecialThing
  assigned: P.message (the driver's HU_Ticker capture, 1 + the d_englsh.h index of its
  table, mapped by DRIVER_MESSAGES to the pickup-message codes of the m3g9..m3g11
  harness) when nonzero, else the previous value (HU_Ticker's clearing is presentation).
- `events.last` / `events.pending`: not projected (decision G5); both are encoded with
  count 0 and the comparator strips `events.*` from both sides.
- `map.*`: from the WAD's ten lumps; the comparator strips `map.*` for hashing (G5).

Identity, removals and link order (exact; the driver's linker-level observers):

- Each thinker's identity is its T.serial (P_AddThinker order). At load, actor slots
  follow the serials (lump order, so the body sits at its THING record), the light
  thinkers take sector-thinker slots 0..L-1 in creation order. Afterwards the frozen rule
  of the birth-identity simulation applies (`movers_snapshot.Store`): a new actor or
  mover takes the lowest free slot of its store, an unlinked one frees its slot with
  generation + 1. The K rows name every thinker freed since the previous dump (K.seq is
  their order), including one spawned and freed between two dumps, which also takes and
  frees a slot. Where a free stands among the spawns of the tic is given by K.adds (the
  add-serial counter at the free): a free reading `a` follows every spawn of serial < a.
  (Trying every interleaving and refusing ambiguity is only the `--infer` fallback.)
- An R row is a thinker marked by P_RemoveThinker and not yet freed: kind 8 for a mover;
  for a mobj the full record (pending_removal 1) from its RR row (the same dump), checked
  equal to the next dump's RM row (the Z_Free wrapper's copy of the same memory, which
  nothing writes between the mark and the free) except the link positions.
- `spatial.link_order` lists the linked actors by descending T.linkserial (the last
  P_SetThingPosition first; P_RemoveMobj unlinks, so pending actors are not listed) and is
  checked against every sector thing list and BLOCKMAP cell list of the dump.
- A dangling reference (-2) or a level reload (K.seq -1) is refused. There is no
  tolerance: every field of the capture comes from the dump and the lumps.

A dump without the observer rows is refused by default. `--infer` (never used by the
comparator) keeps the earlier fallback for such dumps: identities from the
append-and-unlink invariants of the thinker list, pending mobj records rebuilt from the
previous dump and the genuine P_MobjThinker steps for the S_NULL and pickup causes, the
message from the type of the item taken (checked against the bonus count), and the link
order enumerated among the orders the sector and cell lists admit (`link_candidates`;
the first is encoded). It is not exact and is documented only as that fallback.

The module is a library (`Projector`) plus a small CLI:
    level_snapshot.py DUMP_DIR --wad WAD [--map E1M1] [--out DIR] [--infer [--demo LMP]]
writes `tic_NNNNN.bin` (DSIM38, full bytes) per snapshot into --out (scratch only for an
IWAD-derived dump) and prints counts and digests."""
import argparse, copy, hashlib, itertools, os, struct, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
for p in (str(HERE), str(ROOT / 'tools')):
    if p not in sys.path: sys.path.insert(0, p)
import barrels_snapshot as frozen          # the DSIM37 projection (identity constants only)
import movers_snapshot, specials_snapshot, weapon_snapshot
import m3c_snapshot_goldens as v4          # the C-derived states table
import m3h9c_dump
import iwad_counts
from m3h9_geometry import enum_index

base = frozen.base                          # m3f3_snapshot_goldens: n, map_fields
n = base.n
VERSION = 38
POLICY = 22
CONTEXT = b'PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1'
REGISTRY = b'DOOM-INFO-1.10-v1'
MAX_SLOTS = frozen.MAX_SLOTS               # 384, the DSIM37 store
GA_COMPLETED = movers_snapshot.GA_COMPLETED
MF_SPECIAL, MF_SHOOTABLE, MF_NOSECTOR, MF_NOBLOCKMAP = 1, 4, 8, 16
MF_NOGRAVITY, MF_FLOAT, MF_MISSILE, MF_SKULLFLY = 0x200, 0x4000, 0x10000, 0x1000000
GRAVITY = 65536
BONUSADD = 6
THINKER_KINDS = ('M', 'R', 'DR', 'PL', 'FL', 'CE', 'LF', 'LL', 'LS', 'LG', 'X')
LIGHTS = {'LL': 1, 'LS': 2, 'LG': 3, 'LF': 4}
MOVERS = {'DR': 5, 'PL': 6, 'FL': 7}
STATES = v4.metadata(0)[0]                  # sprite, frame, tics, action, next per state
ACTIONS = v4.source.ACTIONS                 # 'None' then the sorted info.c action names
PLAYER_HITSCAN = {ACTIONS.index(a) for a in ('A_Punch', 'A_FirePistol', 'A_FireShotgun', 'A_FireCGun', 'A_Saw')}
MONSTER_HITSCAN = {ACTIONS.index(a) for a in ('A_PosAttack', 'A_SPosAttack', 'A_CPosAttack')}
HITSCAN_SPAWNS = {enum_index('mobjtype_t', t) for t in ('MT_PUFF', 'MT_BLOOD')}
MF_DROPPED = 0x20000
assert len(STATES) == 967 and STATES[0]['name'] == 'S_NULL'
# P_TouchSpecialThing (p_inter.c) message per admitted pickup type; indices are the
# pickup_message() codes of the m3g9/m3g10/m3g11 harness (0 = no message).
MESSAGES = {enum_index('mobjtype_t', t): m for t, m in [
    ('MT_CLIP', 1), ('MT_MISC17', 2), ('MT_MISC22', 3), ('MT_MISC23', 4), ('MT_MISC10', 5),
    ('MT_MISC11', 6), ('MT_SHOTGUN', 8), ('MT_CHAINGUN', 9), ('MT_MISC0', 10), ('MT_MISC1', 11),
    ('MT_MISC2', 12), ('MT_MISC3', 13)]}


# The driver's P.message: 1 + the index of the d_englsh.h string in its PICKUP_MESSAGES
# table (GOTARMOR .. GOTSHOTGUN2); mapped to the DSIM codes above.
DRIVER_MESSAGES = {1: 10, 2: 11, 3: 12, 4: 13, 5: 5, 6: 7, 7: 6, 22: 1, 23: 2, 28: 3, 29: 4, 32: 9, 36: 8}


class ProjectionError(Exception):
    """The dump cannot be projected; `kind` is Ambiguous, DumpDeficiency or Unadmitted."""
    def __init__(self, kind, tic, text):
        super().__init__(f'{kind} at tic {tic}: {text}'); self.kind, self.tic, self.text = kind, tic, text


def demo_skill(path):
    data = Path(path).read_bytes()
    assert data[0] == 110, 'a version-110 demo (decision G1)'
    return data[1]


class MapData:
    """The ten lumps of one map slot of a WAD, the frozen `map_fields` of them, and the
    derived subsector sectors, line front sectors and line specials."""
    def __init__(self, wad, slot):
        data, directory = iwad_counts.read_wad(wad)
        lumps = iwad_counts.map_lumps(data, directory, slot)
        names = ['THINGS', 'LINEDEFS', 'SIDEDEFS', 'VERTEXES', 'SEGS', 'SSECTORS', 'NODES', 'SECTORS',
                 'REJECT', 'BLOCKMAP']
        self.lumps = [lumps[k] for k in names]
        self.digests = [hashlib.sha256(b).hexdigest() for b in self.lumps]
        pwad = self._pwad([b''] + self.lumps)
        with tempfile.TemporaryDirectory(prefix='doom-level-projection-') as d:
            (Path(d) / 'tests/reference').mkdir(parents=True)
            (Path(d) / 'tests/reference/m3f3_map_0.bin').write_bytes(pwad)
            old = base.ROOT
            try:
                base.ROOT = Path(d); base.map_fields.cache_clear() if hasattr(base.map_fields, 'cache_clear') else None
                self.fields, sectors, self.omissions = base.map_fields(0)
            finally:
                base.ROOT = old
                if hasattr(base.map_fields, 'cache_clear'): base.map_fields.cache_clear()
        assert sectors == self.lumps[7]
        lines = list(struct.iter_unpack('<7H', self.lumps[1]))
        sides = list(struct.iter_unpack('<hh8s8s8sH', self.lumps[2]))
        segs = list(struct.iter_unpack('<HHHHHh', self.lumps[4]))
        self.sector_count = len(self.lumps[7]) // 26
        self.special = [struct.unpack_from('<h', self.lumps[1], 14 * i + 6)[0] for i in range(len(lines))]
        self.front = [sides[l[5]][5] for l in lines]
        self.subsector_sector = []
        for count, first in struct.iter_unpack('<HH', self.lumps[5]):
            _, _, _, line, side, _ = segs[first]
            self.subsector_sector.append(sides[lines[line][5 + side]][5])

    @staticmethod
    def _pwad(lumps):
        names = [b'MAP01', b'THINGS', b'LINEDEFS', b'SIDEDEFS', b'VERTEXES', b'SEGS', b'SSECTORS', b'NODES',
                 b'SECTORS', b'REJECT', b'BLOCKMAP']
        body = b''.join(lumps); offset = 12; directory = b''
        for name, lump in zip(names, lumps):
            directory += struct.pack('<ii8s', offset, len(lump), name); offset += len(lump)
        return b'PWAD' + struct.pack('<ii', len(lumps), 12 + len(body)) + body + directory


class Dump:
    """One dump file: H, the thinker rows in list order, S, L (by line), B, P."""
    def __init__(self, path):
        columns, rows = m3h9c_dump.parse(path)
        rec = lambda r: dict(zip(columns[r[0]], map(int, r[1:])))
        self.h = rec(next(r for r in rows if r[0] == 'H'))
        self.thinkers = [(r[0], rec(r)) for r in rows if r[0] in THINKER_KINDS]
        assert [t['idx'] for _, t in self.thinkers] == list(range(len(self.thinkers))) == list(range(self.h['numthinkers']))
        self.sectors = [rec(r) for r in rows if r[0] == 'S']
        self.lines = {x['idx']: x for x in (rec(r) for r in rows if r[0] == 'L')}
        self.buttons = [rec(r) for r in rows if r[0] == 'B']
        self.players = [rec(r) for r in rows if r[0] == 'P']
        self.tic = self.h['leveltime']
        # observer rows (m3h9c_driver.c linker-level observers); absent in older dumps
        self.observed = all(k in columns for k in ('T', 'K', 'RM', 'RR')) and 'gameskill' in columns['H'] \
            and 'message' in columns['P'] and 'adds' in columns['K']
        self.serials = {x['idx']: (x['serial'], x['linkserial']) for x in (rec(r) for r in rows if r[0] == 'T')}
        self.freed = sorted(((x['seq'], x['serial']) for x in (rec(r) for r in rows if r[0] == 'K')))
        # optional K.adds: the add-serial counter at the free (the frees' place among the spawns)
        self.adds = {x['serial']: x['adds'] for x in (rec(r) for r in rows if r[0] == 'K')} \
            if 'K' in columns and 'adds' in columns['K'] else None
        self.removed = {x['serial']: x for x in (rec(r) for r in rows if r[0] == 'RM')}
        # RR: the fields of each pending mobj (an R row), by its list position
        self.pending = {x['idx']: x for x in (rec(r) for r in rows if r[0] == 'RR')}


class Entry:
    """One thinker identity: `cls` actor/light/mover, the store `key`, the current row
    kind and record, pending flag."""
    __slots__ = ('cls', 'key', 'kind', 'row', 'pending', 'inferred', 'sector', 'serial', 'waited')
    def __init__(self, cls, key, kind, row):
        self.cls, self.key, self.kind, self.row, self.pending, self.inferred = cls, key, kind, row, False, False
        self.serial = None; self.waited = False
        self.sector = row.get('sector')


def ident(handle):
    return struct.pack('<IQ', *handle)


class Projector:
    """Feed the dumps of one run in order (load first); `step` returns (DSIM38 bytes,
    report) for each."""
    def __init__(self, mapdata, skill=2, infer=False):
        self.map, self.skill, self.infer = mapdata, skill, infer
        self.entries = []; self.actors = None; self.sthinkers = None; self.next_key = 0
        self.spatial = []; self.message = 0; self.prev = None; self.link_candidates = []

    # ---- identity -------------------------------------------------------------------
    def _new(self, cls, kind, row):
        self.next_key += 1
        return Entry(cls, self.next_key, kind, row)

    def handle(self, e):
        if e.cls == 'actor': return self.actors.handles[e.key]
        if e.cls == 'light': return (e.key, 1)
        return self.sthinkers.handles[e.key]

    @staticmethod
    def _compatible(e, kind, row):
        if kind == 'R': return e.cls in ('actor', 'mover')
        if kind == 'M':
            if e.cls != 'actor': return False
            p = e.row
            if any(p[k] != row[k] for k in ('type', 'sp_x', 'sp_y', 'sp_angle', 'sp_type', 'sp_options', 'player')):
                return False
            if not p['flags'] & MF_SHOOTABLE and not (p['momx'] or p['momy'] or row['momx'] or row['momy']):
                return (p['x'], p['y']) == (row['x'], row['y'])
            return True
        if kind in LIGHTS or kind in MOVERS:
            return e.kind == kind and e.row['sector'] == row['sector']
        return False

    def _align(self, tic, cur):
        """Survivors of the previous list as the longest embeddable prefix of `cur`;
        returns (matches: list of previous Entry per current index < k, k)."""
        cand = [e for e in self.entries if not e.pending]
        j = 0; k = 0
        for i, (kind, row) in enumerate(cur):
            while j < len(cand) and not self._compatible(cand[j], kind, row): j += 1
            if j == len(cand): break
            j += 1; k = i + 1
        # count embeddings of cur[:k] into cand (capped), and record them when few
        sols = []
        def search(i, j, acc):
            if len(sols) > 64: return
            if i == k: sols.append(list(acc)); return
            for jj in range(j, len(cand) - (k - i) + 1):
                if self._compatible(cand[jj], *cur[i]):
                    acc.append(jj); search(i + 1, jj + 1, acc); acc.pop()
                    if len(sols) > 64: return
        # fast path: if every earliest match is also the only possible one, skip search
        search(0, 0, [])
        if len(sols) != 1:
            raise ProjectionError('Ambiguous', tic, f'{len(sols)} thinker alignments of {k} survivors')
        return [cand[j] for j in sols[0]], k

    # ---- stores ---------------------------------------------------------------------
    @staticmethod
    def _merge(store, frees, spawns, adds):
        """The exact order from K.adds: a free whose counter reads `a` follows every spawn
        of serial < a and precedes every later one."""
        events = [(e.key, 0, e) for e in spawns] + [(adds[e.key], 1, e) for e in frees]
        for _, kind, e in sorted(events, key=lambda x: (x[0], 1 - x[1])):   # stable: frees in seq order
            if kind: store.free(e.key)
            else: store.create(e.key)
        return store

    def _allocate(self, tic, store, frees, spawns, hidden=frozenset()):
        """Apply frees (in pass order) and spawns (in append order) to `store` for every
        interleaving; all must agree. A `hidden` key (spawned and freed between two
        dumps) is freed only after its spawn. Returns the resulting store."""
        if not frees or not spawns:
            for e in frees: store.free(e.key)
            for e in spawns: store.create(e.key)
            return store
        total = len(frees) + len(spawns)
        combos = 1
        for i in range(len(spawns)): combos = combos * (total - i) // (i + 1)
        if combos > 20000: raise ProjectionError('Ambiguous', tic, f'{combos} free/spawn interleavings')
        result = None
        for positions in itertools.combinations(range(total), len(spawns)):
            s = copy.deepcopy(store); fi = si = 0; spawnset = set(positions); valid = True
            for t in range(total):
                if t in spawnset: s.create(spawns[si].key); si += 1
                else:
                    if frees[fi].key not in s.handles: valid = False; break
                    s.free(frees[fi].key); fi += 1
            if not valid: continue
            outcome = (s.generations, sorted(s.occupied), [s.handles[e.key] for e in spawns if e.key not in hidden])
            if result is None: result, chosen = outcome, s
            elif outcome != result:
                raise ProjectionError('Ambiguous', tic, 'slot assignment depends on the free/spawn interleaving')
        return chosen

    # ---- pending-removal records ----------------------------------------------------
    def _state_step(self, tic, r):
        """`tics--` and P_SetMobjState through zero-tic states (p_mobj.c:63-95, 405-457):
        returns True when the actor reached S_NULL."""
        if r['tics'] == -1: return False
        r['tics'] -= 1
        if r['tics']: return False
        state = STATES[r['state']]['next']
        while True:
            if state == 0:
                r['state'] = -1; return True
            st = STATES[state]
            if st['action']: raise ProjectionError('DumpDeficiency', tic, 'a removed mobj entered an action state')
            r['state'], r['tics'], r['sprite'], r['frame'] = state, st['tics'], st['sprite'], st['frame']
            if r['tics']: return False
            state = st['next']

    def _pending_record(self, tic, e, player_index, index):
        p = dict(e.row); r = dict(p)
        if p['momx'] or p['momy'] or p['flags'] & MF_SKULLFLY:
            raise ProjectionError('DumpDeficiency', tic, 'removed mobj with x/y momentum (no R record)')
        if (r['z'] != r['floorz']) or r['momz']:
            if p['player'] >= 0 or p['flags'] & (MF_FLOAT | MF_MISSILE):
                raise ProjectionError('DumpDeficiency', tic, 'removed mobj with an unprojected z movement')
            r['z'] += r['momz']                                   # p_mobj.c:237
            if r['z'] <= r['floorz']:
                if r['momz'] < 0: r['momz'] = 0
                r['z'] = r['floorz']
            elif not r['flags'] & MF_NOGRAVITY:
                r['momz'] = -GRAVITY * 2 if r['momz'] == 0 else r['momz'] - GRAVITY
            if r['z'] + r['height'] > r['ceilingz']:
                if r['momz'] > 0: r['momz'] = 0
                r['z'] = r['ceilingz'] - r['height']
        moved = dict(r)
        if self._state_step(tic, r): return r, 'end'
        if p['flags'] & MF_SPECIAL and player_index is not None and index < player_index:
            if (moved['z'], moved['momz']) != (p['z'], p['momz']):
                raise ProjectionError('DumpDeficiency', tic, 'removed pickup moving in z')
            return r, 'pickup'
        raise ProjectionError('DumpDeficiency', tic, 'removed mobj of an unprojected cause (no R record)')

    # ---- link order -----------------------------------------------------------------
    def _links_consistent(self, order, rows):
        pos = {id(e): i for i, e in enumerate(order)}
        sectors, cells = {}, {}
        for e, r in rows:
            if r['sector'] >= 0:
                if r['spos'] < 0: return False
                sectors.setdefault(r['sector'], []).append((r['spos'], e))
            if r['bx'] >= 0:
                if r['bpos'] < 0: return False
                cells.setdefault((r['bx'], r['by']), []).append((r['bpos'], e))
        for group in list(sectors.values()) + list(cells.values()):
            group.sort(key=lambda x: x[0])
            if [x[0] for x in group] != list(range(len(group))): return False
            ranks = [pos.get(id(e)) for _, e in group]
            if None in ranks or ranks != sorted(ranks): return False
        return True

    def _link_order(self, tic, live, moved, maybe, unknown):
        """Every link order the dump admits, fewest in-place relinks first.
        live: [(entry,row)] of non-pending actors in list order; moved: [(time, entry)];
        maybe: [(time, entry)] that may have relinked in place; unknown: entries of
        unknown relink time (in append order)."""
        relinked = {id(e) for _, e in moved} | {id(e) for e in unknown}
        rest = [e for e in self.spatial if id(e) not in relinked]
        results = {}
        for size in range(len(maybe) + 1):
            for extra in itertools.combinations(maybe, size):
                known = sorted(moved + list(extra), key=lambda x: x[0], reverse=True)
                ex = {id(e) for _, e in extra}
                base_rest = [e for e in rest if id(e) not in ex]
                gaps = len(known) + 1
                if gaps ** len(unknown) > 200000:
                    raise ProjectionError('Ambiguous', tic, 'too many link-order candidates')
                for choice in itertools.product(range(gaps), repeat=len(unknown)):
                    front = []
                    for g in range(gaps):
                        # gap g sits after the g most recent known relinks; later spawns first
                        front += [u for u, c in reversed(list(zip(unknown, choice))) if c == g]
                        if g < len(known): front.append(known[g][1])
                    order = front + base_rest
                    key = tuple(id(e) for e in order)
                    if key not in results and self._links_consistent(order, live):
                        results[key] = order
        return list(results.values())

    @staticmethod
    def _thrust(dump, r):
        """A player body may have momentum inside its thinker although it has none at
        either tic end: P_MovePlayer thrusts it from this tic's command in the players
        phase (p_user.c), and P_SlideMove/friction can leave 0 again."""
        if r['player'] < 0: return False
        cmd = dump.players[r['player']]
        return bool(cmd['cmd_forward'] or cmd['cmd_side'])

    @staticmethod
    def _shooters(dump, entries):
        """Relink times of the hitscans of this tic (the --infer cause rule): the player's
        when psprite 0 entered a hitscan action state this tic (P_MovePsprites runs in the
        players phase, before every thinker: time (-1, 0)), and each monster's that
        entered a hitscan attack state in its own thinker (time (index, 0), before its own
        move). A state entered this tic still has its full `tics`."""
        out = []
        p = dump.players[0]; st = p['ps0state']
        if st >= 0 and STATES[st]['action'] in PLAYER_HITSCAN and p['ps0tics'] == STATES[st]['tics']:
            out.append((-1, 0))
        for i, e in enumerate(entries):
            r = e.row
            if e.cls == 'actor' and not e.pending and r['state'] >= 0 and STATES[r['state']]['action'] in MONSTER_HITSCAN \
                    and r['tics'] == STATES[r['state']]['tics']:
                out.append((i, 0))
        return out

    # ---- exact projection from the observer rows -----------------------------------
    def _step_exact(self, dump, following):
        tic = dump.tic; cur = dump.thinkers; report = {'inferred_records': 0, 'pickups': 0}
        if not dump.observed:
            raise ProjectionError('DumpDeficiency', tic, 'the dump lacks the observer rows (T, K with adds, RM, RR, '
                                  'H.gameskill, P.message); only --infer projects such dumps')
        if sorted(dump.serials) != list(range(len(cur))) or any(sv < 0 for sv, _ in dump.serials.values()):
            raise ProjectionError('DumpDeficiency', tic, 'a thinker without its observer serial')
        if any(seq < 0 for seq, _ in dump.freed) and self.prev is not None:
            raise ProjectionError('Unadmitted', tic, 'a level reload (Z_FreeTags frees)')
        self.skill = dump.h['gameskill']
        serial = [dump.serials[i][0] for i in range(len(cur))]
        if self.prev is None:
            assert tic == 0
            lights = sum(k in LIGHTS for k, _ in cur)
            self.actors = movers_snapshot.Store(0); self.sthinkers = movers_snapshot.Store(lights)
            slot = 0
            for (kind, row), sv in zip(cur, serial):
                if kind == 'M':
                    e = Entry('actor', sv, kind, row); self.actors.create(sv)
                elif kind in LIGHTS:
                    e = Entry('light', slot, kind, row); slot += 1
                else:
                    raise ProjectionError('Unadmitted', tic, f'{kind} thinker at load')
                e.serial = sv; self.entries.append(e)
        else:
            known = {e.serial: e for e in self.entries}
            freed = [sv for seq, sv in dump.freed]
            current = set(serial)
            if current & set(freed): raise ProjectionError('DumpDeficiency', tic, 'a freed serial is still listed')
            gone = [e for e in self.entries if e.serial not in current]
            if sorted(e.serial for e in gone) != sorted(sv for sv in freed if sv in known):
                raise ProjectionError('DumpDeficiency', tic, 'the K rows are not the thinkers that left the list')
            if any(e.pending and e.serial in current for e in self.entries):
                raise ProjectionError('DumpDeficiency', tic, 'a pending thinker survived a pass')
            survivors = [sv for sv in serial if sv in known]
            if survivors != [e.serial for e in self.entries if e.serial in current]:
                raise ProjectionError('DumpDeficiency', tic, 'the survivors changed order')
            fresh = [i for i, sv in enumerate(serial) if sv not in known]
            if fresh != list(range(len(cur) - len(fresh), len(cur))) or [serial[i] for i in fresh] != sorted(serial[i] for i in fresh):
                raise ProjectionError('DumpDeficiency', tic, 'new thinkers are not appended in serial order')
            # spawns of this tic in serial order, including the ones freed before this dump
            hidden = [sv for sv in freed if sv not in known]
            spawns = sorted([serial[i] for i in fresh] + hidden)
            if any(min(spawns) < sv for sv in known) if spawns else False:
                raise ProjectionError('DumpDeficiency', tic, 'a new serial below a known one')
            new = {}
            for i in fresh:
                kind, row = cur[i]
                if kind == 'R': continue
                if kind in LIGHTS: raise ProjectionError('Unadmitted', tic, 'a light thinker created after load')
                if kind not in ('M',) + tuple(MOVERS): raise ProjectionError('Unadmitted', tic, f'{kind} thinker')
                if kind == 'M' and row['sp_type']: raise ProjectionError('DumpDeficiency', tic, 'new mobj with a spawnpoint')
            classes = {}
            for sv in spawns:
                if sv in dump.removed or (sv in current and cur[serial.index(sv)][0] == 'M'):
                    classes[sv] = 'actor'
                elif sv in current and cur[serial.index(sv)][0] in MOVERS:
                    classes[sv] = 'mover'
                elif sv in current and serial.index(sv) in dump.pending:
                    classes[sv] = 'actor'           # spawned and marked this tic: its RR row
                else:
                    raise ProjectionError('DumpDeficiency', tic, 'the class of a new thinker is unknown')
            for e in gone:
                if e.cls == 'light': raise ProjectionError('Unadmitted', tic, 'a light thinker removed')
                if e.cls == 'actor' and e.serial not in dump.removed:
                    raise ProjectionError('DumpDeficiency', tic, 'a freed mobj without its RM row')
            order = {sv: k for k, (seq, sv) in enumerate(dump.freed)}
            for store_name, cls in (('actors', 'actor'), ('sthinkers', 'mover')):
                fr = sorted([e for e in gone if e.cls == cls] + [Entry(cls, sv, '', {}) for sv in hidden if classes[sv] == cls],
                            key=lambda e: order[e.key if e.cls != 'light' else e.serial])
                sp = [Entry(cls, sv, '', {}) for sv in spawns if classes[sv] == cls]
                setattr(self, store_name, self._merge(getattr(self, store_name), fr, sp, dump.adds))
            entries = []
            for (kind, row), sv in zip(cur, serial):
                e = known.get(sv)
                if e is None:
                    e = Entry(classes[sv], sv, kind, row); e.serial = sv
                if kind == 'R':
                    if not e.pending:
                        e.pending = True
                        if e.cls == 'actor':
                            i = serial.index(sv)
                            if i not in dump.pending:
                                raise ProjectionError('DumpDeficiency', tic, 'a pending mobj without its RR row')
                            e.row = dict(dump.pending[i])
                            if following is not None:
                                # the next dump's RM row is the Z_Free copy of the same memory
                                rm = following.removed.get(sv)
                                skip = ('tic', 'idx', 'serial', 'bx', 'by', 'bpos', 'sector', 'spos')
                                if rm is None or any(rm[k] != e.row[k] for k in rm if k not in skip):
                                    raise ProjectionError('DumpDeficiency', tic, 'the RR record differs from the next RM row')
                        else:
                            e.kind = 'removed'
                else:
                    e.kind, e.row = kind, row
                    if e.cls == 'mover': e.sector = row['sector']
                entries.append(e)
            self.entries = entries
        # pickups.message: the last message P_TouchSpecialThing assigned (HU_Ticker's
        # clearing is presentation, never read by p_*.c or g_game.c)
        m = dump.players[0]['message']
        if m < 0: raise ProjectionError('DumpDeficiency', tic, 'an unknown pickup message')
        if m:
            if m not in DRIVER_MESSAGES: raise ProjectionError('Unadmitted', tic, 'a pickup message outside the DSIM table')
            self.message = DRIVER_MESSAGES[m]; report['pickups'] = 1
        # link order: ascending link serial is P_SetThingPosition order; most recent first
        live = [(e, dump.serials[i][1]) for i, e in enumerate(self.entries) if e.cls == 'actor' and not e.pending]
        if any(ls < 0 for _, ls in live): raise ProjectionError('DumpDeficiency', tic, 'a linked actor without a link serial')
        self.spatial = [e for e, _ in sorted(live, key=lambda x: x[1], reverse=True)]
        self.link_candidates = [self.spatial]
        if not self._links_consistent(self.spatial, [(e, e.row) for e, _ in live]):
            raise ProjectionError('DumpDeficiency', tic, 'the link serials contradict the sector and cell lists')
        self.prev = dump
        return self._fields(dump), report

    # ---- one snapshot ---------------------------------------------------------------
    def step(self, dump, following=None):
        """Project `dump`; `following` is the next dump of the run (its RM rows hold the
        records of this dump's R mobjs), None at the last one."""
        if not self.infer:
            return self._step_exact(dump, following)
        tic = dump.tic; cur = dump.thinkers; report = {'inferred_records': 0, 'pickups': 0}
        player_index = next((i for i, (k, r) in enumerate(cur) if k == 'M' and r['player'] == 0), None)
        if self.prev is None:
            assert tic == 0
            lights = sum(k in LIGHTS for k, _ in cur)
            self.actors = movers_snapshot.Store(0); self.sthinkers = movers_snapshot.Store(lights)
            slot = 0
            for kind, row in cur:
                if kind == 'M':
                    e = self._new('actor', kind, row); self.actors.create(e.key)
                elif kind in LIGHTS:
                    e = Entry('light', slot, kind, row); slot += 1
                else:
                    raise ProjectionError('Unadmitted', tic, f'{kind} thinker at load')
                self.entries.append(e)
            assert slot == lights
            self.spatial = [e for e in reversed(self.entries) if e.cls == 'actor']
            live = [(e, e.row) for e in self.entries if e.cls == 'actor']
            self.link_candidates = [self.spatial]
            if not self._links_consistent(self.spatial, live):
                raise ProjectionError('DumpDeficiency', tic, 'load link order does not fit the sector and cell lists')
        else:
            prev_list = list(self.entries); prev_pos = {id(e): i for i, e in enumerate(prev_list)}
            prev_player = self.prev.players[0]
            matches, k = self._align(tic, cur)
            survivors = {id(e) for e in matches}
            freed = [e for e in prev_list if id(e) not in survivors]
            for kind, _ in cur[k:]:
                if kind == 'R': raise ProjectionError('DumpDeficiency', tic, 'a thinker spawned and removed within one tic')
                if kind in LIGHTS: raise ProjectionError('Unadmitted', tic, 'a light thinker created after load')
                if kind not in ('M',) + tuple(MOVERS): raise ProjectionError('Unadmitted', tic, f'{kind} thinker')
            new = []
            for kind, row in cur[k:]:
                if kind == 'M' and row['sp_type']: raise ProjectionError('DumpDeficiency', tic, 'new mobj with a spawnpoint')
                new.append(self._new('actor' if kind == 'M' else 'mover', kind, row))
            # pickups: MF_SPECIAL mobjs freed without having been marked in the previous dump
            picked = [e for e in freed if e.cls == 'actor' and not e.pending and e.row['flags'] & MF_SPECIAL]
            if any(e.cls == 'mover' and not e.pending for e in freed):
                raise ProjectionError('DumpDeficiency', tic, 'a mover freed without its pending record')
            if any(e.cls == 'actor' and not e.pending and not e.row['flags'] & MF_SPECIAL for e in freed):
                raise ProjectionError('DumpDeficiency', tic, 'a non-pickup mobj freed without its pending record')
            self.actors = self._allocate(tic, self.actors, [e for e in freed if e.cls == 'actor'],
                                         [e for e in new if e.cls == 'actor'])
            self.sthinkers = self._allocate(tic, self.sthinkers, [e for e in freed if e.cls == 'mover'],
                                            [e for e in new if e.cls == 'mover'])
            entries = []
            for (kind, row), e in zip(cur, matches + new):
                old_row = e.row
                if kind == 'R':
                    if not e.pending:
                        e.pending = True
                        if e.cls == 'actor':
                            e.row, cause = self._pending_record(tic, e, player_index, prev_pos[id(e)])
                            e.inferred = True
                            if cause == 'pickup': picked.append(e)
                        else:
                            e.kind = 'removed'
                else:
                    if e.cls == 'actor' and e.row is not row:
                        row['_prev'] = old_row
                    e.kind, e.row = kind, row
                entries.append(e)
            self.entries = entries
            report['inferred_records'] = sum(e.inferred for e in entries if e.pending)
            # pickup message (--infer)
            p = dump.players[0]
            decayed = prev_player['bonuscount'] - 1 if prev_player['bonuscount'] else 0
            count, rem = divmod(p['bonuscount'] - decayed, BONUSADD)
            if rem or count != len(picked):
                raise ProjectionError('DumpDeficiency', tic, f'{len(picked)} visible pickups, bonus count says {count if not rem else "?"}')
            if picked:
                msgs = set()
                for e in picked:
                    if e.row['type'] not in MESSAGES: raise ProjectionError('Unadmitted', tic, 'pickup type outside the message table')
                    msgs.add(MESSAGES[e.row['type']])
                if len(msgs) > 1: raise ProjectionError('Ambiguous', tic, 'several pickup messages in one tic')
                self.message = msgs.pop(); report['pickups'] = len(picked)
            # link order (--infer)
            live = [(e, e.row) for e in entries if e.cls == 'actor' and not e.pending]
            moved, maybe, unknown, caused = [], [], [], []
            newset = {id(e) for e in new}
            shooters = self._shooters(dump, entries)
            for i, (e, r) in enumerate((e, e.row) for e in entries):
                if e.cls != 'actor' or e.pending: continue
                if id(e) in newset:
                    if r['momx'] or r['momy']: moved.append(((i, 1), e))
                    elif len(shooters) == 1 and (r['type'] in HITSCAN_SPAWNS or r['flags'] & MF_DROPPED):
                        caused.append((shooters[0], e))
                    else: unknown.append(e)
                else:
                    q = r.get('_prev', r)
                    if (q['x'], q['y']) != (r['x'], r['y']): moved.append(((i, 1), e))
                    elif q['momx'] or q['momy'] or r['momx'] or r['momy'] or self._thrust(dump, r):
                        maybe.append(((i, 1), e))
            pending_ids = {id(e) for e in entries if e.pending} | {id(e) for e in freed}
            self.spatial = [e for e in self.spatial if id(e) not in pending_ids]
            if caused:
                found = self._link_order(tic, live, moved + caused, maybe, unknown)
                candidates = found or self._link_order(tic, live, moved, maybe, unknown + [e for _, e in caused])
            else:
                candidates = self._link_order(tic, live, moved, maybe, unknown)
            if not candidates:
                raise ProjectionError('DumpDeficiency', tic, 'no link order fits the sector and cell lists')
            if len(candidates) > 64:
                raise ProjectionError('Ambiguous', tic, f'{len(candidates)} link orders fit the sector and cell lists')
            self.spatial = candidates[0]; self.link_candidates = candidates
            report['link_candidates'] = len(candidates)
            for e in entries:
                if e.cls == 'actor' and not e.pending: e.row.pop('_prev', None)
        self.prev = dump
        return self._fields(dump), report

    # ---- fields ---------------------------------------------------------------------
    def _ref(self, tic, index, entries):
        """A thinker-order index of this dump (-1 NULL) as an optional actor ID."""
        if index == -1: return bytes(13)
        if index == -2: raise ProjectionError('DumpDeficiency', tic, 'a dangling reference')
        e = entries[index]
        if e.cls != 'actor': raise ProjectionError('DumpDeficiency', tic, 'a reference to a sector thinker')
        return b'\1' + ident(self.handle(e))

    def _fields(self, d):
        tic = d.tic; h = d.h; entries = self.entries; p0 = d.players[0]
        actors = [e for e in entries if e.cls == 'actor']
        aorder = [self.handle(e) for e in actors]
        opt = lambda hd: bytes(13) if hd is None else b'\1' + ident(hd)
        f = [('tic', n('Q', tic)), ('tic_rate', n('I', 35)), ('profile', n('I', POLICY)),
             ('registry.schema', REGISTRY), ('context.schema', CONTEXT), ('paused', b'\0'),
             ('rng.gameplay', n('B', h['prndindex'])), ('rng.miscellaneous', n('B', h['rndindex'])),
             ('actors.options', bytes([self.skill, 0, 0])),
             ('level.total_kills', n('i', h['totalkills'])), ('level.total_items', n('i', h['totalitems'])),
             ('level.total_secrets', n('i', h['totalsecret'])), ('level.single_player_kill_count', n('i', p0['killcount'])),
             ('level.exit', n('B', h['gameaction'] == GA_COMPLETED)),
             ('store.max_slots', n('I', MAX_SLOTS)), ('store.slot_count', n('Q', len(self.actors.generations))),
             ('store.head', opt(aorder[0] if aorder else None)), ('store.tail', opt(aorder[-1] if aorder else None)),
             ('thinker.order', b''.join(ident(x) for x in aorder)),
             ('thinker.shared_order', b''.join((b'\0' if e.cls == 'actor' else b'\1') + ident(self.handle(e)) for e in entries))]
        if len(self.actors.generations) > MAX_SLOTS: raise ProjectionError('Unadmitted', tic, 'more than 384 actor slots')
        by_slot = {self.handle(e)[0]: (i, e) for i, e in enumerate(actors)}
        for slot, generation in enumerate(self.actors.generations):
            p = f'actor.{slot}'
            f += [(p + '.generation', n('Q', generation)), (p + '.retired', b'\0'), (p + '.occupied', n('B', slot in by_slot))]
            if slot not in by_slot: continue
            i, e = by_slot[slot]; r = e.row
            assert self.handle(e)[1] == generation
            f += [(p + '.pending_removal', n('B', e.pending)), (p + '.previous', opt(aorder[i - 1] if i else None)),
                  (p + '.next', opt(aorder[i + 1] if i + 1 < len(aorder) else None))]
            for key, col in [('x', 'x'), ('y', 'y'), ('z', 'z'), ('momx', 'momx'), ('momy', 'momy'), ('radius', 'radius'),
                             ('height', 'height'), ('floor', 'floorz'), ('ceiling', 'ceilingz')]:
                f.append((f'{p}.{key}', n('i', r[col])))
            sector = self.map.subsector_sector[r['subsector']]
            if r['sector'] >= 0 and r['sector'] != sector and not e.pending:
                raise ProjectionError('DumpDeficiency', tic, 'actor sector differs from its subsector sector')
            sp = b'\1' + struct.pack('<hhhhH', r['sp_x'], r['sp_y'], r['sp_angle'], r['sp_type'], r['sp_options'] & 0xffff) if r['sp_type'] else b'\0'
            f += [(p + '.angle', n('I', r['angle'])), (p + '.lastlook', n('B', r['lastlook'])), (p + '.flags', n('I', r['flags'] & 0xffffffff)),
                  (p + '.type', n('H', r['type'])), (p + '.state', n('H', 65535 if r['state'] < 0 else r['state'])),
                  (p + '.tics', n('i', r['tics'])), (p + '.health', n('i', r['health'])), (p + '.reaction_time', n('i', r['reactiontime'])),
                  (p + '.sprite', n('H', r['sprite'])), (p + '.frame', n('I', r['frame'] & 0xffffffff)),
                  (p + '.target', self._target(tic, e)), (p + '.death_requested', b'\0'), (p + '.spawnpoint', sp),
                  (p + '.threshold', n('i', r['threshold'])), (p + '.movedir', n('B', r['movedir'])), (p + '.movecount', n('i', r['movecount'])),
                  (p + '.player', n('B', 255 if r['player'] < 0 else r['player'])), (p + '.momz', n('i', r['momz'])),
                  (p + '.subsector', n('H', r['subsector'])), (p + '.sector', n('H', sector))]
        # sector-thinker store (frozen record encoders)
        tags = {i: s['tag'] for i, s in enumerate(d.sectors)}
        records, handles, sector_of, live = [], {}, {}, {}
        for e in entries:
            r = e.row
            if e.cls == 'light':
                k = LIGHTS[e.kind]
                if e.kind == 'LL': rec = (1, e.key, r['sector'], r['count'], r['minlight'], r['maxlight'], r['maxtime'], r['mintime'])
                elif e.kind == 'LS': rec = (2, e.key, r['sector'], r['count'], r['minlight'], r['maxlight'], r['darktime'], r['brighttime'])
                elif e.kind == 'LG': rec = (3, e.key, r['sector'], -1, r['minlight'], r['maxlight'], r['direction'], 0)
                else: rec = (4, e.key, r['sector'], r['count'], r['minlight'], r['maxlight'], 0, 0)
            elif e.cls == 'mover':
                handles[0, e.key] = self.sthinkers.handles[e.key]; sector_of[0, e.key] = e.sector
                if e.kind == 'removed': rec = (8, e.key)
                elif e.kind == 'DR':
                    # Unwritten zone memory (Z_Malloc does not zero; the M3h8 harness's
                    # calloc did, decision N6): EV_VerticalDoor/EV_DoDoor never set
                    # topcountdown for the admitted types; T_VerticalDoor writes it when the
                    # door reaches the top (direction 0), before any read. Until then the
                    # schema value is 0.
                    e.waited = e.waited or r['direction'] == 0
                    rec = (5, e.key, r['sector'], r['type'], r['direction'], r['topheight'],
                           r['topcountdown'] if e.waited else 0, r['speed'], r['topwait'])
                elif e.kind == 'PL':
                    if not r['running']: raise ProjectionError('Unadmitted', tic, 'a plat in stasis')
                    # oldstatus is written only by P_StopPlat (stasis, not admitted) and count
                    # only by T_PlatRaise when the plat starts waiting (status 2, count = wait;
                    # EV_DoPlat's downWaitUpStay sets neither): unwritten zone memory until
                    # then, never read; the schema value is 0 (N6)
                    e.waited = e.waited or r['status'] == 2
                    rec = (6, e.key, r['sector'], r['type'], r['status'], r['count'] if e.waited else 0, r['low'], r['high'], r['speed'], r['wait'],
                           r['crush'], r['tag'], 0)
                else:
                    # newspecial/texture are written only for the change types (not admitted):
                    # unwritten zone memory for lowerFloorToLowest/turboLower, never read; 0 (N6)
                    rec = (7, e.key, r['sector'], r['type'], r['direction'], r['floordestheight'], r['speed'], r['crush'],
                           0, 0)
                if e.kind != 'removed':
                    if r['sector'] in live: raise ProjectionError('Unadmitted', tic, 'two live movers in one sector')
                    live[r['sector']] = e
            else:
                continue
            records.append(rec)
        try:
            store = movers_snapshot.store_fields(records, self.sthinkers.generations, handles, 0, sector_of, tags)
        except AssertionError as err:
            raise ProjectionError('Unadmitted', tic, f'sector-thinker record outside the DSIM35 domain ({err})')
        f += store
        f.append(('spatial.link_order', b''.join(ident(self.handle(e)) for e in self.spatial)))
        f += [('world.blockmap_policy', b'\1'), ('world.conservative_omissions', n('I', self.map.omissions)),
              ('sectors.count', n('I', len(d.sectors)))]
        assert len(d.sectors) == self.map.sector_count
        index_of = {id(e): i for i, e in enumerate(entries)}
        for i, s in enumerate(d.sectors):
            members = [e for e in self.spatial if self.map.subsector_sector[e.row['subsector']] == i and not e.row['flags'] & MF_NOSECTOR]
            listed = sorted((e.row['spos'], id(e)) for e in actors if not e.pending and e.row['sector'] == i)
            if [x for _, x in listed] != [id(e) for e in members]:
                raise ProjectionError('DumpDeficiency', tic, f'sector thing list order differs from the link order')
            sd = s['specialdata']
            if sd != (index_of[id(live[i])] if i in live else -1):
                raise ProjectionError('DumpDeficiency', tic, 'sector specialdata is not its live mover')
            f += [(f'sector.{i}.floor', n('i', s['floorheight'])), (f'sector.{i}.ceiling', n('i', s['ceilingheight'])),
                  (f'sector.{i}.actors', b''.join(ident(self.handle(e)) for e in members)),
                  (f'sector.{i}.soundtarget', self._ref(tic, s['soundtarget'], entries)),
                  (f'sector.{i}.light', n('h', s['lightlevel'])), (f'sector.{i}.special', n('h', s['special']))]
        selected = [i for i, s in enumerate(self.map.special) if s]
        if sorted(d.lines) != selected: raise ProjectionError('DumpDeficiency', tic, 'L rows are not the special lines of the map')
        f += [(f'line.{i}.special', n('h', d.lines[i]['special'])) for i in selected]
        mb = [(b['idx'], b['line'], b['where'], b['btexture'], b['btimer'], self.map.front[b['line']]) for b in d.buttons if b['btimer']]
        if any(b['btimer'] == 0 and b['line'] != -1 for b in d.buttons):
            raise ProjectionError('DumpDeficiency', tic, 'a stopped button with a line')
        f += movers_snapshot.button_fields(mb, self.map.lumps[1], self.map.lumps[2])
        for i, pl in enumerate(d.players):
            f += [(f'player.{i}.actor', bytes(13)), (f'player.{i}.forward', n('b', pl['cmd_forward'])),
                  (f'player.{i}.side', n('b', pl['cmd_side'])), (f'player.{i}.angle_turn', n('h', pl['cmd_turn'])),
                  (f'player.{i}.buttons', n('B', pl['cmd_buttons']))]
        owned = sum(1 << w for w in range(9) if p0[f'wo{w}'])
        weapons = [p0['readyweapon'], p0['pendingweapon'], owned, p0['ammo0'], p0['ammo1'], p0['attackdown'], p0['refire'],
                   p0['bob'], p0['extralight']] + [p0[f'ps{j}{c}'] for j in (0, 1) for c in ('state', 'tics', 'sx', 'sy')]
        try:
            wf = weapon_snapshot.weapon_fields(weapons)
        except AssertionError:
            raise ProjectionError('Unadmitted', tic, 'weapon state outside the DSIM37 domain')
        f += [('original_player.0.mo', self._ref(tic, p0['mo'], entries)), ('original_player.0.health', n('i', p0['health'])),
              ('original_player.0.state', n('B', p0['playerstate'])),
              ('original_player.0.attacker', self._ref(tic, p0['attacker'], entries)),
              ('original_player.0.damage_count', n('i', p0['damagecount'])), ('original_player.0.frags', n('i', p0['frag0']))]
        f += wf + [('original_player.0.pickups.present', b'\1'), ('original_player.0.pickups.bonus_count', n('i', p0['bonuscount'])),
                   ('original_player.0.pickups.item_count', n('i', p0['itemcount'])),
                   ('original_player.0.pickups.message', n('B', self.message)),
                   ('original_player.0.armor.type', n('B', p0['armortype'])), ('original_player.0.armor.points', n('i', p0['armorpoints'])),
                   ('original_player.0.secret_count', n('i', p0['secretcount'])), ('original_player.0.use_down', n('B', p0['usedown'])),
                   ('events.last.count', n('Q', 0)), ('events.pending.count', n('Q', 0))]
        f += self.map.fields
        return f

    def _target(self, tic, e):
        return self._ref(tic, e.row['target'], self.entries)


def link_bytes(proj, order):
    return b''.join(ident(proj.handle(e)) for e in order)


def encode(fields):
    """The DSIM byte string of `fields` under version 38 (the frozen framing)."""
    data = bytearray(b'DSIM' + struct.pack('<IQ', VERSION, len(fields)))
    for path, value in fields:
        path = path.encode(); data += n('I', len(path)) + path + n('Q', len(value)) + value
    return bytes(data)


decode = specials_snapshot.decode


def snapshots(directory):
    """(dump, following dump or None) pairs of a run, in order."""
    names = m3h9c_dump.snapshot_names(directory)
    following = Dump(os.path.join(directory, names[0])) if names else None
    for k in range(len(names)):
        dump = following
        following = Dump(os.path.join(directory, names[k + 1])) if k + 1 < len(names) else None
        yield dump, following


def project(directory, mapdata, skill=2, infer=False):
    """Yield (dsim tic, DSIM38 bytes, report) for every snapshot of a dump directory, in
    order; a ProjectionError propagates to the caller at the first unprojectable tic.
    The default is exact (observer rows required); `infer` selects the fallback."""
    proj = Projector(mapdata, skill, infer)
    for dump, following in snapshots(directory):
        fields, report = proj.step(dump, following)
        yield dump.tic, encode(fields), report


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('dump'); ap.add_argument('--wad', required=True); ap.add_argument('--map', default='E1M1')
    ap.add_argument('--demo'); ap.add_argument('--out')
    ap.add_argument('--infer', action='store_true', help='fallback for dumps without observer rows (never used by the comparator)')
    a = ap.parse_args()
    mapdata = MapData(a.wad, a.map); skill = demo_skill(a.demo) if a.demo else 2
    if a.out: os.makedirs(a.out, exist_ok=True)
    count = 0; seq = hashlib.sha256(); last = None
    try:
        for tic, data, _ in project(a.dump, mapdata, skill, a.infer):
            count += 1; seq.update(hashlib.sha256(data).digest()); last = data
            if a.out: Path(a.out, f'tic_{tic:05}.bin').write_bytes(data)
    except ProjectionError as err:
        print(f'projection stopped: {err.kind} at tic {err.tic}: {err.text}')
        print(f'projected {count} snapshots'); return 1
    print(f'projected {count} snapshots; sequence sha256 {seq.hexdigest()}; final fnv1a64 {fnv(last):016x}')
    return 0


def fnv(data):
    h = 0xcbf29ce484222325
    for b in data: h = ((h ^ b) * 0x100000001b3) & 0xffffffffffffffff
    return h


if __name__ == '__main__':
    sys.exit(main())
