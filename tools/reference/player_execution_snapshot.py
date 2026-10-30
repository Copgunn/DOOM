#!/usr/bin/env python3
"""Independent one-player DSIM22 projection with observed command-stage phases.

Reuses frozen physical/player field primitives. Historical fixed-phase encoders
remain immutable; PG markers establish the new scheduler/event chronology.
"""
import collections
import shutil
import struct
import tempfile
from pathlib import Path
import player_snapshot as physical

base = physical.base
n = base.n


def events(rows):
    out, chunk = [], []
    for row in rows:
        if row[0] in {'PM', 'UC'}:
            continue
        if row[0] != 'PG':
            chunk.append(row)
            continue
        out += physical.events(chunk)
        chunk = []
        code, *values = row[3:]
        if code == 0:
            out.append(b'\0\0')  # Existing Players phase.
        elif code == 1:
            out.append(bytes([4, 0]) + base.ident(values[0]))
        elif code == 2:
            out.append(b'\0\1')  # Existing Thinkers phase.
        else:
            raise ValueError(row)
    return out + physical.events(chunk)


def nonstage_events(rows):
    """Only used by the frozen non-event field projection's legacy scaffolding."""
    return physical.events([r for r in rows if r[0] not in {'PG', 'PM', 'UC'}])


def build(cases, rows, map_files, policy, context, prefix, full):
    outputs = {}
    index = ['case,operation,tic,fnv1a64,bytes_fixture,bytes,fields,event_fields']
    stream = bytearray(b'PBE1')
    event_fields = 0
    event_frames = 0
    with tempfile.TemporaryDirectory(prefix='doom-player-execution-encoding-') as directory:
        temp = Path(directory)
        (temp/'tests/reference').mkdir(parents=True)
        for i, path in enumerate(map_files):
            shutil.copyfile(path, temp/f'tests/reference/m3f3_map_{i}.bin')
        base.ROOT = temp
        base.map_fields.cache_clear()
        base.metadata = lambda _c: []
        base.events = nonstage_events
        for which, values in enumerate(cases):
            c = {k: int(v) for k, v in values.items() if k != 'label'}
            c.update(mode=4, skill=2, no_monsters=0)
            selected = [r for r in rows if r[1] == which]
            grouped = collections.defaultdict(list)
            for row in selected:
                grouped[row[2]].append(row)
            captures = {r[2] for r in selected if r[0] == 'P'}
            occupied, generations, births, queued = {}, [], {}, []
            for op in range(c['tics'] + 1):
                current = grouped[op]
                for row in current:
                    if row[0] != 'E':
                        continue
                    code, birth, _ = row[3:]
                    if code == 1:
                        slot = next((s for s in range(len(generations)) if s not in occupied), len(generations))
                        if slot == len(generations):
                            generations.append(1)
                        occupied[slot] = birth
                        births[birth] = (slot, generations[slot])
                    elif code == 3:
                        slot = births[birth][0]
                        del occupied[slot]
                        generations[slot] += 1
                base.BIRTH_ID = births.copy()
                if op:
                    # The C driver places PG around the source gate before the
                    # first thinker, after any controlled between-tic operation.
                    assert [r[3] for r in current if r[0] == 'PG' and r[3] != 1] == [0, 2]
                    last = queued + events(current) + [b'\0\2', b'\0\3']
                    queued = []
                else:
                    queued += events(current)
                    last = []
                stream += struct.pack('<4I', which, op, len(last), len(queued))
                observed = []
                for family, values in [('events.last', last), ('events.pending', queued)]:
                    observed.append((family + '.count', n('Q', len(values))))
                    for i, value in enumerate(values):
                        stream += n('I', len(value)) + value
                        observed.append((f'{family}.{i}', value))
                event_fields += len(observed) - 2
                event_frames += 1
                if op not in captures:
                    continue
                # Reuse source actor/map/list fields, not the old encoder's
                # adjacent Players/Thinkers phase assumption or zero commands.
                _, fields = base.snapshot(c, selected, op, grouped, diagnostics=True)
                assert base.BIRTH_ID == births
                fields[2:3] = [('profile', n('I', policy)), ('registry.schema', b'DOOM-INFO-1.10-v1'),
                               ('context.schema', context)]
                fields = [f for f in fields if not f[0].startswith('perception.player.')]
                command = next(r[3:] for r in current if r[0] == 'UC')
                commands = dict(zip(['player.0.forward', 'player.0.side', 'player.0.angle_turn', 'player.0.buttons'],
                                    [n('b', command[0]), n('b', command[1]), n('h', command[2]), n('B', command[3])]))
                fields = [(p, commands.get(p, v)) for p, v in fields]
                mo, health, state, attacker, count, frags = next(r[3:] for r in current if r[0] == 'P')
                revised = []
                for path, value in fields:
                    if path.startswith('actor.') and path.endswith('.momz'):
                        stem = path.rsplit('.', 1)[0]
                        birth = occupied[int(stem.split('.')[1])]
                        association = next(r[4] for r in current if r[0] == 'AP' and r[3] == birth)
                        assert association == (0 if birth == mo else -1)
                        revised.append((stem + '.player', n('B', 255 if association < 0 else association)))
                    revised.append((path, value))
                fields = revised
                at = next(i for i, (p, _) in enumerate(fields) if p == 'events.last.count')
                end = next((i for i in range(at, len(fields)) if not fields[i][0].startswith('events.')), len(fields))
                fields[at:end] = [('original_player.0.mo', base.opt(mo)), ('original_player.0.health', n('i', health)),
                    ('original_player.0.state', n('B', state)), ('original_player.0.attacker', base.opt(attacker)),
                    ('original_player.0.damage_count', n('i', count)), ('original_player.0.frags', n('i', frags)), *observed]
                data = physical.frame(fields)
                name = f'{prefix}_state_{which}_{op}.bin' if (which, op) in full else '-'
                if name != '-':
                    outputs['tests/reference/' + name] = data
                index.append(f'{which},{op},{op},{physical.player.fnv(data):016x},{name},{len(data)},{len(fields)},{len(observed)-2}')
    assert sum('_state_' in p for p in outputs) == len(full)
    outputs[f'tests/reference/{prefix}_snapshots.csv'] = ('\n'.join(index)+'\n').encode()
    outputs[f'tests/reference/{prefix}_events.bin'] = bytes(stream)
    return outputs, dict(captures=len(index)-1, full_byte_fixtures=len(full), event_frames=event_frames,
                         ordered_event_fields=event_fields)
