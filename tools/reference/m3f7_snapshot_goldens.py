#!/usr/bin/env python3
"""Compact DSIM-v21 evidence from REUSED M3f5 compiled-C rows; no Rust input.

The unchanged independent observation encoder supplies field/event framing. Source
IDs stay original; only the explicitly synthetic observer body moves to context.
"""
import argparse
import csv
import hashlib
import json
import shutil
import struct
import tempfile
from pathlib import Path
import m3f3_snapshot_goldens as encoder

ROOT = Path(__file__).resolve().parents[2]
REGISTRY = b'DOOM-INFO-1.10-v1'
CONTEXT = b'UNBOUND-BODY-v1'


def fnv(data):
    value = 0xcbf29ce484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    return value


def run(check):
    inputs = ['tools/reference/m3f7_snapshot_goldens.py',
              'tools/reference/m3f3_snapshot_goldens.py',
              'tools/reference/m3c_snapshot_goldens.py',
              'tests/reference/m3f4_identities.csv',
              'tests/reference/m3f5_trace.csv', 'tests/reference/m3f5_cases.csv',
              'tests/reference/m3f5_events.bin']
    metadata = list(csv.reader((ROOT / inputs[3]).read_text().splitlines()[2:]))
    body = sum(row[0] == 'type' for row in metadata)
    assert body == 137
    raw = (ROOT / 'tests/reference/m3f5_trace.csv').read_text().splitlines()
    assert raw.pop(0) == 'DOOM-DEMON-EXEC-v1'
    rows = [[r[0], *map(int, r[1:])] for r in csv.reader(raw)]
    # Not an original actor translation: bind the observer's synthetic target.
    for row in rows:
        if row[0] == 'A' and row[5] == 135:
            row[5] = body
        elif row[0] == 'E' and row[3] == 1 and row[5] == 135:
            row[5] = body
    cases = list(csv.DictReader((ROOT / 'tests/reference/m3f5_cases.csv').open()))
    outputs = {}
    index = ['case,operation,tic,fnv1a64,bytes_fixture,bytes,fields,event_fields']
    events = 0
    with tempfile.TemporaryDirectory(prefix='doom-m3f7-encode-') as directory:
        temporary = Path(directory)
        (temporary / 'tests/reference').mkdir(parents=True)
        maps = sorted((ROOT / 'tests/reference').glob('m3f5_map_*.bin'))
        for path in maps:
            shutil.copyfile(path, temporary / 'tests/reference' / path.name.replace('m3f5', 'm3f3'))
            inputs.append(str(path.relative_to(ROOT)))
        encoder.ROOT = temporary
        encoder.map_fields.cache_clear()
        # Immutable source metadata is identified by schema, not serialized again.
        encoder.metadata = lambda _case: []
        for which, values in enumerate(cases):
            c = {k: int(v) for k, v in values.items() if k != 'label'}
            assert c['mode'] == 4 and c['retire_tick'] == -1
            selected = [r for r in rows if r[1] == which]
            grouped = {}
            for row in selected:
                grouped.setdefault(row[2], []).append(row)
            for op in range(c['tics'] + 1):
                _, fields = encoder.snapshot(c, selected, op, grouped, diagnostics=True)
                assert fields[2][0] == 'profile'
                fields[2:3] = [('profile', struct.pack('<I', 1)),
                               ('registry.schema', REGISTRY), ('context.schema', CONTEXT)]
                data = b'DSIM' + struct.pack('<IQ', 21, len(fields))
                for path, value in fields:
                    path = path.encode()
                    data += struct.pack('<I', len(path)) + path + struct.pack('<Q', len(value)) + value
                name = f'm3f7_state_{which}_{op}.bin' if which == 0 and op in [0, c['tics']] else '-'
                if name != '-':
                    outputs['tests/reference/' + name] = data
                count = sum(path.startswith('events.') for path, _ in fields) - 2
                events += count
                index.append(f'{which},{op},{op},{fnv(data):016x},{name},{len(data)},{len(fields)},{count}')
    outputs['tests/reference/m3f7_snapshots.csv'] = ('\n'.join(index) + '\n').encode()
    manifest = {'schema': 'DSIM-v21', 'execution_profile': 1,
                'registry': REGISTRY.decode(), 'context': CONTEXT.decode(),
                'captures': len(index) - 1, 'full_byte_fixtures': 2,
                'ordered_event_fields': events,
                'new_C_inputs_functions_branches': 0,
                'scope': 'Re-encoding of immutable M3f5 C observations, actual original IDs; observer body135 bound to separate context137. No new gameplay coverage. Two full bytes plus 664 digest checks. All event bytes checked by reusing EV20 with C-observed identity payloads.',
                'inputs_sha256': {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in sorted(inputs)},
                'outputs_sha256': {p: hashlib.sha256(b).hexdigest() for p, b in sorted(outputs.items())}}
    outputs['tests/reference/m3f7_provenance.json'] = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()
    for name, data in outputs.items():
        if check:
            assert (ROOT / name).read_bytes() == data, name
        else:
            (ROOT / name).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} reused C / DSIM-v21: {len(index)-1} captures, 2 full fixtures, {events} event payload fields; {sum(map(len, outputs.values()))} new evidence bytes.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    run(parser.parse_args().check)
