#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
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

"""Public Python/sidecar checks, with explicit genuine-artifact skip accounting."""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parent
# Parent methods, not skip events: one method contains two skipped subtests.
MANUAL_GRAPH_METHODS = frozenset(
    "test_guarded_point_inputs.RetainedGraphInputTests." + name for name in (
        "test_alias_retarget_cannot_hide_selected_graph_mutation",
        "test_context_cleanup_preserves_original_failure_and_global_selection",
        "test_context_cleanup_rejects_graph_or_source_mutation",
        "test_current_nested_source_is_authenticated",
        "test_explicit_cli_selection_keeps_its_original_unpinned_input_flow",
        "test_graph_content_and_deletion_are_live",
        "test_matching_bytes_outside_graph_root_do_not_authorize_escape",
        "test_original_authority_files_cannot_be_replaced",
        "test_positive_receipt_has_original_identities_and_retained_scope",
    )) | frozenset(
    "test_guarded_point_zkir.GuardedPointGraphTests." + name for name in (
        "test_active_operand_cannot_change",
        "test_fresh_nested_graph_and_all_four_guard_valuations",
        "test_inner_guard_alone_cannot_replace_full_nested_guard",
        "test_masked_point_cannot_replace_raw_transcript_value",
        "test_missing_fix_is_not_accepted_as_unchanged_graph",
        "test_original_public_input_guard_cannot_change",
        "test_per_coordinate_guards_must_agree",
        "test_scalar_is_not_part_of_point_normalization",
        "test_unknown_opcode_fails_closed",
        "test_zero_y_is_not_a_valid_inactive_identity",
    ))


def cases(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from cases(item)
        else:
            yield item


def run_suite(suite, *, expected_skips=MANUAL_GRAPH_METHODS, stream=None):
    selected = list(cases(suite))
    ids = [test.id() for test in selected]
    if not ids or len(ids) != len(set(ids)):
        raise ValueError("public discovery is empty or has duplicate case IDs")
    result = unittest.TextTestRunner(stream=stream, verbosity=1).run(suite)
    skipped = {test.id().split(" (", 1)[0] for test, _ in result.skipped}
    accounting = result.testsRun == len(ids) and skipped == set(expected_skips)
    return {
        "format": "compact-public-python-checks/v1",
        "selected_methods": len(ids), "methods_run": result.testsRun,
        "fully_exercised_methods": result.testsRun - len(skipped),
        "skip_events": len(result.skipped), "skipped_parent_methods": sorted(skipped),
        "unexpected_skips": sorted(skipped - expected_skips),
        "missing_expected_skips": sorted(expected_skips - skipped),
        "failures": len(result.failures), "errors": len(result.errors),
        "successful": result.wasSuccessful() and accounting,
        "scope": "public unit controls; manual genuine-graph checks remain separate; no proofs",
    }


def discover():
    loader = unittest.TestLoader()
    suite = loader.discover(str(ROOT), pattern="test_*.py", top_level_dir=str(ROOT))
    path = ROOT / "tests/performance/test_historical_shards.py"
    spec = importlib.util.spec_from_file_location("public_historical_shard_tests", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    suite.addTests(loader.loadTestsFromModule(module))
    return suite


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args(argv)
    for name in ("COMPACT_GUARDED_POINT_ROOT", "COMPACT_CONFORMANCE_BASELINE_ROOT"):
        if name in os.environ:
            parser.error("public checks require " + name + " to be unset")
    receipt = run_suite(discover())
    if args.receipt:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
    return 0 if receipt["successful"] else 1


if __name__ == "__main__":
    sys.exit(main())
