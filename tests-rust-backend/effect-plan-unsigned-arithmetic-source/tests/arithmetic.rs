// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_unsigned_arithmetic_source_fixture::{
    ledger_contract as c, runtime as r,
    support::{U8, Witness, WitnessInput, context, narrow_reference, reference},
};
use r::context::CircuitResult;
use r::recording::RecordedCircuitResult;
use r::{CompactError, Field};
fn u8(value: u128) -> U8 {
    U8::new(value).unwrap()
}
fn expected_calls(tags: &[u64], left: u128, right: u128, difference: u128) -> Vec<WitnessInput> {
    tags.iter()
        .enumerate()
        .map(|(index, tag)| WitnessInput {
            tag: Field::from(*tag),
            value: u8(if tag % 2 == 1 { left } else { right }),
            private: 7 + index as u64,
            difference: u8(if *tag >= 3 && *tag <= 4 {
                difference
            } else {
                0
            }),
            narrowed: u8(0),
        })
        .collect()
}
fn check_calls(a: &Witness, b: &Witness, m: &Witness, expected: &[WitnessInput]) {
    assert_eq!(*a.calls.borrow(), expected);
    assert_eq!(*b.calls.borrow(), expected);
    assert_eq!(*m.calls.borrow(), expected);
}
fn compare<T: PartialEq + std::fmt::Debug>(
    native: &CircuitResult<u64, T>,
    recorded: &RecordedCircuitResult<u64, T>,
    manual: &RecordedCircuitResult<u64, T>,
) {
    for execution in [&recorded.execution, &manual.execution] {
        assert_eq!(execution.result, native.result);
        assert_eq!(execution.context.query.state, native.context.query.state);
        assert_eq!(
            execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(
            execution.context.query.call_context.com_indices,
            native.context.query.call_context.com_indices
        );
        assert_eq!(
            execution.context.private_state,
            native.context.private_state
        );
        assert_eq!(
            execution.private_transcript_outputs,
            native.private_transcript_outputs
        );
        assert_eq!(execution.gas_cost, native.gas_cost);
    }
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(manual.public.verify_ops()).unwrap()
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    let expected = manual
        .public
        .initial()
        .query(
            manual.public.verify_ops(),
            None,
            &manual.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(replay.gas_cost, expected.gas_cost);
}

#[test]
fn unsigned_arithmetic_evaluates_effectful_operands_once_in_source_order() {
    for (left, right) in [(0, 0), (1, 0), (1, 1), (7, 5), (255, 1), (255, 255)] {
        for selected in [false, true] {
            let a = Witness::default();
            let b = Witness::default();
            let m = Witness::default();
            let native = c::run(context(), &a, u8(left), u8(right), selected).unwrap();
            let recorded = c::recorded::run(context(), &b, u8(left), u8(right), selected).unwrap();
            let manual = reference(context(), u8(left), u8(right), selected, &m).unwrap();
            compare(&native, &recorded, &manual);
            assert_eq!(native.result.difference.value(), left - right);
            assert_eq!(
                native.result.product.value(),
                if selected { left * right } else { 0 }
            );
            let tags = if selected {
                vec![1, 2, 3, 4]
            } else {
                vec![1, 2]
            };
            check_calls(
                &a,
                &b,
                &m,
                &expected_calls(&tags, left, right, left - right),
            );
            assert_eq!(native.context.private_state, 7 + tags.len() as u64);
            assert_eq!(native.private_transcript_outputs.len(), tags.len());
        }
    }
}
#[test]
fn checked_result_narrowing_accepts_boundaries_and_refuses_out_of_range_values() {
    for (left, right) in [(0, 255), (15, 17), (255, 1)] {
        for selected in [false, true] {
            let a = Witness::default();
            let b = Witness::default();
            let m = Witness::default();
            let native = c::narrow(context(), &a, u8(left), u8(right), selected).unwrap();
            let recorded =
                c::recorded::narrow(context(), &b, u8(left), u8(right), selected).unwrap();
            let manual = narrow_reference(context(), u8(left), u8(right), selected, &m).unwrap();
            compare(&native, &recorded, &manual);
            assert_eq!(
                native.result.value.value(),
                if selected { left * right } else { 0 }
            );
            let tags = if selected { vec![5, 6] } else { vec![] };
            check_calls(&a, &b, &m, &expected_calls(&tags, left, right, 0));
            assert_eq!(native.context.private_state, 7 + tags.len() as u64);
        }
    }
    for (left, right) in [(16, 16), (255, 255)] {
        let a = Witness::default();
        let b = Witness::default();
        let m = Witness::default();
        let expected = CompactError::UnsignedOutOfRange {
            value: left * right,
            max: 255,
        };
        assert_eq!(
            c::narrow(context(), &a, u8(left), u8(right), true)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::narrow(context(), &b, u8(left), u8(right), true)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            narrow_reference(context(), u8(left), u8(right), true, &m)
                .err()
                .unwrap(),
            expected
        );
        check_calls(&a, &b, &m, &expected_calls(&[5, 6], left, right, 0));
        let a = Witness::default();
        let b = Witness::default();
        let m = Witness::default();
        let native = c::narrow(context(), &a, u8(left), u8(right), false).unwrap();
        let recorded = c::recorded::narrow(context(), &b, u8(left), u8(right), false).unwrap();
        let manual = narrow_reference(context(), u8(left), u8(right), false, &m).unwrap();
        compare(&native, &recorded, &manual);
        check_calls(&a, &b, &m, &[]);
        assert_eq!(native.result.value.value(), 0);
    }
}
#[test]
fn underflow_refuses_after_both_operands_and_before_later_witnesses() {
    for (left, right) in [(0, 1), (5, 7)] {
        for selected in [false, true] {
            let a = Witness::default();
            let b = Witness::default();
            let m = Witness::default();
            assert_eq!(
                c::run(context(), &a, u8(left), u8(right), selected)
                    .err()
                    .unwrap(),
                CompactError::UnsignedUnderflow
            );
            assert_eq!(
                c::recorded::run(context(), &b, u8(left), u8(right), selected)
                    .err()
                    .unwrap(),
                CompactError::UnsignedUnderflow
            );
            assert_eq!(
                reference(context(), u8(left), u8(right), selected, &m)
                    .err()
                    .unwrap(),
                CompactError::UnsignedUnderflow
            );
            check_calls(&a, &b, &m, &expected_calls(&[1, 2], left, right, 0));
        }
    }
}
#[test]
fn witness_refusals_preserve_prefix_and_untaken_branches_skip_callbacks() {
    for tag in 1_u64..=4 {
        let make = || Witness {
            refuse_tag: Some(Field::from(tag)),
            ..Default::default()
        };
        let a = make();
        let b = make();
        let m = make();
        let expected = CompactError::AssertionFailed("operand witness refused".into());
        assert_eq!(
            c::run(context(), &a, u8(7), u8(5), true).err().unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::run(context(), &b, u8(7), u8(5), true)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            reference(context(), u8(7), u8(5), true, &m).err().unwrap(),
            expected
        );
        check_calls(
            &a,
            &b,
            &m,
            &expected_calls(&(1..=tag).collect::<Vec<_>>(), 7, 5, 2),
        );
    }
    for tag in [3_u64, 4] {
        let make = || Witness {
            refuse_tag: Some(Field::from(tag)),
            ..Default::default()
        };
        let a = make();
        let b = make();
        let m = make();
        let native = c::run(context(), &a, u8(7), u8(5), false).unwrap();
        let recorded = c::recorded::run(context(), &b, u8(7), u8(5), false).unwrap();
        let manual = reference(context(), u8(7), u8(5), false, &m).unwrap();
        compare(&native, &recorded, &manual);
        check_calls(&a, &b, &m, &expected_calls(&[1, 2], 7, 5, 2));
    }
    for tag in [5_u64, 6] {
        let make = || Witness {
            refuse_tag: Some(Field::from(tag)),
            ..Default::default()
        };
        let a = make();
        let b = make();
        let m = make();
        let expected = CompactError::AssertionFailed("operand witness refused".into());
        assert_eq!(
            c::narrow(context(), &a, u8(7), u8(5), true).err().unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::narrow(context(), &b, u8(7), u8(5), true)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            narrow_reference(context(), u8(7), u8(5), true, &m)
                .err()
                .unwrap(),
            expected
        );
        check_calls(
            &a,
            &b,
            &m,
            &expected_calls(&(5..=tag).collect::<Vec<_>>(), 7, 5, 0),
        );
    }
}
