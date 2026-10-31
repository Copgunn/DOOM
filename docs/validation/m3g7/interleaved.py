import hashlib,json,platform,re,statistics,subprocess,sys
from pathlib import Path
root=Path('/workspace/doom-tools/m3g7');after=Path('/workspace/DOOM');before=root/'baseline-review'
sys.path.insert(0,str(after/'tools'))
from measure_stepping import fields
source=root/'interleaved-rusage.c';launcher=root/'interleaved-rusage'
source.write_text('''#define _GNU_SOURCE
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
int main(int n,char **v){int s;struct rusage u;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(v[1],v+1);_exit(127);}if(wait4(p,&s,0,&u)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",u.ru_maxrss);return WIFEXITED(s)?WEXITSTATUS(s):128+WTERMSIG(s);}
''')
subprocess.run(['cc','-O2','-Wall','-Wextra','-Werror',str(source),'-o',str(launcher)],check=True)
work=[('player','original_player','demon_source','step'),('player-demon-melee','player_demon_melee','moving_pressure','step'),('commands','player_commands','forward','step'),('player','original_player','idle','advance'),('player-demon-melee','player_demon_melee','prior_impulse','advance')]
provenance={}
for phase,path in [('before',before),('after',after)]:
 def git(*args):return subprocess.check_output(['git',*args],cwd=path,text=True).strip()
 assert not git('status','--porcelain')
 provenance[phase]=dict(commit=git('rev-parse','HEAD'),tree=git('rev-parse','HEAD^{tree}'),status='',sources={p:hashlib.sha256((path/p).read_bytes()).hexdigest() for p in ['crates/doom-sim/examples/original_player.rs','crates/doom-sim/examples/player_commands.rs','crates/doom-sim/examples/player_demon_melee.rs']})
assert provenance['before']['sources']==provenance['after']['sources']
rows=[]
for family,example,label,mode in work:
 row=dict(family=family,label=label,mode=mode,before=[],after=[],process_order=[])
 for i in range(3):
  order=['before','after'] if i%2==0 else ['after','before'];row['process_order'].append(order)
  for phase in order:
   binary=(root/'interleaved-target-before/release/examples'/example) if phase=='before' else (after/'target/release/examples'/example)
   command=[str(launcher),str(binary),'--measure',label,mode]
   p=subprocess.run(command,capture_output=True,text=True,check=True)
   metrics=fields(next(line for line in p.stdout.splitlines() if 'ns=' in line and 'digest=' in line))
   row[phase].append(dict(ns=metrics['ns'],max_rss_kib=int(re.search(r'RSS_KIB=(\d+)',p.stderr).group(1)),digest=metrics['digest'],stdout=p.stdout.strip()))
 assert len({s['digest'] for phase in ['before','after'] for s in row[phase]})==1
 row['medians']={phase:{key:statistics.median(s[key] for s in row[phase])for key in ['ns','max_rss_kib']}for phase in ['before','after']}
 row['delta_percent']=100*(row['medians']['after']['ns']/row['medians']['before']['ns']-1);rows.append(row)
 print(family,label,mode,row['delta_percent'],flush=True)
result=dict(provenance=provenance,rustc=subprocess.check_output(['/workspace/doom-tools/run','rustc','--version'],text=True).strip(),host=platform.platform(),method='Three paired separate release processes per selected outlier workload, order before/after then after/before then before/after. Same unchanged example source, commands, toolchain and wait4 RSS launcher as full batch. Baseline build in detached worktree; current build in main workspace; no other validation/compiler/fuzz work concurrent. Not CPU-isolated; binary build paths differ. Timing/RSS only; full-batch allocation samples retained separately. All full-batch observations remain, including original outliers. No universal timing claim.',results=rows)
(root/'interleaved.json').write_text(json.dumps(result,indent=2)+'\n')
