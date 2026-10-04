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

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("parity_inventory.py")
SPEC = importlib.util.spec_from_file_location("parity_inventory", SCRIPT)
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)


class ParityInventoryTests(unittest.TestCase):
    def test_checked_baseline_roundtrip_and_known_bad_drift(self):
        current = inventory.baseline_rows(inventory.make_inventory(inventory.ROOT, [], None)["rows"])
        checked = json.loads(inventory.DEFAULT_BASELINE.read_text())
        self.assertEqual(checked, current)
        with tempfile.TemporaryDirectory() as directory:
            bad_baseline = Path(directory) / "bad-baseline.json"
            bad_baseline.write_text(json.dumps(checked[:-1]))
            output = Path(directory) / "receipt.json"
            result = subprocess.run([sys.executable, str(SCRIPT), "--baseline", str(bad_baseline),
                                     "--output", str(output)], capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertEqual(len(json.loads(output.read_text())["baseline_diff"]["added"]), 1)

    def test_declarations_and_module_scope(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/sample.compact"
            source.parent.mkdir(parents=True)
            source.write_text("""// export circuit fake(): [];
import CompactStandardLibrary;
include "./part";
witness hint(value: Field): Field;
module Inner<T> {
  export circuit nested(
    amount: Uint<16>
  ): Field { return amount as Field; }
}
export circuit live(value: Field): Field { return value; }
""")
            contract = inventory.parse_source(source, root)
            declarations = contract["declarations"]
            self.assertEqual([(row["kind"], row["name"], row["module_path"])
                              for row in declarations],
                             [("witness", "hint", []), ("module", "Inner", []),
                              ("circuit", "nested", ["Inner"]), ("circuit", "live", [])])
            self.assertEqual(declarations[2]["signature"],
                             "export circuit nested( amount: Uint<16> ): Field")
            self.assertEqual(contract["imports"],
                             [{"kind": "import", "expression": "CompactStandardLibrary"},
                              {"kind": "include", "expression": '"./part"'}])

    def test_baseline_detects_additions_and_removals(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/sample.compact"
            source.parent.mkdir(parents=True)
            source.write_text("export circuit first(): Field { return 1; }\n")
            baseline = root / "baseline.json"
            output = root / "receipt.json"
            def run(*args):
                return subprocess.run([sys.executable, str(SCRIPT), "--root", str(root),
                                       "--output", str(output), *args],
                                      capture_output=True, text=True, check=False)
            self.assertEqual(run("--write-baseline", str(baseline)).returncode, 0)
            self.assertEqual(run("--baseline", str(baseline)).returncode, 0)
            source.write_text("export circuit second(): Field { return 2; }\n")
            self.assertEqual(run("--baseline", str(baseline)).returncode, 1)
            diff = json.loads(output.read_text())["baseline_diff"]
            self.assertEqual([row["name"] for row in diff["added"]], ["second"])
            self.assertEqual([row["name"] for row in diff["removed"]], ["first"])

    def test_compiler_capability_ingestion_and_full_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/sample.compact"
            source.parent.mkdir(parents=True)
            source.write_text("export circuit available(): [];\nexport circuit missing(): [];\n")
            compiler = root / "compiler.py"
            compiler.write_text("""#!/usr/bin/env python3
import json, pathlib, sys
contract = pathlib.Path(sys.argv[-1]) / "contract"
contract.mkdir(parents=True)
report = {"schema_version": 3, "circuits": [
  {"name": "available", "recorded": True, "observed_call": True,
   "proof_required": True, "recording_status": "available"},
  {"name": "missing", "recorded": False, "observed_call": False,
   "proof_required": True, "recording_status": "unavailable"}]}
(contract / "rust-capabilities.json").write_text(json.dumps(report))
""")
            compiler.chmod(0o755)
            output = root / "receipt.json"
            result = subprocess.run([sys.executable, str(SCRIPT), "--root", str(root),
                                     "--compiler", str(compiler), "--require-full", "--output", str(output)],
                                    capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 1, result.stderr)
            summary = json.loads(output.read_text())["summary"]
            self.assertEqual(summary["recorded_available"], 1)
            self.assertEqual(summary["recorded_missing"], 1)
            self.assertEqual(summary["proof_required"], 2)
            self.assertEqual(summary["proof_missing"], 1)
            self.assertEqual(summary["ranked_missing_sources"],
                             [{"source": "examples/rust_backend/sample.compact", "count": 1}])

    def test_nonproof_missing_api_does_not_fail_full_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/sample.compact"
            source.parent.mkdir(parents=True)
            source.write_text("export circuit private_only(): Boolean;\n")
            compiler = root / "compiler.py"
            compiler.write_text("""#!/usr/bin/env python3
import json, pathlib, sys
contract = pathlib.Path(sys.argv[-1]) / "contract"
contract.mkdir(parents=True)
report = {"schema_version": 3, "circuits": [
  {"name": "private_only", "recorded": False, "observed_call": False,
   "proof_required": False, "recording_status": "not_applicable"}]}
(contract / "rust-capabilities.json").write_text(json.dumps(report))
""")
            compiler.chmod(0o755)
            output = root / "receipt.json"
            result = subprocess.run([sys.executable, str(SCRIPT), "--root", str(root),
                                     "--compiler", str(compiler), "--require-full", "--output", str(output)],
                                    capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            summary = json.loads(output.read_text())["summary"]
            self.assertEqual(summary["recorded_missing"], 1)
            self.assertEqual(summary["nonproof"], 1)
            self.assertEqual(summary["proof_missing"], 0)

    def test_receipt_metadata_is_exact_and_optional(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/sample.compact"
            source.parent.mkdir(parents=True)
            source.write_text("export circuit sample(): [];\n")
            (root / "Cargo.lock").write_text('''[[package]]\nname = "midnight-ledger"\nversion = "8.0.3"\nchecksum = "abc"\n''')
            ir = root / "tools/compact-rust-backend/src/ir.rs"
            ir.parent.mkdir(parents=True)
            ir.write_text("pub const SCHEMA_VERSION: u32 = 8;\n")
            runtime = root / "runtime-rs/src/lib.rs"
            runtime.parent.mkdir(parents=True)
            runtime.write_text("pub const RUST_RUNTIME_ABI: u32 = 36;\n")
            result = inventory.make_inventory(root, [], None)
            self.assertNotIn("receipt_metadata", result)
            metadata = inventory.receipt_metadata(root, None, result["contracts"])
            self.assertEqual(metadata["rust_ir_schema"], 8)
            self.assertEqual(metadata["rust_runtime_abi"], 36)
            self.assertEqual(metadata["upstream_packages"]["midnight-ledger"],
                             {"version": "8.0.3", "checksum": "abc"})
            self.assertEqual(metadata["source_manifest_sha256"],
                             inventory.receipt_metadata(root, None, result["contracts"])["source_manifest_sha256"])


if __name__ == "__main__":
    unittest.main()
