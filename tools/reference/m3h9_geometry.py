"""Polygon rooms for the M3h9 observer: the M3h8 builder unchanged, plus the two-sided
scroller inventory and the declared decoration table the M3h9 scenarios are checked
against.

Rooms are described exactly as for m3h8_geometry (a list of sectors, each a list of
integer vertex rings, floor and ceiling heights and an optional dict of light, special,
tag and flats; edges shared in opposite directions become two-sided linedefs whose
front is the sector listed first, every other edge a one-sided ML_BLOCKING wall).
`lines` maps a directed edge to `(special, tag)` or `(special, tag, flags)`; M3h9 uses
the flags form for the blocking two-sided scroller (`ML_BLOCKING` OR-ed into the
builder's `ML_TWOSIDED`). Nothing in the lumps changes relative to M3h8: a type-48 line
on a shared edge is simply a two-sided linedef with special 48, which M3h8's rooms never
built (decision N1) and M3h9's do (decision E1).

What is added to the M3h8 output: `m['scrollers']` lists every line with special 48 as
`{line, two_sided, blocking}` so the reference can state which kind each room carries.
`DECORATIONS` is the declared table of the seven E1M1 decoration types (M3h9 note §2.5:
DoomEd number, `mobjinfo` name, spawn state, radius, height, `MF_SOLID`); the reference
asserts every value against the compiled genuine info.c through the trace, and
`enum_index` reads the position of a `statenum_t`/`mobjtype_t` member from info.h so
those assertions use the source enum, not a copied number. Nothing here reads Rust
output."""
import re
from pathlib import Path
import m3h8_geometry as base
from m3h8_geometry import FLATS, SKY, SKYFLATNUM, TEXTURES, SWITCH_NAME, SWITCH_TEXTURES, SWITCH_SPECIALS, side_names, window_lines  # noqa: F401  (re-exported)
ROOT = Path(__file__).resolve().parents[2]
ML_BLOCKING, ML_TWOSIDED = 1, 4
SCROLL = 48
MF_SOLID, MF_SHOOTABLE, MF_AMBUSH = 2, 4, 32
# DoomEd -> (description, mobjinfo member, spawn state, radius, height, solid)  (M3h9 note §2.5)
DECORATIONS = {
    2028: ('floor lamp', 'MT_MISC31', 'S_COLU', 16, 16, True),
    48: ('tall techno pillar', 'MT_MISC48', 'S_TECHPILLAR', 16, 16, True),
    35: ('candelabra', 'MT_MISC50', 'S_CANDELABRA', 16, 16, True),
    24: ('pool of blood and flesh', 'MT_MISC71', 'S_GIBS', 20, 16, False),
    15: ('dead player', 'MT_MISC62', 'S_PLAY_DIE7', 20, 16, False),
    10: ('bloody mess', 'MT_MISC68', 'S_PLAY_XDIE9', 20, 16, False),
    12: ('bloody mess', 'MT_MISC69', 'S_PLAY_XDIE9', 20, 16, False),
}
assert all(radius < 64 for _, _, _, radius, _, _ in DECORATIONS.values())  # M3h8 §6 window proof


def enum_members(name, header='info.h'):
    """The members of `typedef enum { ... } name;` in a source header, in declaration
    order (comments stripped; the enums read here assign no explicit values)."""
    text = (ROOT / 'linuxdoom-1.10' / header).read_text()
    end = text.index('} ' + name + ';')
    start = text.rindex('typedef enum', 0, end)
    body = re.sub(r'//[^\n]*', '', text[start:end])
    body = body[body.index('{') + 1:]
    members = [m for m in re.findall(r'\b[A-Za-z_]\w*\b', body)]
    assert '=' not in body, name
    return members


def enum_index(name, member, header='info.h'):
    return enum_members(name, header).index(member)


def room(sectors, portals=None, reject=(), tree=None, segment=4096, lines=None):
    m = base.room(sectors, portals=portals, reject=reject, tree=tree, segment=segment, lines=lines)
    m['scrollers'] = [dict(line=i, two_sided=l['back'] is not None, blocking=bool(l['flags'] & ML_BLOCKING))
                      for i, (l, (special, _)) in enumerate(zip(m['lines'], m['line_specials'])) if special == SCROLL]
    for s in m['scrollers']:
        flags = m['lines'][s['line']]['flags']
        assert bool(flags & ML_TWOSIDED) == s['two_sided'], s
    return m
