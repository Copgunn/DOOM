#!/usr/bin/env python3
"""Independent original SARG combat against a genuine owned player.

The frozen active-player driver supplies the declared command gate and original
thinker bodies. This observer changes inputs, not original function bodies.
"""
import argparse
import collections
import csv
import hashlib
import io
import json
from pathlib import Path

import m3g5_reference as previous

ROOT = Path(__file__).resolve().parents[2]
commands = previous.commands
main, legacy = previous.main, previous.legacy
geometry, extent = previous.geometry, previous.extent
SCHEMA = "DOOM-PLAYER-DEMON-MELEE-v1"
DEMON_DOOMED_NUMBER = 3002
MAPTHING_ALL_SKILLS = 1 | 2 | 4
MAPTHING_AMBUSH = 8
CASES = []


def case(label, **changes):
    values = dict(previous.CASES[1])
    values.pop("map")
    values.update(label=label, offset_x=0)
    values.update(changes)
    CASES.append(values)


case("autonomous_chain", tics=416, dx=-256, px=128)
case("near_repeat", tics=192, dx=-64, px=0)
case("pressure_forward_misses", tics=128, dx=-64, px=0,
     forward=8, move_hold=128)
case("pressure_strafe_misses", tics=96, dx=-64, px=0,
     side=8, move_hold=48)
case("near_lethal_corpse", tics=128, dx=-64, px=0, pre_damage=96)
case("prior_momentum", tics=128, dx=-64, px=0,
     hit_at=0, hit_amount=4, physical=1)
case("diagonal_max_damage", tics=128, dx=-40, dy=-40, px=0)
case("range_59", tics=64, dx=-59, px=0)
case("range_60", tics=64, dx=-60, px=0)
case("target_cleared_during_windup", tics=64, dx=-64, px=0, clear_at=20)
case("target_dies_during_windup", tics=96, dx=-64, px=0,
     hit_at=20, hit_amount=100)
case("pain_draw_255", tics=48, dx=-50, px=0, seed=153)
case("turn_while_attack", tics=64, dx=-64, px=0, turn=1024, turn_hold=64)
case("wall_knockback", tics=128, dx=700, px=750, seed=153)
case("boundary_valid_impulse", tics=40, dx=780, px=830, seed=4, offset_x=128)
case("ambush_melee", tics=32, dx=-64, px=0, thing=2)
case("reaction_damage_reset", tics=128, dx=-64, px=0, reaction=40,
     forward=8, move_hold=48)

main.MAPS, main.THINGS, main.KEYS, main.CASES = [], [], [], []
for values in CASES:
    fixture = geometry.fixture(n=1, count=4, blockers=False)
    offset = values["offset_x"]
    fixture["vertices"] = [(6*x + offset, 12*y) for x, y in fixture["vertices"]]
    fixture["lumps"]["VERTEXES"] = b"".join(
        geometry.old.words(vertex) for vertex in fixture["vertices"])
    fixture["range"] = (-768, 768 + offset)
    fixture["center"] = (offset, 0)
    fixture = extent.grid(fixture, (-1024, -1024), 16, 16)
    spawn_flags = MAPTHING_ALL_SKILLS | (MAPTHING_AMBUSH if values["thing"] == 2 else 0)
    main.case(values["label"], n=1, custom=fixture,
              things=[(values["dx"], values["dy"], 0, DEMON_DOOMED_NUMBER, spawn_flags)],
              tics=values["tics"], rejected=False,
              **{"pc_" + key: value for key, value in values.items()
                 if key not in {"label", "tics"}})
    values["map"] = main.CASES[-1]["map"]
commands.CASES = CASES


def sha(data):
    return hashlib.sha256(data).hexdigest()


def run(check, overflow, address):
    flags = list(legacy.base.FLAGS)
    observed = commands.capture(flags)
    raw = observed["m3d3j_trace.csv"]
    for enabled, sanitizers in [
        (overflow, "signed-integer-overflow,integer-divide-by-zero,float-cast-overflow"),
        (address, "address,undefined"),
    ]:
        if enabled:
            strict = commands.capture(
                [flag for flag in flags if flag != "-fwrapv"] +
                ["-fno-wrapv", "-fsanitize=" + sanitizers, "-fno-sanitize-recover=all"])
            assert strict["m3d3j_trace.csv"] == raw
    assert all(values["turn"] >= 0 for values in CASES)
    literal = commands.capture(
        [flag for flag in flags if flag != "-fwrapv"] +
        ["-fno-wrapv", "-fsanitize=undefined", "-fno-sanitize-recover=all"], True)
    assert literal["m3d3j_trace.csv"] == raw
    rows = list(csv.reader(io.StringIO(raw.decode())))
    functions = collections.Counter(row[3] for row in rows if row[0] == "FN")
    frames = collections.defaultdict(list)
    for row in rows:
        frames[tuple(map(int, row[1:3]))].append(row)
    attack_frames = [frame for frame in frames.values()
                     if any(row[0] == "FN" and row[3] == "p_enemy.c:A_SargAttack"
                            for row in frame)]
    attack_checks = [row for frame in attack_frames for row in frame
                     if row[0] == "J" and row[3] == "0" and row[5] == "0"]
    misses = [row for row in attack_checks if row[6] == "0"]
    for row in misses:
        frame = frames[tuple(map(int, row[1:3]))]
        assert not any(item[0] == "J" and item[3] == "1" and item[5] == "4"
                       for item in frame)
    draws = [row for row in rows
             if row[0] == "J" and row[3] == "1" and row[5] == "4"]
    damage = [row for row in rows if row[0] == "G" and row[3] == "2"
              and int(row[2]) > 0 and row[5:7] == ["0", "0"]]
    assert len(draws) == len(damage)
    assert all(int(hit[-1]) == (int(draw[-1]) % 10 + 1) * 4
               for hit, draw in zip(damage, draws))
    assert {4, 40}.issubset({int(row[-1]) for row in damage})
    assert not any(row[0] == "G" and row[3:5] == ["3", "0"] for row in rows)
    selected = {0, 1, 2, 3, 4, 6, 7, 8, 10, 11, 12, 16, 24, 32,
                40, 48, 64, 72, 96, 128, 192, 256, 384}
    captures = {(int(row[1]), int(row[2])) for row in rows
                if (row[0] == "FN" and row[3] == "p_enemy.c:A_SargAttack")
                or (row[0] == "J" and row[3] == "3")
                or (row[0] == "G" and row[3] == "2")}
    keep = [row for row in rows if row[0] not in {"FN", "D", "B", "U", "I", "T"}
            and (row[0] not in {"A", "P", "AP", "R", "S", "LS"}
                 or int(row[2]) in selected
                 or tuple(map(int, row[1:3])) in captures
                 or int(row[2]) == CASES[int(row[1])]["tics"])]
    inside = False
    stage_rng = 0
    for row in keep:
        if row[0] == "PG" and row[3] == "0":
            inside = True
        if row[0] == "PG" and row[3] == "2":
            inside = False
        stage_rng += int(inside and row[0] == "E" and row[3] == "8")
    assert stage_rng == 0
    manifest = json.loads(observed["m3d3j_provenance.json"])
    inventory = dict(
        new_cases=len(CASES), tics=sum(values["tics"] for values in CASES),
        captures=sum(row[0] == "P" for row in keep),
        actor_fields=26*sum(row[0] == "A" and row[4] == "1" for row in keep),
        player_fields=6*sum(row[0] == "P" for row in keep),
        association_fields=sum(row[0] == "AP" for row in keep),
        statistics_fields=4*sum(row[0] == "LS" for row in keep),
        roster_fields=4*sum(row[0] == "R" for row in keep),
        demon_fields=16*sum(row[0] == "DM" for row in keep),
        motion_fields=13*sum(row[0] == "PM" for row in keep),
        command_fields=4*sum(row[0] == "UC" for row in keep),
        command_calls=functions["p_user.c:P_MovePlayer"], command_stage_rng_draws=0,
        gameplay_draws=sum(row[0] == "E" and row[3] == "8" for row in keep),
        setup_discards=0, melee_selections=sum(row[0] == "J" and row[3:4] == ["3"]
                                             and row[5] == "0" for row in keep),
        attack_entries=len(attack_frames), attack_range_checks=len(attack_checks),
        attack_range_misses=len(misses), absent_target_returns=len(attack_frames)-len(attack_checks),
        damage_draws=len(draws), damage_requests=len(damage),
        ignored_attack_requests=sum(row[0] == "G" and row[3] == "12"
                                    for frame in attack_frames for row in frame),
        damage_values=sorted({int(row[-1]) for row in damage}),
        damage_observation_fields=sum(len(row)-3 for row in keep if row[0] == "G"),
        movement_attempts=sum(row[0] == "M" for row in keep),
        collision_candidates=sum(row[0] == "C" for row in keep),
        slide_line_observations=sum(row[0] == "SL" for row in keep),
        compiled_original_bodies=len(manifest["functions"]), added_observer_bodies=0,
        new_engine_source_bodies=0, exercised_unique_functions=len(functions),
        function_entries=sum(functions.values()), literal_defined_cases=len(CASES),
        literal_defined_tics=sum(values["tics"] for values in CASES))
    trace = io.StringIO()
    csv.writer(trace, lineterminator="\n").writerows(keep)
    cases = io.StringIO()
    writer = csv.writer(cases, lineterminator="\n")
    writer.writerow(["case", *CASES[0]])
    writer.writerows([index, *values.values()] for index, values in enumerate(CASES))
    outputs = {"tests/reference/m3g6_trace.csv": (SCHEMA + "\n" + trace.getvalue()).encode(),
               "tests/reference/m3g6_cases.csv": cases.getvalue().encode()}
    known = [ROOT/"tests/reference/m3f5_map_0.bin",
             *sorted((ROOT/"tests/reference").glob("m3g[2345]_map_*.bin"))]
    maps = []
    for index, fixture in enumerate(main.MAPS):
        path = next((path for path in known if path.read_bytes() == fixture["bytes"]), None)
        if path is None:
            path = ROOT/f"tests/reference/m3g6_map_{index}.bin"
            outputs[str(path.relative_to(ROOT))] = fixture["bytes"]
        maps.append(str(path.relative_to(ROOT)))
    outputs["tests/reference/m3g6_maps.rs"] = (
        "// Public synthetic source inputs; GPL-2.0-only.\n"
        "pub const MAP_BYTES: &[&[u8]] = &[\n" +
        "".join(f'    include_bytes!("{Path(path).name}"),\n' for path in maps) +
        "];\n").encode()
    hashes = {file + ":" + name: sha(commands.player._original_body(file, name, result).encode())
              for file, name, result in manifest["functions"]}
    inherited = json.loads((ROOT/"tests/reference/m3g5_provenance.json").read_text())
    assert hashes == inherited["function_sha256"]
    inputs = ["tools/reference/m3g6_reference.py", "tools/reference/m3g5_reference.py",
              "tools/reference/m3g4_reference.py", "tools/reference/m3g3_reference.py",
              "tools/reference/m3g2_reference.py", "tools/reference/m3f3_shim.h"]
    provenance = dict(
        schema=SCHEMA, revision=manifest["revision"], flags=flags, inventory=inventory,
        maps=maps, functions=manifest["functions"], function_sha256=hashes,
        source_sha256={name: sha((legacy.SOURCE/name).read_bytes())
                       for name in inherited["source_sha256"]},
        executed_function_entries=dict(sorted(functions.items())),
        newly_exercised_relative_to_m3g5=sorted(set(functions) - set(inherited["executed_function_entries"])),
        reaction_fragment_sha256=inherited["reaction_fragment_sha256"],
        negative_shift_probe=inherited["negative_shift_probe"],
        turn_adapter=inherited["turn_adapter"],
        inputs_sha256={path: sha((ROOT/path).read_bytes()) for path in inputs},
        assumptions=[
            "Genuine MT_PLAYER0 and MT_SERGEANT12, fixed source playeringame [1,0,0,0]. Canonical pristine metadata; no fixture identity substitution.",
            "Frozen bounded command gate precedes original P_RunThinkers. This is not full P_Ticker/P_PlayerThink/P_DeathThink; no view, weapon, damagecount decay or respawn execution.",
            "All supplied turn commands are nonnegative and literal defined expressions independently match. Inherited negative-turn bit adapter and UB probe are reused qualification, not new negative-input evidence.",
            "A_Chase selects original 8/8/8 attack states; later A_SargAttack faces and rechecks current range before damage RNG. An absent target returns; a dead allocated target can pass range, draw damage and then be ignored by P_DamageMobj.",
            "Medium/commercial, grounded floor0, no armor/powers/cheats/fast/respawn. Reverse thrust is unreachable because height difference is zero. A hit from positive health with damage<=40 cannot select extreme death or the below-50 commercial scream.",
            "Initial health reduction, reaction and between-tic target clearing/death are declared controlled source inputs. autonomous_chain starts at health100 and reaches lethal melee without injected damage.",
            "Source allocation order and actor identity are observed directly. Rust birth/generation projection and invalid-reference/atomicity bounds remain separately specified schema/safety policies.",
            "All-inactive scanning and freed-pointer dereference are never executed. Older retirement/quarantine observations are reused, not new source scenarios.",
            "Selected full actor/player captures plus every per-tic motion/Demon/command/event observation are retained; field families overlap and are not branch coverage. No Rust outputs generate expectations."],
        outputs_sha256={path: sha(data) for path, data in outputs.items()})
    outputs["tests/reference/m3g6_provenance.json"] = (
        json.dumps(provenance, indent=2, sort_keys=True) + "\n").encode()
    for path, data in outputs.items():
        if check:
            assert (ROOT/path).read_bytes() == data, path
        else:
            (ROOT/path).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} {SCHEMA}: {inventory}; '
          f'strict={overflow}; ASan/UBSan={address}; bytes={sum(map(len, outputs.values()))}')


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--overflow-check", action="store_true")
    parser.add_argument("--address-check", action="store_true")
    args = parser.parse_args()
    run(args.check, args.overflow_check, args.address_check)
