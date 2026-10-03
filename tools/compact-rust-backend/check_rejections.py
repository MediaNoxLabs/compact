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

"""Ensure the public Rust compiler target rejects unsupported programs.

The positive fixture checker cannot detect an unsupported construct that
quietly emits a plausible Rust library. This gate pins two source-level
refusals and verifies that no generated Cargo library survives. It also
checks that a later packaging failure cannot publish partial Rust output
or replace a previously complete directory.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


CASES = {
    "unknown-opaque": (
        'import CompactStandardLibrary;\n'
        'export ledger thing: Opaque<"NotAThing">;\n'
        'constructor() {}\n',
        "Rust backend does not yet support this opaque type",
    ),
    "field-to-uint": (
        'export ledger n: Uint<64>;\n'
        'export circuit narrow(f: Field): Uint<64> { return f as Uint<64>; }\n',
        "Rust backend does not yet support Field-to-Uint downcasts",
    ),
}

ROOT = Path(__file__).resolve().parents[2]


def snapshot(path: Path) -> dict[str, str]:
    return {
        item.relative_to(path).as_posix(): hashlib.sha256(item.read_bytes()).hexdigest()
        for item in path.rglob("*") if item.is_file()
    }


def check_output_publication(compactc: str, directory: Path) -> list[str]:
    failures = []
    source = ROOT / "examples/rust_backend/counter.compact"
    output = directory / "publication"
    invalid_environment = os.environ.copy()
    invalid_environment["COMPACT_RUST_RUNTIME_DIR"] = str(directory / "missing-runtime")

    def run_compiler(environment: dict[str, str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [compactc, "--target", "rust", "--skip-zk", str(source), str(output)],
            capture_output=True, text=True, env=environment, check=False,
        )

    def leaked_stage() -> bool:
        return any(directory.glob(f".{output.name}.compactc-stage-*"))

    rejected = run_compiler(invalid_environment)
    if rejected.returncode == 0 or "runtime source directory is invalid" not in rejected.stderr:
        failures.append(f"post-render failure was not reported:\n{rejected.stderr}")
    if output.exists() or leaked_stage():
        failures.append("failed fresh compile published partial output or left staging debris")

    valid = run_compiler(os.environ.copy())
    if valid.returncode or not (output / "contract/lib.rs").is_file() or not (
        output / "contract/Cargo.toml"
    ).is_file():
        failures.append(f"valid output for replacement test was not complete:\n{valid.stderr}")
        return failures
    before = snapshot(output)
    rejected = run_compiler(invalid_environment)
    if rejected.returncode == 0 or snapshot(output) != before or leaked_stage():
        failures.append("failed rebuild changed previously complete output or left staging debris")

    (output / "stale-marker").write_text("previous generated directory")
    valid = run_compiler(os.environ.copy())
    if valid.returncode or (output / "stale-marker").exists() or leaked_stage():
        failures.append(f"successful rebuild did not replace output cleanly:\n{valid.stderr}")

    preserved = snapshot(output)
    file_output = directory / "output-file"
    file_output.write_text("keep this file")
    link_output = directory / "output-link"
    link_output.symlink_to(output, target_is_directory=True)
    for candidate in (file_output, link_output):
        result = subprocess.run(
            [compactc, "--target", "rust", "--skip-zk", str(source), str(candidate)],
            capture_output=True, text=True, env=os.environ.copy(), check=False,
        )
        if result.returncode == 0 or "file or symlink" not in result.stderr:
            failures.append(f"{candidate.name}: unsafe output root was accepted:\n{result.stderr}")
    if file_output.read_text() != "keep this file" or not link_output.is_symlink():
        failures.append("unsafe output root check changed an existing file or symlink")
    if snapshot(output) != preserved:
        failures.append("unsafe output root check changed the valid generated directory")
    return failures


def check_proof_capabilities(compactc: str, directory: Path) -> list[str]:
    failures = []
    source = directory / "product.compact"
    source.write_text(
        "import CompactStandardLibrary;\n"
        "export ledger value: Field;\n"
        "export circuit write(left: Field, right: Field): [] {\n"
        "  value = disclose(left * right);\n"
        "}\n"
    )
    output = directory / "product-output"

    def compile_source(path: Path, destination: Path, strict: bool) -> subprocess.CompletedProcess[str]:
        command = [compactc, "--target", "rust", "--skip-zk"]
        if strict:
            command.append("--rust-require-recording")
        return subprocess.run(
            [*command, str(path), str(destination)],
            capture_output=True, text=True, check=False,
        )

    default = compile_source(source, output, False)
    if default.returncode:
        return [f"native-only product did not compile normally:\n{default.stderr}"]
    report_path = output / "contract/rust-capabilities.json"
    manifest_path = output / "compiler/contract-manifest.json"
    try:
        report_bytes = report_path.read_bytes()
        report = json.loads(report_bytes)
        manifest = json.loads(manifest_path.read_bytes())
        assert report["schema_version"] == 1
        assert report["circuits"] == [{
            "name": "write",
            "source": {"file": "product.compact", "line": 3, "column": 1},
            "recorded": False,
            "observed_call": False,
        }]
        assert manifest["contract"]["rust-capabilities.json"]["hash"] == hashlib.sha256(report_bytes).hexdigest()
    except (AssertionError, FileNotFoundError, KeyError, ValueError) as error:
        failures.append(f"native-only capability report or manifest is wrong: {error}")
    before = snapshot(output)
    rejected = compile_source(source, output, True)
    if rejected.returncode == 0 or "product.compact line 3 char 1" not in rejected.stderr:
        failures.append(f"strict rebuild did not reject at source:\n{rejected.stderr}")
    if snapshot(output) != before:
        failures.append("strict rebuild changed the prior native-only output")
    fresh = directory / "product-strict-output"
    rejected = compile_source(source, fresh, True)
    if rejected.returncode == 0 or fresh.exists() or any(directory.glob(f".{fresh.name}.compactc-stage-*")):
        failures.append("strict fresh rejection published output or left staging debris")

    for name in ("counter", "cell_boolean"):
        path = ROOT / f"examples/rust_backend/{name}.compact"
        destination = directory / f"{name}-strict-output"
        accepted = compile_source(path, destination, True)
        if accepted.returncode:
            failures.append(f"strict {name} compile failed:\n{accepted.stderr}")
            continue
        report = json.loads((destination / "contract/rust-capabilities.json").read_text())
        if not report["circuits"] or any(
            not item["recorded"] or not item["observed_call"] for item in report["circuits"]
        ):
            failures.append(f"strict {name} report overstates proving support")

    collision_source = directory / "call-collision.compact"
    collision_source.write_text(
        "import CompactStandardLibrary;\n"
        "export ledger round: Counter;\n"
        "export circuit bump(): [] { round.increment(1); }\n"
        "export circuit bump_call(): [] { round.increment(1); }\n"
    )
    collision_output = directory / "call-collision-output"
    accepted = compile_source(collision_source, collision_output, False)
    if accepted.returncode:
        failures.append(f"call-name collision did not compile normally:\n{accepted.stderr}")
    else:
        report = json.loads((collision_output / "contract/rust-capabilities.json").read_text())
        actual = [(item["name"], item["recorded"], item["observed_call"]) for item in report["circuits"]]
        if actual != [("bump", True, False), ("bump_call", True, True)]:
            failures.append(f"call-name collision capability report is wrong: {actual}")
        before = snapshot(collision_output)
        rejected = compile_source(collision_source, collision_output, True)
        if rejected.returncode == 0 or "call-collision.compact line 3 char 1" not in rejected.stderr:
            failures.append(f"strict call-name collision did not reject at source:\n{rejected.stderr}")
        if snapshot(collision_output) != before:
            failures.append("strict call-name collision changed the prior output")
    return failures


def main() -> int:
    compactc = os.environ.get("COMPACTC")
    if not compactc or shutil.which(compactc) is None:
        print("Set COMPACTC to a ledger-8 compiler before running this gate", file=sys.stderr)
        return 1
    failures = []
    with tempfile.TemporaryDirectory(prefix="compact-rust-rejections-") as temp:
        directory = Path(temp)
        for name, (source, diagnostic) in CASES.items():
            source_path = directory / f"{name}.compact"
            output_path = directory / name
            source_path.write_text(source)
            result = subprocess.run(
                [compactc, "--target", "rust", "--skip-zk", str(source_path), str(output_path)],
                capture_output=True,
                text=True,
                env=os.environ.copy(),
                check=False,
            )
            message = result.stderr + result.stdout
            if result.returncode == 0:
                failures.append(f"{name}: compiler accepted an unsupported construct")
            elif (
                diagnostic not in message
                or f"{name}.compact line " not in message
                or " char " not in message
            ):
                failures.append(f"{name}: missing source diagnostic:\n{message}")
            if (output_path / "contract" / "lib.rs").exists():
                failures.append(f"{name}: rejected source left a generated Rust library")
            if (output_path / "contract" / "Cargo.toml").exists():
                failures.append(f"{name}: rejected source left a generated Cargo manifest")
            if output_path.exists() or any(directory.glob(f".{name}.compactc-stage-*")):
                failures.append(f"{name}: rejected source left output or staging debris")
        failures.extend(check_output_publication(compactc, directory))
        failures.extend(check_proof_capabilities(compactc, directory))

    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    print(f"Checked {len(CASES)} source rejections, output publication and proof capabilities; {len(failures)} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
