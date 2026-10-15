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

"""Native acceptance boundary with filesystem artifacts and fake compiler I/O."""

from dataclasses import FrozenInstanceError
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from target_checks import native_targets


class FakeCompiler:
    def __init__(self, base, *, fault=None, fail_at=None):
        self.base = base
        self.fault = fault
        self.fail_at = fail_at
        self.events = []
        self.failure = RuntimeError("injected boundary failure")

    def event(self, *event):
        self.events.append(event)
        if len(self.events) == self.fail_at:
            raise self.failure

    def run(self, *args):
        self.event("run", args)
        output = Path(args[-1])
        source = Path(args[-2])
        contract = output / "contract"
        contract.mkdir(parents=True, exist_ok=True)
        if (contract / "linked").is_symlink():
            if self.fault == "follow-symlink":
                (contract / "linked/keep.txt").unlink()
            if self.fault != "leave-symlink":
                (contract / "linked").unlink()
                (output / "keys").unlink()
        label = output.name
        if label in ("ts", "both", "legacy-both"):
            (contract / "index.js").write_text("typescript")
        if label == "ts":
            return
        (contract / "lib.rs").write_text("rust")
        (contract / "Cargo.toml").write_text("manifest")
        info = lambda line: {"file": source.name, "line": line, "column": 1}
        capabilities = {"schema_version": 3, "circuits": [
            {"name": name, "recorded": True, "observed_call": True,
             "proof_required": True, "recording_status": "available"}
            for name in ("increment", "read_round")]}
        ir = {"schema_version": 20,
              "ledger_fields": [{"id": "round", "source": info(18)}],
              "stateful_circuits": [{"source": info(20)}]}
        if label == "native-own-public-key":
            ir = {"witnesses": [], "stateful_circuits": [{"name": "test1", "actions": [
                {"kind": "native_witness_call", "builtin": "own_public_key"}]}]}
            capabilities = {"circuits": [{"name": "test1", "proof_required": False,
                "recorded": True, "observed_call": True, "recording_status": "not_applicable"}]}
            (contract / "lib.rs").write_text("pub fn test1<Private>( context.own_coin_public_key()?")
            if self.fault == "native-unrecorded":
                capabilities["circuits"][0]["recorded"] = False
            if self.fault == "native-unobserved":
                capabilities["circuits"][0]["observed_call"] = False
            if self.fault == "native-unavailable":
                capabilities["circuits"][0]["recording_unavailable"] = {
                    "ir_node": "StateAction::NativeWitnessCall"}
        if label == "native-own-public-key-value":
            witness = {"kind": "native_witness_call", "builtin": "own_public_key"}
            ir = {"schema_version": 20, "witnesses": [], "stateful_circuits": [
                {"name": "key", "return_value": {"value": witness}},
                {"name": "key_bytes", "return_value": {"value": {"kind": "struct_field", "value": witness}}}]}
            capabilities = {"circuits": [{"name": name, "proof_required": False,
                "recorded": True, "observed_call": True,
                "recording_status": "not_applicable"} for name in ("key", "key_bytes")]}
            (contract / "lib.rs").write_text("context.own_coin_public_key()? " * 2)
        source_kinds = {"source-pure": ("circuits", 18), "source-witness": ("witnesses", 20),
                        "source-constructor": ("constructor", 21), "source-alias": ("type_aliases", 27)}
        if label in source_kinds:
            key, line = source_kinds[label]
            owner = {"source": info(line)}
            ir = {"schema_version": 20, key: owner if key == "constructor" else [owner]}
            if self.fault == "source-range" and label == "source-pure":
                owner["source"]["line"] = 0
        if self.fault == "legacy-bytes" and label == "legacy-rust":
            (contract / "lib.rs").write_text("changed")
        (contract / "rust-capabilities.json").write_text(json.dumps(capabilities))
        (contract / "compact-rust-ir.json").write_text(json.dumps(ir))

    def manifest(self, output):
        self.event("manifest", output)

    def capture(self, args, **kwargs):
        self.event("capture", args, kwargs)
        label = Path(args[-1]).name
        message = "requires --rust" if label == "skip-ts-alone" else "cannot be combined"
        code = 1
        if self.fault == label + ":success":
            code = 0
        elif self.fault == label + ":message":
            message = "wrong diagnostic"
        elif self.fault == label + ":output":
            Path(args[-1]).mkdir()
        return subprocess.CompletedProcess(args, code, "", message)


def inputs(fake):
    return dict(root=Path("repository"), counter_source=Path("sources/counter.compact"),
                pure_source=Path("sources/pure.compact"), witness_cell_source=Path("sources/witness.compact"),
                constructor_map_source=Path("sources/constructor.compact"), alias_source=Path("sources/alias.compact"),
                pm_19252_source=Path("sources/native.compact"), native_key_value_source=Path("sources/value.compact"),
                run=fake.run, capture=fake.capture, check_manifest=fake.manifest)


class NativeTargetChecks(unittest.TestCase):
    def test_acceptance_order_source_binding_capture_options_and_frozen_outputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            fake = FakeCompiler(base)
            result = native_targets.check("fake-compiler", base, **inputs(fake))
            self.assertEqual(result, native_targets.NativeTargetOutputs(base / "legacy-rust", base / "pure"))
            with self.assertRaises(FrozenInstanceError):
                result.pure = base
            self.assertFalse(result.pure.exists())
            self.assertTrue((base / "protected/keep.txt").is_file())
            self.assertFalse((base / "rust/contract/linked").is_symlink())
            self.assertFalse((base / "rust/keys").is_symlink())
            sequence = [(event[0], Path(event[1][-1]).name if event[0] != "manifest" else event[1].name)
                        for event in fake.events]
            self.assertEqual(sequence, [
                ("run", "ts"), ("run", "rust"), ("run", "native-own-public-key"),
                ("run", "native-own-public-key-value"), ("run", "source-pure"),
                ("run", "source-witness"), ("run", "source-constructor"), ("run", "source-alias"),
                ("manifest", "rust"), ("run", "rust"), ("manifest", "rust"),
                ("run", "both"), ("manifest", "both"), ("run", "legacy-rust"),
                ("manifest", "legacy-rust"), ("run", "legacy-both"), ("manifest", "legacy-both"),
                ("capture", "mixed-rust"), ("capture", "mixed-skip-ts"), ("capture", "skip-ts-alone")])
            # The source-range loop must not rebind the counter source used afterwards.
            for event in fake.events[9:]:
                if event[0] in ("run", "capture"):
                    self.assertEqual(event[1][-2], "sources/counter.compact")
            for event, flags in zip(fake.events[-3:], [
                    ["--target", "rust", "--rust"], ["--skip-ts", "--target=ts"], ["--skip-ts"]]):
                self.assertEqual(event[1][1:-3], flags)
                self.assertEqual(event[2], {"cwd": Path("repository"), "capture_output": True, "text": True})

    def test_every_callback_failure_stops_at_its_boundary_and_preserves_exception(self):
        for stop in range(1, 21):
            with self.subTest(stop=stop), tempfile.TemporaryDirectory() as tmp:
                fake = FakeCompiler(Path(tmp), fail_at=stop)
                with self.assertRaises(RuntimeError) as error:
                    native_targets.check("fake-compiler", fake.base, **inputs(fake))
                self.assertIs(error.exception, fake.failure)
                self.assertEqual(len(fake.events), stop)

    def test_native_metadata_source_ranges_cleanup_and_legacy_bytes_are_enforced(self):
        for fault, last in [("native-unrecorded", "native-own-public-key"),
                            ("native-unobserved", "native-own-public-key"),
                            ("native-unavailable", "native-own-public-key"), ("source-range", "source-pure"),
                            ("follow-symlink", "rust"), ("leave-symlink", "rust"), ("legacy-bytes", "legacy-rust")]:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as tmp:
                fake = FakeCompiler(Path(tmp), fault=fault)
                with self.assertRaises(AssertionError):
                    native_targets.check("fake-compiler", fake.base, **inputs(fake))
                self.assertEqual(fake.events[-1][0], "run")
                self.assertEqual(Path(fake.events[-1][1][-1]).name, last)

    def test_each_invalid_option_requires_failure_diagnostic_and_absent_output(self):
        for label in ("mixed-rust", "mixed-skip-ts", "skip-ts-alone"):
            for defect in ("success", "message", "output"):
                with self.subTest(label=label, defect=defect), tempfile.TemporaryDirectory() as tmp:
                    fake = FakeCompiler(Path(tmp), fault=label + ":" + defect)
                    with self.assertRaises(AssertionError):
                        native_targets.check("fake-compiler", fake.base, **inputs(fake))
                    self.assertEqual(fake.events[-1][0], "capture")
                    self.assertEqual(Path(fake.events[-1][1][-1]).name, label)


if __name__ == "__main__":
    unittest.main()
