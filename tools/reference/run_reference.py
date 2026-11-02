#!/usr/bin/env python3
"""Run unchanged historical source/encoder commands with decoded raw inputs.

One private workspace per worker, copied (never hardlinked). No Git/history
or network access is needed after dependencies are built. Source --check remains
an actual comparison to immutable expected bytes, not a regeneration from Rust.
With `--jobs N` the suite runs N commands at a time in N private workspaces,
scheduled longest-first from the committed duration table
(`source_suite_durations.json`, measured once on an idle host; a command without
an entry is scheduled last); every command still only compares against committed
bytes, so the order cannot change any result. `--jobs 1` (the default) is the
historical sequential run."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from evidence_storage import ROOT, MANIFEST, codec

SUITE = ROOT/'tools/reference/source_suite.json'
DURATIONS = ROOT/'tools/reference/source_suite_durations.json'
FOLDERS = ['linuxdoom-1.10','tools/reference','tests/reference','tests/support','crates/doom-sim/src','crates/doom-types/src','fuzz/corpus']

def workspace(binary, directory):
    temp=Path(directory)
    for folder in FOLDERS:
        shutil.copytree(ROOT/folder,temp/folder,ignore=shutil.ignore_patterns('__pycache__'))
    subprocess.run([str(binary),'extract',str(ROOT),str(MANIFEST),str(temp)],check=True,cwd=ROOT)
    return temp

def execute(line, cwd):
    parts=shlex.split(line);env=os.environ.copy()
    while parts and '=' in parts[0] and not parts[0].startswith('-'):
        key,value=parts.pop(0).split('=',1);env[key]=value
    if parts[0] in ['python','python3']: parts[0]=sys.executable
    start=time.monotonic()
    result=subprocess.run(parts,cwd=cwd,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    return dict(command=line,exit=result.returncode,seconds=time.monotonic()-start,
                output_sha256=hashlib.sha256(result.stdout).hexdigest()),result.stdout.decode(errors='replace')

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--suite',action='store_true')
    p.add_argument('--jobs',type=int,default=1,help='private workspaces running suite commands concurrently')
    p.add_argument('--report',type=Path)
    p.add_argument('command',nargs=argparse.REMAINDER)
    args=p.parse_args()
    if args.suite and args.command: p.error('suite and command are mutually exclusive')
    if args.jobs<1: p.error('--jobs needs at least one worker')
    if args.suite:
        commands=json.loads(SUITE.read_text())
    else:
        command=args.command
        if command and command[0]=='--': command=command[1:]
        if not command: p.error('supply --suite or a script and its --check arguments')
        script=Path(command[0]).name
        allowed={p.name for p in (ROOT/'tools/reference').glob('*.py')}
        if script not in allowed or not (script.startswith('check_') or '--check' in command[1:] or script=='evidence_pack.py'):
            p.error('only existing check commands are accepted')
        commands=[shlex.join(['python3','tools/reference/'+script,*command[1:]])]
    binary=codec()
    jobs=min(args.jobs,len(commands)) if args.suite else 1
    durations=json.loads(DURATIONS.read_text()) if DURATIONS.exists() else {}
    # Longest first; ties and unknown durations keep the committed suite order.
    queue=sorted(range(len(commands)),key=lambda i:(-durations.get(commands[i],-1),i)) if jobs>1 else list(range(len(commands)))
    results=[None]*len(commands);lock=threading.Lock();failed=[];errors=[]
    def report():
        if args.report: args.report.write_text(json.dumps([r for r in results if r],indent=2)+'\n')
    def worker(temp):
        # Any exception here (a failed workspace, an unrunnable command) is a
        # failure of the whole run, never a skipped command.
        try:
            while True:
                with lock:
                    if failed or errors or not queue: return
                    i=queue.pop(0)
                line=commands[i];print('+ '+line,flush=True)
                r,output=execute(line,temp)
                with lock:
                    results[i]=r
                    # Every command's own output is kept in the log, as the sequential run printed it.
                    print(output,end='' if output.endswith('\n') else '\n',flush=True)
                    print(f"= {r['seconds']:.1f}s exit {r['exit']}: {line}",flush=True)
                    if r['exit']: failed.append(i)
                    report()
        except BaseException as error:
            with lock: errors.append(error)
            raise
    with tempfile.TemporaryDirectory(prefix='doom-source-evidence-') as directory:
        temps=[workspace(binary,Path(directory)/f'worker{k}') for k in range(jobs)]
        threads=[threading.Thread(target=worker,args=(t,)) for t in temps]
        for t in threads: t.start()
        for t in threads: t.join()
    if errors: raise SystemExit(f'worker failure: {errors[0]!r}')
    if failed: raise SystemExit(results[failed[0]]['exit'])
    if any(r is None for r in results): raise SystemExit('not every command ran')
    print(f'Passed {len(commands)} unchanged historical source/encoder commands with {jobs} worker(s)',flush=True)
if __name__=='__main__':main()
