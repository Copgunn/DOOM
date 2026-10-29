#!/usr/bin/env python3
"""Additional combined AI opening/clipping scenarios using the same genuine bodies.
Kept separate from the bounded fuzz catalog so its recorded corpus stays stable.
"""
import argparse,hashlib,json
from pathlib import Path
import m3f3_reference as main
ROOT=main.ROOT
main.MAPS=[];main.THINGS=[];main.KEYS=[];main.CASES=[]
main.case('clipped_sight_step',things=[(-96,-16,0,3002,7)],p0y=16*65536,heights=[(0,128)]*128+[(24,128)]*128,damage_tick=30,kill_tick=55,tics=80)
main.case('closed_sight',things=[(-96,-16,0,3002,7)],p0y=16*65536,heights=[(0,128)]*128+[(128,256)]*128,tics=50)

main.case('high_cell_demon',custom=main.extent.grid(main.extent.scene(n=256,sx=16),(-2048,-2048),256,32,sparse=True),things=[(-900,0,0,3002,7)],p0x=900*65536,tics=80)

def run(check,overflow):
    flags=list(main.legacy.base.FLAGS);out=main.extracted(flags)
    if overflow:
        audited=main.extracted([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow','-fno-sanitize-recover=all'])
        assert {p:b for p,b in out.items() if p.endswith(('.bin','.csv'))}=={p:b for p,b in audited.items() if p.endswith(('.bin','.csv'))}
    trace=out['m3d3j_trace.csv'].decode().splitlines()
    assert any(r.startswith('V,') and r.split(',')[4]=='6' for r in trace),'Must actually execute an opening'
    assert any(r.startswith('V,') and r.split(',')[4]=='7' for r in trace),'Must actually execute source slope clipping'
    manifest=json.loads(out.pop('m3d3j_provenance.json'))
    manifest['assumptions']=[('Negative fixed-coordinate spawn shifts use representable multiplication. The inherited A_Chase direction shift uses an explicit angle_t cast under the recorded BAM ABI policy; shadow/spread/jitter draws use explicit left-before-right order. These adapters and GNU signed-shift assumptions are not portable unmodified ISO-C or exhaustive shift-UB coverage.' if line.startswith('negative-coordinate signed shifts') else line) for line in manifest['assumptions']]
    manifest['adapter_sha256']['m3f3_shim.h']=manifest['adapter_sha256'].pop('m3d3j_shim.h')
    for name in ['m3f3_reference.py','m3f3_clipping_reference.py','m3f1_reference.py','m3f2_reference.py']:
        manifest['adapter_sha256'][name]=hashlib.sha256((ROOT/'tools/reference'/name).read_bytes()).hexdigest()
    published={'tests/reference/'+p.replace('m3d3j','m3f3_clipping'):b for p,b in out.items()}
    published['tests/reference/m3f3_clipping_maps.rs']=('// Generated public synthetic inputs. GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("m3f3_clipping_map_{i}.bin"),\n' for i in range(len(main.MAPS)))+'];\n').encode()
    manifest['outputs_sha256']={p:hashlib.sha256(b).hexdigest() for p,b in published.items()}
    manifest['assumptions']+=['Same70 genuine original bodies/setup fragments/native projection as the main M3f3 adapter; not70 additional unique functions.','Non-cardinal rays execute actual opening and slope clipping on 256-sector/769-line geometry; separately exercise an original closed opening. All selected angles source-defined.']
    path=ROOT/'tests/reference/m3f3_clipping_provenance.json'
    if check:
        old=json.loads(path.read_text())
        for k,v in manifest.items():
            if k not in ['compiler','platform']:assert old[k]==v,k
    else:published['tests/reference/m3f3_clipping_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
    for p,b in published.items():
        if check:assert (ROOT/p).read_bytes()==b,p
        else:(ROOT/p).write_bytes(b)
    print(('Checked' if check else 'Generated')+' M3f3 clipping: 3 scenarios / 3 exercised maps / 210 tics; actual opening/clipping rows asserted; typed high-cell source scenario included.')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');p.add_argument('--overflow-check',action='store_true');a=p.parse_args();run(a.check,a.overflow_check)
