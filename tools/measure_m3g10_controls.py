#!/usr/bin/env python3
"""Interleaved preserved/current production controls; advisory, never a timing gate."""
import argparse
import hashlib
import json
import platform
import re
import statistics
import tempfile
from pathlib import Path
from measure_stepping import ROOT, WORKLOADS, checked, fields

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser=argparse.ArgumentParser();parser.add_argument('baseline_directory',type=Path);args=parser.parse_args()
    if platform.system()!='Linux':raise SystemExit('Linux wait4 RSS observation required')
    directory=args.baseline_directory.resolve();manifest=json.loads((directory/'manifest.json').read_text())
    before=directory/'diagnostic_free';after=ROOT/'target/release/examples/diagnostic_free'
    expected=next(v for v in manifest['binaries']if v['name']=='diagnostic_free')
    assert digest(before)==expected['sha256'] and before.stat().st_size==expected['bytes']
    results=[]
    with tempfile.TemporaryDirectory(prefix='doom-interleaved-')as temp:
        src=Path(temp)/'rusage.c';launcher=Path(temp)/'rusage'
        src.write_text('''#define _GNU_SOURCE
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
int main(int n,char**v){int s;struct rusage u;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(v[1],v+1);_exit(127);}if(wait4(p,&s,0,&u)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",u.ru_maxrss);return WIFEXITED(s)?WEXITSTATUS(s):128+WTERMSIG(s);}
''')
        checked(['cc','-O2','-Wall','-Wextra','-Werror',str(src),'-o',str(launcher)],capture_output=True)
        for label,profile in WORKLOADS:
            row=dict(label=label,profile=profile,mode='advance',before=[],after=[]);digests=set()
            for sample in range(3):
                for variant in (['before','after']if sample%2==0 else['after','before']):
                    binary=before if variant=='before'else after
                    command=[str(binary),'--measure',label,profile,'advance']
                    p=checked([str(launcher),*command],capture_output=True)
                    data=fields(next(v for v in p.stdout.splitlines()if v.startswith('stepping ')));digests.add(data['digest'])
                    row[variant].append(dict(sample=sample,command=command,ns=data['ns'],max_rss_kib=int(re.search(r'RSS_KIB=(\d+)',p.stderr)[1])))
            assert len(digests)==1
            row['digest']=digests.pop();row['medians']={variant:{k:statistics.median(v[k]for v in row[variant])for k in ['ns','max_rss_kib']}for variant in ['before','after']};results.append(row)
    print(json.dumps(dict(baseline_commit=manifest['commit'],baseline_tree=manifest['tree'],feature_commit=checked(['git','rev-parse','HEAD'],capture_output=True).stdout.strip(),before_sha256=digest(before),after_sha256=digest(after),rustc=checked(['rustc','-vV'],capture_output=True).stdout.strip(),host=platform.platform(),results=results,method='Three separate before and after processes per matched100-advance workload. Pair order before/after, after/before, before/after; retain all samples. Archived baseline binary hash/length verified against preserved manifest. Program interval excludes setup and final Snapshot; RSS is whole process. Allocation arrays were already measured in the complete matched batch. No CPU isolation, hard timing gate or universal speed claim.'),separators=(',',':')))
if __name__=='__main__':main()
