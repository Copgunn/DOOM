#!/usr/bin/env python3
"""Independent DSIM38/DSIM39 projection of the M3h9c whole-engine runner's per-tic dumps.

The genuine-engine runner (tools/reference/m3h9c_driver.c, or its M4a fork
tools/reference/m4a_driver.c, both read by m3h9c_dump.py) writes
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
  genuine code has not written yet must read the schema's 0 (DSIM35 decision N6; no
  statement reads them before the first write); the driver's declared ZeroFill
  zero-fills every zone block, so the projection asserts 0 and refuses anything else
  (only the `--infer` fallback, for dumps of earlier drivers, projects 0 there): a
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

Profile `episode1` (DSIM39, policy 23, context PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1; the
M4a Phase 2 plan §A.6, decisions D1-D10; exact mode only). The DSIM38 fields at their
paths, plus, from the same dumps:

- `level.secret_exit` (after `level.exit`) = H.secretexit, refused unless it implies the
  exit request. A dump of the M3h9c driver has no H.secretexit (only the M4a fork,
  m4a_driver.c, writes it): it projects 0 there, exactly, unless the map has a line with
  special 51 or 124 (the only callers of G_SecretExitLevel, p_switch.c:431-434,
  p_spec.c:760-762), which is a DumpDeficiency; `store.max_slots` = 1024 (D1).
- `sector.{i}.floorpic` (after `sector.{i}.special`) = the normalized 8-byte name (D2) of
  flat S.floorpic, resolved in the WAD's own directory as R_InitFlats/R_FlatNumForName do
  (firstflat = last F_START + 1, last match of the upper-cased name, the demo lump
  included); at load every sector's number must equal its SECTORS name's lookup.
- Sector-thinker records (`episode1_store_fields`): doors of vldoor_e 0/1/3/6 unchanged;
  plats with `type` after `high` (downWaitUpStay 1, raiseToNearestAndChange 3, whose
  `low` is never written and must read 0); floors with `direction`, `speed` after
  `floordestheight`, and `texture` (flat name) and `newspecial` for donutRaise only; the
  (type, direction, speed) domain of E1_FLOORS (stair steps are type 0, never written).
  A plat's `count` counts as written once it waits or is blocked going up (p_plats.c:75,
  84, 113).
- The weapon block with `cells`, `rockets`, `max_ammo.0..3` after `shells` (P.ammo2/3,
  P.maxammo0..3; max = base << backpack); after `use_down`: `powers.0..5` (P.pw0..5),
  `cards.0..5` (P.card0..5), `backpack` (P.backpack), with the plan's domains.
- `pickups.message` codes 14..29 for the driver's indices 8..11, 17..20, 24, 25, 30, 33,
  34, 38..40 (EPISODE1_MESSAGES); `tracer` must be NULL on every actor (not encoded).

Both profiles: when the driver's R rows carry `fn`/`sector` (the M4a fork's
P_RemoveThinker observer, m4a_driver.c; M3h9c-driver dumps lack them),
a mover created and marked within one tic is a removed mover (kind 8) of that sector, and
a known mover's R row must name its own kind and sector; without those columns such a
thinker is a DumpDeficiency.

The module is a library (`Projector`) plus a small CLI:
    level_snapshot.py DUMP_DIR --wad WAD [--map E1M1] [--out DIR] [--profile level|episode1]
                      [--infer] [--demo LMP]
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
# R-row `fn` (the driver's P_RemoveThinker observer, when the dump has the column): the kind
# of the removed non-mobj thinker; 4 is a ceiling (not admitted), 0 any other
REMOVED_FN = {1: 'DR', 2: 'PL', 3: 'FL'}
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


class Profile:
    """The capture contract a projection targets: DSIM38 (`level`, policy 22) or DSIM39
    (`episode1`, policy 23; the M4a Phase 2 plan §A.6 and decisions D1-D10)."""
    def __init__(self, name, version, policy, context, max_slots, messages, episode1):
        self.name, self.version, self.policy, self.context = name, version, policy, context
        self.max_slots, self.messages, self.episode1 = max_slots, messages, episode1


LEVEL = Profile('level', VERSION, POLICY, CONTEXT, MAX_SLOTS, DRIVER_MESSAGES, False)
# DSIM39 additions (plan §A.6). Driver message index (m4a_driver.c PICKUP_MESSAGES, 1-based:
# GOTSUPER 8, GOTBLUECARD 9, GOTYELWCARD 10, GOTREDCARD 11, GOTINVIS 17, GOTSUIT 18, GOTMAP 19,
# GOTVISOR 20, GOTROCKET 24, GOTROCKBOX 25, GOTBACKPACK 30, GOTCHAINSAW 33, GOTLAUNCHER 34; the
# locked-door strings PD_BLUEK 38, PD_YELLOWK 39, PD_REDK 40, set by EV_VerticalDoor,
# p_doors.c:378, 392, 405) -> the appended DSIM codes 14..29 (decision D7).
EPISODE1_MESSAGES = {**DRIVER_MESSAGES, **{8: 14, 9: 15, 10: 16, 11: 17, 17: 18, 18: 19, 19: 20, 20: 21, 24: 22,
                                             25: 23, 30: 24, 33: 25, 34: 26, 38: 27, 39: 28, 40: 29}}
EPISODE1 = Profile('episode1', 39, 23, b'PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1', 1024, EPISODE1_MESSAGES, True)
PROFILES = {'level': LEVEL, 'episode1': EPISODE1}
FRAC = 65536
# vldoor_e (p_spec.h:328-339): normal 0, close30ThenOpen 1, open 3, blazeOpen 6. Every admitted
# door has topwait VDOORWAIT 150 (p_doors.c:291, 466) and speed VDOORSPEED 2 units (p_spec.h:364;
# p_doors.c:292, 465), blazeOpen VDOORSPEED*4 (p_doors.c:325, 492).
E1_DOOR_SPEED = {0: 2 * FRAC, 1: 2 * FRAC, 3: 2 * FRAC, 6: 8 * FRAC}
# plattype_e (p_spec.h:274-281): downWaitUpStay 1 (speed PLATSPEED*4, wait 35*PLATWAIT,
# p_plats.c:207-215), raiseToNearestAndChange 3 (speed PLATSPEED/2, wait 0, p_plats.c:185-189);
# crush false for both (p_plats.c:180).
E1_PLAT = {1: (4 * FRAC, 105), 3: (FRAC // 2, 0)}
# (floor_e, direction, speed) of every admitted floor record (p_spec.h:537-570, FLOORSPEED FRACUNIT):
E1_FLOORS = {(0, -1, FRAC),          # lowerFloor, EV_DoFloor p_floor.c:291-296
             (0, -1, FRAC // 2),     # donut hole: lowerFloor at FLOORSPEED/2, p_spec.c:1210-1216
             (0, 1, FRAC // 4),      # stair step build8 (type never written: zero fill, H7), p_floor.c:483-500
             (0, 1, 4 * FRAC),       # stair step turbo16, p_floor.c:497-500
             (1, -1, FRAC),          # lowerFloorToLowest, p_floor.c:299-304
             (2, -1, 4 * FRAC),      # turboLower, p_floor.c:307-314
             (3, 1, FRAC),           # raiseFloor, p_floor.c:319-327
             (4, 1, FRAC),           # raiseFloorToNearest, p_floor.c:339-344
             (11, 1, FRAC // 2)}     # donutRaise, p_spec.c:1196-1203
DONUT_RAISE = 11
# maxammo (p_inter.c:58), doubled once by the first backpack (p_inter.c:590-594)
MAXAMMO = (200, 50, 300, 50)
# power durations (doomdef.h:235-238): INVISTICS = IRONTICS = 60*TICRATE, INFRATICS = 120*TICRATE
INVISTICS, IRONTICS, INFRATICS = 2100, 2100, 4200


class ProjectionError(Exception):
    """The dump cannot be projected; `kind` is Ambiguous, DumpDeficiency or Unadmitted."""
    def __init__(self, kind, tic, text):
        super().__init__(f'{kind} at tic {tic}: {text}'); self.kind, self.tic, self.text = kind, tic, text


def demo_lumps(path):
    """The lump D_AddFile adds for the demo (ExtractFileBase: the upper-cased base name)."""
    return (Path(path).stem.upper()[:8],) if path else ()


def demo_skill(path):
    data = Path(path).read_bytes()
    assert data[0] == 110, 'a version-110 demo (decision G1)'
    return data[1]


class MapData:
    """The ten lumps of one map slot of a WAD, the frozen `map_fields` of them, and the
    derived subsector sectors, line front sectors and line specials."""
    def __init__(self, wad, slot, extra_lumps=()):
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
        self._flats(data, extra_lumps)
        self.front = [sides[l[5]][5] for l in lines]
        self.subsector_sector = []
        for count, first in struct.iter_unpack('<HH', self.lumps[5]):
            _, _, _, line, side, _ = segs[first]
            self.subsector_sector.append(sides[lines[line][5 + side]][5])

    def _flats(self, data, extra_lumps):
        """The flat directory as the genuine R_InitFlats/R_FlatNumForName see it (decision D2):
        lump names as W_AddFile stores them (strncpy of 8 bytes: NUL-padded after the first
        NUL, case kept, w_wad.c), then `extra_lumps` (the demo, added by D_AddFile as a single
        lump named by its upper-cased base name). firstflat = W_GetNumForName("F_START") + 1,
        lastflat = W_GetNumForName("F_END") - 1 (r_data.c R_InitFlats; W_CheckNumForName
        returns the last match). Flat n is the lump firstflat + n."""
        _, count, offset = struct.unpack_from('<4sii', data, 0)
        stored = []
        for k in range(count):
            raw = struct.unpack_from('<8s', data, offset + 16 * k + 8)[0]
            stored.append(raw.split(b'\0')[0].ljust(8, b'\0'))
        stored += [x.encode().upper()[:8].ljust(8, b'\0') for x in extra_lumps]
        self.lump_names = stored
        last = {}
        for k, name in enumerate(stored): last[name] = k
        self.last_lump = last
        self.firstflat = last[b'F_START'.ljust(8, b'\0')] + 1 if b'F_START'.ljust(8, b'\0') in last else None
        self.lastflat = last[b'F_END'.ljust(8, b'\0')] - 1 if b'F_END'.ljust(8, b'\0') in last else None
        # the SECTORS lump's floor names (offset 4, 8 bytes per sector record of 26)
        self.sector_floor_names = [self.lumps[7][26 * i + 4:26 * i + 12] for i in range(len(self.lumps[7]) // 26)]

    @staticmethod
    def normalize(raw):
        """The normalized 8-byte flat name (D2): bytes up to the first NUL, ASCII upper-cased,
        zero-padded (R_FlatNumForName copies 8 bytes and W_CheckNumForName upper-cases them)."""
        return raw.split(b'\0')[0][:8].upper().ljust(8, b'\0')

    def flat_number(self, raw):
        """R_FlatNumForName of a name (r_data.c): the last lump matching the upper-cased name,
        minus firstflat; None when the lump does not exist (the genuine code calls I_Error)."""
        k = self.last_lump.get(self.normalize(raw))
        return None if k is None or self.firstflat is None else k - self.firstflat

    def flat_name(self, number):
        """The normalized name of flat `number` (D2), or None when the number is not one that
        R_FlatNumForName can return: outside 0..numflats-1, or a lump whose stored name is not
        upper case or is not the last lump of its name (W_CheckNumForName would not find it)."""
        if self.firstflat is None or self.lastflat is None or not 0 <= number <= self.lastflat - self.firstflat:
            return None
        k = self.firstflat + number; name = self.lump_names[k]
        if name != self.normalize(name) or self.last_lump.get(name) != k: return None
        return name

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
    __slots__ = ('cls', 'key', 'kind', 'row', 'pending', 'inferred', 'sector', 'serial', 'waited', 'seen_up')
    def __init__(self, cls, key, kind, row):
        self.cls, self.key, self.kind, self.row, self.pending, self.inferred = cls, key, kind, row, False, False
        self.serial = None; self.waited = False; self.seen_up = False
        self.sector = row.get('sector')


def ident(handle):
    return struct.pack('<IQ', *handle)


class Projector:
    """Feed the dumps of one run in order (load first); `step` returns (DSIM38 bytes,
    report) for each."""
    def __init__(self, mapdata, skill=2, infer=False, profile=LEVEL):
        self.map, self.skill, self.infer, self.profile = mapdata, skill, infer, profile
        if profile.episode1 and infer: raise ValueError('the DSIM39 projection is exact only (no --infer)')
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
                elif sv in current and cur[serial.index(sv)][0] == 'R' and 'fn' in cur[serial.index(sv)][1]:
                    # created and marked by P_RemoveThinker within this tic (a one-tic mover):
                    # the R row's fn/sector columns, read by the driver's P_RemoveThinker
                    # observer before the genuine call overwrites the function
                    row = cur[serial.index(sv)][1]
                    if row['fn'] == 4: raise ProjectionError('Unadmitted', tic, 'a ceiling thinker')
                    if row['fn'] not in REMOVED_FN or row['sector'] < 0:
                        raise ProjectionError('DumpDeficiency', tic, 'a one-tic thinker of unknown kind')
                    classes[sv] = 'mover'
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
                            if 'fn' in row and e.kind in MOVERS and (REMOVED_FN.get(row['fn']) != e.kind or row['sector'] != e.sector):
                                raise ProjectionError('DumpDeficiency', tic, 'the R row names another mover kind or sector')
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
            if m not in self.profile.messages: raise ProjectionError('Unadmitted', tic, 'a pickup message outside the DSIM table')
            self.message = self.profile.messages[m]; report['pickups'] = 1
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
        pr = self.profile; ep = pr.episode1
        if ep: self._episode1_checks(d, actors)
        f = [('tic', n('Q', tic)), ('tic_rate', n('I', 35)), ('profile', n('I', pr.policy)),
             ('registry.schema', REGISTRY), ('context.schema', pr.context), ('paused', b'\0'),
             ('rng.gameplay', n('B', h['prndindex'])), ('rng.miscellaneous', n('B', h['rndindex'])),
             ('actors.options', bytes([self.skill, 0, 0])),
             ('level.total_kills', n('i', h['totalkills'])), ('level.total_items', n('i', h['totalitems'])),
             ('level.total_secrets', n('i', h['totalsecret'])), ('level.single_player_kill_count', n('i', p0['killcount'])),
             ('level.exit', n('B', h['gameaction'] == GA_COMPLETED))]
        if ep:
            # H.secretexit (g_game.c:999-1018: G_SecretExitLevel sets it with ga_completed,
            # G_ExitLevel clears it); the flag implies the exit request
            if 'secretexit' in h: se = h['secretexit']
            elif any(s in (51, 124) for s in self.map.special):
                # an M3h9c-driver dump (no H.secretexit) on a map that can reach G_SecretExitLevel
                raise ProjectionError('DumpDeficiency', tic, 'H.secretexit is absent on a map with a secret exit line')
            else: se = 0    # no line calls G_SecretExitLevel (p_switch.c:431-434, p_spec.c:760-762)
            if se not in (0, 1) or (se and h['gameaction'] != GA_COMPLETED):
                raise ProjectionError('Unadmitted', tic, 'secretexit without the exit request')
            f.append(('level.secret_exit', n('B', se)))
        f += [('store.max_slots', n('I', pr.max_slots)), ('store.slot_count', n('Q', len(self.actors.generations))),
             ('store.head', opt(aorder[0] if aorder else None)), ('store.tail', opt(aorder[-1] if aorder else None)),
             ('thinker.order', b''.join(ident(x) for x in aorder)),
             ('thinker.shared_order', b''.join((b'\0' if e.cls == 'actor' else b'\1') + ident(self.handle(e)) for e in entries))]
        if len(self.actors.generations) > pr.max_slots:
            raise ProjectionError('Unadmitted', tic, f'more than {pr.max_slots} actor slots')
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
            elif e.cls == 'mover' and ep:
                handles[0, e.key] = self.sthinkers.handles[e.key]; sector_of[0, e.key] = e.sector
                rec = self._episode1_mover(tic, e)
                if e.kind != 'removed':
                    if r['sector'] in live: raise ProjectionError('Unadmitted', tic, 'two live movers in one sector')
                    live[r['sector']] = e
            elif e.cls == 'mover':
                handles[0, e.key] = self.sthinkers.handles[e.key]; sector_of[0, e.key] = e.sector
                if e.kind == 'removed': rec = (8, e.key)
                elif e.kind == 'DR':
                    # EV_VerticalDoor/EV_DoDoor never set topcountdown for the admitted
                    # types; T_VerticalDoor writes it when the door reaches the top
                    # (direction 0), before any read. Until then the schema value is 0
                    # (DSIM35 decision N6), which the driver's ZeroFill guarantees in C.
                    e.waited = e.waited or r['direction'] == 0
                    top = self._zero(tic, e, 'door topcountdown before the first wait', r['topcountdown'], not e.waited)
                    rec = (5, e.key, r['sector'], r['type'], r['direction'], r['topheight'], top, r['speed'], r['topwait'])
                elif e.kind == 'PL':
                    if not r['running']: raise ProjectionError('Unadmitted', tic, 'a plat in stasis')
                    # oldstatus is written only by P_StopPlat (stasis, not admitted) and count
                    # only by T_PlatRaise when the plat starts waiting (status 2, count = wait;
                    # EV_DoPlat's downWaitUpStay sets neither): 0 until then (N6, ZeroFill)
                    e.waited = e.waited or r['status'] == 2
                    count = self._zero(tic, e, 'plat count before the first wait', r['count'], not e.waited)
                    old = self._zero(tic, e, 'plat oldstatus', r['oldstatus'], True)
                    rec = (6, e.key, r['sector'], r['type'], r['status'], count, r['low'], r['high'], r['speed'], r['wait'],
                           r['crush'], r['tag'], old)
                else:
                    # newspecial/texture are written only for the change types (not admitted): 0 (N6, ZeroFill)
                    rec = (7, e.key, r['sector'], r['type'], r['direction'], r['floordestheight'], r['speed'], r['crush'],
                           self._zero(tic, e, 'floor newspecial', r['newspecial'], True),
                           self._zero(tic, e, 'floor texture', r['texture'], True))
                if e.kind != 'removed':
                    if r['sector'] in live: raise ProjectionError('Unadmitted', tic, 'two live movers in one sector')
                    live[r['sector']] = e
            else:
                continue
            records.append(rec)
        try:
            if ep: store = episode1_store_fields(records, self.sthinkers.generations, handles, sector_of)
            else: store = movers_snapshot.store_fields(records, self.sthinkers.generations, handles, 0, sector_of, tags)
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
            if ep:
                # S.floorpic, the flat number, as its normalized name (D2)
                name = self.map.flat_name(s['floorpic'])
                if name is None: raise ProjectionError('Unadmitted', tic, f'sector {i} floorpic {s["floorpic"]} names no flat')
                f.append((f'sector.{i}.floorpic', name))
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
            if ep:
                wf = episode1_weapon_fields(weapons[:5] + [p0['ammo2'], p0['ammo3']] + [p0[f'maxammo{k}'] for k in range(4)]
                                            + [p0['backpack']] + weapons[5:])
            else:
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
                   ('original_player.0.secret_count', n('i', p0['secretcount'])), ('original_player.0.use_down', n('B', p0['usedown']))]
        if ep: f += self._episode1_player(tic, p0)
        f += [('events.last.count', n('Q', 0)), ('events.pending.count', n('Q', 0))]
        f += self.map.fields
        return f

    # ---- DSIM39 (episode1) --------------------------------------------------------------
    def _episode1_checks(self, d, actors):
        tic = d.tic
        for e in actors:
            # tracer: written only by A_Tracer, A_VileChase/A_Fire and the skull family, none
            # admitted (plan §A.6); asserted NULL, not encoded
            if e.row['tracer'] != -1: raise ProjectionError('Unadmitted', tic, 'an actor with a tracer')
        if self.prev is d and tic == 0:
            # load: every sector's floorpic is R_FlatNumForName of its SECTORS name (p_setup.c
            # P_LoadSectors), which cross-checks the flat directory read here against the driver's
            for i, s in enumerate(d.sectors):
                if self.map.flat_number(self.map.sector_floor_names[i]) != s['floorpic']:
                    raise ProjectionError('DumpDeficiency', tic, f'sector {i} floorpic differs from its SECTORS name')

    def _episode1_mover(self, tic, e):
        """One mover record of the DSIM39 sector-thinker store (plan §A.6) as a dict for
        `episode1_store_fields`; fields the genuine code has not written read the schema's 0
        (ZeroFill) and are asserted, never forced."""
        r = e.row
        if e.kind == 'removed': return dict(kind=8, key=e.key)
        if e.kind == 'DR':
            # topcountdown: written when the door starts waiting, direction 0 (T_VerticalDoor
            # p_doors.c:144 close30ThenOpen at the bottom, :182 normal at the top); EV_DoDoor and
            # EV_VerticalDoor never write it for the admitted types (p_doors.c:264-347, 354-499)
            e.waited = e.waited or r['direction'] == 0
            top = self._zero(tic, e, 'door topcountdown before the first wait', r['topcountdown'], not e.waited)
            if r['type'] not in E1_DOOR_SPEED or r['speed'] != E1_DOOR_SPEED[r['type']] or r['topwait'] != 150 \
                    or r['direction'] not in (-1, 0, 1):
                raise ProjectionError('Unadmitted', tic, f'door type {r["type"]} outside the DSIM39 domain')
            return dict(kind=5, key=e.key, sector=r['sector'], type=r['type'], direction=r['direction'],
                        topheight=r['topheight'], topcountdown=top)
        if e.kind == 'PL':
            if not r['running']: raise ProjectionError('Unadmitted', tic, 'a plat in stasis')
            if r['type'] not in E1_PLAT or (r['speed'], r['wait']) != E1_PLAT[r['type']] or r['crush'] != 0 \
                    or r['status'] not in (0, 1, 2) or r['tag'] != self.prev.sectors[r['sector']]['tag']:
                raise ProjectionError('Unadmitted', tic, f'plat type {r["type"]} outside the DSIM39 domain')
            # count: written by T_PlatRaise when the plat starts waiting (status 2, p_plats.c:84,
            # 113) or is blocked going up (status up -> down, p_plats.c:75); EV_DoPlat never
            # writes it (p_plats.c:170-252). A raiseToNearestAndChange plat starts up, so a
            # down status means it was blocked; a downWaitUpStay plat starts down, so down after
            # up means blocked.
            e.waited = e.waited or r['status'] == 2 or (r['status'] == 1 and (r['type'] == 3 or e.seen_up))
            e.seen_up = e.seen_up or r['status'] == 0
            count = self._zero(tic, e, 'plat count before the first wait', r['count'], not e.waited)
            self._zero(tic, e, 'plat oldstatus', r['oldstatus'], True)      # P_StopPlat only
            if r['type'] == 3:
                # raiseToNearestAndChange never writes low (p_plats.c:185-195): the zero projection (H7)
                low = self._zero(tic, e, 'raiseToNearestAndChange plat low', r['low'], True)
            else:
                low = r['low']
                if low > r['high']: raise ProjectionError('Unadmitted', tic, 'a plat with low above high')
            return dict(kind=6, key=e.key, sector=r['sector'], status=r['status'], count=count, low=low,
                        high=r['high'], type=r['type'])
        # FL: crush is false for every admitted floor (p_floor.c:287, p_spec.c:1198/1212) or
        # never written (stairs, zero fill, H7); texture/newspecial are written only for
        # donutRaise (p_spec.c:1202-1203) and otherwise read the schema's 0
        if (r['type'], r['direction'], r['speed']) not in E1_FLOORS:
            raise ProjectionError('Unadmitted', tic, f'floor type {r["type"]} direction {r["direction"]} speed '
                                  f'{r["speed"]} outside the DSIM39 domain')
        self._zero(tic, e, 'floor crush', r['crush'], True)
        out = dict(kind=7, key=e.key, sector=r['sector'], type=r['type'], floordestheight=r['floordestheight'],
                   direction=r['direction'], speed=r['speed'])
        if r['type'] == DONUT_RAISE:
            name = self.map.flat_name(r['texture'])
            if name is None: raise ProjectionError('Unadmitted', tic, f'donut texture {r["texture"]} names no flat')
            out.update(texture=name, newspecial=r['newspecial'])
        else:
            self._zero(tic, e, 'floor newspecial', r['newspecial'], True)
            self._zero(tic, e, 'floor texture', r['texture'], True)
        return out

    def _episode1_player(self, tic, p0):
        """powers[6] (powertype_t order, doomdef.h:214-222), cards[6] (card_t order,
        doomdef.h:162-171) and backpack, from the P row's pw0..pw5, card0..card5, backpack."""
        pw = [p0[f'pw{k}'] for k in range(6)]; cards = [p0[f'card{k}'] for k in range(6)]
        # domains (plan §A.6): no invulnerability or berserk in Episode 1 single player; the
        # timed powers start at their *TICS (P_GivePower, p_inter.c) and count down
        # (p_user.c:339-353); allmap is the boolean 1
        if pw[0] or pw[1] or not 0 <= pw[2] <= INVISTICS or not 0 <= pw[3] <= IRONTICS or pw[4] not in (0, 1) \
                or not 0 <= pw[5] <= INFRATICS:
            raise ProjectionError('Unadmitted', tic, 'a power outside the DSIM39 domain')
        if any(c not in (0, 1) for c in cards) or any(cards[3:]) or p0['backpack'] not in (0, 1):
            raise ProjectionError('Unadmitted', tic, 'a card or the backpack outside the DSIM39 domain')
        return ([(f'original_player.0.powers.{k}', n('i', pw[k])) for k in range(6)]
                + [(f'original_player.0.cards.{k}', n('B', cards[k])) for k in range(6)]
                + [('original_player.0.backpack', n('B', p0['backpack']))])

    def _zero(self, tic, e, what, value, unwritten):
        """A mover field the genuine code has not written yet: the schema says 0 and the
        driver's ZeroFill makes C read 0, so anything else is refused. The `--infer`
        fallback (dumps of drivers without ZeroFill, whose zone bytes are leftovers)
        projects 0 instead."""
        if not unwritten: return value
        if self.infer: return 0
        if value != 0: raise ProjectionError('DumpDeficiency', tic, f'{what} is {value}, not the schema 0 (ZeroFill?)')
        return 0

    def _target(self, tic, e):
        return self._ref(tic, e.row['target'], self.entries)


def link_bytes(proj, order):
    return b''.join(ident(proj.handle(e)) for e in order)


def episode1_record_fields(slot, rec, sector_of, lights):
    """One occupied slot of the DSIM39 sector-thinker store (plan §A.6): the light records
    unchanged (the frozen DSIM34 encoder), a door unchanged from DSIM35, a plat with `type`
    after `high`, a floor with `direction` and `speed` after `floordestheight` and, for
    donutRaise only, `texture` (normalized flat name) and `newspecial`."""
    p = f'sector_thinker.{slot}'
    kind = rec['kind'] if isinstance(rec, dict) else rec[0]
    if kind < 5: return [(path, v) for path, v in lights if path.startswith(p + '.')]
    out = [(p + '.generation', None), (p + '.occupied', b'\1'), (p + '.kind', n('B', kind))]
    if kind == 8: return out + [(p + '.sector', n('H', sector_of))]
    assert rec['sector'] == sector_of
    out.append((p + '.sector', n('H', rec['sector'])))
    if kind == 5:
        out += [(p + '.type', n('B', rec['type'])), (p + '.direction', n('i', rec['direction'])),
                (p + '.topheight', n('i', rec['topheight'])), (p + '.topcountdown', n('i', rec['topcountdown']))]
    elif kind == 6:
        out += [(p + '.status', n('B', rec['status'])), (p + '.count', n('i', rec['count'])), (p + '.low', n('i', rec['low'])),
                (p + '.high', n('i', rec['high'])), (p + '.type', n('B', rec['type']))]
    else:
        assert kind == 7
        out += [(p + '.type', n('B', rec['type'])), (p + '.floordestheight', n('i', rec['floordestheight'])),
                (p + '.direction', n('i', rec['direction'])), (p + '.speed', n('i', rec['speed']))]
        if rec['type'] == DONUT_RAISE:
            assert len(rec['texture']) == 8
            out += [(p + '.texture', rec['texture']), (p + '.newspecial', n('h', rec['newspecial']))]
    return out


def episode1_store_fields(records, generations, handles, sector_of):
    """The DSIM39 sector-thinker store, framed as movers_snapshot.store_fields frames DSIM35:
    `sector_thinkers.slot_count`, then per slot its generation and occupied flag and, when
    occupied, its record. `records` holds the light tuples (kinds 1-4, slot = load order) and
    the mover dicts of `Projector._episode1_mover`."""
    by_slot = {}
    for rec in records:
        if isinstance(rec, dict): slot, generation = handles[0, rec['key']]
        else: slot, generation = rec[1], 1
        assert generations[slot] == generation and slot not in by_slot
        by_slot[slot] = rec
    lights = specials_snapshot.sector_thinker_fields([r for r in records if not isinstance(r, dict)])
    out = [('sector_thinkers.slot_count', n('Q', len(generations)))]
    for slot, generation in enumerate(generations):
        p = f'sector_thinker.{slot}'
        if slot not in by_slot:
            out += [(p + '.generation', n('Q', generation)), (p + '.occupied', b'\0')]; continue
        rec = by_slot[slot]
        fields = episode1_record_fields(slot, rec, sector_of.get((0, rec['key'])) if isinstance(rec, dict) else None, lights)
        out += [(path, n('Q', generation) if path.endswith('.generation') else v) for path, v in fields]
    return out


def episode1_weapon_fields(row):
    """The DSIM39 weapon block (plan §A.6): the DSIM24 block with `cells`, `rockets` (ammo[2],
    ammo[3], ammotype_t order am_clip, am_shell, am_cell, am_misl, doomdef.h) and
    `max_ammo.0..3` after `shells`. weapontype_t (doomdef.h): fist 0 .. missile 4, chainsaw 7;
    pending wp_nochange 10."""
    ready, pending, owned, clips, shells, cells, rockets, *rest = row
    maxammo, backpack, (latch, refire, bob, light, *sprites) = rest[:4], rest[4], rest[5:]
    weapons = {0, 1, 2, 3, 4, 7}
    assert ready in weapons and pending in weapons | {10}
    assert owned & 3 == 3 and owned & ~sum(1 << w for w in weapons) == 0 and owned & (1 << ready)
    assert backpack in (0, 1) and list(maxammo) == [m << backpack for m in MAXAMMO]
    assert all(0 <= a <= m for a, m in zip((clips, shells, cells, rockets), maxammo))
    assert latch in (0, 1) and 0 <= refire <= 2147483647 and 0 <= bob <= 0x100000 and light in (0, 1, 2)
    prefix = 'original_player.0.weapons'
    fields = [(prefix + '.present', b'\1'), (prefix + '.ready', n('B', ready)), (prefix + '.pending', n('B', pending)),
              (prefix + '.owned', n('H', owned)), (prefix + '.bullets', n('H', clips)), (prefix + '.shells', n('H', shells)),
              (prefix + '.cells', n('H', cells)), (prefix + '.rockets', n('H', rockets))]
    fields += [(f'{prefix}.max_ammo.{k}', n('H', maxammo[k])) for k in range(4)]
    fields += [(prefix + '.attack_down', n('B', latch)), (prefix + '.refire', n('i', refire)),
               (prefix + '.bob', n('i', bob)), (prefix + '.extra_light', n('i', light))]
    assert len(sprites) == 8
    for i in range(2):
        state, tics, x, y = sprites[4 * i:4 * i + 4]
        for key, code, value in [('state', 'H', 65535 if state < 0 else state), ('tics', 'i', tics), ('x', 'i', x), ('y', 'i', y)]:
            fields.append((f'{prefix}.psprite.{i}.{key}', n(code, value)))
    return fields


def encode(fields, version=VERSION):
    """The DSIM byte string of `fields` under `version` (38 by default; the frozen framing)."""
    data = bytearray(b'DSIM' + struct.pack('<IQ', version, len(fields)))
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


def project(directory, mapdata, skill=2, infer=False, profile=LEVEL):
    """Yield (dsim tic, DSIM bytes, report) for every snapshot of a dump directory, in
    order; a ProjectionError propagates to the caller at the first unprojectable tic.
    The default is exact (observer rows required); `infer` selects the fallback (DSIM38
    only). `profile` selects DSIM38 (LEVEL) or DSIM39 (EPISODE1)."""
    proj = Projector(mapdata, skill, infer, profile)
    for dump, following in snapshots(directory):
        fields, report = proj.step(dump, following)
        yield dump.tic, encode(fields, profile.version), report


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('dump'); ap.add_argument('--wad', required=True); ap.add_argument('--map', default='E1M1')
    ap.add_argument('--demo'); ap.add_argument('--out')
    ap.add_argument('--infer', action='store_true', help='fallback for dumps without observer rows (never used by the comparator)')
    ap.add_argument('--profile', choices=sorted(PROFILES), default='level',
                    help='level: DSIM38/policy 22 (default); episode1: DSIM39/policy 23')
    a = ap.parse_args()
    profile = PROFILES[a.profile]
    mapdata = MapData(a.wad, a.map, demo_lumps(a.demo)); skill = demo_skill(a.demo) if a.demo else 2
    if a.out: os.makedirs(a.out, exist_ok=True)
    count = 0; seq = hashlib.sha256(); last = None
    try:
        for tic, data, _ in project(a.dump, mapdata, skill, a.infer, profile):
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
