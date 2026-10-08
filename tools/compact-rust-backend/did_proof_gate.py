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

"""Run original DID proof lifecycles with one frozen artifact inventory."""
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
SOURCE_ROOT = Path("examples/rust_backend/did_adoption")
SOURCE = SOURCE_ROOT / "packages/contract/src/did.compact"
FIXTURE = Path("tests-rust-backend/did-adoption/lib.rs")
KEYS = ("rotateControllerKey", "recoverControllerKey", "deactivate", "setAlsoKnownAs",
        "setService", "removeService", "setSchnorrJubjubVerificationMethod",
        "removeSchnorrJubjubVerificationMethod", "setVerificationMethod",
        "removeVerificationMethod", "verifySchnorrJubjubDigestSignature")
SCENARIOS = {
    "points": {"selector": "--did-point-lifecycle", "installed_operations": list(KEYS[:3]),
               "cases": ["rotate", "recover", "deactivate"], "operations": list(KEYS[:3])},
    "aliases": {"selector": "--did-alias-lifecycle", "installed_operations": list(KEYS[:4]),
                "cases": ["insert-unicode", "remove-unicode"],
                "operations": ["setAlsoKnownAs", "setAlsoKnownAs"]},
    "services": {"selector": "--did-service-lifecycle", "installed_operations": list(KEYS[:6]),
                 "cases": ["insert-unicode", "update-empty-fields", "remove-unicode"],
                 "operations": ["setService", "setService", "removeService"]},
    "schnorr-methods": {"selector": "--did-schnorr-method-lifecycle",
                        "installed_operations": list(KEYS[:8]),
                        "cases": ["insert-unicode", "update-point", "remove-unicode"],
                        "operations": ["setSchnorrJubjubVerificationMethod",
                                       "setSchnorrJubjubVerificationMethod",
                                       "removeSchnorrJubjubVerificationMethod"]},
    "jwk-methods": {"selector": "--did-jwk-method-lifecycle",
                    "installed_operations": list(KEYS[:-1]),
                    "cases": ["insert-unicode", "update-jwk", "remove-unicode"],
                    "operations": ["setVerificationMethod", "setVerificationMethod",
                                   "removeVerificationMethod"]},
    "digest": {"selector": "--did-digest-verification",
               "installed_operations": ["setSchnorrJubjubVerificationMethod",
                                        "verifySchnorrJubjubDigestSignature"],
               "cases": ["insert", "read-valid"],
               "operations": ["setSchnorrJubjubVerificationMethod",
                              "verifySchnorrJubjubDigestSignature"]},
}
EXPORTS = set(KEYS) | {"setVerificationMethod", "removeVerificationMethod",
    "setVerificationMethodRelation",
    "setService", "removeService"}


def require(condition, message):
    if not condition:
        raise common.GateError(message)


def read_json(path):
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"expected a JSON object: {path}")
    return value


def hashes(paths):
    return {str(p): common.sha256(p) for p in sorted(set(paths)) if p.is_file()}


def unchanged(inventory):
    for name, digest in inventory.items():
        require(Path(name).is_file() and common.sha256(Path(name)) == digest,
                f"frozen input changed: {name}")


def prerequisites(env):
    result = {}
    for name in ("MIDNIGHT_PP", "MIDNIGHT_LEDGER_TEST_STATIC_DIR"):
        path = Path(env.get(name, ""))
        require(bool(env.get(name)) and path.is_absolute() and path.is_dir()
                and os.access(path, os.R_OK), f"{name} must name an absolute readable directory")
        result[name] = str(path.resolve())
    zkir = shutil.which("zkir", path=env.get("PATH"))
    require(zkir is not None, "DID proof gate requires zkir on PATH")
    result["zkir"] = str(Path(zkir).resolve())
    return result


def dust_fixture_inventory(directory):
    paths = []
    for name, digest in ledger_static.FIXTURE_HASHES.items():
        data = directory / "dust" / name
        declaration = data.with_name(data.name + ".sha256")
        require(data.is_file() and common.sha256(data) == digest,
                f"upstream Dust fixture mismatch: {name}")
        words = declaration.read_text().split()
        require(bool(words) and words[0] == digest,
                f"upstream Dust digest declaration mismatch: {name}")
        paths.extend([data, declaration])
    prover_declaration = directory / "dust/spend.prover.sha256"
    words = prover_declaration.read_text().split()
    require(bool(words) and words[0] == ledger_static.PROVER_HASH,
            "upstream Dust prover digest declaration mismatch")
    return hashes(paths + [prover_declaration])


def source_inventory():
    manifest = ROOT / SOURCE_ROOT / "source-manifest.json"
    data = read_json(manifest)
    require(data.get("format") == "compact-did-adoption-sources/v1", "unknown DID source manifest")
    paths = [manifest]
    for row in data["files"]:
        path = (manifest.parent / row["path"]).resolve()
        require(path.is_relative_to(manifest.parent.resolve()), "DID source path escapes its closure")
        require(path.is_file() and common.sha256(path) == row["sha256"],
                f"DID source manifest mismatch: {row['path']}")
        paths.append(path)
    require((ROOT / SOURCE).resolve() in paths, "DID entrypoint omitted from source closure")
    for folder in ("runtime-rs", "runtime-rs-macros", "tools/compact-rust-backend/src",
                   "tools/compact-rust-proof-smoke", "tests-rust-backend/did-adoption"):
        paths.extend(p for p in (ROOT / folder).rglob("*") if p.suffix in (".rs", ".json", ".toml"))
    paths.extend([ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "tools/compact-rust-backend/Cargo.toml", Path(common.__file__), Path(ledger_static.__file__), Path(__file__)])
    return hashes(paths)


def capability_inventory(output):
    report = read_json(output / "contract/rust-capabilities.json")
    common.validate_report(report, ROOT / SOURCE)
    rows = common.proof_cross_tab(report, read_json(output / "compiler/contract-info.json"), ROOT / SOURCE)
    require(len(rows) == 12 and {r["name"] for r in rows} == EXPORTS,
            "DID gate requires the complete twelve-export capability inventory")
    require(all(r["proof"] for r in rows), "DID proof applicability changed")
    # All exports record; relation proof scenarios belong to did_relation_gate.py.
    require({r["name"] for r in rows if r["recorded"]} == EXPORTS,
            "DID recorded inventory differs from the reviewed twelve-export scope")
    return report


def validate_summary(summary, scenario):
    spec = SCENARIOS[scenario]
    require(summary.get("format") == "compact-did-proof-result/v1", "missing DID success format")
    for key, value in {"scenario": scenario, "selector": spec["selector"],
                       "installed_operations": spec["installed_operations"],
                       "strictness": "default", "deployment_applied": True,
                       "constructor_execution_proved": False}.items():
        require(summary.get(key) == value, f"DID summary mismatch: {key}")
    calls = summary.get("calls", [])
    require(isinstance(calls, list) and all(isinstance(row, dict) for row in calls),
            "DID summary calls must be an array of checked results")
    require([r.get("case") for r in calls] == spec["cases"], "DID summary omits or reorders cases")
    require([r.get("operation") for r in calls] == spec["operations"], "DID summary changes operations")
    for call in calls:
        require(type(call.get("proof_bytes")) is int and call["proof_bytes"] > 0,
                "DID summary lacks nonempty proof")
        require(call.get("changed_binding_rejected") is True and call.get("applied") is True
                and call.get("replay_refusal") == "IntentAlreadyExists", "DID checked result missing")
    require(summary.get("final_state_file") == f"did-{scenario}-final-state.bin",
            "DID final public-state path is not the fixed scenario path")


def run_gate(directory, compiler, scheme, target, environment=None, *, command=common.run,
             snapshot=common.stable_copy):
    directory = directory.resolve()
    directory.mkdir(parents=True, exist_ok=False, mode=0o700)
    receipt = {"format": "compact-did-proof-gate/v1", "status": "failed", "commands": [],
               "scenarios": {}, "key_operations": list(KEYS), "required_proof_call_count": 16,
               "scope": "offline default-strict deployment/calls; constructor execution unproved; eleven of twelve recorded exports"}
    env = dict(os.environ if environment is None else environment)
    try:
        receipt["prerequisites"] = prerequisites(env)
        receipt["source_hashes"] = source_inventory()
        receipt["dust_fixture_hashes"] = dust_fixture_inventory(
            Path(receipt["prerequisites"]["MIDNIGHT_LEDGER_TEST_STATIC_DIR"]))
        receipt["git_head"] = common.git_head()
        (directory / "bin").mkdir()
        components = receipt["components"] = {}
        for name, path in (("compactc", compiler), ("compactc-scheme", scheme),
                           ("zkir", Path(receipt["prerequisites"]["zkir"]))):
            components[name] = snapshot(path, directory / "bin" / name)
        env.update({"COMPACTC_SCHEME": components["compactc-scheme"]["snapshot"],
                    "COMPACT_RUST_RUNTIME_DIR": str(ROOT), "CARGO_TARGET_DIR": str(target.resolve()),
                    "CARGO_INCREMENTAL": "0", "RUSTUP_TOOLCHAIN": "1.99.0",
                    "PYTHONDONTWRITEBYTECODE": "1"})
        def execute(argv, label, stdout_path=None):
            command([str(v) for v in argv], label, directory, receipt, env=env, stdout_path=stdout_path)
        output = directory / "did"
        execute([components["zkir"]["snapshot"], "--version"], "zkir-version")
        execute([components["compactc"]["snapshot"], "--target", "rust", "--skip-zk",
                 "--rust-runtime-root", ROOT, ROOT / SOURCE, output], "compile-did")
        execute(["rustfmt", "--edition", "2024", output / "contract/lib.rs"], "format-did")
        require((output / "contract/lib.rs").read_bytes() == (ROOT / FIXTURE).read_bytes(),
                "fresh DID fixture differs from the linked proof fixture")
        receipt["capabilities"] = capability_inventory(output)
        metadata = read_json(output / "compiler/rust-compatibility.json")
        require(metadata.get("compatibility") == read_json(ROOT / "runtime-rs/compatibility.json"),
                "generated runtime compatibility differs from current source record")
        receipt["generated_metadata"] = metadata
        execute(["cargo", "+1.99.0", "build", "--locked", "--offline", "-p", "compact-rust-proof-smoke"],
                "build-proof-runner")
        components["proof-runner"] = snapshot(target.resolve() / "debug/compact-rust-proof-smoke",
                                                directory / "bin/compact-rust-proof-smoke")
        runner = components["proof-runner"]["snapshot"]
        material = directory / "proof-material.json"
        execute([runner, "--prepare-proof-material"], "prepare-proof-material", material)
        receipt["proof_material"] = read_json(material)
        prepared = receipt["proof_material"]
        require(prepared.get("format") == "compact-proof-material/v1" and prepared.get("mode") == "prepare",
                "proof preparation did not emit its structured receipt")
        require(Path(prepared["cache_directory"]).resolve() == Path(receipt["prerequisites"]["MIDNIGHT_PP"]),
                "proof preparation used a different material directory")
        require(prepared.get("asset_count") == len(prepared["assets"]) > 0
                and prepared.get("parameter_count") == len(prepared["parameters"]) > 0,
                "proof preparation receipt omits its material inventory")
        (output / "keys").mkdir(exist_ok=True)
        receipt["keygen"] = []
        for operation in KEYS:
            execute([components["zkir"]["snapshot"], "compile", output / f"zkir/{operation}.zkir",
                     output / f"keys/{operation}.prover", output / f"keys/{operation}.verifier"],
                    f"keygen-{operation}")
            log = Path(receipt["commands"][-1]["log"]).read_text()
            model = re.search(r"\(k=(\d+), rows=(\d+)\)", log)
            require(model is not None, f"missing measured key model: {operation}")
            receipt["keygen"].append({"operation": operation, "k": int(model[1]), "rows": int(model[2])})
        artifacts = [output / f"keys/{op}.{ext}" for op in KEYS for ext in ("prover", "verifier")]
        artifacts += [output / f"zkir/{op}.{ext}" for op in KEYS for ext in ("zkir", "bzkir")]
        require(all(p.is_file() and p.stat().st_size > 0 for p in artifacts), "missing generated proof material")
        cache = Path(receipt["prerequisites"]["MIDNIGHT_PP"])
        names = [row["name"] for row in receipt["proof_material"]["assets"] + receipt["proof_material"]["parameters"]]
        names += [f"bls_midnight_2p{row['k']}" for row in receipt["keygen"]]
        parameter_paths = [cache / name for name in names]
        require(all(p.is_file() for p in parameter_paths), "explicit proof cache is missing required material")
        receipt["material_hashes"] = hashes(artifacts + parameter_paths)
        receipt["result_hashes"] = {}
        for scenario, spec in SCENARIOS.items():
            summary_path = output / f"did-{scenario}-result.json"
            require(not summary_path.exists() and not (output / f"did-{scenario}-final-state.bin").exists(),
                    "stale DID scenario result")
            execute([runner, spec["selector"], output], f"prove-{scenario}")
            summary = read_json(summary_path)
            validate_summary(summary, scenario)
            final_state = output / summary["final_state_file"]
            require(final_state.is_file() and not final_state.is_symlink() and final_state.stat().st_size > 0,
                    "DID final public-state artifact missing or invalid")
            summary["final_state_sha256"] = common.sha256(final_state)
            receipt["scenarios"][scenario] = summary
            receipt["result_hashes"].update(hashes([summary_path, final_state]))
        require(set(receipt["scenarios"]) == set(SCENARIOS), "required DID scenario omitted")
        unchanged(receipt["source_hashes"])
        require(source_inventory() == receipt["source_hashes"], "source inventory additions or removals detected")
        unchanged(receipt["dust_fixture_hashes"])
        require(common.git_head() == receipt["git_head"], "git HEAD changed during DID gate")
        unchanged(receipt["material_hashes"])
        unchanged(receipt["result_hashes"])
        unchanged({v["snapshot"]: v["sha256"] for v in components.values()})
        receipt["status"] = "passed"
    except (common.GateError, OSError, ValueError, KeyError) as error:
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
    try:
        receipt = run_gate(args.run_dir, args.compiler, args.scheme, args.cargo_target_dir)
    except OSError as error:
        print(str(error), file=sys.stderr)
        return 1
    print(f"DID proof gate {receipt['status']}; receipt {args.run_dir / 'receipt.json'}")
    return 0 if receipt["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
