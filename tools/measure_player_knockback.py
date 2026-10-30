#!/usr/bin/env python3
"""Separate three-process grounded-player host measurements; no timing gates."""
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
    if platform.system()!='Linux' or platform.libc_ver()[0]!='glibc':
        raise SystemExit('Linux/glibc measurement only; use asserted example elsewhere')
    checked(['cargo','build','-p','doom-sim','--example','player_knockback','--example','diagnostic_free','--release','--locked'])
    binary=ROOT/'target/release/examples/player_knockback'
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
        for label in ['physical','friction','collision','blocker','moving_corpse','initialization']:
            digests=set()
            for mode in (['advance'] if label=='initialization' else ['advance','step']):
                samples=[];allocations=[];command=[binary,'--measure',label,mode]
                for _ in range(3):
                    stdout,rss,_=execute(command);row=fields(next(line for line in stdout.splitlines() if line.startswith('knockback ')));digests.add(row['digest']);samples.append(dict(ns=row['ns'],max_rss_kib=rss))
                    _,_,a=execute(command,True);assert len(a)==1;allocations.append(a[0])
                results.append(dict(label=label,mode=row['mode'],samples=samples,digest=row['digest'],allocation_samples=allocations,medians={k:statistics.median(s[k] for s in samples) for k in samples[0]}))
            assert len(digests)==1
        inputs=['crates/doom-sim/examples/player_knockback.rs','tests/support/m3g3.rs','tests/reference/m3g3_cases.csv','tests/reference/m3f5_map_0.bin','tools/measure_player_knockback.py','tools/measurement/alloc_probe.c']
        inputs.extend(str(p.relative_to(ROOT)) for p in sorted((ROOT/'tests/reference').glob('m3g3_map_*.bin')))
        print(json.dumps(dict(provenance=dict(commit=checked(['git','rev-parse','HEAD'],capture_output=True).stdout.strip(),tree=checked(['git','rev-parse','HEAD^{tree}'],capture_output=True).stdout.strip(),status=checked(['git','status','--porcelain'],capture_output=True).stdout,rustc=checked(['rustc','--version'],capture_output=True).stdout.strip(),host=platform.platform(),source_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs}),controls=controls,results=results,
            method='Three separate release timing/RSS processes and three separate allocation-instrumented processes per workload/mode.100 shared-core updates, scripted damage/removal included; setup and final explicit Snapshot/digest excluded from timer/counters but included in whole-process RSS. Initialization separately measures100 full world constructions+body spawns with prepared input maps; input parse/clone outside timer, disposal of previous worlds included. No executable baseline physical-player equivalent; these are new separately identified workloads. Frozen M3g2 player workloads are measured unchanged by their own tool before and after. Allocation counts use the unchanged validated glibc thread-local interposer, not allocator internals/mappings/peak-live-heap/leak measurement. Requested bytes cumulative. No CPU isolation or timing threshold; retain all samples/outliers.'),separators=(',',':')))
if __name__=='__main__':main()
