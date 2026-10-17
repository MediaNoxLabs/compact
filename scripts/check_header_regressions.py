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

"""Report inherited full-header debt and reject new violations on pull requests."""
import argparse
import hashlib
import io
import os
import json
import logging
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from add_headers import HeaderManager


def base_contents(root, base, names):
    # Resolve paths once, then read unique blobs in one process. Filenames never
    # become line-delimited batch commands, so spaces/newlines remain unambiguous.
    tree = subprocess.run(['git', 'ls-tree', '-rz', '--full-tree', base], cwd=root,
                          capture_output=True, check=True).stdout
    wanted = set(names)
    ids = {}
    for entry in tree.split(b'\0'):
        if not entry:
            continue
        metadata, name = entry.split(b'\t', 1)
        name = os.fsdecode(name)
        if name in wanted:
            mode, kind, oid = metadata.split()
            if kind != b'blob':
                raise ValueError('base path is not a blob: ' + name)
            ids[name] = oid
    unique = list(dict.fromkeys(ids.values()))
    blobs = {}
    if unique:
        raw = subprocess.run(['git', 'cat-file', '--batch'], cwd=root,
                             input=b'\n'.join(unique) + b'\n', capture_output=True, check=True).stdout
        stream = io.BytesIO(raw)
        for expected in unique:
            header = stream.readline().split()
            if len(header) != 3 or header[:2] != [expected, b'blob']:
                raise ValueError('cannot read base blob')
            size = int(header[2])
            content = stream.read(size)
            if len(content) != size or stream.read(1) != b'\n':
                raise ValueError('incomplete base blob')
            blobs[expected] = content
        if stream.read():
            raise ValueError('unexpected base blob output')
    return {name: blobs[ids[name]] if name in ids else None for name in wanted}


def base_bytes(root, base, name):
    return base_contents(root, base, [name])[name]


def classify(root, base, missing):
    originals = base_contents(root, base, [Path(path).relative_to(root).as_posix() for path in missing])
    inherited, violations = [], []
    for path in sorted(missing):
        path = Path(path)
        name = path.relative_to(root).as_posix()
        current = path.read_bytes()
        original = originals[name]
        row = dict(path=name, sha256=hashlib.sha256(current).hexdigest())
        (inherited if original is not None and original == current else violations).append(row)
    return dict(scope='PR header regressions; not full-tree license compliance', base_commit=base,
                missing_full_headers=len(missing), inherited_unchanged=inherited,
                new_or_modified_violations=violations, passed=not violations)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', required=True, help='frozen full PR base commit SHA')
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args(argv)
    if not re.fullmatch(r'[0-9a-f]{40}|[0-9a-f]{64}', args.base):
        parser.error('full base commit SHA required')
    subprocess.run(['git', 'cat-file', '-e', args.base + '^{commit}'], cwd=ROOT, check=True)
    # Original validator and full-template policy are unchanged. Keep per-file debt
    # in the receipt instead of flooding the log with inherited warnings.
    logging.getLogger().setLevel(logging.ERROR)
    manager = HeaderManager(str(ROOT / 'header_config.json'), str(ROOT))
    report = classify(ROOT, args.base, manager.validate_headers())
    with args.report.open('x') as output:
        json.dump(report, output, indent=2)
        output.write('\n')
    print(json.dumps(dict(scope=report['scope'], missing_full_headers=report['missing_full_headers'],
                         inherited_count=len(report['inherited_unchanged']),
                         violations=report['new_or_modified_violations'], passed=report['passed']), indent=2))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error)) from None
