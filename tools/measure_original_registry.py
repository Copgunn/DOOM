#!/usr/bin/env python3
"""Separate original-ID profile workloads, three native wait4 RSS samples each."""
import json
import platform
import re
import statistics
import subprocess
import tempfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
subprocess.run(['cargo','build','-p','doom-sim','--example','original_registry','--release','--locked'],cwd=ROOT,check=True)
with tempfile.TemporaryDirectory(prefix='doom-original-rusage-') as directory:
    source=Path(directory)/'rusage.c'; launcher=Path(directory)/'rusage'
    source.write_text('#include <sys/resource.h>\n#include <sys/wait.h>\n#include <unistd.h>\n#include <stdio.h>\nint main(int n,char** a){int s;struct rusage u;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(a[1],a+1);_exit(127);}if(wait4(p,&s,0,&u)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",u.ru_maxrss);return WIFEXITED(s)?WEXITSTATUS(s):128+WTERMSIG(s);}\n')
    subprocess.run(['cc','-O2',str(source),'-o',str(launcher)],check=True)
    results=[]
    for case in [0,1,2,5,6,7]:
        samples=[]
        for _ in range(3):
            run=subprocess.run([str(launcher),str(ROOT/'target/release/examples/original_registry'),'--measure',str(case)],capture_output=True,text=True,check=True)
            values={k:int(v) for k,v in re.findall(r'(\w+)=(\d+)',run.stdout)}
            values['max_rss_kib']=int(re.search(r'RSS_KIB=(\d+)',run.stderr).group(1));values['output']=run.stdout.strip();samples.append(values)
        results.append(dict(case=case,samples=samples,medians={k:statistics.median(s[k] for s in samples) for k in samples[0] if k!='output'}))
print(json.dumps(dict(host=f'{platform.system()} {platform.machine()}',rustc=subprocess.check_output(['rustc','--version'],text=True).strip(),method='Three separate release processes each. New original-ID workloads; no before baseline and diagnostic volume differs from legacy. 100 parses/constructors/batches/direct entries timed separately; setup excluded from direct-entry timers. 100000 immutable direct lookups and identity queries. 100 existing script updates include snapshots; 100 bounded encodes/decodes separately. Native wait4 RSS includes process/allocator/events. No isolated CPU/frequency control or threshold; preserve every sample/outlier.',results=results),indent=2))
