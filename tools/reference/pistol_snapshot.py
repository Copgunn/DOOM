"""Independent pistol overlays on the frozen original-player field assembler.

Input is compiled-C observations and an explicitly specified diagnostic schema.
The legacy assemblers remain immutable. No Rust observations are accepted.
"""
import struct
import player_execution_snapshot as execution

base = execution.base
n = base.n
physical = execution.physical
old_events = physical.events
old_frame = physical.frame


def events(rows):
    out, chunk = [], []
    aiming = False
    shooting = False
    for row in rows:
        tag, *unused = row
        if tag in {'WP', 'DM'}:
            continue
        if tag == 'X':
            shooting = row[3] == 0
        if tag == 'D' and row[3] == 0 and not shooting:
            # Source aim queries are separately compared read-only evidence.
            # They are not MonsterEvent::Aim in the player weapon contract.
            aiming = True
        if aiming:
            if tag == 'J' and row[3] == 5:
                aiming = False
            continue
        if tag != 'W':
            chunk.append(row)
            continue
        out += old_events(chunk)
        chunk = []
        code, *v = row[3:]
        payload = bytes([14, code])
        if code in (0, 1):
            payload += bytes(v[:2]) + n('H', v[2])
        elif code == 2:
            payload += bytes(v[:2]) + struct.pack('<ii', *v[2:])
        elif code == 3:
            payload += n('B', v[0]) + n('i', v[1])
        elif code == 4:
            payload += bytes(v[:2]) + n('i', v[2])
        elif code == 5:
            payload += base.ident(v[0]) + struct.pack('<Iii', *v[1:])
        elif code == 7:
            payload += n('B', v[0]) + base.ident(v[1])
        elif code == 8:
            payload += bytes(v)
        else:
            raise ValueError(row)
        out.append(payload)
    assert not aiming
    return out + old_events(chunk)


def weapon_fields(row):
    ready, pending, owned, clips, maximum, latch, refire, bob, light, *sprites = row
    assert (ready, pending, owned, maximum) == (1, 10, 3, 200)
    assert 0 <= clips <= 200 and latch in (0, 1)
    assert 0 <= refire <= 200 and 0 <= bob <= 0x100000 and light in (0, 1)
    prefix = 'original_player.0.pistol'
    fields = [(prefix + '.present', b'\1'), (prefix + '.bullets', n('H', clips)),
              (prefix + '.attack_down', n('B', latch)),
              (prefix + '.refire', n('i', refire)),
              (prefix + '.bob', n('i', bob)), (prefix + '.extra_light', n('i', light))]
    assert len(sprites) == 8
    for i in range(2):
        state, tics, x, y = sprites[4*i:4*i+4]
        for key, code, value in [('state', 'H', 65535 if state < 0 else state),
                                 ('tics', 'i', tics), ('x', 'i', x), ('y', 'i', y)]:
            fields.append((f'{prefix}.psprite.{i}.{key}', n(code, value)))
    return fields


def build(cases, rows, maps, prefix, full):
    selected = iter((r[1], r[2]) for r in rows if r[0] == 'P')
    observations = {(r[1], r[2]): r[3:] for r in rows if r[0] == 'WP'}

    def frame(fields):
        case, operation = next(selected)
        assert next(value for path, value in fields if path == 'tic') == n('Q', operation)
        at = next(i for i, (path, _) in enumerate(fields) if path == 'events.last.count')
        revised = fields[:at] + weapon_fields(observations[case, operation]) + fields[at:]
        data = bytearray(b'DSIM' + struct.pack('<IQ', 23, len(revised)))
        for path, value in revised:
            path = path.encode()
            data += n('I', len(path)) + path + n('Q', len(value)) + value
        return bytes(data)

    physical.events, physical.frame = events, frame
    try:
        result = execution.build(cases, rows, maps, 7,
                                 b'PLAYER-DEMON-1-MEDIUM-GROUNDED-PISTOL-v1', prefix, full)
        assert next(selected, None) is None
        outputs, inventory = result
        index = 'tests/reference/' + prefix + '_snapshots.csv'
        lines = outputs[index].decode().splitlines()
        for i in range(1, len(lines)):
            values = lines[i].split(',')
            values[6] = str(int(values[6]) + 14)
            lines[i] = ','.join(values)
        outputs[index] = ('\n'.join(lines) + '\n').encode()
        return outputs, inventory
    finally:
        physical.events, physical.frame = old_events, old_frame
