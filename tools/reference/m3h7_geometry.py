"""Polygon rooms with sector and line specials for the M3h7 observer: the M3h6 builder
plus the SECTORS and LINEDEFS fields the earlier rooms left at their defaults.

Rooms are described exactly as for m3h6_geometry (a list of sectors, each a list of
integer vertex rings plus floor and ceiling heights; edges shared in opposite
directions become two-sided linedefs whose front is the sector listed first, every
other edge a one-sided ML_BLOCKING wall), with one optional fourth element per
sector: a dict of its `light` level, `special`, `tag`, `floor` flat name and
`ceiling` flat name (defaults 160, 0, 0, TESTFLR0, TESTCEIL, the values every
earlier synthetic room wrote). `lines` maps a directed edge `((ax, ay), (bx, by))`
of some ring to a `(special, tag)` pair for the linedef built from it; the edge
must survive the builder unsplit.

What is added to the M3h6 output: the SECTORS lump carries the declared flats, light,
special and tag of every sector; the LINEDEFS lump carries every line's special and
tag. Geometry, vertices, segs, nodes, subsectors, REJECT and BLOCKMAP are the M3h6
builder's bytes unchanged (they do not depend on these fields). `FLATS` is the
declared flat table of the harness (the C adapter's stand-in for
R_FlatNumForName): a flat's number is its index, so `skyflatnum` is the index of
`F_SKY1`. Nothing here reads Rust output."""
import generate_m3b as old
import m3h6_geometry as base
from m3h6_geometry import window_lines  # noqa: F401  (re-exported for the reference)
FLATS = ['TESTFLR0', 'TESTCEIL', 'F_SKY1', 'NUKAGE1', 'FLOOR7_1', 'CEIL3_5']
SKY = 'F_SKY1'
SKYFLATNUM = FLATS.index(SKY)
DEFAULT = dict(light=160, special=0, tag=0, floor='TESTFLR0', ceiling='TESTCEIL')


def room(sectors, portals=None, reject=(), tree=None, segment=4096, lines=None):
    plain = [(rings, floor, ceiling) for rings, floor, ceiling, *_ in sectors]
    attrs = []
    for entry in sectors:
        a = dict(DEFAULT)
        if len(entry) > 3:
            unknown = set(entry[3]) - set(DEFAULT)
            assert not unknown, unknown
            a.update(entry[3])
        assert a['floor'] in FLATS and a['ceiling'] in FLATS, a
        assert 0 <= a['light'] <= 255 and 0 <= a['special'] < 32768 and 0 <= a['tag'] < 32768, a
        attrs.append(a)
    m = base.room(plain, portals=portals, reject=reject, tree=tree, segment=segment)
    specials = [(0, 0)] * len(m['lines'])
    for (a, b), value in (lines or {}).items():
        found = [i for i, l in enumerate(m['lines']) if m['vertices'][l['a']] == tuple(a) and m['vertices'][l['b']] == tuple(b)]
        assert len(found) == 1, ('special edge not built as one linedef', a, b)
        specials[found[0]] = value
    linebytes = b''
    for l, (special, tag) in zip(m['lines'], specials):
        l['special'], l['tag'] = special, tag
        linebytes += old.words([l['a'], l['b'], l['flags'], special, tag, *l['sides']])
    assert len(linebytes) == len(m['lumps']['LINEDEFS'])
    sectorbytes = b''.join(old.words([f, c]) + old.name(a['floor']) + old.name(a['ceiling']) + old.words([a['light'], a['special'], a['tag']])
                           for (f, c), a in zip(m['heights'], attrs))
    assert len(sectorbytes) == len(m['lumps']['SECTORS'])
    m['lumps']['LINEDEFS'] = linebytes
    m['lumps']['SECTORS'] = sectorbytes
    m['bytes'] = old.wad(list(m['lumps'].items()))
    m['sector_attrs'] = attrs
    m['line_specials'] = specials
    return m
