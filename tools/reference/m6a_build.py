#!/usr/bin/env python3
"""Build the M6a headless genuine-engine runner: the M5c runner carried past the level boundary
(docs/M6A_REFERENCE_INVESTIGATION.md section 3.1).

The M6a runner compiles the 49 genuine files of the M5c runner (m5c_build.SOURCES, with every M5a,
M5b and M5c adapter applied exactly as m5c_build.adapted_sources applies it) and SESSION: f_finale.c
whole and unmodified, and wi_stuff.c whole and unmodified through the declared accessor unit
tools/reference/m6a_wi_access.c (it #includes the genuine text and adds one read-only function, the
one new declared adapter; section 3.1 item 5). The driver is generated, not forked by hand: the frozen
tools/reference/m5c_driver.c text with the exact-once patches of PATCHES (each asserted to apply
once) and the new code of tools/reference/m6a_session.c included before main. The stubs are the
frozen tools/reference/m5c_stubs.c text without the five stubs the two new files define (WI_Start,
WI_Ticker, F_StartFinale, F_Ticker, F_Responder). This module imports m5c_build (and through it the
M5b, M5a, M4b and M3h9c build modules) as a library and owns its own build() and contract(); nothing
of theirs changes, and no frozen file is edited.

The declared configuration adds to M5c's (m5c_build.DECLARED_CONFIGURATION):
  - --game-mode registered|retail, required, echoed in every stream header and on stdout (owner's
    ruling U4): the configuration adapter GAME_MODE_ADAPTER stands for IdentifyVersion;
  - --melt-clock counter|unit (default counter, the M5c declared clock); evidence only;
  - --session-out FILE: the session stream (DOOM-M6A-SESSION-v1).

    m6a_build.py --variant strict|overflow|address|all|strict32 [--out DIR] [--cc CC]
    m6a_build.py --contract tests/reference/m6a_runner_contract.json
    m6a_build.py --check            compare the committed contract with the regenerated text (writes nothing)

DIR defaults to $HOME/scratch/doom-m6a-build and must lie outside the repository.
"""
import argparse, json, re, shutil, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import m3h9c_build as base  # noqa: E402
import m4b_build            # noqa: E402
import m5a_build            # noqa: E402
import m5c_build as C       # noqa: E402
from m3h9c_build import ROOT, SOURCE, sha, text  # noqa: E402

HERE = ROOT / 'tools/reference'
SESSION_C, WI_ACCESS = HERE / 'm6a_session.c', HERE / 'm6a_wi_access.c'
CONTRACT = ROOT / 'tests/reference/m6a_runner_contract.json'
SCHEMA = 'DOOM-M6A-RUNNER-v1'
DRIVER_NAME, STUBS_NAME = 'm6a_driver.c', 'm6a_stubs.c'
SESSION = ['f_finale.c', 'wi_stuff.c']
SOURCES = C.SOURCES + SESSION
# wi_stuff.c is compiled through the accessor unit; every other file as itself.
UNITS = C.SOURCES + ['f_finale.c', WI_ACCESS.name]
SESSION_WRAPPED = ['P_SetupLevel', 'WI_Start', 'WI_Ticker', 'F_StartFinale', 'F_Ticker']
WRAPPED = C.WRAPPED + SESSION_WRAPPED
LINK_FLAGS = ['-Wl,' + ','.join(f'--wrap={w}' for w in WRAPPED)]
STUB_DROP = ['WI_Start', 'WI_Ticker', 'F_StartFinale', 'F_Ticker', 'F_Responder']
ACCESSOR_ADAPTER = dict(
    name='wi_stuff.c accessor unit', file='tools/reference/m6a_wi_access.c',
    realised_by='#include "wi_stuff.c" (the genuine text, unchanged and whole) followed by m6a_wi_statics, which only reads '
                'the file statics state, acceleratestage, me, cnt, bcnt, firstrefresh, sp_state, cnt_kills[0], cnt_items[0], '
                'cnt_secret[0], cnt_time, cnt_par, cnt_pause, snl_pointeron, wbs and the animation table (ctr, nexttic)',
    reason='wi_stuff.c keeps the intermission state in file statics (wi_stuff.c:304-331, 749, 1316, 226-275) that no other '
           'unit can name; the session rows need them (docs/M6A_REFERENCE_INVESTIGATION.md section 3.1 item 5).')
GAME_MODE_ADAPTER = dict(
    name='declared game mode', file='d_main.c', line='563-717', original='IdentifyVersion: gamemode from the IWAD file name',
    realised_by='tools/reference/m6a_session.c m6a_set_game_mode: gamemode = the --game-mode value (registered | retail, '
                'required), at the point of the M5c driver text where it hard-coded registered (after W_InitMultipleFiles, '
                'before R_Init and P_InitSwitchList); refused (status 2) when the first --wad file is named, case-insensitively, '
                'doomu.wad and the mode is not retail, doom.wad and the mode is not registered, or doom1.wad, doom2.wad, '
                'doom2f.wad, plutonia.wad or tnt.wad (modes this runner does not admit)',
    reason='owner\'s ruling U4: the game mode is an explicit, declared setting and the genuine engine runs on a scratch copy of '
           'the IWAD named for the mode (doomu.wad retail, d_main.c:690-695; doom.wad registered, 697-702); a synthetic IWAD '
           'under another name runs in the declared mode, where IdentifyVersion would report "Game mode indeterminate" '
           '(d_main.c:711-713).')
DECLARED_CONFIGURATION = dict(
    C.DECLARED_CONFIGURATION,
    process='a fresh process, the first level loaded at gametic 0 during the first G_Ticker of the -playdemo path; G_Ticker '
            'then runs every tic through every exit, intermission, load, reborn and the finale until the demo marker, a '
            'refusal or an I_Error',
    switches=dict(C.DECLARED_CONFIGURATION['switches'], **{'--game-mode': ['registered', 'retail'], '--melt-clock': ['counter', 'unit']}),
    defaults_of_switches=dict(C.DECLARED_CONFIGURATION['defaults_of_switches'], **{'--game-mode': 'none: required',
                                                                                  '--melt-clock': 'counter'}),
    lumps=C.DECLARED_CONFIGURATION['lumps'] + '; past a boundary also the 79 intermission names wi_stuff.c caches '
          '(WI_loadData, wi_stuff.c:1538-1706), D_INTER and D_VICTOR (wi_stuff.c:1508-1515, f_finale.c:114), FLOOR4_8 '
          '(f_finale.c:120) and the stage-1 picture (CREDIT retail, HELP2 otherwise, f_finale.c:714-720)')

# Exact-once patches of the frozen m5c_driver.c text: (old, new).
PATCHES = [
    # the M_Random observer: an intermission bucket and a finale bucket before "other" (section 3.1 item 6)
    ('    if (in_sound) snd_draws++; else if (in_st_ticker) st_draws++; else if (in_wipe) wipe_draws++; else other_draws++;\n',
     '    if (in_sound) snd_draws++; else if (in_st_ticker) st_draws++; else if (in_wipe) wipe_draws++;\n'
     '    else if (m6a_in_wi) m6a_wi_draws++; else if (m6a_in_fin) m6a_fin_draws++; else other_draws++;   /* M6a */\n'),
    ('static int in_sound, in_wipe, snd_draws, wipe_draws, last_rnd, rnd_after_st = -1;   /* M5c */\n',
     'static int in_sound, in_wipe, snd_draws, wipe_draws, last_rnd, rnd_after_st = -1;   /* M5c */\n'
     'static int m6a_in_wi, m6a_in_fin, m6a_wi_draws, m6a_fin_draws, m6a_melt_unit;      /* M6a */\n'
     'static void m6a_melt_begin (int tic);                                              /* M6a */\n'),
    # the melt hook: the Y row, and the declared unit clock behind --melt-clock unit
    ('    wipe_EndScreen (0, 0, SCREENWIDTH, SCREENHEIGHT);        /* d_main.c:327 */\n'
     '    wipestart = I_GetTime () - 1;                             /* d_main.c:329 */\n',
     '    wipe_EndScreen (0, 0, SCREENWIDTH, SCREENHEIGHT);        /* d_main.c:327 */\n'
     '    m6a_melt_begin (tic);                                     /* M6a: the Y row */\n'
     '    wipestart = m6a_melt_unit ? 0 : I_GetTime () - 1;         /* d_main.c:329; M6a: no clock read under --melt-clock unit */\n'),
    ('        do { nowtime = I_GetTime (); tics = nowtime - wipestart; } while (!tics);   /* d_main.c:333-337 */\n',
     '        if (m6a_melt_unit) { nowtime = wipestart; tics = 1; }   /* M6a: the declared unit clock, one tick per step */\n'
     '        else do { nowtime = I_GetTime (); tics = nowtime - wipestart; } while (!tics);   /* d_main.c:333-337 */\n'),
    # options and the declared game mode
    ('        else if (!strcmp (argv[i], "--no-dump")) nodump = 1;\n',
     '        else if (!strcmp (argv[i], "--no-dump")) nodump = 1;\n'
     '        else if (m6a_option (argc, argv, &i)) ;   /* M6a: --game-mode, --melt-clock, --session-out */\n'),
    ('    if (view_out) fputs (VIEW_COLUMNS, view_out);\n',
     '    if (view_out) fputs (VIEW_COLUMNS, view_out);\n'
     '    m6a_begin_streams ();                        /* M6a: the stream headers and the declared configuration */\n'),
    ('    gamemode = registered;                       /* d_main.c:697-701 (doom.wad) */\n',
     '    if (m6a_set_game_mode ()) return 2;          /* M6a: the declared game mode (IdentifyVersion, d_main.c:563-717) */\n'),
    # dumps and view records only in a level; the non-level display
    ('        if (!nodump) {\n            char name[64];\n',
     '        if (!nodump && gamestate == GS_LEVEL) {  /* M6a: dumps in a level only */\n            char name[64];\n'),
    ('        if (view_out) { write_view (tic); fflush (view_out); }\n'
     '        if (render_mode) m5b_display (tic); else m5c_display_norender (tic);\n',
     '        if (view_out && gamestate == GS_LEVEL) { write_view (tic); fflush (view_out); }   /* M6a: in a level only */\n'
     '        if (gamestate != GS_LEVEL) m6a_display_other (tic);   /* M6a: d_main.c:250-256 */\n'
     '        else if (render_mode) m5b_display (tic); else m5c_display_norender (tic);\n'),
    # the session rows; no stop at a boundary: G_Ticker runs every tic
    ('        m5c_tic_end (tic);\n        if (refused) break;',
     '        m5c_tic_end (tic);\n        m6a_tic_end (tic);                       /* M6a: the session rows */\n        if (refused) break;'),
    ('        if (gameaction == ga_completed || gameaction == ga_victory || gameaction == ga_worlddone) {\n'
     '            exited = 1;                          /* the next G_Ticker would enter the intermission */\n'
     '            break;\n'
     '        }\n'
     '        for (i = 0; i < MAXPLAYERS; i++)\n'
     '            if (playeringame[i] && players[i].playerstate == PST_REBORN) reborn = 1;\n'
     '        if (reborn) break;                       /* the next G_Ticker would reload the level */\n',
     '        /* M6a: no stop at an exit or a reborn request; the next G_Ticker runs the genuine gameaction loop */\n'),
    ('    m5b_finish ();\n    m5c_finish ();\n',
     '    m5b_finish ();\n    m5c_finish ();\n    m6a_finish ();\n'),
    # the new code, before main
    ('static double now (void)\n',
     '#include "m6a_session.c"                 /* M6a: the session past the level boundary */\n\nstatic double now (void)\n'),
]
DRIVER_HEADER = ('/*\n * M6a headless genuine-engine runner: the driver (declared adapter), GENERATED by tools/reference/m6a_build.py.\n'
                 ' * It is the frozen tools/reference/m5c_driver.c text with the exact-once patches of m6a_build.PATCHES and the new\n'
                 ' * code of tools/reference/m6a_session.c; neither frozen file is edited. Lines marked M6a are the patches.\n */\n')
STUBS_HEADER = ('/* M6a stubs, GENERATED by tools/reference/m6a_build.py: the frozen tools/reference/m5c_stubs.c text without the stubs of\n'
                ' * ' + ', '.join(STUB_DROP) + ', which the genuine wi_stuff.c and f_finale.c now define. */\n')
DRIVER_BEHAVIOUR = [
    'M6a runner (tools/reference/m6a_build.py; docs/M6A_REFERENCE_INVESTIGATION.md section 3.1): the M5c driver text with the patches of m6a_build.PATCHES and tools/reference/m6a_session.c; the 49 genuine files of the M5c runner with every adapter, plus f_finale.c whole and unmodified and wi_stuff.c whole and unmodified through the accessor unit tools/reference/m6a_wi_access.c; the M5c stubs without WI_Start, WI_Ticker, F_StartFinale, F_Ticker and F_Responder.',
    'No stop at a boundary (M6a; replaces the M5c driver\'s end on an exit and end on a reborn request): G_Ticker runs every tic; the genuine gameaction loop (g_game.c:617-650) runs G_DoCompleted and WI_Start, G_DoWorldDone and the next load, G_DoReborn and the reload, and F_StartFinale. The run ends at the demo marker (end=marker), a refusal (end=refused) or an I_Error.',
    'Per-level bookkeeping (M6a): a P_SetupLevel observer (called by G_DoLoadLevel, g_game.c:484) rebuilds the per-level driver tables (lines with a special, mapped lines) after every load but the first, before the new level\'s first P_Ticker, and writes an L row; the observer table is already reset at each load by the P_InitThinkers observer (p_setup.c:622). Dumps (tic_NNNNN.txt) and view records are written only on tics that end in GS_LEVEL.',
    'Display (M6a): on a tic that ends in GS_INTERMISSION or GS_FINALE the D_Display mirror calls WI_Drawer or F_Drawer (d_main.c:250-256), I_SetPalette (PLAYPAL) on entering a state other than the level (d_main.c:274-275) and the genuine melt when gamestate != wipegamestate (d_main.c:223-229, 320-344), sharing the level mirror\'s statics; the frame stream gets a G row (the whole frame\'s SHA-256, the palette, the palette set during the frame, rndindex and the 2D tracker counts). This display runs with --render none as well.',
    'The M_Random contract (M6a): as M5c, plus an intermission bucket (draws inside WI_Start and WI_Ticker) and a finale bucket (draws inside F_StartFinale and F_Ticker); any other draw is counted as other and must stay 0.',
    'Game mode (M6a, owner\'s ruling U4): --game-mode registered|retail is required and sets gamemode where the M5c driver hard-coded registered (the configuration adapter game_mode_adapter); the mode is printed on stdout and in the #config-m6a lines of the frame and sound streams and the #config line of the session stream.',
    'Session stream (M6a, --session-out FILE, schema DOOM-M6A-SESSION-v1, row kinds in session_row_kinds): an L row per load; per tic a G row (game state and action, episode, map, leveltime, gametic, levelstarttic, both random indices, every M_Random bucket, wipegamestate, secretexit, paused, loads and wipes so far) and a C row (the console player\'s carried and reset fields); in the intermission an I row (wminfo) and a J row (the wi_stuff.c statics through the accessor unit); in the finale an F row (stage, count, flat, text); a Z row of totals. Frame stream additions (DOOM-M6A-FRAMES-v1 on the #schema-m6a line, after the M5c header): the G row and a Y row per melt (its start and end screens\' digests and the palette set when it starts). Sound stream addition: an LV row at every load after the first, before S_Start.',
    'Melt hook (M6a, evidence only): --melt-clock counter (default) is the M5c declared clock (the I_GetTime counter stub); --melt-clock unit passes ticks = 1 to every wipe_ScreenWipe call and reads no clock in the melt. The V row of every step (M5c) is written under either.',
]


def driver_text():
    s = text(C.DRIVER)
    for old, new in PATCHES:
        if s.count(old) != 1: sys.exit(f'driver patch does not apply exactly once ({s.count(old)}): {old[:70]!r}')
        s = s.replace(old, new)
    return DRIVER_HEADER + s


def stubs_text():
    lines, out, skip = text(C.STUBS).split('\n'), [], 0
    dropped = set()
    for line in lines:
        if skip: skip = 0; continue
        m = re.match(r'/\* STUB (\w+) ', line)
        if m and m[1] in STUB_DROP: dropped.add(m[1]); skip = 1; continue
        out.append(line)
    if dropped != set(STUB_DROP): sys.exit(f'stub drop FAILED: {sorted(set(STUB_DROP) - dropped)} not found')
    return STUBS_HEADER + '\n'.join(out)


def stubs():
    found = [dict(symbol=m[0], kind='variable' if m[1] == 'var' else 'function', classification=m[1], vanilla=m[2], reason=m[3])
             for m in base.STUB_LINE.findall(stubs_text())]
    if len({s['symbol'] for s in found}) != len(found): sys.exit('stub list FAILED: duplicate symbol')
    return found


def session_row_kinds():
    src = text(SESSION_C)
    out = {}
    for name in ('M6A_SESSION_COLUMNS', 'M6A_FRAME_COLUMNS'):
        block = src[src.index(f'static const char* {name} ='):]
        block = block[:block.index(';')]
        joined = ''.join(re.findall(r'"((?:[^"\\]|\\.)*)"', block)).replace('\\n', '\n')
        out[name] = {f[1]: f[2:] for f in (l.split(',') for l in joined.strip().split('\n')) if f[0] == '#columns'}
    return out


def contract():
    c = C.contract()
    st = stubs()
    kinds = session_row_kinds()
    c.update(
        schema=SCHEMA,
        purpose='The M5c runner carried past the level boundary: the genuine gameplay code, renderer, status bar, HUD, sound '
                'logic and melt plus the genuine intermission (wi_stuff.c) and finale (f_finale.c), driven through the vanilla '
                '-playdemo path with G_Ticker every tic and a mirror of D_Display for every game state; run in CI on synthetic '
                'WADs and locally on a scratch copy of a real IWAD named for the declared game mode. Expected streams come from '
                'the genuine code, never from Rust output.',
        whole_files=SOURCES,
        source_sha256={f: sha(text(SOURCE / f)) for f in sorted(SOURCES)},
        adapted_sha256=dict(strict={f: sha(s) for f, s in adapted_sources('strict').items()},
                            sanitizer={f: sha(s) for f, s in adapted_sources('overflow').items()}),
        link_flags=LINK_FLAGS, wrapped=WRAPPED,
        accessor_adapter=ACCESSOR_ADAPTER, game_mode_adapter=GAME_MODE_ADAPTER,
        declared_configuration=DECLARED_CONFIGURATION,
        stubs=st, stub_counts=dict(functions=sum(s['kind'] == 'function' for s in st), variables=sum(s['kind'] == 'variable' for s in st),
                                   **{k: sum(s['classification'] == k for s in st) for k in ('io', 'reads', 'writes', 'var')}),
        runner_sha256={'tools/reference/m5c_driver.c': sha(text(C.DRIVER)), 'tools/reference/m5c_stubs.c': sha(text(C.STUBS)),
                       'tools/reference/m6a_session.c': sha(text(SESSION_C)), 'tools/reference/m6a_wi_access.c': sha(text(WI_ACCESS)),
                       'generated m6a_driver.c': sha(driver_text()), 'generated m6a_stubs.c': sha(stubs_text())},
        driver_patches=len(PATCHES),
        driver_behaviour=C.behaviour() + DRIVER_BEHAVIOUR,
        session_row_kinds=kinds['M6A_SESSION_COLUMNS'], m6a_frame_row_kinds=kinds['M6A_FRAME_COLUMNS'],
        m6a_sound_row_kinds={'LV': ['tic', 'episode', 'map']})
    return c


def adapted_sources(variant):
    """Every compiled genuine file with the adapters of this build (f_finale.c and wi_stuff.c: none)."""
    out = C.adapted_sources(variant)
    for f in SESSION: out[f] = text(SOURCE / f)
    return out


def default_out(): return Path.home() / 'scratch' / 'doom-m6a-build'


def build(variant, out, cc='cc'):
    out = out.resolve()
    if out == ROOT or ROOT in out.parents: sys.exit('--out must lie outside the repository')
    if variant in base.OPTIONAL_VARIANTS and not base.probe_m32(cc):
        print(f'skip: {variant}: {cc} cannot build and run a -m32 program (no 32-bit toolchain)'); return None
    work = out / variant
    if work.exists(): shutil.rmtree(work)
    src, obj = work / 'src', work / 'obj'
    src.mkdir(parents=True); obj.mkdir()
    for h in SOURCE.glob('*.h'): shutil.copy2(h, src / h.name)
    srcs = adapted_sources(variant)
    for f, s in srcs.items(): (src / f).write_bytes(s.encode('latin-1'))
    (src / 'layout_probe.c').write_text(base.layout_probe(srcs['r_data.c']))
    (src / 'm5a_types.h').write_text(m5a_build.types_header(srcs))
    (src / DRIVER_NAME).write_bytes(driver_text().encode('latin-1'))
    (src / STUBS_NAME).write_bytes(stubs_text().encode('latin-1'))
    shutil.copy2(SESSION_C, src / SESSION_C.name); shutil.copy2(WI_ACCESS, src / WI_ACCESS.name)
    units = UNITS + ['layout_probe.c', STUBS_NAME, DRIVER_NAME]
    flags, warnings = {**base.VARIANTS, **base.OPTIONAL_VARIANTS}[variant], {}
    for f in units:
        extra = C.FILE_FLAGS.get(f, []) + (C.DRIVER_FLAGS.get(variant, []) if f == DRIVER_NAME else [])
        p = subprocess.run([cc] + flags + extra + ['-I', str(src), '-c', str(src / f), '-o', str(obj / (f[:-2] + '.o'))],
                           capture_output=True, text=True)
        if p.returncode: sys.stderr.write(p.stderr); sys.exit(f'compile FAILED: {f}')
        if 'warning:' in p.stderr:
            warnings[f] = p.stderr.count('warning:'); (work / f'{f}.warnings.txt').write_text(p.stderr)
    nm = subprocess.run(['nm', '--defined-only', '-g', str(obj / 'm6a_stubs.o')], capture_output=True, text=True, check=True)
    defined = {l.split()[-1] for l in nm.stdout.splitlines() if l.strip() and '__odr_asan' not in l
               and '__x86.get_pc_thunk.' not in l} - C.DRIVER_ONLY
    declared = {s['symbol'] for s in stubs()}
    if defined != declared: sys.exit(f'stub list FAILED: defined-not-declared {sorted(defined - declared)}, '
                                     f'declared-not-defined {sorted(declared - defined)}')
    binary = work / 'crunner'
    p = subprocess.run([cc] + flags + LINK_FLAGS + [str(obj / (f[:-2] + '.o')) for f in units] + C.LIBS + ['-o', str(binary)],
                       capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit('link FAILED')
    man = dict(contract(), variant=variant, compiler=subprocess.run([cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0],
               binary=str(binary), binary_sha256=sha(binary.read_bytes()), warnings=warnings)
    if variant in base.OPTIONAL_VARIANTS:
        man.update(optional_variant=dict(flags=flags, note=base.OPTIONAL_NOTES[variant] + '; the M5a renderer adapters are not '
                                         'applied either; the two errno lines of sound_abi_adapters are', source_sha256={f: sha(s) for f, s in srcs.items()}),
                   file=subprocess.run(['file', '-b', str(binary)], capture_output=True, text=True).stdout.strip())
    (out / f'manifest_{variant}.json').write_text(C.render_json(man))
    print(f'{variant}: built {binary}; {sum(warnings.values())} warnings in {len(warnings)} files')
    return binary


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--variant', choices=list(base.VARIANTS) + ['all'] + list(base.OPTIONAL_VARIANTS))
    ap.add_argument('--out', type=Path, default=default_out())
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--contract', type=Path)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    if a.check:
        if not CONTRACT.exists() or CONTRACT.read_text() != C.render_json(contract()):
            sys.exit(f'{CONTRACT.relative_to(ROOT)} differs from the regenerated contract')
        c = json.loads(CONTRACT.read_text())
        print(f"m6a runner contract matches: {len(c['whole_files'])} whole files, {c['driver_patches']} driver patches, "
              f"2 declared M6a adapters (accessor unit, game mode), {c['stub_counts']['functions']} function + "
              f"{c['stub_counts']['variables']} variable stubs, {len(c['wrapped'])} wrapped symbols")
    if a.contract:
        a.contract.write_text(C.render_json(contract())); print(f'wrote {a.contract}')
    if a.variant:
        for v in (base.VARIANTS if a.variant == 'all' else [a.variant]): build(v, a.out, a.cc)
    if not (a.check or a.contract or a.variant): ap.error('nothing to do')


if __name__ == '__main__':
    main()
