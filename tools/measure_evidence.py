#!/usr/bin/env python3
"""Separate cold raw-file versus zstd/integrity measurement, never gameplay time."""
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import sys
import tempfile
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
from measure_stepping import ALLOC,ALLOC_KEYS,checked,fields
PROGRAM=r'''
use doom_test_evidence::{Manifest,load,read_bounded,sha256};
use std::{path::Path,time::Instant,io::Write};
fn main()->Result<(),Box<dyn std::error::Error>> {
 let a:Vec<_>=std::env::args().collect();let mode=a.get(1).ok_or("mode")?;let root=Path::new(a.get(2).ok_or("root")?);
 let manifest=Manifest::parse(include_str!("@MANIFEST@"))?;
 let mut out=std::io::stdout().lock();writeln!(out,"DOOM_ALLOC_BEGIN")?;out.flush()?;
 let start=Instant::now();let mut hash=14695981039346656037u64;let mut total=0;
 for entry in &manifest.entries {
  let bytes=match mode.as_str(){"raw"=>read_bounded(&root.join(&entry.original_path),entry.original_length)?,"zstd"=>load(root,entry)?,_=>return Err("mode".into())};
  if bytes.len()!=entry.original_length || sha256(&bytes)!=entry.original_sha256 {return Err("reference identity".into());}
  total+=bytes.len();for byte in sha256(&bytes).bytes(){hash=(hash^u64::from(byte)).wrapping_mul(1099511628211);}
 }
 let ns=start.elapsed().as_nanos();writeln!(out,"DOOM_ALLOC_END")?;out.flush()?;
 writeln!(out,"evidence mode={mode} files={} bytes={total} ns={ns} digest={hash:016x}",manifest.entries.len())?;Ok(())
}
'''
def main():
 if len(sys.argv)!=2:raise SystemExit('usage: measure_evidence.py RAW_BASELINE_ROOT')
 raw=Path(sys.argv[1]).resolve()
 if platform.system()!='Linux' or platform.libc_ver()[0]!='glibc':raise SystemExit('Linux/glibc measurements only')
 checked(['cargo','build','-p','doom-test-evidence','--release','--locked'])
 checked(['cargo','build','-p','doom-sim','--example','diagnostic_free','--release','--locked'])
 with tempfile.TemporaryDirectory(prefix='doom-evidence-measure-')as directory:
  temp=Path(directory);source=temp/'evidence.rs';source.write_text(PROGRAM.replace('@MANIFEST@',str(ROOT/'tests/reference/zstd_manifest.json')));binary=temp/'evidence'
  library=max((ROOT/'target/release/deps').glob('libdoom_test_evidence-*.rlib'),key=lambda p:p.stat().st_mtime)
  checked(['rustc','--edition=2024','-C','opt-level=3','-L',str(ROOT/'target/release/deps'),'--extern','doom_test_evidence='+str(library),str(source),'-o',str(binary)])
  probe=temp/'alloc.so';checked(['cc','-std=c11','-O2','-Wall','-Wextra','-Werror','-fPIC','-shared',str(ROOT/'tools/measurement/alloc_probe.c'),'-o',str(probe)])
  # The same native wait4 wrapper used by established matched measurements.
  launcher_source=temp/'rusage.c';launcher=temp/'rusage'
  launcher_source.write_text('#define _GNU_SOURCE\n#include <sys/resource.h>\n#include <sys/wait.h>\n#include <unistd.h>\n#include <stdio.h>\nint main(int n,char**v){int s;struct rusage u;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(v[1],v+1);_exit(127);}if(wait4(p,&s,0,&u)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",u.ru_maxrss);return WIFEXITED(s)?WEXITSTATUS(s):128+WTERMSIG(s);}\n')
  checked(['cc','-O2','-Wall','-Wextra','-Werror',str(launcher_source),'-o',str(launcher)])
  controls={}
  for flag in ['--control','--probe']:
   env=os.environ.copy();env['LD_PRELOAD']=str(probe)
   result=checked([str(ROOT/'target/release/examples/diagnostic_free'),flag],env=env,capture_output=True)
   counts=[dict(zip(ALLOC_KEYS,map(int,m)))for m in ALLOC.findall(result.stderr)]
   assert len(counts)==1;controls[flag]=counts[0]
  assert controls['--control']==dict(zip(ALLOC_KEYS,[0,0,0,0,0]))
  assert controls['--probe']==dict(zip(ALLOC_KEYS,[1,0,4096,1,0]))
  results=[];digests=set()
  for mode,root in [('raw',raw),('zstd',ROOT)]:
   samples=[];allocations=[]
   for _ in range(3):
    for instrument in [False,True]:
     env=os.environ.copy()
     if instrument:env['LD_PRELOAD']=str(probe)
     p=checked([str(launcher),str(binary),mode,str(root)],env=env,capture_output=True)
     row=fields(next(line for line in p.stdout.splitlines()if line.startswith('evidence ')));digests.add(row['digest'])
     if instrument:
      counts=[dict(zip(ALLOC_KEYS,map(int,m)))for m in ALLOC.findall(p.stderr)];assert len(counts)==1 and counts[0]['failures']==0;allocations.append(counts[0])
     else:samples.append(dict(ns=row['ns'],max_rss_kib=int(re.search(r'RSS_KIB=(\d+)',p.stderr)[1])))
   results.append(dict(mode=mode,samples=samples,allocation_samples=allocations,digest=row['digest'],medians={k:statistics.median(s[k]for s in samples)for k in samples[0]}))
  assert len(digests)==1
 print(json.dumps(dict(results=results,controls=controls,rustc=checked(['rustc','-vV'],capture_output=True).stdout,host=platform.platform(),program_sha256=hashlib.sha256(PROGRAM.encode()).hexdigest(),manifest_sha256=hashlib.sha256((ROOT/'tests/reference/zstd_manifest.json').read_bytes()).hexdigest(),method='Three separate release timing/RSS and three allocation processes per path. Reads all19 files with decoded identity checks, then drops each byte buffer. Raw root is actual pinned Git baseline checkout, same SHA/length. Both paths use one common executable; zstd additionally validates compressed identity and frame, then decodes. Manifest parsing excluded; directory traversal/read/integrity included. No cold page-cache reset; OS cache and shared-host variance apply. This filesystem microbenchmark is NOT the former include_str test architecture and NOT gameplay. Requested allocations cumulative, not peak live memory or leak coverage.'),indent=2))
if __name__=='__main__':main()
