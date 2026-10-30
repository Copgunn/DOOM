#!/usr/bin/env python3
"""New ownership microbenchmarks; raw and scoped paths, three processes each."""
import json
import platform
import re
import statistics
import subprocess
import tempfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
subprocess.run(['cargo','build','-p','doom-sim','--example','context_ownership','--release','--locked'],cwd=ROOT,check=True)
with tempfile.TemporaryDirectory(prefix='doom-context-rusage-') as directory:
    source=Path(directory)/'rusage.c'; launcher=Path(directory)/'rusage'
    source.write_text('#include <sys/resource.h>\n#include <sys/wait.h>\n#include <unistd.h>\n#include <stdio.h>\nint main(int n,char** a){int s;struct rusage u;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(a[1],a+1);_exit(127);}if(wait4(p,&s,0,&u)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",u.ru_maxrss);return WIFEXITED(s)?WEXITSTATUS(s):128+WTERMSIG(s);}\n')
    subprocess.run(['cc','-O2',str(source),'-o',str(launcher)],check=True)
    results=[]; digests=set()
    for path in ['raw','scoped']:
        samples=[]
        for _ in range(3):
            r=subprocess.run([str(launcher),str(ROOT/'target/release/examples/context_ownership'),'--measure',path],capture_output=True,text=True,check=True)
            values={k:int(v) for k,v in re.findall(r'(\w+_us)=(\d+)',r.stdout)}
            values['max_rss_kib']=int(re.search(r'RSS_KIB=(\d+)',r.stderr).group(1))
            digests.add(re.search(r'digest=([0-9a-f]+)',r.stdout).group(1));values['output']=r.stdout.strip();samples.append(values)
        results.append(dict(path=path,samples=samples,medians={k:statistics.median(s[k] for s in samples) for k in samples[0] if k!='output'}))
    assert len(digests)==1, 'same mutations/retirement must have identical outcomes'
print(json.dumps(dict(host=f'{platform.system()} {platform.machine()}',rustc=subprocess.check_output(['rustc','--version'],text=True).strip(),method='New separate microbenchmark, not a historical before baseline. Three release processes per path. 100000 raw permissive target lookups versus scoped generation/definition/role-aware observations (additional validation differs);100 replacements;100 mark/step-snapshot/spawn/stale-check cycles. Setup/token admission excluded. Same inputs, mutation sequence and final digest. Native wait4 RSS; no CPU/frequency isolation or allocation instrumentation. Do not interpret raw lookup latency as equivalent validation or compare this workload with whole-simulation benchmarks.',results=results),indent=2))
