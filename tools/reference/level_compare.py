#!/usr/bin/env python3
"""Per-tic comparator of the M3h9c whole-level run (decision G5, M3h9c note §5).

    level_compare.py DUMP_DIR RUST_DIR --wad WAD [--map E1M1] [--demo LMP]
                     [--profile level|episode1] [--values] [--json OUT]
    level_compare.py DUMP_DIR --self-check --wad WAD [--map E1M1] [--demo LMP]
                     [--profile level|episode1]

`--profile` selects the capture contract the dumps are projected to: `level` (the
default) is DSIM38, policy 22, context PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1 (wire 22,
M3h9c); `episode1` is DSIM39, policy 23, context PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1
(wire 23, M4a: level.secret_exit, sector floorpic, the new mover records, the widened
weapon block, powers, cards and backpack). The method is the same for both; a Rust
capture whose version word differs from the profile's is a divergence at field
`version` on the first tic.

DUMP_DIR holds the genuine-engine runner's dumps (`load.txt`, `tic_NNNNN.txt`;
tools/reference/m3h9c_driver.c); RUST_DIR the Rust runner's canonical captures
(`tic_NNNNN.bin`, `player_level_run --captures`), where Rust capture N is the state after
N tics and the dump of tic N - 1 (`load.txt` for 0) is the same state. Every dump is
projected to DSIM38 (or DSIM39) by the independent encoder (level_snapshot.py); both byte strings are
read with the encoder's field reader and compared **without the `map.*` and `events.*`
groups** (G5: the event frames are Rust's own records, which the dump does not project;
the static map is compared separately, once: the `map.*` group of the load capture holds
the bytes of the ten map lumps each side located, so comparing the two groups is the
lump check of M3h9c note §5, and `match` requires it). The first differing tic and field path are
reported; `--values` also prints both values and is for local use only (the values of a
real-IWAD run are map-derived). The comparison stops at the first divergence, at the
first tic the encoder cannot project (its `ProjectionError` kind and tic are reported:
a dump deficiency, an ambiguity or an unadmitted state), or where either side ends.

The projection is exact (level_snapshot.py reads the driver's observer rows; a dump
without them is refused) and there is no tolerance of any field. A run is complete when
every C snapshot was compared: when C stopped on the exit tic (`end=exit`) or on a
reborn request (`end=reborn`), Rust holds the same captures and refuses the following
tic (`LevelExited`, `PlayerReborn`), which is not compared.

`--json OUT` writes the counts-only result: tics compared, match, first divergence (tic
and field path), the projection stop, whether the `map.*` groups are equal and the
SHA-256 of each side's group (in the DSIM field framing), the SHA-256
of each side's per-tic stripped-byte sequence (the SHA-256 over the concatenated per-tic
SHA-256 digests of the stripped captures, tics 0..last compared) and each side's final
stripped digest. The stripped capture is the DSIM framing (`DSIM`, u32 version, u64 field
count, then per field u32 path length, path, u64 value length, value) of the remaining
fields in their order.

`--self-check` needs no Rust captures: every dump is projected, decoded with the frozen
field reader (specials_snapshot.decode), re-encoded and asserted identical, with the
profile's version, policy and context (38/22/level or 39/23/episode1), unique field paths and the actor/thinker-order
cross-references consistent; a second projection of the whole directory must give the
same bytes (determinism)."""
import argparse, hashlib, json, os, struct, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L

SCHEMA = 'DOOM-M3H9C-COMPARE-v1'
PST_REBORN = 2


def stripped(version, fields):
    kept = [(p, v) for p, v in fields if not p.startswith('map.') and not p.startswith('events.')]
    data = bytearray(b'DSIM' + struct.pack('<IQ', version, len(kept)))
    for path, value in kept:
        path = path.encode(); data += struct.pack('<I', len(path)) + path + struct.pack('<Q', len(value)) + value
    return bytes(data), kept


def first_difference(a, b):
    """(path, value_a, value_b) of the first difference of two field lists, or None."""
    for i in range(max(len(a), len(b))):
        if i >= len(a): return (b[i][0] + ' (only in Rust)', None, b[i][1])
        if i >= len(b): return (a[i][0] + ' (only in C)', a[i][1], None)
        if a[i][0] != b[i][0]: return (f'{a[i][0]} (C) / {b[i][0]} (Rust)', a[i][1], b[i][1])
        if a[i][1] != b[i][1]: return (a[i][0], a[i][1], b[i][1])
    return None


def self_check(directory, mapdata, skill, profile=L.LEVEL):
    count = 0; seq = []
    for tic, data, _ in L.project(directory, mapdata, skill, profile=profile):
        version, fields = L.decode(data)
        value = dict(fields)
        assert version == profile.version and L.encode(fields, version) == data, tic
        assert len(value) == len(fields), 'unique field paths'
        assert value['profile'] == L.n('I', profile.policy) and value['context.schema'] == profile.context
        assert value['tic'] == L.n('Q', tic)
        order = value['thinker.order']; ids = [order[k:k + 12] for k in range(0, len(order), 12)]
        shared = value['thinker.shared_order']
        assert [shared[k + 1:k + 13] for k in range(0, len(shared), 13) if shared[k] == 0] == ids
        occupied = {int(p.split('.')[1]) for p, v in fields if p.startswith('actor.') and p.endswith('.occupied') and v == b'\1'}
        assert sorted(struct.unpack_from('<I', i)[0] for i in ids) == sorted(occupied)
        for i in ids:
            slot = struct.unpack_from('<I', i)[0]
            assert value[f'actor.{slot}.generation'] == i[4:]
        link = value['spatial.link_order']
        assert set(link[k:k + 12] for k in range(0, len(link), 12)) <= set(ids)
        count += 1; seq.append(hashlib.sha256(data).digest())
    again = [hashlib.sha256(data).digest() for _, data, _ in L.project(directory, mapdata, skill, profile=profile)]
    assert again == seq, 'the projection is deterministic'
    return count, hashlib.sha256(b''.join(seq)).hexdigest()


def map_bytes(fields):
    """The `map.*` fields in the DSIM field framing (path length, path, value length, value)."""
    out = bytearray()
    for path, value in fields:
        path = path.encode(); out += struct.pack('<I', len(path)) + path + struct.pack('<Q', len(value)) + value
    return bytes(out)


def end_kind(directory):
    """'exit' when the last dump has gameaction == ga_completed (the driver stops on the
    exit tic), else 'marker' (the demo ended)."""
    names = L.m3h9c_dump.snapshot_names(directory)
    last = L.Dump(os.path.join(directory, names[-1]))
    if last.h['gameaction'] == L.GA_COMPLETED: return 'exit', last.tic
    if any(p['ingame'] and p['playerstate'] == PST_REBORN for p in last.players): return 'reborn', last.tic
    return 'marker', last.tic


def rust_end(rust_dir):
    refused = os.path.join(rust_dir, 'refused.txt')
    tics = sorted(int(n[4:9]) for n in os.listdir(rust_dir) if n.startswith('tic_') and n.endswith('.bin'))
    if os.path.exists(refused):
        return 'refused', int(Path(refused).read_text().split()[0]), (tics[-1] if tics else None)
    return 'last', None, (tics[-1] if tics else None)


def compare(dump_dir, rust_dir, mapdata, skill, values=False, profile=L.LEVEL):
    result = dict(schema=SCHEMA, tics_compared=0, match=False, first_divergence=None, projection_stop=None,
                  map_group_equal=None)
    kind, last_tic = end_kind(dump_dir)
    rkind, refused_at, rust_last = rust_end(rust_dir)
    result.update(c_end=dict(kind=kind, tic=last_tic), rust_end=dict(kind=rkind, refused_tic=refused_at, last_capture=rust_last))
    seq_c, seq_r = [], []; last_c = last_r = None; text = []
    proj = L.Projector(mapdata, skill, profile=profile)
    try:
        for dump, following in L.snapshots(dump_dir):
            tic = dump.tic
            fields, _ = proj.step(dump, following)
            path = os.path.join(rust_dir, f'tic_{tic:05}.bin')
            if not os.path.exists(path):
                text.append(f'tic {tic}: no Rust capture'); break
            vr, fr = L.decode(Path(path).read_bytes())
            bc, kc = stripped(profile.version, fields); br, kr = stripped(vr, fr)
            if result['map_group_equal'] is None:
                mc = [x for x in fields if x[0].startswith('map.')]; mr = [x for x in fr if x[0].startswith('map.')]
                result['map_group_equal'] = mc == mr
                result['map_group_sha256'] = dict(c=hashlib.sha256(map_bytes(mc)).hexdigest(),
                                                  rust=hashlib.sha256(map_bytes(mr)).hexdigest())
            seq_c.append(hashlib.sha256(bc).digest()); seq_r.append(hashlib.sha256(br).digest())
            last_c, last_r = seq_c[-1].hex(), seq_r[-1].hex()
            result['tics_compared'] += 1
            if vr != profile.version:
                result['first_divergence'] = dict(tic=tic, field='version'); text.append(f'tic {tic}: version {profile.version} versus {vr}'); break
            diff = first_difference(kc, kr)
            if diff:
                result['first_divergence'] = dict(tic=tic, field=diff[0])
                line = f'tic {tic}: first differing field {diff[0]}'
                if values: line += f' (C {diff[1].hex() if diff[1] is not None else "-"} versus Rust {diff[2].hex() if diff[2] is not None else "-"})'
                text.append(line); break
    except L.ProjectionError as err:
        result['projection_stop'] = dict(kind=err.kind, tic=err.tic, reason=err.text)
        text.append(f'projection stopped: {err.kind} at tic {err.tic}: {err.text}')
    # complete: every C snapshot compared (through the exit tic when C stopped there);
    # Rust may hold the same captures and refuse later (LevelExited after the exit tic)
    complete = result['tics_compared'] == len(L.m3h9c_dump.snapshot_names(dump_dir))
    result['complete'] = complete
    result['match'] = complete and result['map_group_equal'] is True and result['first_divergence'] is None \
        and result['projection_stop'] is None
    result['c_sequence_sha256'] = hashlib.sha256(b''.join(seq_c)).hexdigest()
    result['rust_sequence_sha256'] = hashlib.sha256(b''.join(seq_r)).hexdigest()
    result['final'] = dict(c_stripped_sha256=last_c, rust_stripped_sha256=last_r)
    return result, text


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('dump'); ap.add_argument('rust', nargs='?')
    ap.add_argument('--wad', required=True); ap.add_argument('--map', default='E1M1'); ap.add_argument('--demo')
    ap.add_argument('--values', action='store_true', help='print the two differing values (local use only)')
    ap.add_argument('--json'); ap.add_argument('--self-check', action='store_true')
    ap.add_argument('--profile', choices=sorted(L.PROFILES), default='level',
                    help='level: DSIM38/policy 22 (default); episode1: DSIM39/policy 23')
    a = ap.parse_args()
    profile = L.PROFILES[a.profile]
    mapdata = L.MapData(a.wad, a.map, L.demo_lumps(a.demo)); skill = L.demo_skill(a.demo) if a.demo else 2
    if a.self_check:
        try:
            count, digest = self_check(a.dump, mapdata, skill, profile)
        except L.ProjectionError as err:
            print(f'self-check stopped: {err.kind} at tic {err.tic}: {err.text}'); return 1
        print(f'self-check passed: {count} snapshots round-trip through the field reader; projection deterministic; '
              f'sequence sha256 {digest}'); return 0
    if not a.rust: ap.error('RUST_DIR is required without --self-check')
    result, text = compare(a.dump, a.rust, mapdata, skill, a.values, profile)
    for line in text: print(line)
    print(f"tics compared {result['tics_compared']}; match {'yes' if result['match'] else 'no'}; "
          f"map group equal {result['map_group_equal']}; C end {result['c_end']['kind']} at tic "
          f"{result['c_end']['tic']}; Rust end {result['rust_end']['kind']}")
    if a.json: Path(a.json).write_text(json.dumps(result, indent=1) + '\n')
    return 0 if result['match'] else 1


if __name__ == '__main__':
    sys.exit(main())
