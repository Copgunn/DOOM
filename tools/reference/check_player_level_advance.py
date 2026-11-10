#!/usr/bin/env python3
"""Validate the timing caller's final state against independent DSIM38/39 goldens.

This checks state, successful-call counts and the refusal boundary, not speed.
Expected hashes come from the frozen genuine-C projections and fixtures, never
from another Rust run. No captures are produced by the timing caller. Run alone
on an idle machine if retaining the informational timing output.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import level_snapshot as L
from check_player_episode1 import expectation
from level_compare import stripped


def expected_hash(goldens, script, last):
    golden = goldens['scripts'][script]
    if last == golden['tics_projected'] - 1:
        return golden['final_sha256']
    fixture = next(f for f in goldens['fixtures']
                   if f['script'] == script and f['tic'] == last)
    data = (ROOT / fixture['path']).read_bytes()
    assert hashlib.sha256(data).hexdigest() == fixture['sha256']
    version, fields = L.decode(data)
    return hashlib.sha256(stripped(version, fields)[0]).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--binary', required=True)
    args = parser.parse_args()
    cases = []
    level = json.loads((ROOT / 'tests/reference/m3h9c_synthetic_goldens.json').read_text())
    for script, golden in level['scripts'].items():
        last = golden['tics_projected'] - 1
        refusal = last + 1 if golden['end']['kind'] == 'reborn' else None
        cases.append(('level', 'tests/reference/m3h9c_synthetic.wad', 'E1M1',
                      f'tests/reference/m3h9c_scripts/{script}.lmp',
                      script, level, last, refusal))
    episode = json.loads((ROOT / 'tests/reference/m4a_synthetic_goldens.json').read_text())
    manifest = json.loads((ROOT / 'tests/reference/m4a_manifest.json').read_text())
    for wad in manifest['wads']:
        for script in wad['scripts']:
            name = script['lump']
            last, refusal, informational = expectation(script['rust'], episode['scripts'][name])
            assert not informational
            cases.append(('episode1', wad['file'], wad['slot'], script['lmp'],
                          name, episode, last, refusal[0] if refusal else None))
    passed = 0
    for policy, wad, slot, demo, script, goldens, last, refusal in cases:
        result = subprocess.run([args.binary, '--wad', str(ROOT / wad), '--map', slot,
                                 '--demo', str(ROOT / demo), '--policy', policy],
                                capture_output=True, text=True, check=True)
        row = json.loads(result.stdout)
        assert row['admitted'] == last, (script, 'successful-call count')
        assert row['final_tic'] == last, (script, 'final tic')
        if refusal is not None:
            assert row['refused_at'] == refusal, (script, 'required refusal')
        elif row['refused_at'] is not None:
            assert goldens['scripts'][script]['end']['kind'] == 'exit'
            assert row['refused_at'] == last + 1, (script, 'exit refusal')
        assert row['final_sha_stripped'] == expected_hash(goldens, script, last), (script, 'C-derived final state')
        assert row['loop_ns'] >= row['advance_ns'] + row['refused_ns']
        assert row['clock_floor_ns'] <= row['clock_median_ns']
        print(f'{policy} {script}: pass')
        passed += 1
    print(f'check_player_level_advance: {passed} C-derived final states and call/refusal boundaries passed')


if __name__ == '__main__':
    main()
