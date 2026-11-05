"""Polygon rooms for the M3h9b observer: the M3h9 builder unchanged, plus the declared
exploding-barrel table and the BLOCKMAP arithmetic the M3h9b scenarios are checked
against.

Rooms are described exactly as for m3h9_geometry (a list of sectors, each a list of
integer vertex rings, floor and ceiling heights and an optional dict of light, special,
tag and flats; edges shared in opposite directions become two-sided linedefs whose
front is the sector listed first, every other edge a one-sided ML_BLOCKING wall).
Nothing in the lumps changes relative to M3h9.

What is added: `BARREL` is the declared `mobjinfo[MT_BARREL]` row (M3h9b note §2.1:
DoomEd 2035, spawn state `S_BAR1`, health 20, radius 10, height 42, mass 100, flags
`MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD`, death state `S_BEXP`); the reference asserts every
value against the compiled genuine info.c through the trace. `block_of` is the
BLOCKMAP cell of a coordinate (`(v - bmaporg) >> MAPBLOCKSHIFT` on fixed-point values,
p_local.h:40, the arithmetic shift floors), `wrapped_dist` is the value p_map.c:1221
computes on the original two's-complement targets (`(damage + MAXRADIUS) << FRACBITS`
reduced to 32 bits, MAXRADIUS being `32*FRACUNIT` without parentheses, p_local.h:51)
and `window` the block range P_RadiusAttack visits for a spot and a `dist`
(p_map.c:1222-1225). The "intended" 160-unit window is computed only so a case can
state what the wrap changes. Nothing here reads Rust output."""
import m3h9_geometry as base
from m3h9_geometry import DECORATIONS, ML_BLOCKING, enum_members, enum_index, FLATS, SKYFLATNUM, SWITCH_TEXTURES  # noqa: F401  (re-exported)
FRACBITS, FRACUNIT = 16, 65536
MAPBLOCKSHIFT = FRACBITS + 7
MAXRADIUS = 32 * FRACUNIT
MF_SOLID, MF_SHOOTABLE, MF_AMBUSH, MF_NOBLOOD, MF_CORPSE, MF_DROPOFF = 2, 4, 32, 0x80000, 0x100000, 0x400
ANG45 = 0x20000000
# DoomEd 2035 -> declared mobjinfo row (M3h9b note §2.1, info.c:1888-1912)
BARREL = dict(doomednum=2035, member='MT_BARREL', spawnstate='S_BAR1', health=20, radius=10, height=42, mass=100,
              flags=MF_SOLID | MF_SHOOTABLE | MF_NOBLOOD, deathstate='S_BEXP', painchance=0, reactiontime=8)
BARREL_STATES = ('S_BAR1', 'S_BAR2', 'S_BEXP', 'S_BEXP2', 'S_BEXP3', 'S_BEXP4', 'S_BEXP5')
EXPLOSION_DAMAGE = 128  # A_Explode, p_enemy.c:1600


def room(sectors, portals=None, reject=(), tree=None, segment=4096, lines=None):
    return base.room(sectors, portals=portals, reject=reject, tree=tree, segment=segment, lines=lines)


def origin(m):
    """(bmaporgx, bmaporgy, bmapwidth, bmapheight) of a built room, in map units."""
    b = m['blocks']
    return b[0], b[1], b[2], b[3]


def wrapped_dist(damage):
    """p_map.c:1221 on a 32-bit two's-complement int: `(damage + 32*FRACUNIT) << 16`."""
    value = ((damage + MAXRADIUS) << FRACBITS) & 0xFFFFFFFF
    return value - (1 << 32) if value & 0x80000000 else value


def block_of(v, org):
    """The BLOCKMAP column (row) of the fixed-point coordinate v for the origin org (units)."""
    return (v - org * FRACUNIT) >> MAPBLOCKSHIFT


def window(x, y, m, dist):
    """(xl, xh, yl, yh) of P_RadiusAttack for a fixed-point spot (p_map.c:1222-1225)."""
    ox, oy, _, _ = origin(m)
    return block_of(x - dist, ox), block_of(x + dist, ox), block_of(y - dist, oy), block_of(y + dist, oy)


def angle_of(record_angle):
    """p_mobj.c:794 on the original targets: ANG45 * (angle/45), the quotient truncated
    toward zero, the int product reduced to 32 bits and stored in the unsigned angle_t."""
    q = abs(record_angle) // 45 * (-1 if record_angle < 0 else 1)  # C truncation toward zero
    return (q * ANG45) & 0xFFFFFFFF
