#!/usr/bin/env python3
"""Read-only count report over the classic maps of locally held IWADs (M3h6 §4.2, §4.5, §9).

Local use only. The script is never invoked by CI; it exits 0 with a one-line notice
when the IWAD directory is absent. It prints counts and pass/fail facts only: a map is
named by its slot (`E1M1`, `MAP01`), never by any lump beyond the slot, and no coordinate
of any object is printed (spans are differences). Nothing is written anywhere.

    python3 tools/iwad_counts.py                  # $DOOM_IWAD_DIR or /mnt/d/DoomIWADs, text report
    python3 tools/iwad_counts.py --markdown       # the committed counts table
    python3 tools/iwad_counts.py --readiness E1M1 # the §9 readiness counts of one slot
    python3 tools/iwad_counts.py --specials-markdown  # the M3h7 §5.6 specials table
    python3 tools/iwad_counts.py --movers E1M1    # every M3h8 Stage B measurement of one slot
    python3 tools/iwad_counts.py --movers-markdown    # the M3h8 Stage B movers table
    python3 tools/iwad_counts.py --things E1M1    # the M3h9 THING census of one slot
    python3 tools/iwad_counts.py --things-markdown    # the M3h9 census tables (all slots, then E1M1)
    python3 tools/iwad_counts.py --spawn-rule     # the spawn-admission rule over every slot (counts only)
    python3 tools/iwad_counts.py --spawn-rule E1M1    # the same with the per-doomednum violations of one slot
    python3 tools/iwad_counts.py --spawn-rule-markdown    # the spawn-rule table (all slots)
    python3 tools/iwad_counts.py --spawn-fit [SLOT]       # the spawn height fit via R_PointInSubsector (counts only)
    python3 tools/iwad_counts.py --things-budget [SLOT]   # BLOCKMAP/intercept budget facts (counts only)
    python3 tools/iwad_counts.py --episode-markdown [E]   # the M4a episode census (DOOM.WAD, medium skill)
    python3 tools/iwad_counts.py --demos-markdown # the M4a DEMOn lump header facts
    python3 tools/iwad_counts.py DIR ...          # another read-only directory

The rules evaluated are those of docs/M3H6_REFERENCE_INVESTIGATION.md: the arithmetic
certificate (i)–(vii) of §4.2, the boundary rules C1–C3, the closed-cell BLOCKMAP rule,
grid coverage and REJECT length of §4.5, and the slide-window bound of §4.4; the M3h7
§5.6 specials counts and the M3h8 Stage B mover measurements and load rules of
docs/M3H7_REFERENCE_INVESTIGATION.md §4.3, §4.4 (with the reachable-height fixpoint) and
§4.6, recorded in docs/validation/M3H8_IWAD_COUNTS.md; and the M3h9 THING census under
P_SpawnMapThing (p_mobj.c) with the mobjinfo facts of info.c.
"""

from __future__ import annotations

import argparse
import collections
from fractions import Fraction
import math
import os
import re
import struct
import sys

DEFAULT_DIR = os.environ.get("DOOM_IWAD_DIR") or "/mnt/d/DoomIWADs"
MAP_SLOT = re.compile(r"E\dM\d|MAP\d\d")
MAP_LUMPS = (
    "THINGS",
    "LINEDEFS",
    "SIDEDEFS",
    "VERTEXES",
    "SEGS",
    "SSECTORS",
    "NODES",
    "SECTORS",
    "REJECT",
    "BLOCKMAP",
)
NO_SIDE = 0xFFFF
I32_MAX = 2**31 - 1
# §4.2 constants: trace length 2,048 + one cell 128 + the FRACUNIT nudge = 2,177;
# 2,049 for the denominator; 128 intercepts; ±512 heights; 8,192 SlopeDiv.
HITSCAN_REACH = 2_177
HITSCAN_DEN = 2_049
MAX_INTERCEPTS = 128
HEIGHT_BOUND = 512
SLOPE_DIV_BOUND = 8_192
CELL = 128
# Perpendicular offset for the C3 parity probe: (dy, -dx) / 2**34 is shorter than the
# least distance between a half-integer midpoint and any integer line it is not on.
PROBE_SHIFT = 34

# M3h7 §5.6 counts (docs/M3H7_REFERENCE_INVESTIGATION.md): the Stage A admission facts
# of §4.2 and the pure-count Stage B facts of §4.4 and §4.6. They are printed as extra
# lines of the text report and by --specials-markdown; --markdown and --readiness are
# unchanged.
# Stage A (§4.2): admitted sector and line specials, scroller bound and light spawners.
STAGE_A_SECTOR_SPECIALS = frozenset((0, 1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16, 17))
STAGE_A_LINE_SPECIALS = frozenset((0, 48))
LIGHT_SECTOR_SPECIALS = frozenset((1, 2, 3, 4, 8, 12, 13, 17))
SCROLLER = 48
MAXLINEANIMS = 64
# Stage B (§4.3, E1M1's seven types): DR door 1 acts on its back sector; the others act
# on every sector whose tag equals the line's tag (P_FindSectorFromLineTag, so tag 0
# selects every untagged sector, owner decision 11). 11 (exit) moves nothing.
E1M1_LINE_TYPES = (1, 63, 109, 88, 36, 38, 11)
DR_DOOR = 1
TAGGED_DOOR = frozenset((63, 109))
TAGGED_PLAT = frozenset((88,))
TAGGED_FLOOR = frozenset((36, 38))
TAG_USING = TAGGED_DOOR | TAGGED_PLAT | TAGGED_FLOOR
MAXPLATS = 30
MAXBUTTONS = 16
MAXSPECIALCROSS = 8
# Lines whose activation calls P_ChangeSwitchTexture(line, 1) and so can start a button:
# the reusable use switches of P_UseSpecialLine (p_switch.c:514–647) and gun line 46 of
# P_ShootSpecialLine (p_spec.c:988–991).
REUSABLE_SWITCHES = frozenset(
    (42, 43, 45, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 114, 115, 116, 123, 132, 99, 134, 136, 138, 139, 46)
)

# M3h8 Stage B measurements (docs/M3H7_REFERENCE_INVESTIGATION.md §4.3, §4.4, §4.6, §5.6).
# Every vanilla line special by kind and trigger, read from P_CrossSpecialLine
# (p_spec.c:492–870), P_ShootSpecialLine (p_spec.c:959–1000) and P_UseSpecialLine
# (p_switch.c:276–650). The kind decides the `specialdata` structure a tagged target gets:
# doors `vldoor_t`, plats `plat_t`, floors, stairs and donuts `floormove_t`
# (EV_BuildStairs p_floor.c:453, EV_DoDonut p_spec.c:1163), ceilings `ceiling_t`.
# Stairs and donuts are counted on their tagged sectors only (they also move sectors
# found by walking the map, which this table does not follow).
LINE_KINDS = {}


def _kind(kind, trigger, *specials):
    for special in specials:
        LINE_KINDS[special] = (kind, trigger)


_kind("manual door", "DR", 1, 26, 27, 28, 117)
_kind("manual door", "D1", 31, 32, 33, 34, 118)
_kind("door", "W1", 2, 3, 4, 16, 108, 109, 110)
_kind("door", "WR", 75, 76, 86, 90, 105, 106, 107)
_kind("door", "S1", 29, 50, 103, 111, 112, 113, 133, 135, 137)
_kind("door", "SR", 42, 61, 63, 114, 115, 116, 99, 134, 136)
_kind("door", "GR", 46)
_kind("plat", "W1", 10, 22, 53, 121)
_kind("plat", "WR", 87, 88, 95, 120)
_kind("plat", "S1", 14, 15, 20, 21, 122)
_kind("plat", "SR", 62, 66, 67, 68, 123)
_kind("plat", "G1", 47)
_kind("floor", "W1", 5, 19, 30, 36, 37, 38, 56, 58, 59, 119, 130)
_kind("floor", "WR", 82, 83, 84, 91, 92, 93, 94, 96, 98, 128, 129)
_kind("floor", "S1", 18, 23, 55, 71, 101, 102, 131, 140)
_kind("floor", "SR", 45, 60, 64, 65, 69, 70, 132)
_kind("floor", "G1", 24)
_kind("ceiling", "W1", 6, 25, 44, 141)
_kind("ceiling", "WR", 72, 73, 77)
_kind("ceiling", "S1", 41, 49)
_kind("ceiling", "SR", 43)
_kind("floor+ceiling", "W1", 40)  # EV_DoCeiling(raiseToHighest) then EV_DoFloor(lowerFloorToLowest)
_kind("stairs", "W1", 8, 100)
_kind("stairs", "S1", 7, 127)
_kind("donut", "S1", 9)
_kind("light", "W1", 12, 13, 17, 35, 104)
_kind("light", "WR", 79, 80, 81)
_kind("light", "SR", 138, 139)
_kind("teleport", "W1", 39, 125)
_kind("teleport", "WR", 97, 126)
_kind("plat stop", "W1", 54)
_kind("plat stop", "WR", 89)
_kind("crush stop", "W1", 57)
_kind("crush stop", "WR", 74)
_kind("exit", "W", 52, 124)  # P_CrossSpecialLine never clears 52 or 124
_kind("exit", "S1", 11, 51)
_kind("scroll", "-", 48)
# Kinds whose action walks P_FindSectorFromLineTag (p_spec.c:434): every kind but manual
# doors (EV_VerticalDoor reads the back side), exits and the scroller.
UNTAGGED_KINDS = frozenset(("manual door", "exit", "scroll"))
# Kind -> mover families (the `specialdata` structure of each target).
MOVER_FAMILIES = {
    "manual door": ("door",),
    "door": ("door",),
    "plat": ("plat",),
    "floor": ("floor",),
    "stairs": ("floor",),
    "donut": ("floor",),
    "ceiling": ("ceiling",),
    "floor+ceiling": ("floor", "ceiling"),
}
STAGE_B_LINE_SPECIALS = STAGE_A_LINE_SPECIALS | frozenset(E1M1_LINE_TYPES)
W1_ADMITTED = frozenset((109, 36, 38))
# Plane speeds of the admitted bodies, in map units per tic: VDOORSPEED 2 (p_spec.h:364)
# for doors 1 and 63, VDOORSPEED * 4 for 109 blazeOpen (p_doors.c:320–323), PLATSPEED * 4
# for 88 (p_plats.c:208), FLOORSPEED 1 for 38 and FLOORSPEED * 4 for 36 (p_floor.c:299–315).
ADMITTED_SPEED = {1: 2, 63: 2, 109: 8, 88: 4, 38: 1, 36: 4}
# P_FindHighestFloorSurrounding starts at -500 * FRACUNIT (p_spec.c:295): the turbo
# destination of a sector with no two-sided neighbor, or whose neighbor floors all lie
# below -500.
NO_NEIGHBOR_FLOOR = -500
# A fixpoint bound beyond this is reported as divergent (only an inverting door, which
# rule 4 rejects anyway, can make the interval abstraction walk away).
DIVERGED = 1 << 16

# M3h9 THING census (--things SLOT, --things-markdown). The THINGS lump is an array of
# mapthing_t (doomdata.h:203–210): x, y, angle, type, options as five shorts. Its options
# flags are MTF_EASY 1, MTF_NORMAL 2, MTF_HARD 4 (doomdef.h:140–142) and MTF_AMBUSH 8
# (doomdef.h:145); the single-player exclusion bit 16 has no macro and is tested as the
# literal `mthing->options & 16` in P_SpawnMapThing (p_mobj.c:740–741, skipped only when
# !netgame). Only counts of a slot are ever printed, never a record.
MTF_EASY = 1
MTF_NORMAL = 2
MTF_HARD = 4
MTF_AMBUSH = 8
MTF_NOTSINGLE = 16
# Skill → spawn bit of P_SpawnMapThing (p_mobj.c:743–751): sk_baby 1, sk_nightmare 4,
# else 1 << (gameskill - 1), so sk_easy 1, sk_medium 2, sk_hard 4 over the skill enum of
# doomdef.h:147–154. The census names them easy (skills 1–2), medium (3), hard (4–5).
SKILL_BITS = (("easy", MTF_EASY), ("medium", MTF_NORMAL), ("hard", MTF_HARD))
# Deathmatch starts (type 11) are stored before every other test, at most MAX_DM_STARTS
# = 10 of them (p_mobj.c:717–726, doomstat.h:219); every type <= 4 is written to
# playerstarts[type - 1] and handed to P_SpawnPlayer outside deathmatch (p_mobj.c:728–737),
# which returns unless that player is in the game (p_mobj.c:654–655), so in single player
# only type 1 spawns anything. Both branches precede the skill and type checks, so every
# start record of a slot is handled on every skill, and a type <= 0 would index
# playerstarts out of range.
DM_START = 11
MAX_DM_STARTS = 10
# The kinds P_SpawnMapThing turns into a mobj (the census's `spawned`).
SPAWNED_KINDS = ("monster", "pickup", "barrel", "corpse", "obstacle", "decoration")
# The octants `angle/45` the inherited OriginalDemon spawn admission accepts.
ADMITTED_OCTANTS = range(-4, 4)
PLAYER_STARTS = (1, 2, 3, 4)
# Where mobjinfo[] lives: info.c:1106–4669 holds one brace block per mobjtype_t in the
# enum order of info.h:1162–1303; the fields follow mobjinfo_t (info.h:1305–1331), so
# field 0 is doomednum, 16 radius, 17 height (both N*FRACUNIT) and 21 flags (an MF_
# expression of p_mobj.h:120–199, or 0).
SOURCE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), os.pardir, "linuxdoom-1.10")
MOBJINFO_FIELDS = 23
RADIUS_FIELD = 16
HEIGHT_FIELD = 17
FLAGS_FIELD = 21
# Plain names of every mobjtype_t with a doomednum (the DOOM manual's names; no sprite or
# lump name). Types with doomednum -1 (projectiles, puffs, the player) never spawn from a
# THINGS record and are not listed.
HUMAN_NAMES = {
    "MT_POSSESSED": "zombieman",
    "MT_SHOTGUY": "shotgun guy",
    "MT_VILE": "arch-vile",
    "MT_UNDEAD": "revenant",
    "MT_FATSO": "mancubus",
    "MT_CHAINGUY": "heavy weapon dude",
    "MT_TROOP": "imp",
    "MT_SERGEANT": "demon",
    "MT_SHADOWS": "spectre",
    "MT_HEAD": "cacodemon",
    "MT_BRUISER": "baron of hell",
    "MT_KNIGHT": "hell knight",
    "MT_SKULL": "lost soul",
    "MT_SPIDER": "spider mastermind",
    "MT_BABY": "arachnotron",
    "MT_CYBORG": "cyberdemon",
    "MT_PAIN": "pain elemental",
    "MT_WOLFSS": "wolfenstein ss",
    "MT_KEEN": "commander keen",
    "MT_BOSSBRAIN": "boss brain",
    "MT_BOSSSPIT": "boss shooter",
    "MT_BOSSTARGET": "boss spawn spot",
    "MT_TELEPORTMAN": "teleport landing",
    "MT_BARREL": "exploding barrel",
    "MT_MISC0": "green armor",
    "MT_MISC1": "blue armor",
    "MT_MISC2": "health bonus",
    "MT_MISC3": "armor bonus",
    "MT_MISC4": "blue keycard",
    "MT_MISC5": "red keycard",
    "MT_MISC6": "yellow keycard",
    "MT_MISC7": "yellow skull key",
    "MT_MISC8": "red skull key",
    "MT_MISC9": "blue skull key",
    "MT_MISC10": "stimpack",
    "MT_MISC11": "medikit",
    "MT_MISC12": "soulsphere",
    "MT_INV": "invulnerability",
    "MT_MISC13": "berserk",
    "MT_INS": "partial invisibility",
    "MT_MISC14": "radiation suit",
    "MT_MISC15": "computer area map",
    "MT_MISC16": "light amplification visor",
    "MT_MEGA": "megasphere",
    "MT_CLIP": "ammo clip",
    "MT_MISC17": "box of bullets",
    "MT_MISC18": "rocket",
    "MT_MISC19": "box of rockets",
    "MT_MISC20": "energy cell",
    "MT_MISC21": "energy cell pack",
    "MT_MISC22": "shotgun shells",
    "MT_MISC23": "box of shells",
    "MT_MISC24": "backpack",
    "MT_MISC25": "bfg 9000",
    "MT_CHAINGUN": "chaingun",
    "MT_MISC26": "chainsaw",
    "MT_MISC27": "rocket launcher",
    "MT_MISC28": "plasma gun",
    "MT_SHOTGUN": "shotgun",
    "MT_SUPERSHOTGUN": "super shotgun",
    "MT_MISC29": "tall techno floor lamp",
    "MT_MISC30": "short techno floor lamp",
    "MT_MISC31": "floor lamp",
    "MT_MISC32": "tall green pillar",
    "MT_MISC33": "short green pillar",
    "MT_MISC34": "tall red pillar",
    "MT_MISC35": "short red pillar",
    "MT_MISC36": "short red pillar with skull",
    "MT_MISC37": "short green pillar with heart",
    "MT_MISC38": "evil eye",
    "MT_MISC39": "floating skull rock",
    "MT_MISC40": "gray tree",
    "MT_MISC41": "tall blue firestick",
    "MT_MISC42": "tall green firestick",
    "MT_MISC43": "tall red firestick",
    "MT_MISC44": "short blue firestick",
    "MT_MISC45": "short green firestick",
    "MT_MISC46": "short red firestick",
    "MT_MISC47": "stalagmite",
    "MT_MISC48": "tall techno pillar",
    "MT_MISC49": "candle",
    "MT_MISC50": "candelabra",
    "MT_MISC51": "hanging victim, twitching (solid)",
    "MT_MISC52": "hanging victim, arms out (solid)",
    "MT_MISC53": "hanging victim, one-legged (solid)",
    "MT_MISC54": "hanging pair of legs (solid)",
    "MT_MISC55": "hanging leg (solid)",
    "MT_MISC56": "hanging victim, arms out",
    "MT_MISC57": "hanging pair of legs",
    "MT_MISC58": "hanging victim, one-legged",
    "MT_MISC59": "hanging leg",
    "MT_MISC60": "hanging victim, twitching",
    "MT_MISC61": "dead cacodemon",
    "MT_MISC62": "dead player",
    "MT_MISC63": "dead zombieman",
    "MT_MISC64": "dead demon",
    "MT_MISC65": "dead lost soul (invisible)",
    "MT_MISC66": "dead imp",
    "MT_MISC67": "dead shotgun guy",
    "MT_MISC68": "bloody mess",
    "MT_MISC69": "bloody mess",
    "MT_MISC70": "five skulls shish kebab",
    "MT_MISC71": "pool of blood and flesh",
    "MT_MISC72": "skull on a pole",
    "MT_MISC73": "pile of skulls and candles",
    "MT_MISC74": "impaled human",
    "MT_MISC75": "twitching impaled human",
    "MT_MISC76": "large brown tree",
    "MT_MISC77": "burning barrel",
    "MT_MISC78": "hanging victim, guts removed",
    "MT_MISC79": "hanging victim, guts and brain removed",
    "MT_MISC80": "hanging torso, looking down",
    "MT_MISC81": "hanging torso, open skull",
    "MT_MISC82": "hanging torso, looking up",
    "MT_MISC83": "hanging torso, brain removed",
    "MT_MISC84": "pool of blood",
    "MT_MISC85": "pool of blood",
    "MT_MISC86": "pool of brains",
}
# Corpses and gore (bodies, hanging victims, impaled humans, skull piles, pools): every
# one is a plain decoration to the engine (no SPECIAL, SHOOTABLE or COUNTKILL flag), the
# census only marks them so a slot's dead bodies can be counted.
CORPSE_TYPES = frozenset(f"MT_MISC{n}" for n in range(51, 76)) | frozenset(
    f"MT_MISC{n}" for n in range(78, 87)
)
# The -nomonsters test of P_SpawnMapThing (p_mobj.c:767–773) treats MF_COUNTKILL types
# and MT_SKULL (the lost soul, not counted as a kill) as the monsters; so does the census.
MONSTER_EXTRA = frozenset(("MT_SKULL",))
_MOBJ_TABLE = None


def mobj_table():
    """doomednum → (mobjtype_t name, radius, height, flag names) parsed from info.c.

    The parse is of the committed source only (see the layout notes above the constants):
    the enum of info.h gives the type names in order, each `{ // MT_X ... }` block of
    mobjinfo[] gives the 23 fields after its trailing `// field` comments are stripped.
    Radius and height are N*FRACUNIT, returned as the integer N (map units); flags are
    the MF_ names without the prefix (p_mobj.h:120–199).
    """
    global _MOBJ_TABLE
    if _MOBJ_TABLE is not None:
        return _MOBJ_TABLE
    with open(os.path.join(SOURCE_DIR, "info.h"), encoding="ascii", errors="replace") as handle:
        header = handle.read()
    with open(os.path.join(SOURCE_DIR, "info.c"), encoding="ascii", errors="replace") as handle:
        source = handle.read()
    enum = re.search(r"typedef enum \{(.*?)NUMMOBJTYPES", header, re.S).group(1)
    names = re.findall(r"\bMT_\w+", enum)
    body = source[source.index("mobjinfo_t mobjinfo[NUMMOBJTYPES] = {") :]
    blocks = re.findall(r"\{\s*//\s*(MT_\w+)\s*\n(.*?)\n\s*\}", body, re.S)
    if len(blocks) != len(names):
        raise SystemExit(f"iwad_counts: info.c has {len(blocks)} mobjinfo blocks for {len(names)} types")
    table = {}
    for (name, fields), enum_name in zip(blocks, names):
        if name != enum_name:
            raise SystemExit(f"iwad_counts: info.c block {name} is out of enum order ({enum_name})")
        values = [line.split("//")[0].strip().rstrip(",").strip() for line in fields.strip().split("\n")]
        if len(values) != MOBJINFO_FIELDS:
            raise SystemExit(f"iwad_counts: {name} has {len(values)} mobjinfo fields")
        doomednum = int(values[0])
        if doomednum == -1:
            continue
        radius = int(values[RADIUS_FIELD].replace("*FRACUNIT", ""))
        height = int(values[HEIGHT_FIELD].replace("*FRACUNIT", ""))
        flags = frozenset(f[3:] for f in values[FLAGS_FIELD].split("|") if f.startswith("MF_"))
        if doomednum in table:
            raise SystemExit(f"iwad_counts: doomednum {doomednum} appears twice in info.c")
        table[doomednum] = (name, radius, height, flags)
    _MOBJ_TABLE = table
    return table


def thing_kind(doomednum, info):
    """The census class of a doomednum, in the test order of P_SpawnMapThing."""
    if doomednum == DM_START:
        return "dm start"
    if doomednum <= 4:
        return "player start" if doomednum in PLAYER_STARTS else "non-positive type"
    if info is None:
        return "unknown"  # I_Error "Unknown type" (p_mobj.c:758–761)
    name, _radius, _height, flags = info
    if "COUNTKILL" in flags or name in MONSTER_EXTRA:
        return "monster"
    if "SPECIAL" in flags:
        return "pickup"  # P_TouchSpecialThing items
    if name == "MT_BARREL":
        return "barrel"
    if name in CORPSE_TYPES:
        return "corpse"
    if "SOLID" in flags:
        return "obstacle"
    return "decoration"


def read_wad(path):
    with open(path, "rb") as handle:
        data = handle.read()
    _ident, count, offset = struct.unpack_from("<4sii", data, 0)
    lumps = []
    for index in range(count):
        pos, size, name = struct.unpack_from("<ii8s", data, offset + 16 * index)
        lumps.append((name.rstrip(b"\0").decode("ascii", "replace"), pos, size))
    return data, lumps


def map_slots(lumps):
    return [name for name, _, _ in lumps if MAP_SLOT.fullmatch(name)]


def map_lumps(data, lumps, slot):
    start = next(i for i, (name, _, _) in enumerate(lumps) if name == slot)
    out = {}
    for j in range(1, 11):
        if start + j >= len(lumps):
            break
        name, pos, size = lumps[start + j]
        if name in MAP_LUMPS:
            out[name] = data[pos : pos + size]
    return out


def records(blob, fmt):
    size = struct.calcsize(fmt)
    return [struct.unpack_from(fmt, blob, size * i) for i in range(len(blob) // size)]


class MapCounts:
    """Everything the report prints about one map slot."""

    def __init__(self, slot, lumps):
        self.slot = slot
        vertices = records(lumps["VERTEXES"], "<hh")
        lines = records(lumps["LINEDEFS"], "<HHHHHHH")
        sides = records(lumps["SIDEDEFS"], "<hh8s8s8sH")
        sectors = records(lumps["SECTORS"], "<hh8s8sHHH")
        things = records(lumps["THINGS"], "<hhHHH")
        segs = records(lumps["SEGS"], "<HHHHHh")
        subsectors = records(lumps["SSECTORS"], "<HH")
        nodes = records(lumps["NODES"], "<hhhhhhhhhhhhHH")
        self.counts = dict(
            vertices=len(vertices),
            linedefs=len(lines),
            sidedefs=len(sides),
            sectors=len(sectors),
            things=len(things),
            segs=len(segs),
            subsectors=len(subsectors),
            nodes=len(nodes),
        )
        self.things = things
        self.vertices = vertices
        self.nodes, self.segs, self.subsectors = nodes, segs, subsectors
        self.sectors = sectors
        self.lines = lines
        self.sides = sides
        xs = [v[0] for v in vertices]
        ys = [v[1] for v in vertices]
        self.min_x, self.max_x, self.min_y, self.max_y = min(xs), max(xs), min(ys), max(ys)
        self.span_x = self.max_x - self.min_x
        self.span_y = self.max_y - self.min_y
        self.norm1 = 0
        self.norm2_sq = 0
        self.component = 0
        for a, b, *_ in lines:
            dx = vertices[b][0] - vertices[a][0]
            dy = vertices[b][1] - vertices[a][1]
            self.norm1 = max(self.norm1, abs(dx) + abs(dy))
            self.norm2_sq = max(self.norm2_sq, dx * dx + dy * dy)
            self.component = max(self.component, abs(dx), abs(dy))
        self.node_norm = max((abs(n[2]) + abs(n[3]) for n in nodes), default=0)
        self.max_height = max(abs(h) for s in sectors for h in (s[0], s[1]))
        self.sky_ceilings = sum(1 for s in sectors if s[3].rstrip(b"\0") == b"F_SKY1")
        self.sky_floors = sum(1 for s in sectors if s[2].rstrip(b"\0") == b"F_SKY1")
        self.line_specials = sum(1 for l in lines if l[3])
        self.sector_specials = sum(1 for s in sectors if s[5])
        self.one_sided = [(vertices[l[0]], vertices[l[1]]) for l in lines if l[6] == NO_SIDE]
        self.two_sided = len(lines) - len(self.one_sided)
        self.inconsistent_two_sided = sum(
            1 for l in lines if bool(l[2] & 4) != (l[6] != NO_SIDE)
        )
        needed = math.ceil(len(sectors) * len(sectors) / 8)
        self.reject_bytes = len(lumps.get("REJECT", b""))
        self.reject_ok = self.reject_bytes >= needed
        self._blockmap(lumps["BLOCKMAP"], vertices, lines)
        self._boundary()
        self._certificate()
        self._specials()
        self._movers()

    # -- BLOCKMAP ------------------------------------------------------------------

    def _blockmap(self, blob, vertices, lines):
        self.org_x, self.org_y, self.width, self.height = struct.unpack_from("<hhhh", blob, 0)
        self.words = len(blob) // 2
        self.cells = self.width * self.height
        offsets = [struct.unpack_from("<H", blob, 8 + 2 * i)[0] for i in range(self.cells)]
        cell_lists = []
        self.max_per_cell = 0
        self.listings = 0
        for offset in offsets:
            listed = []
            pos = offset
            while True:
                value = struct.unpack_from("<h", blob, 2 * pos)[0]
                pos += 1
                if value == -1:
                    break
                listed.append(value)
            cell_lists.append(listed)
            self.max_per_cell = max(self.max_per_cell, len(listed))
            self.listings += len(listed)
        self.cell_lists = cell_lists
        listed_in = collections.defaultdict(set)
        for cy in range(self.height):
            for cx in range(self.width):
                for li in cell_lists[cy * self.width + cx]:
                    listed_in[li].add((cx, cy))
        self.missing_listings = 0
        self.lines_with_missing = 0
        self.extra_listings = 0
        for index, (a, b, *_) in enumerate(lines):
            (x1, y1), (x2, y2) = vertices[a], vertices[b]
            bx0 = (min(x1, x2) - self.org_x) // CELL
            bx1 = (max(x1, x2) - self.org_x) // CELL
            by0 = (min(y1, y2) - self.org_y) // CELL
            by1 = (max(y1, y2) - self.org_y) // CELL
            need = set()
            for cy in range(max(by0, 0), min(by1, self.height - 1) + 1):
                for cx in range(max(bx0, 0), min(bx1, self.width - 1) + 1):
                    if segment_meets_closed_cell(
                        x1, y1, x2, y2, self.org_x + cx * CELL, self.org_y + cy * CELL
                    ):
                        need.add((cx, cy))
            got = listed_in.get(index, set())
            missing = need - got
            if missing:
                self.missing_listings += len(missing)
                self.lines_with_missing += 1
            self.extra_listings += len(got - need)
        self.grid_covers = all(
            self.org_x <= x < self.org_x + self.width * CELL
            and self.org_y <= y < self.org_y + self.height * CELL
            for x, y in vertices
        )
        self.window_lines = 0
        for cy in range(max(self.height - 1, 1)):
            for cx in range(max(self.width - 1, 1)):
                distinct = set()
                for wy in (cy, cy + 1):
                    for wx in (cx, cx + 1):
                        if wx < self.width and wy < self.height:
                            distinct.update(cell_lists[wy * self.width + wx])
                self.window_lines = max(self.window_lines, len(distinct))

    # -- C1–C3 ---------------------------------------------------------------------

    def _boundary(self):
        walls = self.one_sided
        degree = collections.Counter()
        for a, b in walls:
            degree[a] += 1
            degree[b] += 1
        self.odd_vertices = sum(1 for d in degree.values() if d % 2)
        self.c1_contacts = 0
        for u in range(len(walls)):
            p1, p2 = walls[u]
            for w in range(u + 1, len(walls)):
                q1, q2 = walls[w]
                if (
                    max(p1[0], p2[0]) < min(q1[0], q2[0])
                    or max(q1[0], q2[0]) < min(p1[0], p2[0])
                    or max(p1[1], p2[1]) < min(q1[1], q2[1])
                    or max(q1[1], q2[1]) < min(p1[1], p2[1])
                ):
                    continue
                if non_endpoint_contact(p1, p2, q1, q2):
                    self.c1_contacts += 1
        self.c3_tested = self.c1_contacts == 0 and self.odd_vertices == 0
        self.c3_failures = 0
        if self.c3_tested:
            scale = 1 << PROBE_SHIFT
            scaled = [((2 * a[0] * scale, 2 * a[1] * scale), (2 * b[0] * scale, 2 * b[1] * scale)) for a, b in walls]
            for a, b in walls:
                dx, dy = b[0] - a[0], b[1] - a[1]
                # front side of sidedef 0 is to the right of (dx, dy): midpoint + (dy, -dx)/2^k
                px = (a[0] + b[0]) * scale + 2 * dy
                py = (a[1] + b[1]) * scale - 2 * dx
                if ray_parity(px, py, scaled) != 1:
                    self.c3_failures += 1
        self.boundary_ok = self.c3_tested and self.c3_failures == 0

    # -- §4.2 ----------------------------------------------------------------------

    def _certificate(self):
        l1 = self.norm1
        diag_sq = self.span_x**2 + self.span_y**2
        self.cert = {
            "i": HITSCAN_REACH * 256 * l1 + l1 + 2 <= I32_MAX and 256 * l1 * HITSCAN_DEN <= I32_MAX,
            "ii": 65_536 * diag_sq * self.norm2_sq <= (I32_MAX - l1 - 2) ** 2,
            "iii": self.span_x + self.span_y < 32_768,
            "iv": max(self.span_x, self.span_y) < SLOPE_DIV_BOUND and self.component < SLOPE_DIV_BOUND,
            "v": (
                self.org_x <= self.min_x
                and self.org_y <= self.min_y
                and self.max_x < self.org_x + CELL * self.width
                and self.max_y < self.org_y + CELL * self.height
                and self.max_x - self.org_x < 32_768
                and self.max_y - self.org_y < 32_768
            ),
            "vi": (l1 + self.node_norm) * max(self.span_x, self.span_y) < 2**31,
            "vii": self.max_height <= HEIGHT_BOUND,
        }
        self.cert_ok = all(self.cert.values())
        self.window_ok = self.window_lines <= MAX_INTERCEPTS
        self.geometry_admitted = (
            self.cert_ok
            and self.boundary_ok
            and self.missing_listings == 0
            and self.grid_covers
            and self.reject_ok
            and self.window_ok
        )

    # -- M3h7 §5.6 -----------------------------------------------------------------

    def _specials(self):
        lines, sides, sectors = self.lines, self.sides, self.sectors
        self.line_types = collections.Counter(l[3] for l in lines if l[3])
        self.sector_types = collections.Counter(s[5] for s in sectors if s[5])
        self.tagged_lines = sum(1 for l in lines if l[4])
        self.tagged_sectors = sum(1 for s in sectors if s[6])
        sky = {i for i, s in enumerate(sectors) if s[3].rstrip(b"\0") == b"F_SKY1"}

        # Stage A (§4.2).
        self.sector_specials_a = all(s[5] in STAGE_A_SECTOR_SPECIALS for s in sectors)
        self.line_specials_a = all(l[3] in STAGE_A_LINE_SPECIALS for l in lines)
        scrollers = [l for l in lines if l[3] == SCROLLER]
        self.scrollers = len(scrollers)
        self.two_sided_scrollers = sum(1 for l in scrollers if l[6] != NO_SIDE)
        self.scrollers_one_sided = self.two_sided_scrollers == 0
        self.scrollers_ok = self.scrollers <= MAXLINEANIMS
        self.light_sectors = sum(1 for s in sectors if s[5] in LIGHT_SECTOR_SPECIALS)
        self.stage_a = (
            self.geometry_admitted
            and self.sector_specials_a
            and self.line_specials_a
            and self.scrollers_one_sided
            and self.scrollers_ok
        )

        # Stage B pure counts (§4.4, §4.6), over E1M1's seven line types.
        by_tag = collections.defaultdict(set)
        for i, s in enumerate(sectors):
            by_tag[s[6]].add(i)
        door, plat, floor = set(), set(), set()
        self.one_sided_dr = 0
        self.tag0_tag_using = 0
        for l in lines:
            special, tag = l[3], l[4]
            if special == DR_DOOR:
                if l[6] == NO_SIDE:
                    self.one_sided_dr += 1
                else:
                    door.add(sides[l[6]][5])
            elif special in TAG_USING:
                if tag == 0:
                    self.tag0_tag_using += 1
                family = door if special in TAGGED_DOOR else plat if special in TAGGED_PLAT else floor
                family.update(by_tag.get(tag, ()))
        movers = door | plat | floor
        self.mover_sectors = len(movers)
        self.door_sectors = len(door)
        self.plat_sectors = len(plat)
        self.floor_sectors = len(floor)
        self.plats_ok = self.plat_sectors <= MAXPLATS
        self.multi_family = sum(1 for i in movers if (i in door) + (i in plat) + (i in floor) > 1)
        self_ref = {
            sides[l[5]][5] for l in lines if l[6] != NO_SIDE and sides[l[5]][5] == sides[l[6]][5]
        }
        self.self_ref_doors = len(door & self_ref)
        self.sky_movers = len(movers & sky)
        self.reusable_switches = sum(1 for l in lines if l[3] in REUSABLE_SWITCHES)
        self.switches_ok = self.reusable_switches <= MAXBUTTONS
        # §4.6: distinct two-sided special lines in any 2 x 2 window, same windows as the
        # slide-window rule above.
        crossing = {i for i, l in enumerate(lines) if l[3] and l[6] != NO_SIDE}
        self.window_specials = 0
        for cy in range(max(self.height - 1, 1)):
            for cx in range(max(self.width - 1, 1)):
                distinct = set()
                for wy in (cy, cy + 1):
                    for wx in (cx, cx + 1):
                        if wx < self.width and wy < self.height:
                            distinct.update(crossing.intersection(self.cell_lists[wy * self.width + wx]))
                self.window_specials = max(self.window_specials, len(distinct))
        self.window_specials_ok = self.window_specials <= MAXSPECIALCROSS

    # -- M3h8 Stage B --------------------------------------------------------------

    def _movers(self):
        lines, sides, sectors = self.lines, self.sides, self.sectors
        n = len(sectors)
        front = [sides[l[5]][5] if l[5] != NO_SIDE else None for l in lines]
        back = [sides[l[6]][5] if l[6] != NO_SIDE else None for l in lines]
        # sector->lines of P_GroupLines (p_setup.c:520–551): every line whose front or
        # back sector is the sector, once each, in line order.
        sector_lines = [[] for _ in range(n)]
        for i in range(len(lines)):
            for s in {front[i], back[i]} - {None}:
                sector_lines[s].append(i)
        # getNextSector (p_spec.c:250–261): no neighbor unless ML_TWOSIDED (flag 4); the
        # back sector when the sector is in front (itself for a self-referencing line),
        # else the front sector.
        self.neighbors = []
        for s in range(n):
            out = []
            for i in sector_lines[s]:
                if not lines[i][2] & 4:
                    continue
                other = back[i] if front[i] == s else front[i]
                if other is not None:
                    out.append(other)
            self.neighbors.append(out)
        by_tag = collections.defaultdict(list)
        for i, s in enumerate(sectors):
            by_tag[s[6]].append(i)
        sky = {i for i, s in enumerate(sectors) if s[3].rstrip(b"\0") == b"F_SKY1"}

        # (2) tag-using lines, their kind and their targets (P_FindSectorFromLineTag,
        # tag 0 selects every untagged sector).
        self.kind_of = lambda special: LINE_KINDS.get(special, ("unknown", "?"))
        self.tag_using_lines = []  # (special, kind, trigger, tag == 0, targets)
        self.tagged_untagged_kind = 0  # nonzero tag on a line whose special ignores tags
        families = collections.defaultdict(set)  # family -> sectors
        dr_back = set()
        for i, l in enumerate(lines):
            special, tag = l[3], l[4]
            if not special:
                continue
            kind, trigger = self.kind_of(special)
            if kind == "manual door":
                if back[i] is not None:
                    families["door"].add(back[i])
                    dr_back.add(back[i])
                if tag:
                    self.tagged_untagged_kind += 1
                continue
            if kind in UNTAGGED_KINDS or kind == "unknown":
                if tag:
                    self.tagged_untagged_kind += 1
                continue
            targets = by_tag.get(tag, [])
            self.tag_using_lines.append((special, kind, trigger, tag == 0, list(targets)))
            for family in MOVER_FAMILIES.get(kind, ()):
                families[family].update(targets)
        self.families = {k: set(v) for k, v in families.items()}
        self.tag_using_count = len(self.tag_using_lines)
        self.tag_using_no_target = sum(1 for t in self.tag_using_lines if not t[4])
        self.tag_using_hits = sum(len(t[4]) for t in self.tag_using_lines)
        self.tag_using_max_targets = max((len(t[4]) for t in self.tag_using_lines), default=0)
        self.tag0_lines_all = sum(1 for t in self.tag_using_lines if t[3])

        # (3) manual door lines: two-sidedness of every DR/D1 line by type.
        self.manual_doors = collections.Counter()
        self.manual_doors_one_sided = collections.Counter()
        for i, l in enumerate(lines):
            if self.kind_of(l[3])[0] == "manual door":
                self.manual_doors[l[3]] += 1
                if back[i] is None:
                    self.manual_doors_one_sided[l[3]] += 1

        # (5) mover families over every special, and the admitted-set families.
        all_movers = set().union(*self.families.values()) if self.families else set()
        self.all_mover_sectors = len(all_movers)
        self.family_sizes = {k: len(v) for k, v in sorted(self.families.items())}
        self.all_multi_family = sum(
            1 for s in all_movers if sum(s in v for v in self.families.values()) > 1
        )
        non_door_tagged = set()
        for _special, kind, _t, _z, targets in self.tag_using_lines:
            if any(f != "door" for f in MOVER_FAMILIES.get(kind, ())):
                non_door_tagged.update(targets)
        self.dr_vs_non_door = len(dr_back & non_door_tagged)
        self.all_plat_sectors = len(self.families.get("plat", ()))
        self.all_sky_movers = len(all_movers & sky)

        # (7) self-referencing lines (front and back sector equal) of door sectors.
        doors_all = self.families.get("door", set())
        self.self_ref_door_lines = sum(
            1 for i in range(len(lines)) if back[i] is not None and front[i] == back[i] and front[i] in doors_all
        )
        # (8) / rule 3: mover sectors with no getNextSector neighbor (a door top of
        # MAXINT - 4, P_FindLowestCeilingSurrounding p_spec.c:387).
        self.doors_no_neighbor = sum(1 for s in doors_all if not self.neighbors[s])
        self.movers_no_neighbor = sum(1 for s in all_movers if not self.neighbors[s])

        # Line specials outside the Stage B set (admission impossible anyway).
        self.outside_stage_b = collections.Counter(
            l[3] for l in lines if l[3] not in STAGE_B_LINE_SPECIALS
        )
        self.only_admitted = not self.outside_stage_b

        # (9) reachable heights over the admitted set only.
        self._reachable(lines, back, by_tag, sectors)

        # (13) the Stage B load rules of §4.4 plus the Stage A rules (a superset).
        failures = []
        if not self.geometry_admitted:
            failures.append("geometry (M3h6)")
        if not self.sector_specials_a:
            failures.append("sector specials")
        if not self.only_admitted:
            failures.append("1: line specials outside the admitted set")
        if self.one_sided_dr:
            failures.append("1: one-sided DR line")
        if self.multi_family:
            failures.append("2: shared mover families")
        if self.adm_no_neighbor:
            failures.append("3: mover sector without a two-sided neighbor")
        if self.reach_diverged or not self.reach_within:
            failures.append("4: reachable height beyond ±512")
        if self.inverting_doors:
            failures.append("4: inverting door")
        if not (self.plats_ok and self.switches_ok and self.scrollers_ok):
            failures.append("5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS")
        if self.two_sided_scrollers:
            failures.append("two-sided scroller (Stage A)")
        if not self.window_specials_ok:
            failures.append("6: spechit window")
        self.stage_b_failures = failures
        self.stage_b = not failures

    def _reachable(self, lines, back, by_tag, sectors):
        """§4.4 rule 4: interval fixpoint of every floor and ceiling under the admitted set.

        Heights are map units (every admitted destination is an integer unit height and
        every speed an integer, so FRACUNIT never enters). Rules, each from its C body:
        - doors 1 (EV_VerticalDoor, p_doors.c:497–498) and 63 (EV_DoDoor normal,
          p_doors.c:326–330): top = P_FindLowestCeilingSurrounding - 4; the ceiling goes
          down to the floor (T_VerticalDoor, p_doors.c:118–123) and up to the top, so it
          ranges over [floor, top]; 109 (blazeOpen, p_doors.c:320–323) only rises to top;
        - lift 88 (EV_DoPlat downWaitUpStay, p_plats.c:207–218): floor over
          [min(lowest neighbor floor, own floor), own floor];
        - 38 lowerFloorToLowest (p_floor.c:299–305): down to P_FindLowestFloorSurrounding
          (which starts at the own floor, p_spec.c:274);
        - 36 turboLower (p_floor.c:307–315): P_FindHighestFloorSurrounding (starts at -500,
          p_spec.c:295) plus 8 unless it equals the current floor; T_MovePlane's down branch
          sets the floor to dest at once when floor - speed < dest (p_floor.c:68–71), so a
          destination above the floor is a one-tic jump-up.
        A neighbor is a getNextSector result (ML_TWOSIDED only, itself for a
        self-referencing line). DR, SR and WR rules repeat to a fixpoint; the W1 lines (109,
        36, 38) each fire at most once, so after the first closure there are at most as many
        rounds as W1 lines, each letting every W1 line fire once from the current intervals
        and closing again (every real order of W1 firings is covered round by round).
        """
        floor = [[s[0], s[0]] for s in sectors]
        ceil = [[s[1], s[1]] for s in sectors]
        doors_normal, doors_open, lifts, lowest, turbo = set(), set(), set(), set(), set()
        w1_lines = []
        for i, l in enumerate(lines):
            special, tag = l[3], l[4]
            if special == DR_DOOR and back[i] is not None:
                doors_normal.add(back[i])
            elif special == 63:
                doors_normal.update(by_tag.get(tag, ()))
            elif special == 88:
                lifts.update(by_tag.get(tag, ()))
            if special in W1_ADMITTED:
                w1_lines.append((special, tuple(by_tag.get(tag, ()))))
        nb = self.neighbors
        movers = doors_normal | lifts | {s for _, t in w1_lines for s in t}
        self.adm_no_neighbor = sum(1 for s in movers if not nb[s])
        diverged = False

        def widen(iv, lo, hi):
            changed = lo < iv[0] or hi > iv[1]
            iv[0], iv[1] = min(iv[0], lo), max(iv[1], hi)
            return changed

        def door_top(s):
            # P_FindLowestCeilingSurrounding - 4 over the neighbors' ceiling intervals.
            return min(ceil[o][0] for o in nb[s]) - 4, min(ceil[o][1] for o in nb[s]) - 4

        def highest_floor(s):
            # P_FindHighestFloorSurrounding over the neighbors' floor intervals, folded
            # from -500 (p_spec.c:297-316) as the source does, also when every neighbor
            # floor lies below it.
            return (max([NO_NEIGHBOR_FLOOR] + [floor[o][0] for o in nb[s]]),
                    max([NO_NEIGHBOR_FLOOR] + [floor[o][1] for o in nb[s]]))

        def apply(special, s):
            if not nb[s]:
                return False  # rule 3 failure: MAXINT - 4 top, counted separately
            if special in (1, 63):
                lo, hi = door_top(s)
                return widen(ceil[s], min(floor[s][0], lo), hi)
            if special == 109:
                lo, hi = door_top(s)
                return widen(ceil[s], lo, hi)
            if special == 88:
                return widen(floor[s], min(floor[o][0] for o in nb[s]), floor[s][1])
            if special == 38:
                return widen(floor[s], min(floor[o][0] for o in nb[s]), floor[s][1])
            if special == 36:
                hlo, hhi = highest_floor(s)
                if hlo == hhi == floor[s][0] == floor[s][1]:
                    return False  # destination equals the floor: no +8, no movement
                return widen(floor[s], hlo + 8, hhi + 8)
            return False

        def bounded():
            return all(-DIVERGED < v < DIVERGED for iv in floor + ceil for v in iv)

        def close():
            nonlocal diverged
            while not diverged:
                changed = False
                for s in sorted(doors_normal):
                    changed |= apply(1, s)
                for s in sorted(lifts):
                    changed |= apply(88, s)
                if not bounded():
                    diverged = True
                if not changed:
                    return

        close()
        for _ in range(len(w1_lines)):
            if diverged:
                break
            changed = False
            for special, targets in w1_lines:
                for s in targets:
                    changed |= apply(special, s)
            close()
            if not changed:
                break
        self.reach_diverged = diverged or not bounded()
        values = [v for iv in floor + ceil for v in iv]
        self.reach_min, self.reach_max = min(values), max(values)
        self.reach_within = -HEIGHT_BOUND <= self.reach_min and self.reach_max <= HEIGHT_BOUND
        self.reach_floor = floor
        self.reach_ceil = ceil
        self.reach_moved = sum(
            1 for s in range(len(sectors)) if floor[s][0] != floor[s][1] or ceil[s][0] != ceil[s][1]
        )
        # Inverting doors: the smallest possible top below the floor (taken at its highest
        # reachable value, the conservative reading when the floor itself can move).
        door_sectors = doors_normal | {s for sp, t in w1_lines if sp == 109 for s in t}
        self.adm_door_sectors = len(door_sectors)
        self.inverting_doors = sum(1 for s in door_sectors if nb[s] and door_top(s)[0] < floor[s][1])
        # One-tic plane steps: the admitted speeds, and the turbo jump-up
        # (destination - floor when the destination lies above the floor).
        self.turbo_jump = 0
        for special, targets in w1_lines:
            if special == 36:
                for s in targets:
                    if nb[s]:
                        hlo, hhi = highest_floor(s)
                        if hlo == hhi == floor[s][0] == floor[s][1]:
                            continue  # no +8, no movement
                        self.turbo_jump = max(self.turbo_jump, hhi + 8 - floor[s][0])
        # A door whose ceiling lies above its top drops to the top in one tic: T_MovePlane's
        # ceiling-up branch sets the ceiling to dest once ceiling + speed > dest
        # (p_floor.c:167–181), whatever the direction of the difference.
        self.door_drop = 0
        for s in door_sectors:
            if nb[s]:
                self.door_drop = max(self.door_drop, ceil[s][1] - door_top(s)[0])
        used = {l[3] for l in lines} & set(ADMITTED_SPEED)
        self.max_plane_step = max([ADMITTED_SPEED[sp] for sp in used] + [self.turbo_jump, self.door_drop])
        # Δ of (B2): the largest one-tic rise of a floor or drop of a ceiling (the steps that
        # can leave a thing below its floorz): a closing door 2, a rising lift 4, a turbo
        # jump-up, a door ceiling dropping to its top. Lowering floors and rising ceilings
        # never do.
        rises = [2] if used & {1, 63} else []
        rises += [4] if 88 in used else []
        self.vprime_delta = max(rises + [self.turbo_jump, self.door_drop])

    # -- M3h9 THING census ---------------------------------------------------------

    def things_census(self):
        """Counts of the THINGS lump under P_SpawnMapThing (p_mobj.c:708–797), single player.

        Per doomednum (`rows`): total records; records flagged MTF_EASY, MTF_NORMAL, MTF_HARD;
        records with MTF_AMBUSH; records with bit 16; and the records P_SpawnMapThing handles
        on each skill: a start (type 11 or <= 4) is stored on every skill before the flag
        tests (p_mobj.c:717–737); any other record is dropped when bit 16 is set (:740–741)
        or its skill bit is clear (:750–751), then looked up by doomednum (:754–761) and
        spawned (:784). Per skill (`skills`): the handled counts by kind, totalkills and
        totalitems (:789–792). The census is computed once, on demand, and never prints.
        """
        if hasattr(self, "thing_rows"):
            return
        table = mobj_table()
        rows = {}
        for _x, _y, _angle, dtype, options in self.things:
            doomednum = dtype if dtype < 0x8000 else dtype - 0x10000  # mapthing_t.type is a short
            row = rows.get(doomednum)
            if row is None:
                info = table.get(doomednum)
                row = rows[doomednum] = dict(
                    doomednum=doomednum,
                    info=info,
                    kind=thing_kind(doomednum, info),
                    total=0,
                    easy=0,
                    medium=0,
                    hard=0,
                    ambush=0,
                    notsingle=0,
                    spawn={skill: 0 for skill, _bit in SKILL_BITS},
                )
            row["total"] += 1
            for skill, bit in SKILL_BITS:
                if options & bit:
                    row[skill] += 1
            if options & MTF_AMBUSH:
                row["ambush"] += 1
            if options & MTF_NOTSINGLE:
                row["notsingle"] += 1
            if row["kind"] in ("dm start", "player start", "non-positive type"):
                for skill, _bit in SKILL_BITS:
                    row["spawn"][skill] += 1
                continue
            if options & MTF_NOTSINGLE:
                continue
            for skill, bit in SKILL_BITS:
                if options & bit:
                    row["spawn"][skill] += 1
        self.thing_rows = [rows[k] for k in sorted(rows)]
        self.thing_kinds = collections.Counter(r["kind"] for r in self.thing_rows)
        skills = {}
        for skill, _bit in SKILL_BITS:
            s = collections.Counter()
            for r in self.thing_rows:
                n = r["spawn"][skill]
                if not n:
                    continue
                kind = r["kind"]
                s[kind] += n
                if kind in SPAWNED_KINDS:
                    s["spawned"] += n
                    flags = r["info"][3]
                    if kind in ("corpse", "obstacle", "decoration"):
                        s["solid decorations" if "SOLID" in flags else "non-solid decorations"] += n
                        if kind == "corpse":
                            s["solid corpses" if "SOLID" in flags else "non-solid corpses"] += n
                    if "COUNTKILL" in flags:
                        s["totalkills"] += n
                    if "COUNTITEM" in flags:
                        s["totalitems"] += n
                elif kind == "unknown":
                    s["unknown"] += n  # reaches I_Error on this skill
            skills[skill] = s
        self.thing_skills = skills
        # Angle octants of the spawned records: P_SpawnMapThing sets
        # `mobj->angle = ANG45 * (mthing->angle/45)` (p_mobj.c:794) with C's `int`
        # division of the `short` angle, truncating toward zero; the inherited
        # OriginalDemon admission refuses an octant outside -4..3 (MapSpawnError::Angle).
        octants = {skill: collections.Counter() for skill, _bit in SKILL_BITS}
        for _x, _y, angle, dtype, options in self.things:
            doomednum = dtype if dtype < 0x8000 else dtype - 0x10000
            if rows[doomednum]["kind"] not in SPAWNED_KINDS or options & MTF_NOTSINGLE:
                continue
            signed = angle if angle < 0x8000 else angle - 0x10000
            octant = signed // 45 if signed >= 0 else -(-signed // 45)
            for skill, bit in SKILL_BITS:
                if options & bit:
                    octants[skill][octant] += 1
        self.thing_octants = octants
        starts = collections.Counter()
        for r in self.thing_rows:
            if r["kind"] == "player start":
                starts["player 1" if r["doomednum"] == 1 else "players 2-4"] += r["total"]
            elif r["kind"] in ("dm start", "non-positive type", "unknown"):
                starts[r["kind"]] += r["total"]
        self.thing_starts = starts
        self.dm_stored = min(starts["dm start"], MAX_DM_STARTS)
        self.dm_dropped = starts["dm start"] - self.dm_stored
        self.unknown_doomednums = sum(1 for r in self.thing_rows if r["kind"] == "unknown")
        self.barrel_rows = [r for r in self.thing_rows if r["kind"] == "barrel"]
        self.solid_doomednums = sorted(
            r["doomednum"] for r in self.thing_rows if r["info"] is not None and "SOLID" in r["info"][3]
        )

    # -- §9 ------------------------------------------------------------------------

    def readiness(self):
        uv = [t for t in self.things if (t[4] & 4) and not (t[4] & 16)]
        by_type = collections.Counter(t[3] for t in uv)
        starts = collections.Counter(t[3] for t in self.things if t[3] in (1, 2, 3, 4, 11))
        line_specials = collections.Counter(l[3] for l in self.lines if l[3])
        sector_specials = collections.Counter(s[5] for s in self.sectors if s[5])
        flags = collections.Counter()
        names = ("BLOCKING", "BLOCKMONSTERS", "TWOSIDED", "DONTPEGTOP", "DONTPEGBOTTOM", "SECRET", "SOUNDBLOCK", "DONTDRAW", "MAPPED")
        for l in self.lines:
            for bit, name in enumerate(names):
                if l[2] & (1 << bit):
                    flags[name] += 1
        floors = [s[0] for s in self.sectors]
        ceilings = [s[1] for s in self.sectors]
        return dict(
            uv_single_player_things=len(uv),
            uv_things_by_doomednum=dict(sorted(by_type.items())),
            start_records_by_type=dict(sorted(starts.items())),
            line_specials_by_type=dict(sorted(line_specials.items())),
            tagged_lines=sum(1 for l in self.lines if l[4]),
            sector_specials_by_type=dict(sorted(sector_specials.items())),
            tagged_sectors=sum(1 for s in self.sectors if s[6]),
            sky_ceilings=self.sky_ceilings,
            sky_floors=self.sky_floors,
            line_flags=dict(flags),
            inconsistent_two_sided=self.inconsistent_two_sided,
            floor_range=(min(floors), max(floors)),
            ceiling_range=(min(ceilings), max(ceilings)),
            one_sided_walls=len(self.one_sided),
            same_sector_two_sided=sum(
                1 for l in self.lines if l[6] != NO_SIDE and self.sides[l[5]][5] == self.sides[l[6]][5]
            ),
        )


def segment_meets_closed_cell(x1, y1, x2, y2, cx, cy):
    """Closed segment meets the closed square [cx, cx+128] × [cy, cy+128] (separating axes)."""
    x0, y0, xe, ye = cx, cy, cx + CELL, cy + CELL
    if max(x1, x2) < x0 or min(x1, x2) > xe or max(y1, y2) < y0 or min(y1, y2) > ye:
        return False
    dx, dy = x2 - x1, y2 - y1
    if dx == 0 or dy == 0:
        return True
    signs = [dx * (yy - y1) - dy * (xx - x1) for xx in (x0, xe) for yy in (y0, ye)]
    return min(signs) <= 0 <= max(signs)


def orient(p, q, r):
    return (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])


def on_segment(p, q, r):
    return min(p[0], q[0]) <= r[0] <= max(p[0], q[0]) and min(p[1], q[1]) <= r[1] <= max(p[1], q[1])


def non_endpoint_contact(p1, p2, q1, q2):
    shared = {p1, p2} & {q1, q2}
    if len(shared) == 2:
        return True
    o1, o2, o3, o4 = orient(p1, p2, q1), orient(p1, p2, q2), orient(q1, q2, p1), orient(q1, q2, p2)
    if o1 and o2 and o3 and o4 and (o1 > 0) != (o2 > 0) and (o3 > 0) != (o4 > 0):
        return True
    for p, q, r in ((p1, p2, q1), (p1, p2, q2), (q1, q2, p1), (q1, q2, p2)):
        if orient(p, q, r) == 0 and on_segment(p, q, r) and r not in shared:
            return True
    return False


def segment_meets_box(a, b, lo, hi):
    """Closed segment ab meets the closed box [lo, hi] (the Rust `segment_meets_box`, exact)."""
    if max(a[0], b[0]) < lo[0] or min(a[0], b[0]) > hi[0] or max(a[1], b[1]) < lo[1] or min(a[1], b[1]) > hi[1]:
        return False
    corners = [orient(a, b, q) for q in (lo, (hi[0], lo[1]), hi, (lo[0], hi[1]))]
    return not (all(c > 0 for c in corners) or all(c < 0 for c in corners))


def ray_parity(px, py, walls):
    """Parity of crossings of the ray from (px, py) toward +x with the scaled walls (exact)."""
    crossings = 0
    for (ax, ay), (bx, by) in walls:
        if (ay > py) != (by > py):
            # crossing abscissa xc = ax + (bx - ax) * (py - ay) / (by - ay) > px
            num = (bx - ax) * (py - ay)
            den = by - ay
            if den > 0:
                if ax * den + num > px * den:
                    crossings += 1
            elif ax * den + num < px * den:
                crossings += 1
    return crossings % 2


def flag(value):
    return "pass" if value else "FAIL"


def text_report(counts):
    c = counts.counts
    print(f"== {counts.slot} ==")
    print(
        f"vertices {c['vertices']} linedefs {c['linedefs']} sidedefs {c['sidedefs']} sectors {c['sectors']} "
        f"things {c['things']} segs {c['segs']} subsectors {c['subsectors']} nodes {c['nodes']}"
    )
    print(
        f"span {counts.span_x} x {counts.span_y}; L1 {counts.norm1}; L2^2 {counts.norm2_sq}; "
        f"C {counts.component}; N {counts.node_norm}; max|height| {counts.max_height}"
    )
    print(
        f"blockmap {counts.width}x{counts.height} = {counts.cells} cells, {counts.words} words, "
        f"max {counts.max_per_cell} lines per cell, {counts.listings} listings; "
        f"closed-cell missing {counts.missing_listings} on {counts.lines_with_missing} lines, extra {counts.extra_listings}; "
        f"grid covers vertices {flag(counts.grid_covers)}; 2x2 window max {counts.window_lines} ({flag(counts.window_ok)})"
    )
    print(f"reject {counts.reject_bytes} bytes {flag(counts.reject_ok)}")
    c3 = f"{counts.c3_failures} failing" if counts.c3_tested else "not tested"
    print(
        f"one-sided walls {len(counts.one_sided)}, two-sided {counts.two_sided}; C1 contacts {counts.c1_contacts}; "
        f"C2 odd vertices {counts.odd_vertices}; C3 {c3}; boundary {flag(counts.boundary_ok)}"
    )
    cert = " ".join(f"({k}) {flag(v)}" for k, v in counts.cert.items())
    print(f"certificate {cert}; all {flag(counts.cert_ok)}")
    print(
        f"sky ceilings {counts.sky_ceilings}, sky floors {counts.sky_floors}, line specials {counts.line_specials}, "
        f"sector specials {counts.sector_specials}; geometry admitted {flag(counts.geometry_admitted)}"
    )
    specials_text(counts)
    print()


def types(counter):
    """Type → count as `type:count` pairs in type order, or `none`."""
    return " ".join(f"{k}:{v}" for k, v in sorted(counter.items())) or "none"


def specials_text(counts):
    print(f"line special types {types(counts.line_types)}; tagged lines {counts.tagged_lines}")
    print(f"sector special types {types(counts.sector_types)}; tagged sectors {counts.tagged_sectors}")
    print(
        f"stage A: sector specials in the Stage A set {flag(counts.sector_specials_a)}; "
        f"line specials in {{0, 48}} {flag(counts.line_specials_a)}; "
        f"scrollers {counts.scrollers}, two-sided {counts.two_sided_scrollers}, every scroller one-sided {flag(counts.scrollers_one_sided)}, "
        f"≤ {MAXLINEANIMS} {flag(counts.scrollers_ok)}; light-thinker sectors {counts.light_sectors}; "
        f"sky ceilings {counts.sky_ceilings}, sky floors {counts.sky_floors}; stage A all {flag(counts.stage_a)}"
    )
    print(
        f"stage B: mover sectors {counts.mover_sectors} (door {counts.door_sectors}, plat {counts.plat_sectors}, "
        f"floor {counts.floor_sectors}); plat sectors ≤ {MAXPLATS} {flag(counts.plats_ok)}; "
        f"reusable switch lines {counts.reusable_switches} (≤ {MAXBUTTONS} {flag(counts.switches_ok)}); "
        f"one-sided DR lines {counts.one_sided_dr}; sectors in more than one mover family {counts.multi_family}; "
        f"door sectors with a self-referencing line {counts.self_ref_doors}; "
        f"tag-0 lines with tag-using specials {counts.tag0_tag_using}; sky sectors that are mover sectors {counts.sky_movers}"
    )
    print(
        f"stage B: 2x2 window max two-sided special lines {counts.window_specials} "
        f"(≤ {MAXSPECIALCROSS} {flag(counts.window_specials_ok)})"
    )
    movers_text(counts)


MARKDOWN_COLUMNS = (
    "Slot",
    "Vertices",
    "Linedefs",
    "Sidedefs",
    "Sectors",
    "THINGs",
    "Segs",
    "Subsectors",
    "Nodes",
    "Cells",
    "Words",
    "Max lines/cell",
    "Span x",
    "Span y",
    "L1",
    "Max height",
    "2x2 window",
    "C1",
    "C2 odd",
    "C3",
    "Missing listings",
    "REJECT",
    "(i)",
    "(ii)",
    "(iii)",
    "(iv)",
    "(v)",
    "(vi)",
    "(vii)",
    "Boundary",
    "Geometry admitted",
)


def markdown_row(counts):
    c = counts.counts
    c3 = str(counts.c3_failures) if counts.c3_tested else "n/t"
    cells = [
        counts.slot,
        c["vertices"],
        c["linedefs"],
        c["sidedefs"],
        c["sectors"],
        c["things"],
        c["segs"],
        c["subsectors"],
        c["nodes"],
        counts.cells,
        counts.words,
        counts.max_per_cell,
        counts.span_x,
        counts.span_y,
        counts.norm1,
        counts.max_height,
        counts.window_lines,
        counts.c1_contacts,
        counts.odd_vertices,
        c3,
        counts.missing_listings,
        flag(counts.reject_ok),
        *(flag(counts.cert[k]) for k in ("i", "ii", "iii", "iv", "v", "vi", "vii")),
        flag(counts.boundary_ok),
        flag(counts.geometry_admitted),
    ]
    return "| " + " | ".join(str(x) for x in cells) + " |"


def markdown_report(all_counts):
    print("| " + " | ".join(MARKDOWN_COLUMNS) + " |")
    print("| " + " | ".join("---" for _ in MARKDOWN_COLUMNS) + " |")
    for counts in all_counts:
        print(markdown_row(counts))
    print()
    summary(all_counts, prefix="")


def summary(all_counts, prefix="SUMMARY "):
    n = len(all_counts)
    def count(pred):
        return sum(1 for c in all_counts if pred(c))
    print(f"{prefix}maps {n}")
    for key in ("i", "ii", "iii", "iv", "v", "vi", "vii"):
        print(f"{prefix}certificate ({key}) holds for {count(lambda c, k=key: c.cert[k])} of {n}")
    print(f"{prefix}certificate (i)–(vii) all hold for {count(lambda c: c.cert_ok)} of {n}")
    print(f"{prefix}C1 holds for {count(lambda c: c.c1_contacts == 0)} of {n}")
    print(f"{prefix}C2 holds for {count(lambda c: c.odd_vertices == 0)} of {n}")
    print(f"{prefix}C3 holds (where tested) for {count(lambda c: c.boundary_ok)} of {n}")
    print(f"{prefix}closed-cell BLOCKMAP rule holds for {count(lambda c: c.missing_listings == 0)} of {n}")
    print(f"{prefix}BLOCKMAP covers every vertex for {count(lambda c: c.grid_covers)} of {n}")
    print(f"{prefix}REJECT length sufficient for {count(lambda c: c.reject_ok)} of {n}")
    print(f"{prefix}2x2 window ≤ {MAX_INTERCEPTS} lines for {count(lambda c: c.window_ok)} of {n}; maximum {max(c.window_lines for c in all_counts)}")
    print(f"{prefix}geometry admitted under every rule for {count(lambda c: c.geometry_admitted)} of {n}")
    print(f"{prefix}maxima: linedefs {max(c.counts['linedefs'] for c in all_counts)}, sectors {max(c.counts['sectors'] for c in all_counts)}, "
          f"cells {max(c.cells for c in all_counts)}, words {max(c.words for c in all_counts)}, L1 {max(c.norm1 for c in all_counts)}, "
          f"one-sided walls {max(len(c.one_sided) for c in all_counts)}")


def specials_summary(all_counts, prefix="SUMMARY "):
    n = len(all_counts)
    def count(pred):
        return sum(1 for c in all_counts if pred(c))
    def total(attr):
        return sum(getattr(c, attr) for c in all_counts)
    def most(attr):
        return max(getattr(c, attr) for c in all_counts)
    line_types = sum((c.line_types for c in all_counts), collections.Counter())
    sector_types = sum((c.sector_types for c in all_counts), collections.Counter())
    print(f"{prefix}line special types over all maps {types(line_types)}")
    print(f"{prefix}sector special types over all maps {types(sector_types)}")
    print(f"{prefix}stage A: sector specials in the Stage A set for {count(lambda c: c.sector_specials_a)} of {n}")
    print(f"{prefix}stage A: line specials in {{0, 48}} for {count(lambda c: c.line_specials_a)} of {n}")
    print(f"{prefix}stage A: every scroller one-sided for {count(lambda c: c.scrollers_one_sided)} of {n}; two-sided scrollers {total('two_sided_scrollers')}")
    print(f"{prefix}stage A: scrollers ≤ {MAXLINEANIMS} for {count(lambda c: c.scrollers_ok)} of {n}; maximum {most('scrollers')}")
    print(f"{prefix}stage A: maxima: light-thinker sectors {most('light_sectors')}, sky ceilings {most('sky_ceilings')}, sky floors {most('sky_floors')}")
    print(f"{prefix}stage A all (with geometry admitted) for {count(lambda c: c.stage_a)} of {n}")
    print(f"{prefix}stage B: maxima: mover sectors {most('mover_sectors')}, plat sectors {most('plat_sectors')}, "
          f"reusable switch lines {most('reusable_switches')}, 2x2 window two-sided special lines {most('window_specials')}")
    print(f"{prefix}stage B: plat sectors ≤ {MAXPLATS} for {count(lambda c: c.plats_ok)} of {n}; "
          f"reusable switch lines ≤ {MAXBUTTONS} for {count(lambda c: c.switches_ok)} of {n}; "
          f"2x2 window ≤ {MAXSPECIALCROSS} two-sided special lines for {count(lambda c: c.window_specials_ok)} of {n}")
    for attr, label in (
        ("one_sided_dr", "one-sided DR lines"),
        ("multi_family", "sectors in more than one mover family"),
        ("self_ref_doors", "door sectors with a self-referencing line"),
        ("tag0_tag_using", "tag-0 lines with tag-using specials"),
        ("sky_movers", "sky sectors that are mover sectors"),
    ):
        print(f"{prefix}stage B: {label} {total(attr)} in all; none for {count(lambda c, a=attr: getattr(c, a) == 0)} of {n}")
    print(f"{prefix}stage B: reachable heights and the load rules: see --movers-markdown")


SPECIALS_COLUMNS = (
    "Slot",
    "Line special types",
    "Tagged lines",
    "Sector special types",
    "Tagged sectors",
    "Sector specials A",
    "Line specials A",
    "Scrollers",
    "Two-sided scrollers",
    "Scrollers ≤ 64",
    "Light sectors",
    "Sky ceilings",
    "Sky floors",
    "Stage A all",
    "Mover sectors",
    "Plat sectors",
    "Reusable switches",
    "One-sided DR",
    "Multi-family sectors",
    "Self-ref door sectors",
    "Tag-0 tag-using lines",
    "Sky mover sectors",
    "2x2 two-sided specials",
)


def specials_markdown_report(all_counts):
    print("| " + " | ".join(SPECIALS_COLUMNS) + " |")
    print("| " + " | ".join("---" for _ in SPECIALS_COLUMNS) + " |")
    for c in all_counts:
        cells = [
            c.slot,
            types(c.line_types),
            c.tagged_lines,
            types(c.sector_types),
            c.tagged_sectors,
            flag(c.sector_specials_a),
            flag(c.line_specials_a),
            c.scrollers,
            c.two_sided_scrollers,
            flag(c.scrollers_ok),
            c.light_sectors,
            c.sky_ceilings,
            c.sky_floors,
            flag(c.stage_a),
            c.mover_sectors,
            c.plat_sectors,
            c.reusable_switches,
            c.one_sided_dr,
            c.multi_family,
            c.self_ref_doors,
            c.tag0_tag_using,
            c.sky_movers,
            c.window_specials,
        ]
        print("| " + " | ".join(str(x) for x in cells) + " |")
    print()
    specials_summary(all_counts, prefix="")


def reach_text(c):
    if c.reach_diverged:
        return "diverged"
    return f"[{c.reach_min}, {c.reach_max}]"


def movers_text(c):
    """The M3h8 Stage B counts of one map (every special, then the admitted-set fixpoint)."""
    print(
        f"movers: tag-using lines {c.tag_using_count} hitting {c.tag_using_hits} sectors "
        f"(max {c.tag_using_max_targets} per line, {c.tag_using_no_target} with none, {c.tag0_lines_all} with tag 0); "
        f"tagged lines whose special ignores the tag {c.tagged_untagged_kind}"
    )
    print(
        f"movers: manual door lines {types(c.manual_doors)}, one-sided {types(c.manual_doors_one_sided)}; "
        f"mover sectors (every special) {c.all_mover_sectors} {c.family_sizes or '{}'}; "
        f"in more than one family {c.all_multi_family}; DR back sectors also non-door targets {c.dr_vs_non_door}; "
        f"plat sectors {c.all_plat_sectors}; sky mover sectors {c.all_sky_movers}; "
        f"self-referencing lines of door sectors {c.self_ref_door_lines}; "
        f"door sectors without a two-sided neighbor {c.doors_no_neighbor}, mover sectors {c.movers_no_neighbor}"
    )
    outside = types(c.outside_stage_b)
    print(
        f"movers (admitted set): line specials outside {{0, 48, 1, 63, 109, 88, 36, 38, 11}} {outside}; "
        f"door sectors {c.adm_door_sectors}, without neighbor {c.adm_no_neighbor} movers; "
        f"reachable heights {reach_text(c)} (±{HEIGHT_BOUND} {flag(c.reach_within and not c.reach_diverged)}), "
        f"sectors whose interval widened {c.reach_moved}; inverting doors {c.inverting_doors}; "
        f"turbo jump-up {c.turbo_jump}; door top drop {c.door_drop}; largest one-tic plane step {c.max_plane_step}; "
        f"B2 Δ {c.vprime_delta}"
    )
    print(f"stage B load rules {flag(c.stage_b)}" + (f": {'; '.join(c.stage_b_failures)}" if c.stage_b_failures else ""))


def movers_detail(c):
    """Every M3h8 measurement of one slot in full (--movers SLOT)."""
    print(f"slot {c.slot}")
    print(f"line special types: {types(c.line_types)}")
    print(f"sector special types: {types(c.sector_types)}")
    trig = collections.Counter(t for t in c.line_types if t != SCROLLER)
    print(f"tag/use/cross line types (all but 48): {types(collections.Counter({t: c.line_types[t] for t in trig}))}")
    print(f"  equal to {{1, 63, 109, 88, 36, 38, 11}}: {set(trig) == set(E1M1_LINE_TYPES)}")
    print(f"tagged lines {c.tagged_lines}; tag-using lines {c.tag_using_count}; tagged lines whose special ignores the tag {c.tagged_untagged_kind}")
    for special, kind, trigger, tag0, targets in sorted(c.tag_using_lines):
        kinds = types(collections.Counter(c.sectors[s][5] for s in targets))
        fams = sorted({f for f, v in c.families.items() for s in targets if s in v})
        print(
            f"  line {special} ({trigger} {kind}){' tag 0' if tag0 else ''}: {len(targets)} target sectors; "
            f"target sector special types {kinds}; target families {fams}; "
            f"sky targets {sum(1 for s in targets if c.sectors[s][3].rstrip(bytes(1)) == b'F_SKY1')}"
        )
    print(f"manual door lines by type {types(c.manual_doors)}; one-sided {types(c.manual_doors_one_sided)}")
    print(f"scrollers {c.scrollers}; two-sided {c.two_sided_scrollers}")
    print(f"mover sectors (every special) {c.all_mover_sectors}; by family {c.family_sizes}")
    print(f"  (admitted set) {c.mover_sectors}: door {c.door_sectors}, plat {c.plat_sectors}, floor {c.floor_sectors}")
    print(f"sectors in more than one mover family: every special {c.all_multi_family}, admitted set {c.multi_family}; "
          f"DR back sectors that are non-door targets {c.dr_vs_non_door}")
    print(f"plat sectors {c.all_plat_sectors} (≤ {MAXPLATS} {flag(c.all_plat_sectors <= MAXPLATS)}); "
          f"reusable switch lines {c.reusable_switches} (≤ {MAXBUTTONS} {flag(c.switches_ok)})")
    print(f"sky mover sectors: every special {c.all_sky_movers}, admitted set {c.sky_movers}")
    print(f"self-referencing lines of door sectors {c.self_ref_door_lines}; door sectors with one {c.self_ref_doors}")
    print(f"door sectors without a two-sided neighbor {c.doors_no_neighbor}; mover sectors {c.movers_no_neighbor}")
    print(f"tag-0 lines with tag-using specials: every special {c.tag0_lines_all}, admitted set {c.tag0_tag_using}")
    print(f"2x2 window max two-sided special lines {c.window_specials} (≤ {MAXSPECIALCROSS} {flag(c.window_specials_ok)})")
    print(f"admitted-set fixpoint: door sectors {c.adm_door_sectors}; reachable {reach_text(c)}; "
          f"within ±{HEIGHT_BOUND} {flag(c.reach_within and not c.reach_diverged)}; sectors widened {c.reach_moved}; "
          f"inverting doors {c.inverting_doors}; turbo jump-up {c.turbo_jump}; door top drop {c.door_drop}; "
          f"largest one-tic plane step {c.max_plane_step}; B2 Δ (largest floor rise or ceiling drop) {c.vprime_delta}")
    print(f"line specials outside the Stage B set: {types(c.outside_stage_b)}")
    print(f"stage B load rules {flag(c.stage_b)}" + (f": {'; '.join(c.stage_b_failures)}" if c.stage_b_failures else ""))


def movers_summary(all_counts, prefix="SUMMARY "):
    n = len(all_counts)
    def count(pred):
        return sum(1 for c in all_counts if pred(c))
    only = [c for c in all_counts if c.only_admitted]
    m = len(only)
    print(f"{prefix}stage B: maps using only line specials in {{0, 48, 1, 63, 109, 88, 36, 38, 11}}: {m} of {n} "
          f"({', '.join(c.slot for c in only) or 'none'})")
    rules = (
        ("geometry admitted (M3h6)", lambda c: c.geometry_admitted),
        ("sector specials in the Stage A set", lambda c: c.sector_specials_a),
        ("rule 1 every DR line two-sided", lambda c: c.one_sided_dr == 0),
        ("rule 2 mover families disjoint", lambda c: c.multi_family == 0),
        ("rule 3 every mover sector has a two-sided neighbor", lambda c: c.adm_no_neighbor == 0),
        ("rule 4 reachable heights within ±512", lambda c: c.reach_within and not c.reach_diverged),
        ("rule 4 no inverting door", lambda c: c.inverting_doors == 0),
        ("rule 5 plats, buttons, scrollers", lambda c: c.plats_ok and c.switches_ok and c.scrollers_ok),
        ("scrollers one-sided (Stage A)", lambda c: c.two_sided_scrollers == 0),
        ("rule 6 spechit window ≤ 8", lambda c: c.window_specials_ok),
        ("every Stage B load rule", lambda c: c.stage_b),
    )
    for label, pred in rules:
        print(f"{prefix}stage B: {label}: {sum(1 for c in only if pred(c))} of {m} admitted-set maps, {count(pred)} of {n} maps")
    finite = [c for c in all_counts if not c.reach_diverged]
    if finite:
        print(f"{prefix}stage B: admitted-set reachable range over all maps [{min(c.reach_min for c in finite)}, "
              f"{max(c.reach_max for c in finite)}]; within ±{HEIGHT_BOUND} for {count(lambda c: c.reach_within and not c.reach_diverged)} of {n}; "
              f"diverged {n - len(finite)}")
    print(f"{prefix}stage B: inverting doors (admitted set) {sum(c.inverting_doors for c in all_counts)} in all, "
          f"none for {count(lambda c: c.inverting_doors == 0)} of {n}; maximum turbo jump-up {max(c.turbo_jump for c in all_counts)}; "
          f"maximum door top drop {max(c.door_drop for c in all_counts)}; "
          f"maximum one-tic plane step {max(c.max_plane_step for c in all_counts)}; maximum B2 Δ {max(c.vprime_delta for c in all_counts)}")
    print(f"{prefix}stage B: every special: maxima mover sectors {max(c.all_mover_sectors for c in all_counts)}, "
          f"plat sectors {max(c.all_plat_sectors for c in all_counts)}, tag-using lines {max(c.tag_using_count for c in all_counts)}, "
          f"targets of one line {max(c.tag_using_max_targets for c in all_counts)}; "
          f"plat sectors ≤ {MAXPLATS} for {count(lambda c: c.all_plat_sectors <= MAXPLATS)} of {n}")
    for attr, label in (
        ("all_multi_family", "sectors in more than one mover family"),
        ("dr_vs_non_door", "DR back sectors that are non-door targets"),
        ("self_ref_door_lines", "self-referencing lines of door sectors"),
        ("doors_no_neighbor", "door sectors without a two-sided neighbor"),
        ("tag0_lines_all", "tag-0 lines with tag-using specials"),
        ("tag_using_no_target", "tag-using lines with no target sector"),
        ("all_sky_movers", "sky mover sectors"),
    ):
        print(f"{prefix}stage B: every special: {label} {sum(getattr(c, attr) for c in all_counts)} in all; "
              f"none for {count(lambda c, a=attr: getattr(c, a) == 0)} of {n}")
    one_sided = sum((c.manual_doors_one_sided for c in all_counts), collections.Counter())
    print(f"{prefix}stage B: every special: one-sided manual door lines by type {types(one_sided)}")
    print(f"{prefix}stage B: 2x2 window two-sided special lines: maximum {max(c.window_specials for c in all_counts)}; "
          f"≤ {MAXSPECIALCROSS} for {count(lambda c: c.window_specials_ok)} of {n}")


MOVERS_COLUMNS = (
    "Slot",
    "Outside Stage B set",
    "Tag-using lines",
    "Targets",
    "Tag 0",
    "Manual door one-sided",
    "Mover sectors",
    "Families",
    "Multi-family",
    "Plat sectors",
    "Reusable switches",
    "Scrollers (two-sided)",
    "Sky movers",
    "Self-ref door lines",
    "Doors w/o neighbor",
    "2x2 two-sided specials",
    "Reachable (admitted)",
    "Inverting doors",
    "Turbo jump-up",
    "Door top drop",
    "Max step",
    "B2 Δ",
    "Stage B",
    "Failing rules",
)


def movers_markdown_report(all_counts):
    print("| " + " | ".join(MOVERS_COLUMNS) + " |")
    print("| " + " | ".join("---" for _ in MOVERS_COLUMNS) + " |")
    for c in all_counts:
        fams = ", ".join(f"{k} {v}" for k, v in c.family_sizes.items()) or "none"
        cells = [
            c.slot,
            types(c.outside_stage_b),
            c.tag_using_count,
            c.tag_using_hits,
            c.tag0_lines_all,
            sum(c.manual_doors_one_sided.values()),
            c.all_mover_sectors,
            fams,
            c.all_multi_family,
            c.all_plat_sectors,
            c.reusable_switches,
            f"{c.scrollers} ({c.two_sided_scrollers})",
            c.all_sky_movers,
            c.self_ref_door_lines,
            c.doors_no_neighbor,
            c.window_specials,
            reach_text(c),
            c.inverting_doors,
            c.turbo_jump,
            c.door_drop,
            c.max_plane_step,
            c.vprime_delta,
            flag(c.stage_b),
            "; ".join(c.stage_b_failures) or "none",
        ]
        print("| " + " | ".join(str(x) for x in cells) + " |")
    print()
    movers_summary(all_counts, prefix="")


def triple(row_or_counter, key=None):
    """easy/medium/hard counts as `e/m/h`."""
    if key is None:
        return "/".join(str(row_or_counter[skill]) for skill, _bit in SKILL_BITS)
    return "/".join(str(row_or_counter[skill][key]) for skill, _bit in SKILL_BITS)


def thing_row_text(r):
    """One doomednum of the census: type facts from info.c, then the slot's counts."""
    if r["info"] is None:
        facts = f"{r['kind']} (no mobjinfo doomednum)"
    else:
        name, radius, height, flags = r["info"]
        facts = (
            f"{name} \"{HUMAN_NAMES.get(name, name)}\" {r['kind']} "
            f"[{' '.join(sorted(flags)) or 'no flags'}] radius {radius} height {height}"
        )
    return (
        f"  {r['doomednum']} {facts}: total {r['total']}; flagged {triple(r)}; "
        f"ambush {r['ambush']}; not-single {r['notsingle']}; handled {triple(r['spawn'])}"
    )


def things_skill_text(c, skill):
    s = c.thing_skills[skill]
    return (
        f"{skill}: spawned {s['spawned']} (monsters {s['monster']}, pickups {s['pickup']}, "
        f"decorations solid {s['solid decorations']} non-solid {s['non-solid decorations']} "
        f"of which corpses {s['solid corpses'] + s['non-solid corpses']} (solid {s['solid corpses']}, non-solid {s['non-solid corpses']}), "
        f"barrels {s['barrel']}); totalkills {s['totalkills']}; totalitems {s['totalitems']}; "
        f"unknown types reaching I_Error {s['unknown']}"
    )


def things_detail(c):
    """The M3h9 THING census of one slot in full (--things SLOT)."""
    c.things_census()
    st = c.thing_starts
    print(f"slot {c.slot}")
    print(
        f"THINGS records {len(c.things)} (mapthing_t, doomdata.h:203–210); distinct doomednums {len(c.thing_rows)}; "
        f"doomednums by kind {types(c.thing_kinds)}"
    )
    print(
        "per doomednum (type facts from info.c; counts: total records, flagged easy/medium/hard, "
        "MTF_AMBUSH, bit 16, records P_SpawnMapThing handles per skill in single player):"
    )
    for r in c.thing_rows:
        print(thing_row_text(r))
    print("single player, by skill (P_SpawnMapThing p_mobj.c:740–792):")
    for skill, _bit in SKILL_BITS:
        print(f"  {things_skill_text(c, skill)}")
    print(
        f"starts (handled before the skill check, every skill): player 1 starts {st['player 1']} (P_SpawnPlayer spawns the player), "
        f"player 2-4 starts {st['players 2-4']} (stored, not spawned), deathmatch starts {st['dm start']} "
        f"(stored {c.dm_stored}, beyond MAX_DM_STARTS {c.dm_dropped}), non-positive types {st['non-positive type']}"
    )
    print(
        f"unknown types (no mobjinfo doomednum, p_mobj.c:758–761): doomednums {c.unknown_doomednums}, records {st['unknown']}, "
        f"reaching I_Error per skill {triple(c.thing_skills, 'unknown')}"
    )
    print(
        f"barrels (2035): records {sum(r['total'] for r in c.barrel_rows)}; "
        f"corpse doomednums {sum(1 for r in c.thing_rows if r['kind'] == 'corpse')}, records "
        f"{sum(r['total'] for r in c.thing_rows if r['kind'] == 'corpse')}; "
        f"solid doomednums {' '.join(str(d) for d in c.solid_doomednums) or 'none'}"
    )
    print(
        "angle octants of the spawned records (mthing->angle/45 truncated toward zero as C does, "
        "p_mobj.c:794), per skill e/m/h:"
    )
    print("  " + things_octant_text(c))


def octants_outside(c, skill):
    """Spawned records of `skill` whose octant the inherited admission refuses."""
    return sum(n for q, n in c.thing_octants[skill].items() if q not in ADMITTED_OCTANTS)


def things_octant_text(c):
    present = sorted({q for skill, _bit in SKILL_BITS for q in c.thing_octants[skill]})
    histogram = "; ".join(
        f"octant {q} {'/'.join(str(c.thing_octants[skill][q]) for skill, _bit in SKILL_BITS)}" for q in present
    )
    outside = "/".join(str(octants_outside(c, skill)) for skill, _bit in SKILL_BITS)
    return f"{histogram or 'none'}; outside -4..3 (refused by the OriginalDemon angle admission) {outside}"


THINGS_COLUMNS = (
    "Slot",
    "THINGs",
    "Spawned e/m/h",
    "Monsters e/m/h",
    "Pickups e/m/h",
    "Solid deco e/m/h",
    "Non-solid deco e/m/h",
    "Barrels e/m/h",
    "P1 start",
    "P2-4 starts",
    "DM starts",
    "Unknown types",
)
THINGS_ROW_COLUMNS = (
    "Doomednum",
    "Type",
    "Name",
    "Kind",
    "Flags",
    "Radius",
    "Height",
    "Total",
    "Easy",
    "Medium",
    "Hard",
    "Ambush",
    "Bit 16",
    "Handled e/m/h",
)


def things_markdown_report(all_counts, detail_slot="E1M1"):
    """The M3h9 census tables: every slot, then one row per doomednum of `detail_slot`."""
    for c in all_counts:
        c.things_census()
    print("| " + " | ".join(THINGS_COLUMNS) + " |")
    print("| " + " | ".join("---" for _ in THINGS_COLUMNS) + " |")
    for c in all_counts:
        sk = c.thing_skills
        cells = [
            c.slot,
            len(c.things),
            triple(sk, "spawned"),
            triple(sk, "monster"),
            triple(sk, "pickup"),
            triple(sk, "solid decorations"),
            triple(sk, "non-solid decorations"),
            triple(sk, "barrel"),
            c.thing_starts["player 1"],
            c.thing_starts["players 2-4"],
            c.thing_starts["dm start"],
            c.unknown_doomednums,
        ]
        print("| " + " | ".join(str(x) for x in cells) + " |")
    print()
    things_summary(all_counts, prefix="")
    detail = next((c for c in all_counts if c.slot == detail_slot), None)
    if detail is None:
        return
    print()
    print(f"{detail_slot}: one row per doomednum")
    print()
    print("| " + " | ".join(THINGS_ROW_COLUMNS) + " |")
    print("| " + " | ".join("---" for _ in THINGS_ROW_COLUMNS) + " |")
    for r in detail.thing_rows:
        if r["info"] is None:
            name, human, flags, radius, height = "-", "-", "-", "-", "-"
        else:
            name, radius, height, flag_set = r["info"]
            human = HUMAN_NAMES.get(name, name)
            flags = " ".join(sorted(flag_set)) or "none"
        cells = [
            r["doomednum"],
            name,
            human,
            r["kind"],
            flags,
            radius,
            height,
            r["total"],
            r["easy"],
            r["medium"],
            r["hard"],
            r["ambush"],
            r["notsingle"],
            triple(r["spawn"]),
        ]
        print("| " + " | ".join(str(x) for x in cells) + " |")
    print()
    print(
        f"{detail_slot}: spawned THINGs by angle octant (`angle/45` truncated toward zero, "
        "p_mobj.c:794); the inherited OriginalDemon admission refuses octants outside -4..3"
    )
    print()
    print("| Octant | Easy | Medium | Hard |")
    print("| --- | --- | --- | --- |")
    present = sorted({q for skill, _bit in SKILL_BITS for q in detail.thing_octants[skill]})
    for q in present:
        print(f"| {q} | " + " | ".join(str(detail.thing_octants[skill][q]) for skill, _bit in SKILL_BITS) + " |")
    print(
        "| outside -4..3 | "
        + " | ".join(str(octants_outside(detail, skill)) for skill, _bit in SKILL_BITS)
        + " |"
    )
    print()
    print("Spawned THINGs whose angle octant is outside -4..3, per slot")
    print()
    print("| Slot | Spawned e/m/h | Outside -4..3 e/m/h |")
    print("| --- | --- | --- |")
    for c in all_counts:
        outside = "/".join(str(octants_outside(c, skill)) for skill, _bit in SKILL_BITS)
        print(f"| {c.slot} | {triple(c.thing_skills, 'spawned')} | {outside} |")


def things_summary(all_counts, prefix="SUMMARY "):
    n = len(all_counts)
    for c in all_counts:
        c.things_census()

    def most(key):
        return "/".join(str(max(c.thing_skills[skill][key] for c in all_counts)) for skill, _bit in SKILL_BITS)

    def count(pred):
        return sum(1 for c in all_counts if pred(c))

    print(f"{prefix}things: maps {n}; THINGS records in all {sum(len(c.things) for c in all_counts)}, maximum {max(len(c.things) for c in all_counts)}")
    print(
        f"{prefix}things: maxima per skill e/m/h: spawned {most('spawned')}, monsters {most('monster')}, pickups {most('pickup')}, "
        f"solid decorations {most('solid decorations')}, non-solid decorations {most('non-solid decorations')}, "
        f"barrels {most('barrel')}, totalkills {most('totalkills')}, totalitems {most('totalitems')}"
    )
    print(
        f"{prefix}things: maxima: player 2-4 starts {max(c.thing_starts['players 2-4'] for c in all_counts)}, "
        f"deathmatch starts {max(c.thing_starts['dm start'] for c in all_counts)} "
        f"(beyond MAX_DM_STARTS on {count(lambda c: c.dm_dropped)} of {n})"
    )
    print(
        f"{prefix}things: exactly one player 1 start for {count(lambda c: c.thing_starts['player 1'] == 1)} of {n}; "
        f"no unknown type for {count(lambda c: c.unknown_doomednums == 0)} of {n}; "
        f"no non-positive type for {count(lambda c: c.thing_starts['non-positive type'] == 0)} of {n}; "
        f"barrels present on {count(lambda c: c.barrel_rows)} of {n}; "
        f"corpses present on {count(lambda c: c.thing_kinds['corpse']) } of {n}"
    )
    doomednums = sorted({r["doomednum"] for c in all_counts for r in c.thing_rows})
    print(f"{prefix}things: distinct doomednums over all maps {len(doomednums)}; unmatched {sorted(d for d in doomednums if d not in mobj_table() and d not in PLAYER_STARTS and d != DM_START) or 'none'}")


# -- Spawn-admission rule and budget facts (M3h9b) ---------------------------------------
#
# The Rust predicate (crates/doom-sim/src/spawning.rs, player_map_scale.rs) for every THING
# P_SpawnMapThing would spawn under the E1M1 policy, and for the player body: with `r` the
# mobjinfo radius in map units, the closed box [x-r, x+r] x [y-r, y+r] lies inside the closed
# VERTEXES rectangle (RoomBounds::contains_box), the centre has odd ray parity against the
# one-sided linedefs (Boundary::inside, half-open rule) and the box meets no one-sided linedef
# (Boundary::box_meets_wall). A violation is any of the three.

PLAYER_RADIUS = 16
SPAWN_CATEGORIES = ("monster", "pickup", "solid decoration", "non-solid decoration", "barrel", "player")
HITSCAN_CELL_WINDOW = 18  # cells a straight 2,177-unit trace can visit (2,177 / 128 = 17.01)


def segment_meets_open_box(a, b, lo, hi):
    """Closed segment ab meets the OPEN box (lo, hi) (exact clipping with fractions)."""
    low, up = Fraction(0), Fraction(1)
    low_strict = up_strict = False
    for axis in (0, 1):
        d = b[axis] - a[axis]
        if d == 0:
            if not lo[axis] < a[axis] < hi[axis]:
                return False
            continue
        t1, t2 = sorted((Fraction(lo[axis] - a[axis], d), Fraction(hi[axis] - a[axis], d)))
        if t1 >= low:
            low, low_strict = t1, True
        if t2 <= up:
            up, up_strict = t2, True
    return low < up or (low == up and not low_strict and not up_strict)


def wall_depth(a, b, x, y, radius):
    """floor(radius - L-infinity distance from (x, y) to segment ab): how far the wall intrudes into the box."""
    least, most = 0, radius
    while least < most:
        mid = (least + most) // 2
        if segment_meets_box(a, b, (x - mid, y - mid), (x + mid, y + mid)):
            most = mid
        else:
            least = mid + 1
    return radius - least


MOVER_CATEGORIES = ("monster", "barrel", "player")
DEPTH_BUCKETS = ("0", "1-4", "5-8", "9-12", "13+")


def depth_bucket(depth):
    return "0" if depth < 1 else "1-4" if depth <= 4 else "5-8" if depth <= 8 else "9-12" if depth <= 12 else "13+"


def spawn_category(r):
    """The report category of a census row, or None when the record is not spawned in single player."""
    kind = r["kind"]
    if kind == "player start":
        return "player" if r["doomednum"] == 1 else None
    if kind not in SPAWNED_KINDS:
        return None
    if kind in ("monster", "pickup", "barrel"):
        return kind
    return "solid decoration" if "SOLID" in r["info"][3] else "non-solid decoration"


def spawn_rule_census(c):
    """Per skill: spawned things, rule violations and BLOCKMAP links of one slot (cached; never prints)."""
    if hasattr(c, "spawn_skills"):
        return
    c.things_census()
    rows = {r["doomednum"]: r for r in c.thing_rows}
    lines = [(c.vertices[l[0]], c.vertices[l[1]]) for l in c.lines]
    walls = [(a, b) for (a, b), l in zip(lines, c.lines) if l[6] == NO_SIDE]
    scaled = [((2 * a[0], 2 * a[1]), (2 * b[0], 2 * b[1])) for a, b in walls]  # doubled, as the C3 probe
    grid = collections.defaultdict(list)  # coarse 128-unit grid of line bounding boxes
    for index, (a, b) in enumerate(lines):
        for gx in range(min(a[0], b[0]) // CELL, max(a[0], b[0]) // CELL + 1):
            for gy in range(min(a[1], b[1]) // CELL, max(a[1], b[1]) // CELL + 1):
                grid[(gx, gy)].append(index)
    one_sided = {i for i, l in enumerate(c.lines) if l[6] == NO_SIDE}
    skills = {skill: dict(spawned=collections.Counter(), violations=collections.Counter(),
                          reasons=collections.Counter(), by_type=collections.Counter(), edge=collections.Counter(),
                          overlap=collections.Counter(), depth=collections.Counter(),
                          sp_viol=collections.Counter(), sp_reasons=collections.Counter(), sp_margin=[],
                          cells=collections.Counter()) for skill, _bit in SKILL_BITS}
    for x, y, _angle, dtype, options in c.things:
        doomednum = dtype if dtype < 0x8000 else dtype - 0x10000
        row = rows[doomednum]
        category = spawn_category(row)
        if category is None or (options & MTF_NOTSINGLE and category != "player"):
            continue
        radius = PLAYER_RADIUS if category == "player" else row["info"][1]
        lo, hi = (x - radius, y - radius), (x + radius, y + radius)
        inside_rect = c.min_x <= lo[0] and hi[0] <= c.max_x and c.min_y <= lo[1] and hi[1] <= c.max_y
        odd = ray_parity(2 * x, 2 * y, scaled) == 1
        near = {i for gx in range(lo[0] // CELL, hi[0] // CELL + 1) for gy in range(lo[1] // CELL, hi[1] // CELL + 1)
                for i in grid.get((gx, gy), ())}
        touched = [i for i in near if segment_meets_box(*lines[i], lo, hi)]
        hits_wall = any(i in one_sided for i in touched)
        walls_hit = [lines[i] for i in touched if i in one_sided]
        overlap_walls = [w for w in walls_hit if segment_meets_open_box(*w, lo, hi)]
        deepest = max((wall_depth(*w, x, y, radius) for w in overlap_walls), default=None)
        reasons = {"wall": hits_wall, "even parity": not odd, "outside rectangle": not inside_rect,
                   "any line": bool(touched)}
        # S-prime: centre parity and rectangle; movers also keep the centre (31 - r) / L in front of
        # every wall that meets the open box. Integer cross products, no division but the margin floor.
        sp_geometry = not odd or not inside_rect
        sp_depth = False
        margin = None
        if category in MOVER_CATEGORIES:
            for (ax, ay), (bx, by) in overlap_walls:
                dx, dy = bx - ax, by - ay
                cross = abs(dx * (y - ay) - dy * (x - ax))
                norm = abs(dx) + abs(dy)
                if cross < (31 - radius) * norm:
                    sp_depth = True
                m = cross // norm
                margin = m if margin is None else min(margin, m)
        cx, cy = (x - c.org_x) >> 7, (y - c.org_y) >> 7
        in_bm = 0 <= cx < c.width and 0 <= cy < c.height
        linked = in_bm and "NOBLOCKMAP" not in (row["info"][3] if row["info"] else ())
        for skill, bit in SKILL_BITS:
            if category != "player" and not options & bit:
                continue
            s = skills[skill]
            s["spawned"][category] += 1
            for name, hit in reasons.items():
                if hit:
                    s["reasons"][name] += 1
            if not in_bm:
                s["reasons"]["outside BLOCKMAP"] += 1
            if sp_geometry or sp_depth:
                s["sp_viol"][category] += 1
                s["sp_reasons"]["parity/rectangle"] += sp_geometry
                s["sp_reasons"]["mover-depth"] += sp_depth
            if margin is not None:
                s["sp_margin"].append(margin)
            if hits_wall:
                if overlap_walls:
                    s["overlap"][category] += 1
                    s["depth"][depth_bucket(deepest)] += 1
                else:
                    s["edge"][category] += 1
            if hits_wall or not odd or not inside_rect:
                s["violations"][category] += 1
                s["by_type"][doomednum] += 1
            if linked:
                s["cells"][(cx, cy)] += 1
    c.spawn_skills = skills
    c.spawn_rows = rows


def spawn_pass(c, skill):
    return not sum(c.spawn_skills[skill]["violations"].values())


def edge_overlap_text(s):
    over = s["overlap"]
    movers = sum(over[k] for k in MOVER_CATEGORIES)
    statics = sum(over.values()) - movers
    depth = ", ".join(f"{b} {s['depth'][b]}" for b in DEPTH_BUCKETS)
    return (f"edge-only {sum(s['edge'].values())}, overlap {sum(over.values())} (movers {movers}, statics {statics}); "
            f"overlap depth {depth}")


def sprime_text(c):
    """S-prime result of one slot: pass/fail, violations by kind and reason, embedded-mover margin."""
    spawn_rule_census(c)
    parts = []
    for skill, _bit in SKILL_BITS:
        s = c.spawn_skills[skill]
        by_kind = ", ".join(f"{k} {s['sp_viol'][k]}" for k in SPAWN_CATEGORIES if s["sp_viol"][k]) or "none"
        margin = min(s["sp_margin"]) if s["sp_margin"] else None
        parts.append(
            f"{skill} {flag(not sum(s['sp_viol'].values()))} (violations {sum(s['sp_viol'].values())}: {by_kind}; "
            f"parity/rectangle {s['sp_reasons']['parity/rectangle']}, mover-depth {s['sp_reasons']['mover-depth']}; "
            f"min embedded-mover margin {margin})"
        )
    return f"{c.slot} S-prime: " + "; ".join(parts)


def sprime_summary(all_counts):
    for skill, _bit in SKILL_BITS:
        bad, reasons = collections.Counter(), collections.Counter()
        for c in all_counts:
            bad.update(c.spawn_skills[skill]["sp_viol"])
            reasons.update(c.spawn_skills[skill]["sp_reasons"])
        passing = sum(1 for c in all_counts if not sum(c.spawn_skills[skill]["sp_viol"].values()))
        kinds = ", ".join(f"{k} {bad[k]}" for k in SPAWN_CATEGORIES if bad[k]) or "none"
        print(f"SUMMARY S-prime {skill}: {passing} of {len(all_counts)} slots pass; violations {sum(bad.values())} ({kinds}); "
              f"parity/rectangle {reasons['parity/rectangle']}, mover-depth {reasons['mover-depth']}")


def spawn_rule_text(c):
    spawn_rule_census(c)
    parts = []
    for skill, _bit in SKILL_BITS:
        s = c.spawn_skills[skill]
        parts.append(
            f"{skill} {flag(spawn_pass(c, skill))} (spawned {sum(s['spawned'].values())}, "
            f"violations {sum(s['violations'].values())}, wall-touch {s['reasons']['wall']}, "
            f"even-parity {s['reasons']['even parity']}, outside-rectangle {s['reasons']['outside rectangle']}, "
            f"touching any line {s['reasons']['any line']}, centre outside BLOCKMAP {s['reasons']['outside BLOCKMAP']})"
        )
    return f"{c.slot}: " + "; ".join(parts)


def spawn_rule_detail(c):
    """Violation kinds of one slot (--spawn-rule SLOT): per skill by category and by doomednum."""
    spawn_rule_census(c)
    print(spawn_rule_text(c))
    print(sprime_text(c))
    for skill, _bit in SKILL_BITS:
        s = c.spawn_skills[skill]
        by_cat = ", ".join(f"{k} {s['violations'][k]}/{s['spawned'][k]}" for k in SPAWN_CATEGORIES if s["spawned"][k])
        print(f"  {skill} violations/spawned by category: {by_cat or 'none'}")
        print(f"  {skill} wall touches: {edge_overlap_text(s)}")
        print("  " + skill + " edge/overlap by kind: " + (", ".join(
            f"{k} {s['edge'][k]}/{s['overlap'][k]}" for k in SPAWN_CATEGORIES if s["edge"][k] or s["overlap"][k]) or "none"))
        for doomednum, n in sorted(s["by_type"].items()):
            info = c.spawn_rows[doomednum]["info"]
            name = info[0] if info else "player 1 start"
            print(f"  {skill} violating doomednum {doomednum} {name}: {n}")


def spawn_rule_summary(all_counts):
    for c in all_counts:
        spawn_rule_census(c)
    total = len(all_counts)
    sprime_summary(all_counts)
    for skill, _bit in SKILL_BITS:
        passing = sum(1 for c in all_counts if spawn_pass(c, skill))
        spawned = sum(sum(c.spawn_skills[skill]["spawned"].values()) for c in all_counts)
        bad = collections.Counter()
        for c in all_counts:
            bad.update(c.spawn_skills[skill]["violations"])
        kinds = ", ".join(f"{k} {bad[k]}" for k in SPAWN_CATEGORIES if bad[k]) or "none"
        reasons = collections.Counter()
        for c in all_counts:
            reasons.update(c.spawn_skills[skill]["reasons"])
        total_s = dict(edge=collections.Counter(), overlap=collections.Counter(), depth=collections.Counter())
        for c in all_counts:
            for key in total_s:
                total_s[key].update(c.spawn_skills[skill][key])
        print(f"SUMMARY spawn rule {skill} wall touches: {edge_overlap_text(total_s)}; "
              + ", ".join(f"{k} {total_s['edge'][k]}/{total_s['overlap'][k]}" for k in SPAWN_CATEGORIES) + " (edge/overlap by kind)")
        print(
            f"SUMMARY spawn rule {skill}: {passing} of {total} slots pass; spawned {spawned}; violations {sum(bad.values())} "
            f"({kinds}); wall-touch {reasons['wall']}, even-parity {reasons['even parity']}, "
            f"outside-rectangle {reasons['outside rectangle']}, touching any line {reasons['any line']}, "
            f"centre outside BLOCKMAP {reasons['outside BLOCKMAP']}"
        )


def spawn_rule_markdown_report(all_counts):
    columns = ("Slot", "Spawned e/m/h", "Violations e/m/h", "Wall touch e/m/h", "Even parity e/m/h",
               "Outside rectangle e/m/h", "Touching any line e/m/h", "Centre outside BLOCKMAP e/m/h", "Pass e/m/h")
    print("| " + " | ".join(columns) + " |")
    print("|" + "|".join("---" for _ in columns) + "|")
    for c in all_counts:
        spawn_rule_census(c)

        def trio(f):
            return "/".join(str(f(c.spawn_skills[skill])) for skill, _bit in SKILL_BITS)

        passes = "/".join(flag(spawn_pass(c, skill)) for skill, _bit in SKILL_BITS)
        cells = (
            c.slot,
            trio(lambda s: sum(s["spawned"].values())),
            trio(lambda s: sum(s["violations"].values())),
            trio(lambda s: s["reasons"]["wall"]),
            trio(lambda s: s["reasons"]["even parity"]),
            trio(lambda s: s["reasons"]["outside rectangle"]),
            trio(lambda s: s["reasons"]["any line"]),
            trio(lambda s: s["reasons"]["outside BLOCKMAP"]),
            passes,
        )
        print("| " + " | ".join(cells) + " |")


# -- Spawn height fit (M3h9b) -------------------------------------------------------------
#
# crates/doom-sim/src/spawning.rs calls `spawn_fits(ty, position, SpawnHeight::Floor)` after the
# spawn rule; player_map_scale.rs `spawn_fits` finds the sector with `locate_validated` (the BSP walk
# of R_PointInSubsector) and requires `sector.floor <= z && z + height <= sector.ceiling` with z the
# floor (ceiling - height for SPAWNCEILING types), i.e. height <= ceiling - floor of that sector.

PLAYER_HEIGHT = 56  # mobjinfo[MT_PLAYER].height (info.c)


def fixed_i32(value):
    value &= 0xFFFFFFFF
    return value - 0x100000000 if value & 0x80000000 else value


def point_on_side(x, y, node):
    """R_PointOnSide (r_main.c:162-210) in 16.16 fixed point; node = a NODES record in map units."""
    nx, ny, ndx, ndy = (v << 16 for v in node[:4])
    x, y = x << 16, y << 16
    if not ndx:
        return int(ndy > 0) if x <= nx else int(ndy < 0)
    if not ndy:
        return int(ndx < 0) if y <= ny else int(ndx > 0)
    dx, dy = fixed_i32(x - nx), fixed_i32(y - ny)  # 32-bit wrap as in C
    if fixed_i32(ndy ^ ndx ^ dx ^ dy) < 0:
        return 1 if fixed_i32(ndy ^ dx) < 0 else 0
    left = ((ndy >> 16) * dx) >> 16
    right = (dy * (ndx >> 16)) >> 16
    return 0 if right < left else 1


def point_in_sector(c, x, y):
    """The sector of R_PointInSubsector (r_main.c:801-823) with ss->sector = first seg's sidedef sector."""
    if not c.nodes:
        child = 0
    else:
        child = len(c.nodes) - 1
        while not child & 0x8000:
            node = c.nodes[child]
            child = node[12 + point_on_side(x, y, node)]
        child &= 0x7FFF
    _count, first = c.subsectors[child]
    seg = c.segs[first]
    line = c.lines[seg[3]]
    return c.sides[line[5 + seg[4]]][5]


def spawn_fit_census(c):
    """Per skill: spawned things, fit failures by category and the minimum height of occupied sectors."""
    if hasattr(c, "fit_skills"):
        return
    c.things_census()
    rows = {r["doomednum"]: r for r in c.thing_rows}
    skills = {skill: dict(spawned=collections.Counter(), failures=collections.Counter(), min_height=None)
              for skill, _bit in SKILL_BITS}
    for x, y, _angle, dtype, options in c.things:
        doomednum = dtype if dtype < 0x8000 else dtype - 0x10000
        row = rows[doomednum]
        category = spawn_category(row)
        if category is None or (options & MTF_NOTSINGLE and category != "player"):
            continue
        height = PLAYER_HEIGHT if category == "player" else row["info"][2]
        sector = c.sectors[point_in_sector(c, x, y)]
        room = sector[1] - sector[0]
        for skill, bit in SKILL_BITS:
            if category != "player" and not options & bit:
                continue
            s = skills[skill]
            s["spawned"][category] += 1
            if height > room:
                s["failures"][category] += 1
            s["min_height"] = room if s["min_height"] is None else min(s["min_height"], room)
    c.fit_skills = skills


def spawn_fit_text(c):
    spawn_fit_census(c)
    parts = []
    for skill, _bit in SKILL_BITS:
        s = c.fit_skills[skill]
        by_cat = ", ".join(f"{k} {s['failures'][k]}" for k in SPAWN_CATEGORIES if s["failures"][k]) or "none"
        parts.append(
            f"{skill} {flag(not sum(s['failures'].values()))} (spawned {sum(s['spawned'].values())}, "
            f"fit failures {sum(s['failures'].values())}: {by_cat}; min occupied-sector height {s['min_height']})"
        )
    return f"{c.slot}: " + "; ".join(parts)


def spawn_fit_summary(all_counts):
    for c in all_counts:
        spawn_fit_census(c)
    for skill, _bit in SKILL_BITS:
        bad = collections.Counter()
        for c in all_counts:
            bad.update(c.fit_skills[skill]["failures"])
        passing = sum(1 for c in all_counts if not sum(c.fit_skills[skill]["failures"].values()))
        spawned = sum(sum(c.fit_skills[skill]["spawned"].values()) for c in all_counts)
        kinds = ", ".join(f"{k} {bad[k]}" for k in SPAWN_CATEGORIES if bad[k]) or "none"
        print(f"SUMMARY spawn fit {skill}: {passing} of {len(all_counts)} slots pass; spawned {spawned}; "
              f"fit failures {sum(bad.values())} ({kinds})")


def budget_facts(c, skill):
    """BLOCKMAP/intercept budget numbers of one slot and skill (counts only)."""
    spawn_rule_census(c)
    w, h = c.width, c.height
    things = c.spawn_skills[skill]["cells"]
    lines_at = [[len(c.cell_lists[cy * w + cx]) for cx in range(w)] for cy in range(h)]
    weight = [[lines_at[cy][cx] + things.get((cx, cy), 0) for cx in range(w)] for cy in range(h)]
    paths = {"row": [], "column": [], "diagonal": [], "anti-diagonal": []}
    for cy in range(h):
        paths["row"].append([weight[cy][cx] for cx in range(w)])
    for cx in range(w):
        paths["column"].append([weight[cy][cx] for cy in range(h)])
    for d in range(-(h - 1), w):
        paths["diagonal"].append([weight[cy][cy + d] for cy in range(h) if 0 <= cy + d < w])
    for d in range(w + h - 1):
        paths["anti-diagonal"].append([weight[cy][d - cy] for cy in range(h) if 0 <= d - cy < w])

    def best(path_list, window=None):
        top = 0
        for path in path_list:
            if window is None or len(path) <= window:
                top = max(top, sum(path))
            else:
                run = sum(path[:window])
                top = max(top, run)
                for i in range(window, len(path)):
                    run += path[i] - path[i - window]
                    top = max(top, run)
        return top

    full = {name: best(p) for name, p in paths.items()}
    windowed = {name: best(p, HITSCAN_CELL_WINDOW) for name, p in paths.items()}
    window_lines = 0
    for cy in range(max(h - 1, 1)):
        for cx in range(max(w - 1, 1)):
            distinct = set()
            for wy in (cy, cy + 1):
                for wx in (cx, cx + 1):
                    if wx < w and wy < h:
                        distinct.update(c.cell_lists[wy * w + wx])
            window_lines = max(window_lines, len(distinct))
    return dict(
        width=w, height=h,
        max_things_per_cell=max(things.values(), default=0),
        max_lines_per_cell=max(max(r) for r in lines_at),
        max_line_row_sum=max(sum(r) for r in lines_at),
        max_line_column_sum=max(sum(lines_at[cy][cx] for cy in range(h)) for cx in range(w)),
        window_lines=window_lines,
        full_paths=full, full_max=max(full.values()),
        window_paths=windowed, window_max=max(windowed.values()),
    )


def budget_text(c):
    out = [f"{c.slot}: BLOCKMAP {c.width}x{c.height} cells; max lines/cell {c.max_per_cell}; "
           f"2x2 window distinct lines (Rust slide_window_lines) {budget_facts(c, 'medium')['window_lines']}"]
    for skill, _bit in SKILL_BITS:
        b = budget_facts(c, skill)
        out.append(
            f"  {skill}: max things/cell {b['max_things_per_cell']}; max lines/row {b['max_line_row_sum']}, "
            f"lines/column {b['max_line_column_sum']}; hitscan estimate (lines + things along a straight cell path) "
            f"full path max {b['full_max']} (row {b['full_paths']['row']}, column {b['full_paths']['column']}, "
            f"diagonal {b['full_paths']['diagonal']}, anti-diagonal {b['full_paths']['anti-diagonal']}); "
            f"{HITSCAN_CELL_WINDOW}-cell window max {b['window_max']} (row {b['window_paths']['row']}, "
            f"column {b['window_paths']['column']}, diagonal {b['window_paths']['diagonal']}, "
            f"anti-diagonal {b['window_paths']['anti-diagonal']})"
        )
    return "\n".join(out)


# M4a episode census (--episode-markdown [E]) and demo-lump header census (--demos-markdown).
# Counts and header facts only: no coordinate, angle, ticcmd byte or lump content is printed.
# Wire 22 (PlayerMonstersLevelV1) admission, read from the Rust source: sector specials
# STAGE_A_SECTOR_SPECIALS, line specials {0,1,11,36,38,48,63,88,109} (= STAGE_B_LINE_SPECIALS)
# and the thing set below. The thing set is the union checked by the map-thing loop of
# crates/doom-sim/src/spawning.rs:443-585 (starts 1..4/11 at 444-476; the skill/multiplayer
# filter at 477-486; `pickup || former || decoration || barrel || MT_SERGEANT` at 505-518):
#   pickups      world_pickups.rs:113-128 `kind` (PickupKind::original_type, 63-79): MT_CLIP 2007,
#                MT_MISC17 2048, MT_MISC22 2008, MT_MISC23 2049, MT_MISC10 2011, MT_MISC11 2012,
#                MT_SHOTGUN 2001, MT_CHAINGUN 2002, MT_MISC0 2018, MT_MISC1 2019, MT_MISC2 2014,
#                MT_MISC3 2015;
#   monsters     original_registry.rs:610-614 FORMER_HUMANS (3004, 9, 65), `is_imp` 714-716 (3001),
#                and owner_slot 747-749 = MT_SERGEANT (3002, the demon);
#   decorations  e1m1_rules.rs:34-42 E1M1_DECORATIONS: 2028, 48, 35, 24, 15, 10, 12;
#   barrel       barrels.rs:71-74 `is_barrel`, DoomEd 2035.
# Player start 1 is a body record (spawning.rs:458-474); 2-4 and 11 never spawn on this path.
WIRE22_LINE_SPECIALS = frozenset((0, 1, 11, 36, 38, 48, 63, 88, 109))
WIRE22_THINGS = frozenset(
    (2007, 2048, 2008, 2049, 2011, 2012, 2001, 2002, 2018, 2019, 2014, 2015)  # pickups
    + (3004, 9, 65, 3001, 3002)  # monsters
    + (2028, 48, 35, 24, 15, 10, 12)  # decorations
    + (2035,)  # barrel
)
NONSPAWNING_STARTS = (2, 3, 4, 11)
# Line-special groups, each derived from the case labels of P_CrossSpecialLine
# (p_spec.c:492-870), P_ShootSpecialLine (p_spec.c:959-1000) and P_UseSpecialLine
# (p_switch.c:276-650); the LINE_KINDS table above carries the kind of each special.
# Keyed doors: manual doors 26/27/28 (DR) and 32/33/34 (D1) call EV_VerticalDoor, which tests
# the key cards (p_doors.c:440-480); 99, 133-137 call EV_DoLockedDoor (p_switch.c:495-503,
# 627-634). Crusher ceilings: crushAndRaise 25, 73 (p_spec.c:620-623, 790-794) and 49
# (p_switch.c:421), fastCrushAndRaise 6, 77, silentCrushAndRaise 141, lowerAndCrush 44, 72.
# Ceiling movers that do not crush: 40 (raiseToHighest), 41, 43 (lowerToFloor). Crusher
# stops: 57, 74. Teleports: EV_Teleport 39, 97, 125, 126 (p_spec.c:655-660, 765-769, 900-906,
# 933-937). Exits: G_ExitLevel 11, 52; G_SecretExitLevel 51, 124 (kinds "exit").
LOCKED_LINES = (26, 27, 28, 32, 33, 34, 99, 133, 134, 135, 136, 137)
CRUSHER_LINES = (6, 25, 44, 49, 72, 73, 77, 141)
CEILING_OTHER_LINES = (40, 41, 43)
CRUSH_STOP_LINES = (57, 74)
TELEPORT_LINES = (39, 97, 125, 126)
EXIT_LINES = (11, 51, 52, 124)
STAIR_LINES = (7, 8, 100, 127)
LIGHT_LINES = (12, 13, 17, 35, 79, 80, 81, 104, 138, 139)
LIFT_LINES = tuple(sorted(s for s, (k, _t) in LINE_KINDS.items() if k == "plat" or k == "plat stop"))
DOOR_LINES = tuple(sorted(s for s, (k, _t) in LINE_KINDS.items() if k in ("door", "manual door")))
FLOOR_LINES = tuple(sorted(s for s, (k, _t) in LINE_KINDS.items() if k in ("floor", "floor+ceiling")))
LINE_GROUPS = (
    ("locked doors", LOCKED_LINES),
    ("doors, all (manual and tagged)", DOOR_LINES),
    ("lifts and platforms (plat, plat stop)", LIFT_LINES),
    ("floor movers", FLOOR_LINES),
    ("crusher ceilings", CRUSHER_LINES),
    ("other ceiling movers", CEILING_OTHER_LINES),
    ("crusher stops", CRUSH_STOP_LINES),
    ("stairs", STAIR_LINES),
    ("donut", (9,)),
    ("light line specials", LIGHT_LINES),
    ("teleport lines", TELEPORT_LINES),
    ("exit lines", EXIT_LINES),
    ("scroller 48", (48,)),
)
# Thing groups by DoomEd number (names are read from info.c at run time). Keys MT_MISC4-9;
# weapons are the SPECIAL weapon pickups; powerups are the p_inter.c P_TouchSpecialThing
# cases of the spheres, the invulnerability and the other timed or map items (84 is the Wolf SS, a monster).
KEY_THINGS = (5, 6, 13, 38, 39, 40)
WEAPON_THINGS = (2001, 2002, 2003, 2004, 2005, 2006, 82)
POWERUP_THINGS = (2013, 2022, 2023, 2024, 2025, 2026, 2045, 83)
TELEPORT_DEST_THING = 14
START_NAMES = {1: "player 1 start", 2: "player 2 start", 3: "player 3 start", 4: "player 4 start", 11: "deathmatch start"}


def read_lump(data, lumps, name, index=0):
    found = [(p, s) for n, p, s in lumps if n == name]
    pos, size = found[index]
    return data[pos : pos + size]


def episode_census(data, lumps, slot):
    """The raw lump facts of one slot as counters (no object is kept)."""
    table = mobj_table()
    blobs = map_lumps(data, lumps, slot)
    things = records(blobs["THINGS"], "<hhHHH")
    lines = records(blobs["LINEDEFS"], "<HHHHHHH")
    sectors = records(blobs["SECTORS"], "<hh8s8sHHH")
    medium = collections.Counter()
    total = collections.Counter()
    for _x, _y, _angle, dtype, options in things:
        doomednum = dtype if dtype < 0x8000 else dtype - 0x10000
        total[doomednum] += 1
        if doomednum == DM_START or doomednum in (1, 2, 3, 4):
            medium[doomednum] += 1  # stored before the skill test (p_mobj.c:717-737)
        elif not (options & MTF_NOTSINGLE) and (options & MTF_NORMAL):
            medium[doomednum] += 1
    line_tags = [l[4] for l in lines if l[4]]
    sector_tags = [s[6] for s in sectors if s[6]]
    return dict(
        slot=slot,
        records=len(things),
        total=total,
        medium=medium,
        unknown=sorted(d for d in medium if d not in table and d > 4 and d != DM_START),
        line_specials=collections.Counter(l[3] for l in lines),
        sector_specials=collections.Counter(s[5] for s in sectors),
        lines=len(lines),
        sectors=len(sectors),
        tagged_lines=len(line_tags),
        tagged_sectors=len(sector_tags),
        line_tag_set=set(line_tags),
        sector_tag_set=set(sector_tags),
        tagged_special_lines=sum(1 for l in lines if l[3] and l[4]),
        untagged_nonzero_special_lines=sum(1 for l in lines if l[3] and not l[4]),
    )


def thing_name(doomednum):
    table = mobj_table()
    if doomednum in START_NAMES:
        return START_NAMES[doomednum]
    info = table.get(doomednum)
    return info[0] if info else "UNKNOWN to info.c"


def thing_class(doomednum):
    info = mobj_table().get(doomednum)
    return thing_kind(doomednum, info)


def fmt_nums(nums, named=False):
    if not nums:
        return "none"
    return ", ".join(f"{n} {thing_name(n)}" if named else str(n) for n in sorted(nums))


def wide_table(header, keys, label, cols, getter):
    print("| " + " | ".join(header + [c["slot"] for c in cols]) + " |")
    print("| " + " | ".join(["---"] * len(header) + ["---:"] * len(cols)) + " |")
    for key in keys:
        print("| " + " | ".join(label(key) + [str(getter(c, key)) for c in cols]) + " |")


def episode_markdown_report(data, lumps, episode):
    slots = [f"E{episode}M{m}" for m in range(1, 10)]
    present = set(map_slots(lumps))
    slots = [s for s in slots if s in present]
    if not slots:
        print(f"iwad_counts: episode {episode} has no map slots in DOOM.WAD (skipped)")
        return
    cols = [episode_census(data, lumps, s) for s in slots]
    table = mobj_table()
    print(f"# M4a census: DOOM.WAD episode {episode} on medium skill\n")
    print(
        "Counts only (no coordinate, angle or per-object datum). `medium` = the record is handled by\n"
        "`P_SpawnMapThing` on medium skill (p_mobj.c:717-751): starts (types 1-4 and 11) on every skill,\n"
        "any other type only when `options & 2` is set and `options & 16` (multiplayer only) is clear.\n"
        "Names are the info.c `mobjinfo` entries of the doomednum (info.h enum order).\n"
    )
    print("## (a) THING doomednums on medium\n")
    keys = sorted(set().union(*(c["medium"] for c in cols)))
    wide_table(
        ["doomednum", "info.c name", "class"],
        keys,
        lambda k: [str(k), thing_name(k), thing_class(k)],
        cols,
        lambda c, k: c["medium"].get(k, 0),
    )
    print("| | **handled on medium** | | " + " | ".join(str(sum(c["medium"].values())) for c in cols) + " |")
    print("| | **records in lump (all skills)** | | " + " | ".join(str(c["records"]) for c in cols) + " |")
    print("| | **distinct doomednums on medium** | | " + " | ".join(str(len(c["medium"])) for c in cols) + " |")
    print()
    print("## (b) LINEDEF specials (count of lines)\n")
    keys = sorted(set().union(*(c["line_specials"] for c in cols)))
    wide_table(
        ["special", "kind, trigger", "wire 22"],
        keys,
        lambda k: [
            str(k),
            "-" if k == 0 else (" ".join(LINE_KINDS[k]) if k in LINE_KINDS else "UNKNOWN to LINE_KINDS"),
            "admitted" if k in WIRE22_LINE_SPECIALS else "missing",
        ],
        cols,
        lambda c, k: c["line_specials"].get(k, 0),
    )
    print("| | **linedefs** | | " + " | ".join(str(c["lines"]) for c in cols) + " |")
    print()
    print("## (c) SECTOR specials (count of sectors)\n")
    keys = sorted(set().union(*(c["sector_specials"] for c in cols)))
    wide_table(
        ["special", "wire 22"],
        keys,
        lambda k: [str(k), "admitted" if k in STAGE_A_SECTOR_SPECIALS else "missing"],
        cols,
        lambda c, k: c["sector_specials"].get(k, 0),
    )
    print("| | **sectors** | " + " | ".join(str(c["sectors"]) for c in cols) + " |")
    print("| | **sectors with a nonzero tag** | " + " | ".join(str(c["tagged_sectors"]) for c in cols) + " |")
    print()
    print("## (d) Tags\n")
    rows = (
        ("linedefs with a nonzero tag", lambda c: c["tagged_lines"]),
        ("linedefs with a special and a nonzero tag", lambda c: c["tagged_special_lines"]),
        ("linedefs with a nonzero special and tag 0", lambda c: c["untagged_nonzero_special_lines"]),
        ("sectors with a nonzero tag", lambda c: c["tagged_sectors"]),
        ("distinct nonzero line tags", lambda c: len(c["line_tag_set"])),
        ("distinct nonzero sector tags", lambda c: len(c["sector_tag_set"])),
        ("distinct nonzero tags (lines and sectors)", lambda c: len(c["line_tag_set"] | c["sector_tag_set"])),
        ("line tags matching no sector tag", lambda c: len(c["line_tag_set"] - c["sector_tag_set"])),
    )
    print("| count | " + " | ".join(c["slot"] for c in cols) + " |")
    print("| --- | " + " | ".join("---:" for _ in cols) + " |")
    for name, fn in rows:
        print(f"| {name} | " + " | ".join(str(fn(c)) for c in cols) + " |")
    print()
    print("## (e) Wire 22 classification\n")
    print(
        "Wire 22 admits: line specials {0, 1, 11, 36, 38, 48, 63, 88, 109}; sector specials\n"
        "{0, 1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16, 17}; THING doomednums: player 1 start, plus\n"
        f"{fmt_nums(WIRE22_THINGS, named=True)}.\n"
        "Non-spawning on this path (stored, not spawned): deathmatch starts 11 and player starts 2-4.\n"
    )
    print("| map | missing thing types | missing line specials | missing sector specials | non-spawning starts (records) |")
    print("| --- | --- | --- | --- | --- |")
    for c in cols:
        med = set(c["medium"])
        miss_t = sorted(med - WIRE22_THINGS - {1, 2, 3, 4, 11})
        miss_l = sorted(set(c["line_specials"]) - WIRE22_LINE_SPECIALS)
        miss_s = sorted(set(c["sector_specials"]) - STAGE_A_SECTOR_SPECIALS)
        non = ", ".join(f"{k} x{c['medium'][k]}" for k in NONSPAWNING_STARTS if c["medium"].get(k))
        print(
            f"| {c['slot']} | {len(miss_t)}: {fmt_nums(miss_t)} | {len(miss_l)}: {fmt_nums(miss_l)} "
            f"| {len(miss_s)}: {fmt_nums(miss_s)} | {non or 'none'} |"
        )
    print()
    print("Missing thing types with names and counts on medium:\n")
    print("| map | missing thing types (doomednum name x count) |")
    print("| --- | --- |")
    for c in cols:
        miss_t = sorted(set(c["medium"]) - WIRE22_THINGS - {1, 2, 3, 4, 11})
        print(f"| {c['slot']} | " + (", ".join(f"{t} {thing_name(t)} x{c['medium'][t]}" for t in miss_t) or "none") + " |")
    print()
    print("Missing line specials with line counts:\n")
    print("| map | missing line specials (special x lines) |")
    print("| --- | --- |")
    for c in cols:
        miss_l = sorted(set(c["line_specials"]) - WIRE22_LINE_SPECIALS)
        print(f"| {c['slot']} | " + (", ".join(f"{s} x{c['line_specials'][s]}" for s in miss_l) or "none") + " |")
    print()
    print("Missing sector specials with sector counts:\n")
    print("| map | missing sector specials (special x sectors) |")
    print("| --- | --- |")
    for c in cols:
        miss_s = sorted(set(c["sector_specials"]) - STAGE_A_SECTOR_SPECIALS)
        print(f"| {c['slot']} | " + (", ".join(f"{s} x{c['sector_specials'][s]}" for s in miss_s) or "none") + " |")
    print()
    print("Unknown doomednums (would be `I_Error` in P_SpawnMapThing, p_mobj.c:758-761) after the filter: ", end="")
    unknown = {c["slot"]: c["unknown"] for c in cols if c["unknown"]}
    print(", ".join(f"{s}: {v}" for s, v in unknown.items()) if unknown else "none")
    print()
    print("## (f) Feature groups\n")
    print("Line groups are counts of lines; thing groups are counts of medium records.\n")
    print("| group | members | " + " | ".join(c["slot"] for c in cols) + " |")
    print("| --- | --- | " + " | ".join("---:" for _ in cols) + " |")
    for name, members in LINE_GROUPS:
        print(
            f"| line: {name} | {', '.join(map(str, members))} | "
            + " | ".join(str(sum(c["line_specials"].get(m, 0) for m in members)) for c in cols)
            + " |"
        )
    print()
    thing_groups = (
        ("keys", KEY_THINGS),
        ("teleport destination", (TELEPORT_DEST_THING,)),
        ("weapons", WEAPON_THINGS),
        ("powerups", POWERUP_THINGS),
        ("admitted monsters", (3004, 9, 65, 3001, 3002)),
        ("monsters beyond the admitted set", tuple(sorted(d for d, i in mobj_table().items() if thing_kind(d, i) == "monster" and d not in WIRE22_THINGS))),
        ("pickups beyond the admitted set", tuple(sorted(d for d, i in mobj_table().items() if thing_kind(d, i) == "pickup" and d not in WIRE22_THINGS))),
        ("barrel", (2035,)),
    )
    print("| thing group | members | " + " | ".join(c["slot"] for c in cols) + " |")
    print("| --- | --- | " + " | ".join("---:" for _ in cols) + " |")
    for name, members in thing_groups:
        label = ", ".join(f"{m} {thing_name(m)}" for m in members)
        print(
            f"| {name} | {label} | "
            + " | ".join(str(sum(c["medium"].get(m, 0) for m in members)) for c in cols)
            + " |"
        )
    print()
    print("Names of the KEY, WEAPON and POWERUP members as info.c gives them (a member unknown to info.c is flagged):\n")
    for name, members in (("keys", KEY_THINGS), ("weapons", WEAPON_THINGS), ("powerups", POWERUP_THINGS)):
        print(f"- {name}: " + ", ".join(f"{m} {thing_name(m)}" for m in members))


def demo_markdown_report(directory):
    print("# M4a census: demo lump headers\n")
    print(
        "Header facts only (G_ReadDemoTiccmd layout, g_game.c: version, skill, episode, map, deathmatch,\n"
        "respawn, fast, nomonsters, consoleplayer, playeringame[4]; then 4 bytes per tic per player;\n"
        "then the 0x80 end marker). No ticcmd byte is printed. Skill is 0-based (0 = I'm too young to die,\n"
        "2 = hurt me plenty).\n"
    )
    print(
        "| WAD | lump | bytes | version | skill | episode | map | deathmatch | respawn | fast | nomonsters "
        "| consoleplayer | playeringame[0..3] | players | ticcmds | seconds (35/s) | 0x80 at expected end | stray bytes |"
    )
    print("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- | ---: |")
    for name in sorted(os.listdir(directory)):
        if not name.lower().endswith(".wad"):
            continue
        data, lumps = read_wad(os.path.join(directory, name))
        for lump, pos, size in lumps:
            if not re.fullmatch(r"DEMO\d", lump):
                continue
            blob = data[pos : pos + size]
            if size < 13:
                print(f"| {name} | {lump} | {size} | (shorter than the 13-byte header) |")
                continue
            version, skill, episode, mapno, dm, respawn, fast, nomon, console = blob[:9]
            ingame = list(blob[9:13])
            players = sum(1 for p in ingame if p)
            if players == 0:
                print(f"| {name} | {lump} | {size} | {version} | {skill} | {episode} | {mapno} | no players in game |")
                continue
            body = size - 13
            per_tic = 4 * players
            tics = (body - 1) // per_tic if body >= 1 else 0
            end = 13 + tics * per_tic
            marker = end < size and blob[end] == 0x80
            stray = size - end - (1 if marker else 0)
            print(
                f"| {name} | {lump} | {size} | {version} | {skill} | {episode} | {mapno} | {dm} | {respawn} | {fast} "
                f"| {nomon} | {console} | {'/'.join(str(p) for p in ingame)} | {players} | {tics} | {tics / 35:.1f} "
                f"| {'yes' if marker else 'no'} | {stray} |"
            )


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("directory", nargs="?", default=DEFAULT_DIR)
    parser.add_argument("--markdown", action="store_true", help="print the counts table as Markdown")
    parser.add_argument("--readiness", metavar="SLOT", help="print the §9 readiness counts of one map slot")
    parser.add_argument(
        "--specials-markdown", action="store_true", help="print the M3h7 §5.6 specials counts table as Markdown"
    )
    parser.add_argument("--movers", metavar="SLOT", help="print every M3h8 Stage B measurement of one map slot")
    parser.add_argument(
        "--movers-markdown", action="store_true", help="print the M3h8 Stage B movers table as Markdown"
    )
    parser.add_argument("--things", metavar="SLOT", help="print the M3h9 THING census of one map slot")
    parser.add_argument(
        "--things-markdown", action="store_true", help="print the M3h9 THING census tables as Markdown"
    )
    parser.add_argument(
        "--spawn-rule", nargs="?", const="*", metavar="SLOT",
        help="print the spawn-admission rule counts of every slot, or the violations of one slot",
    )
    parser.add_argument(
        "--spawn-rule-markdown", action="store_true", help="print the spawn-admission rule table as Markdown"
    )
    parser.add_argument(
        "--spawn-fit", nargs="?", const="*", metavar="SLOT",
        help="print the spawn height-fit counts of every slot, or of one slot",
    )
    parser.add_argument(
        "--things-budget", nargs="?", const="*", metavar="SLOT",
        help="print the BLOCKMAP and intercept budget facts of every slot, or of one slot",
    )
    parser.add_argument(
        "--episode-markdown", nargs="?", const="1", metavar="E",
        help="print the M4a episode census of DOOM.WAD (default episode 1) on medium skill as Markdown",
    )
    parser.add_argument(
        "--demos-markdown", action="store_true", help="print the header facts of every DEMOn lump as Markdown"
    )
    args = parser.parse_args(argv)
    if not os.path.isdir(args.directory):
        print(f"iwad_counts: {args.directory} is absent; nothing to report (skipped)")
        return 0
    if args.demos_markdown:
        demo_markdown_report(args.directory)
        return 0
    if args.episode_markdown:
        path = os.path.join(args.directory, "DOOM.WAD")
        if not os.path.isfile(path):
            print(f"iwad_counts: {path} is absent; nothing to report (skipped)")
            return 0
        data, lumps = read_wad(path)
        episode_markdown_report(data, lumps, int(args.episode_markdown))
        return 0
    all_counts = []
    for name in sorted(os.listdir(args.directory)):
        if not name.lower().endswith(".wad"):
            continue
        data, lumps = read_wad(os.path.join(args.directory, name))
        for slot in map_slots(lumps):
            all_counts.append(MapCounts(slot, map_lumps(data, lumps, slot)))
    if not all_counts:
        print("iwad_counts: no map slots found (skipped)")
        return 0
    if args.readiness:
        for counts in all_counts:
            if counts.slot == args.readiness:
                for key, value in counts.readiness().items():
                    print(f"{key}: {value}")
                return 0
        print(f"iwad_counts: slot {args.readiness} not found")
        return 1
    if args.movers:
        for counts in all_counts:
            if counts.slot == args.movers:
                movers_detail(counts)
                return 0
        print(f"iwad_counts: slot {args.movers} not found")
        return 1
    if args.movers_markdown:
        movers_markdown_report(all_counts)
        return 0
    if args.things:
        for counts in all_counts:
            if counts.slot == args.things:
                things_detail(counts)
                return 0
        print(f"iwad_counts: slot {args.things} not found")
        return 1
    if args.things_markdown:
        things_markdown_report(all_counts)
        return 0
    if args.spawn_rule_markdown:
        spawn_rule_markdown_report(all_counts)
        return 0
    if args.spawn_fit:
        chosen = [c for c in all_counts if args.spawn_fit in ("*", c.slot)]
        if not chosen:
            print(f"iwad_counts: slot {args.spawn_fit} not found")
            return 1
        for c in chosen:
            print(spawn_fit_text(c))
        if args.spawn_fit == "*":
            spawn_fit_summary(all_counts)
        return 0
    if args.spawn_rule or args.things_budget:
        slot = args.spawn_rule or args.things_budget
        chosen = [c for c in all_counts if slot in ("*", c.slot)]
        if not chosen:
            print(f"iwad_counts: slot {slot} not found")
            return 1
        for c in chosen:
            if args.things_budget:
                print(budget_text(c))
            elif slot == "*":
                print(spawn_rule_text(c))
                print(sprime_text(c))
            else:
                spawn_rule_detail(c)
        if args.spawn_rule and slot == "*":
            spawn_rule_summary(all_counts)
        return 0
    if args.markdown:
        markdown_report(all_counts)
        return 0
    if args.specials_markdown:
        specials_markdown_report(all_counts)
        return 0
    for counts in all_counts:
        text_report(counts)
    summary(all_counts)
    specials_summary(all_counts)
    movers_summary(all_counts)
    return 0


if __name__ == "__main__":
    sys.exit(main())
