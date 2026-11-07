#!/usr/bin/env python3
"""Independent verifier for the M4a synthetic mechanism-group WADs (stdlib only; imports
nothing from the repository: not the generator m4a_wad.py, not the room builders, not the
M3h9c checker).

    m4a_check_wad.py WAD [--name NAME] [--dump-counts]   exits nonzero on the first violated rule
    m4a_check_wad.py --check                             every WAD of tests/reference/m4a_manifest.json

Re-parses every lump with its linuxdoom-1.10 format (w_wad.c, r_data.c, p_setup.c,
p_switch.c, p_spec.c) and reads the THING table (doomednum, radius, height, flags) from the
genuine linuxdoom-1.10/info.c, then checks:

- formats: directory extents and name form; the required lumps and no TEXTURE2;
  PNAMES/TEXTURE1/patch consistency; every alphSwitchList name of episodes 1-2
  (P_InitSwitchList, registered, p_switch.c:107-148); complete animation ranges; side
  textures and sector flats defined; record sizes, index ranges, ML_TWOSIDED against the
  second side; REJECT and BLOCKMAP sizes; the map lumps in vanilla order after the slot
  named by the manifest;
- things: exactly one player 1 start, not the last THING; every type in info.c; no
  teleport landing (14, MT_TELEPORTMAN, deferred to M4b); spawned counts on medium single
  player equal to the manifest's; every spawned thing fits its sector's height (point
  location through the nodes, R_PointInSubsector); no player, monster or barrel touching a
  one-sided wall;
- lines: every special in the part-1 set; every tagged special's tag on some sector;
  every one-sided S1/SR/GR line (7, 9, 11, 18, 20, 23, 46, 51, 62, 70, 103) carries an
  alphSwitchList texture on its front side, and no other line does (P_ChangeSwitchTexture,
  p_switch.c:197-256, scans top, middle, bottom); every manual door line (1, 26-28, 31-34)
  is two-sided with a closed back sector (EV_VerticalDoor raises sides[sidenum[1]].sector,
  p_doors.c:364, 413); at most eight special two-sided lines in any 2x2 BLOCKMAP window;
- mechanisms (re-derived here from the parsed map, by line index as P_GroupLines builds
  sec->lines, p_setup.c:519-575): the stair chains of every 7/8 tag (EV_BuildStairs,
  p_floor.c:453-554, with a declared busy set), the donut sectors of every 9 tag
  (EV_DoDonut, p_spec.c:1163-1221: s1->lines[0] two-sided, every s2 line two-sided, the
  first s2 line whose back is not s1 has a back sector), the front-sector flat of every
  20/22 line against its target's (p_plats.c:185-195), and the qualifying neighbours of
  every 18/20/22 target (P_FindNextHighestFloor, p_spec.c:326-375), with the per-WAD
  expectations in EXPECT (the comb's 21 neighbours, the 21st the lowest; staircase A
  8..56 with the busy neighbour bumping the height; the plat flats).
"""
import json, re, struct, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / 'tests/reference/m4a_manifest.json'
ALPH = [('SW1BRCOM', 1), ('SW1BRN1', 1), ('SW1BRN2', 1), ('SW1BRNGN', 1), ('SW1BROWN', 1), ('SW1COMM', 1), ('SW1COMP', 1),
        ('SW1DIRT', 1), ('SW1EXIT', 1), ('SW1GRAY', 1), ('SW1GRAY1', 1), ('SW1METAL', 1), ('SW1PIPE', 1), ('SW1SLAD', 1),
        ('SW1STARG', 1), ('SW1STON1', 1), ('SW1STON2', 1), ('SW1STONE', 1), ('SW1STRTN', 1), ('SW1BLUE', 2), ('SW1CMT', 2),
        ('SW1GARG', 2), ('SW1GSTON', 2), ('SW1HOT', 2), ('SW1LION', 2), ('SW1SATYR', 2), ('SW1SKIN', 2), ('SW1VINE', 2),
        ('SW1WOOD', 2)]   # alphSwitchList, p_switch.c:46-90, episodes 1-2 (name2 is SW2...)
SWITCH_NAMES = {n for a, _ in ALPH for n in (a, 'SW2' + a[3:])}
ANIM_FLATS = [('NUKAGE1', 'NUKAGE3'), ('FWATER1', 'FWATER4'), ('SWATER1', 'SWATER4'), ('LAVA1', 'LAVA4'), ('BLOOD1', 'BLOOD3')]
MAP_LUMPS = ['THINGS', 'LINEDEFS', 'SIDEDEFS', 'VERTEXES', 'SEGS', 'SSECTORS', 'NODES', 'SECTORS', 'REJECT', 'BLOCKMAP']
PART1 = {2, 5, 7, 8, 9, 11, 16, 18, 20, 22, 23, 26, 27, 28, 31, 32, 33, 34, 35, 46, 51, 62, 70, 76, 82, 86, 90, 91, 98, 103}
SWITCHED = {7, 9, 11, 18, 20, 23, 46, 51, 62, 70, 103}
MANUAL_DOORS = {1, 26, 27, 28, 31, 32, 33, 34}
UNTAGGED = MANUAL_DOORS | {11, 51}
ML_TWOSIDED, MF_SOLID, MF_SHOOTABLE, MF_COUNTKILL = 4, 2, 4, 0x400000
# Per-WAD mechanism expectations (sector indices, as declared in the generator's registry).
EXPECT = {
    'FLOORS': dict(neighbours={'max_other': 19, 'comb_tag': 9, 'comb_count': 21}),
    'PLATS': dict(plat_flats={2: ('TESTFLR0', 'NUKAGE1'), 3: ('FLOOR7_1', 'NUKAGE1'), 4: ('FLOOR7_1', 'NUKAGE1')}),
    'STAIRS': dict(stairs={7: dict(busy_tag=9, heights=[8, 16, 24, 40, 48, 56]), 8: dict(heights=[8, 16, 24, 32, 40]),
                           10: dict(heights=[8, 16, 24, 32, 40])},
                   order=dict(sector=3, before=8, after=4), donut={11: dict(s3_floor=0, s3_flat='FLOOR7_1')}),
}


def fail(msg):
    print('FAIL: ' + msg)
    sys.exit(1)


def need(cond, msg):
    if not cond: fail(msg)


def name_of(raw):
    return raw.split(b'\0', 1)[0].decode('ascii')


def mobjinfo():
    """doomednum -> dict(name, radius, height, flags) from linuxdoom-1.10/info.c."""
    text = (ROOT / 'linuxdoom-1.10/info.c').read_text(encoding='latin-1')
    body = text[text.index('mobjinfo_t mobjinfo[NUMMOBJTYPES]'):]
    flagval = {m[0]: int(m[1], 0) for m in re.findall(r'(MF_\w+)\s*=\s*(0x[0-9a-fA-F]+|\d+)', (ROOT / 'linuxdoom-1.10/p_mobj.h').read_text())}
    out = {}
    for name, block in re.findall(r'\{\s*// (MT_\w+)(.*?)\n    \}', body, re.S):
        f = {k: v.strip() for v, k in re.findall(r'^\t(.*?),?\t+// (\w+)\s*$', block, re.M)}
        flags = 0
        for t in f['flags'].split('|'):
            t = t.strip()
            if t and t != '0': flags |= flagval[t]
        num = lambda v: int(v.replace('*FRACUNIT', ''))
        out[int(f['doomednum'])] = dict(name=name, radius=num(f['radius']), height=num(f['height']), flags=flags)
    return out


class Map:
    def __init__(self, path, slot):
        d = Path(path).read_bytes()
        self.d = d
        ident, n, ofs = struct.unpack_from('<4sii', d, 0)
        need(ident == b'IWAD', 'identification')
        need(0 <= ofs <= len(d) and ofs + 16 * n == len(d), 'directory extent')
        self.lumps = []
        for i in range(n):
            pos, size, nm = struct.unpack_from('<ii8s', d, ofs + 16 * i)
            need(0 <= pos and pos + size <= ofs, 'lump %d extent' % i)
            s = nm.rstrip(b'\0')
            need(nm == s.ljust(8, b'\0') and b'\0' not in s and s == s.upper(), 'name form %r' % nm)
            self.lumps.append((name_of(nm), pos, size))
        self.names = [x[0] for x in self.lumps]
        self.check_resources()
        im = self.find(slot)
        need(im >= 0 and self.names[im + 1:im + 11] == MAP_LUMPS and self.lumps[im][2] == 0, 'map lumps after ' + slot)
        TH, LD, SD, VX, SG, SS, ND, SC, RJ, BM = [self.body_at(im + k) for k in range(1, 11)]
        need(len(TH) % 10 == 0 and len(LD) % 14 == 0 and len(SD) % 30 == 0 and len(VX) % 4 == 0 and len(SG) % 12 == 0 and
             len(SS) % 4 == 0 and len(ND) % 28 == 0 and len(SC) % 26 == 0, 'lump record sizes')
        self.verts = [struct.unpack_from('<2h', VX, 4 * i) for i in range(len(VX) // 4)]
        self.sides = []
        for i in range(len(SD) // 30):
            r = struct.unpack_from('<2h8s8s8sh', SD, 30 * i)
            self.sides.append(dict(top=name_of(r[2]), bot=name_of(r[3]), mid=name_of(r[4]), sector=r[5]))
        self.secs = []
        for i in range(len(SC) // 26):
            r = struct.unpack_from('<2h8s8s3h', SC, 26 * i)
            self.secs.append(dict(floor=r[0], ceil=r[1], fpic=name_of(r[2]), cpic=name_of(r[3]), light=r[4], special=r[5], tag=r[6]))
        self.lines = []
        for i in range(len(LD) // 14):
            r = struct.unpack_from('<7h', LD, 14 * i)
            self.lines.append(dict(a=r[0], b=r[1], flags=r[2], special=r[3], tag=r[4], s0=r[5], s1=r[6]))
        for L in self.lines:
            need(0 <= L['a'] < len(self.verts) and 0 <= L['b'] < len(self.verts), 'line vertex range')
            need(0 <= L['s0'] < len(self.sides) and (L['s1'] == -1 or 0 <= L['s1'] < len(self.sides)), 'side range')
            need(bool(L['flags'] & ML_TWOSIDED) == (L['s1'] != -1), 'ML_TWOSIDED mismatch')
            L['front'] = self.sides[L['s0']]['sector']
            L['back'] = self.sides[L['s1']]['sector'] if L['s1'] != -1 else None
        for s in self.sides:
            need(0 <= s['sector'] < len(self.secs), 'side sector range')
            for k in ('top', 'bot', 'mid'):
                need(s[k] == '-' or s[k] in self.texs, 'sidedef texture undefined: ' + s[k])
        for s in self.secs:
            need(s['fpic'] in self.flats and s['cpic'] in self.flats, 'sector flat undefined')
        self.segs = [struct.unpack_from('<6h', SG, 12 * i) for i in range(len(SG) // 12)]
        for a, b, ang, ln, sd, off in self.segs:
            need(0 <= a < len(self.verts) and 0 <= b < len(self.verts) and 0 <= ln < len(self.lines) and sd in (0, 1), 'seg fields')
        self.ss = [struct.unpack_from('<2h', SS, 4 * i) for i in range(len(SS) // 4)]
        for cnt, first in self.ss:
            need(cnt > 0 and 0 <= first and first + cnt <= len(self.segs), 'subsector range')
        self.nodes = [struct.unpack_from('<12h2H', ND, 28 * i) for i in range(len(ND) // 28)]
        for nd in self.nodes:
            for c in nd[-2:]:
                need((c & 0x8000 and (c & 0x7FFF) < len(self.ss)) or (not c & 0x8000 and c < len(self.nodes)), 'node child')
        n = len(self.secs)
        need(len(RJ) == (n * n + 7) // 8, 'REJECT size')
        bx, by, bw, bh = struct.unpack_from('<4h', BM, 0)
        words = struct.unpack('<%dh' % (len(BM) // 2), BM)
        need(len(words) >= 4 + bw * bh, 'BLOCKMAP header')
        self.cells, self.bw, self.bh = [], bw, bh
        for c in range(bw * bh):
            o = words[4 + c]
            need(0 < o < len(words), 'blockmap offset')
            lst = []
            while words[o] != -1:
                lst.append(words[o]); o += 1
                need(o < len(words), 'blockmap list unterminated')
            need(all(0 <= x < len(self.lines) for x in lst), 'blockmap line range')
            self.cells.append(lst)
        self.things = [struct.unpack_from('<5h', TH, 10 * i) for i in range(len(TH) // 10)]

    def find(self, nm):          # W_CheckNumForName scans backwards (w_wad.c:377)
        for i in range(len(self.names) - 1, -1, -1):
            if self.names[i] == nm.upper()[:8]: return i
        return -1

    def body_at(self, i):
        return self.d[self.lumps[i][1]:self.lumps[i][1] + self.lumps[i][2]]

    def body(self, nm):
        i = self.find(nm)
        need(i >= 0, 'missing lump ' + nm)
        return self.body_at(i)

    def check_resources(self):
        for req in ('PLAYPAL', 'COLORMAP', 'S_START', 'S_END', 'F_START', 'F_END', 'PNAMES', 'TEXTURE1', 'F_SKY1'):
            need(self.find(req) >= 0, 'missing ' + req)
        need(self.find('TEXTURE2') == -1, 'TEXTURE2 must be absent')
        need(len(self.body('PLAYPAL')) == 768 and len(self.body('COLORMAP')) == 34 * 256, 'PLAYPAL/COLORMAP size')
        need(self.find('S_START') + 1 == self.find('S_END'), 'S_START/S_END not empty')
        fs, fe = self.find('F_START'), self.find('F_END')
        need(fs < fe and all(self.lumps[i][2] == 4096 for i in range(fs + 1, fe)), 'flats')
        self.flats = {self.names[i] for i in range(fs + 1, fe)}
        for a, b in ANIM_FLATS:   # P_InitPicAnims (p_spec.c:131-176)
            if self.find(a) >= 0:
                need(fs < self.find(a) <= self.find(b) < fe, 'animation range ' + a)
        pn = self.body('PNAMES')
        npatch = struct.unpack_from('<i', pn, 0)[0]
        need(len(pn) == 4 + 8 * npatch, 'PNAMES size')
        patches = [name_of(pn[4 + 8 * i:12 + 8 * i]) for i in range(npatch)]
        for p in patches: need(self.find(p) >= 0, 'PNAMES entry unresolved ' + p)
        t1 = self.body('TEXTURE1')
        nt = struct.unpack_from('<i', t1, 0)[0]
        self.texs = []
        for i in range(nt):
            off = struct.unpack_from('<i', t1, 4 + 4 * i)[0]
            nm, masked, w, h, cdir, pc = struct.unpack_from('<8sihhih', t1, off)
            need(off + 22 + 10 * pc <= len(t1) and w > 0 and w & (w - 1) == 0 and h > 0 and pc >= 1, 'texture ' + name_of(nm))
            for j in range(pc):
                ox, oy, pidx, step, cmap = struct.unpack_from('<5h', t1, off + 22 + 10 * j)
                need(0 <= pidx < npatch, 'patch index')
                pb = self.body(patches[pidx])
                pw = struct.unpack_from('<h', pb, 0)[0]
                need(pw > 0 and len(pb) >= 8 + 4 * pw, 'patch header')
                for x in range(pw):
                    p = struct.unpack_from('<i', pb, 8 + 4 * x)[0]
                    need(8 + 4 * pw <= p < len(pb), 'columnofs range')
                    while pb[p] != 0xFF:
                        p += pb[p + 1] + 4
                        need(p < len(pb), 'post overrun')
            self.texs.append(name_of(nm))
        need(len(set(self.texs)) == nt and self.texs[0] == 'TESTWALL', 'texture names / texture 0')
        for a, ep in ALPH:
            need(a in self.texs and 'SW2' + a[3:] in self.texs, 'switch texture missing ' + a)
        need('SKY1' in self.texs, 'missing SKY1')

    # ---- geometry helpers
    def sector_lines(self, s):
        """sec->lines in P_GroupLines order (line index)."""
        return [i for i, L in enumerate(self.lines) if L['front'] == s or L['back'] == s]

    def locate(self, x, y):       # R_PointInSubsector (r_main.c:800-823) with R_PointOnSide (162-212)
        c = len(self.nodes) - 1
        while not c & 0x8000:
            nx, ny, ndx, ndy = self.nodes[c][:4]
            if not ndx: side = int(ndy > 0) if x <= nx else int(ndy < 0)
            elif not ndy: side = int(ndx < 0) if y <= ny else int(ndx > 0)
            else: side = 0 if (y - ny) * ndx < ndy * (x - nx) else 1
            c = self.nodes[c][12 + side]
        cnt, first = self.ss[c & 0x7FFF]
        a, b, ang, ln, sd, off = self.segs[first]
        L = self.lines[ln]
        return self.sides[L['s1'] if sd else L['s0']]['sector']


def stairs(m, tag, busy=()):
    """EV_BuildStairs build8 (p_floor.c:453-554) over the parsed map: [(sector, height)]."""
    out, done = [], set(busy)
    secnum = -1
    while True:
        nxt = [i for i in range(secnum + 1, len(m.secs)) if m.secs[i]['tag'] == tag]
        if not nxt: return out
        secnum = nxt[0]
        if secnum in done: continue
        height = m.secs[secnum]['floor'] + 8
        out.append((secnum, height)); done.add(secnum)
        texture = m.secs[secnum]['fpic']
        sec, ok = secnum, True
        while ok:
            ok = False
            for i in m.sector_lines(sec):
                L = m.lines[i]
                if not L['flags'] & ML_TWOSIDED or L['front'] != secnum: continue
                need(L['back'] is not None, 'two-sided stair line without a back sector')
                t = L['back']
                if m.secs[t]['fpic'] != texture: continue
                height += 8
                if t in done: continue
                sec = secnum = t
                out.append((t, height)); done.add(t); ok = True
                break


def donut(m, tag):
    """EV_DoDonut (p_spec.c:1163-1221): (s1, s2, s3) for each tagged s1, with the null checks."""
    out = []
    for s1 in (i for i, s in enumerate(m.secs) if s['tag'] == tag):
        first = m.lines[m.sector_lines(s1)[0]]
        need(first['back'] is not None, 'donut: s1->lines[0] is one-sided (getNextSector returns NULL)')
        s2 = first['back'] if first['front'] == s1 else first['front']
        need(all(m.lines[i]['back'] is not None for i in m.sector_lines(s2)), 'donut: a one-sided s2 line (the !flags & ML_TWOSIDED test never skips it)')
        cand = [m.lines[i]['back'] for i in m.sector_lines(s2) if m.lines[i]['back'] != s1]
        need(cand, 'donut: no s2 line without s1 on its back')
        out.append((s1, s2, cand[0]))
    return out


def next_highest(m, s):
    """Qualifying neighbours of P_FindNextHighestFloor (p_spec.c:329-375) in line order."""
    q = []
    for i in m.sector_lines(s):
        L = m.lines[i]
        o = L['back'] if L['front'] == s else L['front']
        if o is not None and m.secs[o]['floor'] > m.secs[s]['floor']: q.append(m.secs[o]['floor'])
    return q


def check(path, name, slot='E1M1', expect_things=None, dump=False):
    info = mobjinfo()
    m = Map(path, slot)
    # things
    th = m.things
    starts = [i for i, t in enumerate(th) if t[3] == 1]
    need(len(starts) == 1 and starts[0] != len(th) - 1, 'one player 1 start, not the last THING')
    need(all(t[3] in info or t[3] == 1 for t in th), 'unknown thing types %s' % sorted({t[3] for t in th} - set(info) - {1}))
    need(all(t[3] != 14 for t in th), 'teleport landing 14 present (deferred to M4b)')
    sp = [t for t in th if not t[4] & 16 and t[4] & 2]     # P_SpawnMapThing filter, medium single player
    cnt = {}
    for t in sp: cnt[str(t[3])] = cnt.get(str(t[3]), 0) + 1
    if expect_things is not None:
        need(dict(sorted(cnt.items(), key=lambda kv: int(kv[0]))) == expect_things, 'spawned counts differ from the manifest')
    one_sided = [(m.verts[L['a']], m.verts[L['b']]) for L in m.lines if L['s1'] == -1]
    for t in sp:
        s = m.secs[m.locate(t[0], t[1])]
        h = 56 if t[3] == 1 else info[t[3]]['height']
        need(s['ceil'] - s['floor'] >= h, 'thing %d at (%d,%d) taller than its sector' % (t[3], t[0], t[1]))
        kind = info.get(t[3], dict(flags=MF_SOLID | MF_SHOOTABLE, radius=16))
        if t[3] == 1 or kind['flags'] & MF_SHOOTABLE:
            r = 16 if t[3] == 1 else kind['radius']
            need(not any(box_meets(p, q, t[0], t[1], r) for p, q in one_sided), 'thing %d at (%d,%d) touches a wall' % (t[3], t[0], t[1]))
    # lines
    tags = {s['tag'] for s in m.secs}
    linespec = {}
    for L in m.lines:
        sp_ = L['special']
        if not sp_: continue
        linespec[sp_] = linespec.get(sp_, 0) + 1
        need(sp_ in PART1, 'special %d outside the part-1 set' % sp_)
        if sp_ not in UNTAGGED: need(L['tag'] and L['tag'] in tags, 'special %d tag %d on no sector' % (sp_, L['tag']))
        side = m.sides[L['s0']]
        has_switch = any(side[k] in SWITCH_NAMES for k in ('top', 'mid', 'bot'))
        if sp_ in SWITCHED and L['s1'] == -1: need(has_switch, 'switch line %d without a switch texture' % sp_)
        if sp_ in MANUAL_DOORS:
            need(L['back'] is not None and m.secs[L['back']]['ceil'] == m.secs[L['back']]['floor'], 'manual door %d: back sector not a closed door' % sp_)
    for L in m.lines:
        if L['special'] in SWITCHED and L['s1'] == -1: continue
        for sd in (L['s0'], L['s1']):
            if sd != -1: need(not any(m.sides[sd][k] in SWITCH_NAMES for k in ('top', 'mid', 'bot')), 'switch texture on a non-switch line')
    worst = 0
    for cy in range(max(m.bh - 1, 1)):
        for cx in range(max(m.bw - 1, 1)):
            win = set()
            for yy in range(cy, min(cy + 2, m.bh)):
                for xx in range(cx, min(cx + 2, m.bw)): win |= set(m.cells[yy * m.bw + xx])
            worst = max(worst, sum(1 for i in win if m.lines[i]['special'] and m.lines[i]['s1'] != -1))
    need(worst <= 8, '2x2 window has %d special two-sided lines' % worst)
    # mechanisms
    report = []
    exp = EXPECT.get(name, {})
    for tag in sorted({L['tag'] for L in m.lines if L['special'] in (7, 8)}):
        e = exp.get('stairs', {}).get(tag, {})
        busy = [i for i, s in enumerate(m.secs) if 'busy_tag' in e and s['tag'] == e['busy_tag']]
        chain = stairs(m, tag, busy)
        report.append('stairs tag %d (busy %s): %s' % (tag, busy, ' '.join('%d@%d' % c for c in chain)))
        if 'heights' in e: need([h for _, h in chain] == e['heights'], 'stair heights tag %d: %s' % (tag, chain))
    if 'order' in exp:
        o = exp['order']
        others = [m.lines[i]['back'] if m.lines[i]['front'] == o['sector'] else m.lines[i]['front'] for i in m.sector_lines(o['sector'])]
        need(others.index(o['before']) < others.index(o['after']), 'stair line order')
    for tag in sorted({L['tag'] for L in m.lines if L['special'] == 9}):
        for s1, s2, s3 in donut(m, tag):
            report.append('donut tag %d: s1 %d s2 %d s3 %d (dest %d, flat %s)' % (tag, s1, s2, s3, m.secs[s3]['floor'], m.secs[s3]['fpic']))
            e = exp.get('donut', {}).get(tag)
            if e: need(m.secs[s3]['floor'] == e['s3_floor'] and m.secs[s3]['fpic'] == e['s3_flat'], 'donut destination')
    for L in m.lines:
        if L['special'] in (20, 22):
            for s in (i for i, x in enumerate(m.secs) if x['tag'] == L['tag']):
                front, own = m.secs[L['front']]['fpic'], m.secs[s]['fpic']
                report.append('plat %d tag %d: target %d %s <- front flat %s' % (L['special'], L['tag'], s, own, front))
                e = exp.get('plat_flats', {}).get(L['tag'])
                if e: need((front, own) == e, 'plat flats tag %d' % L['tag'])
    targets = sorted({i for L in m.lines if L['special'] in (18, 20, 22) for i, s in enumerate(m.secs) if s['tag'] == L['tag']})
    for s in targets:
        q = next_highest(m, s)
        report.append('next-highest neighbours of sector %d: %d%s' % (s, len(q), ' (truncated after 20: %d of %s)' % (min(q[:20]), q[20:]) if len(q) > 20 else ''))
        e = exp.get('neighbours')
        if e:
            if m.secs[s]['tag'] == e['comb_tag']:
                need(len(q) == e['comb_count'] and q[20] < min(q[:20]), 'comb neighbours %s' % q)
            else:
                need(len(q) <= e['max_other'], 'sector %d has %d qualifying neighbours' % (s, len(q)))
    print('OK %s (%s): %d bytes; sectors %d, lines %d (two-sided %d), things %d (spawned %d), specials %s, max 2x2 window %d' %
          (path, name, len(m.d), len(m.secs), len(m.lines), sum(L['s1'] != -1 for L in m.lines), len(th), len(sp),
           ' '.join('%dx%d' % kv for kv in sorted(linespec.items())), worst))
    for r in report: print('  ' + r)
    if dump: print('  spawned by doomednum: ' + ', '.join('%s x%d' % kv for kv in sorted(cnt.items(), key=lambda kv: int(kv[0]))))


def box_meets(p, q, cx, cy, r):
    """Closed segment against the closed square of half-width r (Liang-Barsky)."""
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


def main():
    args = sys.argv[1:]
    dump = '--dump-counts' in args
    man = json.loads(MANIFEST.read_text())
    by_file = {w['file']: w for w in man['wads']}
    if '--check' in args:
        for w in man['wads']: check(ROOT / w['file'], w['name'], w['slot'], w['things'], dump)
        return
    if not args or args[0].startswith('--'): sys.exit(__doc__)
    path = args[0]
    rel = str(Path(path).resolve().relative_to(ROOT)) if Path(path).resolve().is_relative_to(ROOT) else path
    w = by_file.get(rel)
    name = args[args.index('--name') + 1] if '--name' in args else (w['name'] if w else Path(path).stem.upper())
    check(path, name, w['slot'] if w else 'E1M1', w['things'] if w else None, dump)


if __name__ == '__main__':
    main()
