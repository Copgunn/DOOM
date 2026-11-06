#!/usr/bin/env python3
"""Build the M4a headless genuine-engine runner: the M3h9c build with the M4a driver fork.

tools/reference/m4a_driver.c is the M4a fork of the merged m3h9c_driver.c (which stays
as merged: frozen wire-22 evidence). This script reuses m3h9c_build.py whole (the same
genuine files, ABI and sanitizer adapters, r_main.c extract, stubs, flags, variants and
the opt-in strict32) through an m3h9c_build.Runner that changes only:

  - the driver: tools/reference/m4a_driver.c;
  - the wrapped symbols: m3h9c_build.WRAPPED + ['P_RemoveThinker'];
  - the driver behaviour: the HU_Ticker entry names the message codes 38..40, and two
    entries are appended (H.secretexit; the R rows' fn and sector);
  - the schema DOOM-M4A-RUNNER-v1 and the contract tests/reference/m4a_runner_contract.json;
  - the default build directory $HOME/scratch/doom-m4a-build.

    m4a_build.py --variant strict|overflow|address|all|strict32 [--out DIR] [--cc CC]
    m4a_build.py --probe-m32 [--cc CC]
    m4a_build.py --contract tests/reference/m4a_runner_contract.json
    m4a_build.py --check

The options mean what they mean for m3h9c_build.py; --check regenerates the M4a contract
in memory and fails if the committed file differs.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import m3h9c_build as base  # noqa: E402
from m3h9c_build import build as _build, contract as _contract, ROOT  # noqa: E402,F401

HU_PREFIX = 'HU_Ticker records players[consoleplayer].message'
M4A = base.Runner(
    'm4a', base.HERE / 'm4a_driver.c', ROOT / 'tests/reference/m4a_runner_contract.json', 'DOOM-M4A-RUNNER-v1',
    wrapped=['P_RemoveThinker'],
    replace=[(HU_PREFIX,
              'HU_Ticker records players[consoleplayer].message for the dump and clears it as vanilla does with the default show_messages 1; P rows report it as 0 (NULL), the 1-based index of the d_englsh.h GOTARMOR..GOTSHOTGUN2 pickup string (1..37), 38..40 for the locked-door strings PD_BLUEK, PD_YELLOWK, PD_REDK (d_englsh.h:128-130, assigned at p_doors.c:378, 392, 405; added for M4a), or -1 (any other).')],
    behaviour=[
        'H rows end with secretexit (added for M4a): g_game.c\'s boolean secretexit (g_game.c:999), set by G_SecretExitLevel (g_game.c:1009-1018) and cleared by G_ExitLevel (g_game.c:1002-1006). New columns are appended at the end of their rows so every earlier column keeps its position.',
        'R rows carry fn and sector (added for M4a): a -Wl,--wrap=P_RemoveThinker observer records, before the genuine P_RemoveThinker (p_tick.c:80-84) overwrites the function pointer, the kind of a removed non-mobj thinker (1 T_VerticalDoor, 2 T_PlatRaise, 3 T_MoveFloor, 4 T_MoveCeiling, 0 other) and its sector index, so a mover created and removed within one tic keeps its sector for the encoder; -1 when unknown.',
    ],
    out_name='doom-m4a-build')

OPTIONAL_VARIANTS = base.OPTIONAL_VARIANTS
VARIANTS = base.VARIANTS
probe_m32 = base.probe_m32


def contract(): return _contract(M4A)


def build(variant, out, cc='cc'): return _build(variant, out, cc, M4A)


def default_out(): return base.default_out(M4A)


if __name__ == '__main__':
    base.main(M4A, __doc__)
