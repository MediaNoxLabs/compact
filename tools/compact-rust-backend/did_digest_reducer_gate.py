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

"""Strict proof for the maintained renamed/reordered String–Point Map reducer."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import sys

import local_parity_gate as common
import resolve_ledger_test_static as ledger_static

ROOT = common.ROOT
SOURCE = Path("examples/rust_backend/did_digest_read_reducer/contract.compact")
FIXTURE = Path("tests-rust-backend/did-digest-read-reducer/lib.rs")
FIXTURE_ROOT = Path("tests-rust-backend/did-digest-read-reducer")
NAME = "verify"


def require(condition, message):
    if not condition:
        raise common.GateError(message)


def read_json(path):
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"expected a JSON object: {path}")
    return value


def hashes(paths):
    return {str(path): common.sha256(path) for path in sorted(set(paths)) if path.is_file()}


def source_inventory():
    paths = [ROOT / SOURCE, ROOT / "Cargo.toml", ROOT / "Cargo.lock",
             ROOT / "tools/compact-rust-proof-smoke/Cargo.toml", Path(__file__),
             Path(common.__file__), Path(ledger_static.__file__)]
    for folder in ("runtime-rs", "tools/compact-rust-backend/src",
                   "tools/compact-rust-proof-smoke/src", FIXTURE_ROOT):
        paths.extend(path for path in (ROOT / folder).rglob("*")
                     if path.suffix in (".rs", ".json", ".toml", ".mjs"))
    return hashes(paths)


def validate_summary(value):
    require(value.get("status") == "passed" and value.get("strictness") == "default",
            "reducer did not pass default strictness")
    for field in ("deployed_constructor_data", "applied", "read_only_contract_data",
                  "changed_binding_rejected"):
        require(value.get(field) is True, f"reducer summary missing {field}")
    require(value.get("constructor_execution_proved") is False,
            "reducer constructor proof boundary changed")
    require(value.get("replay_refusal") == "IntentAlreadyExists",
            "reducer replay refusal changed")
    require(type(value.get("proof_bytes")) is int and value["proof_bytes"] > 0,
            "reducer proof is empty")
    require(type(value.get("final_state_bytes")) is int and value["final_state_bytes"] > 0,
            "reducer final state is empty")


def run_gate(directory, compiler, scheme, target, environment=None, *, command=common.run,
             snapshot=common.stable_copy):
    directory = directory.resolve()
    directory.mkdir(parents=True, exist_ok=False, mode=0o700)
    receipt = {"format": "compact-did-digest-reducer-gate/v1", "status": "failed", "commands": [],
               "scope": "maintained generic String–Point Map reducer; default-strict deployment/read, constructor execution unproved"}
    env = dict(os.environ if environment is None else environment)
    try:
        for key in ("MIDNIGHT_PP", "MIDNIGHT_LEDGER_TEST_STATIC_DIR"):
            path = Path(env.get(key, ""))
            require(bool(env.get(key)) and path.is_absolute() and path.is_dir(),
                    f"{key} must be an absolute existing directory")
        zkir = shutil.which("zkir", path=env.get("PATH"))
        require(zkir is not None, "reducer proof gate requires zkir")
        static_dir = Path(env["MIDNIGHT_LEDGER_TEST_STATIC_DIR"])
        dust_paths = []
        for name, digest in ledger_static.FIXTURE_HASHES.items():
            path = static_dir / "dust" / name
            declaration = path.with_name(path.name + ".sha256")
            require(path.is_file() and common.sha256(path) == digest,
                    f"upstream Dust fixture mismatch: {name}")
            require(declaration.read_text().split()[0] == digest,
                    f"upstream Dust digest declaration mismatch: {name}")
            dust_paths += [path, declaration]
        receipt["dust_fixture_hashes"] = hashes(dust_paths)
        receipt["source_hashes"] = source_inventory()
        receipt["git_head"] = common.git_head()
        (directory / "bin").mkdir()
        components = receipt["components"] = {}
        for name, path in (("compactc", compiler), ("compactc-scheme", scheme), ("zkir", Path(zkir))):
            components[name] = snapshot(path, directory / "bin" / name)
        env.update({"COMPACTC_SCHEME": components["compactc-scheme"]["snapshot"],
                    "COMPACT_RUST_RUNTIME_DIR": str(ROOT), "CARGO_TARGET_DIR": str(target.resolve()),
                    "CARGO_INCREMENTAL": "0", "RUSTUP_TOOLCHAIN": "1.99.0",
                    "PYTHONDONTWRITEBYTECODE": "1"})
        def execute(argv, label, stdout_path=None):
            command([str(arg) for arg in argv], label, directory, receipt, env=env,
                    stdout_path=stdout_path)
        output = directory / "reducer"
        execute([components["compactc"]["snapshot"], "--target", "rust", "--skip-zk",
                 "--rust-runtime-root", ROOT, ROOT / SOURCE, output], "compile-reducer")
        execute(["rustfmt", "--edition", "2024", output / "contract/lib.rs"], "format-reducer")
        require((output / "contract/lib.rs").read_bytes() == (ROOT / FIXTURE).read_bytes(),
                "fresh reducer fixture differs")
        report = read_json(output / "contract/rust-capabilities.json")
        common.validate_report(report, ROOT / SOURCE)
        rows = common.proof_cross_tab(report, read_json(output / "compiler/contract-info.json"),
                                      ROOT / SOURCE)
        require(len(rows) == 1 and rows[0]["name"] == NAME and rows[0]["proof"]
                and rows[0]["recorded"] and rows[0]["observed_call"],
                "renamed reducer proof/recording capability changed")
        receipt["capabilities"] = report
        execute(["cargo", "+1.99.0", "test", "--locked", "--offline", "-p",
                 "compact-rust-did-digest-read-reducer-fixture"], "test-reducer-behavior")
        execute(["cargo", "+1.99.0", "build", "--locked", "--offline", "-p",
                 "compact-rust-proof-smoke"], "build-proof-runner")
        components["proof-runner"] = snapshot(target.resolve() / "debug/compact-rust-proof-smoke",
                                               directory / "bin/compact-rust-proof-smoke")
        runner = components["proof-runner"]["snapshot"]
        material = directory / "proof-material.json"
        execute([runner, "--prepare-proof-material"], "prepare-proof-material", material)
        prepared = read_json(material)
        require(prepared.get("format") == "compact-proof-material/v1"
                and prepared.get("mode") == "prepare"
                and Path(prepared["cache_directory"]).resolve() == Path(env["MIDNIGHT_PP"]).resolve(),
                "proof material preparation changed cache or format")
        receipt["proof_material"] = prepared
        (output / "keys").mkdir(exist_ok=True)
        execute([components["zkir"]["snapshot"], "compile", output / "zkir/verify.zkir",
                 output / "keys/verify.prover", output / "keys/verify.verifier"], "keygen-reducer")
        match = re.search(r"\(k=(\d+), rows=(\d+)\)",
                          Path(receipt["commands"][-1]["log"]).read_text())
        require(match is not None and (int(match[1]), int(match[2])) == (7, 64),
                "reducer key size/rows changed without review")
        receipt["keygen"] = {"operation": NAME, "k": int(match[1]), "rows": int(match[2])}
        artifacts = [output / f"keys/verify.{ext}" for ext in ("prover", "verifier")]
        artifacts += [output / f"zkir/verify.{ext}" for ext in ("zkir", "bzkir")]
        require(all(path.is_file() and path.stat().st_size > 0 for path in artifacts),
                "missing reducer proof artifacts")
        receipt["artifact_hashes"] = hashes(artifacts)
        execute([runner, "--did-digest-reducer", output], "prove-reducer")
        summary = read_json(output / "reducer-proof-result.json")
        validate_summary(summary)
        final_state = output / "reducer-final-state.bin"
        require(final_state.is_file() and final_state.stat().st_size == summary["final_state_bytes"],
                "reducer final state mismatch")
        receipt["summary"] = summary
        receipt["result_hashes"] = hashes([output / "reducer-proof-result.json", final_state])
        require(source_inventory() == receipt["source_hashes"] and common.git_head() == receipt["git_head"],
                "source inventory or HEAD changed during reducer gate")
        require(hashes(artifacts) == receipt["artifact_hashes"]
                and hashes([output / "reducer-proof-result.json", final_state]) == receipt["result_hashes"],
                "reducer proof artifacts changed during gate")
        require(all(Path(row["snapshot"]).is_file() and
                    common.sha256(Path(row["snapshot"])) == row["sha256"]
                    for row in components.values()), "frozen tool changed during reducer gate")
        receipt["status"] = "passed"
    except (common.GateError, OSError, ValueError, KeyError, IndexError) as error:
        receipt["error"] = str(error)
    finally:
        (directory / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, required=True)
    parser.add_argument("--scheme", type=Path, required=True)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--cargo-target-dir", type=Path, required=True)
    args = parser.parse_args()
    receipt = run_gate(args.run_dir, args.compiler, args.scheme, args.cargo_target_dir)
    print(f"DID digest reducer gate {receipt['status']}; receipt {args.run_dir / 'receipt.json'}")
    return 0 if receipt["status"] == "passed" else 1

if __name__ == "__main__":
    raise SystemExit(main())
