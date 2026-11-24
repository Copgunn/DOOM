#!/usr/bin/env python3
"""Predicted against observed melt step counts over the committed M6a melt goldens (docs/M6A_REFERENCE_INVESTIGATION.md
section 1.16.1). Reads only committed files; builds and runs nothing.

For every melt of every item (the Y rows of tests/reference/m6a_melt/<ITEM>.melt, the genuine runner under its
--melt-clock unit evidence switch: one wipe_ScreenWipe call per step, each with ticks = 1), this tool recomputes the
number of steps from linuxdoom-1.10 alone and compares it with the V rows the genuine engine wrote:

  - the M_Random index before the melt: the session golden's G row of the melt's tic (tests/reference/m6a_session/
    <ITEM>.session, column rndindex, the index after that tic's display) minus the melt's 320 draws, since the wipe's
    draws are the last of the tic (d_main.c:325-344);
  - the 320 draws of wipe_initMelt (f_wipe.c:158-166: y[0] = -(M_Random()%16); y[i] = y[i-1] + (M_Random()%3) - 1,
    clamped to 0 above and -15 at -16) from rndtable, parsed from linuxdoom-1.10/m_random.c (M_Random, m_random.c:63-67);
  - wipe_doMelt with ticks = 1 per call (f_wipe.c:184-222, width/2 = 160 columns, height 200): a call returns done when
    no column moved; the number of calls is the step count.

It also checks each melt's V rows (steps 1..N in order, tics = 1, done only on the last step; the last two pictures
both equal to the Y row's end screen, section 1.16.1), its W row (320 draws and the same N) and that the session row's
index exists. Every mismatch is printed and makes the exit status 1.

    m6a_melt_steps.py [--check]   compare every melt (the default; writes nothing)
    m6a_melt_steps.py --selftest  plant faults in memory copies of the goldens and require each to be caught
"""
import argparse, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
MELT_DIR = ROOT / 'tests/reference/m6a_melt'
SESSION_DIR = ROOT / 'tests/reference/m6a_session'
WIDTH, HEIGHT, DRAWS = 320, 200, 320          # SCREENWIDTH, SCREENHEIGHT (doomdef.h); wipe_initMelt draws once per i < width


def rndtable():
    src = (ROOT / 'linuxdoom-1.10/m_random.c').read_text()
    body = src[src.index('rndtable[256]') + len('rndtable[256]'):]
    body = body[:body.index('};')]
    tab = [int(x) for x in re.findall(r'\d+', body)]
    if len(tab) != 256: sys.exit(f'rndtable: {len(tab)} entries parsed, expected 256')
    return tab


TAB = rndtable()


def predicted_steps(index_before):
    """wipe_initMelt's column offsets from M_Random starting after `index_before`, then wipe_doMelt with ticks = 1 per
    call until a call moves nothing; returns (calls, index after the draws)."""
    idx, y = index_before, []
    for i in range(WIDTH):                                     # f_wipe.c:158-166
        idx = (idx + 1) & 0xff                                 # m_random.c:65
        r = TAB[idx]
        if i == 0:
            y.append(-(r % 16))
        else:
            v = y[-1] + (r % 3) - 1
            y.append(0 if v > 0 else (-15 if v == -16 else v))
    y = y[:WIDTH // 2]                                         # f_wipe.c:186 width/=2
    calls = 0
    while True:
        calls += 1
        done = True                                            # f_wipe.c:184
        for i in range(len(y)):                                # f_wipe.c:190-220, one tick
            if y[i] < 0:
                y[i] += 1; done = False
            elif y[i] < HEIGHT:
                dy = y[i] + 1 if y[i] < 16 else 8
                if y[i] + dy >= HEIGHT: dy = HEIGHT - y[i]
                y[i] += dy; done = False
        if done: return calls, idx


def table(text, kind):
    cols = None
    out = []
    for line in text.splitlines():
        f = line.split(',')
        if f[0] == '#columns' and f[1] == kind: cols = f[2:]
        elif f[0] == kind:
            if cols is None or len(f) - 1 != len(cols): raise ValueError(f'{kind} row before or unlike its #columns: {line}')
            out.append(dict(zip(cols, f[1:])))
    return out


def check_item(name, melt_text, session_text):
    """(melts, matches, problems) for one item."""
    problems = []
    Y, V, W = table(melt_text, 'Y'), table(melt_text, 'V'), table(melt_text, 'W')
    G = {int(g['tic']): int(g['rndindex']) for g in table(session_text, 'G')}
    steps = {}
    for v in V: steps.setdefault(int(v['tic']), []).append(v)
    draws = {int(w['tic']): (int(w['draws']), int(w['steps'])) for w in W}
    if set(steps) != {int(y['tic']) for y in Y}: problems.append(f'{name}: the V rows and Y rows name different tics')
    matches = 0
    for y in Y:
        t = int(y['tic'])
        if t not in G: problems.append(f'{name}: melt at tic {t} has no session G row'); continue
        before = (G[t] - DRAWS) & 0xff
        want, after = predicted_steps(before)
        rows = steps.get(t, [])
        got = len(rows)
        seq_ok = [int(r['step']) for r in rows] == list(range(1, got + 1)) and all(r['tics'] == '1' for r in rows) \
            and [r['done'] for r in rows] == ['0'] * (got - 1) + ['1']
        w_ok = draws.get(t) == (DRAWS, got)
        if len(rows) < 2 or rows[-1]['sha256'] != rows[-2]['sha256'] or rows[-1]['sha256'] != y['end_sha256']:
            problems.append(f'{name}: tic {t}: the last two pictures are not both the end screen')
        if after != G[t]: problems.append(f'{name}: tic {t}: the index after 320 draws is {after}, the G row has {G[t]}')
        if not seq_ok: problems.append(f'{name}: tic {t}: V rows not steps 1..{got} with tics 1 and done only on the last')
        if not w_ok: problems.append(f'{name}: tic {t}: W row {draws.get(t)} against (320, {got})')
        last_ok = len(rows) >= 2 and rows[-1]['sha256'] == rows[-2]['sha256'] == y['end_sha256']
        if want == got and seq_ok and w_ok and after == G[t] and last_ok:
            matches += 1
        elif want != got:
            problems.append(f'{name}: tic {t}: index before {before}: predicted {want} steps, observed {got}')
    return len(Y), matches, problems


def check_all(texts):
    """texts: {item: (melt text, session text)}; returns (melts, matches, problems, per item)."""
    total = ok = 0; problems = []; per = []
    for name in sorted(texts):
        n, m, p = check_item(name, *texts[name])
        total += n; ok += m; problems += p; per.append((name, n, m))
    return total, ok, problems, per


def committed():
    texts = {}
    for p in sorted(MELT_DIR.glob('*.melt')):
        s = SESSION_DIR / (p.stem + '.session')
        texts[p.stem] = (p.read_text(), s.read_text() if s.exists() else '')
    return texts


def selftest():
    """Planted faults in memory copies of the committed goldens; each must be caught."""
    base = committed()
    out = []
    t0, ok0, p0, _ = check_all(base)
    out.append(('the committed goldens', f'{ok0} of {t0}', ok0 == t0 and not p0))

    def planted(label, item, edit):
        texts = dict(base)
        texts[item] = edit(*base[item])
        t, ok, p, _ = check_all(texts)
        out.append((label, f'{ok} of {t}; first problem: {p[0] if p else None}', ok == t - 1 and bool(p)))
    # one V row dropped from CHAIN's second melt (tic 126)
    def drop_step(m, s):
        lines = m.splitlines(keepends=True)
        k = next(i for i, l in enumerate(lines) if l.startswith('V,126,5,'))
        return ''.join(lines[:k] + lines[k + 1:]), s
    planted('a dropped melt step (CHAIN tic 126 step 5)', 'CHAIN', drop_step)
    # the session index of CHAIN's first melt moved to an index whose prediction differs
    def move_index(m, s):
        lines = s.splitlines(keepends=True)
        cols = next(l for l in lines if l.startswith('#columns,G,')).rstrip('\n').split(',')[2:]
        k = next(i for i, l in enumerate(lines) if l.startswith('G,0,'))
        f = lines[k].rstrip('\n').split(','); j = cols.index('rndindex') + 1
        g = int(f[j])
        want = predicted_steps((g - DRAWS) & 0xff)[0]
        d = next(d for d in range(1, 256) if predicted_steps((g - DRAWS + d) & 0xff)[0] != want)
        f[j] = str((g + d) & 0xff)
        return m, ''.join(lines[:k] + [','.join(f) + '\n'] + lines[k + 1:])
    planted('a moved M_Random index (CHAIN tic 0)', 'CHAIN', move_index)
    # an extra step appended to SECRET's first melt
    def extra_step(m, s):
        lines = m.splitlines(keepends=True)
        y = next(l for l in lines if l.startswith('Y,')).split(',')[1]
        vs = [i for i, l in enumerate(lines) if l.startswith(f'V,{y},')]
        last = lines[vs[-1]].split(',')
        new = ','.join(['V', y, str(int(last[2]) + 1), '1', '1', last[5]])
        prev = ','.join(last[:4] + ['0'] + last[5:])
        return ''.join(lines[:vs[-1]] + [prev, new] + lines[vs[-1] + 1:]), s
    planted('an extra melt step (SECRET first melt)', 'SECRET', extra_step)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--check', action='store_true', help='compare every melt (the default)')
    ap.add_argument('--selftest', action='store_true')
    a = ap.parse_args()
    if a.selftest:
        res = selftest()
        for label, detail, ok in res: print(f'selftest {label}: {detail}: {"pass" if ok else "FAIL"}')
        sys.exit(0 if all(ok for *_x, ok in res) else 1)
    total, ok, problems, per = check_all(committed())
    for name, n, m in per: print(f'{name}: {m} of {n} melts')
    hist = {}
    for r in range(256):
        c = predicted_steps(r)[0]; hist[c] = hist.get(c, 0) + 1
    print('steps over the 256 starting indices:', ', '.join(f'{k}:{v}' for k, v in sorted(hist.items())))
    for p in problems: print('MISMATCH', p)
    print(f'{ok} of {total} melts: predicted step count = observed (V rows), last two pictures = end screen, {len(per)} items')
    sys.exit(0 if ok == total and not problems else 1)


if __name__ == '__main__':
    main()
