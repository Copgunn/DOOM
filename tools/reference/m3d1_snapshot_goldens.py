#!/usr/bin/env python3
"""Independent DSIM-v5: original C captures and explicit public fixture inputs.

Encodes all 48 setups, nine direct-damage and three hitscan API results, and three settled
multi-tic death results. Does not read Rust output or modify v1-v4 encoders.
Transient shot events are tested separately against C intercept/lifecycle rows.
"""
import argparse
import csv
import hashlib
import json
import struct
from pathlib import Path
import m3c_snapshot_goldens as v4
ROOT = Path(__file__).resolve().parents[2]
U = 65536
n, ident, opt = v4.n, v4.ident, v4.opt
DIRECT = [0, 1, 2, 3, 4, 5, 6, 7, 10]
SHOTS = [13, 14, 38]
FINALS = {5: 76, 7: 76, 38: 81}
def combat(tag, payload=b''):
    return bytes([13, tag]) + payload
def map_fields(which):
    data = (ROOT / f'tests/reference/m3d1_map_{which}.bin').read_bytes()
    count, offset = struct.unpack_from('<ii', data, 4)
    lumps = []
    for i in range(count):
        start, size, _ = struct.unpack_from('<ii8s', data, offset + i * 16)
        lumps.append(data[start:start + size])
    names = ['things', 'linedefs', 'sidedefs', 'vertices', 'segs', 'subsectors', 'nodes', 'sectors', 'reject']
    fields = [('map.' + name, lumps[i + 1]) for i, name in enumerate(names)]
    words = struct.unpack('<' + 'H' * (len(lumps[10]) // 2), lumps[10])
    ranges = b''
    for start in words[4:4 + words[2] * words[3]]:
        end = start
        while words[end] != 65535:
            end += 1
        assert set(words[start:end]) == set(range(len(lumps[2]) // 14))
        ranges += struct.pack('<II', start, end)
    return fields + [('map.blockmap.words', lumps[10]), ('map.blockmap.cell_ranges', ranges)], lumps[8]
def metadata(c):
    states, types, fields = v4.metadata(0)
    properties = types[0][1].copy()
    for i, key in [(0, None), (2, 'health'), (8, 'pain'), (18, 'mass'), (21, 'flags'), (5, None), (17, 'height')]:
        properties[i] = c[key] if key else -1 if i == 0 else 8
    name = b'FIXTURE_PAWN'
    fields[-1] = ('registry.type.136', n('B', len(name)) + name + struct.pack('<23i', *properties))
    return states, properties, fields
def life_events(rows, states):
    return [e for r in rows if r[0] == 'E' and r[3] != 8 for e in v4.event(r[3:], states)]
def damage_events(c, rows, a, states, properties):
    target = ident(1)
    events = [combat(2, target + opt(0 if c['inflictor'] else -1) + opt(0 if c['source'] else -1) + n('i', c['damage']))]
    if c['flags'] & 4 == 0:
        return events + [combat(12, target)]
    if c['inflictor']:
        # These selected cases have due-east inflictors, no prior momentum and
        # no forward-fall branch. Momentum is taken directly from C capture.
        thrust = c['damage'] * 8192 * 100 // c['mass']
        events.append(combat(4, target + n('I', 0) + struct.pack('<iii', thrust, a[5], a[6])))
    events.append(combat(5, target + struct.pack('<ii', c['health'], a[8])))
    random = [r[5] for r in rows if r[0] == 'E' and r[3] == 8]
    assert len(random) == 1
    lethal = a[8] <= 0
    if lethal:
        extreme = a[8] < -c['health']
        state = properties[13 if extreme else 12]
        events.append(combat(7, target + n('H', state) + n('B', extreme)))
        events += life_events(rows, states)
        events += [combat(3, bytes([2, random[0]])), combat(8, target + n('i', a[10]))]
    else:
        events.append(combat(3, bytes([1, random[0]])))
        if random[0] < c['pain']:
            events.append(combat(6, target + n('H', properties[7])))
            events += life_events(rows, states)
        events.append(combat(9, target))
        if c['source']:
            events += [bytes([12, 4]) + target + opt(0), combat(10, target + ident(0))]
            if random[0] >= c['pain']:
                events += life_events(rows, states)
    return events

def trace_object(line, index):
    return b'\0' + n('H', index) if line else b'\1' + ident(index)

def shot_events(c, rows, actors, states, properties):
    events = [combat(0, ident(0) + struct.pack('<Iiii', c['angle'], c['range'], c['slope'], c['damage']))]
    geometry = next(r[4:] for r in rows if r[0] == 'D')
    cells = [r[4:] for r in rows if r[0] == 'B']
    candidates = [r[4:] for r in rows if r[0] == 'C']
    intercepts = sorted([r[4:] for r in rows if r[0] == 'I'], key=lambda r: r[2])
    hit = next(r[3:] for r in rows if r[0] == 'H')
    payload = struct.pack('<4i', *geometry) + n('I', len(cells)) + b''.join(struct.pack('<2i', *cell) for cell in cells)
    payload += n('I', len(candidates)) + b''.join(trace_object(*o) for o in candidates)
    payload += n('I', len(intercepts)) + b''.join(n('i', frac) + trace_object(line, i) for line, i, frac in intercepts)
    payload += b'\1' + trace_object(*hit[:2]) + b'\1' + struct.pack('<3i', *hit[2:]) + n('I', sum(r[0] == 'T' for r in rows))
    events.append(combat(1, payload))
    random = [r for r in rows if r[0] == 'E' and r[3] == 8]
    events += [combat(3, bytes([3, random[0][5]])), combat(3, bytes([4, random[1][5]]))]
    spawned = next(r for r in rows if r[0] == 'E' and r[3] == 1)
    events += v4.event(spawned[3:], states)
    # Third draw is implicit P_SpawnMobj lastlook, fourth is impact tics.
    events += [combat(3, bytes([5, random[3][5]])), combat(11, ident(spawned[4]))]
    if hit[0] == 0 and c['damage']:
        damage_rows = [r for r in rows if r[0] == 'E' and r[3] != 8 and r[4] == hit[1]] + random[4:]
        events += damage_events(c, damage_rows, actors[hit[1]], states, properties)
    return events

def snapshot(c, rows, operation):
    states, properties, registry = metadata(c)
    order, spatial, pending, generations = [], [], set(), []
    for r in rows:
        if r[2] > operation:
            continue
        if r[0] == 'L':
            spatial = [a for a in spatial if a != r[3]]
            spatial.insert(0, r[3])
        if r[0] == 'E':
            code, i = r[3:5]
            if code == 1:
                order.append(i)
                assert i == len(generations)
                generations.append(1)
            if code == 2:
                pending.add(i)
                spatial = [a for a in spatial if a != i]
            if code == 3:
                order.remove(i)
                pending.remove(i)
                generations[i] += 1
    current = [r for r in rows if r[2] == operation]
    actors = {r[3]: r[4:] for r in current if r[0] == 'A'}
    initial = life_events([r for r in rows if r[2] == 0], states)
    last, queued = [], initial
    if operation == 1:
        queued += (shot_events(c, current, actors, states, properties) if c['case'] in SHOTS
                   else damage_events(c, current, actors[1], states, properties))
    elif operation > 1:
        # Selected final captures have no movement or actions; all actors settled.
        assert all(r[3] == 0 for r in current if r[0] == 'E')
        assert all(a[5:8] == [0, 0, 0] for a in actors.values() if a[0])
        last = [b'\0\0', b'\0\1'] + life_events(current, states) + [b'\0\2', b'\0\3']
        queued = []
    fields = [('tic', n('Q', max(operation - 1, 0))), ('tic_rate', n('I', 35)), ('profile', n('I', 5)), ('paused', b'\0'), ('rng.gameplay', n('B', actors[0][-1])), ('rng.miscellaneous', b'\0'), ('actors.options', bytes([2, 0, 0])), ('store.max_slots', n('I', 1024)), ('store.slot_count', n('Q', len(generations))), ('store.head', opt(order[0])), ('store.tail', opt(order[-1])), ('thinker.order', b''.join(ident(i) for i in order))]
    for i, generation in enumerate(generations):
        p = f'actor.{i}'
        a = actors[i]
        fields += [(p + '.generation', n('Q', generation)), (p + '.retired', b'\0'), (p + '.occupied', n('B', a[0]))]
        if not a[0]:
            continue
        _, ty, x, y, z, mx, my, mz, health, state, tics, flags, height, radius, reaction, threshold, target, lastlook, floor, ceiling, leaf, _ = a
        pos = order.index(i)
        fields += [(p + '.pending_removal', n('B', i in pending)), (p + '.previous', opt(order[pos - 1] if pos else -1)), (p + '.next', opt(order[pos + 1] if pos + 1 < len(order) else -1))]
        for key, value in [('x', x), ('y', y), ('z', z), ('momx', mx), ('momy', my), ('radius', radius), ('height', height), ('floor', floor), ('ceiling', ceiling)]:
            fields.append((p + '.' + key, n('i', value)))
        s = states[state]
        fields += [(p + '.angle', n('I', 0)), (p + '.lastlook', n('B', lastlook)), (p + '.flags', n('I', flags)), (p + '.type', n('H', ty)), (p + '.state', n('H', state)), (p + '.tics', n('i', tics)), (p + '.health', n('i', health)), (p + '.reaction_time', n('i', reaction)), (p + '.sprite', n('H', s['sprite'])), (p + '.frame', n('I', s['frame'])), (p + '.target', opt(target)), (p + '.death_requested', b'\0'), (p + '.spawnpoint', b'\0'), (p + '.threshold', n('i', threshold)), (p + '.momz', n('i', mz)), (p + '.subsector', n('H', leaf)), (p + '.sector', n('H', leaf))]
    maps, sectors = map_fields(c['map'])
    fields += [('spatial.link_order', b''.join(ident(i) for i in spatial)), ('world.blockmap_policy', b'\1'), ('world.conservative_omissions', bytes(4)), ('sectors.count', n('I', len(sectors) // 26))]
    for i in range(len(sectors) // 26):
        floor, ceiling = struct.unpack_from('<hh', sectors, i * 26)
        fields += [(f'sector.{i}.floor', n('i', floor * U)), (f'sector.{i}.ceiling', n('i', ceiling * U)), (f'sector.{i}.actors', b''.join(ident(a) for a in spatial if actors[a][-2] == i and not actors[a][11] & 8))]
    for i in range(4):
        fields += [(f'player.{i}.actor', bytes(13)), (f'player.{i}.forward', b'\0'), (f'player.{i}.side', b'\0'), (f'player.{i}.angle_turn', bytes(2)), (f'player.{i}.buttons', b'\0')]
    for prefix, events in [('events.last', last), ('events.pending', queued)]:
        fields += [(prefix + '.count', n('Q', len(events)))] + [(f'{prefix}.{i}', e) for i, e in enumerate(events)]
    fields += maps + registry
    data = b'DSIM' + struct.pack('<IQ', 5, len(fields))
    for path, value in fields:
        path = path.encode()
        data += n('I', len(path)) + path + n('Q', len(value)) + value
    return data

def run(check):
    cases = list(csv.DictReader((ROOT / 'tests/reference/m3d1_cases.csv').open()))
    rows = [[r[0], *map(int, r[1:])] for r in csv.reader((ROOT / 'tests/reference/m3d1_trace.csv').open())]
    outputs, index = {}, ['case,operation,tic,fnv1a64,bytes_fixture']
    for which, values in enumerate(cases):
        c = {k: int(v) for k, v in values.items() if k != 'label'}
        for operation in [0] + ([1] if which in DIRECT + SHOTS else []) + ([FINALS[which]] if which in FINALS else []):
            data = snapshot(c, [r for r in rows if r[1] == which], operation)
            digest = 0xcbf29ce484222325
            for byte in data:
                digest = ((digest ^ byte) * 0x100000001b3) & 0xffffffffffffffff
            name = f'm3d1_state_{which}_{operation}.bin'
            # Store representative full bytes; every capture has an independent hash.
            stored = operation > 0 or which in [0, 38]
            if stored:
                outputs['tests/reference/' + name] = data
            index.append(f'{which},{operation},{max(0, operation - 1)},{digest:016x},{name if stored else "-"}')
    outputs['tests/reference/m3d1_snapshots.csv'] = ('\n'.join(index) + '\n').encode()
    inputs = ['tools/reference/m3d1_snapshot_goldens.py', 'tools/reference/m3c_snapshot_goldens.py', 'tests/reference/m3d1_cases.csv', 'tests/reference/m3d1_trace.csv', 'tests/reference/m3c_states.csv', 'tests/reference/m3c_types.csv'] + [f'tests/reference/m3d1_map_{i}.bin' for i in range(9)]
    provenance = {'schema': 'DSIM-v5', 'captures': len(index) - 1, 'inputs_sha256': {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in inputs}, 'outputs_sha256': {p: hashlib.sha256(b).hexdigest() for p, b in outputs.items()}, 'assumptions': ['all setups; selected due-east damage/hitscan API events projected explicitly from source branches and C rows', 'settled final captures have phases and Think events only; source relinking/removal records reconstruct spatial and storage order', 'no Rust output read; trace geometry, cells, candidates, intercepts and hit positions come from original C']}
    outputs['tests/reference/m3d1_snapshot_provenance.json'] = (json.dumps(provenance, indent=2) + '\n').encode()
    for path, data in outputs.items():
        if check:
            assert (ROOT / path).read_bytes() == data, path
        else:
            (ROOT / path).write_bytes(data)
    print(('Checked' if check else 'Generated') + f' {len(index) - 1} independent DSIM-v5 captures, {len(outputs) - 2} byte fixtures.')
if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    run(parser.parse_args().check)
