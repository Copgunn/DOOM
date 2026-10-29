#!/usr/bin/env python3
"""Linux host measurements; stdout JSON is evidence, never a CI golden/threshold."""
import json,os,platform,re,statistics,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
subprocess.run(['cargo','build','-p','doom-sim','--example','demon_expanded_map','--release','--locked'],cwd=ROOT,check=True)
# A small native launcher keeps Python's pre-exec resident pages out of the
# measured child's high-water RSS. Measurement code is host tooling only.
launcher_dir=tempfile.TemporaryDirectory(prefix='doom-demon-expanded-rusage-')
launcher=Path(launcher_dir.name)/'rusage'
source=launcher.with_suffix('.c')
source.write_text(r"""
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
int main(int argc,char** argv){int status;struct rusage use;pid_t p;if(argc<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(argv[1],argv+1);_exit(127);}if(wait4(p,&status,0,&use)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\n",use.ru_maxrss);return WIFEXITED(status)?WEXITSTATUS(status):128+WTERMSIG(status);}
""")
subprocess.run(['cc','-O2',str(source),'-o',str(launcher)],check=True)
results=[]
for label,profile in [('small_control',17),('small_control',20),('long_lifecycle',19),('long_lifecycle',20),('four_owners',20),('wide_map_lifecycle',20),('expanded_sight_dag',20),('line_table',20),('high_cell_demon',20)]:
    samples=[]
    for _ in range(3):
        cmd=[str(ROOT/'target/release/examples/demon_expanded_map'),'--measure',label,str(profile)]
        child=subprocess.run([str(launcher),*cmd],capture_output=True,text=True,check=True)
        text=child.stdout
        rss=int(re.search(r'RSS_KIB=(\d+)',child.stderr).group(1))
        values={key:(int(value) if value!='-' else None) for key,value in re.findall(r'(parse100_us|prepare100_us|spawn100_us|collision1000_us|candidates|sight1000_us|visits|chase100_us|step100_us|snapshot100_us|snapshot_bytes|lines|sectors|cells|references|prepared_bytes)=(\d+|-)',text)}
        values['max_rss_kib']=rss
        values['output']=text.strip();samples.append(values)
    numeric={key:statistics.median(sample[key] for sample in samples) if samples[0][key] is not None else None for key in samples[0] if key!='output'}
    results.append(dict(case=label,profile=profile,samples=samples,medians=numeric))
print(json.dumps(dict(host=f'{platform.system()} {platform.machine()}',rustc=subprocess.check_output(['rustc','--version'],text=True).strip(),method='Three separate Linux release processes per source-backed input/profile; 100 parses and full constructors timed separately (ownership passed, no parsing/clone in prepare timer); 100 THING batches timed after construction for typed profiles only; 1000 immutable collision and sight queries; 100 direct Chase calls each on fresh same-input targets for typed profiles only; 100 script updates including external scripted damage and Snapshot construction, then 100 complete canonical encodings. Native wait4 RSS includes benchmark/Rust allocator/event overhead, not Python launch pages. Profile17 prepared metrics unavailable (null); profile19 has no typed batch/Chase (zero timer is not claimed execution). Only small_control17/20 are like-for-like gameplay inputs; other sizes/profiles are different workloads. No performance threshold or universal regression claim',results=results),indent=2))
