#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Local Forge CI orchestration. The official helper owns cache identity/metrics."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

import forge_circuits as adapter

ROOT = Path(__file__).resolve().parents[2]
QUALIFICATION = Path(__file__).with_name('forge_ci_qualification.json')
DEFAULT_BUDGET = 1024 * 1024 * 1024


def require(ok, message):
    if not ok:
        raise ValueError(message)


def load(path):
    return adapter.read_json(path.read_bytes())


def write(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def file_hash(path):
    require(path.is_file() and not path.is_symlink(), 'regular file required: ' + str(path))
    return adapter.digest(path)


def execute(argv, directory, phase, *, env=None, failure=False):
    start = time.monotonic()
    result = subprocess.run(list(map(str, argv)), cwd=ROOT, env=env, capture_output=True, text=True)
    write(directory / (phase + '.command.json'), dict(argv=list(map(str, argv)), exit_code=result.returncode,
          wall_seconds=time.monotonic() - start, stdout=result.stdout, stderr=result.stderr))
    if result.returncode != 0 and not failure:
        # Hosted runners disappear after the job. Surface bounded diagnostics;
        # the complete command receipt remains available for artifact upload.
        for label, content in (('stdout', result.stdout), ('stderr', result.stderr)):
            if content:
                print(phase + ' ' + label + ' (last 8192 characters):\n' + content[-8192:],
                      file=sys.stderr)
    require(result.returncode != 0 if failure else result.returncode == 0,
            phase + (' accepted corrupt verifier' if failure else ' failed; inspect retained log'))
    return result.stdout


def parameters(directory, q, *, allow_missing=False):
    missing = []
    for name, expected in q['parameters'].items():
        path = directory / name
        if not os.path.lexists(path):
            missing.append(name)
        else:
            require(file_hash(path) == expected, 'parameter digest mismatch: ' + name)
    require(allow_missing or not missing, 'missing verified parameters: ' + ', '.join(missing))
    return missing


def profile(forge, q, output):
    adapter.validate_consumer()
    value = adapter.read_json(execute([forge, '--profile', q['selector'], 'profile'], output, 'profile'))
    require(value.get('profile') == adapter.PROFILE
            and value.get('ledger_revision') == adapter.REVISION
            and value.get('build_fingerprint') == q['build_fingerprint'],
            'unqualified Forge profile/build/revision')


def compiler_command(compiler, q, output, *, skip=False):
    return [compiler, '--target', 'rust', *(['--skip-zk'] if skip else []),
            '--rust-runtime-root', ROOT, ROOT / q['compact_source'], output]


def bootstrap(args):
    q = load(QUALIFICATION)
    args.output.mkdir(parents=True, exist_ok=False)
    missing = parameters(args.params_dir, q, allow_missing=True)
    require(not missing or args.fetch_params, 'missing parameters; acquisition requires --fetch-params')
    profile(args.forge, q, args.output)
    if missing:
        args.params_dir.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='bootstrap-', dir=args.output) as temporary:
            private = Path(temporary)
            sources = {}
            for name in missing:
                row = q['bootstrap_circuits'][name]
                sources.setdefault(row['compact_source'], []).append((name, row['circuit']))
            for index, (source, circuits) in enumerate(sources.items()):
                generated = private / ('generated-' + str(index))
                execute(compiler_command(args.compiler, dict(q, compact_source=source), generated, skip=True),
                        args.output, 'bootstrap-compile-' + str(index))
                for name, circuit in circuits:
                    execute([args.forge, '--profile', q['selector'], '--cache-dir', private / 'store',
                             '--params-dir', args.params_dir, '--fetch-params', 'prepare',
                             generated / 'zkir' / (circuit + '.zkir')], args.output, 'bootstrap-' + name)
        parameters(args.params_dir, q)
    value = dict(parameters_verified=True, acquired=missing, proof_evidence=False)
    write(args.output / 'bootstrap.json', value)
    return value


def tool_bindings(compiler, proof_runner, helper):
    compiler = compiler.resolve(strict=True)
    paths = dict(compiler=compiler, scheme=compiler.with_name('compactc-scheme'),
                 proof_runner=proof_runner, helper=helper)
    return {name: dict(path=str(path.resolve(strict=True)), sha256=file_hash(path.resolve(strict=True)))
            for name, path in paths.items()}


def check_tools(invocation):
    for name, row in invocation['tools'].items():
        require(file_hash(Path(row['path'])) == row['sha256'], name + ' changed after planning')


def helper_command(invocation, *args):
    helper = Path(invocation['tools']['helper']['path'])
    return [*([sys.executable, '-B'] if helper.suffix == '.py' else []), helper,
            '--binary', invocation['forge'], *args]


def helper(invocation, directory, phase, *args):
    check_tools(invocation)
    value = adapter.read_json(execute(helper_command(invocation, *args), directory, phase))
    check_tools(invocation)
    return value


def workload_inputs(q):
    names = {*q['inputs'], q['compact_source'],
             *(row['compact_source'] for row in q['bootstrap_circuits'].values())}
    for tree in q['input_trees']:
        names.update(p.relative_to(ROOT).as_posix() for p in (ROOT / tree).rglob('*')
                     if p.is_file() and p.suffix in q['input_suffixes'] and '__pycache__' not in p.parts)
    return sorted(names)


def plan(args):
    q = load(QUALIFICATION)
    parameters(args.params_dir, q)
    args.output.mkdir(parents=True, exist_ok=False)
    profile(args.forge, q, args.output)
    tools = tool_bindings(args.compiler, args.proof_runner, args.helper)
    invocation = dict(forge=str(args.forge), params_dir=str(args.params_dir), tools=tools)
    # Executable bytes join declared source bytes in the helper's workload identity.
    workload = json.dumps(dict(selector=q['proof_selector'], target='rust',
                         rust_runtime_root='workspace', proofs=3, corrupt_verifier=True,
                         tools={name: row['sha256'] for name, row in tools.items()}), sort_keys=True)
    argv = ['key', '--workspace', ROOT, '--params-dir', args.params_dir,
            '--profile', q['selector'], '--workload', workload, '--generation', args.generation,
            '--output', args.output / 'key.json']
    for name in q['parameters']:
        argv += ['--parameter', name]
    for name in workload_inputs(q):
        argv += ['--input', name]
    if args.github_output:
        argv += ['--github-output', args.github_output]
    value = helper(invocation, args.output, 'key', *argv)
    write(args.output / 'invocation.json', invocation)
    return value


def artifact_hashes(output):
    paths = [p for tree in ('zkir', 'keys') for p in (output / tree).rglob('*') if p.is_file()]
    require(paths, 'compiler produced no circuit artifacts')
    return {p.relative_to(output).as_posix(): file_hash(p) for p in sorted(paths)}


def exercise(invocation, directory, q):
    env = dict(os.environ, MIDNIGHT_PP=invocation['params_dir'], COMPACT_ZKIR_MODE='forge')
    with tempfile.TemporaryDirectory(prefix='fixtures-', dir=directory) as temporary:
        output = Path(temporary) / 'generated'
        compiler = invocation['tools']['compiler']['path']
        proof_runner = invocation['tools']['proof_runner']['path']
        check_tools(invocation)
        execute([sys.executable, '-B', Path(__file__).with_name('forge_circuits.py'),
                 '--forge', invocation['forge'], '--cache-dir', directory / 'cache',
                 '--params-dir', invocation['params_dir'], '--receipt-dir', directory / 'events',
                 '--', *compiler_command(compiler, q, output)], directory, 'compile', env=env)
        check_tools(invocation)
        hashes = artifact_hashes(output)
        stdout = execute([proof_runner, q['proof_selector'], output], directory, 'proof', env=env)
        check_tools(invocation)
        require(all(stdout.splitlines().count(marker) == 1 for marker in q['success_markers']),
                'missing or repeated proof/ledger success marker')
        negative = Path(temporary) / 'negative'
        shutil.copytree(output, negative, symlinks=False)
        (negative / 'keys' / (q['corrupt_verifier'] + '.verifier')).write_bytes(b'invalid verifier')
        execute([proof_runner, q['proof_selector'], negative], directory, 'corrupt-verifier', env=env, failure=True)
        check_tools(invocation)
        require(artifact_hashes(output) == hashes, 'consumer changed circuit artifacts')
        write(directory / 'artifacts.json', hashes)


def run(args):
    directory = args.plan.resolve(strict=True)
    invocation = load(directory / 'invocation.json')
    q = load(QUALIFICATION)
    require(args.max_bytes > 0, 'byte budget must be positive')
    if args.snapshot_out:
        require(not args.snapshot_out.resolve().is_relative_to((directory / 'cache').resolve()),
                'snapshot output must be outside cache')
    # Exclusive marker refuses reruns and stale test verdict reuse, including failed attempts.
    write(directory / 'run-started.json', dict(started_ns=time.time_ns()))
    planned_tools = json.loads(load(directory / 'key.json')['workload'])['tools']
    require(planned_tools == {name: row['sha256'] for name, row in invocation['tools'].items()},
            'invocation tool hashes differ from the helper workload')
    check_tools(invocation)
    parameters(Path(invocation['params_dir']), q)
    common = ['--key', directory / 'key.json', '--cache-dir', directory / 'cache']
    helper(invocation, directory, 'restore', 'restore', *common,
           '--snapshot', args.snapshot, '--max-bytes', args.max_bytes)
    helper(invocation, directory, 'baseline', 'baseline', *common, '--output', directory / 'baseline.json')
    try:
        exercise(invocation, directory, q)
        if args.uncached_zkir:
            execute([sys.executable, '-B', Path(__file__).with_name('check_forge_circuit_cache.py'),
                     '--compiler', invocation['tools']['compiler']['path'],
                     '--proof-runner', invocation['tools']['proof_runner']['path'],
                     '--forge', invocation['forge'], '--params-dir', invocation['params_dir'],
                     '--uncached-zkir', args.uncached_zkir, '--report', directory / 'uncached-report.json'],
                    directory, 'uncached')
            check_tools(invocation)
    except BaseException:
        # Keep the original failure if the post-failure identity/metrics check also refuses.
        try:
            report = helper(invocation, directory, 'report', 'report', *common, '--baseline', directory / 'baseline.json')
            write(directory / 'cache-report.json', report)
        except (ValueError, OSError):
            pass
        raise
    report = helper(invocation, directory, 'report', 'report', *common, '--baseline', directory / 'baseline.json')
    write(directory / 'cache-report.json', report)
    if args.snapshot_out:
        key = load(directory / 'key.json')
        scope = [item for row in key['parameters'] for item in ['--parameter-digest', row['sha256']]]
        execute([invocation['forge'], '--profile', q['selector'], '--cache-dir', directory / 'cache',
                 'snapshot', '--output', args.snapshot_out, '--partition', key['partition'],
                 '--max-bytes', args.max_bytes, *scope], directory, 'snapshot')
        check_tools(invocation)
    write(directory / 'run.json', dict(passed=True, proof_scenarios=3, corrupt_verifier_rejected=True))
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    subs = parser.add_subparsers(dest='command', required=True)
    for name in ('bootstrap', 'plan'):
        p = subs.add_parser(name)
        for flag in ('compiler', 'forge', 'params-dir', 'output'):
            p.add_argument('--' + flag, type=Path, required=True)
        if name == 'bootstrap':
            p.add_argument('--fetch-params', action='store_true')
        else:
            p.add_argument('--proof-runner', type=Path, required=True)
            p.add_argument('--helper', type=Path, required=True)
            p.add_argument('--generation', required=True)
            p.add_argument('--github-output', type=Path)
    p = subs.add_parser('run')
    p.add_argument('--plan', type=Path, required=True)
    p.add_argument('--snapshot', type=Path, required=True)
    p.add_argument('--uncached-zkir', type=Path)
    p.add_argument('--snapshot-out', type=Path)
    p.add_argument('--max-bytes', type=int, default=DEFAULT_BUDGET)
    args = parser.parse_args(argv)
    for name, value in vars(args).items():
        if isinstance(value, Path):
            setattr(args, name, value.absolute())
    print(json.dumps(dict(bootstrap=bootstrap, plan=plan, run=run)[args.command](args)))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(json.dumps(dict(error=str(error))), file=sys.stderr)
        raise SystemExit(1)
