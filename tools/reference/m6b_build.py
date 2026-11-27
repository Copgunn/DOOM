#!/usr/bin/env python3
"""Build the M6b headless genuine-engine runner: the M6a runner with the original's pause displayed and observed, and
an earlier game of the same process on request (docs/M6B_REFERENCE_INVESTIGATION.md section 6.1).

The M6b runner compiles exactly the genuine files of the M6a runner (m6a_build.SOURCES with every frozen adapter,
through wire27_goldens.sources, so the overflow and address builds carry the two declared p_maputl.c adapters of wire
27, plus the one declared here, SANITIZER_ADAPTERS: p_mobj.c:400). The strict build gains no adapter: Nightmare (skill byte 4 of the demo header, g_game.c:1596) and the pause (a
button byte with BT_SPECIAL, g_game.c:698-712) already run in the original text. What changes is the driver, which is
generated, not forked: m6a_build.driver_text() with the exact-once patches of PATCHES, the M6a session code
tools/reference/m6a_session.c with the one exact-once patch SESSION_PATCH (written to the build directory; the
committed file is not edited), and the new code of tools/reference/m6b_session.c. The M5b and M6a drivers aborted when
a frame was displayed while paused; the patches put the five lines of d_main.c:303-311 there instead.

The declared configuration adds to M6a's:
  - --m6b-out FILE: the M6b stream (DOOM-M6B-RUNNER-v1: T, MP, B and Z rows; m6b_session.c);
  - --prior-game SKILL,RESPAWN,FAST (repeatable; evidence only): an earlier G_InitNew of this process;
  - --selftest-pause-y (test only).

Builds: strict, overflow, address (wire27_goldens.BUILDS; the flags of m3h9c_build.VARIANTS). variant() makes an
altered strict build for the self-tests: the strict build's objects with the named genuine files recompiled from text
with exact-once replacements, relinked; no file of the repository changes.

    m6b_build.py --variant strict|overflow|address|all [--out DIR] [--cc CC]
    m6b_build.py --contract tests/reference/m6b_runner_contract.json
    m6b_build.py --check            compare the committed contract with the regenerated text (writes nothing)

DIR defaults to $HOME/scratch/doom-m6b-build and must lie outside the repository.
"""
import argparse, json, re, shutil, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import m3h9c_build as base      # noqa: E402  (read-only: flags, layout probe, text, sha)
import m5a_build                # noqa: E402
import m5c_build as C           # noqa: E402
import m6a_build as B           # noqa: E402  (read-only: the M6a driver, stubs, units, link flags, contract)
import wire27_goldens as W27    # noqa: E402  (read-only: sources() with the two declared sanitizer adapters)
from m3h9c_build import ROOT, sha, text  # noqa: E402

HERE = ROOT / 'tools/reference'
SESSION_C = HERE / 'm6b_session.c'
CONTRACT = ROOT / 'tests/reference/m6b_runner_contract.json'
SCHEMA = 'DOOM-M6B-RUNNER-CONTRACT-v1'
DRIVER_NAME = 'm6b_driver.c'
BUILDS = ('strict', 'overflow', 'address')
WRAPPED = ['I_PauseSong', 'I_ResumeSong']
LINK_FLAGS = ['-Wl,' + ','.join(f'--wrap={w}' for w in WRAPPED)]
# One declared sanitizer adapter (overflow and address builds only; the strict build compiles the genuine text and is
# the claim), in the form of the frozen ones (m3h9c_build.SANITIZER_ADAPTERS, p_mobj.c:671 and 794: the same statement).
SANITIZER_ADAPTERS = [
    dict(file='p_mobj.c', line=400, original='    mo->angle = ANG45 * (mthing->angle/45);',
         adapted='    mo->angle = (angle_t)((unsigned)ANG45 * (unsigned)(mthing->angle/45));',
         reason='P_NightmareRespawn: ANG45 * q overflows int for quotients outside -4..3 (a spawn angle of 180 or more); the '
                'original targets wrap it modulo 2^32, which the unsigned product computes. The frozen adapters cover the same '
                'statement at p_mobj.c:671 and 794; before M6b no run reached line 400. Observed: UBSan stops the unadapted '
                'overflow build at p_mobj.c:400 on the first respawn of a demon placed facing west (item RESP4).'),
]
PAUSE_DRAW = '    if (paused) m6b_draw_pause ();                           /* M6b: d_main.c:303-311 */\n'
# Exact-once patches of the generated M6a driver text: (old, new).
PATCHES = [
    # the pause picture where the M5b driver aborted (the level frame)
    ('    if (paused) { fprintf (stderr, "display: paused\\n"); abort (); }   /* d_main.c:303-312: never in a demo without pause */\n',
     PAUSE_DRAW),
    # forward declarations, and the new code after the M6a session code
    ('static void m6a_melt_begin (int tic);                                              /* M6a */\n',
     'static void m6a_melt_begin (int tic);                                              /* M6a */\n'
     'static void m6b_draw_pause (void);                                                 /* M6b */\n'),
    ('#include "m6a_session.c"                 /* M6a: the session past the level boundary */\n',
     '#include "m6a_session.c"                 /* M6a: the session past the level boundary */\n'
     '#include "m6b_session.c"                 /* M6b: the pause, a prior game, their observers */\n'),
    # options
    ('        else if (m6a_option (argc, argv, &i)) ;   /* M6a: --game-mode, --melt-clock, --session-out */\n',
     '        else if (m6a_option (argc, argv, &i)) ;   /* M6a: --game-mode, --melt-clock, --session-out */\n'
     '        else if (m6b_option (argc, argv, &i)) ;   /* M6b: --m6b-out, --prior-game, --selftest-pause-y */\n'),
    # earlier games of this process, and the table rows around the demo's G_InitNew
    ('    command_source_begin (demo);                 /* the demo through the genuine -playdemo path */\n',
     '    m6b_prior_games ();                          /* M6b: --prior-game, and the T row before the demo */\n'
     '    command_source_begin (demo);                 /* the demo through the genuine -playdemo path */\n'
     '    m6b_table_row ("demo");                      /* M6b: the tables the demo plays with */\n'),
    # the B row and the totals
    ('        m6a_tic_end (tic);                       /* M6a: the session rows */\n',
     '        m6a_tic_end (tic);                       /* M6a: the session rows */\n'
     '        m6b_tic_end (tic);                       /* M6b: the B row of a paused tic */\n'),
    ('    m5b_finish ();\n    m5c_finish ();\n    m6a_finish ();\n',
     '    m5b_finish ();\n    m5c_finish ();\n    m6a_finish ();\n    m6b_finish ();\n'),
]
# The one exact-once patch of the m6a_session.c text (the intermission and finale frame).
SESSION_PATCH = ('    if (paused) { fprintf (stderr, "display: paused\\n"); abort (); }   /* d_main.c:303-312 */\n', PAUSE_DRAW)
DRIVER_HEADER = ('/*\n * M6b headless genuine-engine runner: the driver (declared adapter), GENERATED by tools/reference/m6b_build.py.\n'
                 ' * It is the generated M6a driver text with the exact-once patches of m6b_build.PATCHES and the new code of\n'
                 ' * tools/reference/m6b_session.c; no committed file is edited. Lines marked M6b are the patches.\n */\n')
PAUSE_ADAPTER = dict(
    name='pause picture in the D_Display mirror', file='d_main.c', line='303-311',
    original='if (paused) { y = automapactive ? 4 : viewwindowy+4; V_DrawPatchDirect (viewwindowx+(scaledviewwidth-68)/2, y, 0, '
             'W_CacheLumpName ("M_PAUSE", PU_CACHE)); }',
    realised_by='tools/reference/m6b_session.c m6b_draw_pause: the five source lines verbatim, called where the M5b level-frame '
                'mirror (m5b_after_view) and the M6a intermission and finale mirror (m6a_display_other) aborted on paused; '
                'd_main.c is not compiled (the D_Display mirror is the M5b declared adapter)',
    reason='the M5b and M6a runners refused a paused frame ("never in a demo without pause"); a demo with the pause command '
           'needs the frame the original draws.')
DRIVER_BEHAVIOUR = [
    'M6b runner (tools/reference/m6b_build.py; docs/M6B_REFERENCE_INVESTIGATION.md section 6.1): the generated M6a driver text with the patches of m6b_build.PATCHES, tools/reference/m6a_session.c with m6b_build.SESSION_PATCH and tools/reference/m6b_session.c; the genuine files, adapters, stubs and flags of the M6a runner, unchanged.',
    'Pause (M6b): the genuine G_Ticker toggles paused for a button byte with BT_SPECIAL whose low two bits are BTS_PAUSE (g_game.c:698-712) and the genuine P_Ticker returns at once while it is set (p_tick.c:135-136). A frame displayed while paused gets the pause picture (the pause adapter); with --render none no level frame is drawn and no picture either, while the intermission and finale frames are always drawn. A tic that ends paused gets a B row.',
    'Pause sound (M6b): I_PauseSong and I_ResumeSong (i_sound.c:848-858, both empty) are observed through -Wl,--wrap; each call is an MP row. Nothing else is changed in sound: S_UpdateSounds and the sound rows run every tic, paused or not.',
    'Prior games (M6b, --prior-game SKILL,RESPAWN,FAST, evidence only): before G_DeferedPlayDemo the runner sets respawnparm and fastparm and calls the genuine G_InitNew (SKILL, 1, 1) once per switch, in order; each is a level load (an L row in the session stream, an LV row in the sound stream after the first). T rows give the state durations S_SARG_RUN1..S_SARG_PAIN2 and the three missile speeds at the start, after each prior game and after the demo\'s own G_InitNew.',
]


def driver_text():
    s = B.driver_text()
    for old, new in PATCHES:
        if s.count(old) != 1: sys.exit(f'driver patch does not apply exactly once ({s.count(old)}): {old[:70]!r}')
        s = s.replace(old, new)
    return DRIVER_HEADER + s


def sources(name):
    """The compiled genuine files of one build: wire27_goldens.sources plus, in the sanitizer builds, SANITIZER_ADAPTERS
    (each original must stand at its declared line; the statement also occurs at p_mobj.c:627, which is left alone)."""
    out = W27.sources(name)
    if W27.BUILDS[name][1]:
        for a in SANITIZER_ADAPTERS:
            lines = out[a['file']].split('\n')
            if lines[a['line'] - 1] != a['original']: sys.exit(f"adapter FAILED: {a['file']}:{a['line']}")
            lines[a['line'] - 1] = a['adapted']
            out[a['file']] = '\n'.join(lines)
    return out


def session_text():
    s = text(B.SESSION_C)
    if s.count(SESSION_PATCH[0]) != 1: sys.exit('the m6a_session.c patch does not apply exactly once')
    return s.replace(*SESSION_PATCH)


def row_kinds():
    src = text(SESSION_C)
    block = src[src.index('static const char* M6B_COLUMNS ='):]
    joined = ''.join(re.findall(r'"((?:[^"\\]|\\.)*)"', block[:block.index(';')])).replace('\\n', '\n')
    return {f[1]: f[2:] for f in (l.split(',') for l in joined.strip().split('\n')) if f[0] == '#columns'}


def contract():
    m6a = B.contract()
    return dict(
        schema=SCHEMA,
        purpose='The M6a runner (tests/reference/m6a_runner_contract.json) with the pause displayed and observed and an earlier '
                'game of the same process on request. Expected streams come from the genuine code, never from Rust output.',
        m6a_contract_sha256=sha(C.render_json(m6a)),
        whole_files=m6a['whole_files'], source_sha256=m6a['source_sha256'], adapted_sha256=m6a['adapted_sha256'],
        sanitizer_adapters=W27.adapter_table() + [dict(a, original_sha256=sha(a['original']), adapted_sha256=sha(a['adapted']),
                                                       builds=['overflow', 'address']) for a in SANITIZER_ADAPTERS],
        wrapped=B.WRAPPED + WRAPPED, link_flags=B.LINK_FLAGS + LINK_FLAGS,
        pause_adapter=PAUSE_ADAPTER,
        declared_configuration=dict(
            m6a['declared_configuration'],
            switches=dict(m6a['declared_configuration']['switches'], **{'--prior-game': 'SKILL,RESPAWN,FAST (repeatable, at most 4)',
                                                                       '--selftest-pause-y': 'test only'}),
            lumps=m6a['declared_configuration']['lumps'] + '; M_PAUSE for a frame displayed while paused (d_main.c:310)'),
        runner_sha256={'tools/reference/m6b_session.c': sha(text(SESSION_C)), 'generated m6b_driver.c': sha(driver_text()),
                       'patched m6a_session.c': sha(session_text()), 'generated m6a_stubs.c': sha(B.stubs_text())},
        driver_patches=len(PATCHES), session_patches=1,
        driver_behaviour=DRIVER_BEHAVIOUR, m6b_row_kinds=row_kinds())


def default_out(): return Path.home() / 'scratch' / 'doom-m6b-build'


def compile_unit(cc, flags, flags_name, src, obj, f):
    extra = C.FILE_FLAGS.get(f, []) + (C.DRIVER_FLAGS.get(flags_name, []) if f == DRIVER_NAME else [])
    p = subprocess.run([cc] + flags + extra + ['-I', str(src), '-c', str(src / f), '-o', str(obj / (f[:-2] + '.o'))],
                       capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit(f'compile FAILED: {f}')


def link(cc, flags, obj, binary):
    units = B.UNITS + ['layout_probe.c', B.STUBS_NAME, DRIVER_NAME]
    p = subprocess.run([cc] + flags + B.LINK_FLAGS + LINK_FLAGS + [str(obj / (f[:-2] + '.o')) for f in units] + C.LIBS
                       + ['-o', str(binary)], capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit('link FAILED')
    return binary


def build(name, out, cc='cc'):
    """One build of the M6b runner under out/name: strict, overflow or address."""
    out = out.resolve()
    if out == ROOT or ROOT in out.parents: sys.exit('--out must lie outside the repository')
    flags_name = W27.BUILDS[name][0]
    work = out / name
    if work.exists(): shutil.rmtree(work)
    src, obj = work / 'src', work / 'obj'
    src.mkdir(parents=True); obj.mkdir()
    for h in base.SOURCE.glob('*.h'): shutil.copy2(h, src / h.name)
    srcs = sources(name)
    for f, s in srcs.items(): (src / f).write_bytes(s.encode('latin-1'))
    (src / 'layout_probe.c').write_text(base.layout_probe(srcs['r_data.c']))
    (src / 'm5a_types.h').write_text(m5a_build.types_header(srcs))
    (src / DRIVER_NAME).write_bytes(driver_text().encode('latin-1'))
    (src / B.STUBS_NAME).write_bytes(B.stubs_text().encode('latin-1'))
    (src / B.SESSION_C.name).write_bytes(session_text().encode('latin-1'))
    for f in (SESSION_C, B.WI_ACCESS): shutil.copy2(f, src / f.name)
    flags = base.VARIANTS[flags_name]
    for f in B.UNITS + ['layout_probe.c', B.STUBS_NAME, DRIVER_NAME]: compile_unit(cc, flags, flags_name, src, obj, f)
    return link(cc, flags, obj, work / 'crunner')


# A genuine file is compiled as itself, except wi_stuff.c, which the accessor unit includes.
UNIT_OF = {'wi_stuff.c': B.WI_ACCESS.name}


def variant(name, replacements, out, cc='cc'):
    """An altered strict build under out/name: replacements is {genuine file: [(old, new)]}, each exact-once in the
    strict build's text of that file. The strict build must exist under out/strict."""
    out = out.resolve()
    strict, work = out / 'strict', out / name
    if not (strict / 'crunner').exists(): sys.exit('variant: the strict build is missing')
    if work.exists(): shutil.rmtree(work)
    shutil.copytree(strict, work)
    src, obj = work / 'src', work / 'obj'
    for f, reps in replacements.items():
        s = (src / f).read_bytes().decode('latin-1')
        for old, new in reps:
            if s.count(old) != 1: sys.exit(f'variant {name}: a replacement of {f} applies {s.count(old)} times: {old[:60]!r}')
            s = s.replace(old, new)
        (src / f).write_bytes(s.encode('latin-1'))
        compile_unit(cc, base.VARIANTS['strict'], 'strict', src, obj, UNIT_OF.get(f, f))
    return link(cc, base.VARIANTS['strict'], obj, work / 'crunner')


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--variant', choices=list(BUILDS) + ['all'])
    ap.add_argument('--out', type=Path, default=default_out())
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--contract', type=Path)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    if a.check:
        if not CONTRACT.exists() or CONTRACT.read_text() != C.render_json(contract()):
            sys.exit(f'{CONTRACT.relative_to(ROOT)} differs from the regenerated contract')
        c = json.loads(CONTRACT.read_text())
        print(f"m6b runner contract matches: {len(c['whole_files'])} whole files (the M6a runner's), {c['driver_patches']} driver "
              f"patches, {c['session_patches']} session patch, {len(WRAPPED)} new wrapped symbols, {len(c['m6b_row_kinds'])} row kinds")
    if a.contract:
        a.contract.write_text(C.render_json(contract())); print(f'wrote {a.contract}')
    if a.variant:
        for v in (BUILDS if a.variant == 'all' else [a.variant]): print(f'{v}: built {build(v, a.out, a.cc)}')
    if not (a.check or a.contract or a.variant): ap.error('nothing to do')


if __name__ == '__main__':
    main()
