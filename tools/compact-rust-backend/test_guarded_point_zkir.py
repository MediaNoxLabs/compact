#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

"""Artifact-dependent mutation controls over baseline/candidate emission artifacts.

Run guarded_point_zkir.py first, then this file with --artifacts OUTPUT. These
mutations challenge the graph gate; official ZKIR VM/proof controls are separate.
Ordinary discovery skips these ten cases when no artifacts are selected. Explicit
CLI or full-gate artifact selection still executes all original assertions.
"""
import argparse
import copy
import json
import os
from pathlib import Path
import unittest

import guarded_point_zkir as gate

ARTIFACTS = None


class GuardedPointGraphTests(unittest.TestCase):
    def setUp(self):
        self.artifacts = ARTIFACTS
        if self.artifacts is None:
            selected = os.environ.get(gate.RETAINED_GRAPH_ENVIRONMENT)
            if selected is None:
                self.skipTest("manual graph artifacts required: use --artifacts or "
                              + gate.RETAINED_GRAPH_ENVIRONMENT)
            self.artifacts = self.enterContext(gate.retained_artifact_root(selected))
        self.before = self.load('baseline')
        self.after = self.load('candidate')
        self.expected = gate.EXPECTATIONS['nested/nested']

    def load(self, mode):
        return json.loads((self.artifacts/mode/'nested/zkir/nested.zkir').read_text())

    def consumer(self):
        for index, instruction in enumerate(self.after['instructions']):
            if instruction['op'] == 'ec_add':
                return index, instruction
        self.fail('nested source must contain ec_add')

    def check(self):
        return gate.check_graph(self.before, self.after, self.expected)

    def test_fresh_nested_graph_and_all_four_guard_valuations(self):
        receipt = self.check()
        self.assertEqual(receipt['coordinate_selects'], 6)
        self.assertEqual([r['active'] for r in receipt['guard_truth_tables'][0]['rows']], [0, 0, 0, 1])
        self.assertEqual([r['active'] for r in receipt['guard_truth_tables'][1]['rows']], [0, 0, 1, 0])

    def test_zero_y_is_not_a_valid_inactive_identity(self):
        index, _ = self.consumer()
        block = self.after['instructions'][index-4:index]
        block[1]['b'] = block[0]['b']
        with self.assertRaisesRegex(ValueError, 'subgroup identity'):
            self.check()

    def test_inner_guard_alone_cannot_replace_full_nested_guard(self):
        index, _ = self.consumer()
        for instruction in self.after['instructions'][index-4:index]:
            instruction['bit'] = 1
        with self.assertRaisesRegex(ValueError, 'incorrect full guard'):
            self.check()

    def test_per_coordinate_guards_must_agree(self):
        index, _ = self.consumer()
        self.after['instructions'][index-4]['bit'] = 0
        with self.assertRaisesRegex(ValueError, 'different guards'):
            self.check()

    def test_masked_point_cannot_replace_raw_transcript_value(self):
        _, consumer = self.consumer()
        # A later public declaration must continue to refer to its original
        # raw/masked-by-transcript-rule value, never our local ECC operand.
        self.after['instructions'].append({'op': 'declare_pub_input', 'var': consumer['a_y']})
        with self.assertRaisesRegex(ValueError, 'escaped'):
            self.check()

    def test_original_public_input_guard_cannot_change(self):
        instruction = next(i for i in self.after['instructions'] if i['op'] == 'public_input')
        instruction['guard'] = None
        with self.assertRaisesRegex(ValueError, 'changes beyond'):
            self.check()

    def test_active_operand_cannot_change(self):
        index, _ = self.consumer()
        select = self.after['instructions'][index-4]
        select['a'] = select['b']
        with self.assertRaisesRegex(ValueError, 'changes beyond'):
            self.check()

    def test_scalar_is_not_part_of_point_normalization(self):
        mul = next(i for i in self.after['instructions'] if i['op'] == 'ec_mul')
        mul['scalar'] = 0
        with self.assertRaisesRegex(ValueError, 'changes beyond'):
            self.check()

    def test_missing_fix_is_not_accepted_as_unchanged_graph(self):
        self.after = copy.deepcopy(self.before)
        with self.assertRaisesRegex(ValueError, 'guarded consumer count'):
            self.check()

    def test_unknown_opcode_fails_closed(self):
        self.after['instructions'].append({'op': 'future_point_gate'})
        with self.assertRaisesRegex(ValueError, 'unrecognized opcode'):
            self.check()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', type=Path, required=True)
    args, unittest_args = parser.parse_known_args()
    ARTIFACTS = args.artifacts.resolve()
    unittest.main(argv=[__file__, *unittest_args])
