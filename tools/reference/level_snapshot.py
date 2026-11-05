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
- `actors.options` = (skill, 0, 0): the dump has no skill column; the skill comes from the
  demo header when `demo=` is given (byte 1 of a version-110 `.lmp`), else 2 (medium, the
  only skill G1 admits).
- Actors: the M rows in thinker order (x, y, z, momentum, radius, height, floorz/ceilingz,
  angle, lastlook, flags, type, state (-1 -> 65535), tics, health, reactiontime, sprite,
  frame, target, threshold, movedir, movecount, player, subsector); `sector` is the
  subsector's sector (P_GroupLines: the sector of the subsector's first seg's sidedef,
  computed from the lumps; checked against M.sector whenever the actor is sector-linked);
  `spawnpoint` is present iff M.sp_type != 0 (P_SpawnMobj zeroes the record; P_SpawnPlayer
  sets none), `death_requested` and `retired` are 0.
- Sector thinkers: LL/LS/LG/LF (kinds 1-4) and DR/PL/FL (kinds 5-7) rows; a removed mover
  (R row of a mover identity) is kind 8 with the sector of its last record. CE rows,
  stasis plats and X rows are outside the layout and raise `Unadmitted`.
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
  secretcount, usedown; `pickups.message` see (5).
- `events.last` / `events.pending`: not projected (decision G5); both are encoded with
  count 0 and the comparator strips `events.*` from both sides.
- `map.*`: from the WAD's ten lumps; the comparator strips `map.*` for hashing (G5).

Problems the dump format leaves open, and how they are solved here:

(1) Slot assignment and identity. A capture names actors by store slot and generation.
    The committed driver writes no per-thinker serial (M3h9c note §3.3 planned one via
    `--wrap=Z_Malloc`; it is not in m3h9c_driver.c), so identity is derived from the
    thinker-list invariants of the genuine code: `P_AddThinker` appends (p_tick.c:65-71),
    `P_RunThinkers` unlinks a marked (`function == -1`) thinker when it reaches it
    (p_tick.c:105-121), so between two dumps the list is the previous list minus the
    freed thinkers, in order, plus the new thinkers appended. Every R thinker of the
    previous dump is freed in the next pass. A previous thinker matches a current row
    when kind and invariants agree (mobj: type, spawnpoint record and player; a
    non-shootable mobj that had and has no momentum also keeps x and y; sector thinker:
    kind and sector; an R row matches any removable thinker). The longest prefix of the
    current list that embeds into the previous list is the survivors (greedy earliest
    match is optimal), the rest are new; an embedding that is not unique raises
    `Ambiguous`. A new R row (spawned and removed within one tic) raises
    `DumpDeficiency`. Slots: at load in spawn order (lump order, the body at its record,
    then the light thinkers in creation order); afterwards a new actor takes the lowest
    free slot and an unlinked one frees its slot with generation + 1, the rule of the
    frozen birth-identity simulation (`movers_snapshot.Store`, m3d3j `E` code 1/3). The
    frees of one tic happen in previous-list order, the spawns in append order, but their
    interleaving is not in the dump: every interleaving is enumerated and the projection
    raises `Ambiguous` unless all give the same handles and store.
    Residual risk without a serial log: a thinker added and freed within one tic between
    two dumps is invisible (it would consume and free a slot); the bonus-count check of
    (5) detects the one E1M1 instance (a drop picked up in its spawn tic).
(2) Pending removal. An R row is a thinker marked by P_RemoveThinker and not yet freed.
    For a mover it is the kind-8 record. For a mobj the frozen layout keeps the full
    record (pending_removal 1, its values at removal); the R row carries no values, so the
    record is reconstructed from the previous dump and the genuine P_MobjThinker steps:
    (a) the S_NULL end of a state sequence in its own thinker: `P_ZMovement` for a
    non-player, non-float, non-skullfly, non-missile mobj without x/y momentum
    (p_mobj.c:217-340), then `tics--` and P_SetMobjState through zero-tic action-free
    states to S_NULL (state none, tics 0; p_mobj.c:63-95, 405-457); (b) a pickup
    (MF_SPECIAL) touched by the player: a pickup shown as R lies before the body in the
    list (a later one is freed in the same pass), so its own thinker ran before the
    touch: the state step of (a), without movement. Any
    other case raises `DumpDeficiency`. These records are marked in `inferred`.
(3) Fields the dump lacks: per-thinker serial, the R records, the global link order, the
    skill, `player->message`, the button `soundorg` (derived, exact), the actor `sector`
    of a sector-unlinked mobj (derived, exact). Fields Rust lacks and the dump has are
    simply not projected: tracer, viewheight, deltaviewheight, powers, cards, backpack,
    frags 1-3, maxammo, cheats, killcount (projected only as the level counter), colormap,
    didsecret, the sector textures/tags/soundtraversed, line flags and side textures,
    button `where`/`btexture`, door `speed`/`topwait` and the other implied mover values
    (asserted by the frozen record encoder instead).
(4) The player under wire 22: the body is the mobj with M.player == 0 at its THING
    position in spawn order; its angle is whatever C has; `damage_count` is P.damagecount
    (it decays in C, and must in Rust under wire 22).
(5) `pickups.message`: the dump has no message column. It is derived from the pickups
    of each tic: every MF_SPECIAL mobj that disappears (freed or marked R without an
    S_NULL end) is one P_TouchSpecialThing; its type gives the message of p_inter.c
    (the medikit's `health < 25` test runs after P_GiveBody, so GOTMEDINEED is
    unreachable; table MESSAGES); several pickups of one tic must agree, and their count
    must equal the bonus-count increments ((bonuscount - decayed previous) / BONUSADD),
    otherwise `DumpDeficiency`.
(6) `spatial.link_order` (the global P_SetThingPosition recency order, most recent
    first; P_RemoveMobj takes the actor out) is not in the dump, only its projections on
    each sector thing list and each BLOCKMAP cell. At load it is the reverse spawn order.
    Per tic: survivors whose x/y changed relinked in their own thinker (pass position);
    a new actor with momentum relinked in its own thinker after spawning; a puff, blood
    or dropped item without momentum relinked when it was spawned, which the cause rule
    fixes when exactly one hitscan ran this tic (the player's psprite or a monster that
    entered a A_PosAttack/A_SPosAttack/A_CPosAttack state this tic; see `_shooters`);
    any other new actor without momentum (or a caused one whose fixed time fits no list)
    relinked at an unknown time, so every insertion gap is enumerated; a survivor with
    momentum at either tic end, or a player body that this tic's command thrusts, but
    unchanged x/y may or may not have relinked in place (a player against a
    wall: P_SlideMove's stairstep `P_TryMove (mo, mo->x, mo->y + mo->momy)` succeeds at
    the same position when `momy` is 0, p_map.c), so both are enumerated. Every
    candidate is checked against every sector list and cell list of the dump. No
    surviving order is `DumpDeficiency`; otherwise the projector keeps every surviving
    order in `link_candidates` (fewest in-place relinks first) and encodes the first.
    The dump does not decide between several: the comparator (level_compare.py) accepts
    the other side's order only when it is one of these C-admissible candidates, counts
    those tics separately and continues from that candidate (`--strict-links` stops
    there instead). This is a dump limitation, not an equivalence proof of that field.

Minimal driver addition that removes every inference above (proposed, not applied):
`--wrap=P_AddThinker`, `--wrap=Z_Free` and `--wrap=P_SetThingPosition` observers keeping
pointer-keyed side tables (one global event counter: the add serial; a per-mobj link
serial), a `T,tic,idx,serial,linkserial` row per thinker, a `K,tic,serial,seq` row per
freed thinker since the previous dump, an `RM` row (M columns) for an R thinker that was
a mobj, `gameskill` in H, and the m3g9/m3g10/m3g11 `pickup_message (p->message)` index in
P. A dangling reference (-2, a pointer to a freed thinker) is not resolved
(`DumpDeficiency`); a T row with serials would identify it.

The module is a library (`Projector`) plus a small CLI:
    level_snapshot.py DUMP_DIR --wad WAD [--map E1M1] [--demo LMP] [--out DIR]
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


class Entry:
    """One thinker identity: `cls` actor/light/mover, the store `key`, the current row
    kind and record, pending flag."""
    __slots__ = ('cls', 'key', 'kind', 'row', 'pending', 'inferred', 'sector')
    def __init__(self, cls, key, kind, row):
        self.cls, self.key, self.kind, self.row, self.pending, self.inferred = cls, key, kind, row, False, False
        self.sector = row.get('sector')


def ident(handle):
    return struct.pack('<IQ', *handle)


class Projector:
    """Feed the dumps of one run in order (load first); `step` returns (DSIM38 bytes,
    report) for each."""
    def __init__(self, mapdata, skill=2):
        self.map, self.skill = mapdata, skill
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
    def _allocate(self, tic, store, frees, spawns):
        """Apply frees (in pass order) and spawns (in append order) to `store` for every
        interleaving; all must agree. Returns the resulting store."""
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
            s = copy.deepcopy(store); fi = si = 0; spawnset = set(positions)
            for t in range(total):
                if t in spawnset: s.create(spawns[si].key); si += 1
                else: s.free(frees[fi].key); fi += 1
            outcome = (s.generations, sorted(s.occupied), [s.handles[e.key] for e in spawns])
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
        """Relink times of the hitscans of this tic (the cause rule of (6)): the player's
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

    # ---- one snapshot ---------------------------------------------------------------
    def step(self, dump):
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
            # pickup message (5)
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
            # link order (6)
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
                    rec = (5, e.key, r['sector'], r['type'], r['direction'], r['topheight'], r['topcountdown'], r['speed'], r['topwait'])
                elif e.kind == 'PL':
                    if not r['running']: raise ProjectionError('Unadmitted', tic, 'a plat in stasis')
                    rec = (6, e.key, r['sector'], r['type'], r['status'], r['count'], r['low'], r['high'], r['speed'], r['wait'],
                           r['crush'], r['tag'], r['oldstatus'])
                else:
                    rec = (7, e.key, r['sector'], r['type'], r['direction'], r['floordestheight'], r['speed'], r['crush'],
                           r['newspecial'], r['texture'])
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


def project(directory, mapdata, skill=2):
    """Yield (dsim tic, DSIM38 bytes, report) for every snapshot of a dump directory, in
    order; a ProjectionError propagates to the caller at the first unprojectable tic."""
    proj = Projector(mapdata, skill)
    for name in m3h9c_dump.snapshot_names(directory):
        dump = Dump(os.path.join(directory, name))
        fields, report = proj.step(dump)
        yield dump.tic, encode(fields), report


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('dump'); ap.add_argument('--wad', required=True); ap.add_argument('--map', default='E1M1')
    ap.add_argument('--demo'); ap.add_argument('--out')
    a = ap.parse_args()
    mapdata = MapData(a.wad, a.map); skill = demo_skill(a.demo) if a.demo else 2
    if a.out: os.makedirs(a.out, exist_ok=True)
    count = 0; seq = hashlib.sha256(); last = None
    try:
        for tic, data, _ in project(a.dump, mapdata, skill):
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
