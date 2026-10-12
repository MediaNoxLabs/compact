# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# 	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Focused scenario dispatch and fake-compiler acceptance boundaries."""

from contextlib import ExitStack, redirect_stdout
import inspect
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import check_compactc_target as cli
from target_checks import application_checks, intent_checks, recording_checks, scalar_checks


OWNERS = (application_checks, intent_checks, recording_checks, scalar_checks)
CHECKS = [(module, name, function) for module in OWNERS
          for name, function in vars(module).items()
          if name.startswith("check_") and callable(function)]


class FocusedTargetChecks(unittest.TestCase):
    def arguments(self, function, base, runner):
        values = {"root": Path("repository"), "proof_requested": False, "run": runner,
                  "capture": runner, "python_executable": "python-under-test",
                  "check_coracle_source": runner}
        return {name: values[name] if name in values else Path("sources") / f"{name}.compact"
                for name in inspect.signature(function).parameters
                if name not in ("compiler", "base")}

    def test_all_22_functions_propagate_first_runner_failure_without_more_work(self):
        self.assertEqual(len(CHECKS), 22)
        for module, name, function in CHECKS:
            for proof in (False, True):
                with self.subTest(module=module.__name__, check=name, proof=proof):
                    events = []
                    failure = subprocess.CalledProcessError(19, ["fake-runner"])

                    def run(*args, **kwargs):
                        events.append((args, kwargs))
                        raise failure

                    with tempfile.TemporaryDirectory() as tmp:
                        arguments = self.arguments(function, Path(tmp), run)
                        if "proof_requested" in arguments:
                            arguments["proof_requested"] = proof
                        with self.assertRaises(subprocess.CalledProcessError) as error:
                            function("compiler-under-test", Path(tmp), **arguments)
                    self.assertIs(error.exception, failure)
                    self.assertEqual(len(events), 1)

    def test_cli_preserves_first_matching_flag_and_returns_before_default_checks(self):
        ordered = ["stateful_assert", "wide_add", "transient_receive_send",
                   "canonical_output_order", "terminal_lexical_return", "composite_zswap",
                   "field_observation", "stateful_struct", "kernel_shielded_effects",
                   "native_zswap_intents", "effectful_return", "qualified_coin_cell",
                   "counter_less_than", "literal_bytes_field", "coracle_start",
                   "coracle_concede", "coracle_withdraw", "coracle_guess", "coracle_root_let",
                   "adt_set_qualified", "test_center_bboard", "field_to_bytes32"]
        for first, second in zip(ordered, ordered[1:]):
            with self.subTest(first=first, second=second), ExitStack() as stack:
                selected = []
                for module, name, _ in CHECKS:
                    stack.enter_context(patch.object(module, name,
                        side_effect=lambda *a, _name=name, **k: selected.append((_name, a, k))))
                stack.enter_context(patch.object(cli, "run", side_effect=AssertionError("default gate ran")))
                stack.enter_context(patch.dict(os.environ, {"COMPACTC": "compiler-under-test",
                                                           "CARGO_TARGET_DIR": "existing-target"}))
                stack.enter_context(patch.object(sys, "argv", ["check", "--proof", "--consumer",
                                                               "--" + first.replace("_", "-"),
                                                               "--" + second.replace("_", "-")]))
                cli.main()
                self.assertEqual([name for name, _, _ in selected], ["check_" + first])
                self.assertEqual(selected[0][1][0], "compiler-under-test")
                self.assertEqual(os.environ["CARGO_TARGET_DIR"], "existing-target")
                if "proof_requested" in selected[0][2]:
                    self.assertTrue(selected[0][2]["proof_requested"])

    def compiler(self, events, names, *, ir=None, info=None, recorded=True):
        def run(*args):
            events.append(args)
            if args[0] != "compiler-under-test":
                return
            output = Path(args[-1])
            (output / "contract").mkdir(parents=True, exist_ok=True)
            (output / "compiler").mkdir(exist_ok=True)
            report = {"circuits": [{"name": name, "proof_required": True,
                                     "recorded": recorded, "observed_call": True} for name in names]}
            (output / "contract/rust-capabilities.json").write_text(json.dumps(report))
            if ir is not None:
                (output / "contract/compact-rust-ir.json").write_text(json.dumps(ir))
            if info is not None:
                (output / "compiler/contract-info.json").write_text(json.dumps(info))
        return run

    def test_field_observation_optional_proof_flags_and_capability_refusal(self):
        names = ["direct", "snapshot", "via_field_helper", "via_snapshot_helper",
                 "pair", "selected", "optional"]
        for proof in (False, True):
            for recorded in (False, True):
                with self.subTest(proof=proof, recorded=recorded), tempfile.TemporaryDirectory() as tmp:
                    base = Path(tmp)
                    events = []
                    run = self.compiler(events, names, recorded=recorded)
                    def invoke():
                        recording_checks.check_field_observation("compiler-under-test", base,
                            root=Path("repo"), proof_requested=proof, run=run)
                    with redirect_stdout(io.StringIO()):
                        if recorded:
                            invoke()
                        else:
                            with self.assertRaises(AssertionError):
                                invoke()
                    self.assertEqual(events[0], ("compiler-under-test", "--target", "rust",
                        "--rust-require-recording", *([] if proof else ["--skip-zk"]),
                        "repo/examples/rust_backend/field_observation_oracle.compact",
                        str(base / "field-observation")))
                    self.assertEqual(len(events), 2 if proof and recorded else 1)
                    if proof and recorded:
                        self.assertEqual(events[1], ("cargo", "run", "--quiet", "-p",
                            "compact-rust-proof-smoke", "--", "--field-observation",
                            str(base / "field-observation")))

    def test_intent_check_keeps_native_check_before_explicit_key_generation(self):
        for proof in (False, True):
            with self.subTest(proof=proof), tempfile.TemporaryDirectory() as tmp:
                base = Path(tmp)
                output = base / "transient-receive-send"
                events = []
                run = self.compiler(events, ["receive_then_send"],
                                    info={"circuits": [{"name": "receive_then_send", "proof": True}]})
                with redirect_stdout(io.StringIO()):
                    intent_checks.check_transient_receive_send("compiler-under-test", base,
                        root=Path("repo"), proof_requested=proof, run=run)
                expected = [("compiler-under-test", "--target", "rust", "--rust-require-recording",
                             "--skip-zk", "repo/examples/rust_backend/transient_receive_send_oracle.compact",
                             str(output)),
                            ("cargo", "+1.99.0", "check", "--offline", "--manifest-path",
                             str(output / "contract/Cargo.toml"))]
                if proof:
                    expected += [("zkir", "compile", str(output / "zkir/receive_then_send.zkir"),
                                  str(output / "keys/receive_then_send.prover"),
                                  str(output / "keys/receive_then_send.verifier")),
                                 ("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke",
                                  "--", "--transient-receive-send", str(output))]
                self.assertEqual(events, expected)

    def test_coracle_source_boundary_precedes_proof_generation(self):
        for proof in (False, True):
            with self.subTest(proof=proof), tempfile.TemporaryDirectory() as tmp:
                base = Path(tmp)
                events = []
                def source(compiler, output_base):
                    events.append(("source", compiler, output_base))
                    (output_base / "coracle").mkdir()
                with redirect_stdout(io.StringIO()):
                    application_checks.check_coracle_start("compiler-under-test", base,
                        proof_requested=proof, run=lambda *a: events.append(a),
                        check_coracle_source=source)
                self.assertEqual(events[0], ("source", "compiler-under-test", base))
                self.assertEqual(len(events), 3 if proof else 1)
                if proof:
                    self.assertEqual(events[1][0:2], ("zkir", "compile"))
                    self.assertEqual(events[2][-2], "--coracle-start")

    def test_captured_compiler_call_preserves_options_and_failure_diagnostics(self):
        for returncode in (0, 7):
            with self.subTest(returncode=returncode), tempfile.TemporaryDirectory() as tmp:
                base = Path(tmp)
                captured = []
                def run(*args):
                    output = Path(args[-1]) / "contract"
                    output.mkdir(parents=True)
                    (output / "lib.rs").write_text(
                        "runtime::add_wide_unsigned runtime::narrow_wide_uint RUST_RUNTIME_ABI == 52")
                    (output / "rust-capabilities.json").write_text(json.dumps({"circuits": [{
                        "recorded": False, "observed_call": False, "proof_required": False,
                        "recording_status": "not_applicable"}]}))
                def capture(command, **options):
                    captured.append((command, options))
                    if returncode == 0:
                        output = Path(command[-1]) / "contract"
                        output.mkdir(parents=True)
                        (output / "lib.rs").write_text("fixture")
                    return subprocess.CompletedProcess(command, returncode, stderr="compiler diagnostic")
                with redirect_stdout(io.StringIO()):
                    if returncode:
                        with self.assertRaisesRegex(AssertionError, "compiler diagnostic"):
                            scalar_checks.check_wide_add("compiler-under-test", base,
                                root=Path("repo"), run=run, capture=capture)
                    else:
                        scalar_checks.check_wide_add("compiler-under-test", base,
                            root=Path("repo"), run=run, capture=capture)
                self.assertEqual(captured, [(
                    ["compiler-under-test", "--target", "rust", "--skip-zk",
                     "repo/test-center/test-contracts/micro-dao.compact", str(base / "original-dao")],
                    {"cwd": Path("repo"), "capture_output": True, "text": True})])

    def test_scalar_counter_ir_must_contain_both_read_and_comparison(self):
        for comparison in (False, True):
            with self.subTest(comparison=comparison), tempfile.TemporaryDirectory() as tmp:
                base = Path(tmp)
                events = []
                ir = {"schema_version": 20, "kind": "counter_read"}
                if comparison:
                    ir["child"] = {"kind": "counter_less_than"}
                run = self.compiler(events, ["compare", "nested", "short_circuit", "checked"], ir=ir)
                with redirect_stdout(io.StringIO()):
                    if comparison:
                        scalar_checks.check_counter_less_than("compiler-under-test", base,
                            root=Path("repo"), proof_requested=True, run=run)
                    else:
                        with self.assertRaises(AssertionError):
                            scalar_checks.check_counter_less_than("compiler-under-test", base,
                                root=Path("repo"), proof_requested=True, run=run)
                self.assertEqual(len(events), 2 if comparison else 1)
                self.assertNotIn("--skip-zk", events[0])


if __name__ == "__main__":
    unittest.main()
