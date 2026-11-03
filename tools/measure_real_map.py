#!/usr/bin/env python3
"""Linux/glibc host observations of the real-map-scale example. No timing thresholds."""
import argparse
import hashlib
import json
import platform
import re
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODES = ['advance', 'step', 'capture']
WORKLOADS = ['town_patrol', 'town_fight', 'gallery_shot', 'wide_sight']
ROOMS = ['town', 'gallery', 'wide']


def fields(line):
    return {key: int(value) if key != 'digest' and value.isdecimal() else value
            for key, value in re.findall(r'(\w+)=([^\s]+)', line)}


def checked(args, **kwargs):
    result = subprocess.run(args, cwd=ROOT, text=True, **kwargs)
    if result.returncode:
        if result.stdout:
            sys.stderr.write(result.stdout)
        if result.stderr:
            sys.stderr.write(result.stderr)
        result.check_returncode()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='small advisory CI smoke; no speed budget')
    parser.add_argument('--samples', type=int, default=11, help='interleaved process samples per workload/mode')
    args = parser.parse_args()
    if platform.system() != 'Linux' or platform.libc_ver()[0] != 'glibc':
        parser.error('wait4 measurements require Linux/glibc; use the asserted example on other hosts')
    checked(['cargo', 'build', '-p', 'doom-sim', '--example', 'player_real_map', '--release', '--locked'])
    binary = ROOT / 'target/release/examples/player_real_map'
    with tempfile.TemporaryDirectory(prefix='doom-real-map-measure-') as scratch:
        scratch = Path(scratch)
        source = scratch / 'rusage.c'
        source.write_text('''#define _GNU_SOURCE
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
int main(int n,char **v){int status;struct rusage use;pid_t p;if(n<2)return 2;p=fork();if(p<0)return 3;if(!p){execv(v[1],v+1);_exit(127);}if(wait4(p,&status,0,&use)<0)return 4;fprintf(stderr,"RSS_KIB=%ld\\n",use.ru_maxrss);return WIFEXITED(status)?WEXITSTATUS(status):128+WTERMSIG(status);}
''')
        launcher = scratch / 'rusage'
        checked(['cc', '-O2', '-Wall', '-Wextra', '-Werror', str(source), '-o', str(launcher)])

        def execute(command):
            result = checked([str(launcher), *map(str, command)], capture_output=True)
            rss = int(re.search(r'RSS_KIB=(\d+)', result.stderr).group(1))
            return result.stdout, rss

        samples = 1 if args.check else args.samples
        workloads = ['town_patrol'] if args.check else WORKLOADS
        modes = ['advance'] if args.check else MODES
        results = []
        for label in workloads:
            for mode in modes:
                rows, digests = [], set()
                for _ in range(samples):
                    output, rss = execute([binary, '--measure', label, mode])
                    row = fields(next(line for line in output.splitlines() if line.startswith('player_real_map ')))
                    digests.add(row['digest'])
                    rows.append({'ns': row['ns'], 'max_rss_kib': rss, 'calls': row['calls'], 'peak_actors': row['peak_actors']})
                assert len(digests) == 1, (label, mode, digests)
                results.append(dict(label=label, mode=mode, digest=digests.pop(), samples=rows,
                                    medians={key: statistics.median(r[key] for r in rows) for key in rows[0]}))
        admissions = []
        for room in (['town'] if args.check else ROOMS):
            rows, digests = [], set()
            for _ in range(samples):
                output, rss = execute([binary, '--admit', room])
                row = fields(next(line for line in output.splitlines() if line.startswith('player_real_map ')))
                digests.add(row['digest'])
                rows.append({'ns': row['ns'], 'max_rss_kib': rss, 'linedefs': row['linedefs'], 'sectors': row['sectors'], 'cells': row['cells']})
            assert len(digests) == 1, (room, digests)
            admissions.append(dict(room=room, digest=digests.pop(), samples=rows,
                                   medians={key: statistics.median(r[key] for r in rows) for key in rows[0]}))
        sources = [ROOT / 'crates/doom-sim/examples/player_real_map.rs', ROOT / 'tests/support/m3h6.rs', ROOT / 'tools/measure_real_map.py']
        provenance = {'commit': checked(['git', 'rev-parse', 'HEAD'], capture_output=True).stdout.strip(),
                      'status': checked(['git', 'status', '--porcelain'], capture_output=True).stdout,
                      'rustc': checked(['rustc', '--version'], capture_output=True).stdout.strip(),
                      'host': platform.platform(), 'libc': platform.libc_ver(),
                      'source_sha256': {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in sources}}
        print(json.dumps(dict(provenance=provenance, smoke=args.check,
             method='Separate release processes per workload/mode; one full committed scripted timeline per process, setup excluded; --admit times the policy admission and preparation of one committed room with parsing excluded. Native wait4 RSS includes setup and final digest. No CPU/frequency/host isolation, timing thresholds or universal performance claim.',
             results=results, admissions=admissions), separators=(',', ':')))


if __name__ == '__main__':
    main()
