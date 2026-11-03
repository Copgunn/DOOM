"""Polygon rooms with movers for the M3h8 observer: the M3h7 builder plus line flags and
the declared switch textures the Stage B bodies read.

Rooms are described exactly as for m3h7_geometry (a list of sectors, each a list of
integer vertex rings, floor and ceiling heights and an optional dict of light,
special, tag and flats; edges shared in opposite directions become two-sided linedefs
whose front is the sector listed first, every other edge a one-sided ML_BLOCKING wall).
`lines` maps a directed edge `((ax, ay), (bx, by))` of some ring to a `(special, tag)`
pair, or to `(special, tag, flags)` whose `flags` are OR-ed into the builder's
(ML_BLOCKING for a one-sided wall, ML_TWOSIDED for a shared edge); the edge must
survive the builder unsplit. Special lines may be one-sided (switches, exits) or
two-sided (doors, walk-over triggers): the builder makes no distinction.

What is added to the M3h7 output: the LINEDEFS lump carries the extra flags, and
`switch_lines` lists the lines whose front side carries the declared switch texture
(every line with special 11 or 63, the switch types of the admitted set). Textures are
not part of the WAD lumps the Rust loader reads (there are no SIDEDEFS texture names
beyond the builder's defaults); the C harness installs them as the declared texture
numbers below, the stand-in for R_TextureNumForName and P_InitSwitchList.
`SWITCH_TEXTURES` is the declared `switchlist` (one pair: off, on); 0 is the number of
every other side texture and is never in the table. Geometry, vertices, segs, nodes,
subsectors, REJECT and BLOCKMAP are the M3h6 builder's bytes unchanged. Nothing here
reads Rust output."""
import generate_m3b as old
import m3h7_geometry as base
from m3h7_geometry import FLATS, SKY, SKYFLATNUM, window_lines  # noqa: F401  (re-exported)
SWITCH_TEXTURES = (1, 2)
SWITCH_SPECIALS = (11, 63)


def room(sectors, portals=None, reject=(), tree=None, segment=4096, lines=None):
    plain = {}
    flags = {}
    for edge, value in (lines or {}).items():
        assert len(value) in (2, 3), value
        plain[edge] = tuple(value[:2])
        if len(value) == 3 and value[2]:
            flags[edge] = value[2]
    m = base.room(sectors, portals=portals, reject=reject, tree=tree, segment=segment, lines=plain)
    if flags:
        for (a, b), extra in flags.items():
            found = [i for i, l in enumerate(m['lines']) if m['vertices'][l['a']] == tuple(a) and m['vertices'][l['b']] == tuple(b)]
            assert len(found) == 1, ('flagged edge not built as one linedef', a, b)
            m['lines'][found[0]]['flags'] |= extra
        linebytes = b''.join(old.words([l['a'], l['b'], l['flags'], special, tag, *l['sides']])
                             for l, (special, tag) in zip(m['lines'], m['line_specials']))
        assert len(linebytes) == len(m['lumps']['LINEDEFS'])
        m['lumps']['LINEDEFS'] = linebytes
        m['bytes'] = old.wad(list(m['lumps'].items()))
    m['switch_lines'] = [i for i, (special, _) in enumerate(m['line_specials']) if special in SWITCH_SPECIALS]
    return m
