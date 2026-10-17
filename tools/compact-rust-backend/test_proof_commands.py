#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Proof-profile selection must preserve workload and failure propagation."""
import os
from pathlib import Path
import shlex
import subprocess
import unittest
import tomllib
from unittest.mock import patch

import check_compactc_target as cli
import proof_commands

ROOT = Path(__file__).resolve().parents[2]


class ProofCommands(unittest.TestCase):
    def test_repository_proof_profile_preserves_checked_workspace_code(self):
        manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
        profile = manifest["profile"]["proof-checks"]
        self.assertEqual(profile["inherits"], "dev")
        self.assertIs(profile["debug-assertions"], True)
        self.assertIs(profile["overflow-checks"], True)
        # Dependency optimization must not become a workspace-wide release build
        # or acquire package-specific overrides that disable the inherited checks.
        self.assertNotIn("opt-level", profile)
        self.assertEqual(profile["package"], {"*": {"opt-level": 3}})

    def test_workflow_preparation_uses_the_same_checked_proof_profile(self):
        workflow = (ROOT / ".github/workflows/build-compiler.yml").read_text()
        preparation = [shlex.split(line.strip()) for line in workflow.splitlines()
                       if line.lstrip().startswith("cargo run ")
                       and "--prepare-proof-material" in line]
        self.assertEqual(len(preparation), 1, "expected one direct proof-material preparation")
        command = preparation[0]
        boundary = command.index("--")
        options = command[:boundary]
        self.assertEqual(options[options.index("--profile") + 1], "proof-checks")
        self.assertEqual(options[options.index("-p") + 1], "compact-rust-proof-smoke")
        self.assertIn("--locked", options)
        self.assertIn("--offline", options)
        self.assertNotIn("--release", options)
        self.assertNotIn("-r", options)
        self.assertEqual(options.count("--profile"), 1)
        self.assertEqual(command[boundary + 1], "--prepare-proof-material")

    def test_capability_preflight_precedes_proofs_and_retains_later_live_gate(self):
        workflow = (ROOT / ".github/workflows/build-compiler.yml").read_text()
        command = ("env -u LD_LIBRARY_PATH nix develop .#compiler --command env "
                   "COMPACTC=compactc python3 -X faulthandler -u "
                   "tools/compact-rust-backend/check_rejections.py")
        self.assertEqual(workflow.count(command), 2)
        first, last = workflow.index(command), workflow.rindex(command)
        self.assertLess(workflow.index("- name: Build compiler"), first)
        self.assertLess(first, workflow.index("- name: Run compiler tests"))
        self.assertLess(first, workflow.index("--prepare-proof-material"))
        self.assertLess(workflow.index("--consumer --proof"), last)

    def test_existing_run_options_and_all_executable_arguments_survive(self):
        for prefix in (["cargo", "run"], ["cargo", "+1.99.0", "run"]):
            for package in ("-p", "--package"):
                original = [*prefix, "--quiet", "--locked", "--offline", package,
                            "compact-rust-proof-smoke", "--", "--coracle-start",
                            "/private/output with spaces", "--profile", "consumer-argument"]
                boundary = original.index("--")
                self.assertEqual(proof_commands.command(original, {}),
                    original[:boundary] + ["--profile", "proof-checks"] + original[boundary:])

    def test_nonproof_work_and_package_like_executable_arguments_are_untouched(self):
        for original in (
            ["cargo", "test", "-p", "compact-rust-proof-smoke"],
            ["cargo", "check", "--manifest-path", "consumer/Cargo.toml"],
            ["cargo", "run", "-p", "compact-rust-backend", "--", "-p", "compact-rust-proof-smoke"],
            ["zkir", "compile", "current.zkir", "current.prover", "current.verifier"],
        ):
            self.assertEqual(proof_commands.command(original,
                {"COMPACT_RUST_PROOF_PROFILE": "invalid"}), original)

    def test_dev_baseline_and_preparation_without_executable_separator(self):
        original = ["cargo", "run", "-p", "compact-rust-proof-smoke"]
        self.assertEqual(proof_commands.command(original, {"COMPACT_RUST_PROOF_PROFILE": "dev"}),
                         original + ["--profile", "dev"])
        preparation = original + ["--", "--prepare-proof-material", "--verify-only"]
        self.assertEqual(proof_commands.command(preparation, {}),
                         original + ["--profile", "proof-checks", *preparation[len(original):]])

    def test_unknown_or_conflicting_profiles_refuse_instead_of_dropping_checks(self):
        original = ["cargo", "run", "-p", "compact-rust-proof-smoke"]
        for profile in ("", "release", "unknown"):
            with self.subTest(profile=profile), self.assertRaises(ValueError):
                proof_commands.command(original, {"COMPACT_RUST_PROOF_PROFILE": profile})
        for option in (["--release"], ["-r"], ["--profile", "dev"], ["--profile=proof-checks"]):
            with self.subTest(option=option), self.assertRaises(ValueError):
                proof_commands.command(original + option, {})

    def test_runner_retains_cwd_environment_and_original_subprocess_failure(self):
        original = ("cargo", "run", "-p", "compact-rust-proof-smoke", "--", "--coracle-start", "output")
        environment = {"COMPACT_RUST_PROOF_PROFILE": "dev", "COMPACT_ZKIR_MODE": "uncached"}
        failure = subprocess.CalledProcessError(23, original)
        with patch.object(cli.subprocess, "run", side_effect=failure) as run:
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                cli.run(*original, cwd=Path("fixture"), env=environment)
        self.assertIs(raised.exception, failure)
        run.assert_called_once_with(proof_commands.command(original, environment),
                                    cwd=Path("fixture"), env=environment, check=True)

    def test_runner_uses_process_environment_when_no_override_is_passed(self):
        original = ("cargo", "run", "-p", "compact-rust-proof-smoke", "--", "--effect-plan-jubjub", "output")
        with patch.dict(os.environ, {"COMPACT_RUST_PROOF_PROFILE": "dev"}), \
                patch.object(cli.subprocess, "run") as run:
            cli.run(*original)
        self.assertEqual(run.call_args.args[0][-5:],
                         ["--profile", "dev", "--", "--effect-plan-jubjub", "output"])
        self.assertIsNone(run.call_args.kwargs["env"])


if __name__ == "__main__":
    unittest.main()
