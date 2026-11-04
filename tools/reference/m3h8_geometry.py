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

What is added to the M3h7 output: the LINEDEFS lump carries the extra flags, and the
SIDEDEFS lump names `SWITCH_NAME` (`SW1COMM`, the first episode-1 `alphSwitchList` name)
as the middle texture of the front side of every line with special 11 or 63 (the switch
types of the admitted set); every other side keeps the builder's `-`, `-`, `TESTWALL`.
`switch_lines` lists those lines. The C harness reads the names back from the SIDEDEFS
bytes and resolves them through the declared synthetic texture table `TEXTURES` (the
stand-in for R_TextureNumForName: `-` is 0, a name is its table index); `SWITCH_TEXTURES`
is the declared `switchlist` built from that table (one pair: off `SW1COMM`, on
`SW2COMM`). `TESTWALL` takes table slot 0, the number vanilla shares between its first
texture and `-`, so every other side texture is 0 and is never in `switchlist`. Geometry,
vertices, segs, nodes, subsectors, REJECT and BLOCKMAP are the M3h6 builder's bytes
unchanged. Nothing here
reads Rust output."""
import generate_m3b as old
import m3h7_geometry as base
from m3h7_geometry import FLATS, SKY, SKYFLATNUM, window_lines  # noqa: F401  (re-exported)
TEXTURES = ('TESTWALL', 'SW1COMM', 'SW2COMM')
SWITCH_NAME = 'SW1COMM'
SWITCH_TEXTURES = (TEXTURES.index('SW1COMM'), TEXTURES.index('SW2COMM'))
SIDE_BYTES = 30  # mapsidedef_t: two offsets, then top, bottom and middle names, then the sector
MIDDLE = 20
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
    if m['switch_lines']:
        sides = bytearray(m['lumps']['SIDEDEFS'])
        for i in m['switch_lines']:
            at = m['lines'][i]['sides'][0] * SIDE_BYTES + MIDDLE
            sides[at:at + 8] = old.name(SWITCH_NAME)
        m['lumps']['SIDEDEFS'] = bytes(sides)
        m['bytes'] = old.wad(list(m['lumps'].items()))
    return m


def side_names(m):
    """The (top, bottom, middle) names of every SIDEDEFS record, in lump order."""
    data = m['lumps']['SIDEDEFS']
    assert len(data) % SIDE_BYTES == 0
    name = lambda raw: raw.split(b'\0', 1)[0].decode('ascii')
    return [tuple(name(data[k + 4 + 8 * j:k + 12 + 8 * j]) for j in range(3)) for k in range(0, len(data), SIDE_BYTES)]
