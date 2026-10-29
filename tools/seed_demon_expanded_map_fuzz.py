#!/usr/bin/env python3
"""Public bounded seeds for combined gameplay; no proprietary data or Rust output."""
import csv,sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]
destination=Path(sys.argv[1] if len(sys.argv)>1 else root/'fuzz/corpus/demon_expanded_map');destination.mkdir(parents=True,exist_ok=True)
cases=list(csv.DictReader((root/'tests/reference/m3f3_cases.csv').open()))
seeds={}
for i,c in enumerate(cases):
    seeds['scenario_'+str(i)]=bytes([i,0,8])+bytes([0,0,7,0])*8
    if c['label'] in ['long_lifecycle','multiple_lifecycle','wide_map_lifecycle','expanded_sight_dag']:
        seeds['death_'+str(i)]=bytes([i,0,8,1,0,150,0])+bytes([0,0,7,0])*7
for mode in range(10):seeds['mutation_'+str(mode)]=bytes([0,mode,8,4,0,0,0,0,0,7,0])
seeds['source_clip_boundary']=bytes([255,0,8,9,0,1,1])
seeds['retirement_reuse']=bytes([0,0,8,5,0,0,0,0,0,0,0,7,0,0,0,9,128,0,1])
for name,data in seeds.items():
    assert len(data)<=35
    (destination/name).write_bytes(data)
print(f'{len(seeds)} public synthetic seeds written.')
