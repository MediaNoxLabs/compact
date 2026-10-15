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

"""Proof-file boundaries with real artifacts and ordered filesystem observations."""

from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from target_checks.proof_artifacts import require_circuit_files


class ProofArtifactChecks(unittest.TestCase):
    def paths(self, output, circuit):
        return [output / "keys" / f"{circuit}.prover",
                output / "keys" / f"{circuit}.verifier",
                output / "zkir" / f"{circuit}.zkir",
                output / "zkir" / f"{circuit}.bzkir"]

    def populate(self, paths):
        for path in paths:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"fixture artifact")

    def test_complete_artifacts_accept_circuit_names_and_paths_with_spaces(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "contract artifacts"
            for circuit in ("increment", "vote$commit", "typed circuit"):
                paths = self.paths(output, circuit)
                self.populate(paths)
                self.assertIsNone(require_circuit_files(output, circuit))

    def test_missing_artifact_stops_at_first_absent_file_in_required_order(self):
        for missing in range(4):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp)
                paths = self.paths(output, "vote$reveal")
                self.populate(paths)
                paths[missing].unlink()
                observed = []
                is_file = Path.is_file

                def inspect(path):
                    observed.append(path)
                    return is_file(path)

                with patch.object(Path, "is_file", inspect):
                    with self.assertRaises(AssertionError) as error:
                        require_circuit_files(output, "vote$reveal")
                self.assertEqual(str(error.exception), "")
                self.assertEqual(observed, paths[:missing + 1])

    def test_directory_cannot_satisfy_an_artifact_requirement(self):
        for directory in range(4):
            with self.subTest(directory=directory), tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp)
                paths = self.paths(output, "check")
                self.populate(paths)
                paths[directory].unlink()
                paths[directory].mkdir()
                with self.assertRaises(AssertionError):
                    require_circuit_files(output, "check")

    def test_success_checks_all_four_files_once_in_order(self):
        output = Path("proof root")
        observed = []

        def inspect(path):
            observed.append(path)
            return True

        with patch.object(Path, "is_file", inspect):
            require_circuit_files(output, "check")
        self.assertEqual(observed, self.paths(output, "check"))

    def test_filesystem_exception_keeps_identity_and_stops_later_checks(self):
        output = Path("proof root")
        observed = []
        failure = OSError("filesystem unavailable")

        def inspect(path):
            observed.append(path)
            if len(observed) == 2:
                raise failure
            return True

        with patch.object(Path, "is_file", inspect):
            with self.assertRaises(OSError) as error:
                require_circuit_files(output, "check")
        self.assertIs(error.exception, failure)
        self.assertEqual(observed, self.paths(output, "check")[:2])


if __name__ == "__main__":
    unittest.main()
