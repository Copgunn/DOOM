#!/usr/bin/env python3
"""Separate provenance operations; same gameplay through two validated access paths."""
import json
import platform
import re
import statistics
import subprocess
import tempfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
subprocess.run(['cargo', 'build', '-p', 'doom-sim', '--example', 'game_instance', '--release', '--locked'], cwd=ROOT, check=True)
with tempfile.TemporaryDirectory(prefix='doom-instance-rusage-') as directory:
    source = Path(directory) / 'rusage.c'
    launcher = Path(directory) / 'rusage'
    source.write_text('#include <sys/resource.h>\n#include <sys/wait.h>\n#include <unistd.h>\n#include <stdio.h>\nint main(int n,char** a){int s;struct rusage u;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(a[1],a+1);_exit(127);}if(wait4(p,&s,0,&u)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",u.ru_maxrss);return WIFEXITED(s)?WEXITSTATUS(s):128+WTERMSIG(s);}\n')
    subprocess.run(['cc', '-O2', str(source), '-o', str(launcher)], check=True)
    results = []
    digests = set()
    for path in ['scoped', 'qualified']:
        samples = []
        for _ in range(3):
            run = subprocess.run([str(launcher), str(ROOT / 'target/release/examples/game_instance'), '--measure', path], capture_output=True, text=True, check=True)
            values = {key: int(value) for key, value in re.findall(r'(\w+_us)=(\d+)', run.stdout)}
            values['max_rss_kib'] = int(re.search(r'RSS_KIB=(\d+)', run.stderr).group(1))
            values['output'] = run.stdout.strip()
            digests.add(re.search(r'digest=([0-9a-f]+)', run.stdout).group(1))
            samples.append(values)
        results.append(dict(path=path, samples=samples, medians={key: statistics.median(s[key] for s in samples) for key in samples[0] if key != 'output'}))
    assert len(digests) == 1, 'identical gameplay must retain identical final state'
print(json.dumps(dict(host=f'{platform.system()} {platform.machine()}', rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(), method='New workload, not a historical before baseline. Three separate release processes per path. Setup/admission excluded. Both time 100000 validated imports and 100000 checked exports (including returned Arc drop); then 100 same-input snapshot-inclusive updates with target observations. Scoped path retains one already scoped actor; qualified path explicitly reimports the same reference on every update. Different boundary validation work is intentional; ordinary tic logic never automatically imports references. Native wait4 peak RSS, no allocation counter or CPU/frequency isolation. Preserve all samples/outliers.', results=results), indent=2))
