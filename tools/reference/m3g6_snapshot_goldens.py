#!/usr/bin/env python3
"""Independent DSIM22 policy6 bytes from original-C melee observations."""
import argparse
import collections
import csv
import hashlib
import json
from pathlib import Path

import player_execution_snapshot as player

ROOT = Path(__file__).resolve().parents[2]


def run(check):
    inputs = ["tools/reference/m3g6_snapshot_goldens.py",
              "tools/reference/player_execution_snapshot.py",
              "tools/reference/player_snapshot.py",
              "tools/reference/m3g2_snapshot_goldens.py",
              "tools/reference/m3f3_snapshot_goldens.py",
              "tools/reference/m3c_snapshot_goldens.py",
              "tests/reference/m3g6_trace.csv", "tests/reference/m3g6_cases.csv",
              "tests/reference/m3g6_provenance.json"]
    lines = (ROOT/inputs[6]).read_text().splitlines()
    assert lines.pop(0) == "DOOM-PLAYER-DEMON-MELEE-v1"
    rows = [[row[0], *map(int, row[1:])] for row in csv.reader(lines)]
    grouped = collections.defaultdict(list)
    for row in rows:
        grouped[tuple(row[1:3])].append(row)
    roster_fields = 0
    for key, frame in grouped.items():
        body = next((row[3:] for row in frame if row[0] == "P"), None)
        if body is None:
            continue
        observed = [row[3:] for row in frame if row[0] == "R"]
        assert observed == [[0, 1, body[1], body[0]], [1, 0, 0, -1],
                            [2, 0, 0, -1], [3, 0, 0, -1]], key
        assert [body[0], 0] in [row[3:] for row in frame if row[0] == "AP"], key
        roster_fields += 4*len(observed)
    cases = list(csv.DictReader((ROOT/inputs[7]).open()))
    maps = json.loads((ROOT/inputs[8]).read_text())["maps"]
    inputs += maps
    outputs, inventory = player.build(
        cases, [row for row in rows if row[0] != "DM"], [ROOT/path for path in maps],
        6, b"PLAYER-DEMON-1-MEDIUM-GROUNDED-MELEE-v1", "m3g6",
        {(0, 92), (0, 330), (0, 416), (2, 32), (6, 122),
         (11, 17), (13, 17), (16, 32)})
    provenance = dict(
        schema="DSIM-v22", execution_profile=6, roster_fields=roster_fields,
        **inventory,
        scope="Original MT_PLAYER0 and MT_SERGEANT12, fixed active player0. "
              "Source player/mo/roster rows and ordered Look/Chase/attack/damage "
              "events independently encode unchanged DSIM22 fields with policy6 "
              "and a distinct melee context. Command gate plus original thinkers "
              "is not full P_Ticker/PlayerThink/DeathThink. Birth/generation "
              "projection remains separate Rust schema policy. No Rust expected outputs.",
        inputs_sha256={path: hashlib.sha256((ROOT/path).read_bytes()).hexdigest()
                       for path in inputs},
        outputs_sha256={path: hashlib.sha256(data).hexdigest()
                        for path, data in outputs.items()})
    outputs["tests/reference/m3g6_snapshot_provenance.json"] = (
        json.dumps(provenance, indent=2, sort_keys=True) + "\n").encode()
    for path, data in outputs.items():
        if check:
            assert (ROOT/path).read_bytes() == data, path
        else:
            (ROOT/path).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} DSIM22 autonomous melee: '
          f'{inventory}; roster_fields={roster_fields}; bytes={sum(map(len, outputs.values()))}')


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    run(parser.parse_args().check)
