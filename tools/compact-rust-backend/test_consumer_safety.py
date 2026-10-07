#!/usr/bin/env python3

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

"""Failure-class and non-destructive cleanup checks for external safety controls."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from consumer_safety import check_consumer_safety, require_expected_rejection


class ConsumerSafetyTests(unittest.TestCase):
    def result(self, code, file="typed_safety_case.rs", extra=()):
        def diagnostic(value, filename):
            return json.dumps({"reason": "compiler-message", "message": {
                "level": "error", "code": {"code": value},
                "spans": [{"is_primary": True, "file_name": filename}],
            }})
        return subprocess.CompletedProcess([], 101, "\n".join([
            diagnostic(code, file), *(diagnostic(c, f) for c, f in extra),
        ]), "")

    def test_only_intended_consumer_error_counts_as_refusal(self):
        require_expected_rejection(self.result("E0382"), "E0382", "typed_safety_case")
        for result in [
            subprocess.CompletedProcess([], 101, "", "offline package missing"),
            subprocess.CompletedProcess([], 0, "", ""),
            self.result("E0308"),
            self.result("E0382", "dependency.rs"),
            self.result("E0382", extra=[("E0432", "typed_safety_case.rs")]),
        ]:
            with self.subTest(result=result), self.assertRaises(RuntimeError):
                require_expected_rejection(result, "E0382", "typed_safety_case")

    def test_valid_compile_failure_cleans_only_created_files(self):
        with tempfile.TemporaryDirectory() as directory:
            consumer = Path(directory)
            tests = consumer / "tests"
            tests.mkdir()
            sentinel = tests / "existing.rs"
            sentinel.write_text("retained")
            with patch("consumer_safety.subprocess.run", return_value=subprocess.CompletedProcess(
                [], 101, "", "missing dependency"
            )), self.assertRaisesRegex(RuntimeError, "valid consumer failed"):
                check_consumer_safety(consumer, {})
            self.assertEqual(list(tests.iterdir()), [sentinel])
            self.assertEqual(sentinel.read_text(), "retained")

    def test_existing_target_is_not_overwritten_or_removed(self):
        with tempfile.TemporaryDirectory() as directory:
            consumer = Path(directory)
            tests = consumer / "tests"
            tests.mkdir()
            target = tests / "typed_safety_thread_returned_context.rs"
            target.write_text("retained")
            with patch("consumer_safety.subprocess.run") as run, self.assertRaises(FileExistsError):
                check_consumer_safety(consumer, {})
            run.assert_not_called()
            self.assertEqual(target.read_text(), "retained")


if __name__ == "__main__":
    unittest.main()
