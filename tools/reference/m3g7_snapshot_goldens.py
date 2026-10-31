#!/usr/bin/env python3
"""Independent DSIM23 pistol/player bytes, exclusively from compiled-C rows."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import pistol_snapshot

ROOT = Path(__file__).resolve().parents[2]


def run(check):
    inputs = ['tools/reference/m3g7_snapshot_goldens.py',
              'tools/reference/pistol_snapshot.py',
              'tools/reference/player_execution_snapshot.py',
              'tools/reference/player_snapshot.py',
              'tools/reference/m3g2_snapshot_goldens.py',
              'tools/reference/m3f3_snapshot_goldens.py',
              'tests/reference/m3g7_trace.csv', 'tests/reference/m3g7_cases.csv',
              'tests/reference/m3g7_provenance.json']
    lines = (ROOT/inputs[6]).read_text().splitlines()
    assert lines.pop(0) == 'DOOM-PLAYER-DEMON-PISTOL-v1'
    rows = [[row[0], *map(int, row[1:])] for row in csv.reader(lines)]
    cases = list(csv.DictReader((ROOT/inputs[7]).open()))
    maps = json.loads((ROOT/inputs[8]).read_text())['maps']
    inputs += maps
    outputs, inventory = pistol_snapshot.build(
        cases, rows, [ROOT/path for path in maps], 'm3g7',
        {(0, 0), (0, 19), (0, 215), (0, 288), (2, 25), (3, 39),
         (6, 25), (9, 26), (10, 19), (15, 25)})
    provenance = dict(
        schema='DSIM-v23', execution_profile=7, **inventory,
        scope='Genuine player, dynamic pistol state, original Demon, actual source '
              'weapon/combat/AI events. Immutable pistol metadata derives from the '
              'explicit policy. Source aim rows are separately observed queries, '
              'not invented monster-aim events. This bounded command/weapon stage '
              'is not full P_Ticker/PlayerThink/DeathThink. Generations are schema '
              'policy; no Rust outputs generate expectations.',
        inputs_sha256={path: hashlib.sha256((ROOT/path).read_bytes()).hexdigest()
                       for path in inputs},
        outputs_sha256={path: hashlib.sha256(data).hexdigest()
                        for path, data in outputs.items()})
    outputs['tests/reference/m3g7_snapshot_provenance.json'] = (
        json.dumps(provenance, indent=2, sort_keys=True)+'\n').encode()
    for path, data in outputs.items():
        if check:
            assert (ROOT/path).read_bytes() == data, path
        else:
            (ROOT/path).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} DSIM23 pistol: {inventory}; '
          f'bytes={sum(map(len, outputs.values()))}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    run(parser.parse_args().check)
