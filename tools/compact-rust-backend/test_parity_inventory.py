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
import re
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("parity_inventory.py")
SPEC = importlib.util.spec_from_file_location("parity_inventory", SCRIPT)
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)
SCOPE_SPEC = importlib.util.spec_from_file_location(
    "check_positive_source_scope", SCRIPT.with_name("check_positive_source_scope.py"))
source_scope = importlib.util.module_from_spec(SCOPE_SPEC)
SCOPE_SPEC.loader.exec_module(source_scope)


class ParityInventoryTests(unittest.TestCase):
    def package_fixture(self, root: Path) -> tuple[Path, Path, Path, Path]:
        package = root / next(iter(inventory.COMPILED_PACKAGE_ROOTS))
        package.parent.mkdir(parents=True)
        package.write_text('include "./parts/reachable";\n')
        reachable = package.parent / "parts/reachable.compact"
        reachable.parent.mkdir()
        reachable.write_text("export pure circuit helper(): Boolean { return true; }\n")
        unreachable = package.parent / "parts/unreachable.compact"
        unreachable.write_text("export pure circuit helper(): Boolean { return false; }\n")
        metadata = root / "compiler-info.json"
        metadata.write_text(json.dumps([{"name": "helper", "pure": True, "proof": False}]))
        compiler = root / "compiler.py"
        compiler.write_text("""#!/usr/bin/env python3
import json, pathlib, sys
root = pathlib.Path(__file__).parent
output = pathlib.Path(sys.argv[-1])
contract = output / "contract"
contract.mkdir(parents=True)
(contract / "rust-capabilities.json").write_text(json.dumps({"schema_version": 3, "circuits": []}))
metadata = output / "compiler"
metadata.mkdir()
(metadata / "contract-info.json").write_text(json.dumps({
    "circuits": json.loads((root / "compiler-info.json").read_text())
}))
""")
        compiler.chmod(0o755)
        return package, reachable, metadata, compiler

    def test_imported_pure_metadata_uses_reachable_source_without_identity_change(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package, reachable, _, compiler = self.package_fixture(root)
            before = inventory.make_inventory(root, [], None)
            after = inventory.make_inventory(root, [], compiler)
            self.assertEqual(inventory.baseline_rows(before["rows"]),
                             inventory.baseline_rows(after["rows"]))
            rows = {row["source"]: row for row in after["rows"] if row["name"] == "helper"}
            reached = rows[inventory.relative_source(reachable, root)]
            self.assertEqual((reached["proof_required"], reached["compiler_pure"],
                              reached["rust_recording_status"], reached["rust_recorded"],
                              reached["compiler_metadata_source"]),
                             (False, True, "not_applicable", None,
                              inventory.relative_source(package, root)))
            self.assertIsNone(rows[inventory.relative_source(
                package.parent / "parts/unreachable.compact", root)]["proof_required"])
            self.assertIsNone(rows[inventory.relative_source(
                package.parent / "parts/unreachable.compact", root)]["compiler_metadata_source"])
            self.assertEqual(after["summary"]["compiled_rust_sources"], 1)
            self.assertEqual(after["summary"]["nonproof"], 1)
            self.assertEqual(after["summary"]["unassessed_exported_circuits"], 1)
            focused = inventory.make_inventory(
                root, [], compiler, {inventory.relative_source(package, root)})
            self.assertEqual(focused["summary"]["sources"], 2)
            self.assertEqual(focused["summary"]["nonproof"], 1)
            self.assertEqual(focused["summary"]["unassessed_exported_circuits"], 0)

    def test_imported_pure_metadata_rejects_ambiguous_or_mismatched_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package, _, metadata, compiler = self.package_fixture(root)
            for entries, error in (
                ([{"name": "helper", "pure": False, "proof": False}],
                 "included declaration disagrees"),
                ([{"name": "helper", "pure": True, "proof": True}],
                 "included declaration disagrees"),
                ([{"name": "other", "pure": True, "proof": False}],
                 "no included declaration"),
                ([], "included declaration has no compiler proof row"),
                ([{"name": "helper", "pure": True, "proof": False}] * 2,
                 "ambiguous compiler proof rows"),
            ):
                with self.subTest(error=error, entries=entries):
                    metadata.write_text(json.dumps(entries))
                    with self.assertRaisesRegex(ValueError, error):
                        inventory.make_inventory(root, [], compiler)
            metadata.write_text(json.dumps([{"name": "helper", "pure": True, "proof": False}]))
            duplicate = package.parent / "parts/duplicate.compact"
            duplicate.write_text("export pure circuit helper(): Boolean { return true; }\n")
            package.write_text('include "./parts/reachable";\ninclude "./parts/duplicate";\n')
            with self.assertRaisesRegex(ValueError, "ambiguous included declaration"):
                inventory.make_inventory(root, [], compiler)
            package.write_text('include "./parts/missing";\n')
            with self.assertRaisesRegex(ValueError, "invalid repository source"):
                inventory.make_inventory(root, [], compiler)
            package.write_text('include "../../../../outside";\n')
            with self.assertRaisesRegex(ValueError, "invalid repository source"):
                inventory.make_inventory(root, [], compiler)
            with tempfile.TemporaryDirectory() as external:
                outside = Path(external) / "outside.compact"
                outside.write_text("export pure circuit helper(): Boolean { return true; }\n")
                escape = package.parent / "parts/escape.compact"
                escape.symlink_to(outside)
                package.write_text('include "./parts/escape";\n')
                with self.assertRaisesRegex(ValueError, "invalid repository source"):
                    inventory.make_inventory(root, [], compiler)
                escape.unlink()
            package.write_text('include ./parts/reachable;\n')
            with self.assertRaisesRegex(ValueError, "not a quoted local path"):
                inventory.make_inventory(root, [], compiler)
    def test_adt_list_positive_sources_are_checked(self):
        manifest = json.loads(inventory.ADT_LIST_SOURCE_MANIFEST.read_text())
        self.assertEqual(source_scope.cohort_membership_failures(manifest), [])
        entries = manifest["positive_sources"]
        self.assertEqual({entry["source"] for entry in entries}, {
            "examples/adt/tests/list_enum.compact",
            "examples/adt/tests/list_field.compact",
        })
        self.assertTrue(all((entry["expected_ts"], entry["expected_rust"]) ==
                            ("success", "success") for entry in entries))
        self.assertTrue(all(entry["proof_circuits"] ==
                            [{"name": "test", "pure": False, "proof": True}]
                            for entry in entries))
        scanned = {path.relative_to(inventory.ROOT).as_posix()
                   for path in inventory.source_paths(inventory.ROOT)}
        self.assertTrue({entry["source"] for entry in entries} <= scanned)

    def test_adt_list_bytes_source_is_checked(self):
        manifest = json.loads(inventory.ADT_LIST_BYTES_SOURCE_MANIFEST.read_text())
        self.assertEqual(source_scope.cohort_membership_failures(manifest), [])
        self.assertEqual(len(manifest["positive_sources"]), 1)
        entry = manifest["positive_sources"][0]
        self.assertEqual(entry["source"], "examples/adt/tests/list_bytes.compact")
        self.assertEqual((entry["expected_ts"], entry["expected_rust"]),
                         ("success", "success"))
        self.assertEqual(entry["proof_circuits"],
                         [{"name": "test", "pure": False, "proof": True}])
        scanned = {path.relative_to(inventory.ROOT).as_posix()
                   for path in inventory.source_paths(inventory.ROOT)}
        self.assertIn(entry["source"], scanned)

    def test_adt_list_vector_field_four_source_is_checked(self):
        manifest = json.loads(inventory.ADT_LIST_VECTOR_FIELD_4_SOURCE_MANIFEST.read_text())
        self.assertEqual(source_scope.cohort_membership_failures(manifest), [])
        self.assertEqual(len(manifest["positive_sources"]), 1)
        entry = manifest["positive_sources"][0]
        self.assertEqual(entry["source"], "examples/adt/tests/list_vector_field_4.compact")
        self.assertEqual((entry["expected_ts"], entry["expected_rust"]),
                         ("success", "success"))
        self.assertEqual(entry["proof_circuits"],
                         [{"name": "test", "pure": False, "proof": True}])
        scanned = {path.relative_to(inventory.ROOT).as_posix()
                   for path in inventory.source_paths(inventory.ROOT)}
        self.assertIn(entry["source"], scanned)

    def test_adt_set_positive_cohort_is_checked_and_glob_locked(self):
        manifest = json.loads(inventory.ADT_SET_SOURCE_MANIFEST.read_text())
        entries = manifest["positive_sources"]
        self.assertEqual(len(entries), 5)
        self.assertEqual(source_scope.cohort_membership_failures(manifest), [])
        self.assertEqual({entry["expected_rust"] for entry in entries}, {"success"})
        self.assertEqual(sum(entry["expected_rust"] == "success" for entry in entries), 5)
        self.assertEqual(sum(len(entry["proof_circuits"]) for entry in entries), 5)
        self.assertTrue(all(circuit == {"name": circuit["name"], "pure": False, "proof": True}
                            for entry in entries for circuit in entry["proof_circuits"]))
        suite = (inventory.ROOT / manifest["suite"]).read_text()
        self.assertIn("buildPathTo('/adt/tests')", suite)
        self.assertIn("files.forEach", suite)
        self.assertIn("toBeSuccess", suite)
        scanned = {path.relative_to(inventory.ROOT).as_posix()
                   for path in inventory.source_paths(inventory.ROOT)}
        self.assertTrue({entry["source"] for entry in entries} <= scanned)
        bad_manifest = {**manifest, "positive_sources": entries[:-1]}
        self.assertIn("source cohort membership changed",
                      source_scope.cohort_membership_failures(bad_manifest)[0])

    def test_top_level_cohort_names_original_sources_and_ts_references(self):
        manifest = json.loads(inventory.TOP_LEVEL_SOURCE_MANIFEST.read_text())
        entries = manifest["positive_sources"]
        self.assertEqual([entry["source"] for entry in entries],
                         ["examples/counter.compact", "examples/tiny.compact"])
        self.assertEqual(manifest["expected_rejections"], [])
        scope = inventory.positive_scope(inventory.ROOT)
        self.assertEqual({entry["source"] for entry in scope["positive_sources"]
                          if entry["source"] in {item["source"] for item in entries}},
                         {item["source"] for item in entries})
        for entry in entries:
            self.assertEqual((entry["expected_ts"], entry["expected_rust"]),
                             ("success", "success"))
            source = inventory.ROOT / entry["source"]
            reference = inventory.ROOT / entry["typescript_reference"]
            fixture_source = inventory.ROOT / entry["rust_fixture_source"]
            fixture = inventory.ROOT / entry["rust_fixture"]
            self.assertTrue(reference.is_file())
            self.assertIn(entry["source"], reference.read_text())
            self.assertTrue(fixture.is_file())
            self.assertEqual(inventory.without_comments(source.read_text()).strip(),
                             inventory.without_comments(fixture_source.read_text()).strip())
            declarations = inventory.parse_source(source, inventory.ROOT)["declarations"]
            exported = {item["name"] for item in declarations
                        if item["kind"] == "circuit" and item["visibility"] == "export"}
            self.assertEqual({item["name"] for item in entry["proof_circuits"]}, exported)
        self.assertEqual(sum(circuit["proof"] for entry in entries
                             for circuit in entry["proof_circuits"]), 5)
        self.assertEqual(sum(not circuit["proof"] for entry in entries
                             for circuit in entry["proof_circuits"]), 1)

    def test_test_center_counter_cohort_preserves_original_source_identity(self):
        manifest = json.loads(inventory.TEST_CENTER_COUNTER_SOURCE_MANIFEST.read_text())
        self.assertEqual(source_scope.cohort_membership_failures(manifest), [])
        self.assertEqual(len(manifest["positive_sources"]), 1)
        entry = manifest["positive_sources"][0]
        self.assertEqual(entry["source"], "test-center/test-contracts/counter.compact")
        self.assertEqual(entry["proof_circuits"],
                         [{"name": "increment", "pure": False, "proof": True}])
        self.assertIn(entry["source"],
                      (inventory.ROOT / manifest["suite"]).read_text())
        self.assertIn("CONTRACTS_ROOT + 'counter.compact'",
                      (inventory.ROOT / entry["typescript_reference"]).read_text())
        self.assertIn(entry["source"],
                      (inventory.ROOT / entry["typescript_capture"]).read_text())
        self.assertEqual(json.loads((inventory.ROOT / entry["typescript_fixture"]).read_text())
                         ["source"], entry["source"])
        self.assertIn(Path(entry["typescript_fixture"]).name,
                      (inventory.ROOT / entry["rust_test"]).read_text())
        self.assertEqual({item["name"] for item in
                          inventory.parse_source(inventory.ROOT / entry["source"],
                                                 inventory.ROOT)["declarations"]
                          if item["kind"] == "circuit" and item["visibility"] == "export"},
                         {"increment"})

    def test_test_center_welcome_cohort_keeps_proof_gaps_explicit(self):
        manifest = json.loads(inventory.TEST_CENTER_WELCOME_SOURCE_MANIFEST.read_text())
        self.assertEqual(source_scope.cohort_membership_failures(manifest), [])
        self.assertEqual(len(manifest["positive_sources"]), 1)
        entry = manifest["positive_sources"][0]
        self.assertEqual(entry["source"], "test-center/test-contracts/welcome.compact")
        self.assertIn(entry["source"],
                      (inventory.ROOT / manifest["suite"]).read_text())
        self.assertIn(entry["source"],
                      (inventory.ROOT / entry["typescript_capture"]).read_text())
        fixture = json.loads((inventory.ROOT / entry["typescript_fixture"]).read_text())
        self.assertEqual(fixture["source"], entry["source"])
        self.assertEqual([case["present"] for case in fixture["cases"]], [False, True])
        self.assertIn(Path(entry["typescript_fixture"]).name,
                      (inventory.ROOT / entry["rust_test"]).read_text())
        self.assertEqual([(item["name"], item["proof"]) for item in entry["proof_circuits"]],
                         [("add_participant", True), ("add_organizer", True),
                          ("check_in", True), ("public_key", False)])
        self.assertEqual({item["name"] for item in
                          inventory.parse_source(inventory.ROOT / entry["source"],
                                                 inventory.ROOT)["declarations"]
                          if item["kind"] == "circuit" and item["visibility"] == "export"},
                         {item["name"] for item in entry["proof_circuits"]})

    def test_test_center_bboard_scope_tracks_recorded_exports(self):
        manifest = json.loads(inventory.TEST_CENTER_BBOARD_SOURCE_MANIFEST.read_text())
        self.assertEqual(source_scope.cohort_membership_failures(manifest), [])
        self.assertEqual(len(manifest["positive_sources"]), 1)
        entry = manifest["positive_sources"][0]
        self.assertEqual(entry["source"], "test-center/test-contracts/bboard.compact")
        self.assertIn(entry["source"],
                      (inventory.ROOT / entry["typescript_capture"]).read_text())
        self.assertEqual(json.loads((inventory.ROOT / entry["typescript_fixture"]).read_text())
                         ["source"], entry["source"])
        self.assertIn(Path(entry["typescript_fixture"]).name,
                      (inventory.ROOT / entry["rust_test"]).read_text())
        self.assertEqual([(item["name"], item["proof"]) for item in entry["proof_circuits"]],
                         [("post", True), ("take_down", True), ("public_key", False)])
        self.assertEqual(entry["expected_recording_gaps"], {})
        self.assertEqual(set(entry["expected_recorded_circuits"]), {"post", "take_down"})
        self.assertEqual({item["name"] for item in
                          inventory.parse_source(inventory.ROOT / entry["source"],
                                                 inventory.ROOT)["declarations"]
                          if item["kind"] == "circuit" and item["visibility"] == "export"},
                         {item["name"] for item in entry["proof_circuits"]})

    def test_pm19252_positive_scope_is_complete_and_excludes_rejection(self):
        scope = json.loads(inventory.POSITIVE_SOURCE_MANIFEST.read_text())
        positive = scope["positive_sources"]
        negative = scope["expected_rejections"]
        self.assertEqual((len(positive), len(negative)), (18, 1))
        all_paths = [entry["source"] for entry in positive + negative]
        self.assertEqual(len(all_paths), len(set(all_paths)))
        self.assertEqual(sorted(all_paths), sorted(path.relative_to(inventory.ROOT).as_posix()
                                                  for path in (inventory.ROOT / "examples/bugs/pm-19252").glob("*.compact")))
        scanned = {path.relative_to(inventory.ROOT).as_posix()
                   for path in inventory.source_paths(inventory.ROOT)}
        self.assertTrue({entry["source"] for entry in positive} <= scanned)
        self.assertTrue({entry["source"] for entry in negative}.isdisjoint(scanned))
        own_public_key = next(entry for entry in positive
                              if entry["source"].endswith("/example_ten.compact"))
        self.assertEqual(own_public_key["expected_rust"], "success")
        self.assertEqual(own_public_key["proof_circuits"],
                         [{"name": "test1", "pure": False, "proof": False}])
        suite = (inventory.ROOT / scope["suite"]).read_text()
        for entry in positive:
            name = Path(entry["source"]).name
            self.assertRegex(suite, re.escape(name) + r"(?:(?!const filePath)[\s\S])*?toBeSuccess")
            self.assertEqual(entry["expected_ts"], "success")
            self.assertEqual(entry["expected_rust"] in ("success", "rejection"), True)
            self.assertEqual(len(entry["proof_circuits"]),
                             len({circuit["name"] for circuit in entry["proof_circuits"]}))
            self.assertTrue(all(isinstance(circuit["proof"], bool)
                                and isinstance(circuit["pure"], bool)
                                for circuit in entry["proof_circuits"]))
            source = inventory.without_comments((inventory.ROOT / entry["source"]).read_text())
            omissions = list(inventory.PURE_DECLARATION.finditer(source))
            self.assertEqual(len(omissions), entry["pure_declarations"])
            self.assertEqual(sum(bool(match.group(1)) for match in omissions),
                             entry["exported_pure_declarations"])
        self.assertEqual(negative[0]["source"], "examples/bugs/pm-19252/example_fourteen.compact")
        self.assertRegex(suite, r"example_fourteen\.compact(?:(?!const filePath)[\s\S])*?toBeFailure")

    def test_original_coracle_and_micro_dao_have_exact_registered_cohorts(self):
        for manifest_path, exports, proof_count, recorded in [
            (inventory.TEST_CENTER_CORACLE_SOURCE_MANIFEST, 9, 4, ["guess", "concede", "withdraw"]),
            (inventory.TEST_CENTER_MICRO_DAO_SOURCE_MANIFEST, 11, 7, ["advance", "vote_reveal", "dao_voting_token", "set_topic", "cash_out", "buy_in", "vote_commit"]),
        ]:
            with self.subTest(manifest=manifest_path.name):
                self.assertIn(manifest_path, inventory.POSITIVE_SOURCE_MANIFESTS)
                manifest = json.loads(manifest_path.read_text())
                self.assertEqual(source_scope.cohort_membership_failures(manifest), [])
                entry, = manifest["positive_sources"]
                declarations = inventory.parse_source(inventory.ROOT / entry["source"],
                                                      inventory.ROOT)["declarations"]
                names = {item["name"] for item in declarations
                         if item["kind"] == "circuit" and item["visibility"] == "export"}
                self.assertEqual({item["name"] for item in entry["proof_circuits"]}, names)
                self.assertEqual(len(names), exports)
                proof_names = {item["name"] for item in entry["proof_circuits"] if item["proof"]}
                self.assertEqual(len(proof_names), proof_count)
                self.assertEqual(set(entry["expected_recording_gaps"]), proof_names - set(recorded))
                self.assertEqual(entry["expected_recorded_circuits"], recorded)

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

    def test_pure_circuits_preserve_visibility_signature_and_module(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/pure.compact"
            source.parent.mkdir(parents=True)
            source.write_text("""// export pure circuit fake(): [];
export { named };
pure circuit named(): Boolean { return true; }
module Inner {
  pure circuit hidden(): Boolean { return false; }
  export pure circuit visible(): Boolean { return true; }
}
""")
            rows = inventory.parse_source(source, root)["declarations"]
            self.assertEqual([(row["name"], row["kind"], row["visibility"], row["module_path"])
                              for row in rows],
                             [("named", "circuit", "export", []),
                              ("Inner", "module", "internal", []),
                              ("hidden", "circuit", "internal", ["Inner"]),
                              ("visible", "circuit", "module_export", ["Inner"])])
            self.assertEqual([row["declared_pure"] for row in rows], [True, False, True, True])
            self.assertEqual(rows[3]["signature"], "export pure circuit visible(): Boolean")

    def test_module_member_export_requires_top_level_reexport(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/module.compact"
            source.parent.mkdir(parents=True)
            source.write_text("""module M {
  export circuit promoted(): [] { }
  export pure circuit helper(): Boolean { return true; }
  export { helper };
}
import M;
export { promoted };
""")
            contract = inventory.parse_source(source, root)
            rows = [row for row in contract["declarations"] if row["kind"] == "circuit"]
            self.assertEqual(contract["named_exports"], ["promoted"])
            self.assertEqual([(row["name"], row["module_path"], row["visibility"])
                              for row in rows],
                             [("promoted", ["M"], "export"),
                              ("helper", ["M"], "module_export")])

    def test_module_only_export_is_not_unassessed_contract_circuit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/module.compact"
            source.parent.mkdir(parents=True)
            source.write_text("""module M {
  export pure circuit helper(): Boolean { return true; }
  export circuit promoted(): [] { }
}
import M;
export { promoted };
""")
            compiler = root / "compiler.py"
            compiler.write_text("""#!/usr/bin/env python3
import json, pathlib, sys
output = pathlib.Path(sys.argv[-1])
contract = output / "contract"
contract.mkdir(parents=True)
(contract / "rust-capabilities.json").write_text(json.dumps({"schema_version": 3, "circuits": [
  {"name": "promoted", "recorded": True, "observed_call": True,
   "proof_required": True, "recording_status": "available"}]}))
metadata = output / "compiler"
metadata.mkdir()
(metadata / "contract-info.json").write_text(json.dumps({"circuits": [
  {"name": "promoted", "pure": False, "proof": True}]}))
""")
            compiler.chmod(0o755)
            output = root / "receipt.json"
            result = subprocess.run([sys.executable, str(SCRIPT), "--root", str(root),
                                     "--compiler", str(compiler), "--require-full", "--output", str(output)],
                                    capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            receipt = json.loads(output.read_text())
            self.assertEqual(receipt["summary"]["exported_circuits"], 1)
            self.assertEqual(receipt["summary"]["module_exported_circuits"], 1)
            self.assertEqual(receipt["summary"]["unassessed_exported_circuits"], 0)
            self.assertEqual(receipt["summary"]["missing_compiler_proof_rows"], [])
            self.assertEqual([(row["name"], row["proof_required"])
                              for row in receipt["rows"] if row["kind"] == "circuit"],
                             [("helper", None), ("promoted", True)])

    def test_checked_module_fixtures_have_distinct_export_scopes(self):
        source_dir = inventory.ROOT / "examples/rust_backend"
        promoted = inventory.parse_source(source_dir / "module_boolean_constructor.compact",
                                          inventory.ROOT)["declarations"]
        self.assertEqual([(row["name"], row["visibility"])
                          for row in promoted if row["name"] == "bump_inner"],
                         [("bump_inner", "export")])
        for filename, members in (
            ("schnorr_attest_oracle.compact",
             {"schnorrVerify", "schnorrVerifyDigest", "schnorrChallengeDigest"}),
            ("struct_collision_oracle.compact",
             {"makeAlpha", "wrapAlpha", "makeBeta", "wrapBeta"}),
        ):
            rows = inventory.parse_source(source_dir / filename, inventory.ROOT)["declarations"]
            self.assertEqual({row["name"] for row in rows if row["visibility"] == "module_export"
                              and row["kind"] == "circuit"}, members)

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

    def test_baseline_detects_pure_circuit_rename(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/pure.compact"
            source.parent.mkdir(parents=True)
            source.write_text("export pure circuit first(): Boolean { return true; }\n")
            baseline = root / "baseline.json"
            output = root / "receipt.json"
            command = [sys.executable, str(SCRIPT), "--root", str(root), "--output", str(output)]
            self.assertEqual(subprocess.run([*command, "--write-baseline", str(baseline)],
                                            capture_output=True).returncode, 0)
            source.write_text("export pure circuit second(): Boolean { return true; }\n")
            result = subprocess.run([*command, "--baseline", str(baseline)], capture_output=True)
            self.assertEqual(result.returncode, 1, result.stderr)
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
compiler = contract.parent / "compiler"
compiler.mkdir()
(compiler / "contract-info.json").write_text(json.dumps({"circuits": [
  {"name": "available", "pure": False, "proof": True},
  {"name": "missing", "pure": False, "proof": True}]}))
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
compiler = contract.parent / "compiler"
compiler.mkdir()
(compiler / "contract-info.json").write_text(json.dumps({"circuits": [
  {"name": "private_only", "pure": False, "proof": False}]}))
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

    def test_pure_compiler_metadata_needs_no_recorded_api(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/pure.compact"
            source.parent.mkdir(parents=True)
            source.write_text("export pure circuit constant(): Boolean { return true; }\n")
            compiler = root / "compiler.py"
            compiler.write_text("""#!/usr/bin/env python3
import json, pathlib, sys
output = pathlib.Path(sys.argv[-1])
contract = output / "contract"
contract.mkdir(parents=True)
(contract / "rust-capabilities.json").write_text(json.dumps({"schema_version": 3, "circuits": []}))
metadata = output / "compiler"
metadata.mkdir()
(metadata / "contract-info.json").write_text(json.dumps({"circuits": [
  {"name": "constant", "pure": True, "proof": False}]}))
""")
            compiler.chmod(0o755)
            output = root / "receipt.json"
            result = subprocess.run([sys.executable, str(SCRIPT), "--root", str(root),
                                     "--compiler", str(compiler), "--require-full", "--output", str(output)],
                                    capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            receipt = json.loads(output.read_text())
            self.assertEqual(receipt["summary"]["declared_pure_circuits"], 1)
            self.assertEqual(receipt["summary"]["known_lexical_pure_omissions"], 0)
            self.assertEqual(receipt["summary"]["unassessed_exported_circuits"], 0)
            self.assertEqual(receipt["summary"]["nonproof"], 1)
            row = receipt["rows"][0]
            self.assertEqual((row["proof_required"], row["compiler_pure"],
                              row["rust_recorded"], row["rust_recording_status"]),
                             (False, True, None, "not_applicable"))

    def test_missing_pure_proof_metadata_is_reported_and_fails_full_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/pure.compact"
            source.parent.mkdir(parents=True)
            source.write_text("export pure circuit missing(): Boolean { return true; }\n")
            compiler = root / "compiler.py"
            compiler.write_text("""#!/usr/bin/env python3
import json, pathlib, sys
output = pathlib.Path(sys.argv[-1])
contract = output / "contract"
contract.mkdir(parents=True)
(contract / "rust-capabilities.json").write_text(json.dumps({"schema_version": 3, "circuits": []}))
metadata = output / "compiler"
metadata.mkdir()
(metadata / "contract-info.json").write_text(json.dumps({"circuits": []}))
""")
            compiler.chmod(0o755)
            output = root / "receipt.json"
            command = [sys.executable, str(SCRIPT), "--root", str(root),
                       "--compiler", str(compiler), "--output", str(output)]
            result = subprocess.run(command, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            receipt = json.loads(output.read_text())
            self.assertEqual(receipt["summary"]["missing_compiler_proof_rows"],
                             [{"source": "examples/rust_backend/pure.compact", "name": "missing",
                               "module_path": [], "declared_pure": True}])
            self.assertIsNone(receipt["rows"][0]["proof_required"])
            self.assertEqual(subprocess.run([*command, "--require-full"],
                                            capture_output=True).returncode, 1)

    def test_receipt_metadata_is_exact_and_optional(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "examples/rust_backend/sample.compact"
            source.parent.mkdir(parents=True)
            source.write_text("export circuit sample(): [];\n")
            (root / "Cargo.lock").write_text('''[[package]]\nname = "midnight-ledger"\nversion = "8.0.3"\nchecksum = "abc"\n''')
            ir = root / "tools/compact-rust-backend/src/ir.rs"
            ir.parent.mkdir(parents=True)
            ir.write_text("pub const SCHEMA_VERSION: u32 = 12;\n")
            runtime = root / "runtime-rs/src/lib.rs"
            runtime.parent.mkdir(parents=True)
            runtime.write_text("pub const RUST_RUNTIME_ABI: u32 = 37;\n")
            result = inventory.make_inventory(root, [], None)
            self.assertNotIn("receipt_metadata", result)
            metadata = inventory.receipt_metadata(root, None, result["contracts"])
            self.assertEqual(metadata["rust_ir_schema"], 12)
            self.assertEqual(metadata["rust_runtime_abi"], 37)
            self.assertEqual(metadata["upstream_packages"]["midnight-ledger"],
                             {"version": "8.0.3", "checksum": "abc"})
            self.assertEqual(metadata["source_manifest_sha256"],
                             inventory.receipt_metadata(root, None, result["contracts"])["source_manifest_sha256"])


if __name__ == "__main__":
    unittest.main()
