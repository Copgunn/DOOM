"""Polygon rooms at real-map scale for the M3h6 observer: the M3h5 builder with an
automatic node builder and format-sized bounds.

Rooms are described exactly as for m3h5_geometry (a list of sectors, each a list of
integer vertex rings: the first ring is the sector's outer boundary listed clockwise
with y up, further rings are holes listed counterclockwise; an edge shared by two
sectors in opposite directions becomes one two-sided linedef, every other edge a
one-sided ML_BLOCKING wall split into pieces of at most `segment` units at exact
integer points). What changes:

- **Automatic BSP.** With `tree=None` the partition of every node is chosen among the
  lines of the segs in that node by the DoomBSP grade (the imbalance between the two
  sides plus eight per split seg; an axis-aligned candidate wins a tie), as
  `EvaluateSplit`/`BuildBSP` of id's node builder do. A group with no splitting
  candidate is a leaf. Seg splits are computed exactly in rationals and **rounded to
  the nearest integer vertex** the way DoomBSP rounds (real maps carry such split
  vertices, slightly off their linedef); the new vertex is appended to VERTEXES. Every
  leaf is asserted non-empty and single-sector by its segs' sides, and convex up to
  the rounding tolerance of one unit. An explicit `tree` still works as before.
- **Bounds.** The M3h5 asserts (128 lines, 256 vertices, 128 leaves and nodes, 256
  sides, 4,096 cells) are replaced by the sizes of the M3h6 harness arrays (the shim
  sizes m3h6_reference.py patches in): 4,096 lines, 8,192 sides, 1,024 sectors, 16,384 segs, 8,192 leaves
  and nodes, 16,384 cells; the BLOCKMAP keeps the format's own bounds (list offsets
  below 32,768 words). The line norm is bounded by the arithmetic certificate's 3,853
  (M3h6 note §4.2) instead of 1,536.

The BLOCKMAP lists every linedef in every cell whose closed 128-unit square meets the
closed segment (exact clip), its origin is the vertex minimum, and line 0 is listed
only by each cell's leading 0. REJECT is zero except for the caller's pairs. Nothing
here reads Rust output."""
import struct
from fractions import Fraction
import generate_m3b as old
from m3h5_geometry import cross, split_wall, meets_cell
BLOCKING, TWOSIDED = 1, 4
MAX_NORM = 3853
LIMITS = dict(lines=4096, sides=8192, sectors=1024, segs=16384, leaves=8192, nodes=8192, cells=16384, vertices=16384)
def grade(o, d, group, vertices):
    """DoomBSP `EvaluateSplit`: the split count times eight plus the imbalance, or
    None when the candidate does not separate the group (nothing on its back side)."""
    front = back = splits = 0
    for s in group:
        ca, cb = cross(o, d, vertices[s['a']]), cross(o, d, vertices[s['b']])
        if ca == 0 and cb == 0:
            if d[0] * (vertices[s['b']][0] - vertices[s['a']][0]) + d[1] * (vertices[s['b']][1] - vertices[s['a']][1]) > 0:
                front += 1
            else:
                back += 1
        elif ca <= 0 and cb <= 0:
            front += 1
        elif ca >= 0 and cb >= 0:
            back += 1
        else:
            splits += 1
            front += 1
            back += 1
    if back == 0 or front == 0:
        return None
    return abs(front - back) + 8 * splits
def room(sectors, portals=None, reject=(), tree=None, segment=1024):
    """`sectors` lists (rings, floor, ceiling); `portals` maps a sector pair to extra
    linedef flags; `tree` is an explicit partition tree or None for the automatic
    builder."""
    portals = dict(portals or {}); n = len(sectors); heights = [(f, c) for _, f, c in sectors]
    vertices = []; index_of = {}
    def vertex(p):
        p = (int(p[0]), int(p[1]))
        if p not in index_of:
            index_of[p] = len(vertices); vertices.append(p)
        return index_of[p]
    owner = {}
    for s, (rings, _, _) in enumerate(sectors):
        for ring in rings:
            for i, a in enumerate(ring):
                b = ring[(i + 1) % len(ring)]; assert a != b and (a, b) not in owner, (s, a, b); owner[(a, b)] = s
    lines = []
    def add(a, b, front, back, flags):
        assert abs(b[0] - a[0]) + abs(b[1] - a[1]) <= MAX_NORM, (a, b)
        lines.append(dict(a=vertex(a), b=vertex(b), front=front, back=back, flags=flags))
    done = set()
    for (a, b), s in owner.items():
        if (a, b) in done: continue
        k = owner.get((b, a))
        if k is None:
            for p, q in split_wall(a, b, segment): add(p, q, s, None, BLOCKING)
        else:
            assert k != s, (s, a, b); done.add((b, a))
            add(a, b, s, k, TWOSIDED | portals.get((min(s, k), max(s, k)), 0))
    linebytes = b''; sidebytes = b''; sidecount = 0
    for l in lines:
        indexes = []
        for sector in [l['front'], l['back']]:
            if sector is None: indexes.append(-1); continue
            indexes.append(sidecount); sidecount += 1
            sidebytes += old.words([0, 0]) + old.name('-') * 2 + old.name('TESTWALL') + old.words([sector])
        l['sides'] = indexes; linebytes += old.words([l['a'], l['b'], l['flags'], 0, 0, *indexes])
    segs = []
    for i, l in enumerate(lines):
        segs.append(dict(a=l['a'], b=l['b'], line=i, side=0, sector=l['front']))
        if l['back'] is not None:
            segs.append(dict(a=l['b'], b=l['a'], line=i, side=1, sector=l['back']))
    nodes = []; leaves = []; nodebytes = b''
    def bbox(group):
        points = [vertices[s[k]] for s in group for k in ('a', 'b')]
        return [max(p[1] for p in points), min(p[1] for p in points), min(p[0] for p in points), max(p[0] for p in points)]
    def leaf(group):
        assert group, 'empty leaf'
        sector = {s['sector'] for s in group}; assert len(sector) == 1, ('leaf spans sectors', sector)
        for s in group:
            o = vertices[s['a']]; d = (vertices[s['b']][0] - o[0], vertices[s['b']][1] - o[1])
            tolerance = abs(d[0]) + abs(d[1])  # one unit of perpendicular distance, for rounded split vertices
            for t in group:
                for k in ('a', 'b'): assert cross(o, d, vertices[t[k]]) <= tolerance, ('concave leaf', o, d, vertices[t[k]])
        leaves.append(dict(segs=group, sector=sector.pop())); return 0x8000 | (len(leaves) - 1)
    def choose(group):
        best = None
        for s in group:
            o = vertices[s['a']]; d = (vertices[s['b']][0] - o[0], vertices[s['b']][1] - o[1])
            g = grade(o, d, group, vertices)
            if g is None: continue
            key = (g, int(d[0] != 0 and d[1] != 0))
            if best is None or key < best[0]: best = (key, s['line'])
        return None if best is None else best[1]
    def build(group, subtree, automatic, depth=0):
        nonlocal nodebytes
        assert depth < 4096, 'partition recursion did not terminate'
        if automatic:
            index = choose(group)
            if index is None: return leaf(group)
            front_tree = back_tree = None
        else:
            if subtree is None: return leaf(group)
            a, b, front_tree, back_tree = subtree
            index = next(i for i, l in enumerate(lines) if vertices[l['a']] == tuple(a) and vertices[l['b']] == tuple(b))
        line = lines[index]; o = vertices[line['a']]; d = (vertices[line['b']][0] - o[0], vertices[line['b']][1] - o[1]); front = []; back = []
        for s in group:
            pa, pb = vertices[s['a']], vertices[s['b']]; ca, cb = cross(o, d, pa), cross(o, d, pb)
            if ca == 0 and cb == 0:
                (front if d[0] * (pb[0] - pa[0]) + d[1] * (pb[1] - pa[1]) > 0 else back).append(s)
            elif ca <= 0 and cb <= 0: front.append(s)
            elif ca >= 0 and cb >= 0: back.append(s)
            else:
                t = Fraction(ca, ca - cb); m = (pa[0] + (pb[0] - pa[0]) * t, pa[1] + (pb[1] - pa[1]) * t)
                if not automatic:
                    assert m[0].denominator == 1 and m[1].denominator == 1, ('non-integer split', pa, pb, o, d)
                # DoomBSP rounds split points to the nearest integer vertex.
                mid = vertex((int(m[0] + Fraction(1, 2)) if m[0] >= 0 else -int(-m[0] + Fraction(1, 2)),
                              int(m[1] + Fraction(1, 2)) if m[1] >= 0 else -int(-m[1] + Fraction(1, 2))))
                # Rounding must not collapse a piece to a point (a loud failure beats a
                # re-split loop); DoomBSP's inputs never do on the committed rooms.
                assert mid != s['a'] and mid != s['b'], ('degenerate split piece', pa, pb, o, d)
                first = dict(s, b=mid); second = dict(s, a=mid)
                (front if ca < 0 else back).append(first); (front if cb < 0 else back).append(second)
        assert front and back, ('partition does not split', index)
        children = [build(front, front_tree, automatic, depth + 1), build(back, back_tree, automatic, depth + 1)]
        nodes.append(dict(x=o[0], y=o[1], dx=d[0], dy=d[1], children=children, line=index))
        nodebytes += old.words([o[0], o[1], d[0], d[1], *bbox(front), *bbox(back)]) + struct.pack('<HH', *children)
        return len(nodes) - 1
    build(segs, tree, tree is None)
    assert {l['sector'] for l in leaves} == set(range(n)), 'every sector needs a subsector'
    segbytes = b''; subbytes = b''; first = 0
    for l in leaves:
        for s in l['segs']: segbytes += old.words([s['a'], s['b'], 0, s['line'], s['side'], 0])
        subbytes += old.words([len(l['segs']), first]); first += len(l['segs'])
    rejectbytes = bytearray((n * n + 7) // 8)
    for a, b in reject:
        for pair in (a * n + b, b * n + a): rejectbytes[pair >> 3] |= 1 << (pair & 7)
    ox = min(x for x, _ in vertices); oy = min(y for _, y in vertices)
    width = (max(x for x, _ in vertices) - ox) // 128 + 1; height = (max(y for _, y in vertices) - oy) // 128 + 1
    cells = width * height; lists = []; offsets = []
    # Candidate lines per cell from the bbox cells of every line, then the exact clip.
    by_cell = [[] for _ in range(cells)]
    for i, l in enumerate(lines):
        if i == 0: continue
        pa, pb = vertices[l['a']], vertices[l['b']]
        for cy in range((min(pa[1], pb[1]) - oy) // 128, (max(pa[1], pb[1]) - oy) // 128 + 1):
            for cx in range((min(pa[0], pb[0]) - ox) // 128, (max(pa[0], pb[0]) - ox) // 128 + 1):
                if meets_cell(pa, pb, ox + cx * 128, oy + cy * 128): by_cell[cy * width + cx].append(i)
    for cell in range(cells):
        offsets.append(4 + cells + len(lists)); lists.append(0); lists.extend(by_cell[cell]); lists.append(-1)
    blocks = [ox, oy, width, height, *offsets, *lists]
    assert max(offsets) < 32768 and len(blocks) <= 32768 and cells <= LIMITS['cells'], (max(offsets), len(blocks), cells)
    assert len(lines) <= LIMITS['lines'] and len(vertices) <= LIMITS['vertices'] and len(leaves) <= LIMITS['leaves'] and len(nodes) <= LIMITS['nodes'] and sidecount <= LIMITS['sides'] and len(segs) <= LIMITS['segs'] and n <= LIMITS['sectors'], (len(lines), len(vertices), len(leaves), len(nodes), sidecount, n)
    lumps = [('MAP01', b''), ('THINGS', b''), ('LINEDEFS', linebytes), ('SIDEDEFS', sidebytes), ('VERTEXES', b''.join(old.words(list(v)) for v in vertices)),
             ('SEGS', segbytes), ('SSECTORS', subbytes), ('NODES', nodebytes), ('SECTORS', b''.join(old.words([f, c]) + b'TESTFLR0TESTCEIL' + old.words([160, 0, 0]) for f, c in heights)),
             ('REJECT', bytes(rejectbytes)), ('BLOCKMAP', old.words(blocks))]
    xs = [x for x, _ in vertices]; ys = [y for _, y in vertices]
    return dict(heights=heights, vertices=vertices, lines=lines, nodes=nodes, leaves=[l['sector'] for l in leaves], blocks=blocks, bytes=old.wad(lumps), lumps=dict(lumps),
                segs=sum(len(l['segs']) for l in leaves), sides=sidecount, cells=cells,
                range=(min(xs), max(xs)), center=((min(xs) + max(xs)) // 2, (min(ys) + max(ys)) // 2))
def window_lines(m):
    """Distinct linedefs listed in any 2 x 2 window of BLOCKMAP cells (the slide-window
    certificate of the M3h6 note §4.4), counting line 0 through each cell's leading 0."""
    blocks = m['blocks']; width, height = blocks[2], blocks[3]
    def listed(cell):
        at = blocks[4 + cell]; out = set()
        while blocks[at] != -1: out.add(blocks[at]); at += 1
        return out
    cells = [listed(c) for c in range(width * height)]
    best = 0
    for cy in range(max(height - 1, 1)):
        for cx in range(max(width - 1, 1)):
            distinct = set()
            for wy in (cy, cy + 1):
                for wx in (cx, cx + 1):
                    if wx < width and wy < height: distinct |= cells[wy * width + wx]
            best = max(best, len(distinct))
    return best
