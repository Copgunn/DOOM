#!/usr/bin/env python3
"""The M6a part 2 golden set's inputs: a narrow heads-up font of our own and the manifest of the part 2 items
(docs/M6A_REFERENCE_INVESTIGATION.md section 7.6, the owner's ruling V9 of 2026-11-23 and its addition of 2026-11-23).

Why a new font. With the M5b synthetic font (tests/reference/m5b_gfx.wad) the second line of the Episode 1 finale text
does not fit in 320 pixels, and F_TextWrite ends the whole crawl for that frame at the first character that would pass
the right edge (f_finale.c:321-322: `if (cx+w > SCREENWIDTH) break;`; there is no wrapping, a new line starts only at
a newline character, f_finale.c:306-311). So the part 1 goldens (FINRET, FINREG, NOHELP2) show the first line and part
of the second only. This tool draws a narrower font so that every line of E1TEXT fits, and proves it from the glyph
widths: the crawl of the new items reaches the end of the text.

Everything here is drawn by this tool; nothing comes from any IWAD, and no IWAD is read. Written:

  tests/reference/m6a2_wads/m6a2_font.wad  a PWAD with the 63 heads-up font patches STCFN033..STCFN095 that HU_Init
                                           caches for '!'..'_' (hu_stuff.c:405-410, HU_FONTSTART and HU_FONTEND,
                                           hu_stuff.h:30-34): 7 rows high, our own 5x7 letters and digits (the M6a
                                           picture font, tools/reference/m6a_wad.py FONT, imported read-only, with J, Q
                                           and Z added) and narrower punctuation; each patch is the glyph plus one
                                           transparent column, offsets 0. Loaded after m5b_gfx.wad, so its lumps are the
                                           ones W_CheckNumForName finds (it scans backwards, so the last of a name wins, w_wad.c:376-386); every
                                           other lump of the set is unchanged.
  tests/reference/m6a2_manifest.json       DOOM-M6A2-MANIFEST-v1: the font WAD by digest, the WAD sets, the two
                                           full-crawl items (FULLREG registered, FULLRET retail: the committed part 1
                                           FINALE demo, read by digest and not rewritten), the part 1 items re-run at
                                           screenblocks 11 for the melt goldens (every item of
                                           tests/reference/m6a_manifest.json, by name), and the crawl facts below.

The crawl facts, computed from E1TEXT (parsed from linuxdoom-1.10/d_englsh.h:359-374) and the glyph widths exactly as
F_TextWrite walks the text (f_finale.c:294-325: cx = 10, cy = 10; a newline sets cx = 10 and adds 11 to cy; a character
outside the font, `c < 0 || c > HU_FONTSIZE` after toupper, adds 4 to cx and draws nothing; a glyph is drawn at cx and
adds its width): the number of lines and of empty lines, of characters, glyphs drawn, 4-pixel gaps and newlines; the
largest right edge (every glyph must end at or before x = 320, so no break happens); and, with TEXTSPEED 3 and TEXTWAIT
250 (f_finale.c:56-57), the finale tic on which each character is first counted ((finalecount - 10) / 3 characters,
f_finale.c:298-300; finale tic n has finalecount n, the G_Ticker that ran G_DoCompleted being tic 1, M6a note section
1.8), the tic of the last drawn character and of the last character, and the tic stage 1 begins (finalecount >
strlen*3 + 250, f_finale.c:241).

    m6a2_wad.py            write the font WAD and the manifest
    m6a2_wad.py --check    regenerate in memory and compare with the committed files (writes nothing)
    m6a2_wad.py --facts    print the crawl facts
    m6a2_wad.py --selftest plant faults in memory and require each to be caught: the part 1 (M5b) font, with which the
                           second line breaks at the right edge; the glyph E made 65 pixels wide, so a line passes x = 320;
                           a flipped byte in a copy of the generated font WAD, which the comparison of --check must see
"""
import argparse, hashlib, json, re, struct, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m6a_wad as W              # noqa: E402  the M6a picture font, patch() and wad() (read-only)

WAD_DIR = 'tests/reference/m6a2_wads'
FONT_WAD = f'{WAD_DIR}/m6a2_font.wad'
MANIFEST = 'tests/reference/m6a2_manifest.json'
MANIFEST_SCHEMA = 'DOOM-M6A2-MANIFEST-v1'
HU_FONTSTART, HU_FONTEND = ord('!'), ord('_')    # hu_stuff.h:30-31
HU_FONTSIZE = HU_FONTEND - HU_FONTSTART + 1      # hu_stuff.h:34: 63
SCREENWIDTH, SCREENHEIGHT = 320, 200             # doomdef.h
TEXTSPEED, TEXTWAIT = 3, 250                     # f_finale.c:56-57
LINE_STEP, GAP, X0, Y0 = 11, 4, 10, 10           # f_finale.c:294-295, 308-309, 316
INK = 176                                        # not one of FLOOR4_8's indices (m6a_wad.gfx_lumps), so every glyph shows

# The glyphs the M6a picture font lacks, and narrower punctuation (rows top to bottom, '1' = ink); each row of a glyph has
# the same width. The letters and digits of m6a_wad.FONT are taken as they are.
EXTRA = {
    'J': '00111 00010 00010 00010 00010 10010 01100', 'Q': '01110 10001 10001 10001 10101 10010 01101',
    'Z': '11111 00001 00010 00100 01000 10000 11111',
    '!': '1 1 1 1 1 0 1', '"': '101 101 000 000 000 000 000', '#': '01010 11111 01010 01010 11111 01010 00000',
    '$': '00100 01111 10100 01110 00101 11110 00100', '&': '01100 10010 10100 01000 10101 10010 01101',
    "'": '1 1 0 0 0 0 0', '(': '01 10 10 10 10 10 01', ')': '10 01 01 01 01 01 10',
    '*': '00000 10101 01110 11111 01110 10101 00000', '+': '00000 00100 00100 11111 00100 00100 00000',
    ',': '00 00 00 00 00 01 10', '-': '000 000 000 111 000 000 000', '.': '0 0 0 0 0 0 1',
    '/': '00001 00010 00010 00100 01000 01000 10000', ':': '0 0 1 0 0 1 0', ';': '00 00 01 00 00 01 10',
    '<': '0001 0010 0100 1000 0100 0010 0001', '=': '0000 0000 1111 0000 1111 0000 0000',
    '>': '1000 0100 0010 0001 0010 0100 1000', '?': '01110 10001 00001 00110 00100 00000 00100',
    '@': '01110 10001 10111 10101 10111 10000 01111', '[': '11 10 10 10 10 10 11',
    '\\': '10000 01000 01000 00100 00010 00010 00001', ']': '11 01 01 01 01 01 11',
    '^': '00100 01010 10001 00000 00000 00000 00000', '_': '00000 00000 00000 00000 00000 00000 11111',
}


def glyphs():
    """{character: rows} for the 63 characters '!'..'_'."""
    out = {}
    for code in range(HU_FONTSTART, HU_FONTEND + 1):
        ch = chr(code)
        bits = EXTRA.get(ch) or W.FONT.get(ch)
        if bits is None: sys.exit(f'no glyph for {ch!r}')
        rows = bits.split()
        assert len(rows) == 7 and len({len(r) for r in rows}) == 1 and set(''.join(rows)) <= {'0', '1'}, ch
        assert '1' in ''.join(rows), f'{ch!r} has no ink'
        out[ch] = rows
    return out


def glyph_patch(rows):
    """The patch of one glyph: its columns, then one transparent column (the space after it); offsets 0."""
    w = len(rows[0]) + 1
    return W.patch([[INK if x < w - 1 and rows[y][x] == '1' else None for x in range(w)] for y in range(len(rows))])


def font_lumps():
    return [(f'STCFN{code:03d}', glyph_patch(rows)) for code, rows in ((ord(c), r) for c, r in glyphs().items())]


def widths(lumps):
    """{font index: (width, height, left, top)} from the patch headers (patch_t, r_defs.h)."""
    return {int(n[5:]) - HU_FONTSTART: struct.unpack_from('<hhhh', b, 0) for n, b in lumps}


def e1text():
    """E1TEXT from d_englsh.h (the adjacent string literals of the #define, escapes \\n and \\" only)."""
    src = (ROOT / 'linuxdoom-1.10/d_englsh.h').read_text(encoding='latin-1')
    start = src.index('#define E1TEXT')
    block = src[start:src.index('#define E2TEXT')]
    text = ''.join(re.findall(r'"((?:[^"\\]|\\.)*)"', block)).replace('\\n', '\n').replace('\\"', '"')
    if '\\' in text: sys.exit('E1TEXT: an escape this parser does not know')
    return text


def crawl(text, w):
    """F_TextWrite's walk of the whole text (f_finale.c:301-325) with the font widths `w`: per character (index, char,
    kind 'newline' | 'gap' | 'glyph', x, y of a glyph); raises if a glyph would pass the right edge (the break) or
    the text holds the one index F_TextWrite lets past the array (c == HU_FONTSIZE, '`')."""
    cx, cy, out = X0, Y0, []
    for i, ch in enumerate(text):
        if ch == '\n':
            cx, cy = X0, cy + LINE_STEP; out.append((i, ch, 'newline', None, None)); continue
        c = ord(ch.upper()) - HU_FONTSTART
        if c == HU_FONTSIZE: raise ValueError(f'character {i} ({ch!r}) reads hu_font[{HU_FONTSIZE}], past the array')
        if c < 0 or c > HU_FONTSIZE:
            cx += GAP; out.append((i, ch, 'gap', None, None)); continue
        width, height = w[c][0], w[c][1]
        if cx + width > SCREENWIDTH: raise ValueError(f'character {i} ({ch!r}) at x {cx} width {width}: the crawl breaks')
        if cy + height > SCREENHEIGHT: raise ValueError(f'character {i} ({ch!r}) at y {cy}: below the screen')
        out.append((i, ch, 'glyph', cx, cy)); cx += width
    return out


def first_tic(i):
    """The finale tic on which character `i` (0-based) is first counted: (finalecount - 10) / 3 >= i + 1."""
    return X0 + TEXTSPEED * (i + 1)


def facts(lumps=None):
    lumps = lumps or font_lumps()
    w = widths(lumps)
    text = e1text()
    walk = crawl(text, w)
    kinds = [k for _i, _c, k, _x, _y in walk]
    lines = text.split('\n')
    if lines[-1] == '': lines = lines[:-1]                 # the text ends with a newline
    right = max(x + w[ord(c.upper()) - HU_FONTSTART][0] for _i, c, k, x, _y in walk if k == 'glyph')
    bottom = max(y + w[ord(c.upper()) - HU_FONTSTART][1] for _i, c, k, _x, y in walk if k == 'glyph')
    last_glyph = max(i for i, _c, k, _x, _y in walk if k == 'glyph')
    stage1 = len(text) * TEXTSPEED + TEXTWAIT + 1          # f_finale.c:241: finalecount > strlen*3 + 250
    return dict(text_source='linuxdoom-1.10/d_englsh.h:359-374 (E1TEXT)', characters=len(text),
                text_sha256=hashlib.sha256(text.encode('latin-1')).hexdigest(),
                lines=len(lines), empty_lines=sum(1 for l in lines if not l), newlines=kinds.count('newline'),
                glyphs_drawn=kinds.count('glyph'), gaps_4px=kinds.count('gap'),
                gap_characters=sorted({c for _i, c, k, _x, _y in walk if k == 'gap'}),
                longest_line_characters=max(len(l) for l in lines), largest_right_edge=right, lowest_glyph_bottom=bottom,
                every_glyph_fits=right <= SCREENWIDTH and bottom <= SCREENHEIGHT,
                last_glyph_index=last_glyph, last_glyph_finale_tic=first_tic(last_glyph),
                last_character_finale_tic=first_tic(len(text) - 1), stage1_finale_tic=stage1,
                idle_tics_after_last_glyph=stage1 - 1 - first_tic(last_glyph))


# ---------------------------------------------------------------- the manifest ---------------------------
GFX_M5B, SFX_M5C = W.GFX_M5B, W.SFX_M5C
P1 = W.WAD_DIR
WADSETS = {'fullcrawl': [f'{P1}/episode.wad', GFX_M5B, SFX_M5C, f'{P1}/m6a_gfx.wad', FONT_WAD, f'{P1}/m6a_music.wad']}
FINALE_LMP = f'{W.SCRIPT_DIR}/FINALE.lmp'                   # the part 1 demo of FINREG and FINRET, read by digest
ITEMS = [
    ('FULLREG', 'FINALE', 'registered', 'fullcrawl',
     'the commands of FINREG (E1M8 into the finale) with the narrow font: the whole of E1TEXT crawls, every line begun by '
     'its newline, the last character on finale tic 1327, 243 idle tics, the stage-1 wipe on finale tic 1571 to HELP2'),
    ('FULLRET', 'FINALE', 'retail', 'fullcrawl',
     'the same commands in retail mode: the whole crawl, then the stage-1 wipe to CREDIT'),
]


def manifest(font):
    lmp = (ROOT / FINALE_LMP).read_bytes()
    p1 = json.loads((ROOT / W.MANIFEST).read_text())
    want = next(i for i in p1['items'] if i['name'] == 'FINREG')
    if want['lmp'] != FINALE_LMP or want['lmp_sha256'] != hashlib.sha256(lmp).hexdigest():
        sys.exit('the part 1 FINALE demo is not the one FINREG names')
    return dict(schema=MANIFEST_SCHEMA, generator='tools/reference/m6a2_wad.py',
                files=[dict(file=FONT_WAD, bytes=len(font), sha256=hashlib.sha256(font).hexdigest())],
                wadsets=WADSETS,
                items=[dict(name=n, lump=l, mode=m, wadset=ws, what=what, map=lmp[3], skill=lmp[1], tics=want['tics'],
                            lmp=FINALE_LMP, lmp_sha256=hashlib.sha256(lmp).hexdigest()) for n, l, m, ws, what in ITEMS],
                melt_sb11=dict(manifest=W.MANIFEST, manifest_sha256=hashlib.sha256((ROOT / W.MANIFEST).read_bytes()).hexdigest(),
                               items=[i['name'] for i in p1['items']] + [n for n, *_ in ITEMS]),
                crawl=facts())


def generate():
    font = W.wad(b'PWAD', font_lumps())
    f = facts()
    if not f['every_glyph_fits'] or f['characters'] != 440 or f['gaps_4px'] != 69:
        sys.exit(f'crawl facts: {f}')
    return [(FONT_WAD, font), (MANIFEST, (json.dumps(manifest(font), indent=1) + '\n').encode())]


def selftest():
    """Each planted fault must be caught (see the module text)."""
    out = []
    m5b = (ROOT / GFX_M5B).read_bytes()
    n, off = struct.unpack_from('<ii', m5b, 4)
    lumps = []
    for k in range(n):
        o, size, nm = struct.unpack_from('<ii8s', m5b, off + 16 * k)
        nm = nm.split(b'\0', 1)[0].decode()
        if nm.startswith('STCFN'): lumps.append((nm, m5b[o:o + size]))
    try:
        crawl(e1text(), widths(lumps)); caught = None
    except ValueError as e:
        caught = str(e)
    out.append(('the part 1 (M5b) font', f'{caught}', bool(caught) and 'the crawl breaks' in caught))
    g = glyphs()
    wide = [(f'STCFN{ord(c):03d}', glyph_patch([r + '0' * 59 for r in rows] if c == 'E' else rows)) for c, rows in g.items()]
    try:
        crawl(e1text(), widths(wide)); caught = None
    except ValueError as e:
        caught = str(e)
    out.append(('the glyph E made 65 pixels wide', f'{caught}', bool(caught) and 'the crawl breaks' in caught))
    font = bytearray(generate()[0][1]); font[-20] ^= 1
    committed = (ROOT / FONT_WAD).read_bytes()
    out.append(('a flipped byte in a copy of the font WAD', f'differs from the committed file: {bytes(font) != committed}',
                bytes(font) != committed and generate()[0][1] == committed))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--facts', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    a = ap.parse_args()
    out = generate()
    if a.selftest:
        res = selftest()
        for what, detail, ok in res: print(f"selftest {what}: {detail}: {'pass' if ok else 'FAIL'}")
        if not all(ok for *_x, ok in res): sys.exit(1)
        if not a.check: return
    if a.facts:
        for k, v in facts().items(): print(f'{k}: {v}')
        return
    if a.check:
        bad = [f for f, b in out if not (ROOT / f).exists() or (ROOT / f).read_bytes() != b]
        extra = [str(p.relative_to(ROOT)) for p in sorted((ROOT / WAD_DIR).iterdir())
                 if str(p.relative_to(ROOT)) not in {f for f, _ in out}] if (ROOT / WAD_DIR).is_dir() else []
        if bad or extra: sys.exit('differs or missing: ' + ', '.join(bad) + ('; not generated: ' + ', '.join(extra) if extra else ''))
        f = facts()
        print(f"{len(out)} files match; E1TEXT: {f['characters']} characters, {f['lines']} lines ({f['empty_lines']} empty), "
              f"{f['glyphs_drawn']} glyphs, {f['gaps_4px']} gaps, {f['newlines']} newlines; right edge {f['largest_right_edge']} <= 320; "
              f"last glyph on finale tic {f['last_glyph_finale_tic']}, stage 1 on {f['stage1_finale_tic']}")
        return
    for f, b in out:
        (ROOT / f).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / f).write_bytes(b)
    print(f'wrote {len(out)} files')


if __name__ == '__main__':
    main()
