# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

"""Experimental process shards around the unchanged authenticated law runner.

Whole modules stay together. Each worker owns its contexts, unittest fixtures,
temporary files and report. No derived data or validation result is cached.
"""
import argparse
import concurrent.futures
import contextlib
import hashlib
import json
import os
from pathlib import Path
import resource
import subprocess
import sys
import threading
import time
import unittest
from unittest import mock

ORIGINAL_RUNNER = {
    'tools/compact-rust-backend/historical_law_dispatcher.py': 'ea4bea54b59536956442b9e99841806190716064f111b30774fe0aad4a02c275',
    'tools/compact-rust-backend/historical_law_runtime.json': '1b3a5759c6d2e92b1d8ac88aceca5aae910a771087fbe593481bdce890c4fdc7',
}


def peak_bytes(kind):
    return resource.getrusage(kind).ru_maxrss * (1 if sys.platform == "darwin" else 1024)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def initialize(args):
    authenticate_runner(args.root)
    sys.path.insert(0, str(args.root / 'tools/compact-rust-backend'))
    import historical_law_dispatcher as runner
    if Path(runner.__file__).resolve() != args.root / 'tools/compact-rust-backend/historical_law_dispatcher.py':
        raise ValueError('foreign original runner import')
    runner.ROOT = args.root
    runner.ARCHIVE_ROOT = args.archive
    runner.GRAPH_ROOT = args.graph
    descriptor = runner.load_descriptor(args.root)
    runner.authenticate_sources(args.root, descriptor)
    runner.prepare_guarded_point_artifacts(args.root, descriptor, args.graph)
    from archived_conformance_test_support import authenticate_archive
    authenticate_archive(args.archive, descriptor['archived_conformance']['old_source_fingerprints'])
    import test_guarded_point_zkir
    test_guarded_point_zkir.ARTIFACTS = args.graph
    suites, identities = runner.load_suites(args.root, descriptor)
    return runner, descriptor, suites, identities


def authenticate_runner(root):
    for relative, expected in ORIGINAL_RUNNER.items():
        if digest(root / relative) != expected:
            raise ValueError('original runner source changed: ' + relative)


def join_reports(reports, planned, jobs):
    """Require one successful execution of every selected original identity."""
    actual = []
    if len(reports) != len(jobs):
        raise ValueError('worker report cardinality differs')
    for report, job in zip(reports, jobs):
        if (report['passed'] is not True or report['status'] != 'completed'
                or type(report['failures']) is not int or report['failures'] != 0
                or type(report['errors']) is not int or report['errors'] != 0
                or report['skipped'] != [] or report['not_run_case_ids'] != {}
                or report['actual_case_ids_match_plan'] is not True
                or report['planned_case_ids'] != job['group_case_ids']
                or set(report['groups']) != set(job['group_case_ids'])
                or report['modules'] != job['modules']):
            raise ValueError('worker failed or skipped original laws')
        for group, expected in job['group_case_ids'].items():
            outcome = report['groups'][group]
            if (outcome['actual_case_ids'] != expected or type(outcome['tests_run']) is not int
                    or outcome['tests_run'] != len(expected) or outcome['failures'] != []
                    or outcome['errors'] != [] or outcome['skipped'] != []):
                raise ValueError('worker group differs from retained assignment')
        worker_ids = [identity for group in report['groups'].values() for identity in group['actual_case_ids']]
        if (type(report['tests_run']) is not int or report['tests_run'] != len(worker_ids)
                or type(report['planned_case_count']) is not int or report['planned_case_count'] != len(worker_ids)
                or worker_ids != job['case_ids']
                or [row['id'] for row in report['case_timings']] != worker_ids):
            raise ValueError('worker count differs from executed identities')
        actual.extend(worker_ids)
    if len(actual) != len(set(actual)) or set(actual) != set(planned):
        raise ValueError('duplicate, missing or extra case in worker reports')
    return actual


def worker(args):
    runner, descriptor, suites, identities = initialize(args)
    requested = json.loads(args.job.read_text())
    if requested['runner_sha256'] != digest(Path(__file__)):
        raise ValueError('worker runner differs from parent')
    modules = set(requested['modules'])
    assigned = {name for names in descriptor['groups'].values() for name in names}
    if not modules <= assigned or not modules:
        raise ValueError('worker module assignment is empty or unknown')
    selected = {}
    for group, suite in suites.items():
        cases = [case for case in runner.flatten_cases(suite) if case.__class__.__module__ in modules]
        suites[group] = unittest.TestSuite(cases)
        selected[group] = [case.id() for case in cases]
    planned = [identity for values in selected.values() for identity in values]
    if planned != requested['case_ids']:
        raise ValueError('worker discovery differs from planned assignment')
    timings = []
    depth = [0]
    original = unittest.TextTestResult

    class TimedResult(original):
        def startTest(self, case):
            self.case_start = time.perf_counter()
            self.top_level_case = depth[0] == 0 and case.id() in planned
            depth[0] += 1
            super().startTest(case)

        def stopTest(self, case):
            depth[0] -= 1
            if self.top_level_case:
                timings.append({'id': case.id(), 'seconds': time.perf_counter() - self.case_start})
            super().stopTest(case)

    with mock.patch.object(unittest, 'TextTestResult', TimedResult):
        report = runner.run_suites(args.root, descriptor, suites, selected, verbosity=2,
                                  stop_after_failed_group=True,
                                  archived_conformance_root=args.archive)
    runner.prepare_guarded_point_artifacts(args.root, descriptor, args.graph)
    from archived_conformance_test_support import authenticate_archive
    authenticate_archive(args.archive, descriptor['archived_conformance']['old_source_fingerprints'])
    runner.authenticate_sources(args.root, descriptor)
    authenticate_runner(args.root)
    if digest(Path(__file__)) != requested['runner_sha256']:
        raise ValueError('worker runner changed during execution')
    report.update(case_timings=timings, peak_memory_bytes=max(peak_bytes(resource.RUSAGE_SELF), peak_bytes(resource.RUSAGE_CHILDREN)),
                  runner_sha256=requested['runner_sha256'], modules=requested['modules'])
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    return 0 if report['passed'] else 1


def parent(args):
    start = time.perf_counter()
    runner, descriptor, suites, identities = initialize(args)
    module_ids = {}
    for group in runner.GROUPS:
        for identity in identities[group]:
            module_ids.setdefault(identity.split('.')[0], []).append(identity)
    selected_modules = set(args.modules or module_ids)
    if not selected_modules or not selected_modules <= module_ids.keys():
        raise ValueError('selected modules empty or unknown')
    # Largest modules first; assignment is stable and only balances case count.
    # Measured timing weights can be added later without changing admission.
    buckets = [[] for _ in range(min(args.workers, len(selected_modules)))]
    weights = [0] * len(buckets)
    for module in sorted(selected_modules, key=lambda m: (-len(module_ids[m]), m)):
        index = min(range(len(buckets)), key=lambda i: (weights[i], i))
        buckets[index].append(module)
        weights[index] += len(module_ids[module])
    jobs, requests = [], []
    script = Path(__file__).resolve()
    script_pin = digest(script)
    for index, modules in enumerate(buckets):
        # Keep original discovery order inside each worker and every module.
        cases = [identity for group in runner.GROUPS for identity in identities[group]
                 if identity.split('.')[0] in modules]
        job = args.output.parent / (args.output.stem + f'.worker{index}.job.json')
        output = args.output.parent / (args.output.stem + f'.worker{index}.json')
        log = args.output.parent / (args.output.stem + f'.worker{index}.log')
        if any(path.exists() for path in (job, output, log)):
            raise ValueError('worker evidence path already exists')
        request = {'modules': modules, 'case_ids': cases, 'runner_sha256': script_pin,
                   'group_case_ids': {group: [identity for identity in identities[group] if identity.split('.')[0] in modules]
                                      for group in runner.GROUPS}}
        job.write_text(json.dumps(request, indent=2) + '\n')
        requests.append(request)
        jobs.append((job, output, log))

    running, lock, stopped = set(), threading.Lock(), threading.Event()
    sampled_peak = [0]
    sampling = {'count': 0, 'errors': []}

    def sample_memory():
        while not stopped.wait(0.5):
            with lock:
                roots = set(running)
            roots.add(os.getpid())
            try:
                output = subprocess.run(['/bin/ps', '-axo', 'pid=,ppid=,rss='], capture_output=True, text=True, check=True).stdout
            except (OSError, subprocess.CalledProcessError) as error:
                sampling['errors'].append(str(error))
                return
            try:
                rows = [tuple(map(int, line.split())) for line in output.splitlines() if len(line.split()) == 3]
            except (ValueError, TypeError) as error:
                sampling['errors'].append(str(error))
                return
            sampling['count'] += 1
            descendants = set(roots)
            while True:
                expanded = descendants | {pid for pid, parent, rss in rows if parent in descendants}
                if expanded == descendants:
                    break
                descendants = expanded
            sampled_peak[0] = max(sampled_peak[0], sum(rss * 1024 for pid, parent, rss in rows if pid in descendants))

    def execute(job):
        request, output, log = job
        command = [sys.executable, '-B', str(script), '--root', str(args.root), '--archive', str(args.archive),
                   '--graph', str(args.graph), '--output', str(output), '--job', str(request)]
        with log.open('w') as stream:
            process = subprocess.Popen(command, stdout=stream, stderr=subprocess.STDOUT, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'))
            with lock:
                running.add(process.pid)
            try:
                code = process.wait()
            finally:
                with lock:
                    running.remove(process.pid)
        if code:
            raise ValueError('worker failed: ' + str(log))
        return json.loads(output.read_text())

    sampler = threading.Thread(target=sample_memory, daemon=True)
    sampler.start()
    try:
        with concurrent.futures.ThreadPoolExecutor(max_workers=len(jobs)) as executor:
            reports = list(executor.map(execute, jobs))
    finally:
        stopped.set()
        sampler.join()
    for report in reports:
        if report['runner_sha256'] != script_pin or report['descriptor_sha256'] != runner.canonical(descriptor):
            raise ValueError('worker source identity differs from parent')
    planned = [identity for group in runner.GROUPS for identity in identities[group]
               if identity.split('.')[0] in selected_modules]
    actual = join_reports(reports, planned, requests)
    runner.authenticate_sources(args.root, descriptor)
    runner.prepare_guarded_point_artifacts(args.root, descriptor, args.graph)
    from archived_conformance_test_support import authenticate_archive
    authenticate_archive(args.archive, descriptor['archived_conformance']['old_source_fingerprints'])
    authenticate_runner(args.root)
    if digest(script) != script_pin:
        raise ValueError('parent runner changed during execution')
    report = {'passed': True, 'scope': 'full-discovered' if selected_modules == module_ids.keys() else 'explicit-module-subset',
              'planned_case_ids': planned, 'actual_case_ids': actual, 'tests_run': len(actual), 'workers': len(jobs),
              'elapsed_seconds': time.perf_counter() - start, 'runner_sha256': digest(script),
              'descriptor_sha256': runner.canonical(descriptor),
              'maximum_worker_largest_process_peak_bytes': max(r['peak_memory_bytes'] for r in reports),
              'sampled_orchestrator_and_worker_tree_peak_rss_bytes': sampled_peak[0],
              'rss_sampling_interval_seconds': 0.5,
              'rss_samples': sampling['count'], 'rss_sampling_errors': sampling['errors'],
              'memory_qualification': 'Largest-process rusage includes worker and child maxima. Concurrent tree RSS includes orchestrator and sampler processes; 0.5s samples are a lower bound on true peak. No summed-peak memory bound claimed.',
              'worker_reports': [{'path': str(output), 'sha256': digest(output), 'log_sha256': digest(log)} for _, output, log in jobs],
              'case_timings': [row for r in reports for row in r['case_timings']],
              'qualification': 'Original assertions, verifiers, descriptors and live source contexts unchanged; fresh process per module bucket. Already-running peers may finish after another worker refuses; each worker preserves stop-after-failed-group. Per-case timings exclude module/class fixture and context setup, which remain included in elapsed walltime.'}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k not in ('planned_case_ids', 'actual_case_ids', 'case_timings')}))
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('root', 'archive', 'graph', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--workers', type=int, default=2)
    parser.add_argument('--modules', nargs='+')
    parser.add_argument('--job', type=Path)
    args = parser.parse_args()
    if args.workers < 1 or args.workers > 4:
        parser.error('workers must be between one and four')
    args.root, args.archive, args.graph = args.root.resolve(), args.archive.resolve(), args.graph.resolve()
    if any(args.output.resolve().is_relative_to(root) for root in (args.root, args.archive, args.graph)):
        parser.error('evidence output must be outside all source/fixture roots')
    if args.output.exists():
        parser.error('output must be fresh')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    return worker(args) if args.job else parent(args)


if __name__ == '__main__':
    sys.exit(main())
