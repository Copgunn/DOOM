#!/usr/bin/env python3
"""Independent DSIM44 projection (profile `episode1v6`): wire 28's canonical schema from the genuine engine's dumps.

DSIM44 is the canonical schema of wire 28 (`PlayerMonstersEpisode1V6`: wire 27 plus skills 0 and 1, ruling U12 (iii),
and the spawn rule S'' with the monster escape witness, section 1.17 option 1 of the M6a note): DSIM43's layout field
for field. Neither rule adds state or a random draw (section 1.12), so this module is the DSIM43 projection of
level_snapshot_wire27.py, imported as a library and never edited, with exactly three values changed and nothing else:

- the version word is 44 (VERSION);
- `profile` is 28 (POLICY);
- `context.schema` is `PLAYER-MONSTERS-EPISODE1-6-v1` (CONTEXT).

Skills. `actors.options[0]` is H.gameskill of every dump (level_snapshot.py `_step_exact`), so skills 0 and 1 project
like 2 and 3. The DSIM40 projector's constructor (level_snapshot_m4b.py) admits skills 2 and 3 only (decision I4) and
keeps the demo header's skill as `header_skill`, which every dump's H.gameskill must equal. For skill 0 or 1 this
projector builds that chain with skill 2 and then sets `header_skill` to the real skill, so the comparison with
H.gameskill stays in force; nothing else in the chain reads the constructor's skill. Nightmare (4) stays refused.

`compare43(f44, f43)` asserts that two projections of the same dump differ in `profile` and `context.schema` only
(the version word is not a field; it differs in the framing). Library only: `Projector`, `VERSION`, `compare43`.
"""
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L           # noqa: E402
import level_snapshot_wire27 as W    # noqa: E402  the DSIM43 projection (read-only)

VERSION = 44
POLICY = 28
CONTEXT = b'PLAYER-MONSTERS-EPISODE1-6-v1'
SKILLS = (0, 1, 2, 3)               # sk_baby .. sk_hard (doomdef.h skill_t); nightmare refused
EPISODE1V6 = L.Profile('episode1v6', VERSION, POLICY, CONTEXT, W.EPISODE1V5.max_slots, W.EPISODE1V5.messages, True)
assert len(CONTEXT) == len(W.CONTEXT)


class Projector(W.Projector):
    """The DSIM43 projector with the DSIM44 profile (policy 28 and the wire-28 context), admitting skills 0 to 3."""
    def __init__(self, mapdata, skill, map_number, sounds, start):
        if skill not in SKILLS: raise ValueError(f'skill {skill} outside 0-3 (ruling U12; nightmare is M6b)')
        super().__init__(mapdata, max(skill, 2), map_number, sounds, start)
        self.header_skill = skill
        self.profile = EPISODE1V6


def compare43(f44, f43):
    """Raise AssertionError unless the DSIM44 fields are the DSIM43 fields with profile 28 and the wire-28 context."""
    names = ('profile', 'context.schema')
    assert [p for p, _ in f44] == [p for p, _ in f43]
    assert [(p, v) for p, v in f44 if p not in names] == [(p, v) for p, v in f43 if p not in names]
    d44, d43 = dict(f44), dict(f43)
    assert (d43['profile'], d44['profile']) == (L.n('I', W.POLICY), L.n('I', POLICY))
    assert (d43['context.schema'], d44['context.schema']) == (W.CONTEXT, CONTEXT)
