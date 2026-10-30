#!/usr/bin/env python3
"""New defined target/lifetime inputs to the frozen genuine-body C observer.

No Rust input. Retired allocations are quarantined, as in the inherited observer;
logical retirement is observed, never libc-free/address-reuse equivalence.
"""
import argparse
import csv
import hashlib
import io
import json
from pathlib import Path
import m3f5_reference as source

ROOT = Path(__file__).resolve().parents[2]
main = source.main
SCHEMA = 'DOOM-OWNERSHIP-v1'
main.MAPS, main.THINGS, main.KEYS, main.CASES = [], [], [], []
for label, tics in [('replace_clear', 6), ('damage_retarget', 12),
                    ('target_corpse', 8), ('owner_corpse', 40),
                    ('pending_target', 3), ('replacement_after_retire', 6),
                    ('pending_owner', 3), ('corpse_retains_retired', 8)]:
    main.case(label, n=1, tics=tics, p1active=1)

# Declared harness inputs, not invented original target-setter functions.
INITIAL = '''
owned[0]->target=players[0].mo;observe(6,0,ident(players[0].mo));
if(c==4||c==5)P_RemoveMobj(players[0].mo);
if(c==6)P_RemoveMobj(owned[0]);
'''
OPERATIONS = '''
if(c==0&&t==1){owned[0]->target=players[1].mo;observe(6,0,ident(players[1].mo));}
if(c==0&&t==2){owned[0]->target=NULL;observe(6,0,-1);}
if(c==1&&t==1)P_DamageMobj(owned[0],NULL,players[1].mo,1);
if(c==2&&t==1)P_DamageMobj(players[0].mo,NULL,owned[0],1000);
if((c==3||c==7)&&t==1)P_DamageMobj(owned[0],NULL,players[0].mo,150);
if((c==4||c==5)&&t==1){
owned[0]->target=NULL;observe(6,0,-1);playeringame[0]=0;players[0].mo=NULL;
}
if(c==5&&t==2){
players[0].mo=P_SpawnMobj(v[V_P0X],v[V_P0Y],v[V_P0Z],MT_MISC85);
playeringame[0]=1;players[0].health=100;
owned[0]->target=players[0].mo;observe(6,0,ident(players[0].mo));
}
if(c==7&&t==2){
mobj_t* old=players[0].mo;playeringame[0]=0;players[0].mo=NULL;P_RemoveMobj(old);
/* Owner is already dying; its admitted death actions never dereference target. */
}
'''
code = main.legacy.MAIN
needle = 'capture();\nfor(t=1;'
assert code.count(needle) == 1
code = code.replace(needle, INITIAL + needle)
needle = 'if(t==v[V_DAMAGE_TICK])'
assert code.count(needle) == 1
main.legacy.MAIN = code.replace(needle, OPERATIONS + needle)


def run(check, overflow, address):
    flags = list(main.legacy.base.FLAGS)
    output = source.capture(flags)
    observed = {p: b for p, b in output.items() if p.endswith(('.bin', '.csv'))}
    if overflow:
        strict = source.capture([f for f in flags if f != '-fwrapv'] + [
            '-fno-wrapv', '-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow',
            '-fno-sanitize-recover=all'])
        assert observed == {p: b for p, b in strict.items() if p.endswith(('.bin', '.csv'))}
    if address:
        strict = source.capture([f for f in flags if f != '-fwrapv'] + [
            '-fno-wrapv', '-fsanitize=address,undefined', '-fno-sanitize-recover=all'])
        assert observed == {p: b for p, b in strict.items() if p.endswith(('.bin', '.csv'))}
    inherited = json.loads((ROOT / 'tests/reference/m3f5_provenance.json').read_text())
    manifest = json.loads(output['m3d3j_provenance.json'])
    for key in ['function_sha256', 'source_sha256']:
        assert manifest[key] == inherited[key], key
    assert len(main.MAPS) == 1
    assert output['m3d3j_map_0.bin'] == (ROOT / 'tests/reference/m3f5_map_0.bin').read_bytes()
    raw = output['m3d3j_trace.csv']
    rows = list(csv.reader(io.StringIO(raw.decode())))
    live = [r for r in rows if r[0] == 'A' and r[4] == '1']
    assert {int(r[5]) for r in live} <= {12, 135}
    inventory = dict(new_input_cases=len(main.CASES), tics=sum(c['tics'] for c in main.CASES),
                     captures=sum(r[0] == 'LS' for r in rows), actor_fields=26 * len(live),
                     target_fields=len(live), statistics_fields=4 * sum(r[0] == 'LS' for r in rows),
                     C_gameplay_draws=sum(r[0] == 'E' and r[3] == '8' for r in rows),
                     compiled_original_bodies=len(manifest['functions']), new_unique_bodies=0)
    outputs = {'tests/reference/m3f8_trace.csv': (SCHEMA + '\n').encode() + raw,
               'tests/reference/m3f8_cases.csv': output['m3d3j_cases.csv']}
    inputs = ['tools/reference/m3f8_reference.py', 'tools/reference/m3f5_reference.py',
              'tests/reference/m3f5_provenance.json', 'tests/reference/m3f5_map_0.bin']
    provenance = dict(schema=SCHEMA, revision=manifest['revision'], flags=flags,
                      inventory=inventory, inherited_source_inventory='tests/reference/m3f5_provenance.json',
                      input_sha256={p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in inputs},
                      script_sha256=hashlib.sha256((INITIAL + OPERATIONS).encode()).hexdigest(),
                      outputs_sha256={p: hashlib.sha256(b).hexdigest() for p, b in outputs.items()},
                      assumptions=[
                          'Same70 genuine original bodies and2 setup fragments, checked against frozen M3f5 source hashes. New inputs, not new function/branch coverage.',
                          'Original MT_SERGEANT12/SARG throughout. Only source MT_MISC85 observer body135 has the inherited explicit player-derived unbound descriptor; not a real player.',
                          'Manual target writes are declared inputs; damage replacement, death and thinker retirement execute genuine source bodies.',
                          'Inherited GNU ABI/arithmetic/instrumentation/synthetic geometry/roster adapters remain; source bytes are untouched.',
                          'Z_Free logically retires but quarantines bytes until case reset. No gameplay dereference of a retired target, libc-free safety or zone/address reuse equivalence is claimed.',
                          'Case7 retains an opaque target identity in dying-owner actions which do not read the target. Post-retirement actor fields are excluded from comparisons.',
                          'Slot/generation reuse in DSIM is independently declared Rust schema policy, separate from the C allocation/birth sequence.',
                          'Inherited isolated setup/filter/credit probes also run but are not counted as new lifetime inputs or published again. No Rust output or commercial assets.'])
    outputs['tests/reference/m3f8_provenance.json'] = (json.dumps(provenance, indent=2, sort_keys=True) + '\n').encode()
    for path, data in outputs.items():
        if check:
            assert (ROOT / path).read_bytes() == data, path
        else:
            (ROOT / path).write_bytes(data)
    print(f'{"Checked" if check else "Generated"} {SCHEMA}: {inventory}; strict={overflow}; ASan/UBSan={address}; evidence_bytes={sum(map(len, outputs.values()))}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--overflow-check', action='store_true')
    parser.add_argument('--address-check', action='store_true')
    args = parser.parse_args()
    run(args.check, args.overflow_check, args.address_check)
