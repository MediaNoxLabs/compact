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

"""Replay one pinned TypeScript oracle against an explicitly supplied offline toolchain."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ORACLE = Path(__file__).resolve().parent
ROOT = ORACLE.parents[2]
SOURCE = ROOT / "examples/rust_backend/vc_passport_adoption/src/digital-passport-credential.compact"
PROVENANCE = json.loads((ORACLE / "provenance.json").read_text())
PROFILES = {
    "branch": ("branch_typescript_and_rust", "branch-0.31.133-ledger8.0.3"),
    "upstream": ("upstream_typescript", "upstream-0.31.1-ledger8.0.2"),
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(args: list[str]) -> None:
    completed = subprocess.run(args, capture_output=True, text=True, check=False, timeout=120)
    if completed.returncode:
        raise RuntimeError(f"{args[0]} exited {completed.returncode}: {completed.stderr}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, required=True)
    parser.add_argument("--compiler", type=Path, required=True)
    parser.add_argument("--runtime-package", type=Path, required=True)
    args = parser.parse_args()
    field, label = PROFILES[args.profile]
    identity = PROVENANCE[field]
    compiler = args.compiler.resolve(strict=True)
    runtime = args.runtime_package.resolve(strict=True)
    assert sha256(compiler) == identity["compiler_sha256"], "compiler byte identity differs"
    assert json.loads((runtime / "package.json").read_text())["version"] == identity["runtime_version"]
    assert SOURCE.is_file(), f"missing pinned source {SOURCE}"
    with tempfile.TemporaryDirectory(prefix="compact-vc-passport-oracle-") as temporary:
        output = Path(temporary)
        command = [str(compiler)]
        if args.profile == "branch":
            command += ["--target", "ts"]
        command += ["--skip-zk", str(SOURCE), str(output)]
        run(command)
        generated = output / "contract/index.js"
        assert sha256(generated) == identity["generated_js_sha256"], "generated JS differs"
        package_link = output / "contract/node_modules/@midnight-ntwrk/compact-runtime"
        package_link.parent.mkdir(parents=True)
        package_link.symlink_to(runtime, target_is_directory=True)
        for suffix, script in (("ts", "capture.mjs"), ("age", "capture-age.mjs"),
                               ("request", "capture-request.mjs"),
                               ("private-parts", "capture-private-parts.mjs"),
                               ("protocol", "capture-protocol.mjs"),
                               ("roots", "capture-roots.mjs"),
                               ("bindings", "capture-bindings.mjs"),
                               ("signed-flow", "capture-signed-flow.mjs"),
                               ("authorization", "capture-authorization.mjs"),
                               ("protocol-roundtrip", "capture-protocol-roundtrip.mjs"),
                               ("complete-flow", "capture-complete-flow.mjs")):
            actual = output / f"{suffix}.json"
            command = ["node", str(ORACLE / script), str(generated)]
            if suffix == "age":
                command.append(str(ORACLE / "age-inputs.json"))
            command += [label, str(actual)]
            run(command)
            expected = ORACLE / f"{args.profile}-{suffix}-capture.json"
            assert json.loads(actual.read_text()) == json.loads(expected.read_text()), expected
    print(f"{args.profile}: 193 exact cases across all 75 exports match checked-in capture")


if __name__ == "__main__":
    main()
