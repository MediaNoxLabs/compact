#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

"""Finite source-to-ZKIR regression gate for guarded v2 Jubjub consumers.

Requires distinct baseline and candidate Scheme compactc binaries. This gate checks
emission and SSA graphs only; official VM/proof checks are separate evidence.
"""
from __future__ import annotations

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import subprocess

FIXTURES = Path(__file__).parent / 'tests/proof-regressions/guarded-point-read'
# Exhaustive only over the explicit Boolean inputs of these finite fixtures.
EXPECTATIONS = {
    'effect_plan_jubjub/read_branch': ['selected', 'not-selected'],
    'effect_plan_jubjub/compose': [],
    'effect_plan_jubjub/raw_after_write': [],
    'nested/nested': ['outer-and-inner', 'outer-and-not-inner'],
    'witness/from_witness': ['selected', 'not-selected'],
    'aggregate/from_aggregate': ['selected', 'not-selected'],
    'invalid/invalid': ['selected'],
    'unconditional/unconditional': [],
    'field_pair/fields': [],
}
# Deliberately fail closed on new opcodes, rather than guessing output arity or
# treating every JSON integer as an SSA reference (pi_skip.count is not one).
OPCODES = {
    'constrain_to_boolean': (0, ('var',)),
    'declare_pub_input': (0, ('var',)),
    'pi_skip': (0, ('guard',)),
    'output': (0, ('var',)),
    'load_imm': (1, ()),
    'cond_select': (1, ('bit', 'a', 'b')),
    'public_input': (1, ('guard',)),
    'private_input': (1, ('guard',)),
    'add': (1, ('a', 'b')),
    'mul': (1, ('a', 'b')),
    'ec_mul_generator': (2, ('scalar',)),
    'ec_add': (2, ('a_x', 'a_y', 'b_x', 'b_y')),
    'ec_mul': (2, ('a_x', 'a_y', 'scalar')),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def indexed(program):
    next_id = program['num_inputs']
    rows, producers = [], {}
    for position, instruction in enumerate(program['instructions']):
        require(instruction['op'] in OPCODES, f"unrecognized opcode {instruction['op']}")
        count, _ = OPCODES[instruction['op']]
        outputs = list(range(next_id, next_id + count))
        rows.append((position, instruction, outputs))
        for value in outputs:
            producers[value] = (position, instruction)
        next_id += count
    return rows, producers


def evaluate_guard(value, inputs, producers):
    if value < len(inputs):
        return inputs[value]
    _, producer = producers[value]
    op = producer['op']
    if op == 'load_imm':
        return int(producer['imm'], 16)
    if op == 'cond_select':
        bit = evaluate_guard(producer['bit'], inputs, producers)
        require(bit in (0, 1), 'guard is not Boolean')
        return evaluate_guard(producer['a' if bit else 'b'], inputs, producers)
    if op in ('add', 'mul'):
        a = evaluate_guard(producer['a'], inputs, producers)
        b = evaluate_guard(producer['b'], inputs, producers)
        return a + b if op == 'add' else a * b
    raise ValueError(f'guard depends on unsupported opcode {op}')


def expected_guard(name, inputs):
    a = inputs[0]
    if name == 'selected':
        return a
    if name == 'not-selected':
        return 1-a
    return a * (inputs[1] if name == 'outer-and-inner' else 1-inputs[1])


def check_graph(baseline, candidate, expected):
    """Remove only consumer-local identity selects, recover exact old SSA graph."""
    require({k: v for k, v in baseline.items() if k != 'instructions'} ==
            {k: v for k, v in candidate.items() if k != 'instructions'},
            'program metadata changed')
    rows, producers = indexed(candidate)
    removed, guards = {}, []
    for position, instruction, _ in rows:
        op = instruction['op']
        if op not in ('ec_add', 'ec_mul'):
            continue
        fields = ('a_x', 'a_y', 'b_x', 'b_y') if op == 'ec_add' else ('a_x', 'a_y')
        inputs = [instruction[field] for field in fields]
        # A guarded consumer receives an immediately preceding block of selects.
        block = rows[position-len(fields):position]
        if len(block) != len(fields) or [r[2] for r in block] != [[v] for v in inputs]:
            continue
        if any(r[1]['op'] != 'cond_select' for r in block):
            continue
        bits = {r[1]['bit'] for r in block}
        require(len(bits) == 1, 'point coordinates use different guards')
        for coordinate, (_, select, outputs) in enumerate(block):
            _, fallback = producers[select['b']]
            require(fallback['op'] == 'load_imm' and
                    int(fallback['imm'], 16) == coordinate % 2,
                    'inactive point is not the subgroup identity (0, 1)')
            removed[outputs[0]] = select['a']
        guards.append(next(iter(bits)))
    require(len(guards) == len(expected),
            f'guarded consumer count {len(guards)} != expected {len(expected)}')
    truth_tables = []
    for guard, name in zip(guards, expected):
        table = []
        for bits in itertools.product((0, 1), repeat=2 if name.startswith('outer') else 1):
            # The remaining parameters are irrelevant to a statement guard.
            inputs = list(bits) + [17] * (candidate['num_inputs'] - len(bits))
            actual = evaluate_guard(guard, inputs, producers)
            require(actual == expected_guard(name, inputs), f'incorrect full guard {name}: {bits}')
            table.append({'inputs': list(bits), 'active': actual})
        truth_tables.append({'guard': name, 'rows': table})
    # Fresh select values must be private to the ECC point operand use. Changing
    # an input/transcript/shared value must fail, even if graph erasure could hide it.
    for _, instruction, _ in rows:
        for field in OPCODES[instruction['op']][1]:
            if instruction[field] in removed:
                require(instruction['op'] in ('ec_add', 'ec_mul') and field.endswith(('_x', '_y')),
                        'guard select escaped its ECC point operand')
    mapping = {i: i for i in range(candidate['num_inputs'])}
    next_id = candidate['num_inputs']
    recovered = []
    for _, instruction, outputs in rows:
        if outputs and outputs[0] in removed:
            mapping[outputs[0]] = mapping[removed[outputs[0]]]
            continue
        normalized = dict(instruction)
        for field in OPCODES[instruction['op']][1]:
            normalized[field] = None if instruction[field] is None else mapping[instruction[field]]
        recovered.append(normalized)
        for value in outputs:
            mapping[value] = next_id
            next_id += 1
    require(recovered == baseline['instructions'],
            'changes beyond consumer-local selects (including input/transcript/SSA changes)')
    return {'guarded_consumers': len(guards), 'coordinate_selects': len(removed),
            'guard_truth_tables': truth_tables,
            'baseline_instruction_count': len(baseline['instructions']),
            'candidate_instruction_count': len(candidate['instructions'])}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compile_sources(compiler, output):
    for source in sorted(FIXTURES.glob('*.compact')):
        subprocess.run([str(compiler), '--skip-zk', '--emit-rust-ir', str(source),
                        str(output/source.stem)], check=True, capture_output=True, text=True)


def run(baseline_compiler, candidate_compiler, output):
    output.mkdir(parents=True, exist_ok=True)
    require(digest(baseline_compiler) != digest(candidate_compiler), 'compiler binaries must differ')
    for mode, compiler in [('baseline', baseline_compiler), ('candidate', candidate_compiler)]:
        compile_sources(compiler, output/mode)
    reports = []
    for key, expected in EXPECTATIONS.items():
        fixture, circuit = key.split('/')
        paths = [output/mode/fixture/'zkir'/f'{circuit}.zkir' for mode in ('baseline', 'candidate')]
        report = check_graph(*(json.loads(path.read_text()) for path in paths), expected)
        if not expected:
            require(paths[0].read_bytes() == paths[1].read_bytes(), f'{key}: unconditional bytes changed')
        reports.append({'fixture': key, 'source_sha256': digest(FIXTURES/f'{fixture}.compact'),
                        'baseline_zkir_sha256': digest(paths[0]), 'candidate_zkir_sha256': digest(paths[1]),
                        'byte_identical': paths[0].read_bytes() == paths[1].read_bytes(), **report})
    receipt = {'schema_version': 1, 'scope': 'finite source-to-ZKIR emission; not VM or proof evidence',
               'baseline_compiler_sha256': digest(baseline_compiler),
               'candidate_compiler_sha256': digest(candidate_compiler), 'circuits': reports}
    (output/'emission-report.json').write_text(json.dumps(receipt, indent=2)+'\n')
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-compiler', type=Path, required=True)
    parser.add_argument('--candidate-compiler', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        report = run(args.baseline_compiler.resolve(), args.candidate_compiler.resolve(), args.output.resolve())
    except subprocess.CalledProcessError as error:
        parser.exit(1, error.stderr)
    except (ValueError, KeyError) as error:
        parser.exit(1, str(error)+'\n')
    print(f"validated {len(report['circuits'])} finite circuit emission comparisons")


if __name__ == '__main__':
    main()
