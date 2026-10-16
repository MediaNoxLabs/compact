#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Reversible repository-wide workflow controls. Plans are read-only by default."""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import subprocess

# Keep source qualification, required gates and security automation available.
PROTECTED = frozenset({'rust-backend-checks.yml', 'forge-local-ci.yml',
                       'build-compiler.yml', 'codeql.yml', 'scan.yml'})
PAUSED = {
    'build-debug-test.yml': 'TypeScript debugger; enable for debugger changes',
    'build-runtime-test.yml': 'TypeScript runtime; enable for runtime changes',
    'build-vscplugin.yml': 'Editor extension; enable for editor changes',
    'test-vscplugin-installation.yml': 'Editor installation tests',
    'test-vscplugin-integration.yml': 'Editor integration tests',
    'ci-specification.yml': 'Agda; enable for specification changes or full qualification',
    'compact-release.yml': 'Unrelated compact CLI release matrix',
    'compact-test.yml': 'Unrelated compact CLI; enable for tools/compact changes',
    'check-docs-sync.yml': 'Upstream documentation synchronization schedule',
    'compiler-extracted.yml': 'Extracted compiler release qualification',
    'compiler-fuzzer.yml': 'Large scheduled fuzzing; enable for full qualification',
    'compatibility-test.yml': 'Upstream release compatibility orchestration',
    'compute-release-notes.yml': 'Release orchestration',
    'dev-publish.yml': 'Development publishing',
    'generate-docs.yml': 'Release documentation publishing',
    'internal-release.yml': 'Internal publishing',
    'public-release.yml': 'Public publishing',
    'release-build.yml': 'Reusable release build',
    'release-runtime.yml': 'Runtime publishing',
    'release-test.yml': 'Reusable release tests',
    'release-vscplugin.yml': 'Editor publishing',
    'test-contracts.yml': 'Upstream contract suite; enable for full qualification',
}
STATES = frozenset({'active', 'disabled_manually'})


def require(condition, message):
    if not condition:
        raise ValueError(message)


class GitHub:
    def __init__(self, repo):
        require(re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9-]*/[A-Za-z0-9_.-]+', repo) and repo.split('/')[1] not in {'.', '..'}, 'explicit OWNER/REPO required')
        self.repo = repo

    def request(self, endpoint, method=None, paginate=False):
        argv = ['gh', 'api']
        if method:
            argv += ['--method', method]
        if paginate:
            argv += ['--paginate', '--slurp']
        argv += ['repos/' + self.repo + '/' + endpoint]
        result = subprocess.run(argv, capture_output=True, text=True, check=False, timeout=60)
        require(result.returncode == 0, result.stderr.strip() or 'GitHub request failed')
        return json.loads(result.stdout) if result.stdout.strip() else None

    def inventory(self):
        pages = self.request('actions/workflows?per_page=100', paginate=True)
        rows = [row for page in pages for row in page['workflows']]
        require(len({row['id'] for row in rows}) == len(rows), 'duplicate workflow IDs')
        return rows

    def get(self, workflow_id):
        return self.request('actions/workflows/' + str(workflow_id))

    def set(self, workflow_id, state):
        require(state in STATES, 'unsupported workflow state')
        self.request('actions/workflows/' + str(workflow_id) +
                     ('/enable' if state == 'active' else '/disable'), method='PUT')


def workflow_name(row):
    path = row['path']
    require(path.startswith('.github/workflows/') and
            len(Path(path).parts) == 3 and path.endswith(('.yml', '.yaml')), 'unexpected workflow path')
    return Path(path).name


def plan(rows, desired):
    by_name = {}
    for row in rows:
        name = workflow_name(row)
        require(name not in by_name, 'ambiguous workflow filename: ' + name)
        by_name[name] = row
    require(not (set(desired) - set(by_name)), 'workflow not registered: ' + ', '.join(sorted(set(desired) - set(by_name))))
    changes = []
    for name, target in sorted(desired.items()):
        require(target in STATES, 'unsupported target state')
        require(not (target != 'active' and name in PROTECTED), 'protected workflow: ' + name)
        row = by_name[name]
        require(row['state'] in STATES, 'unsupported current workflow state: ' + name)
        if row['state'] != target:
            changes.append(dict(id=row['id'], path=row['path'], before=row['state'], after=target))
    return changes


def restore_plan(snapshot, repo, rows):
    require(snapshot.get('schema') == 1 and snapshot.get('repo') == repo, 'snapshot schema/repository mismatch')
    current = {row['id']: row for row in rows}
    desired = {}
    for row in snapshot['changes']:
        require(set(row) == {'id', 'path', 'before', 'after'} and row['before'] in STATES
                and row['after'] in STATES, 'invalid snapshot operation')
        require(row['id'] in current and current[row['id']]['path'] == row['path'], 'workflow identity changed')
        require(current[row['id']]['state'] in {row['before'], row['after']}, 'workflow state changed externally')
        name = workflow_name(row)
        require(name not in desired, 'duplicate snapshot operation')
        desired[name] = row['before']
    return plan(rows, desired)


def apply(client, rows, changes, snapshot):
    require(snapshot is not None, '--snapshot is required with --apply')
    # Create-only snapshot precedes every remote write, including partial failures.
    with snapshot.open('x') as stream:
        json.dump(dict(schema=1, repo=client.repo, created_at=datetime.now(timezone.utc).isoformat(),
                       inventory=rows, changes=changes), stream, indent=2)
        stream.write('\n')
    for operation in changes:
        live = client.get(operation['id'])
        require(live['path'] == operation['path'] and live['state'] == operation['before'],
                'workflow changed after planning; restore snapshot to reconcile partial changes')
        client.set(operation['id'], operation['after'])
        live = client.get(operation['id'])
        require(live['path'] == operation['path'] and live['state'] == operation['after'],
                'GitHub did not confirm requested workflow state')


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', required=True, help='explicit OWNER/REPO; never inferred from upstream')
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('list')
    for name in ('focus', 'enable', 'disable', 'restore'):
        command = sub.add_parser(name)
        if name in ('enable', 'disable'):
            command.add_argument('workflows', nargs='+', help='exact workflow filenames')
        if name == 'restore':
            command.add_argument('input', type=Path)
        command.add_argument('--apply', action='store_true')
        command.add_argument('--snapshot', type=Path, help='new file recording state before any writes')
    args = parser.parse_args(argv)
    client = GitHub(args.repo)
    rows = client.inventory()
    if args.command == 'list':
        print(json.dumps(rows, indent=2))
        return
    if args.command == 'restore':
        changes = restore_plan(json.loads(args.input.read_text()), args.repo, rows)
    else:
        selected = ({workflow_name(row): 'disabled_manually' for row in rows if workflow_name(row) in PAUSED}
                    if args.command == 'focus' else
                    {name: 'active' if args.command == 'enable' else 'disabled_manually' for name in args.workflows})
        changes = plan(rows, selected)
    print(json.dumps(dict(repo=args.repo, apply=args.apply, changes=changes,
                         scope='repository-wide future triggers; running jobs are not cancelled'), indent=2), flush=True)
    if args.apply:
        apply(client, rows, changes, args.snapshot)
        print(json.dumps(dict(applied=len(changes), snapshot=str(args.snapshot))))


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        raise SystemExit(str(error)) from None
