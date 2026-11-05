#!/usr/bin/env python3
"""Synthetic DSIM38 goldens of the M3h9c whole-level run, from the genuine-engine dumps.

    m3h9c_snapshot_goldens.py --dumps RUNS_DIR [--check]

RUNS_DIR is the `runs` directory of `m3h9c_reference.py --keep-dumps --work DIR` (the
strict variant's dumps, `strict_<SCRIPT>/`). Every script of tests/reference/m3h9c_scripts
is projected by the independent encoder (level_snapshot.py, exact mode: the driver's
observer rows) on tests/reference/m3h9c_synthetic.wad; nothing here reads Rust output.
Outputs (synthetic, committable):

- tests/reference/m3h9c_synthetic_goldens.json: per script the snapshot count, the end
  (marker, or exit with its tic), the projection stop if the encoder refuses a tic
  (kind and tic; the comparison then covers the tics before it), the per-tic digest list
  (the first 16 hex digits of the SHA-256 of each capture without `map.*` and `events.*`,
  the comparator's stripped form), the SHA-256 of the concatenated full per-tic SHA-256
  digests, the final stripped SHA-256, and the tics whose pending records are unrecorded
  (a pending mobj in a run's last dump; level_snapshot.strip_unrecorded);
- tests/reference/m3h9c_level_<SCRIPT>_<tic>.bin: full DSIM38 captures (map included,
  no events) at chosen tics (FIXTURES), listed with their SHA-256 in the JSON.

`--check` regenerates everything in memory and fails unless it equals the committed
files. tools/reference/check_player_level.py checks the Rust example's captures against
these goldens."""
import argparse, hashlib, json, os, struct, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
from level_compare import stripped, end_kind
from m3h9_geometry import enum_index

SCHEMA = 'DOOM-M3H9C-SYNTHETIC-GOLDENS-v1'
WAD = ROOT / 'tests/reference/m3h9c_synthetic.wad'
SCRIPTS = ROOT / 'tests/reference/m3h9c_scripts'
OUT = ROOT / 'tests/reference/m3h9c_synthetic_goldens.json'
MT_BARREL = enum_index('mobjtype_t', 'MT_BARREL')
S_BEXP4 = enum_index('statenum_t', 'S_BEXP4')
PST_DEAD = 1


def chosen(script, tic, value, state):
    """Fixture rules, evaluated on the projected fields of each tic (first match wins)."""
    picks = []
    if script == 'SYNIDLE' and tic == 0: picks.append('load')
    dead = value['original_player.0.state'] == bytes([PST_DEAD])
    if dead and 'death' not in state:
        state['death'] = tic; picks.append('player death')
    if 'death' in state and tic == state['death'] + 35 and 'deaththink' not in state:
        state['deaththink'] = tic; picks.append('P_DeathThink, 35 tics after the death')
    if value['level.exit'] == b'\1' and 'exit' not in state:
        state['exit'] = tic; picks.append('exit tic')
    if 'barrel' not in state:
        for p, v in value.items():
            if p.endswith('.type') and v == L.n('H', MT_BARREL):
                if value[p[:-4] + 'state'] == L.n('H', S_BEXP4):
                    state['barrel'] = tic; picks.append('barrel at S_BEXP4 (A_Explode)'); break
    if script == 'SYNCHAOS' and tic == 2500: picks.append('chaos mid-run')
    if script == 'SYNFIGHT' and tic == state.get('last'): picks.append('fight last tic')
    return picks


def generate(runs):
    mapdata = L.MapData(str(WAD), 'E1M1')
    goldens = dict(schema=SCHEMA, encoder='tools/reference/level_snapshot.py (exact)',
                   wad_sha256=hashlib.sha256(WAD.read_bytes()).hexdigest(), scripts={}, fixtures=[])
    outputs = {}
    for lmp in sorted(SCRIPTS.glob('*.lmp')):
        script = lmp.stem; directory = os.path.join(runs, f'strict_{script}')
        names = L.m3h9c_dump.snapshot_names(directory)
        kind, last = end_kind(directory)
        entry = dict(lmp_sha256=hashlib.sha256(lmp.read_bytes()).hexdigest(), snapshots=len(names),
                     end=dict(kind=kind, tic=last), projection_stop=None, unrecorded=[])
        proj = L.Projector(mapdata); digests = []; state = dict(last=last); final = None
        try:
            for dump, following in L.snapshots(directory):
                fields, _ = proj.step(dump, following)
                full = L.encode(fields)
                value = dict(fields)
                for why in chosen(script, dump.tic, value, state):
                    path = f'tests/reference/m3h9c_level_{script}_{dump.tic}.bin'
                    if path not in outputs:
                        outputs[path] = full
                        goldens['fixtures'].append(dict(path=path, script=script, tic=dump.tic, why=why,
                                                        sha256=hashlib.sha256(full).hexdigest()))
                if proj.unrecorded:
                    entry['unrecorded'].append(dict(tic=dump.tic, slots=sorted(proj.unrecorded)))
                    fields = L.strip_unrecorded(fields, proj.unrecorded)
                data, _ = stripped(L.VERSION, fields)
                digests.append(hashlib.sha256(data).digest()); final = digests[-1].hex()
        except L.ProjectionError as err:
            entry['projection_stop'] = dict(kind=err.kind, tic=err.tic, reason=err.text)
        entry.update(tics_projected=len(digests), sequence_sha256=hashlib.sha256(b''.join(digests)).hexdigest(),
                     final_sha256=final, tic_digests=[d.hex()[:16] for d in digests])
        goldens['scripts'][script] = entry
    outputs[str(OUT.relative_to(ROOT))] = (json.dumps(goldens, indent=1) + '\n').encode()
    return outputs, goldens


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--dumps', required=True, help="m3h9c_reference.py's runs directory (kept dumps)")
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    outputs, goldens = generate(a.dumps)
    for script, e in goldens['scripts'].items():
        stop = e['projection_stop']
        print(f"{script}: snapshots {e['snapshots']} end {e['end']['kind']} at tic {e['end']['tic']}; projected "
              f"{e['tics_projected']}" + (f"; projection stopped: {stop['kind']} at tic {stop['tic']}" if stop else '') +
              f"; sequence {e['sequence_sha256'][:16]}")
    print(f"fixtures: {len(goldens['fixtures'])}")
    if a.check:
        bad = [p for p, b in outputs.items() if not (ROOT / p).exists() or (ROOT / p).read_bytes() != b]
        committed = {str(p.relative_to(ROOT)) for p in (ROOT / 'tests/reference').glob('m3h9c_level_*.bin')}
        extra = committed - set(outputs)
        if bad or extra:
            print(f'check FAILED: {len(bad)} differing, {len(extra)} unexpected files'); return 1
        print(f'check passed: {len(outputs)} files'); return 0
    for path in (ROOT / 'tests/reference').glob('m3h9c_level_*.bin'):
        if str(path.relative_to(ROOT)) not in outputs: path.unlink()
    for p, b in outputs.items(): (ROOT / p).write_bytes(b)
    print(f'wrote {len(outputs)} files'); return 0


if __name__ == '__main__':
    sys.exit(main())
