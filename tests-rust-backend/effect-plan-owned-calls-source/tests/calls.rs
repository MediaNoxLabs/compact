// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_owned_calls_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, support::*,
};
use r::{Field, OpaqueBytes};
fn witness(payload: Vec<u8>, refuse_call: Option<usize>) -> Witness {
    Witness {
        payload: OpaqueBytes::from(payload),
        refuse_call,
        ..Default::default()
    }
}
fn assert_calls(witness: &Witness, selected: bool, count: usize) {
    let markers = expected_markers(selected);
    let calls = witness.calls.borrow();
    assert_eq!(calls.len(), count);
    for (i, event) in calls.iter().enumerate() {
        assert_eq!(event.tag, if i == 0 { "bytes" } else { "trace" });
        assert_eq!(event.private, 7 + i as u64);
        assert_eq!(event.marker, Field::from(markers[i]));
        assert_eq!(
            event.blob,
            if i < 7 {
                OpaqueBytes::default()
            } else {
                witness.payload.clone()
            }
        );
    }
}
#[test]
fn owned_internal_interfaces_preserve_nested_argument_order_and_alias_reuse() {
    for selected in [false, true] {
        for payload in [vec![], vec![0, 1, 255], vec![255; 128]] {
            let a = witness(payload.clone(), None);
            let b = witness(payload.clone(), None);
            let m = witness(payload, None);
            let native = c::run(context(), &a, selected, true).unwrap();
            let recorded = c::recorded::run(context(), &b, selected, true).unwrap();
            let manual = reference(context(), selected, true, &m).unwrap();
            assert_eq!(native.result, Field::from(1_u64));
            assert_eq!(
                native.private_transcript_outputs,
                private_outputs(&a, selected)
            );
            let count = if selected { 11 } else { 7 };
            assert_calls(&a, selected, count);
            assert_eq!(*a.calls.borrow(), *b.calls.borrow());
            assert_eq!(*a.calls.borrow(), *m.calls.borrow());
            for execution in [&recorded.execution, &manual.execution] {
                assert_eq!(execution.result, native.result);
                assert_eq!(execution.context.query.state, native.context.query.state);
                assert_eq!(
                    execution.context.query.effects,
                    native.context.query.effects
                );
                assert_eq!(execution.gas_cost, native.gas_cost);
                assert_eq!(
                    execution.private_transcript_outputs,
                    native.private_transcript_outputs
                );
                assert_eq!(execution.context.private_state, 7 + count as u64);
            }
            assert_eq!(
                slots::blob
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                a.payload
            );
            assert_eq!(
                slots::marker
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                Field::from(if selected { 8_u64 } else { 6 })
            );
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
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, reference.gas_cost);
        }
    }
}
#[test]
fn every_callback_failure_preserves_source_prefix_and_branch_skipping() {
    for selected in [false, true] {
        for index in 0..11 {
            let a = witness(vec![7, 255], Some(index));
            let b = witness(vec![7, 255], Some(index));
            let m = witness(vec![7, 255], Some(index));
            let native = c::run(context(), &a, selected, true);
            let recorded = c::recorded::run(context(), &b, selected, true);
            let manual = reference(context(), selected, true, &m);
            let count = if selected { 11 } else { 7 };
            if index < count {
                let expected =
                    r::CompactError::AssertionFailed(format!("callback {index} refused"));
                assert_eq!(native.err().unwrap(), expected);
                assert_eq!(recorded.err().unwrap(), expected);
                assert_eq!(manual.err().unwrap(), expected);
                assert_calls(&a, selected, index + 1);
            } else {
                assert!(
                    native.unwrap().result == Field::from(1_u64)
                        && recorded.unwrap().execution.result == Field::from(1_u64)
                        && manual.unwrap().execution.result == Field::from(1_u64)
                );
                assert_calls(&a, selected, count);
            }
            assert_eq!(*a.calls.borrow(), *b.calls.borrow());
            assert_eq!(*a.calls.borrow(), *m.calls.borrow());
        }
    }
}
#[test]
fn owned_helper_assertion_runs_after_its_callback_and_is_skipped_with_branch() {
    for selected in [false, true] {
        let a = witness(vec![7, 255], None);
        let b = witness(vec![7, 255], None);
        let m = witness(vec![7, 255], None);
        let native = c::run(context(), &a, selected, false);
        let recorded = c::recorded::run(context(), &b, selected, false);
        let manual = reference(context(), selected, false, &m);
        if selected {
            let expected = r::CompactError::AssertionFailed("owned stateful helper refused".into());
            assert_eq!(native.err().unwrap(), expected);
            assert_eq!(recorded.err().unwrap(), expected);
            assert_eq!(manual.err().unwrap(), expected);
        } else {
            assert!(
                native.unwrap().result == Field::from(1_u64)
                    && recorded.unwrap().execution.result == Field::from(1_u64)
                    && manual.unwrap().execution.result == Field::from(1_u64)
            );
        }
        assert_calls(&a, selected, if selected { 11 } else { 7 });
        assert_eq!(*a.calls.borrow(), *b.calls.borrow());
        assert_eq!(*a.calls.borrow(), *m.calls.borrow());
    }
}
