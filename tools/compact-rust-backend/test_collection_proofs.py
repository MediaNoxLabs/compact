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

"""Collection preparation command, artifact, and failure boundaries."""

from dataclasses import FrozenInstanceError
from pathlib import Path
import unittest
from unittest.mock import patch

from target_checks import collection_proofs


CASES = (
    ("set", "set-proof", ("add", "contains", "remove", "seen_size", "seen_is_empty",
                          "add_field", "contains_field", "reset_fields", "choose")),
    ("set_oracle", "set-oracle-proof", ("check",)),
    ("map", "map-proof", ("put", "put_pair", "put_default", "has", "get", "remove_key",
                          "table_size", "table_is_empty", "reset_table")),
    ("constructor_map", "constructor-map-proof", ("table_size", "history_size",
                                                  "get_true", "get_false_history")),
    ("list", "list-proof", ("item_count", "items_empty", "first_item", "prepend",
                            "drop_first", "clear_items")),
    ("list_shapes", "list-shapes-proof", ("push_flag", "push_count", "push_tag",
                                        "push_choice", "push_packet", "first_packet",
                                        "first_choice")),
)


class CollectionProofPreparation(unittest.TestCase):
    def setUp(self):
        self.base = Path("temporary artifacts")
        self.sources = {f"{field}_source": Path("source contracts") / f"{field}.compact"
                        for field, _, _ in CASES}

    def expected_events(self):
        events = []
        for field, directory, circuits in CASES:
            output = self.base / directory
            events.append(("run", ("compactc-under-test", "--target", "rust",
                                   str(self.sources[f"{field}_source"]), str(output))))
            events.append(("manifest", output))
            if field == "map":
                events.append(("consumer", output, self.base))
            for circuit in circuits:
                for directory, extension in (("keys", "prover"), ("keys", "verifier"),
                                             ("zkir", "zkir"), ("zkir", "bzkir")):
                    events.append(("file", output / directory / f"{circuit}.{extension}"))
        return events

    def invoke(self, events, *, fail_at=None, failure=None):
        def record(event):
            events.append(event)
            if len(events) - 1 == fail_at:
                if event[0] == "file":
                    return False
                raise failure
            return True

        def run(*arguments):
            record(("run", arguments))

        def manifest(output):
            record(("manifest", output))

        def consumer(output, base):
            record(("consumer", output, base))

        with patch.object(Path, "is_file", lambda path: record(("file", path))):
            return collection_proofs.prepare(
                "compactc-under-test", self.base, **self.sources,
                run=run, check_manifest=manifest,
                check_observed_map_call_consumer=consumer,
            )

    def test_exact_compilation_manifest_consumer_and_file_order(self):
        events = []
        artifacts = self.invoke(events)
        self.assertEqual(events, self.expected_events())
        for field, directory, _ in CASES:
            self.assertEqual(getattr(artifacts, field), self.base / directory)
        with self.assertRaises(FrozenInstanceError):
            artifacts.set = Path("changed")

    def test_every_failed_boundary_stops_later_preparation(self):
        expected = self.expected_events()
        for index, event in enumerate(expected):
            with self.subTest(index=index, event=event):
                events = []
                failure = RuntimeError(f"failed {event[0]}")
                expected_error = AssertionError if event[0] == "file" else RuntimeError
                with self.assertRaises(expected_error) as error:
                    self.invoke(events, fail_at=index, failure=failure)
                self.assertEqual(events, expected[:index + 1])
                if event[0] != "file":
                    self.assertIs(error.exception, failure)

    def test_map_consumer_precedes_map_artifacts_and_next_compilation(self):
        events = []
        self.invoke(events)
        consumer = events.index(("consumer", self.base / "map-proof", self.base))
        self.assertEqual(events[consumer - 1], ("manifest", self.base / "map-proof"))
        self.assertEqual(events[consumer + 1],
                         ("file", self.base / "map-proof/keys/put.prover"))
        self.assertFalse(any(event[0] == "run" and "constructor-list-proof" in event[-1][-1]
                             for event in events))


if __name__ == "__main__":
    unittest.main()
