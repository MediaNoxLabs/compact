#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Check the pinned source and test inventory for the 37 codegen-rust oracles.

This checks provenance and the presence of executable Rust/TypeScript fixture
links. It does not claim result, state, gas, or transcript parity: those claims
belong to the individual executing tests and the M2 acceptance matrix.
"""

import hashlib
import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = Path(__file__).with_name("oracle_acceptance.json")


def non_comment_lines(data: bytes) -> bytes:
    """Ignore only full-line Compact comments, preserving executable text."""
    return "\n".join(
        line for line in data.decode().splitlines() if not line.lstrip().startswith("//")
    ).strip().encode()


def relative_file(name: str) -> Path:
    path = ROOT / name
    if path.resolve().is_relative_to(ROOT.resolve()) and path.is_file():
        return path
    raise ValueError(f"missing or invalid repository file: {name}")


def main() -> int:
    manifest = json.loads(MANIFEST.read_text())
    fixtures = manifest["fixtures"]
    failures = []
    if len(fixtures) != 37:
        failures.append(f"expected 37 oracle entries, found {len(fixtures)}")
    names = [entry["oracle_source"] for entry in fixtures]
    if len(set(names)) != len(names):
        failures.append("duplicate oracle source")

    for entry in fixtures:
        name = entry["oracle_source"]
        try:
            source = relative_file(entry["source"])
            contents = source.read_bytes()
            if hashlib.sha256(contents).hexdigest() != entry["source_sha256"]:
                failures.append(f"{name}: local source bytes changed")
            if hashlib.sha256(non_comment_lines(contents)).hexdigest() != entry["non_comment_sha256"]:
                failures.append(f"{name}: executable source differs from the pinned oracle")

            tests = entry["rust_tests"]
            references = entry["typescript_fixtures"]
            if not tests or not references:
                failures.append(f"{name}: missing Rust test or TypeScript fixture link")
            test_contents = [relative_file(path).read_text() for path in tests]
            for path in references:
                relative_file(path)
                if not any(path.split("/")[-1] in text for text in test_contents):
                    failures.append(f"{name}: test does not reference {path}")
        except (KeyError, ValueError, UnicodeDecodeError) as exc:
            failures.append(f"{name}: {exc}")

    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    print(f"Checked {len(fixtures)} pinned oracle sources; {len(failures)} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
