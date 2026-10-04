#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Compile the checked TS-positive source cohort and record exact proof metadata.

This gate establishes compiler acceptance and capability metadata only. It does
not execute TypeScript, Rust, proofs, or ledger transactions.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = Path(__file__).with_name("parity_positive_sources.json")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compile_source(compiler: Path, source: Path, target: str, output: Path) -> subprocess.CompletedProcess[str]:
    command = [str(compiler), "--target", target, "--skip-zk"]
    if target == "rust":
        command += ["--rust-runtime-root", str(ROOT)]
    return subprocess.run([*command, str(source), str(output)], cwd=ROOT,
                          env=os.environ.copy(), capture_output=True, text=True, check=False)


def proof_map(circuits: list[dict]) -> dict[str, tuple[bool, bool]]:
    names = [item["name"] for item in circuits]
    if len(names) != len(set(names)):
        raise ValueError("duplicate compiler contract-info circuit name")
    if any(type(item.get("proof")) is not bool or type(item.get("pure")) is not bool
           for item in circuits):
        raise ValueError("missing Boolean proof/pure compiler metadata")
    return {item["name"]: (item["pure"], item["proof"]) for item in circuits}


def check(compiler: Path) -> tuple[dict, list[str]]:
    manifest = json.loads(MANIFEST.read_text())
    receipt = {
        "format_version": 1,
        "scope": "PM-19252 compiler acceptance/proof metadata; no executing parity",
        "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "compiler": {"path": str(compiler), "sha256": sha256(compiler)},
        "manifest_sha256": sha256(MANIFEST),
        "suite": manifest["suite"],
        "sources": [],
    }
    failures = []
    for expected in [*manifest["positive_sources"], *manifest["expected_rejections"]]:
        source = ROOT / expected["source"]
        row = {"source": expected["source"], "source_sha256": sha256(source),
               "expected_ts": expected["expected_ts"]}
        with tempfile.TemporaryDirectory(prefix="compact-positive-scope-") as temporary:
            output = Path(temporary)
            ts = compile_source(compiler, source, "ts", output / "ts")
            row["ts_compile"] = "success" if ts.returncode == 0 else "rejection"
            if row["ts_compile"] != expected["expected_ts"]:
                failures.append(f"{source.name}: TypeScript compile status changed")
            if expected["expected_ts"] == "rejection":
                row["ts_diagnostic"] = ts.stderr.strip()
                if expected["diagnostic_contains"] not in ts.stderr:
                    failures.append(f"{source.name}: expected rejection diagnostic missing")
                receipt["sources"].append(row)
                continue
            info = output / "ts/compiler/contract-info.json"
            if not info.is_file():
                failures.append(f"{source.name}: successful TypeScript compile has no contract-info")
                receipt["sources"].append(row)
                continue
            circuits = json.loads(info.read_text())["circuits"]
            row["proof_circuits"] = [{key: item[key] for key in ("name", "pure", "proof")}
                                     for item in circuits]
            try:
                if proof_map(circuits) != proof_map(expected["proof_circuits"]):
                    failures.append(f"{source.name}: compiler proof applicability changed")
            except (KeyError, ValueError) as error:
                failures.append(f"{source.name}: {error}")
            rust = compile_source(compiler, source, "rust", output / "rust")
            row["rust_compile"] = "success" if rust.returncode == 0 else "rejection"
            if row["rust_compile"] != expected["expected_rust"]:
                failures.append(f"{source.name}: Rust compile status changed")
            if rust.returncode:
                row["rust_diagnostic"] = rust.stderr.strip()
                if expected.get("rust_diagnostic_contains", "") not in rust.stderr:
                    failures.append(f"{source.name}: expected Rust rejection diagnostic missing")
            else:
                report_path = output / "rust/contract/rust-capabilities.json"
                if not report_path.is_file():
                    failures.append(f"{source.name}: successful Rust compile has no capability report")
                else:
                    report = json.loads(report_path.read_text())
                    row["rust_capability_schema"] = report.get("schema_version")
                    row["rust_capabilities"] = [{key: item[key] for key in
                                                 ("name", "proof_required", "recording_status")}
                                                for item in report["circuits"]]
                    if report.get("schema_version") != 3:
                        failures.append(f"{source.name}: expected Rust capability schema 3")
                    for capability in report["circuits"]:
                        proof = proof_map(circuits).get(capability["name"])
                        if proof is None or capability.get("proof_required") is not proof[1]:
                            failures.append(f"{source.name}.{capability['name']}: Rust proof flag disagrees")
            receipt["sources"].append(row)
    receipt["summary"] = {
        "positive_sources": len(manifest["positive_sources"]),
        "expected_rejections": len(manifest["expected_rejections"]),
        "ts_positive_compiled": sum(row["ts_compile"] == "success" for row in receipt["sources"]),
        "rust_positive_compiled": sum(row.get("rust_compile") == "success" for row in receipt["sources"]),
        "rust_positive_rejected": sum(row.get("rust_compile") == "rejection" for row in receipt["sources"]),
        "compiler_proof_true": sum(circuit["proof"] is True for row in receipt["sources"]
                                   for circuit in row.get("proof_circuits", [])),
        "compiler_proof_false": sum(circuit["proof"] is False for row in receipt["sources"]
                                    for circuit in row.get("proof_circuits", [])),
        "known_lexical_pure_omissions": sum(item["known_lexical_pure_omissions"]
                                           for item in manifest["positive_sources"]),
    }
    receipt["failures"] = failures
    return receipt, failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    compiler = args.compiler.resolve()
    if not compiler.is_file():
        parser.error(f"compiler not found: {compiler}")
    receipt, failures = check(compiler)
    args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(json.dumps(receipt["summary"], sort_keys=True))
    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
