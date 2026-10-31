#!/usr/bin/env python3
"""Matched legacy weapon-profile control using frozen public C-backed inputs."""
import hashlib
import json
import os
import platform
import re
import statistics
import tempfile
from pathlib import Path
ROOT = Path(os.environ.get('DOOM_SOURCE_ROOT', Path(__file__).resolve().parents[1]))
import sys
sys.path.insert(0, str(ROOT/'tools'))
from measure_stepping import ALLOC, ALLOC_KEYS, checked, fields

PROGRAM = r'''
#[path = "@FIXTURES@"]
mod fixtures;
use std::{hint::black_box,io::Write,time::Instant};
fn main()->Result<(),Box<dyn std::error::Error>> {
    let mode=std::env::args().nth(1).ok_or("mode required")?;
    if mode!="advance" && mode!="step" {return Err("unknown mode".into());}
    let cases=fixtures::cases();
    let mut sim=fixtures::setup(&cases[1]);
    let commands=fixtures::commands(1);
    let mut output=std::io::stdout().lock();
    output.write_all(b"DOOM_ALLOC_BEGIN\n")?;output.flush()?;
    let start=Instant::now();
    for c in commands.iter().copied().take(100) {
        if mode=="advance" {sim.advance(c)?;} else {drop(black_box(sim.step(c)?));}
    }
    let elapsed=start.elapsed();
    output.write_all(b"DOOM_ALLOC_END\n")?;output.flush()?;
    writeln!(output,"legacy_pistol label=held mode={mode} calls=100 ns={} digest={:016x}",elapsed.as_nanos(),sim.snapshot().digest())?;
    Ok(())
}
'''

def main():
    if platform.system() != 'Linux' or platform.libc_ver()[0] != 'glibc':
        raise SystemExit('Linux/glibc measurement only')
    checked(['cargo','build','-p','doom-sim','--example','diagnostic_free','--release','--locked'])
    with tempfile.TemporaryDirectory(prefix='doom-legacy-weapons-') as directory:
        temp=Path(directory);src=temp/'legacy.rs'
        source=PROGRAM.replace('@FIXTURES@',str(ROOT/'tests/support/m3d2.rs'))
        src.write_text(source);binary=temp/'legacy'
        command=['rustc','--edition=2024','-C','opt-level=3','-L',str(ROOT/'target/release/deps'),str(src),'-o',str(binary)]
        for crate in ['doom_sim','doom_types','doom_map','doom_wad']:
            library=max((ROOT/'target/release/deps').glob('lib'+crate+'-*.rlib'),key=lambda p:p.stat().st_mtime)
            command += ['--extern',crate+'='+str(library)]
        compiler_env=os.environ.copy();compiler_env["CARGO_MANIFEST_DIR"]=str(ROOT/"crates/doom-sim")
        checked(command,env=compiler_env)
        probe=temp/'alloc.so'
        checked(['cc','-std=c11','-O2','-Wall','-Wextra','-Werror','-fPIC','-shared',str(ROOT/'tools/measurement/alloc_probe.c'),'-o',str(probe)])
        launcher_source=temp/'rusage.c';launcher=temp/'rusage'
        launcher_source.write_text('''#define _GNU_SOURCE
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
int main(int n,char **v){int s;struct rusage u;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(v[1],v+1);_exit(127);}if(wait4(p,&s,0,&u)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",u.ru_maxrss);return WIFEXITED(s)?WEXITSTATUS(s):128+WTERMSIG(s);}
''')
        checked(['cc','-O2','-Wall','-Wextra','-Werror',str(launcher_source),'-o',str(launcher)])
        def execute(args,instrument=False):
            env=os.environ.copy()
            if instrument:env['LD_PRELOAD']=str(probe)
            r=checked([str(launcher),*map(str,args)],env=env,capture_output=True)
            rss=int(re.search(r'RSS_KIB=(\d+)',r.stderr).group(1))
            allocations=[dict(zip(ALLOC_KEYS,map(int,a)))for a in ALLOC.findall(r.stderr)]
            assert all(a['failures']==0 for a in allocations)
            return r.stdout,rss,allocations
        controls={}
        for flag in ['--control','--probe']:
            _,_,a=execute([ROOT/'target/release/examples/diagnostic_free',flag],True)
            assert len(a)==1;controls[flag]=a[0]
        assert controls['--control']==dict(zip(ALLOC_KEYS,[0,0,0,0,0]))
        assert controls['--probe']==dict(zip(ALLOC_KEYS,[1,0,4096,1,0]))
        results=[];digests=set()
        for mode in ['advance','step']:
            samples=[];allocations=[]
            for _ in range(3):
                stdout,rss,_=execute([binary,mode]);row=fields(next(s for s in stdout.splitlines()if s.startswith('legacy_pistol ')))
                digests.add(row['digest']);samples.append(dict(ns=row['ns'],max_rss_kib=rss))
                _,_,a=execute([binary,mode],True);assert len(a)==1;allocations.append(a[0])
            results.append(dict(label='historical_pistol_held',mode=mode,samples=samples,digest=row['digest'],allocation_samples=allocations,medians={k:statistics.median(s[k]for s in samples)for k in samples[0]}))
        assert len(digests)==1
    paths=['tests/support/m3d2.rs','tests/support/m3d1.rs','tests/reference/m3d2_cases.csv','tests/reference/m3d2_commands.csv','tools/measurement/alloc_probe.c','tools/measure_stepping.py']
    print(json.dumps(dict(provenance=dict(commit=checked(['git','rev-parse','HEAD'],capture_output=True).stdout.strip(),tree=checked(['git','rev-parse','HEAD^{tree}'],capture_output=True).stdout.strip(),status=checked(['git','status','--porcelain'],capture_output=True).stdout,rustc=checked(['rustc','--version'],capture_output=True).stdout.strip(),host=platform.platform(),driver_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),program_sha256=hashlib.sha256(source.encode()).hexdigest(),source_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in paths}),controls=controls,results=results,method='Frozen M3d2 held-pistol fixture1, first100 updates. Existing public helper, synthetic target136 and historical Profile6 semantics. Three separate release timing/RSS processes and three allocation processes per mode. Kernel built with normal Cargo release, standalone caller rustc opt-level3. Setup/input collection/final explicit snapshot excluded from interval, included in process RSS. Allocation interposer controls validated; cumulative requests are not peak-live-heap/leak measurement. Shared host, no isolation or timing gate.'),separators=(',',':')))

if __name__=='__main__':main()
