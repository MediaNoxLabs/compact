# This file is part of Compact.
# Copyright (C) 2025 Midnight Foundation
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

"""Subprocess tests for read-only header CLI paths and default add behavior."""

from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent
SCRIPT = ROOT / "add_headers.py"


class HeaderCommandTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="compact-header-cli-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "example.py").write_bytes(b"#!/usr/bin/env python3\nprint('unchanged')\n")

    def snapshot(self):
        return {str(path.relative_to(self.root)): path.read_bytes()
                for path in self.root.rglob("*") if path.is_file()}

    def command(self, *args):
        return subprocess.run([sys.executable, str(SCRIPT), *args], cwd=self.root,
                              capture_output=True, text=True, check=False)

    def configure(self):
        shutil.copyfile(ROOT / "header_config.json", self.root / "header_config.json")

    def test_help_does_not_scan_or_modify_without_configuration(self):
        before = self.snapshot()
        result = self.command("--help")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("--validate", result.stdout)
        self.assertNotIn("error loading configuration", result.stderr)
        self.assertEqual(before, self.snapshot())

    def test_unknown_arguments_never_fall_through_to_add(self):
        self.configure()
        before = self.snapshot()
        for args in [("--unknown",), ("--val",), ("typo",), ("--validate", "--unknown"),
                     ("--validate", "unexpected-positional")]:
            with self.subTest(args=args):
                result = self.command(*args)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("unrecognized arguments", result.stderr)
                self.assertEqual(before, self.snapshot())

    def test_validation_is_read_only_and_default_add_is_idempotent(self):
        self.configure()
        before = self.snapshot()
        result = self.command("--validate")
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(before, self.snapshot())
        result = self.command()
        self.assertEqual(result.returncode, 0, result.stderr)
        changed = (self.root / "example.py").read_text()
        self.assertTrue(changed.startswith("#!/usr/bin/env python3\n"))
        self.assertTrue(changed.endswith("print('unchanged')\n"))
        self.assertIn("Licensed under the Apache License", changed)
        after = self.snapshot()
        self.assertEqual(self.command("--validate").returncode, 0)
        self.assertEqual(self.command().returncode, 0)
        self.assertEqual(after, self.snapshot())


if __name__ == "__main__":
    unittest.main()
