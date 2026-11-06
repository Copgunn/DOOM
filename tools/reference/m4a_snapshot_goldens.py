#!/usr/bin/env python3
"""Synthetic DSIM39 goldens of the M4a synthetic WADs, from the genuine-engine dumps.

    m4a_snapshot_goldens.py [--dumps RUNS_DIR | --work DIR] [--check]

Manifest-driven (tests/reference/m4a_manifest.json, schema DOOM-M4A-MANIFEST-v1): every
script of every WAD is projected by the independent encoder (level_snapshot.py, exact
mode, profile `episode1`: DSIM39, policy 23, context PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1)
from the strict variant's dumps (`strict_<SCRIPT>/`) on its WAD. Nothing here reads Rust
output. RUNS_DIR is the `runs` directory of `m4a_reference.py --keep-dumps --work DIR`;
without --dumps the tool runs `m4a_reference.py --keep-dumps --check --work DIR` itself
(DIR outside the repository, default ~/scratch/doom-m4a-goldens; the dumps are deleted
afterwards). Outputs (synthetic, committable):

- tests/reference/m4a_synthetic_goldens.json: per script its WAD, the snapshot count, the
  end (marker, exit or reborn, with the DSIM tic of the last snapshot, cross-checked with
  the manifest's c_end/end_tic), the manifest's Rust expectation, the projection stop if
  the encoder refuses a tic (kind, tic and reason; the goldens then cover the tics before
  it), the per-tic digest list (the first 16 hex digits of the SHA-256 of each capture
  without `map.*` and `events.*`, the comparator's stripped form), the SHA-256 of the
  concatenated full per-tic SHA-256 digests and the final stripped SHA-256;
- tests/reference/m4a_episode1_<SCRIPT>_<tic>.bin: full DSIM39 captures (map included,
  no events) at the tics of FIXTURES, listed with their SHA-256 in the JSON.

DSIM tic numbering: the load is tic 0 and the dump `tic_NNNNN` (after G_Ticker N) is tic
N + 1. The manifest's event tics are dump tics, so the capture showing an event is its
event tic + 1, and the last capture compared before a refusal at an event is the event
tic itself.

`--check` regenerates everything in memory and fails unless it equals the committed
files. tools/reference/check_player_episode1.py checks the Rust example against these
goldens."""
import argparse, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
from level_compare import stripped, end_kind

SCHEMA = 'DOOM-M4A-SYNTHETIC-GOLDENS-v1'
MANIFEST = ROOT / 'tests/reference/m4a_manifest.json'
OUT = ROOT / 'tests/reference/m4a_synthetic_goldens.json'
PROFILE = L.EPISODE1
# Fixtures: (script, manifest event `what` or None for the load, offset to the event's dump
# tic, why). Offset 1: the capture showing the event; offset 0: the last capture before
# the tic that produces the event (the last compared tic before a Rust refusal there).
FIXTURES = [
    ('DOORS1', '26 without the blue key: PD_BLUEK, no thinker', 1, 'locked door bumped without the key (PD_BLUEK)'),
    ('DOORS1', 'blue keycard', 1, 'third keycard picked up (all three cards)'),
    ('DOORS3', 'RD5 hits the barrel and reverses (crushed, default case)', 1, 'close-30 door reversed by the barrel'),
    ('FLOORS3', 'the comb reaches 64', 1, 'comb at its destination (20-neighbour limit)'),
    ('PLATS1', 'P20 takes the alcove floor TESTFLR0', 1, 'raiseToNearestAndChange: floorpic and special changed'),
    ('PLATS2', None, 0, 'last tic before the blocked raiseToNearestAndChange plat (H7 refusal)'),
    ('STAIRS1', 'S1 7: a1 to 8', 1, 'staircase built (busy step skipped, height bumped)'),
    ('STAIRS2', None, 0, 'last tic before the blocked stair step (H7 refusal)'),
    ('DONUT1', 'the ring takes the pool flat', 1, 'donut ring finished: texture and special applied'),
    ('EXIT2', 'S1 51: secretexit set', 1, 'secret exit requested'),
    ('SECT11', 'barrel blast capped to health - 1', 1, 'sector 11: damage capped at health - 1'),
    ('SECT11', 'health <= 10: G_ExitLevel', 1, 'sector 11: exit at health <= 10'),
    ('SAW1', 'A_Saw hit: next tic forwardmove 100 (MF_JUSTATTACKED)', 1, 'chainsaw lunge: command rewritten to forward 100'),
    ('ROCKET1', 'rocket explosion', 1, 'rocket explosion (S_EXPLODE1)'),
    ('ITEMS1', 'backpack', 1, 'backpack: max ammo doubled'),
    ('TIMERS', 'invisibility runs out', 1, 'invisibility expires: MF_SHADOW cleared'),
    ('SPECT2', 'a Spectre bites the imp (infighting)', 1, 'Spectre-imp infighting'),
    ('SCALE2', None, None, 'load of the large map'),
]


def manifest():
    man = json.loads(MANIFEST.read_text())
    if man.get('schema') != 'DOOM-M4A-MANIFEST-v1': sys.exit('manifest schema')
    return man


def fixture_tics(man):
    """{script: {dsim tic: why}} from FIXTURES and the manifest's event tics."""
    scripts = {s['lump']: s for w in man['wads'] for s in w['scripts']}
    out = {}
    for script, what, offset, why in FIXTURES:
        s = scripts[script]
        if what is None and offset is None: tic = 0
        elif what is None: tic = s['rust']['tic'] + offset          # the manifest's refusal event tic
        else: tic = next(e['tic'] for e in s['must_show'] if e['what'] == what) + offset
        out.setdefault(script, {})[tic] = why
    return out


def end_check(script, s, kind, last, secretexit):
    """The dump's end against the manifest (marker: end_tic = the DSIM tic of the last
    snapshot; exit and reborn: end_tic is the last dump tic, one less)."""
    want = s['end_tic'] if kind == 'marker' else s['end_tic'] + 1
    if kind != s['c_end'] or last != want or secretexit != s['secretexit']:
        raise SystemExit(f'{script}: the dumps end {kind} at tic {last} (secretexit {secretexit}); the manifest says '
                         f"{s['c_end']} at {s['end_tic']} (secretexit {s['secretexit']})")


def generate(runs):
    man = manifest(); picks = fixture_tics(man)
    goldens = dict(schema=SCHEMA, encoder='tools/reference/level_snapshot.py (exact, profile episode1)',
                   dsim=dict(version=PROFILE.version, policy=PROFILE.policy, context=PROFILE.context.decode()),
                   manifest_sha256=hashlib.sha256(MANIFEST.read_bytes()).hexdigest(), scripts={}, fixtures=[])
    outputs = {}
    for w in man['wads']:
        wad = ROOT / w['file']
        if hashlib.sha256(wad.read_bytes()).hexdigest() != w['sha256']: sys.exit(f"{w['name']}: WAD differs from the manifest")
        for s in w['scripts']:
            script = s['lump']; lmp = ROOT / s['lmp']; directory = os.path.join(runs, f'strict_{script}')
            mapdata = L.MapData(str(wad), w['slot'], L.demo_lumps(lmp))
            names = L.m3h9c_dump.snapshot_names(directory)
            kind, last = end_kind(directory)
            final_dump = L.Dump(os.path.join(directory, names[-1]))
            end_check(script, s, kind, last, bool(final_dump.h['secretexit']))
            entry = dict(wad=w['name'], wad_sha256=w['sha256'], slot=w['slot'],
                         lmp_sha256=hashlib.sha256(lmp.read_bytes()).hexdigest(), snapshots=len(names),
                         end=dict(kind=kind, tic=last), manifest_end=dict(c_end=s['c_end'], end_tic=s['end_tic']),
                         rust=s['rust'], projection_stop=None)
            proj = L.Projector(mapdata, L.demo_skill(lmp), profile=PROFILE); digests = []; final = None
            want = picks.get(script, {})
            try:
                for dump, following in L.snapshots(directory):
                    fields, _ = proj.step(dump, following)
                    if dump.tic in want:
                        full = L.encode(fields, PROFILE.version)
                        path = f'tests/reference/m4a_episode1_{script}_{dump.tic}.bin'
                        outputs[path] = full
                        goldens['fixtures'].append(dict(path=path, script=script, tic=dump.tic, why=want[dump.tic],
                                                        sha256=hashlib.sha256(full).hexdigest()))
                    data, _ = stripped(PROFILE.version, fields)
                    digests.append(hashlib.sha256(data).digest()); final = digests[-1].hex()
            except L.ProjectionError as err:
                entry['projection_stop'] = dict(kind=err.kind, tic=err.tic, reason=err.text)
            missing = [t for t in want if not any(f['script'] == script and f['tic'] == t for f in goldens['fixtures'])]
            if missing: print(f'{script}: fixture tics {missing} not projected')
            entry.update(tics_projected=len(digests), sequence_sha256=hashlib.sha256(b''.join(digests)).hexdigest(),
                         final_sha256=final, tic_digests=[d.hex()[:16] for d in digests])
            goldens['scripts'][script] = entry
    outputs[str(OUT.relative_to(ROOT))] = (json.dumps(goldens, indent=1) + '\n').encode()
    return outputs, goldens


def reference_runs(work):
    work = work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    subprocess.run([sys.executable, str(HERE / 'm4a_reference.py'), '--work', str(work), '--keep-dumps', '--check'],
                   check=True, stdout=subprocess.DEVNULL)
    return work / 'runs'


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--dumps', help="m4a_reference.py's runs directory (kept dumps)")
    ap.add_argument('--work', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-m4a-goldens',
                    help='scratch directory for the reference run when --dumps is not given')
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    runs = a.dumps or str(reference_runs(a.work))
    try:
        outputs, goldens = generate(runs)
    finally:
        if not a.dumps: shutil.rmtree(Path(runs).parent, ignore_errors=True)
    for script, e in goldens['scripts'].items():
        stop = e['projection_stop']
        print(f"{script}: snapshots {e['snapshots']} end {e['end']['kind']} at tic {e['end']['tic']}; projected "
              f"{e['tics_projected']}" + (f"; projection stopped: {stop['kind']} at tic {stop['tic']}: {stop['reason']}" if stop else '')
              + f"; sequence {e['sequence_sha256'][:16]}; final {(e['final_sha256'] or '-')[:16]}")
    print(f"fixtures: {len(goldens['fixtures'])}")
    committed = {str(p.relative_to(ROOT)) for p in (ROOT / 'tests/reference').glob('m4a_episode1_*.bin')}
    if a.check:
        bad = [p for p, b in outputs.items() if not (ROOT / p).exists() or (ROOT / p).read_bytes() != b]
        extra = committed - set(outputs)
        if bad or extra:
            print(f'check FAILED: {len(bad)} differing, {len(extra)} unexpected files'); return 1
        print(f'check passed: {len(outputs)} files'); return 0
    for path in committed - set(outputs): (ROOT / path).unlink()
    for p, b in outputs.items(): (ROOT / p).write_bytes(b)
    print(f'wrote {len(outputs)} files'); return 0


if __name__ == '__main__':
    sys.exit(main())
