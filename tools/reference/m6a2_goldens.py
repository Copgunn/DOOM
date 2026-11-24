#!/usr/bin/env python3
"""The M6a part 2 golden set from the genuine engine only (docs/M6A_REFERENCE_INVESTIGATION.md section 7.6, the
owner's ruling V9 of 2026-11-23 and its addition of 2026-11-24). Never from Rust output.

A separate set, read only by new tests: it writes new files under new paths and leaves every part 1 golden, WAD,
script and manifest byte for byte as it is. It imports the part 1 tools read-only (tools/reference/m6a_goldens.py:
the runner command, the golden projections, the per-item record with its build variants and witnesses, the
address_bounds allowance; m6a_build.py; m6a_melt_steps.py: the step count predicted from rndtable) and changes none.

Two parts:

  full crawl   the items of tests/reference/m6a2_manifest.json (tools/reference/m6a2_wad.py): FULLREG (registered) and
               FULLRET (retail), the part 1 FINALE demo with the narrow font WAD, so the whole of E1TEXT crawls. Per item
               exactly the part 1 record (m6a_goldens.item_record: strict at screenblocks 10 with dumps and at 11; the
               overflow, address and address_bounds builds at 10, every stream and the dump set identical to strict; a
               strict run with --melt-clock unit at 10 whose other rows equal the default run's; the U8 witnesses), and
               the crawl checked frame by frame at both screen sizes: (a) the tics on which the genuine finale frame
               changes are exactly finale tic 1 and the tic each drawn glyph is first counted ((finalecount - 10) / 3
               characters, f_finale.c:298), so the frames keep changing until the tic of the last character and stay
               unchanged from there to stage 1; (b) every finale frame's SHA-256 equals an independent model of
               F_TextWrite and of the stage-1 V_DrawPatch (this file: the flat tiled as f_finale.c:276-289, each glyph
               drawn as v_video.c:219-262 draws a patch, read from the item's WAD files, the last lump of a name winning
               as w_wad.c:376-386), which proves how many lines and characters each genuine frame shows; (c) stage 1 on
               finale tic 1,571 (f_finale.c:241) with its forced wipe, and no finale M_Random draw.
  melt sb11    every item of the part 1 manifest and the two full-crawl items run by the strict build at screenblocks 11
               with --melt-clock unit (one wipe_ScreenWipe call with ticks = 1 per step, the declared schedule of the M6a
               note section 1.16.1), and by the overflow, address and address_bounds builds the same way: every stream
               identical to strict (the address build compared through its last tic when the declared report at
               st_stuff.c:1292 stops it, as in part 1). Every row but the melt rows must equal the item's committed
               golden (part 1: tests/reference/m6a_frames/<ITEM>.sb11.frames, m6a_sound, m6a_session; full crawl: the
               goldens of this tool); the melt rows go to a new melt file, and every melt's step count must equal the
               count m6a_melt_steps predicts from rndtable and the M_Random index before the melt, with the last two
               pictures equal to the end screen.

Goldens (written without --check, compared byte for byte with --check, which writes nothing to the repository):
  tests/reference/m6a2_session/<ITEM>.session        full crawl: the session stream (DOOM-M6A-SESSION-v1), screenblocks 10
  tests/reference/m6a2_sound/<ITEM>.sound            full crawl: the sound stream as part 1 projects it
  tests/reference/m6a2_frames/<ITEM>.sb<N>.frames    full crawl: the frame stream's golden rows at 10 and 11 (part 1's
                                                     projection: no melt rows)
  tests/reference/m6a2_melt/<ITEM>.sb10.melt         full crawl: the melt rows at screenblocks 10
  tests/reference/m6a2_melt/<ITEM>.sb11.melt         every item: the melt rows at screenblocks 11
  tests/reference/m6a2_results.json                  DOOM-M6A2-RESULTS-v1: inputs and tools by digest, per item the
                                                     counts, variants, melts and their kinds, the crawl facts observed,
                                                     the golden digests
The melt files (DOOM-M6A2-MELT-v1) claim, per the owner's ruling V4 (2026-11-23), one SHA-256 per melt step over the
picture's 64,000 palette indices and the palette per melt, under the declared schedule.

--selftest proves the comparisons can fail: a pixel flipped in the runner (--selftest-pixel) on CHAIN's first frame at
screenblocks 11 must make the melt comparison differ first at that melt's Y row; a scratch copy of the font WAD with the
first glyph of the text changed must make the genuine full-crawl frames differ first at the finale tic that glyph is
first drawn; the crawl model with a newline treated as a space, with the line step one pixel off and with one character
too few must each disagree with the genuine frames at the expected tic; and a dropped melt step and a moved M_Random
index in memory copies of the new sb11 melt goldens must each be caught by the step-count check.

    m6a2_goldens.py [--check] [--selftest] [--only ITEM[,ITEM]] [--work DIR] [--jobs N] [--cc CC]

Address runs use ASAN_OPTIONS=detect_leaks=0 (m6a_goldens.ENV): leak detection is disabled, so they are not
LeakSanitizer coverage. --work must lie outside the repository.
"""
import argparse, concurrent.futures as cf, hashlib, json, struct, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m3h9c_build as base        # noqa: E402
import m5c_reference as M5C       # noqa: E402  (read-only: last_tic_of, end_of)
import m6a_build                  # noqa: E402
import m6a_goldens as G           # noqa: E402  (read-only: run, item_record, golden projections, bounds_check)
import m6a_melt_steps as MS       # noqa: E402  (read-only: check_item, predicted_steps)
import m6a2_wad as W2             # noqa: E402

MANIFEST = ROOT / W2.MANIFEST
RESULTS = ROOT / 'tests/reference/m6a2_results.json'
SCHEMA = 'DOOM-M6A2-RESULTS-v1'
P2 = 'tests/reference/m6a2_'
DIRS = {k: ROOT / f'{P2}{k}' for k in ('session', 'sound', 'frames', 'melt')}
TOOLS = ['m6a2_goldens.py', 'm6a2_wad.py', 'm6a_goldens.py', 'm6a_build.py', 'm6a_melt_steps.py', 'm6a_session.c',
         'm6a_wi_access.c']
MELT_HEADER = ('#schema,DOOM-M6A2-MELT-v1\n'
               '#claim,one SHA-256 per melt step over the picture\'s 64000 palette indices and the palette per melt (owner\'s '
               'ruling V4 of 2026-11-23) under the declared schedule: one wipe_ScreenWipe call with ticks = 1 per step '
               '(docs/M6A_REFERENCE_INVESTIGATION.md section 1.16.1; the runner\'s --melt-clock unit)\n')
SW, SH = 320, 200
sha = lambda b: hashlib.sha256(b).hexdigest()


def items2(only=None):
    """The full-crawl items, in the shape m6a_goldens.run reads."""
    man = json.loads(MANIFEST.read_text())
    out = [dict(it, wads=[ROOT / w for w in man['wadsets'][it['wadset']]], lmp_path=ROOT / it['lmp']) for it in man['items']]
    return [i for i in out if not only or i['name'] in only]


def melt_items(only=None):
    """Every item of the sb11 melt list: the part 1 items (m6a_goldens.items) and the full-crawl items."""
    man = json.loads(MANIFEST.read_text())
    p1 = {i['name']: i for i in G.items()}
    p2 = {i['name']: i for i in items2()}
    out = [p1.get(n) or p2[n] for n in man['melt_sb11']['items']]
    if man['melt_sb11']['manifest_sha256'] != sha((ROOT / man['melt_sb11']['manifest']).read_bytes()):
        sys.exit('the part 1 manifest is not the one tests/reference/m6a2_manifest.json names')
    return [i for i in out if not only or i['name'] in only]


def melt_text(unit):
    """A unit-clock run's melt rows under this set's header (the rows m6a_goldens.melt_rows takes)."""
    t = G.melt_rows(unit['frames'], unit['sound'])
    assert t.startswith(G.MELT_HEADER)
    return MELT_HEADER + t[len(G.MELT_HEADER):]


# ---------------------------------------------------------------- the crawl model --------------------------
def lumps_of(wads):
    """{name: bytes} over the WAD files in order, the last of a name winning (W_CheckNumForName scans backwards,
    w_wad.c:376-386)."""
    out = {}
    for w in wads:
        b = Path(w).read_bytes()
        n, off = struct.unpack_from('<ii', b, 4)
        for k in range(n):
            o, s, nm = struct.unpack_from('<ii8s', b, off + 16 * k)
            out[nm.split(b'\0', 1)[0].decode('latin-1').upper()] = b[o:o + s]
    return out


def draw_patch(screen, x, y, p):
    """V_DrawPatch (v_video.c:204-262) into a 320x200 bytearray; refuses a patch outside the screen (the RANGECHECK
    test, v_video.c:222-232, which would print and skip it)."""
    w, h, left, top = struct.unpack_from('<hhhh', p, 0)
    x, y = x - left, y - top
    if x < 0 or x + w > SW or y < 0 or y + h > SH: raise ValueError(f'patch at {x},{y} exceeds the screen')
    for col in range(w):
        o = struct.unpack_from('<i', p, 8 + 4 * col)[0]
        while p[o] != 0xff:
            top_delta, length = p[o], p[o + 1]
            for k in range(length):
                screen[(y + top_delta + k) * SW + x + col] = p[o + 3 + k]
            o += length + 4


class Crawl:
    """An independent model of F_Drawer for Episode 1 (f_finale.c:261-325, 700-721) over the item's WAD files, with
    optional planted faults: 'newline-as-space', 'line-step' (12 instead of 11), 'one-short' (one character fewer)."""

    def __init__(self, wads, mode, fault=None):
        L = lumps_of(wads)
        self.flat, self.art = L['FLOOR4_8'], L['CREDIT' if mode == 'retail' else 'HELP2']
        self.font = [L[f'STCFN{c:03d}'] for c in range(W2.HU_FONTSTART, W2.HU_FONTEND + 1)]
        self.text, self.fault = W2.e1text(), fault

    def text_frame(self, finalecount):
        s = bytearray(SW * SH)
        for y in range(SH):                                          # f_finale.c:276-289
            row = self.flat[(y & 63) << 6:((y & 63) << 6) + 64]
            s[y * SW:(y + 1) * SW] = row * (SW // 64)
        cx, cy = W2.X0, W2.Y0
        count = max(0, (finalecount - 10) // W2.TEXTSPEED) - (self.fault == 'one-short')
        for ch in self.text[:max(0, count)]:
            if ch == '\n' and self.fault != 'newline-as-space':
                cx, cy = W2.X0, cy + (12 if self.fault == 'line-step' else W2.LINE_STEP); continue
            c = ord(' ' if ch == '\n' else ch.upper()) - W2.HU_FONTSTART
            if c < 0 or c > W2.HU_FONTSIZE: cx += W2.GAP; continue
            w = struct.unpack_from('<h', self.font[c], 0)[0]
            if cx + w > SW: break
            draw_patch(s, cx, cy, self.font[c]); cx += w
        return bytes(s)

    def art_frame(self, previous):
        s = bytearray(previous)
        draw_patch(s, 0, 0, self.art)
        return bytes(s)


def finale_frames(frames_text, session_text):
    """[(finale tic, stage, finalecount, frame digest)] from a run's frame G rows and session F rows."""
    Gf = {int(g['tic']): g for g in G.table(frames_text, 'G') if g['gamestate'] == '2'}
    F = {int(f['tic']): f for f in G.table(session_text, 'F')}
    if not Gf: return []
    f0 = min(Gf)
    return [(t - f0 + 1, int(F[t]['finalestage']), int(F[t]['finalecount']), Gf[t]['sha256']) for t in sorted(Gf)]


def crawl_check(item, frames_text, session_text, fault=None):
    """(facts, problems): the crawl of one run against the model and the glyph schedule (see the module text)."""
    model = Crawl(item['wads'], item['mode'], fault)
    font = [(f'STCFN{c + W2.HU_FONTSTART:03d}', b) for c, b in enumerate(model.font)]
    fx = W2.facts(lumps=font)
    rows = finale_frames(frames_text, session_text)
    problems, prev, prev_model, changes, equal = [], None, None, [], 0
    first_bad = None
    for n, stage, count, digest in rows:
        if stage == 0:
            if count != n: problems.append(f'finale tic {n}: finalecount {count}')
            pic = model.text_frame(count)
            prev_model = pic
        else:
            pic = model.art_frame(prev_model)
        if sha(pic) == digest: equal += 1
        elif first_bad is None: first_bad = n
        if digest != prev: changes.append(n)
        prev = digest
    if first_bad is not None: problems.append(f'finale tic {first_bad}: the frame differs from the model')
    stage1 = next((n for n, s, _c, _d in rows if s == 1), None)
    text_changes = [n for n in changes if stage1 is None or n < stage1]
    walk = W2.crawl(W2.e1text(), W2.widths(font))
    want = [1] + [W2.first_tic(i) for i, _c, k, _x, _y in walk if k == 'glyph']
    if text_changes != want: problems.append(f'the text frames change on {len(text_changes)} tics, the glyph schedule has {len(want)}')
    if stage1 != fx['stage1_finale_tic']: problems.append(f'stage 1 on finale tic {stage1}, expected {fx["stage1_finale_tic"]}')
    last = text_changes[-1] if text_changes else None
    lines = sorted({y for _i, _c, k, _x, y in walk if k == 'glyph'})
    facts = dict(finale_tics=len(rows), finale_frames_equal_to_model=equal, text_frame_changes=len(text_changes),
                 last_glyph_finale_tic=last, frames_equal_from_last_glyph_to_stage1=(stage1 - last) if last and stage1 else None,
                 stage1_finale_tic=stage1, lines_with_glyphs=len(lines), lines=fx['lines'],
                 characters_counted_by_stage1=min(len(model.text), max(0, (stage1 - 1 - 10) // W2.TEXTSPEED)) if stage1 else None,
                 glyphs_drawn=fx['glyphs_drawn'], gaps_4px=fx['gaps_4px'], newlines=fx['newlines'])
    if fault is None and (equal != len(rows) or last != fx['last_glyph_finale_tic']):
        problems.append(f'{equal} of {len(rows)} finale frames equal to the model; last glyph on finale tic {last}')
    return facts, problems


# ---------------------------------------------------------------- runs ------------------------------------
def p1_golden(name, kind):
    ext = {'sound': 'sound', 'session': 'session'}.get(kind)
    return (ROOT / (f'tests/reference/m6a_frames/{name}.sb11.frames' if kind == 'frames'
                    else f'tests/reference/m6a_{kind}/{name}.{ext}')).read_text()


def crawl_both(item, s10_frames, s11_frames, session):
    out, fails = {}, []
    for sb, fr in ((10, s10_frames), (11, s11_frames)):
        facts, problems = crawl_check(item, fr, session)
        out[f'sb{sb}'] = facts
        fails += [f'{item["name"]} sb{sb}: {p}' for p in problems]
    return out, fails


def melt11_record(item, bins, runs, reference):
    """The screenblocks-11 unit-clock runs of one item: (record, melt text, failures). `reference`: {frames, sound,
    session} goldens the non-melt rows must equal."""
    N, fails = item['name'], []
    r = {v: G.run(bins[v], item, runs / f'{N}_m11_{v}', 11, extra=['--melt-clock', 'unit']) for v in G.VARIANTS}
    s = r['strict']
    ok = dict(frames=G.without(G.golden_frames(s['frames']), ()) == G.without(reference['frames'], ()),
              sound=G.without(G.golden_sound(s['sound']), ()) == G.without(reference['sound'], ()),
              session=G.without(s['session'], ()) == G.without(reference['session'], ()))
    if not all(ok.values()): fails.append(f'{N}: sb11 unit rows other than the melt rows differ from the golden: {ok}')
    ubs = lambda x: sorted({l.strip().rsplit('/', 1)[-1] for l in x['stderr'].splitlines() if 'runtime error' in l})
    variants = {}
    for v in G.VARIANTS[1:]:
        ref, got, note = s, r[v], {}
        if v == 'address' and got['end'].startswith('sanitizer'):
            last = M5C.last_tic_of(got)
            if ubs(got) != [G.DECLARED_UB]: fails.append(f'{N}: sb11 address reports {ubs(got)}')
            ref, got = G.truncated(s, last), G.truncated(got, last)
            note = dict(stopped_by=G.DECLARED_UB, compared_through_tic=last)
        if v == 'address_bounds':
            note = dict(bounds_reports=ubs(got))
            remainder, _n, problems = G.bounds_check(got['stderr'], s['stderr'])
            if problems: fails += [f'{N}: sb11 address_bounds: {p}' for p in problems]
            else: got['end'] = M5C.end_of(dict(got, stderr=remainder))   # the run's end without the declared report
        keys = ('frames', 'sound', 'session', 'view') + (('end',) if not note.get('stopped_by') else ())
        diff = [k for k in keys if got[k] != ref[k]]
        if diff: fails.append(f'{N}: sb11 unit {v} differs from strict ({diff})')
        variants[v] = dict(identical_to_strict=not diff, **note)
    for x, who in [(s, 'strict')] + [(r[v], v) for v in G.VARIANTS[1:]]:
        if x['end'].startswith(('sanitizer', 'abnormal')) and not (who == 'address' and variants['address'].get('stopped_by')):
            fails.append(f'{N}: sb11 unit {who} ended {x["end"]}')
    text = melt_text(s)
    melts, matches, problems = MS.check_item(N, text, reference['session'])
    fails += problems
    if matches != melts: fails.append(f'{N}: sb11: {matches} of {melts} melts match the predicted step counts')
    Y = G.table(text, 'Y') if 'Y,' in text else []
    V = G.table(text, 'V') if 'V,' in text else []
    Gs = {int(g['tic']): g for g in G.table(reference['session'], 'G')} if 'G,' in reference['session'] else {}
    Ls = {int(l['tic']) for l in G.table(reference['session'], 'L')} if 'L,' in reference['session'] else set()
    kinds = {}
    for y in Y:
        k = kinds.setdefault(melt_kind(y, Gs, Ls), dict(melts=0, steps=0))
        k['melts'] += 1; k['steps'] += sum(1 for v in V if v['tic'] == y['tic'])
    rec = dict(end=s['end'], melts=len(Y), steps=[sum(1 for v in V if v['tic'] == y['tic']) for y in Y],
               predicted_steps_equal=matches, other_rows_equal_to_golden=ok, kinds=kinds, variants=variants,
               golden=sha(text.encode()))
    return rec, text, fails


def melt_kind(y, Gs, Ls):
    """m6a_goldens.item_record's classification of a melt (the game states before and at its tic)."""
    t = int(y['tic'])
    if t - 1 not in Gs: return 'session start'
    a, b = Gs[t - 1]['gamestate'], y['gamestate']
    return {('0', '1'): 'level to intermission', ('1', '0'): 'intermission to level', ('0', '2'): 'level to finale',
            ('2', '2'): 'finale text to art screen'}.get((a, b), 'reborn reload' if (a, b) == ('0', '0') and t in Ls else f'{a} to {b}')


def full_item(item, bins, runs):
    """Every run of one full-crawl item: (record, texts, failures)."""
    N = item['name']
    rec, texts, fails = G.item_record(item, bins, runs, False)
    moved = {}
    for p, t in texts.items():
        kind, stem = p.split('/')[-2][len('m6a_'):], p.split('/')[-1]
        if kind == 'melt': t, stem = MELT_HEADER + t[len(G.MELT_HEADER):], f'{N}.sb10.melt'
        moved[f'{P2}{kind}/{stem}'] = t
    rec['goldens'] = {p: sha(t.encode()) for p, t in moved.items()}
    session = moved[f'{P2}session/{N}.session']
    rec['crawl'], f2 = crawl_both(item, moved[f'{P2}frames/{N}.sb10.frames'], moved[f'{P2}frames/{N}.sb11.frames'], session)
    fails += f2
    melt10 = moved[f'{P2}melt/{N}.sb10.melt']
    melts, matches, problems = MS.check_item(N, melt10, session)
    fails += problems
    rec['melt']['predicted_steps_equal'] = matches
    if matches != melts: fails.append(f'{N}: sb10: {matches} of {melts} melts match the predicted step counts')
    fin = [g for g in G.table(session, 'G') if g['gamestate'] == '2']
    if fin and fin[-1]['fin_draws'] != fin[0]['fin_draws']: fails.append(f'{N}: a finale M_Random draw')
    rec['finale_session_tics'] = [int(fin[0]['tic']), int(fin[-1]['tic'])] if fin else None
    return rec, moved, fails


def first_diff(a, b):
    return G.first_diff(a, b)


def selftest(bins, work):
    """Each planted fault must be caught at its place (see the module text)."""
    out = []
    chain = G.items('CHAIN')[0]
    gold = (ROOT / f'{P2}melt/CHAIN.sb11.melt').read_text()
    r = G.run(bins['strict'], chain, work / 'self_pixel11', 11, extra=['--melt-clock', 'unit', '--selftest-pixel', '0', '160', '100'])
    d = first_diff(gold, melt_text(r))
    out.append(('a flipped pixel on CHAIN tic 0 at screenblocks 11', f'first melt-file difference {d and d[1][:24]}',
                bool(d and d[1].startswith('Y,0,'))))
    # the font WAD with the first glyph of the text ('O', index 0, drawn first on finale tic 13) changed, in scratch
    item = items2(['FULLRET'])[0]
    g = W2.glyphs()
    rows = g['O'][:]
    rows[3] = rows[3][:2] + ('1' if rows[3][2] == '0' else '0') + rows[3][3:]
    lumps = [(f'STCFN{ord(c):03d}', W2.glyph_patch(rows if c == 'O' else r_)) for c, r_ in g.items()]
    font = work / 'selftest_font' / 'm6a2_font.wad'
    font.parent.mkdir(parents=True, exist_ok=True); font.write_bytes(W2.W.wad(b'PWAD', lumps))
    planted = dict(item, wads=[font if w.name == 'm6a2_font.wad' else w for w in item['wads']])
    r = G.run(bins['strict'], planted, work / 'self_font', 10)
    gold_frames = (ROOT / f'{P2}frames/FULLRET.sb10.frames').read_text()
    d = first_diff(gold_frames, G.golden_frames(r['frames']))
    f0 = json.loads(RESULTS.read_text())['full_crawl']['FULLRET']['finale_session_tics'][0]
    out.append(('the first glyph of the text changed in a scratch font WAD', f'first frame difference {d and d[1][:24]} '
                f'(finale tic 13 is session tic {f0 + 12})', bool(d and d[1].startswith(f'G,{f0 + 12},'))))
    # the crawl model's planted faults against the genuine frames
    fr = gold_frames
    se = (ROOT / f'{P2}session/FULLRET.session').read_text()
    text = W2.e1text()
    first_after_newline = text.index('\n') + 1
    for fault, want in (('newline-as-space', W2.first_tic(first_after_newline)), ('line-step', W2.first_tic(first_after_newline)),
                        ('one-short', W2.first_tic(0))):
        _f, problems = crawl_check(item, fr, se, fault)
        bad = next((p for p in problems if 'differs from the model' in p), None)
        out.append((f'the crawl model with {fault}', f'{bad}', bad == f'finale tic {want}: the frame differs from the model'))
    # the step-count check on the new sb11 melt goldens
    sess = (ROOT / 'tests/reference/m6a_session/CHAIN.session').read_text()
    lines = gold.splitlines(keepends=True)
    y2 = [l.split(',')[1] for l in lines if l.startswith('Y,')][1]
    k = next(i for i, l in enumerate(lines) if l.startswith(f'V,{y2},5,'))
    n, m, p = MS.check_item('CHAIN', ''.join(lines[:k] + lines[k + 1:]), sess)
    out.append(('a dropped step in a copy of CHAIN.sb11.melt', f'{m} of {n}; {p[:1]}', m == n - 1 and bool(p)))
    sl = sess.splitlines(keepends=True)
    cols = next(l for l in sl if l.startswith('#columns,G,')).rstrip('\n').split(',')[2:]
    j = cols.index('rndindex') + 1
    k = next(i for i, l in enumerate(sl) if l.startswith('G,0,'))
    f = sl[k].rstrip('\n').split(','); gidx = int(f[j])
    want = MS.predicted_steps((gidx - MS.DRAWS) & 0xff)[0]
    dd = next(x for x in range(1, 256) if MS.predicted_steps((gidx - MS.DRAWS + x) & 0xff)[0] != want)
    f[j] = str((gidx + dd) & 0xff)
    n, m, p = MS.check_item('CHAIN', gold, ''.join(sl[:k] + [','.join(f) + '\n'] + sl[k + 1:]))
    out.append(('a moved M_Random index under CHAIN.sb11.melt', f'{m} of {n}; {p[:1]}', m == n - 1 and bool(p)))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m6a2-goldens')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--only')
    ap.add_argument('--jobs', type=int, default=1)
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    only = {x.upper() for x in a.only.split(',')} if a.only else None
    bins = G.build_all(work, a.cc)
    run_all = a.check or a.only or not a.selftest
    full = items2(only) if run_all else []
    melts = melt_items(only) if run_all else []
    failures, results, texts, melt_results = [], {}, {}, {}
    runs = work / 'runs'
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        futs = {i['name']: pool.submit(full_item, i, bins, runs) for i in full}
        for name, fut in futs.items():
            rec, tx, fails = fut.result()
            failures += fails; results[name] = rec; texts.update(tx)
            c = rec['crawl']['sb10']
            print(f"{name}: {rec['end']}; {rec['session_tics']} tics, {rec['wipes']} wipes, {rec['other_frames']} other frames; "
                  f"crawl: {c['finale_frames_equal_to_model']} of {c['finale_tics']} finale frames equal to the model, "
                  f"{c['text_frame_changes']} text-frame changes, last glyph on finale tic {c['last_glyph_finale_tic']}, "
                  f"stage 1 on {c['stage1_finale_tic']}; {'ok' if not fails else 'see failures'}", flush=True)

        def reference(i):
            if i['name'] in results:
                n = i['name']
                return dict(frames=texts[f'{P2}frames/{n}.sb11.frames'], sound=texts[f'{P2}sound/{n}.sound'],
                            session=texts[f'{P2}session/{n}.session'])
            return {k: p1_golden(i['name'], k) for k in ('frames', 'sound', 'session')}
        mf = {i['name']: pool.submit(melt11_record, i, bins, runs, reference(i)) for i in melts
              if i['name'] in results or i['name'] not in {x['name'] for x in items2()}}
        for name, fut in mf.items():
            rec, text, fails = fut.result()
            failures += fails; melt_results[name] = rec; texts[f'{P2}melt/{name}.sb11.melt'] = text
            print(f"{name} sb11 melts: {rec['melts']} melts, {sum(rec['steps'])} steps, {rec['predicted_steps_equal']} as predicted; "
                  f"kinds {sorted(rec['kinds'])}; {'ok' if not fails else 'see failures'}", flush=True)
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    if run_all and not a.only:
        man = json.loads(MANIFEST.read_text())
        kinds = {}
        for rec in melt_results.values():
            for k, v in rec['kinds'].items():
                t = kinds.setdefault(k, dict(melts=0, steps=0)); t['melts'] += v['melts']; t['steps'] += v['steps']
        doc = dict(schema=SCHEMA, runner_contract_sha256=sha(m6a_build.CONTRACT.read_bytes()),
                   tools_sha256={f'tools/reference/{n}': sha(base.text(ROOT / 'tools/reference' / n).encode('latin-1')) for n in TOOLS},
                   manifest_sha256=sha(MANIFEST.read_bytes()),
                   inputs_sha256={p: sha((ROOT / p).read_bytes()) for p in sorted({w for ws in man['wadsets'].values() for w in ws}
                                                                                   | {i['lmp'] for i in man['items']})},
                   switches=dict(M5C.RECOMMENDED, **{'--render': 'every', '--melt-clock': 'counter (melt files: unit)'}),
                   views=[10, 11], variants=G.VARIANTS, asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
                   claim='every golden is the genuine engine\'s stream (strict build), identical in the overflow, address and '
                         'address_bounds builds; the melt files claim one SHA-256 per melt step and the palette per melt under '
                         'the declared schedule (ruling V4)',
                   crawl=man['crawl'],
                   full_crawl=results,
                   melt_sb11=dict(items=len(melt_results), melts=sum(r['melts'] for r in melt_results.values()),
                                  steps=sum(sum(r['steps']) for r in melt_results.values()),
                                  predicted_steps_equal=sum(r['predicted_steps_equal'] for r in melt_results.values()),
                                  kinds={k: kinds[k] for k in sorted(kinds)}, per_item=melt_results))
        texts[str(RESULTS.relative_to(ROOT))] = json.dumps(doc, indent=1) + '\n'
        if a.check:
            bad = [p for p, t in texts.items() if not (ROOT / p).exists() or (ROOT / p).read_text() != t]
            extra = [str(p.relative_to(ROOT)) for d in DIRS.values() if d.is_dir() for p in sorted(d.iterdir())
                     if str(p.relative_to(ROOT)) not in texts]
            if bad or extra:
                for p in bad:
                    o = work / 'regenerated' / p; o.parent.mkdir(parents=True, exist_ok=True); o.write_text(texts[p])
                sys.exit(f'differs: {bad}; not generated: {extra} (regenerated text under {work / "regenerated"})')
            print(f'{len(texts)} golden files match ({len(results)} full-crawl items, {len(melt_results)} sb11 melt items)')
        else:
            for d in DIRS.values():
                if d.is_dir():
                    for p in d.iterdir():
                        if str(p.relative_to(ROOT)) not in texts: p.unlink()
            for p, t in texts.items():
                (ROOT / p).parent.mkdir(parents=True, exist_ok=True); (ROOT / p).write_text(t)
            print(f'wrote {len(texts)} golden files ({len(results)} full-crawl items, {len(melt_results)} sb11 melt items)')
    if a.selftest:
        outcomes = selftest(bins, work)
        for what, detail, ok in outcomes: print(f"selftest {what}: {detail}: {'pass' if ok else 'FAIL'}")
        if not all(ok for _, _, ok in outcomes): sys.exit(1)


if __name__ == '__main__':
    main()
