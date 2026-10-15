// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_witness_domain_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, support::*,
};
use r::{Field, OpaqueBytes};

fn witness(payload: Vec<u8>, refuse: Option<&'static str>) -> Witness {
    Witness {
        payload: OpaqueBytes::from(payload),
        refuse,
        ..Default::default()
    }
}
#[test]
fn owned_witness_results_preserve_order_private_outputs_and_public_replay() {
    for selected in [false, true] {
        for payload in [vec![], vec![0, 1, 255], vec![255; 128]] {
            let a = witness(payload.clone(), None);
            let b = witness(payload.clone(), None);
            let m = witness(payload, None);
            let native = c::run(context(), &a, selected).unwrap();
            let recorded = c::recorded::run(context(), &b, selected).unwrap();
            let manual = reference(context(), selected, &m).unwrap();
            assert!(native.result);
            assert_eq!(
                native.private_transcript_outputs,
                private_outputs(&a, selected)
            );
            let tags: &[&str] = if selected {
                &["bytes", "vector", "tuple", "wide", "packet"]
            } else {
                &["bytes"]
            };
            assert_eq!(
                a.calls
                    .borrow()
                    .iter()
                    .map(|event| event.tag)
                    .collect::<Vec<_>>(),
                tags
            );
            for (index, event) in a.calls.borrow().iter().enumerate() {
                assert_eq!(event.private, 7 + index as u64);
                assert_eq!(event.marker, Field::from(1_u64));
                assert_eq!(
                    event.blob,
                    if index == 0 {
                        OpaqueBytes::default()
                    } else {
                        a.payload.clone()
                    }
                );
            }
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
                assert_eq!(execution.context.private_state, 7 + tags.len() as u64);
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
                if selected {
                    r::transient_hash(vector_value())
                } else {
                    Field::from(1_u64)
                }
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
fn callback_refusal_stops_at_the_selected_witness_after_prior_writes() {
    let tags = ["bytes", "vector", "tuple", "wide", "packet"];
    for (index, tag) in tags.into_iter().enumerate() {
        for selected in [false, true] {
            let a = witness(vec![7, 255], Some(tag));
            let b = witness(vec![7, 255], Some(tag));
            let m = witness(vec![7, 255], Some(tag));
            let native = c::run(context(), &a, selected);
            let recorded = c::recorded::run(context(), &b, selected);
            let manual = reference(context(), selected, &m);
            if selected || index == 0 {
                let expected = r::CompactError::AssertionFailed(format!("{tag} refused"));
                assert_eq!(native.err().unwrap(), expected);
                assert_eq!(recorded.err().unwrap(), expected);
                assert_eq!(manual.err().unwrap(), expected);
                assert_eq!(
                    a.calls
                        .borrow()
                        .iter()
                        .map(|event| event.tag)
                        .collect::<Vec<_>>(),
                    &tags[..=index]
                );
            } else {
                assert!(
                    native.unwrap().result
                        && recorded.unwrap().execution.result
                        && manual.unwrap().execution.result
                );
                assert_eq!(a.calls.borrow().len(), 1);
            }
            assert_eq!(*a.calls.borrow(), *b.calls.borrow());
            assert_eq!(*a.calls.borrow(), *m.calls.borrow());
            for (i, event) in a.calls.borrow().iter().enumerate() {
                assert_eq!(event.private, 7 + i as u64);
                assert_eq!(event.marker, Field::from(1_u64));
                assert_eq!(
                    event.blob,
                    if i == 0 {
                        OpaqueBytes::default()
                    } else {
                        a.payload.clone()
                    }
                );
            }
        }
    }
}
