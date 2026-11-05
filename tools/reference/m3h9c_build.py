#!/usr/bin/env python3
"""Build the M3h9c headless genuine-engine runner (whole genuine files, declared adapters).

The runner compiles g_game.c, the p_*.c gameplay files and their support files from the
preserved linuxdoom-1.10 source whole (WHOLE), plus a declared verbatim extract of
r_main.c (its global-definition block and six geometry bodies), the declared stubs
(m3h9c_stubs.c) and the headless driver (m3h9c_driver.c); p_saveg.c is left out. Declared
LP64 ABI adapters (ABI_ADAPTERS, every build) and sanitizer adapters (SANITIZER_ADAPTERS,
the two sanitizer builds only) are exact one-line text replacements: the build fails
unless each original text occurs exactly once in its file and stands at its declared line.

    m3h9c_build.py --variant strict|overflow|address|all [--out DIR] [--cc CC]
    m3h9c_build.py --contract tests/reference/m3h9c_runner_contract.json
    m3h9c_build.py --check

A build writes DIR/<variant>/crunner and DIR/manifest_<variant>.json (the contract plus
the compiler, the binary path and its SHA-256, the warnings). DIR defaults to
$HOME/scratch/doom-m3h9c-build and must lie outside the repository. --contract writes the
host-independent part (sources, adapters, extract, flags, stubs, driver behaviour, dump
row kinds) and --check regenerates it in memory and fails if the committed file differs;
neither compiles anything. The runner itself reads a WAD and a demo lump at run time:
tools/local/e1m1_reference.py runs it on a local IWAD, which never enters the repository.
"""
import argparse, hashlib, json, os, re, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'linuxdoom-1.10'
HERE = ROOT / 'tools/reference'
CONTRACT = ROOT / 'tests/reference/m3h9c_runner_contract.json'
DRIVER, STUBS = HERE / 'm3h9c_driver.c', HERE / 'm3h9c_stubs.c'
SCHEMA = 'DOOM-M3H9C-RUNNER-v1'
REVISION = 'a77dfb96cb91780ca334d0d4cfd86957558007e0'
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode('latin-1')).hexdigest()

WHOLE = ('g_game.c p_ceilng.c p_doors.c p_enemy.c p_floor.c p_inter.c p_lights.c p_map.c p_maputl.c '
         'p_mobj.c p_plats.c p_pspr.c p_setup.c p_sight.c p_spec.c p_switch.c p_telept.c p_tick.c p_user.c '
         'm_random.c m_fixed.c m_bbox.c m_argv.c tables.c info.c d_items.c sounds.c dstrings.c doomstat.c '
         'doomdef.c w_wad.c z_zone.c r_sky.c r_data.c').split()

BASE = ['-std=gnu89', '-O0', '-fwrapv', '-fno-strict-aliasing', '-ffp-contract=off',
        '-fexcess-precision=standard', '-fsigned-char', '-DNORMALUNIX', '-DLINUX']
_NOWRAP = [f for f in BASE if f != '-fwrapv'] + ['-fno-wrapv']
VARIANTS = {
    'strict': BASE,
    'overflow': _NOWRAP + ['-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow',
                           '-fno-sanitize-recover=all'],
    'address': _NOWRAP + ['-fsanitize=address,undefined', '-fno-sanitize-recover=all', '-fno-sanitize=alignment'],
}
VARIANT_NOTES = {
    'overflow': 'run as is',
    'address': 'run with ASAN_OPTIONS=detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage); '
               '-fno-sanitize=alignment because z_zone.c rounds block sizes to 4 (z_zone.c:195) while the '
               'LP64 memblock_t needs 8-byte alignment',
}
FILE_FLAGS = {'r_data.c': ['-include', 'stdint.h'], 'z_zone.c': ['-include', 'stdint.h']}
# Linker-level observers (m3h9c_driver.c __wrap_*): every cross-object call of these reaches
# the driver's recording wrapper, which then calls the genuine function.
WRAPPED = ['P_AddThinker', 'Z_Free', 'P_SetThingPosition', 'P_InitThinkers', 'Z_Malloc']
LINK_FLAGS = ['-Wl,' + ','.join(f'--wrap={w}' for w in WRAPPED)]

ABI_ADAPTERS = [
    dict(file='p_setup.c', line=536, original='    linebuffer = Z_Malloc (total*4, PU_LEVEL, 0);',
         adapted='    linebuffer = Z_Malloc (total*sizeof(*linebuffer), PU_LEVEL, 0);',
         reason='P_GroupLines: linebuffer holds total line_t pointers (8 bytes each on LP64); total*4 overruns the zone block and Z_CheckHeap (g_game.c:488) calls I_Error.'),
    dict(file='r_data.c', line=90, original='    void\t\t**columndirectory;\t// OBSOLETE',
         adapted='    int\t\tcolumndirectory;\t// OBSOLETE',
         reason='On-disk maptexture_t: a 4-byte field as on the 32-bit original, so patchcount is at byte 20 and patches at 22 (the driver checks the generated layout probe before running).'),
    dict(file='r_data.c', line=482, original='    textures = Z_Malloc (numtextures*4, PU_STATIC, 0);',
         adapted='    textures = Z_Malloc (numtextures*sizeof(void*), PU_STATIC, 0);',
         reason='R_InitTextures: textures is a pointer array (texture_t**).'),
    dict(file='r_data.c', line=483, original='    texturecolumnlump = Z_Malloc (numtextures*4, PU_STATIC, 0);',
         adapted='    texturecolumnlump = Z_Malloc (numtextures*sizeof(void*), PU_STATIC, 0);',
         reason='R_InitTextures: a pointer array (short**).'),
    dict(file='r_data.c', line=484, original='    texturecolumnofs = Z_Malloc (numtextures*4, PU_STATIC, 0);',
         adapted='    texturecolumnofs = Z_Malloc (numtextures*sizeof(void*), PU_STATIC, 0);',
         reason='R_InitTextures: a pointer array (unsigned short**).'),
    dict(file='r_data.c', line=485, original='    texturecomposite = Z_Malloc (numtextures*4, PU_STATIC, 0);',
         adapted='    texturecomposite = Z_Malloc (numtextures*sizeof(void*), PU_STATIC, 0);',
         reason='R_InitTextures: a pointer array (byte**).'),
    dict(file='r_data.c', line=642, original='    colormaps = (byte *)( ((int)colormaps + 255)&~0xff); ',
         adapted='    colormaps = (byte *)( ((intptr_t)colormaps + 255)&~0xff); ',
         reason='R_InitColormaps truncates a pointer to int; the runner does not call it, the file is adapted so it compiles LP64-clean (stdint.h comes from -include on the command line).'),
    dict(file='z_zone.c', line=440, original='    if (tag >= PU_PURGELEVEL && (unsigned)block->user < 0x100)',
         adapted='    if (tag >= PU_PURGELEVEL && (uintptr_t)block->user < 0x100)',
         reason='Z_ChangeTag: the owner test truncates the 64-bit user pointer to 32 bits, so a valid owner whose address has small low 32 bits (observed under ASan on the CI runner: W_CacheLumpNum re-caching the one patch lump) raises "an owner is required for purgable blocks"; the 32-bit original compares the whole pointer, which uintptr_t does on LP64 (stdint.h comes from -include on the command line).'),
]
SANITIZER_ADAPTERS = [
    dict(file='p_mobj.c', line=671, original='    mobj->angle\t= ANG45 * (mthing->angle/45);',
         adapted='    mobj->angle\t= (angle_t)((unsigned)ANG45 * (unsigned)(mthing->angle/45));',
         reason='P_SpawnPlayer: ANG45 * q overflows int for quotients outside -4..3; the original targets wrap it modulo 2^32, which the unsigned product computes (same statement form as p_mobj.c:794).'),
    dict(file='p_mobj.c', line=794, original='    mobj->angle = ANG45 * (mthing->angle/45);',
         adapted='    mobj->angle = (angle_t)((unsigned)ANG45 * (unsigned)(mthing->angle/45));',
         reason='P_SpawnMapThing: the declared M3h9b adapter (tools/reference/m3h9b_reference.py SANITIZER_ADAPTERS[0]).'),
    dict(file='p_map.c', line=1221, original='    dist = (damage+MAXRADIUS)<<FRACBITS;',
         adapted='    dist = (fixed_t)(((unsigned)damage + (unsigned)MAXRADIUS) << FRACBITS);',
         reason='P_RadiusAttack: the declared M3h9b adapter (tools/reference/m3h9b_reference.py SANITIZER_ADAPTERS[1]).'),
    dict(file='p_enemy.c', line=816, original='    angle += (P_Random()-P_Random())<<20;',
         adapted='    angle = (int)((unsigned)angle + (unsigned)((P_Random()-P_Random())<<20));',
         reason='A_PosAttack: `int angle` holds the actor angle (angle_t converted to int) and the spread is added in int, which overflows for angles near the int limits; the original targets wrap it modulo 2^32, as the unsigned sum does (the P_Random subexpression is unchanged). Needed: the overflow build reports this statement on the synthetic chaos script.'),
    dict(file='p_enemy.c', line=839, original='\tangle = bangle + ((P_Random()-P_Random())<<20);',
         adapted='\tangle = (int)((unsigned)bangle + (unsigned)((P_Random()-P_Random())<<20));',
         reason='A_SPosAttack: the same int sum as p_enemy.c:816; declared for robustness.'),
    dict(file='p_enemy.c', line=860, original='    angle = bangle + ((P_Random()-P_Random())<<20);',
         adapted='    angle = (int)((unsigned)bangle + (unsigned)((P_Random()-P_Random())<<20));',
         reason='A_CPosAttack: the same int sum as p_enemy.c:816; declared for robustness.'),
]
R_MAIN_FUNCS = ['R_PointOnSide', 'R_PointOnSegSide', 'R_PointToAngle', 'R_PointToAngle2', 'R_PointToDist',
                'R_PointInSubsector']
DRIVER_BEHAVIOUR = [
    'Sequence: Z_Init; W_InitMultipleFiles over --wad files added as D_AddFile does (IWAD, then the .lmp named by its base name); gamemode = registered (d_main.c:697-701); R_InitTextures; R_InitFlats; P_InitSwitchList; P_InitPicAnims; D_CheckNetGame single-player values; singledemo = true; G_DeferedPlayDemo; G_DoPlayDemo; dump "load"; then { G_Ticker; gametic++; dump } per tic.',
    'The demo header selects skill, episode and map (g_game.c:1596-1598); the driver has no map argument.',
    'I_Quit (vanilla exit(0), i_system.c:116-124) sets a flag and longjmps to the driver, so no statement after the call in G_CheckDemoStatus or G_Ticker runs, as with exit.',
    'After each dumped tic the driver stops if gameaction is ga_completed, ga_victory or ga_worlddone (end=exit, set during P_Ticker by G_ExitLevel/G_SecretExitLevel, g_game.c:1002-1018): G_Ticker handles gameaction before reading the next ticcmd (g_game.c:617-652, 656-669), so the next call would run G_DoCompleted and enter the intermission (g_game.c:1133-1140), which the runner does not model; a demo that reaches its marker ends end=marker. WI_Start still aborts as a safety net.',
    'After each dumped tic the driver also stops if a player in the game is PST_REBORN (end=reborn; P_DeathThink on BT_USE, p_user.c:227-228): the next G_Ticker would run its reborn loop (g_game.c:612-614), G_DoReborn would set ga_loadlevel (g_game.c:924-931) and the level would reload (g_game.c:621-623), outside the admitted domain (decision G11).',
    'After every tic the driver calls the genuine Z_CheckHeap (z_zone.c:399-420), a read-only walk of the zone; vanilla calls it only in G_DoLoadLevel (g_game.c:488).',
    'Linker-level observers (-Wl,--wrap): P_AddThinker gives each thinker a serial (T rows), P_SetThingPosition gives the thing a link serial (the global link order is the ascending current link serial), Z_Free of a known thinker records it as freed since the previous dump (K rows, in free order) and, for a removed mobj, a copy of the mobj taken before the genuine free (RM rows); P_InitThinkers (after Z_FreeTags at level setup) drops every entry (K rows with seq -1). K rows also carry adds, the P_AddThinker count at the free; a pending mobj (R row) is followed by an RR row with its M columns. Each of these wrappers only records and then calls the genuine function.',
    'ZeroFill: the Z_Malloc wrapper zero-fills every block after the genuine allocation. Only values the original leaves undefined change: plat_t.count (EV_DoPlat, p_plats.c:173; read only in waiting after being set, p_plats.c:75, 84, 113, 120) and floormove_t.newspecial (EV_DoFloor, p_floor.c:282; set and read only for lowerAndChange and donutRaise, p_floor.c:231, 242, 422, 433, p_spec.c:1203) made the strict and address dumps differ. lowerAndChange without a neighbour at the destination height reads newspecial and texture uninitialised in vanilla (p_floor.c:242-243); the fill makes them 0 there.',
    'HU_Ticker records players[consoleplayer].message for the dump and clears it as vanilla does with the default show_messages 1; P rows report it as 0 (NULL), the 1-based index of the d_englsh.h GOTARMOR..GOTSHOTGUN2 pickup string, or -1 (any other).',
    '--selftest-extra-draw TIC calls the public P_Random () once before G_Ticker of tic TIC (test only).',
    'The zone is 6 MiB (vanilla mb_used, i_system.c:51) unless --zone-mb is given.',
]

def text(path): return Path(path).read_bytes().decode('latin-1')

def apply_once(src, a):
    n = src.count(a['original'])
    if n != 1: sys.exit(f"adapter FAILED: {a['file']}:{a['line']}: original text occurs {n} times")
    if src.split('\n')[a['line'] - 1] != a['original']:
        sys.exit(f"adapter FAILED: {a['file']}:{a['line']}: the original text is not at its declared line")
    return src.replace(a['original'], a['adapted'])

def adapter_table(adapters, builds):
    return [dict(file=a['file'], line=a['line'], original=a['original'], adapted=a['adapted'],
                 original_sha256=sha(a['original']), adapted_sha256=sha(a['adapted']), builds=builds,
                 reason=a['reason']) for a in adapters]

def adapted_sources(variant):
    applied = ABI_ADAPTERS + (SANITIZER_ADAPTERS if variant != 'strict' else [])
    out = {}
    for f in WHOLE:
        s = text(SOURCE / f)
        for a in applied:
            if a['file'] == f: s = apply_once(s, a)
        out[f] = s
    return out

def body(src, name):
    """Brace-matched definition: a return-type line, a line holding only the name, the
    parameter list and the body (the layout of every extracted r_main.c definition)."""
    ms = list(re.finditer(r'^[^\s/#][^\n;]*\n' + name + r'[ \t]*\n', src, re.M))
    if len(ms) != 1: sys.exit(f'extract FAILED: {name}: {len(ms)} definitions')
    i, depth = src.index('{', ms[0].end()), 0
    while True:
        depth += {'{': 1, '}': -1}.get(src[i], 0)
        if src[i] == '}' and depth == 0: return ms[0].start(), i + 1
        i += 1

def r_main_extract():
    src = text(SOURCE / 'r_main.c'); lines = src.split('\n')
    span = lambda a, b: (lines.index(a) + 1, lines.index(b) + 1)
    inc, glob = span('#include <stdlib.h>', '#include "r_sky.h"'), span('int\t\t\tviewangleoffset;', 'void (*spanfunc) (void);')
    chunk = lambda r: '\n'.join(lines[r[0] - 1:r[1]])
    parts = ['/* Declared extract of genuine r_main.c (m3h9c_runner_contract.json r_main_extract); verbatim below. */',
             f'/* r_main.c:{inc[0]}-{inc[1]} */', chunk(inc), f'/* r_main.c:{glob[0]}-{glob[1]} */', chunk(glob)]
    rec = dict(file='r_main.c', file_sha256=sha(src), includes=dict(lines=f'{inc[0]}-{inc[1]}', sha256=sha(chunk(inc))),
               globals=dict(lines=f'{glob[0]}-{glob[1]}', sha256=sha(chunk(glob))), functions=[])
    for n in R_MAIN_FUNCS:
        s, e = body(src, n); b = src[s:e]
        l0, l1 = src.count('\n', 0, s) + 1, src.count('\n', 0, e) + 1
        parts += [f'/* r_main.c:{l0}-{l1} */', b]
        rec['functions'].append(dict(name=n, lines=f'{l0}-{l1}', sha256=sha(b)))
    return '\n'.join(parts) + '\n', rec

def layout_probe(rdata):
    m = re.search(r'typedef struct\s*\{[^{}]*?\}\s*mappatch_t;', rdata)
    t = re.search(r'typedef struct\s*\{[^{}]*?\}\s*maptexture_t;', rdata)
    if not (m and t): sys.exit('layout probe FAILED: typedefs not found')
    return ('/* Generated from the adapted r_data.c text: the verbatim on-disk texture typedefs. */\n'
            '#include <stddef.h>\n#include "doomtype.h"\n' + m.group(0) + '\n' + t.group(0) + '\n'
            'int layout_patchcount_offset (void) { return (int)offsetof (maptexture_t, patchcount); }\n'
            'int layout_patches_offset (void) { return (int)offsetof (maptexture_t, patches); }\n'
            'int layout_mappatch_size (void) { return (int)sizeof (mappatch_t); }\n')

STUB_LINE = re.compile(r'^/\* STUB (\w+) \[(io|reads|writes|var)\] ([\w.]+:[\d\-,]+(?: \([^)]*\))?): (.*?) \*/$', re.M)
def stubs():
    found = [dict(symbol=m[0], kind='variable' if m[1] == 'var' else 'function', classification=m[1],
                  vanilla=m[2], reason=m[3]) for m in STUB_LINE.findall(text(STUBS))]
    if len({s['symbol'] for s in found}) != len(found): sys.exit('stub list FAILED: duplicate symbol')
    return found

def dump_row_kinds():
    src = text(DRIVER)
    block = src[src.index('static const char* COLUMNS ='):]; block = block[:block.index(';')]
    joined = ''.join(re.findall(r'"((?:[^"\\]|\\.)*)"', block)).replace('\\n', '\n')
    return {f[1]: f[2:] for f in (l.split(',') for l in joined.strip().split('\n'))}

def contract():
    _, extract = r_main_extract()
    st = stubs()
    adapted = {f: sha(s) for f, s in adapted_sources('strict').items()}
    san = {f: sha(s) for f, s in adapted_sources('overflow').items()}
    return dict(
        schema=SCHEMA, revision=REVISION,
        purpose='Headless whole-file build of the genuine gameplay code driven through the vanilla -playdemo path; run locally on a real IWAD and in CI on a synthetic WAD. Expected observations come from the genuine code, never from Rust output.',
        whole_files=WHOLE, excluded=dict(p_saveg_c='aborting stubs for its eight P_(Un)Archive* functions and save_p',
                                         r_main_c='declared extract only (r_main_extract)'),
        source_sha256={f: sha((SOURCE / f).read_bytes()) for f in sorted(set(WHOLE) | {'r_main.c'})},
        header_sha256={p.name: sha(p.read_bytes()) for p in sorted(SOURCE.glob('*.h'))},
        adapted_sha256=dict(strict=adapted, sanitizer=san),
        flags=VARIANTS, variant_notes=VARIANT_NOTES, file_flags=FILE_FLAGS, link_flags=LINK_FLAGS, wrapped=WRAPPED,
        abi_adapters=adapter_table(ABI_ADAPTERS, ['strict', 'overflow', 'address']),
        sanitizer_adapters=adapter_table(SANITIZER_ADAPTERS, ['overflow', 'address']),
        r_main_extract=extract,
        stubs=st, stub_counts=dict(functions=sum(s['kind'] == 'function' for s in st),
                                   variables=sum(s['kind'] == 'variable' for s in st),
                                   **{c: sum(s['classification'] == c for s in st) for c in ('io', 'reads', 'writes', 'var')}),
        runner_sha256={'tools/reference/m3h9c_driver.c': sha(DRIVER.read_bytes()),
                       'tools/reference/m3h9c_stubs.c': sha(STUBS.read_bytes())},
        driver_behaviour=DRIVER_BEHAVIOUR, dump_row_kinds=dump_row_kinds())

def render(c): return json.dumps(c, indent=1, sort_keys=False) + '\n'

def default_out(): return Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-m3h9c-build'

def build(variant, out, cc='cc'):
    out = out.resolve()
    if out == ROOT or ROOT in out.parents: sys.exit('--out must lie outside the repository')
    work = out / variant
    if work.exists(): shutil.rmtree(work)
    src, obj = work / 'src', work / 'obj'
    src.mkdir(parents=True); obj.mkdir()
    for h in SOURCE.glob('*.h'): shutil.copy2(h, src / h.name)
    sources = adapted_sources(variant)
    for f, s in sources.items(): (src / f).write_bytes(s.encode('latin-1'))
    extract, _ = r_main_extract()
    (src / 'r_main_extract.c').write_bytes(extract.encode('latin-1'))
    (src / 'layout_probe.c').write_text(layout_probe(sources['r_data.c']))
    shutil.copy2(DRIVER, src / 'm3h9c_driver.c'); shutil.copy2(STUBS, src / 'm3h9c_stubs.c')
    units = WHOLE + ['r_main_extract.c', 'layout_probe.c', 'm3h9c_stubs.c', 'm3h9c_driver.c']
    flags, warnings = VARIANTS[variant], {}
    for f in units:
        p = subprocess.run([cc] + flags + FILE_FLAGS.get(f, []) + ['-I', str(src), '-c', str(src / f), '-o',
                           str(obj / (f[:-2] + '.o'))], capture_output=True, text=True)
        if p.returncode: sys.stderr.write(p.stderr); sys.exit(f'compile FAILED: {f}')
        if 'warning:' in p.stderr:
            warnings[f] = p.stderr.count('warning:'); (work / f'{f}.warnings.txt').write_text(p.stderr)
    nm = subprocess.run(['nm', '--defined-only', '-g', str(obj / 'm3h9c_stubs.o')], capture_output=True, text=True, check=True)
    defined = {l.split()[-1] for l in nm.stdout.splitlines() if l.strip() and '__odr_asan' not in l} - {'stub_quit_flag', 'stub_quit_jmp',
              'stub_quit_armed', 'stub_zone_mb', 'stub_gettime_counter', 'stub_hu_message', 'stub_hu_ran'}
    declared = {s['symbol'] for s in stubs()}
    if defined != declared: sys.exit(f'stub list FAILED: defined-not-declared {sorted(defined - declared)}, '
                                     f'declared-not-defined {sorted(declared - defined)}')
    binary = work / 'crunner'
    p = subprocess.run([cc] + flags + LINK_FLAGS + [str(obj / (f[:-2] + '.o')) for f in units] + ['-o', str(binary)],
                       capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit('link FAILED')
    man = dict(contract(), variant=variant, compiler=subprocess.run([cc, '--version'], capture_output=True,
               text=True).stdout.split('\n')[0], binary=str(binary), binary_sha256=sha(binary.read_bytes()),
               warnings=warnings)
    (out / f'manifest_{variant}.json').write_text(render(man))
    print(f'{variant}: built {binary}; {sum(warnings.values())} warnings in {len(warnings)} files')
    return binary

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--variant', choices=list(VARIANTS) + ['all'])
    ap.add_argument('--out', type=Path, default=default_out())
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--contract', type=Path, help='write the host-independent contract to this path')
    ap.add_argument('--check', action='store_true', help='fail if the committed contract differs')
    a = ap.parse_args()
    if a.check:
        if CONTRACT.read_text() != render(contract()): sys.exit(f'{CONTRACT.relative_to(ROOT)} differs from the regenerated contract')
        c = json.loads(CONTRACT.read_text())
        print(f"m3h9c runner contract matches: {len(c['whole_files'])} whole files, {len(c['abi_adapters'])} ABI + "
              f"{len(c['sanitizer_adapters'])} sanitizer adapters, {len(c['r_main_extract']['functions'])} extracted bodies, "
              f"{c['stub_counts']['functions']} function + {c['stub_counts']['variables']} variable stubs")
    if a.contract:
        a.contract.write_text(render(contract())); print(f'wrote {a.contract}')
    if a.variant:
        for v in (VARIANTS if a.variant == 'all' else [a.variant]): build(v, a.out, a.cc)
    if not (a.check or a.contract or a.variant): ap.error('nothing to do')

if __name__ == '__main__':
    main()
