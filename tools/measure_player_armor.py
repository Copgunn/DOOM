#!/usr/bin/env python3
"""Separate three-process active-player/Demon host measurements; no timing gates."""
import argparse
import hashlib
import json
import os
import platform
import re
import statistics
import subprocess
import tempfile
from pathlib import Path
from measure_stepping import ALLOC, ALLOC_KEYS, checked, fields
ROOT=Path(__file__).resolve().parents[1]

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--smoke',action='store_true')
    args=parser.parse_args()
    if platform.system()!='Linux' or platform.libc_ver()[0]!='glibc':
        raise SystemExit('Linux/glibc measurement only; use asserted example elsewhere')
    checked(['cargo','build','-p','doom-sim','--example','player_armor','--example','diagnostic_free','--release','--locked'])
    binary=ROOT/'target/release/examples/player_armor'
    with tempfile.TemporaryDirectory(prefix='doom-player-measure-') as directory:
        temp=Path(directory);library=temp/'alloc.so'
        checked(['cc','-std=c11','-O2','-Wall','-Wextra','-Werror','-fPIC','-shared',str(ROOT/'tools/measurement/alloc_probe.c'),'-o',str(library)])
        src=temp/'rusage.c';src.write_text('''#define _GNU_SOURCE
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
int main(int n,char **v){int s;struct rusage u;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(v[1],v+1);_exit(127);}if(wait4(p,&s,0,&u)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",u.ru_maxrss);return WIFEXITED(s)?WEXITSTATUS(s):128+WTERMSIG(s);}
''');launcher=temp/'rusage';checked(['cc','-O2','-Wall','-Wextra','-Werror',str(src),'-o',str(launcher)])
        def execute(command,probe=False):
            env=os.environ.copy()
            if probe:env['LD_PRELOAD']=str(library)
            p=checked([str(launcher),*map(str,command)],env=env,capture_output=True)
            rss=int(re.search(r'RSS_KIB=(\d+)',p.stderr).group(1));alloc=[dict(zip(ALLOC_KEYS,map(int,a))) for a in ALLOC.findall(p.stderr)]
            assert all(a['failures']==0 for a in alloc)
            return p.stdout,rss,alloc
        controls={}
        for flag in ['--control','--probe']:
            _,_,a=execute([ROOT/'target/release/examples/diagnostic_free',flag],True);assert len(a)==1;controls[flag]=a[0]
        assert controls['--control']==dict(zip(ALLOC_KEYS,[0,0,0,0,0]))
        assert controls['--probe']==dict(zip(ALLOC_KEYS,[1,0,4096,1,0]))
        results=[]
        for label in (['collect_green','blue_absorb'] if args.smoke else ['initialize','none_idle','green_idle','blue_idle','sixteen','collect_green','collect_blue','green_refuse','blue_refuse','green_absorb','blue_absorb','green_deplete','blue_deplete','armored_lethal','moving_corpse','mixed_resources','pending_weapon_death','full_encounter','full_depletion']):
            digests=set()
            for mode in (['advance'] if label=='initialize' else ['advance','step','capture']):
                samples=[];allocations=[];command=[binary,'--measure',label,mode]
                for _ in range(3):
                    stdout,rss,_=execute(command);row=fields(next(line for line in stdout.splitlines() if line.startswith('player_armor ')));digests.add(row['digest']);samples.append(dict(ns=row['ns'],calls=row['calls'],ns_per_100=row['ns']*100/row['calls'],max_rss_kib=rss))
                    _,_,a=execute(command,True);assert len(a)==1;allocations.append(a[0])
                results.append(dict(label=label,mode=row['mode'],samples=samples,digest=row['digest'],allocation_samples=allocations,medians={k:statistics.median(s[k] for s in samples) for k in samples[0]}))
            assert len(digests)==1
        inputs=['crates/doom-sim/examples/player_armor.rs','tests/support/m3g10.rs','tests/reference/m3g10_cases.csv','tools/measure_player_armor.py','tools/measurement/alloc_probe.c','tools/measure_stepping.py','tests/support/actor_player_observations.rs','tests/support/player_execution_evidence.rs','tests/support/evidence_pack.rs','tests/reference/m3g10_trace.dref']
        inputs += ['tests/reference/m3g10_maps.rs','tests/reference/m3g10_contract.json']
        inputs.extend(json.loads((ROOT/'tests/reference/m3g10_contract.json').read_text())['maps'])
        inputs=sorted(set(inputs))
        print(json.dumps(dict(provenance=dict(commit=checked(['git','rev-parse','HEAD'],capture_output=True).stdout.strip(),tree=checked(['git','rev-parse','HEAD^{tree}'],capture_output=True).stdout.strip(),status=checked(['git','status','--porcelain'],capture_output=True).stdout,rustc=checked(['rustc','--version'],capture_output=True).stdout.strip(),host=platform.platform(),source_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs}),smoke=args.smoke,controls=controls,results=results,
            method='Three independent timing/RSS processes and three separately allocation-instrumented processes per workload/mode. initialize reports100 complete map/player/item/weapon initializations, separately from gameplay updates. Exactly100 updates except full_encounter768 and full_depletion1408, normalized to100; allocation totals cover the entire interval. Setup, source parsing, warmup and final digest outside timer/counters but inside whole-process RSS. Warmups green/blue_refuse2, green_absorb450, blue_absorb360, green_deplete660, blue_deplete850, armored_lethal730, moving_corpse1180; others0. All commands from independently committed C streams, default commands after their end. Idle uses default commands. step and advance+capture measure intentional diagnostic cost separately. Shared validated glibc interposer: cumulative requested bytes, not peak live heap or leak measurement. No CPU isolation or hard timing gate. Short-window labels are continuous scenarios, not isolated action costs. Normal-health full encounters obtain all armor and healing from real world contacts. Shared-host timings are advisory.'),separators=(',',':')))
if __name__=='__main__':main()
