#!/usr/bin/env python3
"""Independent command-player DSIM22 and complete source-ordered event evidence."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import player_execution_snapshot as player

ROOT = Path(__file__).resolve().parents[2]


def run(check):
    inputs = ['tools/reference/m3g4_snapshot_goldens.py', 'tools/reference/player_execution_snapshot.py',
              'tools/reference/player_snapshot.py', 'tools/reference/m3g2_snapshot_goldens.py',
              'tools/reference/m3f3_snapshot_goldens.py', 'tools/reference/m3c_snapshot_goldens.py',
              'tests/reference/m3g4_trace.csv', 'tests/reference/m3g4_cases.csv', 'tests/reference/m3g4_provenance.json']
    text = (ROOT/inputs[6]).read_text().splitlines()
    assert text.pop(0) == 'DOOM-PLAYER-COMMAND-v1'
    rows = [[r[0], *map(int, r[1:])] for r in csv.reader(text)]
    cases = list(csv.DictReader((ROOT/inputs[7]).open()))
    maps = json.loads((ROOT/inputs[8]).read_text())['maps']
    inputs += maps
    outputs, inventory = player.build(cases, rows, [ROOT/p for p in maps], 4,
        b'PLAYER-COMMAND-1-MEDIUM-GROUNDED-v1', 'm3g4', {(1, 1), (6, 8), (12, 4), (13, 4), (15, 16), (18, 72)})
    provenance = dict(schema='DSIM-v22', execution_profile=4, **inventory,
        scope='New compiled-C player command/reaction gate and full source-ordered PG phase events. Negative turns use the explicitly declared historical unsigned-shift interpretation, not ISO-C equivalence. Existing DSIM22 fields/tags and PBE1 framing; distinct policy4/context. Frozen non-event field and player/event primitives reused, old fixed-phase scaffolding replaced from actual source stage observations. Birth/generation projection is independent Rust schema policy, not C allocator identity. No Rust expected output.',
        inputs_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs},
        outputs_sha256={p:hashlib.sha256(b).hexdigest() for p, b in outputs.items()})
    outputs['tests/reference/m3g4_snapshot_provenance.json'] = (json.dumps(provenance, indent=2, sort_keys=True)+'\n').encode()
    for p, data in outputs.items():
        if check:
            assert (ROOT/p).read_bytes() == data, p
        else:
            (ROOT/p).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} DSIM22 player commands: {inventory}; bytes={sum(map(len, outputs.values()))}')


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--check', action='store_true')
    run(p.parse_args().check)
