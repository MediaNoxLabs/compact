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

"""Shielded gate boundaries with synthetic compiler artifacts and captured commands."""

from contextlib import redirect_stdout
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
from target_checks import shielded


CASES = {
    "receive": {
        "check": shielded.check_receive,
        "circuits": ["accept", "accept_renamed"],
        "primitives": ["create_zswap_output", "CoinReceive", "crate::pure_circuits::coinCommitment"],
        "proofs": ["--shielded-receive", "--wallet-funded-receive", "--observational-receive"],
    },
    "merge": {
        "check": shielded.check_merge,
        "circuits": ["merge_qualified", "receive_then_merge"],
        "primitives": [],
        "proofs": ["--shielded-merge"],
    },
    "send": {
        "check": shielded.check_send,
        "circuits": ["send_to_self", "send_to_user", "send_to_contract", "send_forward"],
        "primitives": ["create_zswap_input", "create_zswap_output", "subtract_unsigned",
                       "degrade_to_transient", "transient_hash", "upgrade_from_transient"],
        "proofs": ["--shielded-send", "--observational-send"],
    },
}


class ShieldedTargetChecks(unittest.TestCase):
    def runner(self, kind, *, mutate=None, fail_proof=False):
        commands = []

        def run(*arguments):
            commands.append(arguments)
            if arguments[0] != "compactc-under-test":
                if fail_proof:
                    raise subprocess.CalledProcessError(7, arguments)
                return
            contract = Path(arguments[-1]) / "contract"
            contract.mkdir(parents=True)
            report = {"circuits": [
                {"name": name, "proof_required": True, "recorded": True, "observed_call": True}
                for name in CASES[kind]["circuits"]
            ]}
            (contract / "compact-rust-ir.json").write_text(json.dumps({"schema_version": 20}))
            (contract / "rust-capabilities.json").write_text(json.dumps(report))
            # Text-only inputs to the gate's existing primitive checks; not executable Rust.
            (contract / "lib.rs").write_text("\n".join(CASES[kind]["primitives"]))
            if mutate:
                mutate(contract)

        return commands, run

    def test_optional_proof_commands_keep_order_flags_and_shared_artifact_root(self):
        for kind, case in CASES.items():
            for proof in [False, True]:
                with self.subTest(kind=kind, proof=proof), tempfile.TemporaryDirectory() as tmp:
                    base = Path(tmp)
                    source = base / "contract source.compact"
                    output = base / f"shielded-{kind}"
                    commands, run = self.runner(kind)
                    with redirect_stdout(io.StringIO()):
                        case["check"]("compactc-under-test", base, source, proof=proof, run=run)
                    expected = [("compactc-under-test", "--target", "rust", "--rust-require-recording",
                                 *([] if proof else ["--skip-zk"]), str(source), str(output))]
                    if proof:
                        expected += [("cargo", "+1.99.0", "run", "--offline", "--quiet", "-p",
                                      "compact-rust-proof-smoke", "--", selector, str(output))
                                     for selector in case["proofs"]]
                    self.assertEqual(commands, expected)

    def test_invalid_capabilities_abort_before_proof_commands(self):
        for kind, case in CASES.items():
            for field, value in [("name", "unexpected"), ("proof_required", False),
                                 ("recorded", False), ("observed_call", False)]:
                with self.subTest(kind=kind, field=field), tempfile.TemporaryDirectory() as tmp:
                    def mutate(contract):
                        path = contract / "rust-capabilities.json"
                        report = json.loads(path.read_text())
                        report["circuits"][0][field] = value
                        path.write_text(json.dumps(report))

                    commands, run = self.runner(kind, mutate=mutate)
                    with self.assertRaises(AssertionError):
                        case["check"]("compactc-under-test", Path(tmp), Path("source.compact"),
                                      proof=True, run=run)
                    self.assertEqual(len(commands), 1)

    def test_wrong_schema_and_missing_primitives_abort_before_proof_commands(self):
        for kind in ["receive", "send"]:
            for missing in [None, *CASES[kind]["primitives"]]:
                with self.subTest(kind=kind, missing=missing), tempfile.TemporaryDirectory() as tmp:
                    def mutate(contract):
                        if missing is None:
                            (contract / "compact-rust-ir.json").write_text('{"schema_version": 19}')
                        else:
                            path = contract / "lib.rs"
                            path.write_text(path.read_text().replace(missing, ""))

                    commands, run = self.runner(kind, mutate=mutate)
                    with self.assertRaises(AssertionError):
                        CASES[kind]["check"]("compactc-under-test", Path(tmp), Path("source.compact"),
                                            proof=True, run=run)
                    self.assertEqual(len(commands), 1)

    def test_failed_proof_stops_followup_proofs_and_success_summary(self):
        for kind, case in CASES.items():
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as tmp:
                commands, run = self.runner(kind, fail_proof=True)
                output = io.StringIO()
                with redirect_stdout(output), self.assertRaises(subprocess.CalledProcessError):
                    case["check"]("compactc-under-test", Path(tmp), Path("source.compact"),
                                  proof=True, run=run)
                self.assertEqual(len(commands), 2)
                self.assertEqual(commands[-1][-2], case["proofs"][0])
                self.assertEqual(output.getvalue(), "")

    def test_cli_keeps_flag_precedence_source_selection_and_temporary_lifetime(self):
        for flags, expected, source in [
            (["--shielded-receive", "--shielded-merge", "--shielded-send"],
             "receive", cli.SHIELDED_RECEIVE_SOURCE),
            (["--shielded-merge", "--shielded-send"], "merge",
             cli.ROOT / "examples/rust_backend/shielded_merge_oracle.compact"),
            (["--shielded-send"], "send", cli.SHIELDED_SEND_SOURCE),
        ]:
            with self.subTest(flags=flags):
                calls = []

                def check(compiler, base, actual_source, *, proof, run):
                    self.assertTrue(base.is_dir())
                    calls.append((compiler, base, actual_source, proof, run))

                with patch.object(sys, "argv", ["check_compactc_target.py", *flags]), \
                     patch.dict(os.environ, {"COMPACTC": "compactc-under-test"}), \
                     patch.object(shielded, "check_receive") as receive, \
                     patch.object(shielded, "check_merge") as merge, \
                     patch.object(shielded, "check_send") as send:
                    selected = {"receive": receive, "merge": merge, "send": send}
                    selected[expected].side_effect = check
                    cli.main()
                    for name, mock in selected.items():
                        self.assertEqual(mock.call_count, int(name == expected))
                self.assertEqual(len(calls), 1)
                compiler, base, actual_source, proof, run = calls[0]
                self.assertEqual((compiler, actual_source, proof, run),
                                 ("compactc-under-test", source, False, cli.run))
                self.assertFalse(base.exists())


if __name__ == "__main__":
    unittest.main()
