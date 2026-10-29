#!/usr/bin/env python3
"""Count actual C observations; compiled inventory is not coverage."""
import csv,json
csv.field_size_limit(16*1024*1024)
from collections import Counter
from pathlib import Path
root=Path(__file__).resolve().parents[2]/'tests/reference'
def rows(name):return list(csv.DictReader((root/f'm3f2_{name}.csv').open()))
def ints(value):return [] if value=='-' else value.split(':')
def records(value):return [] if value=='-' else value.split(';')
movement=rows('movement');sight=rows('sight');probes=rows('probes');bsp=rows('bsp');opening=rows('openings');arithmetic=rows('arithmetic')
assert len(rows('boundary'))==1
samples=len(movement)+sum(r['other']!='-' for r in movement)
kinds=Counter();maxwork=0
for row in sight:
    trace=ints(row['visits']);assert len(trace)%5==0
    counts=Counter(int(k) for k in trace[::5]);kinds.update(counts)
    maxwork=max(maxwork,sum(counts[k] for k in range(4)))
cases=rows('cases');maps={int(c['map']) for c in cases}
assert maps=={int(r['map']) for r in sight}
print(json.dumps(dict(functions=len(json.loads((root/'m3f2_provenance.json').read_text())['functions']),scripts=len(cases),actually_exercised_maps=len(maps),tics=len(movement),actor_samples=samples,actor_fields=samples*15,bsp_lookups=len(bsp),bsp_result_fields=2*len(bsp),destination_probes=len(probes),probe_fields=len(probes)*15,movement_decisions=sum(len(records(r['tries'])) for r in movement),movement_result_fields=15*sum(len(records(r['tries'])) for r in movement),ordered_line_candidates=sum(len(ints(r['lines'])) for r in movement),ordered_thing_candidates=sum(len(ints(r['things'])) for r in movement),thinker_entries=sum(len(ints(r['thinkers'])) for r in movement),sector_lists=sum(len(r['members'].split('|')) for r in movement),opening_rows=len(opening),opening_fields=len(opening)*4,ordered_movement_openings=sum(len(records(r['openings'])) for r in movement),sight_queries=len(sight),sight_scalar_fields=6*len(sight),full_sight_visits=sum(kinds.values()),sight_visit_fields=5*sum(kinds.values()),sight_kinds=dict(sorted(kinds.items())),maximum_executed_sight_work=maxwork,seg_entries_also_in_full_trace=sum(len(ints(r['segs'])) for r in sight),rng_index_comparisons=len(movement)+len(sight),primitive_arithmetic_rows=len(arithmetic),primitive_result_fields=4*len(arithmetic),boundary_component_rows=1,boundary_component_results=5,captured_xy_raw_range=[min(min(int(r['x']),int(r['y'])) for r in movement),max(max(int(r['x']),int(r['y'])) for r in movement)],qualifications=['40 extracted bodies plus original numeric/RNG translation units; not function/branch coverage','named-field geometry/unbound-state adapter, not complete original loader or actor ABI','RNG indexes compared, not a complete raw-draw transcript','seg-only assertion overlaps complete sight-order assertion; not additional unique observations','two-actor script has direct C comparisons; independent canonical encoder excludes it']),indent=2))
