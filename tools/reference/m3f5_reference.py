#!/usr/bin/env python3
"""Original type12/SARG execution, with no Demon identity adaptation in C.

Only source-derived observers and public synthetic inputs supply expected data.
The target135/player-roster and GNU arithmetic adapters remain explicit.
"""
import argparse
import csv
import hashlib
import io
import json
import re
from pathlib import Path
import m3f3_reference as main
import generate_m3c as metadata

ROOT = main.ROOT
SCHEMA = 'DOOM-DEMON-EXEC-v1'
main.MAPS, main.THINGS, main.KEYS, main.CASES = [], [], [], []
main.case('canonical_small', n=1, tics=80)
main.case('canonical_lifecycle', damage_tick=45, kill_tick=75, tics=130)
main.case('canonical_four_owners', things=[(-96,-40,0,3002,7),(-96,40,0,3002,7),(-32,-40,0,3002,7),(-32,40,0,3002,7)], tics=80)
main.case('canonical_shared_death', n=1, things=[(-32,-32,0,3002,7),(32,32,-180,3002,7)], p0x=0, body_health=1,
          enter_all=metadata.STATE_NAMES.index('S_SARG_ATK2'), tics=45)
main.case('canonical_ambush_sound', things=[(-96,0,0,3002,15)], sound_tick=2, p0health=0, tics=60)
main.case('canonical_clipping', things=[(-96,-16,0,3002,7)], p0y=16*65536,
          heights=[(0,128)]*128+[(24,128)]*128, damage_tick=30, kill_tick=55, tics=80)
main.case('canonical_high_grid', custom=main.extent.grid(main.extent.scene(n=256,sx=16),(-2048,-2048),256,32,sparse=True),
          things=[(-900,0,0,3002,7)], p0x=900*65536, tics=80)
main.case('canonical_line_bound', n=1, count=1024, tics=40)
main.case('canonical_rng_wrap', n=1, skip=253, damage_tick=20, kill_tick=40, tics=60)

# Change setup/wrappers, never the extracted genuine gameplay function bodies.
original_main = main.legacy.MAIN
start = original_main.index('static void definitions(void)')
end = original_main.index('static void reset(', start)
source_setup = '''static void definitions(void){
/* Explicit unbound target fixture; original Demon and all states stay pristine. */
mobjinfo[135]=mobjinfo[MT_PLAYER];mobjinfo[135].doomednum=-1;
mobjinfo[135].spawnhealth=1000;mobjinfo[135].painchance=0;
mobjinfo[135].radius=16*FRACUNIT;mobjinfo[135].flags=6;
}\n'''
main.legacy.MAIN = original_main[:start] + source_setup + original_main[end:]
translation = 'if(admit_native&&inside_map&&type==MT_SERGEANT)type=136;'
assert main.legacy.SPAWN_WRAPPERS.count(translation) == 1
main.legacy.SPAWN_WRAPPERS = main.legacy.SPAWN_WRAPPERS.replace(translation, '')


def capture(flags):
    return main.extracted(flags)


def rows(data):
    return list(csv.reader(io.StringIO(data.decode())))


def run(check, overflow, address=False):
    flags = list(main.legacy.base.FLAGS)
    output = capture(flags)
    if overflow:
        strict = capture([f for f in flags if f != '-fwrapv'] + ['-fno-wrapv', '-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow', '-fno-sanitize-recover=all'])
        assert {p:b for p,b in output.items() if p.endswith(('.bin','.csv'))} == {p:b for p,b in strict.items() if p.endswith(('.bin','.csv'))}, 'strict original-C observation differs'
    if address:
        sanitized = capture([f for f in flags if f != '-fwrapv'] + ['-fno-wrapv', '-fsanitize=address,undefined', '-fno-sanitize-recover=all'])
        assert {p:b for p,b in output.items() if p.endswith(('.bin','.csv'))} == {p:b for p,b in sanitized.items() if p.endswith(('.bin','.csv'))}, 'ASan/UBSan observation differs'
    old_manifest = json.loads(output.pop('m3d3j_provenance.json'))
    trace = rows(output['m3d3j_trace.csv'])
    actors = [r for r in trace if r[0] == 'A' and r[4] != '0']
    demons = [r for r in actors if r[5] == '12']
    assert demons and not any(r[5] == '136' for r in actors)
    observed = {int(r[13]) for r in demons}
    expected = set(range(metadata.STATE_NAMES.index('S_SARG_STND'), metadata.STATE_NAMES.index('S_SARG_DIE6') + 1))
    assert expected <= observed, ('unobserved executable source states', expected - observed)
    assert not observed.intersection(range(496,502)), 'raise execution is excluded'
    assert any(r[0] == 'V' and r[4] == '7' for r in trace), 'actual clipping must execute'
    outputs = {}
    for name, data in output.items():
        destination = 'tests/reference/' + name.replace('m3d3j', 'm3f5')
        if name.endswith('_trace.csv') or name.endswith('_spawn.csv'):
            data = (SCHEMA + '\n').encode() + data
        outputs[destination] = data
    outputs['tests/reference/m3f5_maps.rs'] = ('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n' + ''.join(f'    include_bytes!("m3f5_map_{i}.bin"),\n' for i in range(len(main.MAPS))) + '];\n').encode()
    inventory = dict(old_manifest['evidence_inventory'])
    inventory.update(canonical_demon_actor_observations=len(demons), canonical_state_ordinals=sorted(observed),
                     source_actor_field_comparisons=26*len(actors), canonical_definition_fields=2*len(demons))
    manifest = {'schema':SCHEMA, 'revision':old_manifest['revision'], 'flags':flags,
                'address_audit_flags':['-fno-wrapv','-fsanitize=address,undefined','-fno-sanitize-recover=all'],
                'strict_audit_flags':['-fno-wrapv','-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow','-fno-sanitize-recover=all'],
                'functions':old_manifest['functions'], 'function_sha256':old_manifest['function_sha256'],
                'source_sha256':old_manifest['source_sha256'], 'setup_fragments':old_manifest['setup_fragments'],
                'evidence_inventory':inventory,
                'adapter_sha256':{name:hashlib.sha256((ROOT/'tools/reference'/name).read_bytes()).hexdigest() for name in ['m3f5_reference.py','m3f3_reference.py','m3f3_shim.h','m3f1_reference.py','m3f2_reference.py']},
                'legacy_adapter_sha256':{('m3f3_shim.h' if k=='m3d3j_shim.h' else k):v for k,v in old_manifest['adapter_sha256'].items()},
                'setup_override_sha256':hashlib.sha256(source_setup.encode()).hexdigest(),
                'assumptions':['Original type12/info12 and source475..495 throughout genuine gameplay; no Demon table mutation or spawn identity translation in C.',
                               'Target135 remains an explicit source-player-derived unbound body; roster/player0 fallback statistics remain declared context projections, not real player gameplay.',
                               'Same70 compiled original bodies plus2 setup fragments as M3f3, not additional function/branch coverage.',
                               'Inherited GNU ABI, source extraction/instrumentation, named-field synthetic geometry, representable negative spawn multiplication, explicit BAM direction shift and left-before-right paired draw adapters; not portable unmodified ISO C.',
                               'Normal source-defined THING angles, no Nightmare/Fast/respawn/raise, Spectre or Shotgun Guy execution. No Rust output or proprietary assets.',
                               'Raw canonical source protocol is separately versioned; DSIM-v20 comparison requires an explicitly declared independent identity projection.'],
                'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()}}
    outputs['tests/reference/m3f5_provenance.json'] = (json.dumps(manifest,indent=2)+'\n').encode()
    for path,data in outputs.items():
        target = ROOT/path
        if check:
            assert target.read_bytes() == data, path
        else:
            target.write_bytes(data)
    print(f'{"Checked" if check else "Generated"} {SCHEMA}: {len(main.CASES)} cases, {sum(c["tics"] for c in main.CASES)} tics; {len(actors)*26} actor fields; {len(demons)*2} canonical definition fields; overflow audit={overflow}; ASan/UBSan={address}.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--overflow-check', action='store_true')
    parser.add_argument('--address-check', action='store_true')
    args = parser.parse_args()
    run(args.check, args.overflow_check, args.address_check)
