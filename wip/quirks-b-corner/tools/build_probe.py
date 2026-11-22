#!/usr/bin/env python3
"""Build the PROBE variant of the strict M4b genuine runner (scratch only, declared probes).

Starts from the strict build tree written by tools/reference/m4b_build.py --variant strict
(BUILD/strict/src: the genuine whole files with the declared ABI adapters, the r_main extract,
the stubs and the unchanged m4a_driver.c) and applies OBSERVATION-ONLY probes:

  P1  p_maputl.c P_PathTraverse: writes, to the file named by $CORNER_PROBE_LOG (nothing when
      unset), one PT line per call (gametic, flags, the four arguments before the nudge, the
      nudged start, xt1/yt1/xt2/yt2, mapxstep/mapystep, ystep/xstep, the initial
      yintercept/xintercept), one C line per loop iteration (mapx, mapy, yintercept,
      xintercept as the loop sees them at the top of the iteration), and after the loop one E
      line (iterations run, whether the end cell was reached, intercept_p - intercepts) and one
      I line per collected intercept (frac, isaline, line index or mobj type and address
      order). It reads only; it never writes a game variable.
  P2  m4a_driver.c: when $CORNER_DIRECT names a file, after the load dump the driver calls the
      genuine P_PathTraverse once per line "x1 y1 x2 y2 flags" (fixed-point ints) with a
      traverser that returns true (declared direct-call adapter: no game code calls it with
      these arguments), then exits 0 before the first tic.

The game state of a demo run must be byte-identical to the unmodified strict runner
(checked by run_scenes.py: every dump file compared)."""
import json, shutil, subprocess, sys
from pathlib import Path

BUILD = Path('/home/copgunn/scratch/quirks/followup-b/corner/build')
src0, out = BUILD / 'strict/src', BUILD / 'probe'

def rep(text, old, new):
    n = text.count(old)
    if n != 1: sys.exit(f'probe patch FAILED: {old[:60]!r} occurs {n} times')
    return text.replace(old, new)

PROBE_DEFS = r'''
/* ---- CORNER PROBE (observation only; scratch build) ---- */
#include <stdio.h>
extern int gametic;
static FILE* probe_fp; static int probe_ready;
static FILE* probe_file (void)
{
    if (!probe_ready) { const char* p = getenv ("CORNER_PROBE_LOG"); probe_ready = 1; if (p) probe_fp = fopen (p, "w"); }
    return probe_fp;
}
/* ---- end CORNER PROBE definitions ---- */
'''

def main():
    if out.exists(): shutil.rmtree(out)
    shutil.copytree(src0, out / 'src')
    (out / 'obj').mkdir()
    s = (out / 'src/p_maputl.c').read_text(encoding='latin-1')
    s = rep(s, '\nintercept_t\tintercepts[MAXINTERCEPTS];\n', PROBE_DEFS + '\nintercept_t\tintercepts[MAXINTERCEPTS];\n')
    s = rep(s, '    int\t\tcount;\n\t\t\n    earlyout = flags & PT_EARLYOUT;',
            '    int\t\tcount;\n    fixed_t probe_a[4]; FILE* probe_f = probe_file ();\n\t\t\n'
            '    probe_a[0] = x1; probe_a[1] = y1; probe_a[2] = x2; probe_a[3] = y2;\n'
            '    earlyout = flags & PT_EARLYOUT;')
    s = rep(s, '    mapx = xt1;\n    mapy = yt1;\n',
            '    mapx = xt1;\n    mapy = yt1;\n'
            '    if (probe_f) fprintf (probe_f, "PT tic %d flags %d args %d %d %d %d start %d %d first %d %d last %d %d '
            'step %d %d ystep %d xstep %d yint %d xint %d\\n", gametic, flags, probe_a[0], probe_a[1], probe_a[2], probe_a[3], '
            'trace.x, trace.y, xt1, yt1, xt2, yt2, mapxstep, mapystep, ystep, xstep, yintercept, xintercept);\n')
    s = rep(s, '    for (count = 0 ; count < 64 ; count++)\n    {\n',
            '    for (count = 0 ; count < 64 ; count++)\n    {\n'
            '\tif (probe_f) fprintf (probe_f, "C %d %d %d %d\\n", mapx, mapy, yintercept, xintercept);\n')
    s = rep(s, '    // go through the sorted list\n    return P_TraverseIntercepts ( trav, FRACUNIT );',
            '    if (probe_f) {\n'
            '\tintercept_t* pi;\n'
            '\tfprintf (probe_f, "E iters %d reached %d intercepts %d\\n", count < 64 ? count + 1 : 64, '
            'mapx == xt2 && mapy == yt2, (int)(intercept_p - intercepts));\n'
            '\tfor (pi = intercepts; pi < intercept_p; pi++)\n'
            '\t    fprintf (probe_f, "I %d %d %d\\n", pi->frac, pi->isaline, pi->isaline ? (int)(pi->d.line - lines) : (int)pi->d.thing->type);\n'
            '\tfflush (probe_f);\n'
            '    }\n'
            '    // go through the sorted list\n    return P_TraverseIntercepts ( trav, FRACUNIT );')
    (out / 'src/p_maputl.c').write_bytes(s.encode('latin-1'))
    d = (out / 'src/m4a_driver.c').read_text(encoding='latin-1')
    d = rep(d, 'int main (int argc, char** argv)\n{',
            '/* CORNER PROBE P2: declared direct-call adapter */\n'
            'static boolean corner_trav (intercept_t* in) { (void)in; return true; }\n'
            'static void corner_direct (const char* path)\n'
            '{\n'
            '    FILE* f = fopen (path, "r"); int a, b, c, e, fl;\n'
            '    if (!f) { fprintf (stderr, "cannot open %s\\n", path); exit (7); }\n'
            '    while (fscanf (f, "%d %d %d %d %d", &a, &b, &c, &e, &fl) == 5) P_PathTraverse (a, b, c, e, fl, corner_trav);\n'
            '    fclose (f);\n'
            '}\n'
            'int main (int argc, char** argv)\n{')
    d = rep(d, '    if (!nodump) dump (out, "load.txt", -1);\n',
            '    if (!nodump) dump (out, "load.txt", -1);\n'
            '    if (getenv ("CORNER_DIRECT")) { corner_direct (getenv ("CORNER_DIRECT")); printf ("direct: done\\n"); return 0; }\n')
    (out / 'src/m4a_driver.c').write_bytes(d.encode('latin-1'))
    man = json.loads((BUILD / 'manifest_strict.json').read_text())
    flags, ff, link = man['flags']['strict'], man['file_flags'], man['link_flags']
    units = man['whole_files'] + ['r_main_extract.c', 'layout_probe.c', 'm3h9c_stubs.c', 'm4a_driver.c']
    for f in units:
        p = subprocess.run(['cc'] + flags + ff.get(f, []) + ['-I', str(out / 'src'), '-c', str(out / 'src' / f), '-o',
                           str(out / 'obj' / (f[:-2] + '.o'))], capture_output=True, text=True)
        if p.returncode: sys.stderr.write(p.stderr); sys.exit(f'compile FAILED: {f}')
    p = subprocess.run(['cc'] + flags + link + [str(out / 'obj' / (f[:-2] + '.o')) for f in units] + ['-o', str(out / 'crunner')],
                       capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit('link FAILED')
    print('probe: built', out / 'crunner')

if __name__ == '__main__':
    main()
