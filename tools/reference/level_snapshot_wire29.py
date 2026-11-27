#!/usr/bin/env python3
"""Independent DSIM45 projection (profile `episode1v7`): wire 29's canonical schema from the genuine engine's dumps.

DSIM45 is the canonical schema of wire 29 (`PlayerMonstersEpisode1V7`: wire 28 plus Nightmare, the -respawn and -fast
parameters and the pause's simulation part; docs/M6B_REFERENCE_INVESTIGATION.md, rulings N1, N2, N4, N6, N10). It is
the DSIM44 layout of level_snapshot_wire28.py (the DSIM43 chain of level_snapshot_wire27.py, imported read-only and
never edited) with these differences and nothing else:

- the version word is 45, `profile` is 29 and `context.schema` is `PLAYER-MONSTERS-EPISODE1-7-v1` (same length);
- `actors.options[0]` may also be 4 (`sk_nightmare`, doomdef.h:153): H.gameskill of every dump, as before;
- `actors.parameters`, 2 bytes, right after `actors.options`: `respawnparm`, `fastparm` (0 or 1 each), the demo
  header's bytes 5 and 6 (G_DoPlayDemo, g_game.c:1600-1601; ruling N1). `respawnmonsters` (g_game.c:1416-1419) and
  the fast table (g_game.c:1421-1428; ruling N4 (a): fast iff Nightmare or fastparm, no saved table) follow from
  them and the skill, so neither is a field;
- `level.gametic_lead`, u64, right after `level.start.gametic`: how far `gametic` is ahead of the level's own tic
  count (ruling N6 (a)), H.gametic - H.leveltime of each dump (the runner dumps after gametic++, and P_Ticker counts
  leveltime once per level tic, level_snapshot_m6a.py). It equals `level.start.gametic` until the level is paused
  and grows by one per paused tic (G_Ticker runs while P_Ticker returns at once, p_tick.c:135-136; gametic++ in the
  loop, d_main.c:384). The DSIM42 projector requires H.gametic - H.leveltime to be constant over the level; it is
  given each dump with gametic rewritten to the first dump's lead plus leveltime (in memory, the dump file is not
  changed), so its start group keeps the load's gametic. The chain reads a capture's `events.sounds` rows at the
  dump's leveltime - 1 (level_snapshot_m5c.sound_fields), which is its session tic only until a pause; each dump's
  sound stream is therefore read ahead by the paused tics so far (lead - the load's lead), so the rows are those of
  the session tic the capture follows. (Corrected 2026-11-27, M6b phase 2 step 6: the first version read them at the
  leveltime, which moved every sound record after a pause by the paused count.)

Two projection rules new with the respawn, each given to the DSIM39 projector in memory and put back in its output:
a respawned monster carries its corpse's spawnpoint (`Projector.respawned`), and a `target` or `player->attacker`
left pointing at the freed corpse is the stale handle of the mobj it last named (`Projector.stale`).

The skill. The DSIM40 projector admits skills 2 and 3 (decision I4) and keeps `header_skill`, which every dump's
H.gameskill must equal; as in level_snapshot_wire28.py the chain is built at skill 2 or 3 and `header_skill` is then
set to the real skill. Nightmare's THING bit is 4, hard's (p_mobj.c:745-748), so the chain is built at 3 for it.
The caller gives the skill G_InitNew plays: a header byte above 4 is played as 4 (g_game.c:1379-1380).

`compare44(f45, f44)` asserts that the projections of one dump differ exactly as listed (the version word is not a
field; it differs in the framing). Library only: `Projector`, `VERSION`, `compare44`, `PARAMETERS`, `LEAD`, `Ahead`.
"""
import struct, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L           # noqa: E402
import level_snapshot_wire27 as W27  # noqa: E402  the DSIM43 projection (read-only)
import level_snapshot_wire28 as W28  # noqa: E402  the DSIM44 projection (read-only)

VERSION = 45
POLICY = 29
CONTEXT = b'PLAYER-MONSTERS-EPISODE1-7-v1'
SKILLS = (0, 1, 2, 3, 4)             # sk_baby .. sk_nightmare (doomdef.h:147-154)
OPTIONS, PARAMETERS = 'actors.options', 'actors.parameters'
START_GAMETIC, LEAD = 'level.start.gametic', 'level.gametic_lead'
EPISODE1V7 = L.Profile('episode1v7', VERSION, POLICY, CONTEXT, W28.EPISODE1V6.max_slots, W28.EPISODE1V6.messages, True)
assert len(CONTEXT) == len(W28.CONTEXT)


class Ahead:
    """A sound stream read `ahead` session tics later: rows(t) are the stream's rows(t + ahead)."""
    def __init__(self, base, ahead):
        self.base, self.ahead, self.last_tic = base, ahead, base.last_tic - ahead

    def rows(self, tic):
        return self.base.rows(tic + self.ahead)


class Projector(W27.Projector):
    """The DSIM43 projector with the DSIM45 profile, skills 0 to 4, the two parameters and the gametic lead."""
    def __init__(self, mapdata, skill, map_number, sounds, start, respawn, fast):
        if skill not in SKILLS: raise ValueError(f'skill {skill} outside 0-4')
        if respawn not in (0, 1) or fast not in (0, 1): raise ValueError('respawnparm and fastparm are 0 or 1')
        super().__init__(mapdata, 3 if skill == 4 else max(skill, 2), map_number, sounds, start)
        self.header_skill = skill
        self.profile = EPISODE1V7
        self.parameters = bytes([respawn, fast])
        self.first_lead = None
        self.unpaused_sounds = sounds
        self.last = {}          # what each reference last named (stale)

    def step(self, dump, following=None):
        h = dump.h
        lead = h['gametic'] - h['leveltime']
        if self.first_lead is None: self.first_lead = lead
        if lead < self.first_lead:
            raise L.ProjectionError('DumpDeficiency', dump.tic, f'gametic lead {lead} below the load\'s {self.first_lead}')
        h['gametic'] = self.first_lead + h['leveltime']
        self.sounds = Ahead(self.unpaused_sounds, lead - self.first_lead)
        respawned = self.respawned(dump)
        kept = {sv: {k: row[k] for k in SPAWNPOINT} for sv, row in respawned}
        for _sv, row in respawned: row.update(dict.fromkeys(SPAWNPOINT, 0))
        # A paused tic runs no thinker pass (p_tick.c:135-136): a mobj P_RemoveMobj marked before it stays listed,
        # pending, until the first unpaused tic's P_RunThinkers frees it (p_tick.c:107-112), so its RR row is
        # checked against the RM row of the dump after the last paused one.
        if self.prev is not None and dump.tic == self.prev.tic:
            for e in self.entries: e.pending = False
        if following is not None and following.tic == dump.tic: following = None
        p0 = dump.players[0]
        attacker = p0['attacker']
        if attacker == -2: p0['attacker'] = -1
        try: fields, report = super().step(dump, following)
        finally:
            for sv, row in respawned: row.update(kept[sv])
            p0['attacker'] = attacker
        patch = {f'actor.{self.actors.handles[sv][0]}.spawnpoint': spawnpoint(kept[sv]) for sv, _ in respawned}
        if attacker == -2: patch['original_player.0.attacker'] = self.stale(dump.tic, 'attacker')
        for e in self.entries:
            t = e.row.get('target', -1) if e.cls == 'actor' else -1
            if t != -2: self.last[e.serial] = self.entries[t].serial if t >= 0 else None
        if attacker != -2: self.last['attacker'] = self.entries[attacker].serial if attacker >= 0 else None
        out = []
        for path, value in fields:
            out.append((path, patch.pop(path, value)))
            if path == OPTIONS: out.append((PARAMETERS, self.parameters))
            if path == START_GAMETIC: out.append((LEAD, L.n('Q', lead)))
        assert len(out) == len(fields) + 2 and not patch
        return out, report

    def stale(self, tic, owner):
        """A reference to a freed mobj (the dump's -2) as the handle of the mobj it last named: the simulation keeps
        the stale handle. Before respawning, only missiles, puffs, blood, fogs and barrels were freed, which no
        `target` or `attacker` names; P_NightmareRespawn frees a corpse (p_mobj.c:408) that the player's mobj `target`
        and `player->attacker` may still name (P_DamageMobj, p_inter.c:847, 904-911). `owner` is the referring mobj's
        serial or 'attacker'."""
        last = self.last.get(owner)
        if last is None or any(x.serial == last for x in self.entries):
            raise L.ProjectionError('DumpDeficiency', tic, 'a dangling reference to no freed mobj')
        return b'\1' + L.ident(self.actors.handles[last])

    def _target(self, tic, e):
        return self.stale(tic, e.serial) if e.row['target'] == -2 else super()._target(tic, e)

    def respawned(self, dump):
        """[(serial, row)] of the mobjs new in this dump that carry a spawnpoint: the monsters P_NightmareRespawn
        spawns (p_mobj.c:392-399, `mo->spawnpoint = mobj->spawnpoint`), each of the type and spawnpoint of a corpse
        it removes in the same call (408: pending in this dump), and only while respawnmonsters is set (skill 4 or
        respawnparm, g_game.c:1416-1419). Every other new mobj has none (P_SpawnMobj zeroes the record), which the
        DSIM39 projector requires; it is given these rows with the spawnpoint zeroed (in memory) and the field is
        put back in the output."""
        if self.prev is None: return []
        known = {e.serial for e in self.entries}
        corpses = [dump.pending[i] for i, (kind, _) in enumerate(dump.thinkers) if kind == 'R' and i in dump.pending]
        out = []
        for i, (kind, row) in enumerate(dump.thinkers):
            sv = dump.serials[i][0]
            if kind != 'M' or not row['sp_type'] or sv in known: continue
            if not (self.header_skill == 4 or self.parameters[0]) or not any(
                    c['type'] == row['type'] and all(c[k] == row[k] for k in SPAWNPOINT) for c in corpses):
                raise L.ProjectionError('DumpDeficiency', dump.tic, 'a new mobj with a spawnpoint that is not a respawn')
            out.append((sv, row))
        return out


SPAWNPOINT = ('sp_x', 'sp_y', 'sp_angle', 'sp_type', 'sp_options')


def spawnpoint(r):
    """The DSIM39 `spawnpoint` field of a mobj row (level_snapshot.py `_fields`)."""
    return b'\1' + struct.pack('<hhhhH', r['sp_x'], r['sp_y'], r['sp_angle'], r['sp_type'], r['sp_options'] & 0xffff)


def compare44(f45, f44):
    """Raise AssertionError unless the DSIM45 fields are the DSIM44 fields with profile 29, the wire-29 context and
    the two new fields in their places."""
    renamed = ('profile', 'context.schema')
    rest = [(p, v) for p, v in f45 if p not in (PARAMETERS, LEAD)]
    assert [p for p, _ in rest] == [p for p, _ in f44]
    assert [(p, v) for p, v in rest if p not in renamed] == [(p, v) for p, v in f44 if p not in renamed]
    names = [p for p, _ in f45]
    assert names[names.index(OPTIONS) + 1] == PARAMETERS and names[names.index(START_GAMETIC) + 1] == LEAD
    d45, d44 = dict(f45), dict(f44)
    assert (d44['profile'], d45['profile']) == (L.n('I', W28.POLICY), L.n('I', POLICY))
    assert (d44['context.schema'], d45['context.schema']) == (W28.CONTEXT, CONTEXT)
