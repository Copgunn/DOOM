#!/usr/bin/env python3
"""Independent verifier for the M3h9c synthetic game WAD (stdlib only; imports nothing from
the repository, in particular not the generator m3h9c_wad.py or the room builders).

    m3h9c_check_wad.py WAD [--dump-counts]     exits nonzero on the first violated rule

Re-parses every lump with its linuxdoom-1.10 format (w_wad.c, r_data.c, p_setup.c,
p_switch.c, p_spec.c; citations in m3h9c_wad.py) and checks: directory extents and name
form, the required lumps and no TEXTURE2, PNAMES/TEXTURE1/patch consistency (in-range
columnofs, terminated columns), every alphSwitchList name of episodes 1-2, complete
animation ranges, side textures and sector flats defined, lump record sizes and index
ranges, REJECT and BLOCKMAP sizes, THING counts against the admitted bounds, sector heights
at every THING (point location through the nodes), no embedded barrel or player, at most
eight special two-sided lines in any 2x2 BLOCKMAP window, the player start among the
THINGs and a clear straight line from the start to the exit switch within use range."""
import struct
import sys

# alphSwitchList (p_switch.c:46-90), copied here: (name1, name2, episode)
ALPH = [('SW1BRCOM', 'SW2BRCOM', 1), ('SW1BRN1', 'SW2BRN1', 1), ('SW1BRN2', 'SW2BRN2', 1), ('SW1BRNGN', 'SW2BRNGN', 1),
        ('SW1BROWN', 'SW2BROWN', 1), ('SW1COMM', 'SW2COMM', 1), ('SW1COMP', 'SW2COMP', 1), ('SW1DIRT', 'SW2DIRT', 1),
        ('SW1EXIT', 'SW2EXIT', 1), ('SW1GRAY', 'SW2GRAY', 1), ('SW1GRAY1', 'SW2GRAY1', 1), ('SW1METAL', 'SW2METAL', 1),
        ('SW1PIPE', 'SW2PIPE', 1), ('SW1SLAD', 'SW2SLAD', 1), ('SW1STARG', 'SW2STARG', 1), ('SW1STON1', 'SW2STON1', 1),
        ('SW1STON2', 'SW2STON2', 1), ('SW1STONE', 'SW2STONE', 1), ('SW1STRTN', 'SW2STRTN', 1),
        ('SW1BLUE', 'SW2BLUE', 2), ('SW1CMT', 'SW2CMT', 2), ('SW1GARG', 'SW2GARG', 2), ('SW1GSTON', 'SW2GSTON', 2),
        ('SW1HOT', 'SW2HOT', 2), ('SW1LION', 'SW2LION', 2), ('SW1SATYR', 'SW2SATYR', 2), ('SW1SKIN', 'SW2SKIN', 2),
        ('SW1VINE', 'SW2VINE', 2), ('SW1WOOD', 'SW2WOOD', 2),
        ('SW1PANEL', 'SW2PANEL', 3), ('SW1ROCK', 'SW2ROCK', 3), ('SW1MET2', 'SW2MET2', 3), ('SW1WDMET', 'SW2WDMET', 3),
        ('SW1BRIK', 'SW2BRIK', 3), ('SW1MOD1', 'SW2MOD1', 3), ('SW1ZIM', 'SW2ZIM', 3), ('SW1STON6', 'SW2STON6', 3),
        ('SW1TEK', 'SW2TEK', 3), ('SW1MARB', 'SW2MARB', 3), ('SW1SKULL', 'SW2SKULL', 3)]
# animdefs (p_spec.c:98-118): flats (start, end) -- a present start needs the whole range
ANIM_FLATS = [('NUKAGE1', 'NUKAGE3'), ('FWATER1', 'FWATER4'), ('SWATER1', 'SWATER4'), ('LAVA1', 'LAVA4'), ('BLOOD1', 'BLOOD3')]
KIND = {2001: 'shotgun', 2007: 'clip', 2008: 'shells', 2011: 'stimpack', 2012: 'medikit', 2014: 'health bonus',
        2015: 'armor bonus', 2018: 'green armor', 2019: 'blue armor', 2048: 'bullet box', 2049: 'shell box',
        3004: 'zombieman', 3001: 'imp', 2035: 'barrel', 2028: 'floor lamp', 48: 'tech pillar', 35: 'candelabra',
        24: 'blood pool', 15: 'dead player', 10: 'bloody mess 10', 12: 'bloody mess 12', 1: 'player 1 start'}
MONSTERS, BARRELS = {3004, 3001}, {2035}
DECOR = {2028, 48, 35, 24, 15, 10, 12}
PICK = {2001, 2007, 2008, 2011, 2012, 2014, 2015, 2018, 2019, 2048, 2049}
RADIUS = {3004: 20, 3001: 20, 2035: 10, 1: 16, 2028: 16, 48: 16, 35: 16}
MAXMAP = ['E1M1', 'THINGS', 'LINEDEFS', 'SIDEDEFS', 'VERTEXES', 'SEGS', 'SSECTORS', 'NODES', 'SECTORS', 'REJECT', 'BLOCKMAP']


def fail(msg):
    print('FAIL: ' + msg)
    sys.exit(1)


def need(cond, msg):
    if not cond:
        fail(msg)


def name_of(raw):
    return raw.split(b'\0', 1)[0].decode('ascii')


def main():
    path = sys.argv[1]
    dump = '--dump-counts' in sys.argv
    d = open(path, 'rb').read()
    ident, n, ofs = struct.unpack_from('<4sii', d, 0)
    need(ident == b'IWAD', 'identification')
    need(0 <= ofs <= len(d) and ofs + 16 * n == len(d), 'directory extent')
    lumps = []
    for i in range(n):
        pos, size, nm = struct.unpack_from('<ii8s', d, ofs + 16 * i)
        need(0 <= pos and pos + size <= ofs, 'lump %d extent' % i)
        raw = nm
        need(raw == raw.rstrip(b'\0').ljust(8, b'\0') and b'\0' not in raw.rstrip(b'\0') and raw.rstrip(b'\0') == raw.rstrip(b'\0').upper(), 'name form %r' % raw)
        lumps.append((name_of(nm), pos, size))
    names = [x[0] for x in lumps]

    def find(nm):               # W_CheckNumForName: scans backwards (w_wad.c:377)
        for i in range(n - 1, -1, -1):
            if names[i] == nm.upper()[:8]:
                return i
        return -1

    def body(nm):
        i = find(nm)
        need(i >= 0, 'missing lump ' + nm)
        return d[lumps[i][1]:lumps[i][1] + lumps[i][2]]

    for req in ('PLAYPAL', 'COLORMAP', 'S_START', 'S_END', 'F_START', 'F_END', 'PNAMES', 'TEXTURE1', 'F_SKY1'):
        need(find(req) >= 0, 'missing ' + req)
    need(find('TEXTURE2') == -1, 'TEXTURE2 must be absent')
    need(len(body('PLAYPAL')) == 768 and len(body('COLORMAP')) == 34 * 256, 'PLAYPAL/COLORMAP size')
    need(find('S_START') + 1 == find('S_END'), 'S_START/S_END not empty')
    fs, fe = find('F_START'), find('F_END')
    need(fs < fe, 'F_START after F_END')
    flats = {names[i] for i in range(fs + 1, fe)}
    need(all(lumps[i][2] == 4096 for i in range(fs + 1, fe)), 'flat size')
    for a, b in ANIM_FLATS:     # P_InitPicAnims (p_spec.c:131-176)
        if find(a) >= 0:
            ia, ib = find(a), find(b)
            need(ib >= ia and ia > fs and ib < fe, 'animation range ' + a)
    need(find('F_SKY1') in range(fs + 1, fe), 'F_SKY1 outside flats')

    # PNAMES / TEXTURE1 (r_data.c:437-527)
    pn = body('PNAMES')
    npatch = struct.unpack_from('<i', pn, 0)[0]
    need(len(pn) == 4 + 8 * npatch, 'PNAMES size')
    patchnames = [name_of(pn[4 + 8 * i:12 + 8 * i]) for i in range(npatch)]
    for p in patchnames:
        need(find(p) >= 0, 'PNAMES entry unresolved ' + p)
    t1 = body('TEXTURE1')
    nt = struct.unpack_from('<i', t1, 0)[0]
    texs = {}
    patches_used = set()
    for i in range(nt):
        off = struct.unpack_from('<i', t1, 4 + 4 * i)[0]
        need(0 <= off <= len(t1), 'texture directory')
        nm, masked, w, h, cdir, pc = struct.unpack_from('<8siHHiH'.replace('H', 'h'), t1, off)
        tn = name_of(nm)
        need(off + 22 + 10 * pc <= len(t1), 'texture extent ' + tn)
        need(w > 0 and w & (w - 1) == 0 and h > 0, 'texture %s width not a power of two' % tn)
        need(pc >= 1, 'texture without patches ' + tn)
        covered = [0] * w
        for j in range(pc):
            ox, oy, pidx, step, cmap = struct.unpack_from('<5h', t1, off + 22 + 10 * j)
            need(0 <= pidx < npatch, 'patch index in ' + tn)
            ln = find(patchnames[pidx])
            need(ln >= 0 and ln > 0, 'patch unresolved in ' + tn)
            pb = d[lumps[ln][1]:lumps[ln][1] + lumps[ln][2]]
            pw, ph, lo, to = struct.unpack_from('<4h', pb, 0)
            need(pw > 0 and len(pb) >= 8 + 4 * pw, 'patch header ' + patchnames[pidx])
            patches_used.add(patchnames[pidx])
            for x in range(max(ox, 0), min(ox + pw, w)):
                co = struct.unpack_from('<i', pb, 8 + 4 * (x - ox))[0]
                need(8 + 4 * pw <= co < len(pb) and co + 3 <= 0xFFFF + 3, 'columnofs range in ' + tn)
                # column must be a valid post list ending in 0xFF
                p = co
                while pb[p] != 0xFF:
                    need(p + 4 <= len(pb), 'post overrun')
                    p += pb[p + 1] + 4
                    need(p < len(pb), 'post overrun')
                covered[x] += 1
        need(all(covered), 'column without a patch in ' + tn)
        texs[tn] = (w, h)
    need(len(texs) == nt, 'duplicate texture names')
    for a, b, ep in ALPH:
        if ep <= 2:             # P_InitSwitchList, registered (p_switch.c:107-148)
            need(a in texs and b in texs, 'switch texture missing %s/%s' % (a, b))
    for t in ('SKY1', 'TESTWALL'):
        need(t in texs, 'missing texture ' + t)
    need(next(iter(texs)) == 'TESTWALL', 'texture 0 should be the plain wall')

    # map lumps in vanilla order right after E1M1 (p_setup.c ML_* numbering)
    im = find('E1M1')
    need(im >= 0 and names[im:im + 11] == MAXMAP, 'map lump order')
    need(lumps[im][2] == 0, 'E1M1 marker not empty')

    def ml(k):
        return d[lumps[im + k][1]:lumps[im + k][1] + lumps[im + k][2]]
    TH, LD, SD, VX, SG, SS, ND, SC, RJ, BM = [ml(k) for k in range(1, 11)]
    need(len(TH) % 10 == 0 and len(LD) % 14 == 0 and len(SD) % 30 == 0 and len(VX) % 4 == 0 and len(SG) % 12 == 0 and
         len(SS) % 4 == 0 and len(ND) % 28 == 0 and len(SC) % 26 == 0, 'lump record sizes')
    verts = [struct.unpack_from('<2h', VX, 4 * i) for i in range(len(VX) // 4)]
    sides = []
    for i in range(len(SD) // 30):
        r = struct.unpack_from('<2h8s8s8sh', SD, 30 * i)
        sides.append(dict(top=name_of(r[2]), bot=name_of(r[3]), mid=name_of(r[4]), sector=r[5]))
    secs = []
    for i in range(len(SC) // 26):
        r = struct.unpack_from('<2h8s8s3h', SC, 26 * i)
        secs.append(dict(floor=r[0], ceil=r[1], fpic=name_of(r[2]), cpic=name_of(r[3]), light=r[4], special=r[5], tag=r[6]))
    lines = []
    for i in range(len(LD) // 14):
        r = struct.unpack_from('<7h', LD, 14 * i)
        lines.append(dict(a=r[0], b=r[1], flags=r[2], special=r[3], tag=r[4], s0=r[5], s1=r[6]))
    for L in lines:
        need(0 <= L['a'] < len(verts) and 0 <= L['b'] < len(verts), 'line vertex range')
        need(0 <= L['s0'] < len(sides) and (L['s1'] == -1 or 0 <= L['s1'] < len(sides)), 'side range')
        need(bool(L['flags'] & 4) == (L['s1'] != -1), 'ML_TWOSIDED mismatch')
    for s in sides:
        need(0 <= s['sector'] < len(secs), 'side sector range')
        for k in ('top', 'bot', 'mid'):    # R_CheckTextureNumForName: '-' is no texture (r_data.c:~690)
            need(s[k] == '-' or s[k] in texs, 'sidedef texture undefined: ' + s[k])
    for s in secs:
        need(s['fpic'] in flats and s['cpic'] in flats, 'sector flat undefined %s/%s' % (s['fpic'], s['cpic']))
    segs = [struct.unpack_from('<6h', SG, 12 * i) for i in range(len(SG) // 12)]
    for a, b, ang, ln, sd, off in segs:
        need(0 <= a < len(verts) and 0 <= b < len(verts) and 0 <= ln < len(lines) and sd in (0, 1), 'seg fields')
    for cnt, first in (struct.unpack_from('<2h', SS, 4 * i) for i in range(len(SS) // 4)):
        need(cnt > 0 and 0 <= first and first + cnt <= len(segs), 'subsector range')
    nodes = [struct.unpack_from('<12h2H', ND, 28 * i) for i in range(len(ND) // 28)]
    for nd in nodes:
        for c in nd[-2:]:
            need((c & 0x8000 and (c & 0x7FFF) < len(SS) // 4) or (not c & 0x8000 and c < len(nodes)), 'node child')
    nsec = len(secs)
    need(len(RJ) == (nsec * nsec + 7) // 8, 'REJECT size')
    bx, by, bw, bh = struct.unpack_from('<4h', BM, 0)
    need(len(BM) >= 8 + 2 * bw * bh, 'BLOCKMAP header')
    bw_words = struct.unpack('<%dh' % (len(BM) // 2), BM)
    cells = []
    for c in range(bw * bh):
        o = bw_words[4 + c]
        need(0 < o < len(bw_words), 'blockmap offset')
        lst = []
        while bw_words[o] != -1:
            lst.append(bw_words[o]); o += 1
            need(o < len(bw_words), 'blockmap list unterminated')
        need(all(0 <= x < len(lines) for x in lst), 'blockmap line range')
        cells.append(lst)

    # THINGS
    things = [struct.unpack_from('<5h', TH, 10 * i) for i in range(len(TH) // 10)]
    starts = [i for i, t in enumerate(things) if t[3] == 1]
    need(len(starts) == 1, 'need exactly one player 1 start')
    need(starts[0] != len(things) - 1, 'player start must not be last')
    angs = {t[2] for t in things}
    need(any(a >= 180 for a in angs), 'no angle >= 180')

    def medium(t):
        return not t[4] & 16 and bool(t[4] & 2)      # P_SpawnMapThing filter (p_mobj.c:740-750), skill 2
    sp = [t for t in things if medium(t)]
    cnt = {}
    for t in sp:
        cnt[t[3]] = cnt.get(t[3], 0) + 1
    unknown = {t[3] for t in things} - set(KIND)
    need(not unknown, 'unadmitted thing types %s' % sorted(unknown))
    nm, npk, nbr, ndc = (sum(v for k, v in cnt.items() if k in S) for S in (MONSTERS, PICK, BARRELS, DECOR))
    need(nm <= 8 and npk <= 64 and nbr <= 16 and ndc <= 32, 'wire-21 load bounds')

    # geometry facts: point location, height fit, solid-embedding (spawn rule S'), 2x2 window rule
    def point_on_side(x, y, nd):    # R_PointOnSide (r_main.c:162-212), exact in map units
        nx, ny, ndx, ndy = nd[:4]
        if not ndx:
            return int(ndy > 0) if x <= nx else int(ndy < 0)
        if not ndy:
            return int(ndx < 0) if y <= ny else int(ndx > 0)
        left, right = ndy * (x - nx), (y - ny) * ndx
        return 0 if right < left else 1

    def locate(x, y):               # R_PointInSubsector (r_main.c:800-823): children are nd[12], nd[13]
        c = len(nodes) - 1
        while not c & 0x8000:
            c = nodes[c][12 + point_on_side(x, y, nodes[c])]
        return c & 0x7FFF
    ss = [struct.unpack_from('<2h', SS, 4 * i) for i in range(len(SS) // 4)]

    def sector_at(x, y):
        c = locate(x, y)
        cnt_, first = ss[c]
        a, b, ang, ln, sd, off = segs[first]
        L = lines[ln]
        return sides[L['s1'] if sd else L['s0']]['sector']
    minh = min(secs[sector_at(t[0], t[1])]['ceil'] - secs[sector_at(t[0], t[1])]['floor'] for t in sp)
    need(minh >= 72, 'spawn height fit: minimum sector height %d' % minh)
    h_of = {1: 56, 2035: 42, 3004: 56, 3001: 56}
    for t in sp:
        s = secs[sector_at(t[0], t[1])]
        need(s['ceil'] - s['floor'] >= h_of.get(t[3], 16), 'thing taller than its sector')
    one_sided = [(verts[L['a']], verts[L['b']]) for L in lines if L['s1'] == -1]

    def seg_meets_box(p, q, cx, cy, r):
        # segment vs closed square: Liang-Barsky
        x0, y0, x1, y1 = p[0], p[1], q[0], q[1]
        t0, t1 = 0.0, 1.0
        for pp, qq in ((-(x1 - x0), x0 - (cx - r)), (x1 - x0, (cx + r) - x0), (-(y1 - y0), y0 - (cy - r)), (y1 - y0, (cy + r) - y0)):
            if pp == 0:
                if qq < 0: return False
            else:
                t = qq / pp
                if pp < 0:
                    if t > t1: return False
                    t0 = max(t0, t)
                else:
                    if t < t0: return False
                    t1 = min(t1, t)
        return True
    for t in sp:
        if t[3] in BARRELS or t[3] == 1:
            r = RADIUS[t[3]]
            need(not any(seg_meets_box(p, q, t[0], t[1], r) for p, q in one_sided), 'embedded barrel/player at %s' % (t[3],))
    # rule 6: distinct two-sided special lines in any 2x2 cell window <= 8
    worst = 0
    for cy in range(max(bh - 1, 1)):
        for cx in range(max(bw - 1, 1)):
            win = set()
            for yy in range(cy, min(cy + 2, bh)):
                for xx in range(cx, min(cx + 2, bw)):
                    win |= set(cells[yy * bw + xx])
            worst = max(worst, len([i for i in win if lines[i]['special'] and lines[i]['s1'] != -1]))
    need(worst <= 8, 'rule 6 window has %d special lines' % worst)

    # exit: one S1 type-11 line with a switch texture, straight line from the start
    exits = [L for L in lines if L['special'] == 11]
    need(len(exits) == 1 and exits[0]['s1'] == -1, 'one one-sided S1 exit line')
    swn = {x for a, b, e in ALPH for x in (a, b)}
    need(sides[exits[0]['s0']]['mid'] in swn, 'exit switch texture not in alphSwitchList')
    st = things[starts[0]]
    ea, eb = verts[exits[0]['a']], verts[exits[0]['b']]
    mid = ((ea[0] + eb[0]) // 2, (ea[1] + eb[1]) // 2)
    blockers = [(p, q) for p, q in one_sided if (p, q) != (ea, eb) and seg_inter((st[0], st[1]), mid, p, q)]
    need(not blockers, 'start to exit crosses a wall')
    need(((st[0] - mid[0]) ** 2 + (st[1] - mid[1]) ** 2) ** 0.5 < 64 + 16, 'exit beyond use range (64 + body)')
    solids = [t for t in sp if t[3] in (set(RADIUS) - {1, 3004, 3001} | MONSTERS | BARRELS)]
    for t in solids:
        need(dist_pt_seg((t[0], t[1]), (st[0], st[1]), mid) > RADIUS[t[3]] + 16, 'solid on the path to the exit')

    mover = set()
    for L in lines:
        if L['special'] == 1 and L['s1'] != -1:
            mover.add(sides[L['s0']]['sector']); mover.add(sides[L['s1']]['sector'])
    doors = {sides[L['s1']]['sector'] for L in lines if L['special'] == 1} | {sides[L['s0']]['sector'] for L in lines if L['special'] == 1}
    doors = {s for s in doors if secs[s]['ceil'] == secs[s]['floor']}
    lifts = {s for s in range(nsec) if any(secs[s]['tag'] == L['tag'] and L['tag'] for L in lines if L['special'] in (62, 88))}
    movers = len(doors) + len(lifts)
    scrollers = [L for L in lines if L['special'] == 48]
    sectspec = {}
    for s in secs:
        sectspec[s['special']] = sectspec.get(s['special'], 0) + 1
    linespec = {}
    for L in lines:
        linespec[L['special']] = linespec.get(L['special'], 0) + 1
    need(movers >= 9 and len(scrollers) >= 8 and all(L['s1'] != -1 for L in scrollers), 'movers/scrollers')
    for sp_ in (1, 2, 3, 7, 8, 9, 12, 17):
        need(sectspec.get(sp_), 'missing sector special %d' % sp_)
    need(linespec.get(1) and (linespec.get(88) or linespec.get(62)), 'missing DR door / lift line')
    need(any(s['cpic'] == 'F_SKY1' for s in secs), 'no sky sector')

    print('OK %s: %d bytes, %d lumps' % (path, len(d), n))
    print('textures %d (switch names %d), patches %d (PNAMES %d), flats %d' %
          (nt, sum(2 for a, b, e in ALPH if e <= 2), len(patches_used), npatch, len(flats)))
    print('map: sectors %d, lines %d (two-sided %d), sides %d, vertices %d, segs %d, subsectors %d, nodes %d, blockmap %dx%d' %
          (nsec, len(lines), sum(1 for L in lines if L['s1'] != -1), len(sides), len(verts), len(segs), len(ss), len(nodes), bw, bh))
    print('things: total %d, spawned on medium %d (monsters %d, pickups %d, barrels %d, decorations %d, player start 1)' %
          (len(things), len(sp), nm, npk, nbr, ndc))
    print('player start is THING index %d of %d (%d follow); angles >= 180 present; distinct angles %d' %
          (starts[0], len(things), len(things) - 1 - starts[0], len(angs)))
    print('movers %d (doors %d, lifts %d), scrollers %d (two-sided), exit lines %d, max special lines in a 2x2 window %d, min thing sector height %d' %
          (movers, len(doors), len(lifts), len(scrollers), len(exits), worst, minh))
    if dump:
        print('by kind (medium): ' + ', '.join('%s %d' % (KIND[k], v) for k, v in sorted(cnt.items(), key=lambda kv: KIND[kv[0]])))
        print('sector specials: ' + ', '.join('%d x%d' % kv for kv in sorted(sectspec.items())))
        print('line specials: ' + ', '.join('%d x%d' % kv for kv in sorted(linespec.items())))
        print('sector flats: ' + ', '.join(sorted({s['fpic'] for s in secs} | {s['cpic'] for s in secs})))


def seg_inter(a, b, c, d):
    def o(p, q, r):
        v = (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
        return (v > 0) - (v < 0)
    return o(a, b, c) != o(a, b, d) and o(c, d, a) != o(c, d, b)


def dist_pt_seg(p, a, b):
    ax, ay, bx, by = a[0], a[1], b[0], b[1]
    dx, dy = bx - ax, by - ay
    t = 0 if dx == dy == 0 else max(0, min(1, ((p[0] - ax) * dx + (p[1] - ay) * dy) / (dx * dx + dy * dy)))
    return ((p[0] - ax - t * dx) ** 2 + (p[1] - ay - t * dy) ** 2) ** 0.5


if __name__ == '__main__':
    main()
