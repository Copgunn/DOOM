#!/usr/bin/env python3
"""Linux host measurements; stdout JSON is evidence, never a CI golden/threshold."""
import json,os,platform,re,statistics,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
subprocess.run(['cargo','build','-p','doom-sim','--example','map_extent','--release','--locked'],cwd=ROOT,check=True)
# A small native launcher keeps Python's pre-exec resident pages out of the
# measured child's high-water RSS. Measurement code is host tooling only.
launcher_dir=tempfile.TemporaryDirectory(prefix='doom-map-extent-rusage-')
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
for index,legacy in [(0,True),(0,False),(3,False),(7,False),(9,False),(10,False),(20,False),(21,False),(22,False),(27,False),(29,False)]:
    samples=[]
    for _ in range(3):
        cmd=[str(ROOT/'target/release/examples/map_extent'),'--measure',str(index)]+(['profile18'] if legacy else [])
        child=subprocess.run([str(launcher),*cmd],capture_output=True,text=True,check=True)
        text=child.stdout
        rss=int(re.search(r'RSS_KIB=(\d+)',child.stderr).group(1))
        values={key:int(value) for key,value in re.findall(r'(parse100_us|prepare100_us|collision1000_us|candidates|sight1000_us|sight_visits|sight_work|step100_us|lines|sectors|cells|references|prepared_bytes)=(\d+)',text)}
        values['max_rss_kib']=rss
        values['output']=text.strip();samples.append(values)
    numeric={key:statistics.median(sample[key] for sample in samples) for key in samples[0] if key!='output'}
    results.append(dict(map=index,profile18=legacy,samples=samples,medians=numeric))
print(json.dumps(dict(host=f'{platform.system()} {platform.machine()}',rustc=subprocess.check_output(['rustc','--version'],text=True).strip(),method='three separate release processes/case; 100 separate parses and 100 complete Simulation preparations including geometry/sight validation, BLOCKMAP/topology preparation and canonical map cache, excluding parsing, 1000 immutable collision queries, 1000 sight queries, 100 tics including snapshots; native wait4 launcher peak RSS in KiB includes the Rust process and excludes Python launch pages; no performance acceptance threshold',results=results),indent=2))
