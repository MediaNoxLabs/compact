// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_witness_inputs_source_fixture::support::*;
use compact_rust_effect_plan_witness_inputs_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use r::{context::CircuitResult, recording::RecordedCircuitResult};
fn equal<T: std::fmt::Debug + PartialEq>(
    native: &CircuitResult<u64, T>,
    recorded: &RecordedCircuitResult<u64, T>,
    manual: &RecordedCircuitResult<u64, T>,
) {
    for actual in [&recorded.execution, &manual.execution] {
        assert_eq!(actual.result, native.result);
        assert_eq!(actual.context.private_state, native.context.private_state);
        assert_eq!(actual.context.query.state, native.context.query.state);
        assert_eq!(actual.context.query.effects, native.context.query.effects);
        assert_eq!(actual.gas_cost, native.gas_cost);
        assert_eq!(
            actual.private_transcript_outputs,
            native.private_transcript_outputs
        );
    }
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(manual.public.verify_ops()).unwrap()
    );
    assert!(!recorded.public.verify_ops().is_empty());
    let replay = |value: &RecordedCircuitResult<u64, T>| {
        value
            .public
            .initial()
            .query(
                value.public.verify_ops(),
                None,
                &value.execution.context.cost_model,
            )
            .unwrap()
    };
    let a = replay(recorded);
    let b = replay(manual);
    assert_eq!(a.context.state, native.context.query.state);
    assert_eq!(a.context.effects, native.context.query.effects);
    assert_eq!(a.gas_cost, b.gas_cost);
}
#[test]
fn raw_owned_inputs_arrive_unchanged_and_callback_mutation_cannot_affect_reuse() {
    for (text, bytes) in [
        ("", vec![]),
        ("é文🦀", vec![255, 254]),
        ("hi\0world", vec![0, 1, 0, 2]),
        ("long text exceeding thirty-one encoded bytes", vec![3; 65]),
        ("bad\0", vec![1]),
        ("good", vec![1, 0]),
        ("\0\0", vec![0, 0]),
    ] {
        for selected in [false, true] {
            let (a, b, m) = (Witness::default(), Witness::default(), Witness::default());
            // Reuse each witness object across invocations with fresh private contexts.
            for iteration in 0..2 {
                let native =
                    c::run(context(), &a, text.into(), bytes.clone().into(), selected).unwrap();
                let recorded =
                    c::recorded::run(context(), &b, text.into(), bytes.clone().into(), selected)
                        .unwrap();
                let reference = manual(context(), text, &bytes, selected, &m).unwrap();
                equal(&native, &recorded, &reference);
                assert!(native.result.value);
                assert_eq!(
                    slots::calls
                        .inspect(native.context.query.state.get_ref())
                        .unwrap(),
                    6
                );
                let count = if selected { 2 } else { 1 };
                assert_eq!(native.context.private_state, 7 + count);
                assert_eq!(
                    native.private_transcript_outputs,
                    vec![r::fab::AlignedValue::from(true); count as usize]
                );
                let mut expected = vec![Event {
                    private: 7,
                    counter: 6,
                    arguments: arguments(text, &bytes),
                }];
                if selected {
                    expected.push(Event {
                        private: 8,
                        counter: 6,
                        arguments: arguments(text, &bytes),
                    });
                }
                let expected = (0..=iteration)
                    .flat_map(|_| expected.iter().cloned())
                    .collect::<Vec<_>>();
                for witness in [&a, &b, &m] {
                    assert_eq!(*witness.events.borrow(), expected);
                }
            }
        }
    }
}
#[test]
fn callback_refusal_preserves_prefix_and_unselected_callback_remains_lazy() {
    for kind in [Refusal::False, Refusal::Error] {
        for ordinal in [1, 2] {
            for selected in [false, true] {
                let make = || Witness {
                    refusal: Some((ordinal, kind)),
                    ..Default::default()
                };
                let (a, b, m) = (make(), make(), make());
                let native = c::run(context(), &a, "raw\0".into(), vec![9, 0].into(), selected);
                let recorded =
                    c::recorded::run(context(), &b, "raw\0".into(), vec![9, 0].into(), selected);
                let reference = manual(context(), "raw\0", &[9, 0], selected, &m);
                if ordinal == 2 && !selected {
                    equal(&native.unwrap(), &recorded.unwrap(), &reference.unwrap());
                } else {
                    let message = match kind {
                        Refusal::Error => format!("callback {ordinal} refused"),
                        Refusal::False => if ordinal == 1 {
                            "first callback refused"
                        } else {
                            "second callback refused"
                        }
                        .into(),
                    };
                    let expected = r::CompactError::AssertionFailed(message);
                    assert_eq!(native.err().unwrap(), expected);
                    assert_eq!(recorded.err().unwrap(), expected);
                    assert_eq!(reference.err().unwrap(), expected);
                }
                let count = if ordinal == 1 || !selected { 1 } else { 2 };
                let expected = (0..count)
                    .map(|i| Event {
                        private: 7 + i,
                        counter: 6,
                        arguments: arguments("raw\0", &[9, 0]),
                    })
                    .collect::<Vec<_>>();
                for witness in [&a, &b, &m] {
                    assert_eq!(*witness.events.borrow(), expected);
                }
            }
        }
    }
}
