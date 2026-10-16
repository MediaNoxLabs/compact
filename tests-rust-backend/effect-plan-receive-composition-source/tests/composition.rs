// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_receive_composition_source_fixture::{
    ledger_contract as c, runtime as r,
    support::{Witness, WitnessInput, reference},
    types,
};
use r::context::{CircuitContext, ConstructorContext};
use r::{BoundedUint, CompactError, FixedBytes};
fn context() -> CircuitContext<u64> {
    let mut ctx = c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(r::ledger::ContractAddress(r::ledger::HashOutput([3; 32])))
        .with_coin_public_key_bytes([7; 32]);
    ctx.set_zswap_output_start(9).unwrap();
    ctx
}
fn coin(value: u128) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new([1; 32]),
        color: FixedBytes::new([2; 32]),
        value: BoundedUint::new(value).unwrap(),
    }
}
#[test]
fn witness_selected_receive_preserves_native_execution_and_independent_public_replay() {
    for selected in [false, true] {
        for amount in [42_u128, 1_u128 << 64] {
            let a = Witness {
                selected,
                ..Default::default()
            };
            let b = Witness {
                selected,
                ..Default::default()
            };
            let m = Witness {
                selected,
                ..Default::default()
            };
            let native = c::acceptSelected(context(), &a, coin(amount)).unwrap();
            let recorded = c::recorded::acceptSelected(context(), &b, coin(amount)).unwrap();
            let manual = reference(context(), &m, coin(amount)).unwrap();
            for witness in [&a, &b, &m] {
                assert_eq!(
                    *witness.calls.borrow(),
                    vec![WitnessInput {
                        private: 7,
                        value: amount,
                        accepted: false
                    }]
                );
            }
            for result in [&recorded.execution, &manual.execution] {
                assert_eq!(result.context.query.state, native.context.query.state);
                assert_eq!(result.context.query.effects, native.context.query.effects);
                assert_eq!(
                    result.context.query.call_context.com_indices,
                    native.context.query.call_context.com_indices
                );
                assert_eq!(result.context.private_state, 8);
                assert_eq!(
                    result.private_transcript_outputs,
                    native.private_transcript_outputs
                );
                assert_eq!(result.gas_cost, native.gas_cost);
                assert_eq!(
                    result.context.circuit_zswap(),
                    native.context.circuit_zswap()
                );
            }
            assert_eq!(native.context.private_state, 8);
            assert_eq!(
                native.context.circuit_zswap().outputs().len(),
                usize::from(selected)
            );
            assert_eq!(
                native.private_transcript_outputs.len(),
                1 + usize::from(selected)
            );
            if selected {
                let output = &native.context.circuit_zswap().outputs()[0];
                assert_eq!(output.provisional_index, 9);
                assert_eq!(output.coin.value, amount);
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
    }
}
#[test]
fn failed_witness_observes_prior_write_and_does_not_produce_a_recording() {
    let a = Witness {
        selected: true,
        refuse: true,
        ..Default::default()
    };
    let b = Witness {
        selected: true,
        refuse: true,
        ..Default::default()
    };
    let m = Witness {
        selected: true,
        refuse: true,
        ..Default::default()
    };
    let failures = [
        c::acceptSelected(context(), &a, coin(42)).err().unwrap(),
        c::recorded::acceptSelected(context(), &b, coin(42))
            .err()
            .unwrap(),
        reference(context(), &m, coin(42)).err().unwrap(),
    ];
    for error in failures {
        assert!(
            matches!(error, CompactError::AssertionFailed(ref message) if message == "receive choice refused")
        );
    }
    for witness in [&a, &b, &m] {
        assert_eq!(
            *witness.calls.borrow(),
            vec![WitnessInput {
                private: 7,
                value: 42,
                accepted: false
            }]
        );
    }
}
