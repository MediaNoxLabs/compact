#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Negative controls for the source capability preservation gate."""
import copy
import unittest
import frontend_return_plan_gate as gate


def accepted(recorded=True, observed=True, proof=True, names=("call",)):
    return {"exit_code": 0, "capabilities": {"circuits": [
        {"name": name, "recorded": recorded, "observed_call": observed,
         "proof_required": proof} for name in names]}}


class CompatibilityTests(unittest.TestCase):
    def errors(self, baseline, candidate):
        return gate.compare("source.compact", baseline, candidate)["errors"]

    def test_failed_native_compilation_is_not_a_capability_success(self):
        self.assertEqual(self.errors(accepted(), {"exit_code": 255}),
                         ["previously accepted source failed"])

    def test_recorded_and_observed_regressions_are_independent(self):
        self.assertEqual(self.errors(accepted(), accepted(recorded=False)), ["call: lost recorded"])
        self.assertEqual(self.errors(accepted(), accepted(observed=False)), ["call: lost observed_call"])

    def test_disappeared_circuit_cannot_satisfy_preservation_vacuously(self):
        self.assertEqual(self.errors(accepted(), accepted(names=())), ["missing circuit call"])

    def test_proof_requirement_cannot_be_silently_erased(self):
        self.assertEqual(self.errors(accepted(), accepted(proof=False)), ["call: changed proof requirement"])

    def test_new_native_coverage_can_remain_recording_unavailable(self):
        self.assertEqual(self.errors({"exit_code": 255}, accepted(False, False)), [])


class SubtractionNormalizationTests(unittest.TestCase):
    def fixture(self):
        arithmetic = {"kind": "unsigned_subtract", "max": "65535",
                      "left": {"kind": "parameter", "name": "value"},
                      "right": {"kind": "unsigned_literal", "value": "1", "max": "65535"}}
        circuits = []
        for name, explicit in (("mutate_then_subtract", False), ("explicit_guard_then_subtract", True)):
            actions = [{"kind": "assert", "message": "explicit guard before mutation"}] if explicit else []
            actions.append({"kind": "counter_increment"})
            circuits.append({"name": name, "result": {"kind": "unsigned", "max": "65535"},
                             "actions": [], "return_value": {"kind": "effectful", "body": {
                                 "kind": "conditional", "then": {"kind": "sequence", "actions": actions,
                                     "result": {"kind": "value", "value": copy.deepcopy(arithmetic)}}}}})
        return {"exit_code": 0, "ir": {"stateful_circuits": circuits}}

    def check(self, fixture):
        return gate.regression_errors("checked_subtraction.compact", {"exit_code": 255}, fixture)

    def test_valid_normalization_controls(self):
        self.assertEqual(self.check(self.fixture()), [])

    def test_deleting_subtraction_does_not_pass_absence_of_assert_check(self):
        fixture = self.fixture()
        fixture["ir"]["stateful_circuits"][0]["return_value"]["body"]["then"]["result"]["value"] = {"kind": "parameter", "name": "value"}
        self.assertTrue(self.check(fixture))

    def test_changed_bound_or_operand_is_rejected(self):
        for key, value in (("max", "255"), ("right", {"kind": "unsigned_literal", "value": "2", "max": "65535"})):
            fixture = self.fixture()
            arithmetic = fixture["ir"]["stateful_circuits"][0]["return_value"]["body"]["then"]["result"]["value"]
            arithmetic[key] = value
            self.assertTrue(self.check(fixture))

    def test_missing_or_reordered_user_assertion_is_rejected(self):
        for reordered in (False, True):
            fixture = self.fixture()
            actions = fixture["ir"]["stateful_circuits"][1]["return_value"]["body"]["then"]["actions"]
            if reordered:
                actions.reverse()
            else:
                actions.pop(0)
            self.assertTrue(self.check(fixture))


if __name__ == "__main__":
    unittest.main()
