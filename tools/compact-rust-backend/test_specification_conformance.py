#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess

import specification_conformance as gate


def grammar(name="Lsrc"):
    return ["define-language", name, ["entry", "Program"],
            ["terminals", ["symbol", ["name", "var"]]],
            ["Program", ["p"], ["program", "expr"]],
            ["Expression", ["expr"], ["if", "a", "b", "c"], ["quote", "datum"]]]


class GrammarTests(unittest.TestCase):
    def test_duplicate_json_keys_are_refused(self):
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            gate.load_json('{"family": "unknown", "family": "supported"}')


    def test_normalization_is_deterministic_for_declaration_order(self):
        raw = grammar()
        reordered = copy.deepcopy(raw)
        reordered[4:] = reversed(reordered[4:])
        reordered[4][2:] = reversed(reordered[4][2:])
        reordered[3][1][1].reverse()
        self.assertEqual(gate.normalize_grammar(raw), gate.normalize_grammar(reordered))

    def test_argument_order_and_unknown_constructor_are_not_erased(self):
        original = gate.normalize_grammar(grammar())
        changed = grammar()
        changed[-1][2] = ["if", "a", "c", "b"]
        self.assertNotEqual(original, gate.normalize_grammar(changed))
        changed[-1].append(["future-operator", "expr"])
        self.assertEqual(len(gate.normalize_grammar(changed)["requirements"]),
                         len(original["requirements"]) + 1)

    def test_duplicate_owner_or_production_is_refused(self):
        for raw in (grammar() + [grammar()[-1]], grammar()):
            if len(raw) == len(grammar()):
                raw[-1].append(raw[-1][-1])
            with self.assertRaisesRegex(ValueError, "duplicate"):
                gate.normalize_grammar(raw)

    def test_missing_or_dangling_entry_is_refused(self):
        missing = grammar()
        missing.pop(2)
        dangling = grammar()
        dangling[2][1] = "Absent"
        for raw in [missing, dangling]:
            with self.assertRaisesRegex(ValueError, "entry"):
                gate.normalize_grammar(raw)


class GateTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        (self.root / "grammar.ss").write_text("real source identity\n")
        (self.root / "reference.mdx").write_text("# Reference\nnormative semantics\n")
        self.inventory = {"format_version": 1, "compiler_version": "2.0", "language_version": "2.0",
                          "sources": {"grammar.ss": gate.file_hash(self.root / "grammar.ss")},
                          "grammars": {name: gate.normalize_grammar(grammar(name)) for name in gate.GRAMMARS}}
        artifact = [["Compiler version", "1.0"], ["Language version", "1.0"], *grammar()]
        (self.root / "Lsrc.json").write_text(json.dumps(artifact))
        (self.root / "Lsrc.agda").write_text("-- *** Compiler version: 1.0\n-- *** Language version: 1.0\n")
        self.baseline = {"format_version": 1, "grammar_inventory": copy.deepcopy(self.inventory),
                         "sources": {"reference.mdx": gate.file_hash(self.root / "reference.mdx")},
                         "semantic_families": [{"id": "conditional", "status": "unknown"}],
                         "grammar_classifications": {i: {"family": "conditional"}
                                                     for i in gate.inventory_ids(self.inventory)},
                         "formal_artifacts": [{"path": name, "sha256": gate.file_hash(self.root / name),
                                               "status": "known-stale"} for name in ("Lsrc.json", "Lsrc.agda")]}

        self.baseline["evidence"] = []
        self.baseline["requirements"] = []
        self.baseline["reference_headings"] = [dict(x, family="conditional", source={
            "path": "reference.mdx", "sha256": gate.file_hash(self.root / "reference.mdx")})
            for x in gate.reference_headings((self.root / "reference.mdx").read_text())]

    def run_gate(self):
        return gate.check(self.root, self.inventory, self.baseline)

    def test_exact_mapping_pass_keeps_formal_debt_and_unknown_semantics(self):
        result = self.run_gate()
        self.assertEqual(result["status"], "passed")
        self.assertTrue(all(x["current_disposition"] == "known formal debt" for x in result["formal_artifacts"]))
        self.assertIn("Unknown support stays unknown", result["qualification"])
        self.assertEqual(self.baseline["semantic_families"][0]["status"], "unknown")

    def test_added_constructor_requires_new_classification(self):
        raw = grammar()
        raw[-1].append(["future", "expr"])
        self.inventory["grammars"]["Lsrc"] = gate.normalize_grammar(raw)
        errors = self.run_gate()["errors"]
        self.assertTrue(next(x for x in errors if x["kind"] == "grammar-requirements-drift")["added"])
        self.assertTrue(next(x for x in errors if x["kind"] == "classification-drift")["missing"])

    def test_changed_signature_and_removed_production_fail(self):
        for mutation in ("change", "remove"):
            raw = grammar()
            if mutation == "change":
                raw[-1][2].append("extra-argument")
            else:
                raw[-1].pop()
            self.inventory["grammars"]["Lsrc"] = gate.normalize_grammar(raw)
            result = self.run_gate()
            self.assertEqual(result["status"], "failed")
            self.assertTrue(next(x for x in result["errors"] if x["kind"] == "grammar-requirements-drift")["removed"])

    def test_missing_and_orphan_classifications_fail(self):
        identifier = next(iter(self.baseline["grammar_classifications"]))
        del self.baseline["grammar_classifications"][identifier]
        self.baseline["grammar_classifications"]["imaginary"] = {"family": "conditional"}
        error = next(x for x in self.run_gate()["errors"] if x["kind"] == "classification-drift")
        self.assertEqual(error["missing"], [identifier])
        self.assertEqual(error["orphaned"], ["imaginary"])

    def test_unknown_family_is_not_a_classification(self):
        self.baseline["grammar_classifications"][next(iter(self.baseline["grammar_classifications"]))] = {"family": "typo"}
        self.assertEqual(self.run_gate()["errors"][0]["kind"], "unknown-family")

    def test_source_and_normative_reference_drift_fail(self):
        for name in ("grammar.ss", "reference.mdx"):
            (self.root / name).write_text("changed content\n")
        errors = self.run_gate()["errors"]
        self.assertEqual({x["path"] for x in errors if x["kind"] == "source-drift"},
                         {"grammar.ss", "reference.mdx"})

    def test_conflicting_baseline_source_cannot_be_overwritten(self):
        self.baseline["sources"]["grammar.ss"] = "0" * 64
        self.assertIn({"kind": "conflicting-source-identity", "path": "grammar.ss"},
                      self.run_gate()["errors"])

    def test_historical_execution_hash_does_not_replace_current_source_guard(self):
        self.baseline["evidence"] = [{
            "id": "historical-observation",
            "recorded_source_hashes": {"reference.mdx": "0" * 64},
            "current_test_source": {
                "path": "reference.mdx", "line": 1,
                "sha256": gate.file_hash(self.root / "reference.mdx"),
            },
        }]
        self.assertEqual(self.run_gate()["errors"], [])
        (self.root / "reference.mdx").write_text("# Reference\nchanged semantics\n")
        errors = self.run_gate()["errors"]
        self.assertIn({"kind": "source-drift", "path": "reference.mdx"}, errors)
        self.assertIn({
            "kind": "anchor-source-drift",
            "location": "baseline.evidence[0].current_test_source",
            "path": "reference.mdx",
        }, errors)
        self.assertEqual(self.baseline["evidence"][0]["recorded_source_hashes"],
                         {"reference.mdx": "0" * 64})

    def test_baseline_version_annotation_must_match(self):
        self.baseline["baseline"] = {"compiler_version": "wrong"}
        self.assertIn({"kind": "baseline-version-drift", "field": "compiler_version"},
                      self.run_gate()["errors"])

    def test_cached_cli_checks_sources_and_labels_no_fresh_extraction(self):
        baseline = self.root / "baseline.json"
        report = self.root / "report.json"
        baseline.write_text(json.dumps(self.baseline))
        argv = ["specification_conformance.py", "--repo-root", str(self.root),
                "--baseline", str(baseline), "--report", str(report)]
        with patch("sys.argv", argv):
            self.assertEqual(gate.main(), 0)
        self.assertEqual(json.loads(report.read_text())["evidence_mode"],
                         "cached source-bound baseline check; no fresh extraction")
        (self.root / "grammar.ss").write_text("changed")
        with patch("sys.argv", argv):
            self.assertEqual(gate.main(), 1)

    def test_dangling_or_changed_repository_anchor_is_not_accepted(self):
        self.baseline["semantic_families"][0]["normative_sources"] = [
            {"path": "reference.mdx", "sha256": "0" * 64, "line": 999}]
        kinds = {x["kind"] for x in self.run_gate()["errors"]}
        self.assertIn("anchor-source-drift", kinds)
        self.assertIn("invalid-anchor-line", kinds)
        self.baseline["semantic_families"][0]["normative_sources"][0]["path"] = "missing.mdx"
        with self.assertRaises(FileNotFoundError):
            self.run_gate()

    def test_dangling_evidence_and_duplicate_semantic_ids_fail(self):
        self.baseline["evidence"] = [{"id": "E1"}, {"id": "E1"}]
        self.baseline["requirements"] = [{"id": "B1", "evidence_ids": ["missing"]}, {"id": "B1"}]
        errors = self.run_gate()["errors"]
        self.assertEqual(sum(x["kind"] == "duplicate-semantic-id" for x in errors), 2)
        self.assertIn({"kind": "dangling-evidence-reference", "id": "B1"}, errors)

    def test_reference_heading_removal_is_detected_and_fences_ignored(self):
        text = "# Title\n```text\n# Not a heading\n```\n## Detail\nBody\n"
        (self.root / "reference.mdx").write_text(text)
        sha = gate.file_hash(self.root / "reference.mdx")
        self.baseline["sources"]["reference.mdx"] = sha
        self.baseline["reference_headings"] = [dict(x, family="conditional", source={
            "path": "reference.mdx", "sha256": sha}) for x in gate.reference_headings(text)]
        self.assertEqual(len(self.baseline["reference_headings"]), 2)
        self.assertEqual(self.run_gate()["status"], "passed")
        self.baseline["reference_headings"].pop()
        self.assertIn({"kind": "reference-heading-drift"}, self.run_gate()["errors"])

    def test_affirmative_proof_or_unqualified_execution_status_is_refused(self):
        support = {key: "unknown" for key in gate.SUPPORT_VALUES}
        support["native"] = "selected-cases-observed"
        support["executed_proof"] = True
        self.baseline["semantic_families"][0]["support"] = support
        errors = self.run_gate()["errors"]
        self.assertIn({"kind": "unsupported-proof-claim", "id": "conditional"}, errors)
        self.assertIn({"kind": "observed-status-without-evidence", "id": "conditional"}, errors)
        support["native"] = "all-supported"
        self.assertIn({"kind": "invalid-support-status", "id": "conditional", "axis": "native"},
                      self.run_gate()["errors"])

    def test_missing_semantic_collections_are_refused(self):
        for key in ("evidence", "requirements", "reference_headings"):
            value = self.baseline.pop(key)
            with self.assertRaisesRegex(ValueError, "missing or invalid semantic collection"):
                self.run_gate()
            self.baseline[key] = value

    def test_compatibility_annotations_must_match_bound_metadata(self):
        path = "tools/compact-rust-backend/src/compatibility.json"
        target = self.root / path
        target.parent.mkdir(parents=True)
        target.write_text(json.dumps({"ir_schema": 20, "runtime_abi": 50, "ledger_version": "ledger-8.0.3"}))
        self.baseline["sources"][path] = gate.file_hash(target)
        self.baseline["baseline"] = {"private_ir_version": 20, "runtime_abi": 50, "native_ledger_version": "8.0.3"}
        self.assertEqual(self.run_gate()["status"], "passed")
        self.baseline["baseline"]["runtime_abi"] = 999
        self.assertIn({"kind": "compatibility-version-drift", "field": "runtime_abi"},
                      self.run_gate()["errors"])

    def test_version_change_is_reported(self):
        self.inventory["language_version"] = "3.0"
        self.assertIn({"kind": "inventory-drift", "field": "language_version"}, self.run_gate()["errors"])

    def test_changed_formal_file_requires_review(self):
        (self.root / "Lsrc.agda").write_text("different artifact")
        with self.assertRaisesRegex(ValueError, "formal artifact changed"):
            self.run_gate()

    def test_formal_staleness_cannot_be_silently_accepted(self):
        self.baseline["formal_artifacts"][0]["status"] = "current"
        with self.assertRaisesRegex(ValueError, "unacknowledged formal drift"):
            self.run_gate()

    def test_missing_formal_inventory_is_explicit_failure(self):
        self.baseline["formal_artifacts"] = []
        self.assertIn({"kind": "missing-formal-artifact-inventory"}, self.run_gate()["errors"])

    def test_inventory_tampering_is_refused(self):
        self.inventory["grammars"]["Lsrc"]["requirements"][0]["production"] = "tampered"
        with self.assertRaisesRegex(ValueError, "digest"):
            self.run_gate()

    def test_repo_source_path_cannot_escape(self):
        with self.assertRaisesRegex(ValueError, "escapes"):
            gate.checked_path(self.root, "../outside")

    def test_missing_live_extractor_never_falls_back(self):
        with self.assertRaisesRegex(ValueError, "Chez executable unavailable"):
            gate.extract(self.root, self.root / "missing-scheme", self.root)

    def test_failed_or_malformed_live_output_never_falls_back(self):
        scheme = self.root / "scheme"
        scheme.write_text("placeholder")
        scheme.chmod(0o700)
        (self.root / "nanopass.ss").write_text("placeholder")
        for source in gate.SOURCES:
            path = self.root / source
            path.parent.mkdir(exist_ok=True)
            path.write_text("placeholder")
        for result in (subprocess.CompletedProcess([], 1, "", "source import failed"),
                       subprocess.CompletedProcess([], 0, "not JSON", "")):
            with patch.object(gate.subprocess, "run", return_value=result):
                with self.assertRaisesRegex(ValueError, "extraction failed|invalid JSON"):
                    gate.extract(self.root, scheme, self.root)


class RepositoryBaselineTests(unittest.TestCase):
    def test_checked_in_baseline_matches_current_sources_and_mappings(self):
        baseline = gate.load_json(Path(gate.__file__).with_suffix(".json").read_text())
        result = gate.check(gate.ROOT, baseline["grammar_inventory"], baseline)
        self.assertEqual(result["errors"], [])
        self.assertEqual(result["status"], "passed")


if __name__ == "__main__":
    unittest.main()
