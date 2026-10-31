#!/usr/bin/env python3
"""Three-process timing/allocation measurements of existing private weapon stages.

For baseline reproduction copy the identical committed test harness and cfg(test)
hook/fixture visibility into a detached baseline worktree. Engine code stays at
its original commit. No benchmark mutation of authoritative gameplay is performed.
"""
import json,subprocess,os,hashlib,re,sys
from pathlib import Path
import argparse,platform
parser=argparse.ArgumentParser(description='Private legacy preview/commit cost samples; no production API')
parser.add_argument('--source-root',type=Path,default=Path(__file__).resolve().parents[1])
parser.add_argument('--target-dir',type=Path)
parser.add_argument('--logs',type=Path,required=True)
a=parser.parse_args();root=a.source_root.resolve();target=(a.target_dir or root/'target').resolve();out=a.logs.resolve();out.mkdir(parents=True,exist_ok=True)
if platform.system()!='Linux' or platform.libc_ver()[0]!='glibc':raise SystemExit('Linux/glibc only')
base=out
env=os.environ.copy();env['CARGO_TARGET_DIR']=str(target);env['DOOM_WEAPON_STAGE_COST']='1';env['DOOM_SOURCE_ROOT']=str(root)
p=subprocess.run(['cargo','test','-p','doom-sim','--release','--lib','legacy_weapon_stages','--no-run','--locked','--message-format=json'],cwd=root,env=env,text=True,capture_output=True,check=True)
artifacts=[json.loads(line)for line in p.stdout.splitlines()if line.startswith('{')];binary=next(a['executable']for a in artifacts if a.get('reason')=='compiler-artifact' and a.get('executable'))
probe=base/'stage-alloc.so';subprocess.run(['cc','-std=c11','-O2','-Wall','-Wextra','-Werror','-fPIC','-shared',str(root/'tools/measurement/alloc_probe.c'),'-o',str(probe)],check=True)
results=[]
controls={}
subprocess.run(['cargo','build','-p','doom-sim','--example','diagnostic_free','--release','--locked'],cwd=root,env=env,check=True,stdout=sys.stderr)
for flag in ['--control','--probe']:
 calibration=env.copy();calibration['LD_PRELOAD']=str(probe)
 result=subprocess.run([str(target/'release/examples/diagnostic_free'),flag],cwd=root,env=calibration,text=True,capture_output=True,check=True)
 sys.path.insert(0,str(root/'tools'));from measure_stepping import ALLOC,ALLOC_KEYS
 counts=[dict(zip(ALLOC_KEYS,map(int,m)))for m in ALLOC.findall(result.stderr)];assert len(counts)==1;controls[flag]=counts[0]
assert controls['--control']==dict(zip(ALLOC_KEYS,[0,0,0,0,0]))
assert controls['--probe']==dict(zip(ALLOC_KEYS,[1,0,4096,1,0]))
for instrument in [False,True]:
 for sample in range(3):
  childenv=env.copy()
  if instrument:childenv['LD_PRELOAD']=str(probe)
  log=out/f'weapon-stages-{instrument}-{sample}.log'
  with log.open('wb')as output:
   child=subprocess.Popen([binary,'legacy_weapon_stages','--nocapture','--test-threads=1'],cwd=root,env=childenv,stdout=output,stderr=subprocess.STDOUT)
   _,status,usage=os.wait4(child.pid,0);child.returncode=os.waitstatus_to_exitcode(status)
  assert child.returncode==0,log
  text=log.read_text();rows=[]
  for m in re.finditer(r'weapon_stage case=(\d+) stage=(\w+) calls=100 ns=(\d+) digest=([0-9a-f]+)',text):rows.append(dict(case=int(m[1]),stage=m[2],ns=int(m[3]),digest=m[4]))
  sys.path.insert(0,str(root/'tools'));from measure_stepping import ALLOC,ALLOC_KEYS
  allocations=[dict(zip(ALLOC_KEYS,map(int,m)))for m in ALLOC.findall(text)]
  assert len(rows)==8
  if instrument:assert len(allocations)==8 and all(a['failures']==0 for a in allocations)
  results.append(dict(instrumented=instrument,sample=sample,rows=rows,allocation_samples=allocations,max_rss_kib=usage.ru_maxrss,log_sha256=hashlib.sha256(log.read_bytes()).hexdigest()))
value=dict(results=results,controls=controls,host=platform.platform(),harness_sha256=hashlib.sha256((root/'crates/doom-sim/src/weapon_stage_cost_tests.rs').read_bytes()).hexdigest(),commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),rustc=subprocess.check_output(['rustc','-vV'],text=True),method='Same private test harness on pinned baseline and after. Cases0/5/23/24: pistol/shotgun/fist/chaingun actual firing boundary found outside timing.100 independent worlds prepared outside each preview or already-preflighted commit interval; weapon stage only, not whole advance.3 timing and3 allocation processes, every process reports all8 stages. Whole-process native wait4 RSS includes setup/evidence. Baseline adds only cfg(test) module and shared fixture visibility; after includes identical instrumentation and zstd test loading. No CPU isolation or page-cache resets.')
(out/'weapon-stages.json').write_text(json.dumps(value,indent=2)+'\n')
print(json.dumps(value,indent=2),flush=True)
