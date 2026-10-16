// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_counter_hash_source_fixture::{
    ledger_contract as c, ledger_slots, runtime as r,
    support::{Amount, Witness, WitnessInput, products_reference, reference},
};
use r::context::{CircuitContext, CircuitResult, ConstructorContext};
use r::recording::RecordedCircuitResult;
use r::{CompactError, Field, FixedBytes};
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
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
fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../oracles/ts-primitives.json")).unwrap()
}
fn unhex<const N: usize>(s: &str) -> FixedBytes<N> {
    assert_eq!(s.len(), 2 * N);
    FixedBytes::new(std::array::from_fn(|i| {
        u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()
    }))
}
#[test]
fn counter_hash_calls_conversion_edges_and_reused_pair_match_ts_primitives() {
    let data = oracle();
    assert_eq!(data["version"], "0.16.101");
    let cases = data["run"].as_array().unwrap();
    assert_eq!(cases.len(), 18);
    for case in cases {
        let bytes = unhex::<32>(case["encoded"].as_str().unwrap());
        let seed = Field::from_le_bytes(&bytes.0).unwrap();
        let raw_amount = case["amount"].as_u64().unwrap();
        let amount = Amount::new(raw_amount as u128).unwrap();
        let selected = case["selected"].as_bool().unwrap();
        let a = Witness::default();
        let b = Witness::default();
        let m = Witness::default();
        let native = c::run(context(), &a, seed, amount, selected).unwrap();
        let recorded = c::recorded::run(context(), &b, seed, amount, selected).unwrap();
        let manual = reference(context(), seed, amount, selected, &m).unwrap();
        compare(&native, &recorded, &manual);
        assert_eq!(native.result.encoded, bytes);
        assert_eq!(
            native.result.digest,
            unhex::<32>(case["digest"].as_str().unwrap())
        );
        assert_eq!(
            native.result.pair,
            unhex::<32>(case["pair"].as_str().unwrap())
        );
        assert_eq!(
            ledger_slots::round
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            257 + raw_amount
        );
        let expected = if selected {
            vec![WitnessInput {
                private: 7,
                seed,
                round: 257 + raw_amount,
            }]
        } else {
            vec![]
        };
        assert_eq!(*a.calls.borrow(), expected);
        assert_eq!(*b.calls.borrow(), expected);
        assert_eq!(*m.calls.borrow(), expected);
        assert_eq!(native.context.private_state, if selected { 8 } else { 7 });
        assert_eq!(
            native.private_transcript_outputs.len(),
            usize::from(selected)
        );
    }
}
#[test]
fn fixed_byte_products_include_zero_width_and_empty_products() {
    let data = oracle();
    for case in data["products"].as_array().unwrap() {
        let values = case["value"].as_array().unwrap();
        let value = FixedBytes::new(std::array::from_fn(|i| values[i].as_u64().unwrap() as u8));
        let native = c::products(context(), value, FixedBytes::new([])).unwrap();
        let recorded = c::recorded::products(context(), value, FixedBytes::new([])).unwrap();
        let manual = products_reference(context(), value).unwrap();
        compare(&native, &recorded, &manual);
        for (name, actual) in [
            ("bytes", native.result.bytes),
            ("vector", native.result.vector),
            ("nested", native.result.nested),
            ("empty", native.result.empty),
            ("emptyVector", native.result.emptyVector),
            ("emptyStruct", native.result.emptyStruct),
        ] {
            assert_eq!(actual, unhex::<32>(case[name].as_str().unwrap()), "{name}");
        }
        assert_eq!(native.context.private_state, 7);
        assert!(native.private_transcript_outputs.is_empty());
    }
}
#[test]
fn witness_failure_observes_increment_and_unselected_witness_is_skipped() {
    let seed = Field::from(5_u64);
    let amount = Amount::new(3).unwrap();
    for selected in [false, true] {
        let a = Witness {
            refuse: true,
            ..Default::default()
        };
        let b = Witness {
            refuse: true,
            ..Default::default()
        };
        let m = Witness {
            refuse: true,
            ..Default::default()
        };
        let native = c::run(context(), &a, seed, amount, selected);
        let recorded = c::recorded::run(context(), &b, seed, amount, selected);
        let manual = reference(context(), seed, amount, selected, &m);
        if selected {
            let expected = CompactError::AssertionFailed("counter hash witness refused".into());
            assert_eq!(native.err().unwrap(), expected);
            assert_eq!(recorded.err().unwrap(), expected);
            assert_eq!(manual.err().unwrap(), expected);
            assert_eq!(
                *m.calls.borrow(),
                [WitnessInput {
                    private: 7,
                    seed,
                    round: 260
                }]
            );
        } else {
            compare(&native.unwrap(), &recorded.unwrap(), &manual.unwrap());
            assert!(m.calls.borrow().is_empty());
        }
        assert_eq!(*a.calls.borrow(), *m.calls.borrow());
        assert_eq!(*b.calls.borrow(), *m.calls.borrow());
    }
}
