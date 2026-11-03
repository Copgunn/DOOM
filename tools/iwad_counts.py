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
    python3 tools/iwad_counts.py DIR ...          # another read-only directory

The rules evaluated are those of docs/M3H6_REFERENCE_INVESTIGATION.md: the arithmetic
certificate (i)–(vii) of §4.2, the boundary rules C1–C3, the closed-cell BLOCKMAP rule,
grid coverage and REJECT length of §4.5, and the slide-window bound of §4.4.
"""

from __future__ import annotations

import argparse
import collections
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
        f"(≤ {MAXSPECIALCROSS} {flag(counts.window_specials_ok)}); reachable heights not computed"
    )


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
    print(f"{prefix}stage B: reachable heights not computed")


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


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("directory", nargs="?", default=DEFAULT_DIR)
    parser.add_argument("--markdown", action="store_true", help="print the counts table as Markdown")
    parser.add_argument("--readiness", metavar="SLOT", help="print the §9 readiness counts of one map slot")
    parser.add_argument(
        "--specials-markdown", action="store_true", help="print the M3h7 §5.6 specials counts table as Markdown"
    )
    args = parser.parse_args(argv)
    if not os.path.isdir(args.directory):
        print(f"iwad_counts: {args.directory} is absent; nothing to report (skipped)")
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
