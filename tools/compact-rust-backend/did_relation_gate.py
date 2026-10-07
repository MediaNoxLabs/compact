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

"""Qualify the original DID relation and three maintained structural reducers."""
import argparse
import json
import os
from pathlib import Path
import shutil
import sys

import did_proof_gate as did
import local_parity_gate as common
import resolve_ledger_test_static as ledger_static

ROOT = common.ROOT
RELATION = "setVerificationMethodRelation"
ORIGINAL_OPERATIONS = (
    "setVerificationMethod", "removeVerificationMethod", RELATION,
    "setSchnorrJubjubVerificationMethod", "verifySchnorrJubjubDigestSignature",
)
SCENARIOS = {
    "relations": (
        "--did-relation-lifecycle",
        [*(f"accept-{name}" for name in ("ed", "x", "bls1", "bls2", "p256", "secp")),
         *(f"relation-{index}-{action}" for index in range(1, 6) for action in ("insert", "remove"))],
        ["setVerificationMethod"] * 6 + [RELATION] * 10,
    ),
    "relation-schnorr": (
        "--did-relation-schnorr", ["insert", "read-valid", "relation-insert"],
        ["setSchnorrJubjubVerificationMethod", "verifySchnorrJubjubDigestSignature", RELATION],
    ),
}
REDUCERS = {
    "relation-two": ("did_relation_two_reducer", "did-relation-two-reducer", "update"),
    "relation-four": ("did_relation_four_reducer", "did-relation-four-reducer", "update"),
    "relation-nested": ("did_relation_nested_reducer", "did-relation-nested-reducer", "check"),
}


def require(condition, message):
    if not condition:
        raise common.GateError(message)


def read_json(path):
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"expected JSON object: {path}")
    return value


def hashes(paths):
    return {str(path): common.sha256(path) for path in sorted(set(paths))}


def checked_file(path):
    require(path.is_file() and not path.is_symlink() and path.stat().st_size > 0,
            f"missing, linked or empty proof artifact: {path}")
    return path


def validate_original(summary, scenario):
    selector, cases, operations = SCENARIOS[scenario]
    installed = (["setVerificationMethod", "removeVerificationMethod", RELATION]
                 if scenario == "relations" else
                 ["setSchnorrJubjubVerificationMethod", "verifySchnorrJubjubDigestSignature", RELATION])
    require(summary.get("format") == "compact-did-proof-result/v1"
            and summary.get("scenario") == scenario and summary.get("selector") == selector,
            "wrong original DID relation scenario")
    require(summary.get("installed_operations") == installed
            and summary.get("strictness") == "default" and summary.get("deployment_applied") is True
            and summary.get("constructor_execution_proved") is False,
            "original relation strictness/deployment boundary changed")
    calls = summary.get("calls")
    require(isinstance(calls, list) and [(c.get("case"), c.get("operation")) for c in calls]
            == list(zip(cases, operations)), "original relation call inventory changed")
    for call in calls:
        require(type(call.get("proof_bytes")) is int and call["proof_bytes"] > 0
                and call.get("applied") is True and call.get("changed_binding_rejected") is True
                and call.get("replay_refusal") == "IntentAlreadyExists",
                f"original relation proof/apply/replay failed: {call.get('case')}")
    require(summary.get("final_state_file") == f"did-{scenario}-final-state.bin",
            "original relation final state path changed")


def validate_reducer(summary, kind):
    require(summary.get("format") == "compact-did-primitive-reducer-proof/v1"
            and summary.get("kind") == kind and summary.get("status") == "passed"
            and summary.get("strictness") == "default"
            and summary.get("constructor_data_deployed") is True
            and summary.get("constructor_execution_proved") is False,
            "relation reducer identity/strictness changed")
    cases = {
        "relation-two": ("authentication-insert", "authentication-remove"),
        "relation-four": ("delegation-insert", "delegation-remove"),
        "relation-nested": ("missing-signing-short-circuit", "x25519-check"),
    }[kind]
    operation = REDUCERS[kind][2]
    calls = summary.get("calls")
    require(isinstance(calls, list) and [(c.get("case"), c.get("operation")) for c in calls]
            == [(case, operation) for case in cases], "relation reducer call inventory changed")
    for call in calls:
        require(type(call.get("proof_bytes")) is int and call["proof_bytes"] > 0
                and type(call.get("state_bytes")) is int and call["state_bytes"] > 0
                and all(call.get(flag) is True for flag in ("applied", "changed_binding_rejected",
                    "recorded_state_matches", "replay_unchanged"))
                and call.get("replay_refusal") == "IntentAlreadyExists",
                f"relation reducer proof/apply/replay failed: {call.get('case')}")


def source_inventory():
    # The original source is an imported closure, not just did.compact.
    original_closure = did.source_inventory()
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__),
             Path(common.__file__), Path(did.__file__), Path(ledger_static.__file__),
             ROOT / did.SOURCE, ROOT / did.FIXTURE,
             ROOT / "examples/rust_backend/did_adoption/source-manifest.json",
             ROOT / "tools/compact-rust-backend/did_relation_capture.mjs",
             ROOT / "tools/compact-rust-proof-smoke/Cargo.toml"]
    paths.extend((ROOT / "examples/rust_backend/did_adoption").rglob("*.compact"))
    paths += [ROOT / f"examples/rust_backend/{stem}.compact" for stem, _, _ in REDUCERS.values()]
    for folder in ("runtime-rs", "runtime-rs-macros", "tools/compact-rust-backend/src",
                   "tools/compact-rust-proof-smoke/src", "tests-rust-backend/did-adoption",
                   *(f"tests-rust-backend/{name}" for _, name, _ in REDUCERS.values())):
        paths.extend(p for p in (ROOT / folder).rglob("*") if p.suffix in (".rs", ".json", ".toml"))
    inventory = hashes(checked_file(path) for path in paths)
    inventory.update(original_closure)
    return inventory


def copy_or_generate(source, destination, generate):
    destination.parent.mkdir(parents=True, exist_ok=True)
    if source is None:
        generate()
    else:
        shutil.copyfile(checked_file(source), destination)
    return checked_file(destination)


def run_gate(directory, compiler, scheme, target, *, reuse_material=None, reuse_reducers=None,
             public_dir=None, environment=None, command=common.run, snapshot=common.stable_copy):
    directory = directory.resolve()
    directory.mkdir(parents=True, exist_ok=False, mode=0o700)
    receipt = {"format": "compact-did-relation-gate/v1", "status": "failed", "commands": [],
               "required_original_calls": 19, "required_reducer_calls": 6,
               "scope": "original relation and generic reducers, default-strict proof/apply/replay; constructor execution unproved",
               "scenarios": {}, "reducers": {}}
    env = dict(os.environ if environment is None else environment)
    try:
        prereq = did.prerequisites(env)
        receipt["prerequisites"] = prereq
        receipt["dust_fixture_hashes"] = did.dust_fixture_inventory(Path(prereq["MIDNIGHT_LEDGER_TEST_STATIC_DIR"]))
        receipt["source_hashes"] = source_inventory()
        receipt["git_head"] = common.git_head()
        components = receipt["components"] = {}
        (directory / "bin").mkdir()
        for name, path in (("compactc", compiler), ("compactc-scheme", scheme),
                           ("zkir", Path(prereq["zkir"]))):
            components[name] = snapshot(path, directory / "bin" / name)
        env.update({"COMPACTC_SCHEME": components["compactc-scheme"]["snapshot"],
                    "COMPACT_RUST_RUNTIME_DIR": str(ROOT), "CARGO_TARGET_DIR": str(target.resolve()),
                    "CARGO_INCREMENTAL": "0", "RUSTUP_TOOLCHAIN": "1.99.0",
                    "PYTHONDONTWRITEBYTECODE": "1"})
        if public_dir is not None:
            public_dir = public_dir.resolve()
            require(public_dir.is_dir() and not any(public_dir.iterdir()),
                    "public interchange destination must exist and be empty")
            env["COMPACT_DID_PUBLIC_INTERCHANGE"] = str(public_dir)
            receipt["public_interchange_directory"] = str(public_dir)
        else:
            env.pop("COMPACT_DID_PUBLIC_INTERCHANGE", None)
        def execute(argv, label, stdout=None):
            command([str(arg) for arg in argv], label, directory, receipt, env=env, stdout_path=stdout)
        original = directory / "did"
        execute([components["compactc"]["snapshot"], "--target", "rust", "--skip-zk",
                 "--rust-runtime-root", ROOT, ROOT / did.SOURCE, original], "compile-original-did")
        execute(["rustfmt", "--edition", "2024", original / "contract/lib.rs"], "format-original-did")
        require((original / "contract/lib.rs").read_bytes() == (ROOT / did.FIXTURE).read_bytes(),
                "original DID generated Rust differs from reviewed fixture")
        caps = read_json(original / "contract/rust-capabilities.json")
        common.validate_report(caps, ROOT / did.SOURCE)
        tab = common.proof_cross_tab(caps, read_json(original / "compiler/contract-info.json"), ROOT / did.SOURCE)
        require(len(tab) == 12 and {r["name"] for r in tab} == did.EXPORTS
                and all(r["proof"] and r["recorded"] and r["observed_call"] for r in tab),
                "original DID twelve-export proof/recording inventory changed")
        receipt["original_capabilities"] = caps
        execute(["cargo", "+1.99.0", "build", "--locked", "--offline", "-j", "4", "-p",
                 "compact-rust-proof-smoke"], "build-proof-runner")
        components["proof-runner"] = snapshot(target.resolve() / "debug/compact-rust-proof-smoke",
                                               directory / "bin/compact-rust-proof-smoke")
        runner = components["proof-runner"]["snapshot"]
        material_path = directory / "proof-material.json"
        execute([runner, "--prepare-proof-material"], "prepare-proof-material", material_path)
        prepared = read_json(material_path)
        require(prepared.get("format") == "compact-proof-material/v1"
                and prepared.get("mode") == "prepare"
                and Path(prepared["cache_directory"]).resolve() == Path(prereq["MIDNIGHT_PP"]),
                "proof material preparation changed")
        receipt["proof_material"] = prepared
        # Verifiers for installed entrypoints are loaded even if a case never calls them.
        original_ops = sorted(ORIGINAL_OPERATIONS)
        artifacts = []
        for operation in original_ops:
            for subdir, extension in (("zkir", "zkir"), ("zkir", "bzkir"),
                                      ("keys", "prover"), ("keys", "verifier")):
                destination = original / f"{subdir}/{operation}.{extension}"
                source = reuse_material / f"{subdir}/{operation}.{extension}" if reuse_material else None
                if source is None and extension == "bzkir":
                    continue
                if source is None and extension in ("prover", "verifier"):
                    # The prover is generated once for each operation below.
                    continue
                if source is None:
                    require(destination.is_file(), f"compiler omitted ZKIR: {operation}")
                else:
                    if extension == "zkir":
                        require(common.sha256(checked_file(source)) == common.sha256(checked_file(destination)),
                                f"{operation}: retained ZKIR differs from fresh compiler output")
                    copy_or_generate(source, destination, lambda: None)
            if reuse_material is None:
                execute([components["zkir"]["snapshot"], "compile",
                         original / f"zkir/{operation}.zkir", original / f"keys/{operation}.prover",
                         original / f"keys/{operation}.verifier"], f"keygen-{operation}")
            artifacts.extend(checked_file(original / f"{subdir}/{operation}.{extension}")
                             for subdir, extension in (("zkir", "zkir"), ("zkir", "bzkir"),
                                                       ("keys", "prover"), ("keys", "verifier")))
        receipt["original_material_hashes"] = hashes(artifacts)
        for scenario, (selector, _, _) in SCENARIOS.items():
            path = original / f"did-{scenario}-result.json"
            require(not path.exists(), "stale original relation summary")
            execute([runner, selector, original], f"prove-{scenario}")
            summary = read_json(path)
            validate_original(summary, scenario)
            state = checked_file(original / summary["final_state_file"])
            receipt["scenarios"][scenario] = {"summary": summary, "hashes": hashes([path, state])}
        require(sum(len(value["summary"]["calls"]) for value in receipt["scenarios"].values()) == 19,
                "original relation call count changed")
        for kind, (stem, fixture, operation) in REDUCERS.items():
            source = ROOT / f"examples/rust_backend/{stem}.compact"
            output = directory / kind
            execute([components["compactc"]["snapshot"], "--target", "rust", "--skip-zk",
                     "--rust-runtime-root", ROOT, source, output], f"compile-{kind}")
            execute(["rustfmt", "--edition", "2024", output / "contract/lib.rs"], f"format-{kind}")
            require((output / "contract/lib.rs").read_bytes() ==
                    (ROOT / f"tests-rust-backend/{fixture}/lib.rs").read_bytes(),
                    f"{kind}: reviewed generated fixture changed")
            report = read_json(output / "contract/rust-capabilities.json")
            common.validate_report(report, source)
            tab = common.proof_cross_tab(report, read_json(output / "compiler/contract-info.json"), source)
            require(len([row for row in tab if row["name"] == operation and row["proof"]
                         and row["recorded"] and row["observed_call"]]) == 1,
                    f"{kind}: selected circuit not proof/recording applicable")
            materials = []
            for subdir, extension in (("zkir", "zkir"), ("zkir", "bzkir"),
                                      ("keys", "prover"), ("keys", "verifier")):
                destination = output / f"{subdir}/{operation}.{extension}"
                source_material = (reuse_reducers / f"proof-{kind}" / subdir / f"{operation}.{extension}"
                                   if reuse_reducers else None)
                if source_material is not None:
                    if extension == "zkir":
                        require(common.sha256(checked_file(source_material)) ==
                                common.sha256(checked_file(destination)),
                                f"{kind}: retained ZKIR differs from fresh compiler output")
                    copy_or_generate(source_material, destination, lambda: None)
                materials.append(destination)
            if reuse_reducers is None:
                execute([components["zkir"]["snapshot"], "compile", output / f"zkir/{operation}.zkir",
                         output / f"keys/{operation}.prover", output / f"keys/{operation}.verifier"], f"keygen-{kind}")
            before = hashes(checked_file(path) for path in materials)
            execute([runner, "--did-primitive-reducer", kind, output], f"prove-{kind}")
            summary_path = output / "proof-result.json"
            summary = read_json(summary_path)
            validate_reducer(summary, kind)
            states = [checked_file(output / f"{call['case']}-state.bin") for call in summary["calls"]]
            receipt["reducers"][kind] = {"summary": summary, "material_hashes": before,
                                         "result_hashes": hashes([summary_path, *states])}
            require(hashes(materials) == before, f"{kind}: proof material changed")
        require(sum(len(value["summary"]["calls"]) for value in receipt["reducers"].values()) == 6,
                "reducer proof call count changed")
        require(source_inventory() == receipt["source_hashes"] and common.git_head() == receipt["git_head"],
                "source or HEAD changed during relation gate")
        did.unchanged(receipt["original_material_hashes"])
        for value in receipt["scenarios"].values():
            did.unchanged(value["hashes"])
        for value in receipt["reducers"].values():
            did.unchanged(value["material_hashes"])
            did.unchanged(value["result_hashes"])
        did.unchanged({value["snapshot"]: value["sha256"] for value in components.values()})
        receipt["status"] = "passed"
    except (common.GateError, OSError, ValueError, KeyError, TypeError, IndexError) as error:
        receipt["error"] = str(error)
    finally:
        (directory / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("compiler", "scheme", "cargo-target-dir", "run-dir"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--reuse-material", type=Path)
    parser.add_argument("--reuse-reducers", type=Path)
    parser.add_argument("--public-dir", type=Path)
    args = parser.parse_args()
    result = run_gate(args.run_dir, args.compiler, args.scheme, args.cargo_target_dir,
                      reuse_material=args.reuse_material, reuse_reducers=args.reuse_reducers,
                      public_dir=args.public_dir)
    print(f"DID relation gate {result['status']}; receipt {args.run_dir / 'receipt.json'}")
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
