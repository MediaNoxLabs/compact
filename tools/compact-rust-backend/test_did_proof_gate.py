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

import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import did_proof_gate as gate


def put(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value) if not isinstance(value, str) else value)


def summary(scenario):
    spec = gate.SCENARIOS[scenario]
    return {"format": "compact-did-proof-result/v1", "scenario": scenario,
            "selector": spec["selector"], "installed_operations": spec["installed_operations"],
            "strictness": "default", "deployment_applied": True,
            "constructor_execution_proved": False, "final_state_file": f"did-{scenario}-final-state.bin",
            "calls": [{"case": case, "operation": operation, "proof_bytes": 123,
                       "changed_binding_rejected": True, "applied": True,
                       "replay_refusal": "IntentAlreadyExists"}
                      for case, operation in zip(spec["cases"], spec["operations"])]}


class OrchestrationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.target = self.root / "target"
        self.patch = patch.object(gate, "ROOT", self.root)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        for name in ("compiler", "scheme", "tools/zkir"):
            path = self.root / name
            put(path, "#!/bin/sh\nexit 0\n")
            path.chmod(0o755)
        for name in ("pp", "static"):
            (self.root / name).mkdir()
        self.env = {"MIDNIGHT_PP": str(self.root / "pp"),
                    "MIDNIGHT_LEDGER_TEST_STATIC_DIR": str(self.root / "static"),
                    "PATH": str(self.root / "tools")}
        put(self.root / gate.SOURCE, "source")
        put(self.root / gate.SOURCE_ROOT / "source-manifest.json", {
            "format": "compact-did-adoption-sources/v1", "files": [{
                "path": "packages/contract/src/did.compact",
                "sha256": gate.common.sha256(self.root / gate.SOURCE)}]})
        put(self.root / gate.FIXTURE, "fixture")
        put(self.root / "runtime-rs/compatibility.json", {"runtime_abi": 50})
        fixture = self.root / "static/dust/spend.bzkir"
        put(fixture, "test fixture")
        digest = gate.common.sha256(fixture)
        put(fixture.with_name("spend.bzkir.sha256"), digest)
        put(fixture.with_name("spend.prover.sha256"), gate.ledger_static.PROVER_HASH)
        self.fixture_patch = patch.object(gate.ledger_static, "FIXTURE_HASHES", {"spend.bzkir": digest})
        self.fixture_patch.start()
        self.addCleanup(self.fixture_patch.stop)
        self.calls = []
        self.failure = None
        self.mutation = lambda label, output: None

    def command(self, argv, label, directory, receipt, *, env, stdout_path):
        self.calls.append(label)
        log = directory / "logs" / (label + ".log")
        put(log, "ok")
        receipt["commands"].append({"argv": argv, "label": label, "log": str(log),
                                    "exit_code": 1 if label == self.failure else 0})
        if label == self.failure:
            raise gate.common.GateError("controlled failure: " + label)
        output = directory / "did"
        if label == "compile-did":
            put(output / "contract/lib.rs", "fixture")
            rows = [{"name": name, "proof_required": True, "recorded": name in gate.KEYS,
                     "observed_call": name in gate.KEYS,
                     "recording_status": "available" if name in gate.KEYS else "unavailable"}
                    for name in sorted(gate.EXPORTS)]
            for row in rows:
                if not row["recorded"]:
                    row["recording_unavailable"] = {"code": "unsupported_action", "ir_node": "StateAction::Let", "path": "actions[0]", "detail": "bounded gap"}
                    row["observed_call_unavailable"] = {"code": "recording_unavailable", "ir_node": "StateAction::Let", "path": "actions[0]", "detail": "bounded gap"}
            put(output / "contract/rust-capabilities.json", {"schema_version": 3, "circuits": rows})
            put(output / "compiler/contract-info.json", {"circuits": [
                {"name": name, "proof": True} for name in sorted(gate.EXPORTS)]})
            put(output / "compiler/rust-compatibility.json", {"compatibility": {"runtime_abi": 50}})
            for op in gate.KEYS:
                put(output / f"zkir/{op}.zkir", op)
        elif label == "build-proof-runner":
            binary = self.target / "debug/compact-rust-proof-smoke"
            put(binary, "#!/bin/sh\nexit 0\n")
            binary.chmod(0o755)
        elif label == "prepare-proof-material":
            put(self.root / "pp/builtin-key", "public key")
            put(self.root / "pp/bls_midnight_2p11", "public SRS")
            put(stdout_path, {"format": "compact-proof-material/v1", "mode": "prepare",
                              "cache_directory": str(self.root / "pp"),
                              "asset_count": 1, "assets": [{"name": "builtin-key"}],
                              "parameter_count": 1, "parameters": [{"name": "bls_midnight_2p11"}]})
        elif label.startswith("keygen-"):
            op = label.removeprefix("keygen-")
            for ext in ("prover", "verifier"):
                put(output / f"keys/{op}.{ext}", ext)
            put(output / f"zkir/{op}.bzkir", "binary IR")
            put(self.root / "pp/bls_midnight_2p11", "public SRS")
            put(log, "Compiling circuit (k=11, rows=1)")
        elif label.startswith("prove-"):
            scenario = label.removeprefix("prove-")
            put(output / f"did-{scenario}-result.json", summary(scenario))
            put(output / f"did-{scenario}-final-state.bin", "public state")
        self.mutation(label, output)

    def run_gate(self):
        return gate.run_gate(self.root / "run", self.root / "compiler", self.root / "scheme",
                             self.target, self.env, command=self.command)

    def test_three_scenarios_and_six_keys_are_mandatory(self):
        result = self.run_gate()
        self.assertEqual(result["status"], "passed", result.get("error"))
        self.assertEqual(set(result["scenarios"]), {"points", "aliases", "services"})
        self.assertEqual(len(result["keygen"]), 6)
        self.assertEqual(sum(len(row["calls"]) for row in result["scenarios"].values()), 8)
        self.assertEqual(self.calls[-3:], ["prove-points", "prove-aliases", "prove-services"])

    def test_missing_prerequisite_refuses_before_commands_and_retains_failure(self):
        self.env.pop("MIDNIGHT_PP")
        result = self.run_gate()
        self.assertEqual(result["status"], "failed")
        self.assertIn("MIDNIGHT_PP", result["error"])
        self.assertEqual(self.calls, [])
        self.assertEqual(gate.read_json(self.root / "run/receipt.json"), result)

    def test_alias_failure_is_not_masked_by_successful_point_run(self):
        self.failure = "prove-aliases"
        result = self.run_gate()
        self.assertEqual(result["status"], "failed")
        self.assertEqual(set(result["scenarios"]), {"points"})
        self.assertEqual(result["commands"][-1]["exit_code"], 1)

    def test_service_failure_is_not_masked_by_prior_successful_lifecycles(self):
        self.failure = "prove-services"
        result = self.run_gate()
        self.assertEqual(result["status"], "failed")
        self.assertEqual(set(result["scenarios"]), {"points", "aliases"})
        self.assertEqual(result["commands"][-1]["exit_code"], 1)

    def test_existing_output_directory_is_never_reused(self):
        put(self.root / "run/sentinel", "keep")
        with self.assertRaises(FileExistsError):
            self.run_gate()
        self.assertEqual((self.root / "run/sentinel").read_text(), "keep")
        self.assertEqual(self.calls, [])

    def test_generated_fixture_mismatch_refuses_before_build(self):
        self.mutation = lambda label, output: put(output / "contract/lib.rs", "changed") if label == "compile-did" else None
        result = self.run_gate()
        self.assertIn("fresh DID fixture differs", result["error"])
        self.assertNotIn("build-proof-runner", self.calls)

    def test_changed_frozen_source_is_rejected(self):
        self.mutation = lambda label, output: put(self.root / gate.SOURCE, "changed") if label == "prove-aliases" else None
        self.assertIn("frozen input changed", self.run_gate()["error"])

    def test_missing_key_material_refuses_before_proof(self):
        self.mutation = lambda label, output: (output / "keys/deactivate.prover").unlink() if label == "keygen-setAlsoKnownAs" else None
        self.assertIn("missing generated proof material", self.run_gate()["error"])
        self.assertNotIn("prove-points", self.calls)

    def test_missing_success_summary_is_a_failed_gate(self):
        self.mutation = lambda label, output: (output / "did-services-result.json").unlink() if label == "prove-services" else None
        self.assertEqual(self.run_gate()["status"], "failed")

    def test_nonzero_keygen_retains_failure_before_any_proof(self):
        self.failure = "keygen-deactivate"
        result = self.run_gate()
        self.assertEqual(result["status"], "failed")
        self.assertNotIn("prove-points", self.calls)
        self.assertEqual(result["commands"][-1]["exit_code"], 1)

    def test_changed_key_after_successful_calls_is_rejected(self):
        self.mutation = lambda label, output: put(output / "keys/deactivate.verifier", "changed") if label == "prove-aliases" else None
        self.assertIn("frozen input changed", self.run_gate()["error"])

    def test_preexisting_scenario_success_cannot_short_circuit_execution(self):
        self.mutation = lambda label, output: put(output / "did-points-result.json", summary("points")) if label == "keygen-setAlsoKnownAs" else None
        self.assertIn("stale DID scenario result", self.run_gate()["error"])
        self.assertNotIn("prove-points", self.calls)

    def test_capability_inventory_cannot_hide_an_unsupported_export(self):
        def mutate(label, output):
            if label == "compile-did":
                path = output / "contract/rust-capabilities.json"
                report = gate.read_json(path)
                report["circuits"] = [r for r in report["circuits"] if r["name"] != "setService"]
                put(path, report)
        self.mutation = mutate
        self.assertIn("twelve-export", self.run_gate()["error"])
        self.assertNotIn("build-proof-runner", self.calls)

    def test_new_source_file_during_run_refuses_changed_inventory(self):
        self.mutation = lambda label, output: put(self.root / "runtime-rs/src/new.rs", "new input") if label == "prove-aliases" else None
        self.assertIn("source inventory additions", self.run_gate()["error"])

    def test_commit_change_during_run_refuses_acceptance(self):
        with patch.object(gate.common, "git_head", side_effect=["before", "after"]):
            self.assertIn("git HEAD changed", self.run_gate()["error"])

    def test_upstream_dust_fixture_mismatch_refuses_before_commands(self):
        put(self.root / "static/dust/spend.bzkir", "changed fixture")
        self.assertIn("upstream Dust fixture mismatch", self.run_gate()["error"])
        self.assertEqual(self.calls, [])

    def test_summary_cannot_redirect_final_state_outside_run(self):
        value = summary("points")
        value["final_state_file"] = "../other.bin"
        with self.assertRaises(gate.common.GateError):
            gate.validate_summary(value, "points")

    def test_summary_cannot_omit_or_reorder_checked_calls(self):
        for field in ("calls", "installed_operations"):
            with self.subTest(field=field):
                value = summary("points")
                value[field] = value[field][:-1]
                with self.assertRaises(gate.common.GateError):
                    gate.validate_summary(value, "points")
        value = summary("aliases")
        value["calls"][0]["proof_bytes"] = 0
        with self.assertRaises(gate.common.GateError):
            gate.validate_summary(value, "aliases")
        value = summary("services")
        value["calls"][1]["operation"] = "removeService"
        with self.assertRaises(gate.common.GateError):
            gate.validate_summary(value, "services")


if __name__ == "__main__":
    unittest.main()
