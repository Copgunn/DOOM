#!/usr/bin/env python3
"""Independent DSIM24 pistol/player bytes, exclusively from compiled-C rows."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import weapon_snapshot
from evidence_pack import pack, unpack

ROOT = Path(__file__).resolve().parents[2]


def run(check):
    inputs = ['tools/reference/m3g8_snapshot_goldens.py',
              'tools/reference/weapon_snapshot.py',
              'tools/reference/player_execution_snapshot.py',
              'tools/reference/player_snapshot.py',
              'tools/reference/m3g2_snapshot_goldens.py',
              'tools/reference/m3f3_snapshot_goldens.py',
              'tests/reference/m3g8_trace.dref', 'tests/reference/m3g8_cases.csv',
              'tests/reference/m3g8_provenance.json']
    lines = unpack((ROOT/inputs[6]).read_bytes()).decode().splitlines()
    assert lines.pop(0) == 'DOOM-PLAYER-DEMON-WEAPONS-v1'
    rows = [[row[0], *map(int, row[1:])] for row in csv.reader(lines)]
    cases = list(csv.DictReader((ROOT/inputs[7]).open()))
    maps = json.loads((ROOT/inputs[8]).read_text())['maps']
    inputs += maps
    outputs, inventory = weapon_snapshot.build(
        cases, rows, [ROOT/path for path in maps], 'm3g8',
        {(0,0),(1,19),(1,96),(5,19),(10,128),(12,18),(13,129),(18,15),(19,19),(24,64),(25,512),(26,512)})
    events=outputs.pop('tests/reference/m3g8_events.bin')
    outputs['tests/reference/m3g8_events.dref']=pack(events)
    inventory.update(expanded_event_bytes=len(events),packed_event_bytes=len(outputs['tests/reference/m3g8_events.dref']))
    inputs.append('tools/reference/evidence_pack.py')
    provenance = dict(
        schema='DSIM-v24', execution_profile=8, **inventory,
        scope='Genuine player, dynamic four-weapon state, original Demon, actual source '
              'weapon/combat/AI events. Immutable weapon metadata derives from the '
              'explicit policy. Source aim rows are separately observed queries, '
              'not invented monster-aim events. This bounded command/weapon stage '
              'is not full P_Ticker/PlayerThink/DeathThink. Generations are schema '
              'policy; no Rust outputs generate expectations.',
        inputs_sha256={path: hashlib.sha256((ROOT/path).read_bytes()).hexdigest()
                       for path in inputs},
        outputs_sha256={path: hashlib.sha256(data).hexdigest()
                        for path, data in outputs.items()})
    outputs['tests/reference/m3g8_snapshot_provenance.json'] = (
        json.dumps(provenance, indent=2, sort_keys=True)+'\n').encode()
    for path, data in outputs.items():
        if check:
            assert (ROOT/path).read_bytes() == data, path
        else:
            (ROOT/path).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} DSIM24 pistol: {inventory}; '
          f'bytes={sum(map(len, outputs.values()))}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    run(parser.parse_args().check)
