# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

"""Reject incomplete qualification results without changing original law tests."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

BACKEND = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("historical_law_shards", BACKEND / "historical_law_shards.py")
shards = importlib.util.module_from_spec(spec)
spec.loader.exec_module(shards)


def accepted():
    groups = {name: [] for name in ("ordinary", "earlier_inventory", "latest_inventory", "current_measurement")}
    groups["earlier_inventory"] = ["module.Case.first", "module.Case.second"]
    ids = groups["earlier_inventory"]
    job = {"modules": ["module"], "group_case_ids": groups, "case_ids": ids}
    report = {"passed": True, "status": "completed", "failures": 0, "errors": 0,
              "skipped": [], "not_run_case_ids": {}, "actual_case_ids_match_plan": True,
              "planned_case_ids": groups, "planned_case_count": 2, "tests_run": 2,
              "modules": ["module"], "case_timings": [{"id": identity, "seconds": 1} for identity in ids],
              "groups": {name: {"actual_case_ids": values, "tests_run": len(values),
                                "failures": [], "errors": [], "skipped": []} for name, values in groups.items()}}
    return report, job, ids


class AccountingTests(unittest.TestCase):
    def refused(self, report, job, ids):
        with self.assertRaises((ValueError, KeyError, TypeError)):
            shards.join_reports([report], ids, [job])

    def test_complete_ordered_assignment_is_accepted(self):
        report, job, ids = accepted()
        self.assertEqual(shards.join_reports([report], ids, [job]), ids)

    def test_missing_required_outcomes_are_refused(self):
        report, job, ids = accepted()
        for key in report:
            with self.subTest(key=key):
                damaged = copy.deepcopy(report)
                del damaged[key]
                self.refused(damaged, job, ids)

    def test_success_and_counts_require_exact_types(self):
        report, job, ids = accepted()
        for key, value in [("passed", 1), ("failures", False), ("errors", False),
                           ("tests_run", True), ("planned_case_count", True),
                           ("actual_case_ids_match_plan", 1)]:
            with self.subTest(key=key):
                damaged = copy.deepcopy(report)
                damaged[key] = value
                self.refused(damaged, job, ids)

    def test_missing_extra_and_duplicate_executions_are_refused(self):
        report, job, ids = accepted()
        for reports, planned, jobs in [([], ids, [job]), ([report], ids + ["unrun"], [job]),
                                      ([report], ids[:-1], [job]), ([report, report], ids, [job, job])]:
            with self.subTest(planned=planned, workers=len(reports)):
                with self.assertRaises(ValueError):
                    shards.join_reports(reports, planned, jobs)

    def test_group_order_and_assignment_cannot_be_changed(self):
        report, job, ids = accepted()
        for change in ("order", "module", "group"):
            damaged = copy.deepcopy(report)
            if change == "order":
                damaged["groups"]["earlier_inventory"]["actual_case_ids"].reverse()
            elif change == "module":
                damaged["modules"] = ["other"]
            else:
                damaged["groups"]["ordinary"]["actual_case_ids"] = [ids[0]]
            self.refused(damaged, job, ids)

    def test_skipped_failed_or_partial_worker_is_not_success(self):
        report, job, ids = accepted()
        for key, value in [("skipped", [{"case": ids[0]}]), ("errors", 1), ("failures", 1),
                           ("status", "failed-partial"), ("not_run_case_ids", {"earlier_inventory": ids})]:
            damaged = copy.deepcopy(report)
            damaged[key] = value
            self.refused(damaged, job, ids)

    def test_nested_or_missing_timing_rows_are_refused(self):
        report, job, ids = accepted()
        for rows in ([], report["case_timings"] + [{"id": "nested.synthetic", "seconds": 1}]):
            damaged = copy.deepcopy(report)
            damaged["case_timings"] = rows
            self.refused(damaged, job, ids)

    def test_original_driver_bytes_are_mandatory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in shards.ORIGINAL_RUNNER:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"unreviewed source")
            with self.assertRaisesRegex(ValueError, "original runner source changed"):
                shards.authenticate_runner(root)


if __name__ == "__main__":
    unittest.main()
