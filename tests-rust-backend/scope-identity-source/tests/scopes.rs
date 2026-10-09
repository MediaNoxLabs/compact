// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_scope_identity_source_fixture::{
    ledger_contract as c, ledger_slots, pure_circuits, runtime as r,
};
use r::context::{CircuitContext, ConstructorContext};
use r::ledger::{CoinCommitment, CoinNullifier, HashOutput, KernelClaim};
use r::recording::RecordingFrame;
use r::{BoundedUint, Field, FixedBytes};
fn context(seed: u16) -> CircuitContext<()> {
    c::initial_state(
        ConstructorContext::new(()),
        BoundedUint::new(seed as u128).unwrap(),
    )
    .unwrap()
    .into_circuit_context(Default::default())
}
#[test]
fn terminal_shadow_restores_outer_parameter_for_native_and_recorded() {
    let outer = FixedBytes::new([1; 32]);
    let inner = FixedBytes::new([2; 32]);
    let native = c::shadow(context(5), outer, inner).unwrap();
    let recorded = c::recorded::shadow(context(5), outer, inner).unwrap();
    let manual = RecordingFrame::new(context(5))
        .kernel_claim(KernelClaim::Nullifier(CoinNullifier(HashOutput(inner.0))))
        .unwrap()
        .kernel_claim(KernelClaim::CoinSpend(CoinCommitment(HashOutput(outer.0))))
        .unwrap()
        .finish(());
    for execution in [&native, &recorded.execution] {
        assert_eq!(
            execution.context.query.effects,
            manual.execution.context.query.effects
        );
        assert_eq!(
            execution.context.query.state,
            manual.execution.context.query.state
        );
        assert_eq!(execution.gas_cost, manual.execution.gas_cost);
    }
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(manual.public.verify_ops()).unwrap()
    );
}
#[test]
fn genuine_local_return_and_sibling_nested_suffix_scopes_match_manual_writes() {
    for name in ["local_return", "siblings", "nested", "suffix"] {
        let value = Field::from(11_u64);
        let (native, recorded, writes, expected) = match name {
            "local_return" => (
                c::local_return(context(5), value).unwrap(),
                c::recorded::local_return(context(5), value).unwrap(),
                vec![12],
                12,
            ),
            "siblings" => (
                c::siblings(context(5), value).unwrap(),
                c::recorded::siblings(context(5), value).unwrap(),
                vec![2, 3],
                11,
            ),
            "nested" => (
                c::nested(context(5), value).unwrap(),
                c::recorded::nested(context(5), value).unwrap(),
                vec![3, 2],
                11,
            ),
            "suffix" => (
                c::suffix(
                    context(5),
                    value,
                    Field::from(7_u64),
                    Field::from(5_u64),
                    Field::from(3_u64),
                )
                .unwrap(),
                c::recorded::suffix(
                    context(5),
                    value,
                    Field::from(7_u64),
                    Field::from(5_u64),
                    Field::from(3_u64),
                )
                .unwrap(),
                vec![7, 3],
                19,
            ),
            _ => unreachable!(),
        };
        let mut frame = RecordingFrame::new(context(5));
        for value in writes {
            frame = ledger_slots::marker
                .record_write(frame, Field::from(value as u64))
                .unwrap();
        }
        let manual = frame.finish(Field::from(expected as u64));
        for execution in [&native, &recorded.execution] {
            assert_eq!(execution.result, manual.execution.result, "{name}");
            assert_eq!(
                execution.context.query.state, manual.execution.context.query.state,
                "{name}"
            );
            assert_eq!(
                execution.context.query.effects, manual.execution.context.query.effects,
                "{name}"
            );
            assert_eq!(execution.gas_cost, manual.execution.gas_cost, "{name}");
        }
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            serde_json::to_value(manual.public.verify_ops()).unwrap(),
            "{name}"
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
        let reference = manual
            .public
            .initial()
            .query(
                manual.public.verify_ops(),
                None,
                &manual.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(replay.context.state, native.context.query.state);
        assert_eq!(replay.gas_cost, reference.gas_cost);
    }
}
#[test]
fn constructor_loop_items_and_counter_actions_keep_formal_identity() {
    for seed in [0_u16, 5, 65535] {
        assert_eq!(
            c::read_count(context(seed)).unwrap().result.value(),
            seed as u128 + 9
        );
        let amount = BoundedUint::new(17).unwrap();
        let native = c::counter_scopes(context(seed), amount).unwrap();
        let recorded = c::recorded::counter_scopes(context(seed), amount).unwrap();
        assert_eq!(native.result.value(), seed as u128 + 28);
        assert_eq!(recorded.execution.result, native.result);
        assert_eq!(
            recorded.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    }
}
#[test]
fn pure_fold_and_dollar_spelling_collisions_preserve_values() {
    for seed in [0_u64, 11, 93] {
        assert_eq!(
            pure_circuits::alias_a(Field::from(seed), Field::from(7_u64)).unwrap(),
            Field::from(seed)
        );
        assert_eq!(
            pure_circuits::alias_b(Field::from(seed), Field::from(7_u64)).unwrap(),
            Field::from(seed)
        );
        assert_eq!(
            pure_circuits::folded(Field::from(seed)).unwrap(),
            Field::from(seed + 3)
        );
        assert_eq!(
            pure_circuits::dollars(Field::from(seed), Field::from(7_u64)).unwrap(),
            Field::from(seed + 7)
        );
    }
}
