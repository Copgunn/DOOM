#!/usr/bin/env python3
"""Check the Rust whole-level run of the synthetic scripts against the encoder goldens.

    check_player_level.py RUST_ROOT

RUST_ROOT holds one directory per synthetic script (`RUST_ROOT/<SCRIPT>/`), each the
output of `player_level_run --wad tests/reference/m3h9c_synthetic.wad --map E1M1 --demo
tests/reference/m3h9c_scripts/<SCRIPT>.lmp --out RUST_ROOT/<SCRIPT> --captures`. The
goldens (tests/reference/m3h9c_synthetic_goldens.json) were projected from the genuine
engine's dumps by m3h9c_snapshot_goldens.py, never from Rust output. Per script every
golden tic's Rust capture is stripped as the comparator strips it (no `map.*`, no
`events.*`) and its SHA-256
prefix compared with the golden digest; the first differing tic is reported, then the
sequence hash and the final digest. Each fixture's stripped bytes are compared with the
Rust capture of its tic, and the first differing field path is named. The C run that
ended on the exit tic is matched through that tic (Rust refuses the next one). A script
whose projection stopped (`projection_stop`) is checked over the tics projected and
reported as partial. Exit status 0 only when every check passes."""
import argparse, hashlib, json, os, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
from level_compare import stripped, first_difference

GOLDENS = ROOT / 'tests/reference/m3h9c_synthetic_goldens.json'


def rust_stripped(path):
    version, fields = L.decode(Path(path).read_bytes())
    return stripped(version, fields)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('rust_root')
    a = ap.parse_args()
    goldens = json.loads(GOLDENS.read_text()); failures = 0
    for script, e in goldens['scripts'].items():
        directory = os.path.join(a.rust_root, script)
        digests = []; mismatch = None
        for tic in range(e['tics_projected']):
            path = os.path.join(directory, f'tic_{tic:05}.bin')
            if not os.path.exists(path): mismatch = (tic, 'missing Rust capture'); break
            data, _ = rust_stripped(path)
            digests.append(hashlib.sha256(data).digest())
            if digests[-1].hex()[:16] != e['tic_digests'][tic]: mismatch = (tic, 'digest'); break
        ok = mismatch is None and hashlib.sha256(b''.join(digests)).hexdigest() == e['sequence_sha256'] \
            and (digests[-1].hex() if digests else None) == e['final_sha256']
        partial = e['projection_stop'] is not None
        status = 'pass' if ok else f'FAIL at tic {mismatch[0]} ({mismatch[1]})' if mismatch else 'FAIL (sequence)'
        print(f"{script}: {status}; {len(digests)} of {e['snapshots']} tics checked; end {e['end']['kind']}"
              + (f"; partial: projection stopped ({e['projection_stop']['kind']} at tic {e['projection_stop']['tic']})" if partial else ''))
        failures += not ok
    for f in goldens['fixtures']:
        fixture = ROOT / f['path']
        assert hashlib.sha256(fixture.read_bytes()).hexdigest() == f['sha256'], f['path']
        version, fields = L.decode(fixture.read_bytes())
        _, mine = stripped(version, fields)
        path = os.path.join(a.rust_root, f['script'], f"tic_{f['tic']:05}.bin")
        if not os.path.exists(path):
            print(f"fixture {f['path']}: FAIL (no Rust capture)"); failures += 1; continue
        _, theirs = rust_stripped(path)
        diff = first_difference(mine, theirs)
        print(f"fixture {f['path']} ({f['why']}): " + ('pass' if diff is None else f'FAIL at field {diff[0]}'))
        failures += diff is not None
    print('check_player_level: ' + ('all checks passed' if not failures else f'{failures} failures'))
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
