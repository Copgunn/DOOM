#!/usr/bin/env python3
"""Linux/glibc host observations of the barrels-policy example (exploding barrels, the radius attack, 64 pickups) in the synthetic M3h9b rooms, not the real E1M1. No timing thresholds."""
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
WORKLOADS = ['cluster_chain', 'dense64_fight', 'range_patrol', 'liftroom_ride', 'dense64_lift']


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
    checked(['cargo', 'build', '-p', 'doom-sim', '--example', 'player_barrels', '--release', '--locked'])
    binary = ROOT / 'target/release/examples/player_barrels'
    with tempfile.TemporaryDirectory(prefix='doom-barrels-measure-') as scratch:
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
        workloads = WORKLOADS
        modes = MODES
        results = []
        for label in workloads:
            for mode in modes:
                rows, digests = [], set()
                for _ in range(samples):
                    output, rss = execute([binary, '--measure', label, mode])
                    row = fields(next(line for line in output.splitlines() if line.startswith('player_barrels ')))
                    digests.add(row['digest'])
                    rows.append({'ns': row['ns'], 'max_rss_kib': rss, 'calls': row['calls'], 'linked_at_start': row['linked_at_start'], 'peak_actors': row['peak_actors'],
                                 'explosions': row['explosions'], 'sight_blocked': row['sight_blocked'], 'damaged': row['damaged'],
                                 'damaged_peak_tic': row['damaged_peak_tic'], 'barrel_deaths': row['barrel_deaths'],
                                 'change_sector': row['change_sector'], 'change_sector_peak_tic': row['change_sector_peak_tic'],
                                 'thing_clips': row['thing_clips'], 'thing_clips_peak_tic': row['thing_clips_peak_tic']})
                assert len(digests) == 1, (label, mode, digests)
                results.append(dict(label=label, mode=mode, digest=digests.pop(), samples=rows,
                                    medians={key: statistics.median(r[key] for r in rows) for key in rows[0]}))
        # The final state of a workload is the same in every mode and sample.
        for label in workloads:
            assert len({r['digest'] for r in results if r['label'] == label}) == 1, label
        sources = [ROOT / 'crates/doom-sim/examples/player_barrels.rs', ROOT / 'tests/support/m3h9b.rs', ROOT / 'tests/support/m3h9.rs', ROOT / 'tools/measure_barrels.py']
        provenance = {'commit': checked(['git', 'rev-parse', 'HEAD'], capture_output=True).stdout.strip(),
                      'status': checked(['git', 'status', '--porcelain'], capture_output=True).stdout,
                      'rustc': checked(['rustc', '--version'], capture_output=True).stdout.strip(),
                      'host': platform.platform(), 'libc': platform.libc_ver(),
                      'source_sha256': {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in sources}}
        print(json.dumps(dict(provenance=provenance, smoke=args.check,
             method='Separate release processes per workload/mode; one full timeline per process in the synthetic M3h9b rooms (not the real E1M1), setup excluded: cluster_chain: the committed chain_reaction timeline (140 tics, seven explosions in three generations); dense64_fight: 1,200 tics in the dense64 room (64 pickups, 16 barrels, 6 monsters, 27 decorations), the player walking to the barrel rows and firing along them, then a fixed patrol with BT_USE; range_patrol: 1,200 tics of the same patrol without BT_ATTACK in the range hall with its barrel (no barrel damage); liftroom_ride: the committed M3h9 decorations_ride_lift timeline under the barrels policy (P_ChangeSector work of a lift); dense64_lift: the P_ChangeSector measurement at the linked-thing bound N = 129, in the M3h9 liftroom geometry with its THINGS replaced in memory (121 linked at spawn: the player, 8 monsters, 64 pickups, 32 decorations and 16 barrels, every per-kind bound full; the 8 drops that make 129 come only from kills, none here), 120 of them inside the blockbox of the lift sector, the player crossing the WR 88 line back and forth for 1,200 tics so the lift cycles under its riders (Rust-only, not C evidence). Per-tic work (explosions, candidates reaching P_CheckSight blocked and visible with the per-tic peak, barrel deaths, P_ChangeSector calls and PIT_ChangeSector thing clips with the per-tic peak) recorded from an unmeasured run of the same inputs. Native wait4 RSS includes setup and final digest. No CPU/frequency/host isolation, timing thresholds or universal performance claim.',
             results=results), separators=(',', ':')))


if __name__ == '__main__':
    main()
