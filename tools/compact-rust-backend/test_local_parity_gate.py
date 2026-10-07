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

import os
import json
from unittest.mock import patch
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import local_parity_gate as gate


class BuildReceiptTests(unittest.TestCase):
    def inputs(self, root, environment):
        def observe(command, **kwargs):
            self.assertEqual(kwargs["env"], environment)
            return subprocess.CompletedProcess(command, 0, f"{command[0]} observed-version\n", "")
        with patch.object(gate.subprocess, "run", side_effect=observe):
            return gate.build_input_receipt(environment, root)

    def test_observed_tools_and_selected_settings_exclude_secrets(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            environment = {"RUSTUP_TOOLCHAIN": "1.99.0", "CARGO_HOME": str(root / "cargo"),
                           "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_BUILD_JOBS": "2",
                           "RUSTFLAGS": "--cfg private_secret_value", "GITHUB_TOKEN": "token_secret",
                           "CARGO_REGISTRIES_PRIVATE_TOKEN": "registry_secret",
                           "CARGO_PROFILE_TEST_LTO": "unexpected_secret"}
            receipt = self.inputs(root, environment)
            self.assertEqual(receipt["tools"]["rustc"]["version"], "rustc observed-version")
            self.assertEqual(receipt["tools"]["cargo"]["version"], "cargo observed-version")
            settings = receipt["selected_environment"]
            self.assertEqual(settings["CARGO_PROFILE_TEST_DEBUG"], {"value": "0"})
            self.assertEqual(settings["RUSTUP_TOOLCHAIN"], {"value": "1.99.0"})
            self.assertEqual(settings["RUSTFLAGS"]["sha256"],
                             gate.hashlib.sha256(environment["RUSTFLAGS"].encode()).hexdigest())
            serialized = json.dumps(receipt)
            for secret in ("private_secret_value", "token_secret", "registry_secret", "unexpected_secret",
                           "GITHUB_TOKEN", "CARGO_REGISTRIES_PRIVATE_TOKEN"):
                self.assertNotIn(secret, serialized)

    def test_absent_overrides_and_optional_config_are_explicit(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            receipt = self.inputs(root, {"CARGO_HOME": str(root / "cargo")})
            self.assertEqual(receipt["selected_environment"], {})
            self.assertIsNone(receipt["cargo_config_sha256"]["ancestor_0/.cargo/config.toml"])
            self.assertIsNone(receipt["cargo_config_sha256"]["cargo_home/config"])
            self.assertIsNone(receipt["lock_sha256"]["Cargo.lock"])

    def test_config_and_lock_are_hashed_without_disclosing_contents(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            config = root / ".cargo/config.toml"
            config.parent.mkdir()
            config.write_text('[registries.private]\ntoken = "config_secret"\n')
            lock = root / "Cargo.lock"
            lock.write_text("locked identity\n")
            environment = {"CARGO_HOME": str(root / "cargo")}
            first = self.inputs(root, environment)
            second = self.inputs(root, environment)
            self.assertEqual(first, second)
            self.assertEqual(first["cargo_config_sha256"]["ancestor_0/.cargo/config.toml"], gate.sha256(config))
            self.assertEqual(first["lock_sha256"]["Cargo.lock"], gate.sha256(lock))
            self.assertNotIn("config_secret", json.dumps(first))

    def test_unavailable_version_probe_is_not_a_new_build_prerequisite(self):
        with tempfile.TemporaryDirectory() as tmp, \
             patch.object(gate.subprocess, "run", side_effect=FileNotFoundError("secret path")):
            receipt = gate.build_input_receipt({"CARGO_HOME": tmp}, Path(tmp))
            self.assertEqual(receipt["tools"]["cargo"]["error_kind"], "FileNotFoundError")
            self.assertNotIn("secret path", json.dumps(receipt))

    def test_failed_version_probe_does_not_publish_stderr(self):
        with tempfile.TemporaryDirectory() as tmp, \
             patch.object(gate.subprocess, "run", return_value=subprocess.CompletedProcess(
                 [], 7, "unqualified stdout", "private stderr")):
            receipt = gate.build_input_receipt({"CARGO_HOME": tmp}, Path(tmp))
            self.assertEqual(receipt["tools"]["cargo"]["exit_code"], 7)
            self.assertNotIn("private stderr", json.dumps(receipt))
            self.assertNotIn("unqualified stdout", json.dumps(receipt))

    def test_unchanged_inputs_retain_both_snapshots(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            before = self.inputs(root, {"CARGO_HOME": tmp})
            receipt = {"build_inputs": before}
            with patch.object(gate, "build_input_receipt", return_value=before):
                gate.finish_build_input_receipt(receipt, {"CARGO_HOME": tmp}, root)
            self.assertEqual(receipt["build_inputs_after"], before)
            self.assertEqual(receipt["build_input_drift"], [])

    def test_changed_lock_or_config_refused_after_retaining_both_snapshots(self):
        for name, group, key in (("Cargo.lock", "lock_sha256", "Cargo.lock"),
                                 (".cargo/config.toml", "cargo_config_sha256",
                                  "ancestor_0/.cargo/config.toml")):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("before")
                environment = {"CARGO_HOME": str(root / "home")}
                before = self.inputs(root, environment)
                # Existing failure evidence must not be recast as a pass or lost.
                receipt = {"build_inputs": before, "status": "failed", "error": "original failure",
                           "commands": [{"exit_code": 3, "log_sha256": "retained"}]}
                path.write_text("after")
                after = self.inputs(root, environment)
                with patch.object(gate, "build_input_receipt", return_value=after), \
                     self.assertRaisesRegex(gate.GateError, "selected build input changed"):
                    gate.finish_build_input_receipt(receipt, environment, root)
                self.assertEqual(receipt["build_inputs"], before)
                self.assertEqual(receipt["build_inputs_after"], after)
                self.assertEqual(receipt["build_input_drift"], [f"{group}:{key}"])
                self.assertEqual(receipt["status"], "failed")
                self.assertEqual(receipt["error"], "original failure")
                self.assertEqual(receipt["commands"], [{"exit_code": 3, "log_sha256": "retained"}])

    def test_success_and_failure_both_bind_closed_log_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            receipt = {"commands": []}
            for code in (0, 3):
                command = [sys.executable, "-c", f"import sys; print('observed output'); sys.exit({code})"]
                if code:
                    with self.assertRaisesRegex(gate.GateError, r"failed \(3\)"):
                        gate.run(command, "failure", root, receipt)
                else:
                    gate.run(command, "success", root, receipt)
                entry = receipt["commands"][-1]
                self.assertEqual(entry["exit_code"], code)
                self.assertEqual(entry["argv"], command)
                self.assertEqual(entry["log_sha256"], gate.sha256(Path(entry["log"])))
                self.assertEqual(Path(entry["log"]).read_text(), "observed output\n")

    def test_separate_stdout_and_stderr_are_both_bound_on_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            receipt = {"commands": []}
            output = root / "stdout.json"
            with self.assertRaises(gate.GateError):
                gate.run([sys.executable, "-c",
                          "import sys; print('stdout body'); print('stderr body', file=sys.stderr); sys.exit(4)"],
                         "separate", root, receipt, stdout_path=output)
            entry = receipt["commands"][0]
            self.assertEqual(entry["stdout"], {"path": str(output), "sha256": gate.sha256(output)})
            self.assertEqual(entry["log_sha256"], gate.sha256(Path(entry["log"])))
            self.assertEqual(output.read_text(), "stdout body\n")
            self.assertEqual(Path(entry["log"]).read_text(), "stderr body\n")


class WorkspaceTestPlanTests(unittest.TestCase):
    def test_full_gate_reports_missing_fee_fixture_before_compiler_or_build(self):
        environment = os.environ.copy()
        environment.pop("MIDNIGHT_LEDGER_TEST_STATIC_DIR", None)
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = Path(temporary) / "gate"
            result = subprocess.run(
                [sys.executable, str(Path(gate.__file__)), "--full", "--compiler",
                 str(Path(temporary) / "missing-compiler"), "--run-dir", str(run_dir)],
                env=environment, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 1)
            self.assertIn("MIDNIGHT_LEDGER_TEST_STATIC_DIR", result.stderr)
            self.assertIn("ledger/static", result.stderr)
            self.assertFalse(run_dir.exists())

    def test_full_gate_invokes_unit_composition_registry_and_both_scenario_driver(self):
        import did_proof_gate
        calls = []
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "compiler"
            compiler.write_text("#!/bin/sh\nexit 0\n")
            compiler.chmod(0o755)
            def record(command, label, directory, receipt, **kwargs):
                calls.append((label, command))
                if label == "workspace-test-metadata":
                    kwargs["stdout_path"].write_text(json.dumps({"workspace_members": [], "packages": []}))
                if label == "did-proof-lifecycles":
                    raise gate.GateError("controlled stop after mandatory DID invocation")
            with patch.object(sys, "argv", ["gate", "--full", "--compiler", str(compiler),
                    "--scheme", str(compiler), "--run-dir", str(root / "run")]), \
                 patch.dict(os.environ, {"MIDNIGHT_LEDGER_TEST_STATIC_DIR": str(root)}), \
                 patch.object(gate.shutil, "which", return_value=str(compiler)), \
                 patch.object(did_proof_gate, "prerequisites", return_value={}), \
                 patch.object(gate, "select_sources", return_value=[]), \
                 patch.object(gate.inventory, "source_paths", return_value=[]), \
                 patch.object(gate, "compare_baseline"), \
                 patch.object(gate.inventory, "receipt_metadata", return_value={}), \
                 patch.object(gate, "build_input_receipt", side_effect=[
                     {"lock_sha256": {"Cargo.lock": "before"}},
                     {"lock_sha256": {"Cargo.lock": "after"}}]), \
                 patch.object(gate, "run", side_effect=record):
                self.assertEqual(gate.main(), 1)
            failed = json.loads((root / "run/receipt.json").read_text())
            self.assertEqual(failed["error"], "controlled stop after mandatory DID invocation")
            self.assertEqual(failed["status"], "failed")
            self.assertEqual(failed["build_input_drift"], ["lock_sha256:Cargo.lock"])
            self.assertIn("lock_sha256:Cargo.lock", failed["build_input_error"])
            self.assertEqual(failed["build_inputs_after"]["lock_sha256"]["Cargo.lock"], "after")
            by_label = dict(calls)
            registry = by_label["unit-composition-source-scope"]
            self.assertEqual(registry[registry.index("--manifest") + 1],
                             str(gate.ROOT / "tools/compact-rust-backend/parity_positive_unit_composition_sources.json"))
            self.assertIn("did_proof_gate.py", by_label["did-proof-lifecycles"][1])
            self.assertIn("consumer-proof-ledger", by_label)
            self.assertEqual(by_label["qualified-set-path-admission"][1:], [
                str(gate.ROOT / "tools/compact-rust-backend/check_compactc_target.py"),
                "--adt-set-qualified"])
            self.assertNotIn("--skip", by_label["did-proof-lifecycles"])

    def test_generated_library_guard_falls_back_for_test_or_unknown_expansion(self):
        safe = '#[derive(Clone)]\npub fn f() { if !(true) { assert!(false); } }'
        self.assertTrue(gate.generated_library_has_no_test_hooks(safe))
        for extra in ['#[test] fn unit() {}', '#[cfg_attr(test, test)] fn unit() {}',
                      '#[cfg(test)] mod tests {}', 'include!("tests.rs");',
                      'mod hidden_tests;', 'custom_tests!();',
                      '#[derive(UnknownMacro)] struct T;', 'r#if!();']:
            with self.subTest(extra=extra):
                self.assertFalse(gate.generated_library_has_no_test_hooks(safe + extra))

    def test_added_inline_test_and_new_package_keep_exhaustive_selection(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / 'lib.rs'
            source.write_text('pub fn value() -> bool { true }')
            def package(name):
                return {'id': name, 'name': name, 'targets': [
                    {'name': name, 'kind': ['lib'], 'src_path': str(source)},
                    {'name': 'behavior', 'kind': ['test'], 'src_path': str(Path(tmp) / 'behavior.rs')}]}
            generated = package('generated')
            core = package('new-core')
            core['targets'][0]['src_path'] = str(Path(tmp) / 'unverified.rs')
            metadata = {'workspace_members': ['generated', 'new-core'], 'packages': [generated, core]}
            plan = gate.workspace_test_plan(metadata, {source})
            self.assertEqual(plan['integration_packages'], ['generated'])
            self.assertEqual(plan['all_target_packages'], ['new-core'])
            self.assertEqual(plan['omitted_empty_library_harnesses'][0]['integration_targets'], ['behavior'])
            source.write_text(source.read_text() + '\n#[test] fn added_unit() { assert!(value()); }')
            plan = gate.workspace_test_plan(metadata, {source})
            self.assertEqual(plan['integration_packages'], [])
            self.assertEqual(plan['all_target_packages'], ['generated', 'new-core'])
            command = gate.planned_test_commands(plan)[0][1]
            self.assertIn('--all-targets', command)
            self.assertNotIn('--test', command)

    def test_fixture_extra_target_or_no_integration_test_falls_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / 'lib.rs'
            source.write_text('pub fn value() {}')
            lib = {'name': 'fixture', 'kind': ['lib'], 'src_path': str(source)}
            test = {'name': 'behavior', 'kind': ['test'], 'src_path': str(source)}
            example = {'name': 'example', 'kind': ['example'], 'src_path': str(source)}
            for targets in [[lib], [lib, test, example]]:
                metadata = {'workspace_members': ['fixture'], 'packages': [
                    {'id': 'fixture', 'name': 'fixture', 'targets': targets}]}
                plan = gate.workspace_test_plan(metadata, {source})
                self.assertEqual(plan['all_target_packages'], ['fixture'])
                self.assertFalse(plan['integration_packages'])


class MaintainedFixtureAndProofTests(unittest.TestCase):
    def test_shared_registry_and_newly_registered_sources(self):
        import check_fixture_outputs as freshness
        import fixture_inventory
        mapping = gate.fixture_map()
        self.assertEqual(mapping, fixture_inventory.fixture_map())
        self.assertEqual(freshness.EXTRA_SOURCES, fixture_inventory.extra_sources())
        self.assertEqual(len(mapping), 198)
        inventory_paths = set(gate.inventory.source_paths(gate.ROOT))
        self.assertTrue(set(mapping) <= inventory_paths)
        for source in fixture_inventory.SPECIAL_FIXTURES:
            selected = gate.select_sources([source], False)
            self.assertEqual(selected, [(gate.ROOT / source, mapping[gate.ROOT / source])])

    def test_duplicate_stems_have_distinct_artifact_directories(self):
        sources = [gate.ROOT / name for name in (
            "examples/rust_backend/vc_passport_adoption/src/digital-passport-credential.compact",
            "examples/rust_backend/digital-passport-credential/src/digital-passport-credential.compact")]
        outputs = [gate.fixture_output(Path("/tmp/control"), source) for source in sources]
        self.assertNotEqual(*outputs)
        self.assertEqual(outputs, [gate.fixture_output(Path("/tmp/control"), source) for source in sources])

    def run_proofs(self, root, fault=None):
        calls, receipt = [], {}
        formats = {"did-proof-lifecycles": "compact-did-proof-gate/v1",
                   "did-digest-reducer-proof": "compact-did-digest-reducer-gate/v1",
                   "did-primitive-reducer-proofs": "compact-did-primitive-reducer-gate/v1",
                   "did-relation-proofs": "compact-did-relation-gate/v1",
                   "acc-jubjub-proofs": "compact-acc-jubjub-gate/v1"}
        def command(argv, label, directory, receipt, **kwargs):
            calls.append((label, argv))
            self.assertNotIn("--only", argv)
            self.assertNotIn("--skip", argv)
            self.assertEqual(argv[argv.index("--compiler") + 1], str(root / "bin/compactc"))
            self.assertEqual(argv[argv.index("--scheme") + 1], str(root / "bin/compactc-scheme"))
            flag = "--output" if "--output" in argv else "--run-dir"
            child = Path(argv[argv.index(flag) + 1]); child.mkdir()
            value = {"format": formats[label], "status": "passed"}
            if fault and label == fault[0]:
                if fault[1] == "changed_earlier":
                    (root / "did-proof/receipt.json").write_text("changed")
                    (child / "receipt.json").write_text(json.dumps(value))
                    return
                if fault[1] == "command":
                    raise gate.GateError("controlled child exit failure")
                if fault[1] == "missing":
                    return
                if fault[1] == "malformed":
                    (child / "receipt.json").write_text("{")
                    return
                value = {"format": "wrong", "status": "passed"} if fault[1] == "format" else {"format": formats[label], "status": "failed"}
            (child / "receipt.json").write_text(json.dumps(value))
        gate.run_required_proof_gates(root, root / "bin/compactc", {"CARGO_TARGET_DIR": str(root / "target")}, receipt, command=command)
        return calls, receipt

    def test_full_proof_orchestration_requires_every_child_and_hashes_receipts(self):
        with tempfile.TemporaryDirectory() as tmp:
            calls, receipt = self.run_proofs(Path(tmp))
            self.assertEqual([label for label, _ in calls], ["did-proof-lifecycles", "did-digest-reducer-proof", "did-primitive-reducer-proofs", "did-relation-proofs", "acc-jubjub-proofs"])
            self.assertEqual(set(receipt), {"did_proof_gate", "did_digest_reducer_gate", "did_primitive_reducer_gate", "did_relation_gate", "acc_jubjub_gate"})
            for row in receipt.values():
                self.assertEqual(row["sha256"], gate.sha256(Path(row["path"])))

    def test_previous_child_receipt_cannot_change_while_later_gate_runs(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(gate.GateError, "changed during orchestration"):
                self.run_proofs(Path(tmp), ("did-primitive-reducer-proofs", "changed_earlier"))

    def test_failed_missing_or_invalid_child_receipts_fail_orchestration(self):
        for label in ("did-proof-lifecycles", "did-digest-reducer-proof", "did-primitive-reducer-proofs", "did-relation-proofs", "acc-jubjub-proofs"):
            for fault in ("command", "missing", "malformed", "format", "status"):
                with self.subTest(label=label, fault=fault), tempfile.TemporaryDirectory() as tmp:
                    with self.assertRaises((gate.GateError, OSError, ValueError)):
                        self.run_proofs(Path(tmp), (label, fault))


if __name__ == '__main__':
    unittest.main()
