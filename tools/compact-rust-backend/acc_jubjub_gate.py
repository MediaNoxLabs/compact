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

"""Qualify the bounded ACC Jubjub Cell slice; no full-account adoption claim.

Fresh ledger8 TS is checked against retained independently captured original
Compact0.35/runtime0.20 point expectations. Only apply is proof-qualified; raw
has behavior success/refusal controls. Constructor data is deployed, but its
execution is not proved. Keys are generated fresh (no unchecked reuse mode).
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil

import local_parity_gate as common
import did_primitive_reducer_gate as primitive
import resolve_ledger_test_static as ledger_static

ROOT = common.ROOT
SOURCE = Path("examples/rust_backend/jubjub_scalar_cell.compact")
IR = Path("tools/compact-rust-backend/tests/jubjub-scalar-cell-schema20-ir.json")
FIXTURE_ROOT = Path("tests-rust-backend/jubjub-scalar-cell")
FIXTURE = FIXTURE_ROOT / "lib.rs"
REFERENCE = FIXTURE_ROOT / "oracle/cases.json"
CAPTURE = Path("tools/compact-rust-backend/acc_jubjub_capture.mjs")
CASES = ("zero", "one", "eight", "q_minus_one", "q", "q_plus_one", "native_max")
ROW = {"kind": "acc-jubjub-cell", "proof_cases": [
    {"case": name, "operation": "apply"} for name in CASES]}
require = primitive.require
read_json = primitive.read_json
hashes = primitive.hashes


def source_inventory():
    files = [ROOT / path for path in (SOURCE, IR, FIXTURE, REFERENCE, CAPTURE,
             "Cargo.toml", "Cargo.lock", "tools/compact-rust-proof-smoke/Cargo.toml",
             "tools/compact-rust-backend/Cargo.toml",
             "tools/compact-rust-backend/test_acc_jubjub_gate.py",
             "tools/compact-rust-backend/test_local_parity_gate.py")]
    files += [Path(__file__), Path(common.__file__), Path(primitive.__file__),
              Path(ledger_static.__file__)]
    require(all(path.is_file() for path in files), "missing maintained qualification input")
    for folder in ("runtime-rs", "runtime-rs-macros", "testkit-rs",
                   "tools/compact-rust-backend/src", "tools/compact-rust-proof-smoke/src",
                   FIXTURE_ROOT):
        files.extend(path for path in (ROOT / folder).rglob("*")
                     if path.suffix in (".rs", ".json", ".toml", ".compact", ".mjs"))
    return hashes(files)


def validate_capture(value):
    require(value.get("format") == "compact-acc-jubjub-wrapper-capture/v1"
            and value.get("kind") == ROW["kind"], "wrong capture identity")
    require([(r.get("id"), r.get("operation")) for r in value.get("cases", [])]
            == [(name, "apply") for name in CASES], "capture must contain exact seven apply cases")
    require([r.get("id") for r in value.get("rawCases", [])] == ["raw-q_minus_one", "raw-q"],
            "capture must contain raw canonical success and q refusal")
    require([r.get("id") for r in value.get("refusals", [])] == [
        "raw-q-generator", "raw-q-point", "negative", "native-modulus", "number-not-bigint"],
        "capture refusal controls changed")


def check_hashes(expected, message):
    require(bool(expected) and all(Path(path).is_file() and common.sha256(Path(path)) == digest
                                  for path, digest in expected.items()), message)


NODE_SHARED_OBJECTS = "console.log(JSON.stringify({sharedObjects:process.report.getReport().sharedObjects}))"


def node_library_identity(report):
    """Bind loaded Node libraries; macOS system-cache objects have no loose files."""
    paths = report.get("sharedObjects")
    require(isinstance(paths, list) and paths and all(isinstance(p, str) and Path(p).is_absolute() for p in paths),
            "invalid Node shared-library report")
    system = sorted(p for p in paths if p.startswith(("/System/Library/", "/usr/lib/")))
    external = sorted(set(paths) - set(system))
    require(external and all(Path(path).is_file() for path in external), "missing Node dynamic library")
    return {"file_hashes": hashes([Path(path) for path in external]),
            "system_libraries": system, "system_scope": "host OS libraries; names retained, not byte qualified"}


def run_gate(directory, compiler, scheme, target, environment=None, *, command=common.run,
             snapshot=common.stable_copy):
    directory = directory.resolve()
    directory.mkdir(parents=True, exist_ok=False, mode=0o700)
    receipt = {"format": "compact-acc-jubjub-gate/v1", "status": "failed", "commands": [],
               "scope": __doc__, "expected_proof_cases": ROW["proof_cases"],
               "oracle_mode": "retained-original-reference"}
    env = dict(os.environ if environment is None else environment)
    try:
        for key in ("MIDNIGHT_PP", "MIDNIGHT_LEDGER_TEST_STATIC_DIR"):
            path = Path(env.get(key, ""))
            require(bool(env.get(key)) and path.is_absolute() and path.is_dir(),
                    f"{key} must be an absolute existing directory")
        discovered = {name: shutil.which(name, path=env.get("PATH")) for name in ("zkir", "node", "rustup")}
        require(all(discovered.values()), "ACC Jubjub gate requires zkir, node and rustup")
        dust_paths = []
        for name, digest in ledger_static.FIXTURE_HASHES.items():
            path = Path(env["MIDNIGHT_LEDGER_TEST_STATIC_DIR"]) / "dust" / name
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
        formatter_selection = directory / "rustfmt-path.txt"
        command([discovered["rustup"], "which", "--toolchain", "1.99.0", "rustfmt"],
                "select-rustfmt", directory, receipt, env=env, stdout_path=formatter_selection)
        formatter = Path(formatter_selection.read_text().strip())
        require(formatter.is_absolute() and formatter.is_file() and os.access(formatter, os.X_OK),
                "rustup did not resolve pinned rustfmt")
        # rustfmt loads toolchain-relative shared libraries, so moving its binary
        # breaks macOS. Bind the selected executable in place and recheck it.
        components["rustfmt"] = {"source": str(formatter), "snapshot": str(formatter),
                                  "sha256": common.sha256(formatter), "size": formatter.stat().st_size,
                                  "execution": "pinned toolchain executable in place"}
        del discovered["rustup"]
        node_path = Path(discovered.pop("node")).resolve()
        components["node"] = {"source": str(node_path), "snapshot": str(node_path),
                              "sha256": common.sha256(node_path), "size": node_path.stat().st_size,
                              "execution": "in place with loaded shared-library identities"}
        for name, path in (("compactc", compiler), ("compactc-scheme", scheme),
                           *((name, Path(path)) for name, path in discovered.items())):
            components[name] = snapshot(path, directory / "bin" / name)
        env.update({"COMPACTC_SCHEME": components["compactc-scheme"]["snapshot"],
                    "COMPACT_RUST_RUNTIME_DIR": str(ROOT), "CARGO_TARGET_DIR": str(target.resolve()),
                    "CARGO_INCREMENTAL": "0", "RUSTUP_TOOLCHAIN": "1.99.0",
                    "PYTHONDONTWRITEBYTECODE": "1"})

        def execute(argv, label, stdout_path=None):
            command([str(arg) for arg in argv], label, directory, receipt, env=env,
                    stdout_path=stdout_path)

        node = components["node"]["snapshot"]
        node_libraries = directory / "node-libraries.json"
        execute([node, "-e", NODE_SHARED_OBJECTS], "freeze-node-runtime", node_libraries)
        receipt["node_libraries"] = node_library_identity(read_json(node_libraries))
        runtime_inventory = directory / "runtime-inventory.json"
        execute([node, ROOT / CAPTURE, "--runtime-inventory"], "freeze-ts-runtime", runtime_inventory)
        receipt["ts_runtime_hashes"] = read_json(runtime_inventory)
        check_hashes(receipt["ts_runtime_hashes"], "invalid initial TS runtime inventory")
        reference = read_json(ROOT / REFERENCE)
        validate_capture(reference)
        output = directory / ROW["kind"]
        execute([components["compactc"]["snapshot"], "--target", "ts", "--target", "rust", "--skip-zk",
                 "--rust-runtime-root", ROOT, ROOT / SOURCE, output], "compile-jubjub")
        # Keep generated source untouched; the maintained fixture uses pinned rustfmt.
        generated_source_hash = common.sha256(output / "contract/lib.rs")
        formatted = directory / "formatted-fixture.rs"
        formatted.write_bytes((output / "contract/lib.rs").read_bytes())
        execute([components["rustfmt"]["snapshot"], "--edition", "2024", formatted],
                "compare-formatted-fixture")
        require(common.sha256(output / "contract/lib.rs") == generated_source_hash,
                "formatter changed compiler-generated source")
        require(formatted.read_bytes() == (ROOT / FIXTURE).read_bytes(), "generated Jubjub fixture differs")
        require(read_json(output / "contract/compact-rust-ir.json") == read_json(ROOT / IR),
                "maintained Jubjub IR differs")
        report = read_json(output / "contract/rust-capabilities.json")
        common.validate_report(report, ROOT / SOURCE)
        cap = common.proof_cross_tab(report, read_json(output / "compiler/contract-info.json"), ROOT / SOURCE)
        require([row["name"] for row in cap] == ["apply", "raw"]
                and all(all(row.get(k) is True for k in ("proof", "recorded", "observed_call")) for row in cap),
                "Jubjub apply/raw proof/recording capabilities changed")
        receipt["capabilities"] = report
        generated = [path for folder in ("contract", "compiler", "zkir")
                     for path in (output / folder).rglob("*") if path.is_file()]
        require((output / "zkir/apply.zkir").is_file(), "compiler did not emit apply ZKIR")
        receipt["generated_hashes"] = hashes(generated)
        capture = directory / "ts-capture.json"
        provenance = directory / "capture-provenance.json"
        execute([node, ROOT / CAPTURE, "--generated", output / "contract/index.js", "--reference", ROOT / REFERENCE,
                 "--boundaries", ROOT / FIXTURE_ROOT / "oracle/boundaries.json", "--source", ROOT / SOURCE,
                 "--provenance", provenance], "capture-ts-reference", capture)
        actual = read_json(capture)
        validate_capture(actual)
        require(actual == reference, "fresh ledger8 TS differs from retained independent oracle")
        provenance_data = read_json(provenance)
        require(provenance_data.get("format") == "compact-acc-jubjub-wrapper-provenance/v1"
                and provenance_data.get("oracleMode") == "retained-original-reference"
                and provenance_data.get("runtimeHashes") == receipt["ts_runtime_hashes"],
                "capture provenance or runtime selection changed")
        input_hashes = {str(Path(row["path"]).resolve()): row["sha256"] for row in provenance_data["inputs"]}
        required_inputs = {str(path.resolve()) for path in (ROOT / CAPTURE, ROOT / SOURCE, ROOT / REFERENCE,
                           ROOT / FIXTURE_ROOT / "oracle/boundaries.json", output / "contract/index.js")}
        require(required_inputs <= input_hashes.keys(), "capture provenance omitted required input")
        check_hashes(input_hashes, "capture input identity mismatch")
        capture_hashes = hashes([capture, provenance, runtime_inventory, formatted, formatter_selection, node_libraries])
        execute(["cargo", "+1.99.0", "test", "--locked", "--offline", "-j4", "-p",
                 "compact-rust-jubjub-scalar-cell-fixture", "--all-features"], "test-jubjub-behavior")
        execute(["cargo", "+1.99.0", "build", "--locked", "--offline", "-j4", "-p",
                 "compact-rust-proof-smoke"], "build-proof-runner")
        components["proof-runner"] = snapshot(target.resolve() / "debug/compact-rust-proof-smoke",
                                               directory / "bin/compact-rust-proof-smoke")
        runner = components["proof-runner"]["snapshot"]
        material = directory / "proof-material.json"
        execute([runner, "--prepare-proof-material"], "prepare-proof-material", material)
        prepared = read_json(material)
        require(prepared.get("format") == "compact-proof-material/v1" and prepared.get("mode") == "prepare"
                and Path(prepared["cache_directory"]).resolve() == Path(env["MIDNIGHT_PP"]).resolve(),
                "proof material selection changed")
        receipt["proof_material"] = prepared
        material_hashes = hashes([material])
        (output / "keys").mkdir(exist_ok=True)
        execute([components["zkir"]["snapshot"], "compile", output / "zkir/apply.zkir",
                 output / "keys/apply.prover", output / "keys/apply.verifier"], "keygen-apply")
        match = re.search(r"\(k=(\d+), rows=(\d+)\)", Path(receipt["commands"][-1]["log"]).read_text())
        require(match is not None and (int(match[1]), int(match[2])) == (11, 1190),
                "apply key shape changed without review")
        receipt["keygen"] = {"operation": "apply", "k": 11, "rows": 1190}
        artifacts = [output / f"{folder}/apply.{ext}" for folder, ext in (
            ("keys", "prover"), ("keys", "verifier"), ("zkir", "zkir"), ("zkir", "bzkir"))]
        require(all(path.is_file() and path.stat().st_size > 0 for path in artifacts),
                "missing or empty apply proof material")
        receipt["artifact_hashes"] = hashes(artifacts)
        execute([runner, "--did-primitive-reducer", ROW["kind"], output], "prove-apply")
        summary = read_json(output / "proof-result.json")
        primitive.validate_summary(summary, ROW)
        results = [output / "proof-result.json"]
        for call in summary["calls"]:
            state = output / f"{call['case']}-state.bin"
            require(state.is_file() and state.stat().st_size == call["state_bytes"],
                    f"{call['case']}: accepted state bytes mismatch")
            results.append(state)
        receipt["summary"] = summary
        receipt["result_hashes"] = hashes(results) | capture_hashes | material_hashes
        node_recheck = directory / "node-libraries-final.json"
        execute([node, "-e", NODE_SHARED_OBJECTS], "recheck-node-runtime", node_recheck)
        require(node_library_identity(read_json(node_recheck)) == receipt["node_libraries"],
                "Node shared-library identities changed during gate")
        receipt["result_hashes"].update(hashes([node_recheck]))
        require(source_inventory() == receipt["source_hashes"] and common.git_head() == receipt["git_head"],
                "source inventory or HEAD changed during gate")
        check_hashes(input_hashes, "capture inputs changed during gate")
        for field in ("generated_hashes", "ts_runtime_hashes", "dust_fixture_hashes", "artifact_hashes", "result_hashes"):
            check_hashes(receipt[field], f"{field} changed during gate")
        require(all(Path(row["snapshot"]).is_file() and common.sha256(Path(row["snapshot"])) == row["sha256"]
                    for row in components.values()), "frozen tool changed during gate")
        receipt["status"] = "passed"
    except (common.GateError, OSError, ValueError, KeyError, IndexError, TypeError) as error:
        receipt["error"] = str(error)
    finally:
        (directory / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("compiler", "scheme", "target", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    receipt = run_gate(args.output, args.compiler, args.scheme, args.target)
    print(f"ACC Jubjub gate {receipt['status']}; receipt {args.output / 'receipt.json'}")
    return 0 if receipt["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
