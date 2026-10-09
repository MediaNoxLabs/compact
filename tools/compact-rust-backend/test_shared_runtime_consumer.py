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

"""Shared-runtime consumer boundary checks; no compiler or Cargo is launched."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import check_compactc_target as cli
from target_checks import shared_runtime


ASSETS = Path(__file__).parent / "consumers/shared_runtime"
# Order is significant: each check runs before the next example is installed.
EXAMPLES = [
    "wrong_cell_value", "wrong_set_element", "wrong_vector_key",
    "nested_map_scalar_lookup", "wrong_list_element", "wrong_map_key",
    "wrong_map_value", "wrong_merkle_leaf", "recorded_merkle_index",
    "recorded_historic_index", "wrong_recorded_historic_leaf",
    "recorded_history_reset", "recorded_historic_tree_reset",
    "recorded_historic_known", "wrong_recorded_merkle_leaf",
    "plain_merkle_reset_history",
]
RUN_EXAMPLES = {
    "recorded_history_reset", "recorded_historic_tree_reset", "recorded_historic_known",
}
POSITIVE_EXAMPLES = RUN_EXAMPLES | {"recorded_merkle_index", "recorded_historic_index"}
DIAGNOSTICS = {
    "wrong_cell_value": "expected `bool`, found `u64`",
    "wrong_set_element": "error[E0308]: mismatched types; found `bool`",
    "wrong_vector_key": "error[E0308]: mismatched types; FixedVector",
    "nested_map_scalar_lookup": "no lookup on MapNode",
    "wrong_list_element": "error[E0308]: mismatched types; found `bool`",
    "wrong_map_key": "error[E0308]: mismatched types; expected `bool`",
    "wrong_map_value": "error[E0308]: mismatched types; found `bool`",
    "wrong_merkle_leaf": "error[E0308]: mismatched types; expected `BoundedUint<255>`",
    "wrong_recorded_historic_leaf": "error[E0308]: mismatched types; expected `BoundedUint<255>`",
    "wrong_recorded_merkle_leaf": "error[E0308]: mismatched types; expected `BoundedUint<255>`",
    "plain_merkle_reset_history": "error[E0599]; MerkleSlot<T, DEPTH, true>",
}


class ConsumerHarness:
    def __init__(self, root, base, *, environment=None, mutate_contract=None, outcomes=None):
        self.root, self.base = root, base
        self.environment = {} if environment is None else environment
        self.mutate_contract = mutate_contract
        self.outcomes = {} if outcomes is None else outcomes
        self.sources = [("counter", root / "counter.compact"),
                        ("field_add", root / "field add.compact")]
        self.events = []

    def run(self, *arguments, **kwargs):
        if arguments[0] == "compiler-under-test":
            self.events.append(("compile", arguments, kwargs))
            contract = Path(arguments[-1]) / "contract"
            contract.mkdir(parents=True)
            package = "package_" + contract.parent.name.removeprefix("shared-")
            (contract / "Cargo.toml").write_text(
                f'[package]\nname = "{package}"\n[dependencies]\n'
                f'midnight-compact-runtime = {{ path = {json.dumps(str(self.root / "runtime-rs"))} }}\n'
            )
            if self.mutate_contract:
                self.mutate_contract(contract)
            return
        self.process("run", arguments, kwargs)

    def execute(self, arguments, **kwargs):
        return self.process("execute", tuple(arguments), kwargs)

    def process(self, boundary, arguments, kwargs):
        consumer = kwargs["cwd"]
        installed = sorted(str(p.relative_to(consumer)) for p in consumer.rglob("*.rs"))
        self.events.append((boundary, arguments, dict(kwargs), installed))
        name = arguments[-1] if "--example" in arguments else "test"
        outcome = self.outcomes.get(name)
        if isinstance(outcome, Exception):
            raise outcome
        if outcome is None:
            outcome = (0, "") if name == "test" or name in POSITIVE_EXAMPLES else (1, DIAGNOSTICS[name])
        return subprocess.CompletedProcess(arguments, outcome[0], stdout="", stderr=outcome[1])

    def check_manifest(self, output, **kwargs):
        self.events.append(("manifest", output, kwargs))

    def check(self):
        shared_runtime.check(
            "compiler-under-test", self.base, self.root, self.sources,
            assets=ASSETS, run=self.run, execute=self.execute,
            check_manifest=self.check_manifest, environ=self.environment,
        )


class SharedRuntimeConsumerChecks(unittest.TestCase):
    def test_sources_manifest_commands_and_incremental_assets_keep_order(self):
        with tempfile.TemporaryDirectory(prefix="shared consumer ") as temporary:
            root = Path(temporary)
            harness = ConsumerHarness(root, root)
            harness.check()
            consumer = root / "shared-consumer"
            for index, (name, source) in enumerate(harness.sources):
                output = root / f"shared-{name}"
                self.assertEqual(harness.events[index * 2], (
                    "compile", ("compiler-under-test", "--target", "rust", "--rust-runtime-root",
                                str(root), "--skip-zk", str(source), str(output)), {},
                ))
                self.assertEqual(harness.events[index * 2 + 1], (
                    "manifest", output, {"require_zkir": name != "field_add"},
                ))
            manifest = tomllib.loads((consumer / "Cargo.toml").read_text())
            self.assertEqual(manifest["package"], {
                "name": "compactc-shared-runtime-smoke", "version": "0.1.0", "edition": "2024",
            })
            self.assertEqual(manifest["dependencies"], {
                **{f"package_{name}": {"path": str(root / f"shared-{name}/contract")}
                   for name, _ in harness.sources},
                "midnight-compact-runtime": {"path": str(root / "runtime-rs")},
            })
            process_events = harness.events[len(harness.sources) * 2:]
            environment = {"CARGO_TARGET_DIR": str(root / "target/compactc-consumer")}
            self.assertEqual(process_events[0], (
                "execute", ("cargo", "test", "--quiet"),
                {"cwd": consumer, "env": environment, "check": True}, ["tests/both.rs"],
            ))
            self.assertEqual(len(process_events), 17)
            for index, name in enumerate(EXAMPLES):
                kwargs = {"cwd": consumer, "env": environment}
                if name not in RUN_EXAMPLES:
                    kwargs.update(capture_output=True, text=True)
                self.assertEqual(process_events[index + 1], (
                    "run" if name in RUN_EXAMPLES else "execute",
                    ("cargo", "check", "--quiet", "--example", name), kwargs,
                    sorted(["tests/both.rs"] + [f"examples/{item}.rs" for item in EXAMPLES[:index + 1]]),
                ))
            for asset in ASSETS.rglob("*.rs"):
                self.assertEqual((consumer / asset.relative_to(ASSETS)).read_bytes(), asset.read_bytes())
            self.assertEqual(harness.environment, {})

    def test_environment_is_copied_after_compilation_and_existing_target_is_retained(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            environment = {"CARGO_TARGET_DIR": "/chosen target", "OTHER": "preserved"}
            harness = ConsumerHarness(root, root, environment=environment,
                                      mutate_contract=lambda _: environment.update(AFTER_COMPILE="yes"))
            harness.check()
            self.assertEqual(environment, {
                "CARGO_TARGET_DIR": "/chosen target", "OTHER": "preserved", "AFTER_COMPILE": "yes",
            })
            for event in harness.events:
                if event[0] in {"run", "execute"}:
                    self.assertEqual(event[2]["env"], environment)
                    self.assertIsNot(event[2]["env"], environment)

    def test_runtime_path_and_no_bundled_runtime_are_required_before_cargo(self):
        def wrong_path(contract):
            manifest = contract / "Cargo.toml"
            manifest.write_text(manifest.read_text().replace("runtime-rs", "wrong-runtime"))

        for mutation in [wrong_path, lambda contract: (contract / "runtime-rs").mkdir()]:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                harness = ConsumerHarness(root, root, mutate_contract=mutation)
                with self.assertRaises(AssertionError):
                    harness.check()
                self.assertFalse((root / "shared-consumer").exists())
                self.assertEqual([event[0] for event in harness.events], ["compile", "manifest"])

    def test_wrong_rejection_diagnostic_or_unexpected_success_stops_later_examples(self):
        for outcome in [(0, ""), (1, "unrelated compiler failure")]:
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                harness = ConsumerHarness(root, root, outcomes={"wrong_set_element": outcome})
                with self.assertRaises(AssertionError):
                    harness.check()
                self.assertEqual(sorted(p.stem for p in (root / "shared-consumer/examples").iterdir()),
                                 ["wrong_cell_value", "wrong_set_element"])

    def test_supported_example_failure_stops_later_checks(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            harness = ConsumerHarness(root, root, outcomes={"recorded_merkle_index": (1, "index failed")})
            with self.assertRaisesRegex(AssertionError, "index failed"):
                harness.check()
            self.assertFalse((root / "shared-consumer/examples/recorded_historic_index.rs").exists())

    def test_process_exceptions_propagate_before_later_assets_are_installed(self):
        for step in ["test", "recorded_history_reset"]:
            with self.subTest(step=step), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                failure = subprocess.CalledProcessError(7, ["cargo", step])
                harness = ConsumerHarness(root, root, outcomes={step: failure})
                with self.assertRaises(subprocess.CalledProcessError) as caught:
                    harness.check()
                self.assertIs(caught.exception, failure)
                next_path = "examples" if step == "test" else "examples/recorded_historic_tree_reset.rs"
                self.assertFalse((root / "shared-consumer" / next_path).exists())

    def test_cli_supplies_all_ten_sources_and_existing_process_boundaries(self):
        with patch.object(shared_runtime, "check") as check:
            cli.check_shared_runtime_consumer("compiler", Path("consumer base"))
        check.assert_called_once_with(
            "compiler", Path("consumer base"), cli.ROOT,
            (
                ("counter", cli.SOURCE), ("field_add", cli.PURE_SOURCE),
                ("cell_boolean", cli.CELL_SOURCE), ("set_oracle", cli.SET_SOURCE),
                ("map_boolean_field", cli.MAP_BOOLEAN_SOURCE),
                ("nested_map_oracle", cli.NESTED_MAP_SOURCE), ("list_field", cli.LIST_SOURCE),
                ("merkle_tree_oracle", cli.MERKLE_SOURCE),
                ("hmt_insert_oracle", cli.HISTORIC_MERKLE_SOURCE),
                ("vector_key_adt", cli.VECTOR_KEY_SOURCE),
            ),
            assets=cli.ROOT / "tools/compact-rust-backend/consumers/shared_runtime",
            run=cli.run, execute=subprocess.run, check_manifest=cli.check_manifest, environ=os.environ,
        )


if __name__ == "__main__":
    unittest.main()
