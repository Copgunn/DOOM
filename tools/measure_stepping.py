#!/usr/bin/env python3
"""Linux/glibc host observations. No timing thresholds or engine instrumentation."""
import argparse
import hashlib
import json
import os
import platform
import re
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODES = ['step', 'advance', 'advance_snapshot', 'capture', 'encode']
WORKLOADS = [('small_control', '17'), ('small_control', '20'),
             ('long_lifecycle', '20'), ('four_owners', '20'),
             ('wide_map_lifecycle', '20'), ('expanded_sight_dag', '20'),
             ('line_table', '20'), ('high_cell_demon', '20'),
             ('canonical_small', 'original'), ('actor_capacity', 'original')]
ALLOC = re.compile(r'ALLOC calls=(\d+) reallocs=(\d+) requested_bytes=(\d+) frees=(\d+) failures=(\d+)')
ALLOC_KEYS = ['calls', 'reallocs', 'requested_bytes', 'frees', 'failures']


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
    args = parser.parse_args()
    if platform.system() != 'Linux' or platform.libc_ver()[0] != 'glibc':
        parser.error('allocation and wait4 measurements require Linux/glibc; use the asserted example on other hosts')
    checked(['cargo', 'build', '-p', 'doom-sim', '--example', 'diagnostic_free', '--release', '--locked'])
    compiled = checked(['cargo', 'test', '-p', 'doom-sim', '--lib', '--release', '--locked', '--no-run', '--message-format=json'], capture_output=True)
    artifacts = [json.loads(line) for line in compiled.stdout.splitlines() if line.startswith('{')]
    test_binary = next(item['executable'] for item in artifacts
                       if item.get('reason') == 'compiler-artifact' and item.get('executable') and item['target']['name'] == 'doom_sim')
    binary = ROOT / 'target/release/examples/diagnostic_free'
    with tempfile.TemporaryDirectory(prefix='doom-stepping-measure-') as scratch:
        scratch = Path(scratch)
        library = scratch / 'alloc_probe.so'
        checked(['cc', '-std=c11', '-O2', '-Wall', '-Wextra', '-Werror', '-fPIC', '-shared', str(ROOT / 'tools/measurement/alloc_probe.c'), '-o', str(library)])
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

        def execute(command, probe=False, transaction=False):
            env = os.environ.copy()
            if probe:
                env['LD_PRELOAD'] = str(library)
            if transaction:
                env['DOOM_STEPPING_COST'] = '1'
            result = checked([str(launcher), *map(str, command)], env=env, capture_output=True)
            rss = int(re.search(r'RSS_KIB=(\d+)', result.stderr).group(1))
            metrics = [dict(zip(ALLOC_KEYS, map(int, row))) for row in ALLOC.findall(result.stderr)]
            assert all(row['failures'] == 0 for row in metrics), metrics
            return result.stdout, rss, metrics

        controls = {}
        for flag in ['--control', '--probe']:
            _, _, rows = execute([binary, flag], probe=True)
            assert len(rows) == 1, 'allocator markers were not intercepted'
            controls[flag] = rows[0]
        assert controls['--control'] == dict(zip(ALLOC_KEYS, [0, 0, 0, 0, 0])), controls
        assert controls['--probe'] == dict(zip(ALLOC_KEYS, [1, 0, 4096, 1, 0])), controls
        results = []
        for label, profile in ([('small_control', '20')] if args.check else WORKLOADS):
            digests = set()
            for mode in MODES:
                command = [binary, '--measure', label, profile, mode]
                samples, allocations = [], []
                for _ in range(3):
                    output, rss, _ = execute(command)
                    row = fields(next(line for line in output.splitlines() if line.startswith('stepping ')))
                    digests.add(row['digest'])
                    samples.append({'ns': row['ns'], 'max_rss_kib': rss, 'bytes': row['bytes']})
                    _, _, rows = execute(command, probe=True)
                    assert len(rows) == 1, rows
                    allocations.append(rows[0])
                results.append(dict(label=label, profile=profile, mode=mode, digest=row['digest'], samples=samples,
                                    medians={key: statistics.median(row[key] for row in samples) for key in samples[0]},
                                    allocation_samples=allocations))
            assert len(digests) == 1, (label, profile, digests)
            selected = {row['mode']: row for row in results if row['label'] == label and row['profile'] == profile}
            for i in range(3):
                assert selected['advance']['allocation_samples'][i]['calls'] < selected['step']['allocation_samples'][i]['calls']
                assert selected['step']['allocation_samples'][i] == selected['advance_snapshot']['allocation_samples'][i]
            assert selected['capture']['allocation_samples'][0]['requested_bytes'] > 0
            assert selected['encode']['allocation_samples'][0]['requested_bytes'] > 0

        transaction_samples = []
        repetitions = 1 if args.check else 3
        command = [test_binary, 'demon_expanded_map::transaction_cost_tests::projection_mutation_is_isolated_and_component_costs_are_observable', '--exact', '--nocapture', '--test-threads=1']
        for sample in range(repetitions):
            output, rss, _ = execute(command, transaction=True)
            records = [fields(line) for line in output.splitlines() if line.startswith('transaction ')]
            assert len(records) == 15, output
            probe_output, _, allocations = execute(command, probe=True, transaction=True)
            probe_records = [fields(line) for line in probe_output.splitlines() if line.startswith('transaction ')]
            assert len(allocations) == len(probe_records) == len(records)
            for row, allocation in zip(records, allocations):
                if row['operation'] == 'staging':
                    assert allocation['calls'] == row['calls'] * 2
                    assert allocation['reallocs'] == 0
                    assert allocation['requested_bytes'] == row['calls'] * (row['slot_bytes'] + row['spatial_reserved_bytes'])
                transaction_samples.append(dict(sample=sample, **row, allocation=allocation, max_rss_kib=rss))

        sources = sorted((ROOT / 'crates/doom-sim/src').rglob('*.rs')) + [ROOT / 'tests/support/m3g1.rs', ROOT / 'tests/support/m3g1_profiles.rs', ROOT / 'crates/doom-sim/examples/diagnostic_free.rs', ROOT / 'tools/measure_stepping.py', ROOT / 'tools/measurement/alloc_probe.c']
        provenance = {'commit': checked(['git', 'rev-parse', 'HEAD'], capture_output=True).stdout.strip(),
                      'tree': checked(['git', 'rev-parse', 'HEAD^{tree}'], capture_output=True).stdout.strip(),
                      'status': checked(['git', 'status', '--porcelain'], capture_output=True).stdout,
                      'rustc': checked(['rustc', '--version'], capture_output=True).stdout.strip(),
                      'host': platform.platform(), 'libc': platform.libc_ver(),
                      'cc': checked(['cc', '--version'], capture_output=True).stdout.splitlines()[0],
                      'source_sha256': {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in sources}}
        print(json.dumps(dict(provenance=provenance, smoke=args.check, controls=controls,
             method='Three separate release processes per workload/mode. Setup excluded; update timers include identical scripted inputs and destruction of returned snapshots. Capture uses frozen state after100 advances; encode uses a prebuilt snapshot. These frozen components are not additive to evolving update medians. Allocation samples are separate instrumented processes, NOT timing/RSS samples. Linux/glibc interposition counts successful malloc/calloc/aligned requests, realloc requests, cumulative requested bytes and explicit nonnull frees in the marked workload thread; allocator internals/direct mappings/other threads/peak-live-heap/leaks are not measured. Native wait4 RSS includes setup, allocator, events and final verification capture/encoding outside timer. Private component samples include test-harness RSS, are100 calls on frozen initial state and are separate from evolving update work. Staging bytes include vacant logical slots; spatial allocation reserves max_slots while copying only current links. No CPU/frequency/host isolation, timing thresholds or universal performance claim.',
             results=results, transaction_samples=transaction_samples), separators=(',', ':')))


if __name__ == '__main__':
    main()
