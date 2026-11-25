#!/usr/bin/env python3
"""Independent DSIM43 projection (profile `episode1v5`): wire 27's canonical schema from the genuine engine's dumps.

DSIM43 is the canonical schema of wire 27 (`PlayerMonstersEpisode1V5`, owner's ruling 2 of 2026-11-24): DSIM42's
layout field for field. This module is the DSIM42 projection of level_snapshot_m6a.py, imported as a library and never
edited, with exactly three values changed and nothing else:

- the version word is 43 (VERSION);
- `profile` is 27 (POLICY);
- `context.schema` is `PLAYER-MONSTERS-EPISODE1-5-v1` (CONTEXT).

`compare42(f43, f42)` asserts that two projections of the same dump differ in `profile` and `context.schema` only
(the version word is not a field; it differs in the framing). tools/reference/wire27_goldens.py runs it on every
capture of every item. Library only: `Projector`, `VERSION`, `compare42`.
"""
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L          # noqa: E402
import level_snapshot_m6a as S      # noqa: E402  the DSIM42 projection (read-only)

VERSION = 43
POLICY = 27
CONTEXT = b'PLAYER-MONSTERS-EPISODE1-5-v1'
EPISODE1V5 = L.Profile('episode1v5', VERSION, POLICY, CONTEXT, S.EPISODE1V4.max_slots, S.EPISODE1V4.messages, True)
assert len(CONTEXT) == len(S.CONTEXT)


class Projector(S.Projector):
    """The DSIM42 projector with the DSIM43 profile (policy 27 and the wire-27 context)."""
    def __init__(self, mapdata, skill, map_number, sounds, start):
        super().__init__(mapdata, skill, map_number, sounds, start)
        self.profile = EPISODE1V5


def compare42(f43, f42):
    """Raise AssertionError unless the DSIM43 fields are the DSIM42 fields with profile 27 and the wire-27 context."""
    names = ('profile', 'context.schema')
    assert [p for p, _ in f43] == [p for p, _ in f42]
    assert [(p, v) for p, v in f43 if p not in names] == [(p, v) for p, v in f42 if p not in names]
    d43, d42 = dict(f43), dict(f42)
    assert (d42['profile'], d43['profile']) == (L.n('I', S.POLICY), L.n('I', POLICY))
    assert (d42['context.schema'], d43['context.schema']) == (S.CONTEXT, CONTEXT)
