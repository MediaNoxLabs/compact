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
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import did_digest_reducer_gate as gate


def put(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value) if not isinstance(value, str) else value)


class ReducerGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.target = self.root / "target"
        self.patch = patch.object(gate, "ROOT", self.root)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        self.source_patch = patch.object(gate, "source_inventory", return_value={})
        self.source_patch.start()
        self.addCleanup(self.source_patch.stop)
        self.head_patch = patch.object(gate.common, "git_head", return_value="frozen-test-head")
        self.head_patch.start()
        self.addCleanup(self.head_patch.stop)
        for name in ("compiler", "scheme", "tools/zkir"):
            path = self.root / name
            put(path, "#!/bin/sh\nexit 0\n")
            path.chmod(0o755)
        for name in ("pp", "static"):
            (self.root / name).mkdir()
        fixture = self.root / "static/dust/spend.bzkir"
        put(fixture, "pinned Dust fixture")
        digest = gate.common.sha256(fixture)
        put(fixture.with_name("spend.bzkir.sha256"), digest)
        self.static_patch = patch.object(gate.ledger_static, "FIXTURE_HASHES", {"spend.bzkir": digest})
        self.static_patch.start()
        self.addCleanup(self.static_patch.stop)
        put(self.root / gate.FIXTURE, "fresh fixture")
        self.env = {"MIDNIGHT_PP": str(self.root / "pp"),
                    "MIDNIGHT_LEDGER_TEST_STATIC_DIR": str(self.root / "static"),
                    "PATH": str(self.root / "tools")}
        self.calls = []
        self.corrupt = None

    def command(self, argv, label, directory, receipt, *, env, stdout_path):
        self.calls.append(label)
        log = directory / "logs" / (label + ".log")
        put(log, "Compiling circuit (k=7, rows=64)" if label == "keygen-reducer" else "ok")
        receipt["commands"].append({"argv": argv, "label": label, "log": str(log), "exit_code": 0})
        output = directory / "reducer"
        if label == "compile-reducer":
            put(output / "contract/lib.rs", "different" if self.corrupt == "fixture" else "fresh fixture")
            put(output / "contract/rust-capabilities.json", {"schema_version": 3, "circuits": [{
                "name": "verify", "proof_required": True, "recorded": True,
                "observed_call": True, "recording_status": "available"}]})
            put(output / "compiler/contract-info.json", {"circuits": [{"name": "verify", "proof": True}]})
            put(output / "zkir/verify.zkir", "source IR")
        elif label == "build-proof-runner":
            runner = self.target / "debug/compact-rust-proof-smoke"
            put(runner, "#!/bin/sh\nexit 0\n")
            runner.chmod(0o755)
        elif label == "prepare-proof-material":
            put(stdout_path, {"format": "compact-proof-material/v1", "mode": "prepare",
                              "cache_directory": str(self.root / "pp")})
        elif label == "keygen-reducer":
            for ext in ("prover", "verifier"):
                put(output / f"keys/verify.{ext}", ext)
            put(output / "zkir/verify.bzkir", "binary IR")
        elif label == "prove-reducer":
            put(output / "reducer-final-state.bin", "state")
            put(output / "reducer-proof-result.json", {
                "status": "passed", "strictness": "default", "deployed_constructor_data": True,
                "applied": True, "read_only_contract_data": True,
                "changed_binding_rejected": False if self.corrupt == "binding" else True,
                "constructor_execution_proved": False, "replay_refusal": "IntentAlreadyExists",
                "proof_bytes": 2912, "final_state_bytes": 5})

    def run_gate(self):
        return gate.run_gate(self.root / "run", self.root / "compiler", self.root / "scheme",
                             self.target, self.env, command=self.command)

    def test_fresh_compilation_capture_test_keygen_and_strict_proof_are_mandatory(self):
        receipt = self.run_gate()
        self.assertEqual(receipt["status"], "passed", receipt.get("error"))
        self.assertEqual(self.calls, ["compile-reducer", "format-reducer", "test-reducer-behavior",
                                      "build-proof-runner", "prepare-proof-material",
                                      "keygen-reducer", "prove-reducer"])
        self.assertEqual(receipt["keygen"], {"operation": "verify", "k": 7, "rows": 64})

    def test_stale_generated_fixture_refuses_before_tests_and_keys(self):
        self.corrupt = "fixture"
        receipt = self.run_gate()
        self.assertIn("fresh reducer fixture differs", receipt["error"])
        self.assertNotIn("test-reducer-behavior", self.calls)

    def test_missing_changed_binding_rejection_refuses_acceptance(self):
        self.corrupt = "binding"
        receipt = self.run_gate()
        self.assertEqual(receipt["status"], "failed")
        self.assertIn("changed_binding_rejected", receipt["error"])
        self.assertEqual(self.calls[-1], "prove-reducer")


if __name__ == "__main__":
    unittest.main()
