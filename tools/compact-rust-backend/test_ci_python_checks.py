# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Prove that the public gate cannot turn failed/skipped/incomplete work green."""
import io
import unittest

import ci_python_checks as gate


class PublicGateTests(unittest.TestCase):
    def execute(self, action, expected_skips=frozenset()):
        class Probe(unittest.TestCase):
            def runTest(self):
                action(self)
        return gate.run_suite(unittest.TestSuite([Probe()]),
                              expected_skips=expected_skips, stream=io.StringIO())

    def test_success_runs_the_body(self):
        called = []
        receipt = self.execute(lambda _: called.append("executed"))
        self.assertEqual(called, ["executed"])
        self.assertTrue(receipt["successful"])
        self.assertEqual(receipt["fully_exercised_methods"], 1)

    def test_failure_and_exception_fail_the_gate(self):
        def error(_):
            raise RuntimeError("actual body error")
        for action in (lambda case: case.fail("actual assertion"), error):
            with self.subTest(action=action):
                receipt = self.execute(action)
                self.assertFalse(receipt["successful"])
                self.assertEqual(receipt["failures"] + receipt["errors"], 1)

    def test_unknown_skip_cannot_pass(self):
        receipt = self.execute(lambda case: case.skipTest("unexpected"))
        self.assertFalse(receipt["successful"])
        self.assertEqual(len(receipt["unexpected_skips"]), 1)
        self.assertEqual(receipt["fully_exercised_methods"], 0)

    def test_missing_expected_skip_requires_a_policy_review(self):
        receipt = self.execute(lambda _: None, frozenset({"previously.manual.case"}))
        self.assertFalse(receipt["successful"])
        self.assertEqual(receipt["missing_expected_skips"], ["previously.manual.case"])

    def test_empty_and_duplicate_selection_refuse(self):
        class Probe(unittest.TestCase):
            def runTest(self):
                pass
        for suite in (unittest.TestSuite(), unittest.TestSuite([Probe(), Probe()])):
            with self.subTest(suite=suite), self.assertRaisesRegex(ValueError, "empty or.*duplicate"):
                gate.run_suite(suite, stream=io.StringIO())

    def test_subtest_skips_count_events_and_parent_separately(self):
        class Probe(unittest.TestCase):
            def runTest(self):
                for n in (1, 2):
                    with self.subTest(n=n):
                        self.skipTest("same parent, two events")
        case = Probe()
        receipt = gate.run_suite(unittest.TestSuite([case]),
                                 expected_skips=frozenset({case.id()}), stream=io.StringIO())
        self.assertTrue(receipt["successful"])
        self.assertEqual(receipt["skip_events"], 2)
        self.assertEqual(len(receipt["skipped_parent_methods"]), 1)
        self.assertEqual(receipt["fully_exercised_methods"], 0)

    def test_early_termination_cannot_qualify_unrun_cases(self):
        class Stopper(unittest.TestCase):
            def runTest(self):
                pass

            def run(self, result=None):
                result = super().run(result)
                result.stop()
                return result

        class Unrun(unittest.TestCase):
            def runTest(self):
                self.fail("must remain unrun after explicit stop")

        receipt = gate.run_suite(unittest.TestSuite([Stopper(), Unrun()]),
                                 expected_skips=frozenset(), stream=io.StringIO())
        self.assertEqual(receipt["selected_methods"], 2)
        self.assertEqual(receipt["methods_run"], 1)
        self.assertEqual(receipt["failures"], 0)
        self.assertFalse(receipt["successful"])
