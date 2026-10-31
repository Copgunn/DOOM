#!/usr/bin/env python3
"""Run unchanged historical source/encoder commands with decoded raw inputs.

One private workspace per invocation, copied (never hardlinked). No Git/history
or network access is needed after dependencies are built. Source --check remains
an actual comparison to immutable expected bytes, not a regeneration from Rust.
"""
import argparse
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from evidence_storage import ROOT, MANIFEST, codec

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--suite',action='store_true')
    p.add_argument('--report',type=Path)
    p.add_argument('command',nargs=argparse.REMAINDER)
    args=p.parse_args()
    if args.suite and args.command: p.error('suite and command are mutually exclusive')
    if args.suite:
        commands=json.loads((ROOT/'tools/reference/source_suite.json').read_text())
    else:
        command=args.command
        if command and command[0]=='--': command=command[1:]
        if not command: p.error('supply --suite or a script and its --check arguments')
        script=Path(command[0]).name
        allowed={p.name for p in (ROOT/'tools/reference').glob('*.py')}
        if script not in allowed or not (script.startswith('check_') or '--check' in command[1:] or script=='evidence_pack.py'):
            p.error('only existing check commands are accepted')
        commands=[shlex.join(['python3','tools/reference/'+script,*command[1:]])]
    binary=codec(); results=[]
    with tempfile.TemporaryDirectory(prefix='doom-source-evidence-') as directory:
        temp=Path(directory)
        for folder in ['linuxdoom-1.10','tools/reference','tests/reference','tests/support','crates/doom-sim/src','crates/doom-types/src','fuzz/corpus']:
            shutil.copytree(ROOT/folder,temp/folder,ignore=shutil.ignore_patterns('__pycache__'))
        subprocess.run([str(binary),'extract',str(ROOT),str(MANIFEST),str(temp)],check=True,cwd=ROOT)
        for line in commands:
            parts=shlex.split(line);env=os.environ.copy()
            while parts and '=' in parts[0] and not parts[0].startswith('-'):
                key,value=parts.pop(0).split('=',1);env[key]=value
            if parts[0] in ['python','python3']: parts[0]=sys.executable
            print('+ '+line,flush=True);start=time.monotonic()
            result=subprocess.run(parts,cwd=temp,env=env)
            results.append(dict(command=line,exit=result.returncode,seconds=time.monotonic()-start))
            if args.report: args.report.write_text(json.dumps(results,indent=2)+'\n')
            if result.returncode: raise SystemExit(result.returncode)
    print(f'Passed {len(results)} unchanged historical source/encoder commands',flush=True)
if __name__=='__main__':main()
