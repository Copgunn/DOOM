#!/usr/bin/env python3
"""Local-only census and bound check for Episode 1 (never in CI: it reads the commercial IWAD in place, read-only,
never copied). For each map E1M1..E1M9 it reports, as counts and PASS/FAIL only:

  A. whether every line listed in a BLOCKMAP cell meets that cell (the converse of the closed-cell load rule,
     crates/doom-sim/src/player_map_scale.rs derive_closed_cell), under two readings of the leading 0 word;
  B. whether |num| and |den| of P_InterceptVector (p_maputl.c:231-251) and P_InterceptVector2 (p_sight.c:109-126)
     are provably below 2^31 for every trace kind, so that wires 23-26's checked_intercept (sight.rs), whose final
     Fixed::fixed_div uses unsigned_abs where FixedDiv uses abs (m_fixed.c:62), cannot meet INT_MIN there.

A. Definition. A cell is the closed square [ox+128x, ox+128x+128] x [oy+128y, oy+128y+128] from the BLOCKMAP origin;
a line meets it when its closed segment meets the closed square (inclusive bounding-box overlap, and the supporting
line does not leave all four corners strictly on one side), in exact integers. Reading (a): the leading 0 word is
the format's marker, not a listing. Reading (b): it is a listing of linedef 0, as P_BlockLinesIterator iterates it
(p_maputl.c:493-504). Miss class: "0" when every listing meets its cell, "1" when every missing listing's line meets
a cell within Chebyshev distance 1 of the listed cell, "more than 1" otherwise.

B. Derivation (each step names the C it rests on; R is the map's closed vertex rectangle, in map units).
 1. Roles. PIT_AddLineIntercepts calls P_InterceptVector(&trace, &dl) with dl the line: start v1, delta (dx, dy) in
    whole units (p_maputl.c:588-589; P_MakeDivline 211-218); PIT_AddThingIntercepts passes a thing's diagonal,
    2*radius per axis through its centre (p_maputl.c:616-663). P_CrossSubsector calls P_InterceptVector2(&strace,
    &divl), the same body, with divl the line (p_sight.c:176, 185-188, 227).
 2. Exact form. FixedMul is ((long long)a*b)>>16 (m_fixed.c:44-49). With whole-unit L = (Ldx, Ldy), dl.dy>>8 is
    256*Ldy and FixedMul(a, Ldy<<16) = a*Ldy exactly, so (p_maputl.c:240, 246-248; p_sight.c:117, 123-124)
      den = floor(Ldy*Tx/256) - floor(Ldx*Ty/256),   num = floor(X/256)*Ldy + floor(Y/256)*Ldx,
    with T the trace delta and X = v1.x - t.x, Y = t.y - v1.y in raw units, t the trace start. Radii in info.c are
    whole units, so a thing diagonal has the same form.
 3. Denominator: |den| < (|Ldy|*|Tx| + |Ldx|*|Ty|)/256 + 1.
 4. Numerator: X*Ldy + Y*Ldx = Ldy*(p.x - t.x) - Ldx*(p.y - t.y) for every point p of the line (p - v1 is parallel
    to L), so with d the Chebyshev distance in units from t to the closed segment and L1 = |Ldx| + |Ldy|,
    |num| < 256*d*L1 + L1 (the two floors lose less than L1).
 5. Verdict. A bound <= 2^31 - 1 makes the value differ from INT_MIN = -2^31, the one operand where unsigned_abs
    and abs disagree: PASS. Otherwise FAIL, meaning not proven (not that INT_MIN is reached). checked_intercept
    returns the source's value or refuses (sight.rs), so PASS means no silent divergence for that operand.
 6. Deltas. trace.dx = x2 - x1 after the +FRACUNIT nudge (p_maputl.c:777-786), so |T| <= |delta| + 1 unit;
    |finesine| <= 65535 (tables.c:584-1866). Aim and attack deltas are (distance>>16)*finecosine (p_map.c:1034-1035,
    1076-1077): use 64 (USERANGE p_local.h:56; p_map.c:1144-1147), punch and saw 64 (MELEERANGE and MELEERANGE+1
    after >>16, p_local.h:57; p_pspr.c:483-516), 1024 (p_pspr.c:607-616, 795; p_mobj.c:949-959), 2048
    (MISSILERANGE p_local.h:58; p_pspr.c:639, 721-723; p_enemy.c:813-862). A slide traces mo->mom (p_map.c:738-743),
    clamped to 30 (p_mobj.c:137-145). One projection maps a per-axis bound m to m + ceil(m/2) (FixedMul by
    bestslidefrac <= FRACUNIT, p_map.c:768-777; P_HitSlideLine 596-629 with P_AproxDistance, p_maputl.c:49-58).
    P_SlideMove traces at entry and after one projection, and leaves at most two (hitcount, p_map.c:706-710,
    781-786); P_XYMovement calls it at most twice (p_mobj.c:150-191). Slide deltas: 30, 45, 67.5 or 101.25 units.
    Sight: strace delta = t2 - t1 (p_sight.c:342-343).
 7. Starts (premise from the Rust policy, not from the C). Every actor centre is in R before each committed tic,
    and after it (M4A_REFERENCE_INVESTIGATION.md A6; teleport landings too, teleport.rs:63); a tic that fails is
    refused with its state preserved, so only committed tics matter. Players think first (p_tick.c:150-152) and use
    and fire from the tic-start centre (p_user.c:326, 334): margin 1 (the nudge). Another actor seen during a
    thinker is at its tic-start or final centre. A monster attacks or looks after its own P_XYMovement (<= 30 per
    axis; a blocked non-player stops, p_mobj.c:150-191) and before its P_Move (p_enemy.c:726-765; speed <= 16,
    info.c): margin 64 for sight ends, 65 for monster traces. Before a slide trace the player moved at most
    30 + 45 + 67.5 = 142.5 units in a first P_SlideMove (flush 756-763, re-flush, then stairstep 749-751 or
    projection 784) plus 67.5 (the second call's flush); EV_Teleport zeroes momentum at a landing in R
    (p_telept.c:103-125). With radius 16 (info.c:1125; p_map.c:714-733) and the nudge the margin is 227.
 8. Lines met. The block walk visits at most 64 cells, each step moving one coordinate by 1 (p_maputl.c:846-876;
    the comment at 841-842 says round-off can skip the end cell: the walk can then run past it along the extended
    line), from the start cell, which holds t (790-791). Off-grid cells list nothing (481-487). A line listed under
    reading (a) in a visited cell, on a map of miss class (a) <= 1, has a point within one more cell: d < 128*(63 +
    1 + 1) = 8320. In every case d <= Dg, the largest Chebyshev distance from a corner of R grown by the margin to
    the segment (d is convex in t). Each line takes min(Dg, 8320) where that holds and Dg otherwise; linedef 0
    (every cell's leading 0) always takes Dg, and so does every line for sight, which meets lines through the BSP
    (p_sight.c:160-227).
 9. Things (aim, attack, punch, saw: PT_ADDTHINGS; slide and use add lines only, p_map.c:738-743, 1147). The
    diagonal passes through the centre c, so d <= |c - t| <= span + 2*margin, with L1 = 4r, r <= 128 (info.c:1619).
10. Lines tested: every word of every list (reading b) for traces, every linedef for sight.

Usage: e1_blockmap_listings.py --iwad PATH [--out FILE] [--verbose] | --iwad PATH --check RESULTS.json | --selftest
--verbose prints the bound magnitudes to stdout for local review only; they are never written to the results.
"""
import argparse
import hashlib
import json
import struct
import sys
from fractions import Fraction

MAPS = ["E1M%d" % i for i in range(1, 10)]
LIMIT = 2**31 - 1
U = 65536
WALK = 128 * (63 + 1 + 1)
# kind: (start margin, raw per-axis delta bound or None for sight, adds things)
KINDS = {
    "slide": (227, 6635520 + U, False),
    "use_melee_64": (1, 64 * U + U, True),
    "aim_1024": (1, 1024 * U + U, True),
    "attack_2048": (65, 2048 * U + U, True),
    "sight": (64, None, False),
}
DEFINITION = " ".join(__doc__.split("B. Derivation")[0].split("A. Definition. ")[1].split())


def orient(a, b, p):
    return (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])


def meets(a, b, lo, hi):
    if max(a[0], b[0]) < lo[0] or min(a[0], b[0]) > hi[0] or max(a[1], b[1]) < lo[1] or min(a[1], b[1]) > hi[1]:
        return False
    cs = [orient(a, b, p) for p in (lo, (hi[0], lo[1]), hi, (lo[0], hi[1]))]
    return not (all(c > 0 for c in cs) or all(c < 0 for c in cs))


def cheb_to_segment(p, a, b):
    """Exact Chebyshev distance from point p to the closed segment ab (convex piecewise linear in s)."""
    u, v, dx, dy = p[0] - a[0], p[1] - a[1], b[0] - a[0], b[1] - a[1]
    cands = {Fraction(0), Fraction(1)}
    for n, d in ((u, dx), (v, dy), (u - v, dx - dy), (u + v, dx + dy)):
        if d and 0 <= Fraction(n, d) <= 1:
            cands.add(Fraction(n, d))
    return min(max(abs(u - s * dx), abs(v - s * dy)) for s in cands)


def read_map(f, directory, name):
    idx = next(i for i, (_, n) in enumerate(directory) if n == name.encode())
    found = {}
    for (pos, size), n in directory[idx + 1: idx + 12]:
        f.seek(pos)
        found.setdefault(n, f.read(size))
    vr, lr, bm = found[b"VERTEXES"], found[b"LINEDEFS"], found[b"BLOCKMAP"]
    verts = [struct.unpack_from("<hh", vr, 4 * i) for i in range(len(vr) // 4)]
    lines = [tuple(verts[k] for k in struct.unpack_from("<HH", lr, 14 * i)) for i in range(len(lr) // 14)]
    return verts, lines, list(struct.unpack_from("<%dH" % (len(bm) // 2), bm, 0))


def census(lines, words):
    """Part A over parsed data: lines as ((x1, y1), (x2, y2)), words the BLOCKMAP lump as unsigned 16-bit words."""
    ox, oy, w, h = (v - 65536 if v > 32767 else v for v in words[:4])
    res = {"cells": w * h, "invalid_line_refs": 0, "cells_without_leading_zero": 0}
    for k in "ab":
        res.update({"listings_" + k: 0, "meet_" + k: 0, "miss_" + k: 0})
    worst = {"a": 0, "b": 0}
    listed_a, listed_b = set(), set()
    for cy in range(h):
        for cx in range(w):
            k = words[4 + cy * w + cx]
            lst = []
            while words[k] != 0xFFFF:
                lst.append(words[k])
                k += 1
            lead = bool(lst) and lst[0] == 0
            res["cells_without_leading_zero"] += not lead
            lo = (ox + 128 * cx, oy + 128 * cy)
            hi = (lo[0] + 128, lo[1] + 128)
            for pos, ln in enumerate(lst):
                if ln >= len(lines):
                    res["invalid_line_refs"] += 1
                    continue
                a, b = lines[ln]
                ok = meets(a, b, lo, hi)
                dist = 0
                if not ok:
                    dist = 2  # "more than 1" unless a neighbouring cell meets the line
                    if any(meets(a, b, (lo[0] + 128 * i, lo[1] + 128 * j), (hi[0] + 128 * i, hi[1] + 128 * j))
                           for i in (-1, 0, 1) for j in (-1, 0, 1)):
                        dist = 1
                listed_b.add(ln)
                keys = ("b",) if pos == 0 and lead else ("a", "b")
                if keys == ("a", "b"):
                    listed_a.add(ln)
                for key in keys:
                    res["listings_" + key] += 1
                    res[("meet_" if ok else "miss_") + key] += 1
                    worst[key] = max(worst[key], dist)
    for key in "ab":
        res["miss_class_" + key] = ["0", "1", "more than 1"][worst[key]]
    return res, listed_a, listed_b


def bounds(lines, verts, res, listed_a, listed_b, max_radius=128):
    """Part B: per kind, PASS/FAIL for line/thing numerator/denominator and linedef 0; magnitudes for --verbose."""
    xs, ys = [v[0] for v in verts], [v[1] for v in verts]
    span = (max(xs) - min(xs), max(ys) - min(ys))
    walk_ok = res["miss_class_a"] in ("0", "1") and res["invalid_line_refs"] == 0
    out, mags, line0 = {}, {}, {}
    for kind, (m, delta, things) in KINDS.items():
        corners = [(x, y) for x in (min(xs) - m, max(xs) + m) for y in (min(ys) - m, max(ys) + m)]
        tx, ty = (delta, delta) if delta else ((span[0] + 2 * m) * U, (span[1] + 2 * m) * U)
        tested = range(len(lines)) if kind == "sight" else sorted(listed_b)
        num = den = n0 = 0
        for i in tested:
            a, b = lines[i]
            ldx, ldy = abs(b[0] - a[0]), abs(b[1] - a[1])
            d = max(cheb_to_segment(c, a, b) for c in corners)
            if kind != "sight" and i != 0 and i in listed_a and walk_ok:
                d = min(d, Fraction(WALK))
            bn = 256 * d * (ldx + ldy) + (ldx + ldy)
            num = max(num, bn)
            n0 = bn if i == 0 else n0
            den = max(den, Fraction(ldy * tx + ldx * ty, 256) + 1)
        verdict = lambda x: "PASS" if x <= LIMIT else "FAIL"
        out[kind] = {"line_numerator": verdict(num), "line_denominator": verdict(den)}
        line0[kind] = verdict(n0)
        mags[kind] = [num, den]
        if things:
            l1 = 4 * max_radius
            tn = 256 * (max(span) + 2 * m) * l1 + l1
            td = Fraction(l1 * tx, 256) + 1
            out[kind].update({"thing_numerator": verdict(tn), "thing_denominator": verdict(td)})
            mags[kind] += [tn, td]
    return out, line0, mags


def analyse(lines, verts, words):
    res, listed_a, listed_b = census(lines, words)
    out, line0, mags = bounds(lines, verts, res, listed_a, listed_b)
    return {"blockmap": res, "linedef0_numerator": line0, "intercepts": out}, mags


def selftest():
    """Synthetic map: a 3x1 grid, one short line listed where it does not reach (two cells away), and one line long
    enough that every numerator bound fails. Proves both checks and --check comparison can fail."""
    verts = [(0, 0), (10, 10), (0, 0), (30000, 30000)]
    lines = [(verts[0], verts[1]), (verts[2], verts[3])]
    words = [0, 0, 3, 1, 7, 11, 13, 0, 0, 1, 0xFFFF, 0, 0xFFFF, 0, 0, 0xFFFF]
    r, _ = analyse(lines, verts, words)
    b = r["blockmap"]
    checks = [
        (b["listings_a"], 3), (b["miss_a"], 1), (b["miss_class_a"], "more than 1"), (b["miss_class_b"], "more than 1"),
        (r["intercepts"]["attack_2048"]["line_numerator"], "FAIL"), (r["linedef0_numerator"]["slide"], "PASS"),
    ]
    words_ok = [0, 0, 3, 1, 7, 10, 12, 0, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF]
    good, _ = analyse(lines[:1], verts[:2], words_ok)
    checks += [(good["blockmap"]["miss_class_a"], "0"), (good["intercepts"]["sight"]["line_numerator"], "PASS")]
    checks.append((compare(r, good), False))
    bad = [(got, want) for got, want in checks if got != want]
    print("selftest: %d checks, %d failed" % (len(checks), len(bad)))
    return 1 if bad else 0


def compare(expected, result):
    strip = lambda d: {k: v for k, v in d.items() if k != "script_sha256"}
    return strip(expected) == strip(result)


def sha256_file(path):
    d = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            d.update(chunk)
    return d.hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--iwad")
    ap.add_argument("--out")
    ap.add_argument("--check")
    ap.add_argument("--verbose", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if not args.iwad:
        ap.error("--iwad is required")
    maps = {}
    with open(args.iwad, "rb") as f:
        ident, count, off = struct.unpack("<4sii", f.read(12))
        if ident not in (b"IWAD", b"PWAD"):
            raise SystemExit("not a WAD")
        f.seek(off)
        raw = f.read(16 * count)
        directory = [(struct.unpack_from("<ii", raw, 16 * i), raw[16 * i + 8:16 * i + 16].rstrip(b"\0"))
                     for i in range(count)]
        size = f.seek(0, 2)
        for name in MAPS:
            verts, lines, words = read_map(f, directory, name)
            maps[name], mags = analyse(lines, verts, words)
            if args.verbose:
                for kind, vals in mags.items():
                    print("%s %-13s bounds/(2^31-1): %s" % (name, kind, " ".join("%.3f" % (v / LIMIT) for v in vals)))
    result = {"definition": DEFINITION, "kinds": sorted(KINDS), "iwad_size_bytes": size,
              "iwad_sha256": sha256_file(args.iwad), "script_sha256": sha256_file(__file__), "maps": maps}
    if args.check:
        with open(args.check) as fh:
            ok = compare(json.load(fh), result)
        print("%s: results %s %s" % ("PASS" if ok else "FAIL", "match" if ok else "differ from", args.check))
        return 0 if ok else 1
    text = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
