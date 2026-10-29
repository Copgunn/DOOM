#!/usr/bin/env python3
"""Separate canonical API workload; three processes, native wait4 peak RSS."""
import json
import platform
import re
import statistics
import subprocess
import tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
subprocess.run(['cargo','build','-p','doom-sim','--example','canonical_demon','--release','--locked'],cwd=ROOT,check=True)
with tempfile.TemporaryDirectory(prefix='doom-canonical-rusage-') as directory:
    source=Path(directory)/'rusage.c';launcher=Path(directory)/'rusage'
    source.write_text('#include <sys/resource.h>\n#include <sys/wait.h>\n#include <unistd.h>\n#include <stdio.h>\nint main(int n,char** a){int s;struct rusage u;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(a[1],a+1);_exit(127);}if(wait4(p,&s,0,&u)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",u.ru_maxrss);return WIFEXITED(s)?WEXITSTATUS(s):128+WTERMSIG(s);}\n')
    subprocess.run(['cc','-O2',str(source),'-o',str(launcher)],check=True)
    samples=[]
    for _ in range(3):
        r=subprocess.run([str(launcher),str(ROOT/'target/release/examples/canonical_demon'),'--measure'],capture_output=True,text=True,check=True)
        values={k:int(v) for k,v in re.findall(r'(\w+)=(\d+)',r.stdout)}
        values['max_rss_kib']=int(re.search(r'RSS_KIB=(\d+)',r.stderr).group(1));values['output']=r.stdout.strip();samples.append(values)
    medians={k:statistics.median(s[k] for s in samples) for k in samples[0] if k!='output'}
print(json.dumps(dict(host=f'{platform.system()} {platform.machine()}',rustc=subprocess.check_output(['rustc','--version'],text=True).strip(),
 method='New canonical_api_small workload, no pre-M3f5 API baseline. Three separate release processes. 100 parses/constructors/typed batches timed separately; 100 direct canonical ATK2 entries each in a fresh same-input setup, target/roster installation outside timers; 1000 complete binding/definition queries; 100 existing script updates each preceded by a canonical identity query, including snapshots. Native wait4 peak RSS covers process/allocator overhead. Not directly comparable with old workloads that omit these queries, no isolated CPU/frequency control, no performance threshold.',samples=samples,medians=medians),indent=2))
