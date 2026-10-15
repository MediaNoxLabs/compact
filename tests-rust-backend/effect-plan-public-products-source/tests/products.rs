// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_public_products_source_fixture::{
    ledger_contract as c, runtime as r, support::*,
};
use r::fab::{Alignment, AlignmentAtom};
#[test]
fn mixed_products_preserve_ownership_atoms_alignment_and_full_recording() {
    for (text, bytes) in [
        ("", vec![]),
        ("é文🦀", vec![255, 254]),
        ("hi\0world", vec![0, 1, 0, 2]),
    ] {
        for selected in [false, true] {
            let (state, key) = setup();
            let input = input(text, &bytes);
            let replacement = if selected { vec![9, 0, 8] } else { vec![9, 0] };
            let (a, b, m, w) = (
                Witness::new(&replacement),
                Witness::new(&replacement),
                Witness::new(&replacement),
                Witness::new(&replacement),
            );
            let native = c::run(state.circuit_context(5), &a, input.clone(), selected).unwrap();
            let recorded =
                c::recorded::run(state.circuit_context(5), &b, input.clone(), selected).unwrap();
            let manual = manual(state.circuit_context(5), &input, selected, &m).unwrap();
            assert_eq!(native.result, expected(&input, selected, &a.bytes));
            for actual in [&recorded.execution, &manual.execution] {
                assert_eq!(actual.result, native.result);
                assert_eq!(actual.context.query.state, native.context.query.state);
                assert_eq!(actual.context.query.effects, native.context.query.effects);
                assert_eq!(actual.context.private_state, native.context.private_state);
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
            let replay = recorded
                .public
                .initial()
                .query(
                    recorded.public.verify_ops(),
                    None,
                    &recorded.execution.context.cost_model,
                )
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            // Compare whole-transcript gas against an independently built whole transcript.
            let reference = manual
                .public
                .initial()
                .query(
                    manual.public.verify_ops(),
                    None,
                    &manual.execution.context.cost_model,
                )
                .unwrap();
            assert_eq!(replay.gas_cost, reference.gas_cost);
            let contract = c::Contract::from(w.clone());
            let call = contract
                .recording()
                .run_call(&state, 5, input.clone(), selected)
                .unwrap();
            assert_eq!(call.recorded().execution.result, native.result);
            for witness in [&a, &b, &m, &w] {
                assert_eq!(
                    *witness.calls.borrow(),
                    if selected { vec![(5, 1)] } else { vec![] }
                );
            }
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            let mut expected_input = product_atoms(&input);
            expected_input.push(if selected { vec![1] } else { vec![] });
            assert_eq!(atoms(&prepared.input), expected_input);
            let mut expected_output = product_atoms(&native.result.chosen);
            expected_output.extend(product_atoms(&input));
            assert_eq!(atoms(&prepared.output), expected_output);
            let shape = product_alignment();
            let boolean = Alignment::singleton(AlignmentAtom::Bytes { length: 1 });
            assert_eq!(
                prepared.input.alignment,
                Alignment::concat([&shape, &boolean])
            );
            assert_eq!(
                prepared.output.alignment,
                Alignment::concat([&shape, &shape])
            );
        }
    }
}
#[test]
fn bad_product_leaves_refuse_before_callbacks_and_bad_output_refuses_after_callback() {
    for leaf in 0..5 {
        let (state, _) = setup();
        let mut input = input("canonical", &[1]);
        match leaf {
            0 => input.pair.0.0.push('\0'),
            1 => input.pair.1.0.push(0),
            2 => input.items.0[0].0.push(0),
            3 => input.items.0[1].0.push(0),
            _ => input.nested.text.0.push('\0'),
        }
        let (a, b, w) = (Witness::new(&[9]), Witness::new(&[9]), Witness::new(&[9]));
        let native = c::run(state.circuit_context(5), &a, input.clone(), true).unwrap();
        let recorded = c::recorded::run(state.circuit_context(5), &b, input.clone(), true).unwrap();
        assert_eq!(native.result, recorded.execution.result);
        assert_eq!(native.result.original, input);
        let contract = c::Contract::from(w.clone());
        assert_eq!(
            contract
                .recording()
                .run_call(&state, 5, input, true)
                .err()
                .unwrap(),
            invalid()
        );
        assert!(w.calls.borrow().is_empty());
    }
    for bad in [vec![0], vec![9, 0], vec![9, 0, 0]] {
        let (state, _) = setup();
        let input = input("canonical", &[1]);
        let (a, b, w) = (Witness::new(&bad), Witness::new(&bad), Witness::new(&bad));
        let native = c::run(state.circuit_context(5), &a, input.clone(), true).unwrap();
        let recorded = c::recorded::run(state.circuit_context(5), &b, input.clone(), true).unwrap();
        assert_eq!(native.result, recorded.execution.result);
        assert_eq!(native.result.chosen.items.0[1].0, bad);
        let contract = c::Contract::from(w.clone());
        assert_eq!(
            contract
                .recording()
                .run_call(&state, 5, input, true)
                .err()
                .unwrap(),
            invalid()
        );
        assert_eq!(*w.calls.borrow(), vec![(5, 1)]);
    }
}
