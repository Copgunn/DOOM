#!/usr/bin/env python3
"""Check the Rust wire-23 run of the M4a synthetic scripts against the DSIM39 encoder goldens.

    check_player_episode1.py --binary EXAMPLE [--out DIR] [--example-args ARGS] [--require]
    check_player_episode1.py --rust-root DIR

Manifest-driven (tests/reference/m4a_manifest.json). With --binary (the built Rust example,
for instance target/release/examples/player_level_run), every script runs as

    EXAMPLE --wad <wad file> --map <slot> --demo <script .lmp> --out DIR/<SCRIPT> --captures [ARGS]

(ARGS from --example-args, split on spaces, for a wire-selecting option should the example
need one). Before any run, `EXAMPLE --help` must mention wire 23 / the episode1 policy (the
words `episode1`, `Episode1` or `23`); otherwise the tool prints "SKIP: example not
available (no wire 23)" and exits 0 (1 with --require). With --rust-root the captures are
already in DIR/<SCRIPT>/ (`tic_NNNNN.bin`, optional `refused.txt` whose first word is the
refused tic).

The goldens (tests/reference/m4a_synthetic_goldens.json) were projected from the genuine
engine's dumps by m4a_snapshot_goldens.py, never from Rust output. Each Rust capture is
stripped as the comparator strips it (no `map.*`, no `events.*`) and the SHA-256 prefix of
each tic compared with the golden digest; the first differing tic is reported, then the
sequence hash over the compared tics. The manifest's `rust` expectation decides the range
and the ending (DSIM tic numbering: the manifest's event tics are dump tics, capture
N = dump tic N - 1):

- "match": every golden tic matches; a `marker` script has no refused.txt; an `exit`
  script may have one naming the tic after the exit capture with `LevelExited`.
- {"refuse": "PlayerReborn", "tic": T} (T the reborn-request dump tic): captures 0..T+1
  match and refused.txt names T+2 with PlayerReborn (as SYNDIE in M3h9c).
- {"refuse": NAME, "tic": T} for an H7 witness (T the dump tic of the blocked move):
  captures 0..T match and refused.txt names T+1 with the witness token (REFUSAL_TOKENS:
  PlatBlocked, StairBlocked; the manifest's NAME text is also accepted).
- "tbd": compared as "match" but reported as informational; it does not fail the check.

A script whose golden projection stopped (`projection_stop`) is compared over the tics
projected and reported partial; partial coverage of a required range fails. Each fixture is
compared field by field with the Rust capture of its tic when that tic is in the script's
compared range (first differing field path named). Exit status 0 only when every
non-informational check passes."""
import argparse, hashlib, json, os, subprocess, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
from level_compare import stripped, first_difference

MANIFEST = ROOT / 'tests/reference/m4a_manifest.json'
GOLDENS = ROOT / 'tests/reference/m4a_synthetic_goldens.json'
# manifest refusal names -> the token the Rust refusal must carry (plan §A.6 witness errors)
REFUSAL_TOKENS = {'PlayerReborn': 'PlayerReborn',
                  'blocked raiseToNearestAndChange plat (H7)': 'PlatBlocked',
                  'blocked stair step (H7)': 'StairBlocked'}


def rust_stripped(path):
    version, fields = L.decode(Path(path).read_bytes())
    return version, stripped(version, fields)


def expectation(rust, golden):
    """(last capture to compare, required refusal (tic, tokens) or None, informational)."""
    last = golden['snapshots'] - 1
    if rust == 'tbd': return last, None, True
    if rust == 'match': return last, None, False
    name, t = rust['refuse'], rust['tic']
    tokens = {REFUSAL_TOKENS.get(name, name), name}
    if name == 'PlayerReborn': return t + 1, (t + 2, tokens), False
    return t, (t + 1, tokens), False


def example_available(binary):
    try:
        p = subprocess.run([binary, '--help'], capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired):
        return False
    text = p.stdout + p.stderr
    return any(w in text for w in ('episode1', 'Episode1', '23'))


def run_examples(binary, out, extra, man):
    for w in man['wads']:
        for s in w['scripts']:
            target = out / s['lump']
            target.mkdir(parents=True, exist_ok=True)
            cmd = [binary, '--wad', str(ROOT / w['file']), '--map', w['slot'], '--demo', str(ROOT / s['lmp']),
                   '--out', str(target), '--captures'] + extra
            p = subprocess.run(cmd, capture_output=True, text=True)
            print(f"ran {s['lump']}: exit {p.returncode}")


def check(rust_root, man, goldens):
    failures = 0; ranges = {}
    for w in man['wads']:
        for s in w['scripts']:
            script = s['lump']; g = goldens['scripts'][script]; directory = Path(rust_root) / script
            if g['lmp_sha256'] != hashlib.sha256((ROOT / s['lmp']).read_bytes()).hexdigest():
                print(f'{script}: FAIL (the .lmp differs from the goldens)'); failures += 1; continue
            last, refusal, info = expectation(s['rust'], g)
            limit = min(last, g['tics_projected'] - 1)
            partial = limit < last
            digests, mismatch = [], None
            for tic in range(limit + 1):
                path = directory / f'tic_{tic:05}.bin'
                if not path.exists(): mismatch = (tic, 'missing Rust capture'); break
                version, (data, _) = rust_stripped(path)
                if version != L.EPISODE1.version: mismatch = (tic, f'version {version}'); break
                digests.append(hashlib.sha256(data).digest())
                if digests[-1].hex()[:16] != g['tic_digests'][tic]: mismatch = (tic, 'digest'); break
            ok = mismatch is None
            if ok and limit == g['tics_projected'] - 1:
                ok = hashlib.sha256(b''.join(digests)).hexdigest() == g['sequence_sha256'] and \
                    (digests[-1].hex() if digests else None) == g['final_sha256']
                if not ok: mismatch = (limit, 'sequence')
            refused = directory / 'refused.txt'
            words = refused.read_text().split() if refused.exists() else []
            if refusal is not None:
                want, tokens = refusal
                if words[:1] != [str(want)] or not any(t in ' '.join(words[1:]) for t in tokens):
                    ok = False; mismatch = mismatch or (want, f'expected refused.txt: {want} {sorted(tokens)[0]}')
            elif g['end']['kind'] == 'exit' and words[:1] == [str(g['end']['tic'] + 1)] and 'LevelExited' in ' '.join(words[1:]):
                pass
            elif words:
                ok = False; mismatch = mismatch or (int(words[0]), 'unexpected refused.txt')
            if partial and ok: ok = False; mismatch = (limit + 1, 'golden projection stopped before the required range')
            ranges[script] = limit if mismatch is None or mismatch[0] > limit else mismatch[0] - 1
            status = 'pass' if ok else f'FAIL at tic {mismatch[0]} ({mismatch[1]})'
            stop = g['projection_stop']
            print(f"{script}: {status}{' (informational: rust tbd)' if info else ''}; {len(digests)} of {last + 1} "
                  f"required tics checked; end {g['end']['kind']}"
                  + (f"; partial: projection stopped ({stop['kind']} at tic {stop['tic']})" if stop else ''))
            failures += (not ok) and not info
    for f in goldens['fixtures']:
        fixture = ROOT / f['path']
        if hashlib.sha256(fixture.read_bytes()).hexdigest() != f['sha256']:
            print(f"fixture {f['path']}: FAIL (fixture bytes differ from the goldens)"); failures += 1; continue
        if f['tic'] > ranges.get(f['script'], -1):
            print(f"fixture {f['path']}: not compared (outside the matched range of {f['script']})"); continue
        version, fields = L.decode(fixture.read_bytes())
        _, mine = stripped(version, fields)
        _, (_, theirs) = rust_stripped(Path(rust_root) / f['script'] / f"tic_{f['tic']:05}.bin")
        diff = first_difference(mine, theirs)
        print(f"fixture {f['path']} ({f['why']}): " + ('pass' if diff is None else f'FAIL at field {diff[0]}'))
        failures += diff is not None
    return failures


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument('--binary', help='the built Rust example (run per script)')
    src.add_argument('--rust-root', help='existing captures, one directory per script')
    ap.add_argument('--out', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-m4a-rust',
                    help='capture directory for --binary (outside the repository)')
    ap.add_argument('--example-args', default='', help='extra arguments for the example (space-separated)')
    ap.add_argument('--require', action='store_true', help='fail instead of skipping when the example lacks wire 23')
    a = ap.parse_args()
    man = json.loads(MANIFEST.read_text()); goldens = json.loads(GOLDENS.read_text())
    if goldens.get('manifest_sha256') != hashlib.sha256(MANIFEST.read_bytes()).hexdigest():
        print('FAIL: the goldens were made from another manifest (rerun m4a_snapshot_goldens.py)'); return 1
    root = a.rust_root
    if a.binary:
        if not example_available(a.binary):
            print('SKIP: example not available (no wire 23 in its --help)'); return 1 if a.require else 0
        out = a.out.resolve()
        if out == ROOT or ROOT in out.parents: sys.exit('--out must lie outside the repository')
        run_examples(a.binary, out, a.example_args.split(), man); root = str(out)
    failures = check(root, man, goldens)
    print('check_player_episode1: ' + ('all checks passed' if not failures else f'{failures} failures'))
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
