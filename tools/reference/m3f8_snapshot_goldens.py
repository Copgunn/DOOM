#!/usr/bin/env python3
"""Independent compact v21 encoding of NEW ownership C observations; no Rust input."""
import argparse
import csv
import hashlib
import json
import shutil
import struct
import tempfile
from pathlib import Path
import m3f3_snapshot_goldens as encoder
from m3f7_snapshot_goldens import REGISTRY, CONTEXT, fnv

ROOT = Path(__file__).resolve().parents[2]


def run(check):
    inputs = ['tools/reference/m3f8_snapshot_goldens.py', 'tools/reference/m3f3_snapshot_goldens.py',
              'tools/reference/m3f7_snapshot_goldens.py', 'tools/reference/m3c_snapshot_goldens.py',
              'tests/reference/m3f8_trace.csv', 'tests/reference/m3f8_cases.csv',
              'tests/reference/m3f4_identities.csv', 'tests/reference/m3f5_map_0.bin']
    metadata = list(csv.reader((ROOT / inputs[6]).read_text().splitlines()[2:]))
    body = sum(row[0] == 'type' for row in metadata)
    assert body == 137
    raw = (ROOT / inputs[4]).read_text().splitlines()
    assert raw.pop(0) == 'DOOM-OWNERSHIP-v1'
    rows = [[r[0], *map(int, r[1:])] for r in csv.reader(raw)]
    for row in rows:
        if row[0] == 'A' and row[5] == 135:
            row[5] = body
        elif row[0] == 'E' and row[3] == 1 and row[5] == 135:
            row[5] = body
    cases = list(csv.DictReader((ROOT / inputs[5]).open()))
    outputs = {}
    stream = bytearray(b'EV21')
    index = ['case,operation,tic,fnv1a64,bytes_fixture,bytes,fields,event_fields']
    events = 0
    with tempfile.TemporaryDirectory(prefix='doom-m3f8-encoding-') as directory:
        temporary = Path(directory)
        (temporary / 'tests/reference').mkdir(parents=True)
        shutil.copyfile(ROOT / inputs[7], temporary / 'tests/reference/m3f3_map_0.bin')
        encoder.ROOT = temporary
        encoder.map_fields.cache_clear()
        encoder.metadata = lambda _case: []
        for which, values in enumerate(cases):
            c = {k: int(v) for k, v in values.items() if k != 'label'}
            selected = [r for r in rows if r[1] == which]
            grouped = {}
            for row in selected:
                grouped.setdefault(row[2], []).append(row)
            for op in range(c['tics'] + 1):
                _, fields = encoder.snapshot(c, selected, op, grouped, diagnostics=True)
                fields[2:3] = [('profile', struct.pack('<I', 1)),
                               ('registry.schema', REGISTRY), ('context.schema', CONTEXT)]
                data = b'DSIM' + struct.pack('<IQ', 21, len(fields))
                for path, value in fields:
                    path = path.encode()
                    data += struct.pack('<I', len(path)) + path + struct.pack('<Q', len(value)) + value
                observed = [(p, b) for p, b in fields if p.startswith('events.')]
                stream += struct.pack('<III', which, op, len(observed))
                for path, value in observed:
                    path = path.encode()
                    stream += struct.pack('<I', len(path)) + path + struct.pack('<Q', len(value)) + value
                count = len(observed) - 2
                events += count
                name = f'm3f8_state_{which}_{op}.bin' if (which, op) in [(4, 0), (5, 2)] else '-'
                if name != '-':
                    outputs['tests/reference/' + name] = data
                index.append(f'{which},{op},{op},{fnv(data):016x},{name},{len(data)},{len(fields)},{count}')
    outputs['tests/reference/m3f8_snapshots.csv'] = ('\n'.join(index) + '\n').encode()
    outputs['tests/reference/m3f8_events.bin'] = bytes(stream)
    provenance = dict(schema='DSIM-v21', captures=len(index) - 1, full_byte_fixtures=2,
                      ordered_event_fields=events,
                      scope='New C inputs; original type12/SARG retained. Only observer body135 binds to explicit context137. Independent schema policy assigns lowest vacant slots/generations to C birth order, not proof of C allocator reuse. Complete event bytes for every capture; two unique pending/reuse full fixtures. No Rust input; no DSIM format change.',
                      input_sha256={p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in inputs},
                      outputs_sha256={p: hashlib.sha256(b).hexdigest() for p, b in outputs.items()})
    outputs['tests/reference/m3f8_snapshot_provenance.json'] = (json.dumps(provenance, indent=2, sort_keys=True) + '\n').encode()
    for path, data in outputs.items():
        if check:
            assert (ROOT / path).read_bytes() == data, path
        else:
            (ROOT / path).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} NEW C / DSIM-v21: {len(index)-1} captures,2 full fixtures,{events} ordered event fields; {sum(map(len, outputs.values()))} bytes.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    run(parser.parse_args().check)
