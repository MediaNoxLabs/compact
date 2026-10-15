#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""#530 prologue decoder controls; these do not execute ARM64 instructions."""
import unittest

from calibrate_effect_plan_frames import static_reservation


PREFIX = [
    'stp x28, x27, [sp, #-0x20]!',
    'stp x29, x30, [sp, #0x10]',
    'add x29, sp, #0x10',
]


def disassemble(instructions):
    # LLVM objdump form, including addresses used by the actual loop back edge.
    return '\n'.join(f'{0x1000 + i * 4:x}: 00000000 \t{asm}'
                     for i, asm in enumerate(instructions))


def loop_prologue():
    # Retained failure: block allocated 0xa000 pages +0x630 tail +0x20 saves.
    return PREFIX + [
        'sub x9, sp, #0xa, lsl #12 ; =0xa000',
        'sub sp, sp, #0x1, lsl #12 ; =0x1000',
        'ldr xzr, [sp]',
        'cmp sp, x9',
        'b.ne 0x1010 <block+0x10>',
        'sub sp, sp, #0x630',
        'ldr xzr, [sp]',
        'add x9, sp, #0xa, lsl #12',
        'str x9, [sp, #0x8c0]',
    ]


class PrologueTests(unittest.TestCase):
    def test_real_failing_recursive_frame_counts_loop_and_tail_not_partial_32(self):
        self.assertEqual(static_reservation(disassemble(loop_prologue())), 42576)

    def test_public_driver_loop_has_its_independent_larger_reservation(self):
        instructions = loop_prologue()
        instructions[3] = 'sub x9, sp, #0xb, lsl #12'
        instructions[8] = 'sub sp, sp, #0xaa0'
        self.assertEqual(static_reservation(disassemble(instructions)), 47808)

    def test_unrolled_page_probes_do_not_stop_after_first_4128_bytes(self):
        instructions = PREFIX + [
            'sub sp, sp, #0x1, lsl #12', 'ldr xzr, [sp]',
            'sub sp, sp, #4096', 'ldr xzr, [sp]',
            'sub sp, sp, #0x1, lsl #12', 'ldr xzr, [sp]',
            'sub sp, sp, #0x1, lsl #12', 'ldr xzr, [sp]',
            'sub sp, sp, #0x5c0', 'ldr xzr, [sp]',
            'add x9, sp, #0x4, lsl #12',
        ]
        self.assertEqual(static_reservation(disassemble(instructions)), 17888)

    def test_small_prologue_and_body_instruction_boundary(self):
        instructions = PREFIX + ['sub sp, sp, #0x200', 'str x8, [sp, #0x8]']
        self.assertEqual(static_reservation(disassemble(instructions)), 544)

    def test_loop_requires_exact_page_probe_bound_and_back_edge(self):
        mutations = {
            3: ['sub x9, sp, #0x1001', 'sub x9, sp, #0', 'sub x9, sp, x10'],
            4: ['sub sp, sp, #0x800', 'sub sp, sp, x10'],
            5: ['ldr x0, [sp]', 'nop'],
            6: ['cmp sp, x10', 'cmp x9, sp'],
            7: ['b.ne 0x100c', 'b.eq 0x1010', 'b.ne 0x1014'],
        }
        for index, alternatives in mutations.items():
            for altered in alternatives:
                with self.subTest(index=index, altered=altered):
                    instructions = loop_prologue()
                    instructions[index] = altered
                    self.assertIsNone(static_reservation(disassemble(instructions)))

    def test_truncated_prologue_never_claims_a_partial_reservation(self):
        instructions = loop_prologue()
        for length in range(1, 11):
            with self.subTest(length=length):
                self.assertIsNone(static_reservation(disassemble(instructions[:length])))

    def test_unrecognized_register_allocation_or_stack_probe_is_unknown(self):
        tails = [
            ['sub sp, sp, x9'],
            ['mov x9, #0x1000', 'sub sp, sp, x9'],
            ['bl 0x2000 <___chkstk_darwin>', 'sub sp, sp, x9'],
            ['and sp, sp, #0xfffffffffffffff0'],
            ['mov sp, x9'],
            ['nop', 'str x0, [sp, #-0x10]!'],
        ]
        for tail in tails:
            with self.subTest(tail=tail):
                self.assertIsNone(static_reservation(disassemble(PREFIX + tail)))

    def test_no_qualified_prologue_is_unknown_not_zero(self):
        self.assertIsNone(static_reservation(''))
        self.assertIsNone(static_reservation(disassemble(['ret'])))

    def test_body_nonstack_arithmetic_does_not_inflate_reservation(self):
        instructions = PREFIX + ['sub sp, sp, #64', 'mov x0, x1',
                                 'sub x2, x2, #4096', 'add sp, sp, #64', 'ret']
        self.assertEqual(static_reservation(disassemble(instructions)), 96)


if __name__ == '__main__':
    unittest.main()
