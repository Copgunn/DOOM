#!/usr/bin/env python3
"""The independent reader's differential harness for P_InterceptVector (wire 27), committed as a tool.

Written on 2026-11-24 by the independent reader of pull request #82 as a scratch harness (docs/validation/WIRE27.md
section 11) and committed on 2026-11-26 by the certificate audit (docs/validation/AUDIT1.md, item 6); gen, model and
cmp below are the reader's code, changed only to take a file handle, to return their result and to name the
six callers of ruling S1 (aim_line and aim_thing where the reader's run, made before that ruling, had one aim). Three parties:

  C      the genuine text of m_fixed.c:43-87 and p_maputl.c:231-284, extracted here from linuxdoom-1.10 into --work and
         compiled with -fwrapv under wire27_read_drv.c (FixedDiv recorded, abs(INT_MIN) flagged, I_Error caught);
  Rust   crates/doom-sim/src/intercept_vector.rs included unmodified by path, and sight.rs's checked_intercept (its
         lines, extracted here), in a scratch crate under --work built from wire27_read_main.rs;
  model  big-integer arithmetic in this file (model) plus the admitted wrap classes per caller, read from
         tests/reference/wire27_results.json.

For --rows operand sets from --seed it requires: the model's operands equal the C's; wires 23 to 26
(checked_intercept) answer None exactly where an intermediate leaves int and otherwise the C's value (rows with an
INT_MIN operand are only counted: that is the known silent divergence of wires 23 to 26); wire 27 admits, per caller
and wrap class, exactly what the results file says, with the C's value, and refuses the rest; an INT_MIN operand is
refused as AbsMin; sight never wraps. Output: one count per kind of row and the number of mismatches, written to
tests/reference/wire27_read_stats.txt (--check compares instead of writing, and fails on any mismatch).

--selftest plants two faults and requires each to be reported: PLANT=1 in the C (every quotient off by one) and, in a
scratch copy of intercept_vector.rs, `(num ^ den) >= 0` replaced by `(num | den) >= 0` (a wire 27 mutation survivor
of record section 9). No IWAD is read.

    wire27_read.py [--check] [--selftest] [--work DIR] [--rows N] [--seed S] [--cc CC]
"""
import argparse
import json
import random
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
STATS = ROOT / 'tests/reference/wire27_read_stats.txt'
RESULTS = ROOT / 'tests/reference/wire27_results.json'
INTERCEPT = ROOT / 'crates/doom-sim/src/intercept_vector.rs'
# (file, first line, last line, the text those two lines must hold): a moved body stops the tool.
BODIES = {
    'fixed_body.inc': ('linuxdoom-1.10/m_fixed.c', 43, 87, 'fixed_t', '}'),
    'iv_body.inc': ('linuxdoom-1.10/p_maputl.c', 231, 284, 'P_InterceptVector', '#endif'),
}
FLAGS = ['-std=gnu89', '-O1', '-fwrapv', '-fno-strict-aliasing', '-w']
PLANT_RUST = ('let max = (num ^ den) >= 0;', 'let max = (num | den) >= 0;')
CARGO = """[package]
name = "wire27-read"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
doom-types = { path = "%s" }

[profile.release]
debug-assertions = true
overflow-checks = true
"""

MIN, MAX = -2**31, 2**31 - 1


def w(v):
    return (v + 2**31) % 2**32 - 2**31


def fits(v):
    return MIN <= v <= MAX


def mul(a, b):
    return (a * b) >> 16


def gen(n, seed, out):
    r = random.Random(seed)

    def any32():
        bits = r.randint(1, 32)
        v = r.getrandbits(bits)
        return w(v if r.random() < .5 else -v)

    special = [0, 1, -1, 2, -2, 255, 256, -256, 16384, -16384, MIN, MAX, MIN + 1, MAX - 1, 1 << 30, -(1 << 30)]

    def target():
        k = r.random()
        if k < .35:
            return r.choice(special)
        if k < .7:
            return r.randint(-70000, 70000)
        return any32()

    for i in range(n):
        s = r.random()
        if s < .25:
            row = [any32() for _ in range(8)]
        elif s < .55:
            u = 65536
            v1 = [r.randint(-32767, 32767) * u, r.randint(-32767, 32767) * u]
            L = r.choice([200, 4000, 16000, 33000])
            d1 = [w(r.randint(-L, L) * u), w(r.randint(-L, L) * u)]
            T = r.choice([64, 102, 1024, 2048]) * u
            d2 = [r.randint(-T, T), r.randint(-T, T)]
            near = r.choice([300, 3000, 20000]) * u
            v2 = [w(v1[0] + r.randint(-near, near)), w(v1[1] + r.randint(-near, near))]
            row = v2 + d2 + v1 + d1
        else:
            # Targeted: v1.dx = 2^24 makes FixedMul(v1.dx>>8, b) == b, so den and num can be steered.
            v1dy, v2dx = any32(), any32()
            p1 = w(mul(v1dy >> 8, v2dx))
            v2dy = w(p1 - target())
            if r.random() < .5:
                x, v1y = 0, 0
                v2y = w(target() & ~255) if r.random() < .7 else MIN
                v1x = v2x = any32()
            else:
                v2x, v1x, v2y, v1y = any32(), any32(), any32(), any32()
                m = w(mul(w(v1x - v2x) >> 8, v1dy))
                # y>>8 * 256 == target - m (mod 2^32) when the low 8 bits agree
                want = w(target() - m) & ~255
                v2y = w(v1y + want)
            row = [v2x, v2y, v2dx, v2dy, v1x, v1y, 1 << 24, v1dy]
        print(*row, file=out)


def model(row):
    """(checked_ok, class or None, exact den, exact num) with big integers."""
    v2x, v2y, v2dx, v2dy, v1x, v1y, v1dx, v1dy = row
    ok = True
    m1, m2 = mul(v1dy >> 8, v2dx), mul(v1dx >> 8, v2dy)
    ok &= fits(m1) and fits(m2)
    den_exact = m1 - m2
    den_int = w(w(m1) - w(m2))
    if ok and not fits(den_exact):
        ok = False
    if ok and den_exact == 0:
        return True, None, den_int, None, den_exact, None
    x, y = v1x - v2x, v2y - v1y
    num_exact = mul(x >> 8, v1dy) + mul(y >> 8, v1dx)
    xi, yi = w(x), w(y)
    n1, n2 = mul(xi >> 8, v1dy), mul(yi >> 8, v1dx)
    num_int = w(w(n1) + w(n2))
    if ok:
        ok = fits(x) and fits(y) and fits(n1) and fits(n2) and fits(n1 + n2)
    return ok, None, den_int, num_int, den_exact, num_exact


def cmp(inp, cf, rf, admitted, quiet=False):
    callers = ["aim_line", "aim_thing", "shoot_line", "shoot_thing", "slide", "use"]
    stats, bad = Counter(), 0

    def fail(msg, *a):
        nonlocal bad
        bad += 1
        if bad <= 20 and not quiet:
            print("MISMATCH", msg, *a)

    for li, cl, rl in zip(open(inp), open(cf), open(rf)):
        row = [int(t) for t in li.split()]
        c, rs = cl.split(), rl.split()
        ok, _, den_int, num_int, den_exact, num_exact = model(row)
        checked_rs, per, sight = rs[0], rs[1:7], rs[7]
        if c[0] == "I":
            # FixedDiv2's I_Error: only acceptable behind an INT_MIN operand, where Rust must refuse (AbsMin).
            cnum, cden = int(c[1]), int(c[2])
            stats["C I_Error (operand INT_MIN: %s)" % (cnum == MIN or cden == MIN)] += 1
            if not (cnum == MIN or cden == MIN) or (cnum, cden) != (num_int, den_int):
                fail("C I_Error without INT_MIN", row, c)
            if any("AbsMin" not in g for g in per) or sight not in ("N", "EAbsMin") or (sight == "N") != (not ok):
                fail("I_Error row not refused as AbsMin", row, rs)
            stats["rows"] += 1
            continue
        if c[0] == "Z":
            cval, absmin = int(c[1]), False
            if den_int != 0:
                fail("model den != 0 but C returned early", row)
            klass = None if ok else ("den_zero" if den_exact != 0 else "den_zero_exact")
        else:
            cnum, cden, cval, cflag = (int(t) for t in c[1:5])
            if (cnum, cden) != (num_int, den_int):
                fail("model operands differ from C", row, c)
            absmin = cnum == MIN or cden == MIN
            if absmin != bool(cflag):
                fail("abs(INT_MIN) flag", row, c)
            sat = (abs(cnum) >> 14) >= abs(cden)
            if ok:
                klass = None
            else:
                de = "den_exact" if den_int == den_exact else "den_wrong"
                ne = "num_exact" if num_int == num_exact else "num_wrong"
                br = ("max" if (cnum ^ cden) >= 0 else "min") if sat else "div2"
                klass = "%s.%s.%s" % (de, ne, br)
        # checked_intercept (wires 23-26): None exactly where an intermediate leaves int.
        if (checked_rs != "N") != ok:
            fail("checked Some/None", row, checked_rs, ok)
        if ok and not absmin and int(checked_rs[1:]) != cval:
            fail("checked value != C", row, checked_rs, c)
        if ok and absmin:
            stats["wire26 value on an INT_MIN operand (%s C)" % ("equals" if int(checked_rs[1:]) == cval else "differs from")] += 1
        for name, got in zip(callers, per):
            if absmin:
                want = "refuse"
            elif klass is None:
                want = "admit"
            else:
                want = "admit" if klass in admitted.get(name, []) else "refuse"
            stats["%s %s %s" % (name, "absmin" if absmin else ("nowrap" if klass is None else "wrap." + klass), want)] += 1
            if want == "admit":
                if got[0] != "O" or int(got[1:]) != cval:
                    fail("admitted value", name, klass, row, got, c)
            else:
                if got[0] != "E":
                    fail("should refuse", name, klass, row, got, c)
                elif absmin and "AbsMin" not in got:
                    fail("refusal kind", name, row, got)
        want_s = "N" if not ok else ("EAbsMin" if absmin else "O%d" % cval)
        if sight != want_s:
            fail("sight", row, sight, want_s)
        stats["rows"] += 1
    return ["%9d  %s" % (stats[k], k) for k in sorted(stats)], bad


def run(cmd, **kw):
    r = subprocess.run(cmd, **kw)
    if r.returncode:
        sys.exit('wire27_read: %s exited %d' % (cmd[0], r.returncode))
    return r


def extract(work):
    for name, (path, first, last, head, tail) in BODIES.items():
        lines = (ROOT / path).read_text(encoding='latin-1').split('\n')[first - 1:last]
        if lines[0].strip() != head or lines[-1].strip() != tail:
            sys.exit('wire27_read: %s:%d-%d is not the expected body' % (path, first, last))
        (work / name).write_text('\n'.join(lines) + '\n', encoding='latin-1')
    sight = (ROOT / 'crates/doom-sim/src/sight.rs').read_text()
    start = sight.index('pub(crate) fn checked_intercept(')
    (work / 'checked_extract.rs').write_text(sight[start:sight.index('\n}\n', start) + 3])


def build(work, cc, plant_c=0, plant_rust=False):
    """The C driver and the Rust crate under `work`; returns the two executables."""
    work.mkdir(parents=True, exist_ok=True)
    extract(work)
    exe = work / 'drv'
    run([cc, *FLAGS, '-DPLANT=%d' % plant_c, '-I', str(work), str(HERE / 'wire27_read_drv.c'), '-o', str(exe)])
    intercept = INTERCEPT
    if plant_rust:
        text = INTERCEPT.read_text()
        if text.count(PLANT_RUST[0]) != 1:
            sys.exit('wire27_read: the planted line is not in intercept_vector.rs exactly once')
        intercept = work / 'intercept_vector.rs'
        intercept.write_text(text.replace(*PLANT_RUST))
    crate = work / 'rs'
    (crate / 'src').mkdir(parents=True, exist_ok=True)
    (crate / 'Cargo.toml').write_text(CARGO % (ROOT / 'crates/doom-types').as_posix())
    main = (HERE / 'wire27_read_main.rs').read_text()
    main = main.replace('@EXTRACT@', (work / 'checked_extract.rs').as_posix()).replace('@INTERCEPT@', intercept.as_posix())
    (crate / 'src/main.rs').write_text(main)
    run(['cargo', 'build', '--release', '--offline', '--quiet', '--manifest-path', str(crate / 'Cargo.toml'),
         '--target-dir', str(work / 'target')])
    return exe, work / 'target/release/wire27-read'


def differential(work, cc, rows, seed, **plant):
    exe, rs = build(work, cc, **plant)
    inp = work / 'rows.txt'
    with open(inp, 'w') as f:
        gen(rows, seed, f)
    outs = []
    for tool, name in ((exe, 'c.out'), (rs, 'rs.out')):
        with open(inp) as i, open(work / name, 'w') as o:
            r = subprocess.run([str(tool)], stdin=i, stdout=o)
        if r.returncode:   # an assertion inside the Rust driver is a finding too
            return ['%s exited %d' % (name, r.returncode)], 1
        outs.append(work / name)
    return cmp(inp, *outs, json.loads(RESULTS.read_text())['admitted_wrap_classes'], quiet=bool(plant))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'wire27' / 'read')
    ap.add_argument('--rows', type=int, default=300000); ap.add_argument('--seed', type=int, default=27)
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    lines, bad = differential(a.work / 'clean', a.cc, a.rows, a.seed)
    text = '\n'.join(['# tools/reference/wire27_read.py --rows %d --seed %d' % (a.rows, a.seed), *lines,
                      'MISMATCHES: %d' % bad]) + '\n'
    print(text, end='')
    if bad:
        sys.exit('wire27_read: %d mismatches' % bad)
    default = (a.rows, a.seed) == (ap.get_default('rows'), ap.get_default('seed'))
    if a.check and default:
        if STATS.read_text() != text:
            sys.exit('wire27_read: the counts differ from %s' % STATS.relative_to(ROOT))
        print('check: equal to', STATS.relative_to(ROOT))
    elif default:
        STATS.write_text(text)
    if a.selftest:
        for label, plant in (('C quotient off by one', {'plant_c': 1}), ('Rust `^` to `|`', {'plant_rust': True})):
            _, planted = differential(a.work / 'plant', a.cc, a.rows, a.seed, **plant)
            print('selftest: planted fault (%s): %d mismatches' % (label, planted))
            if not planted:
                sys.exit('wire27_read: the planted fault (%s) was not reported' % label)


if __name__ == '__main__':
    main()
