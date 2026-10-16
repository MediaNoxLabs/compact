#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest

import source_consumer_gate as consumers


class ConsumerBindingTests(unittest.TestCase):
    def test_cargo_identity_selects_actual_output_and_refuses_ambiguity(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            messages = root / "messages.jsonl"
            metadata = {"packages": [{"id": "workspace#consumer", "name": "consumer"}]}
            row = {"reason": "build-script-executed", "package_id": "workspace#consumer",
                   "out_dir": str(root / "actual")}
            unrelated = dict(row, package_id="registry#dependency")
            messages.write_text("\n".join(map(json.dumps, [unrelated, row])) + "\n")
            self.assertEqual(consumers.cargo_output_directories(messages, metadata),
                             {"consumer": (root / "actual").resolve()})
            messages.write_text("\n".join(map(json.dumps, [row, dict(row, out_dir=str(root / "other"))])))
            with self.assertRaisesRegex(ValueError, "ambiguous"):
                consumers.cargo_output_directories(messages, metadata)

    def test_missing_build_output_or_symlink_cannot_establish_a_consumer(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / "actual"
            directory.mkdir()
            registration = SimpleNamespace(package="consumer", generated_artifact="contract.rs")
            with self.assertRaisesRegex(ValueError, "missing Cargo"):
                consumers.consumer_artifact(registration, {})
            outside = root / "outside.rs"
            outside.write_text("outside")
            (directory / "contract.rs").symlink_to(outside)
            with self.assertRaisesRegex(ValueError, "escaped"):
                consumers.consumer_artifact(registration, {"consumer": directory})

    def test_compiled_artifact_must_match_fresh_emission_not_retained_input(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            generated, original, formatted, expected = [root / name for name in
                                                        ("fresh.rs", "actual.rs", "formatted.rs", "host.rs")]
            generated.write_text("crate::Ledger")
            expected.write_bytes(consumers.place_in_host(generated.read_bytes(), "nested"))
            original.write_bytes(expected.read_bytes())
            formatted.write_bytes(expected.read_bytes())
            row = SimpleNamespace(source=root / "contract.compact", package="consumer",
                                  consumer=root / "fixture.rs", namespace="nested")
            receipt = consumers.compare_consumer_artifact(row, generated, formatted, original, expected)
            self.assertEqual(receipt["artifact_sha256"], consumers.sha256(original))
            self.assertEqual(receipt["generated_sha256"], consumers.sha256(generated))
            formatted.write_text("a stale retained artifact")
            with self.assertRaisesRegex(ValueError, "differs from compiled consumer"):
                consumers.compare_consumer_artifact(row, generated, formatted, original, expected)

    def test_input_drift_and_conflicting_authority_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "build.rs"
            path.write_text("before")
            pin = consumers.sha256(path)
            row = SimpleNamespace(input_fingerprints=((path, pin),))
            self.assertEqual(consumers.check_input_fingerprints([row]), [{"path": str(path), "sha256": pin}])
            other = SimpleNamespace(input_fingerprints=((path, "0" * 64),))
            with self.assertRaisesRegex(ValueError, "conflicting"):
                consumers.check_input_fingerprints([row, other])
            path.write_text("after")
            with self.assertRaisesRegex(ValueError, "changed"):
                consumers.check_input_fingerprints([row])

    def test_only_declared_host_placement_changes_generated_bytes(self):
        source = b"crate::Ledger; other::Ledger;"
        self.assertEqual(consumers.place_in_host(source, None), source)
        self.assertEqual(consumers.place_in_host(source, "nested"),
                         b"crate::nested::Ledger; other::Ledger;")

    def test_compiled_integration_owner_requires_actual_include_dependency(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            owner = root / "tests/nested.rs"
            owner.parent.mkdir()
            owner.write_text('include!(concat!(env!("OUT_DIR"), "/nested.rs"));')
            out = root / "build/out"
            out.mkdir(parents=True)
            generated = out / "nested.rs"
            generated.write_text("actual")
            executable = root / "nested-123"
            executable.write_text("compiled test executable")
            depfile = executable.with_suffix(".d")
            depfile.write_text(f'{executable}: {owner} {generated}\n')
            row = SimpleNamespace(source=root / "contract.compact", package="consumer",
                                  consumer=owner, generated_artifact="nested.rs")
            metadata = {"packages": [{"id": "workspace#consumer", "name": "consumer"}]}
            messages = root / "test-messages.jsonl"
            records = [{"reason": "build-script-executed", "package_id": "workspace#consumer", "out_dir": str(out)},
                       {"reason": "compiler-artifact", "package_id": "workspace#consumer",
                        "target": {"src_path": str(owner), "kind": ["test"], "name": "nested"},
                        "profile": {"test": True}, "filenames": [str(executable)], "executable": str(executable)}]
            messages.write_text("\n".join(map(json.dumps, records)) + "\nrunning 1 test\ntest succeeds ... ok\n")
            binding, = consumers.consumed_artifact_bindings(row, messages, metadata, root)
            self.assertEqual(binding["artifact_sha256"], consumers.sha256(executable))
            self.assertEqual(binding["generated_sha256"], consumers.sha256(generated))
            # An output directory alone does not prove the test included it.
            depfile.write_text(f'{executable}: {owner}\n')
            with self.assertRaisesRegex(ValueError, 'no compiled target consumes'):
                consumers.consumed_artifact_bindings(row, messages, metadata, root)

    def test_dependency_paths_preserve_spaces_and_do_not_match_other_artifacts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            depfile = root / "compiled.d"
            depfile.write_text('target: tests/owner\\ name.rs build/out/contract.rs \\\n other.rs\n\n')
            self.assertEqual(consumers.depfile_inputs(depfile, root),
                {root / "tests/owner name.rs", root / "build/out/contract.rs", root / "other.rs"})
            depfile.write_text('not a rustc dependency rule\n')
            with self.assertRaisesRegex(ValueError, 'invalid rustc'):
                consumers.depfile_inputs(depfile, root)


if __name__ == "__main__":
    unittest.main()
