// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::*, values::*, witness::*, *};
use compact_rust_effect_plan_exact_hash_input_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use r::transaction::{ObservedCallError, PrepareCallError};
struct Scenario<'a> {
    id: &'a str,
    payload: &'a [u8],
    fallback: &'a [u8],
    selected: bool,
    permit: bool,
    fail: Option<&'static str>,
}
impl Scenario<'_> {
    fn outcome(&self) -> (Vec<Event>, Option<r::CompactError>) {
        let mut events = vec![];
        if self.selected {
            events.push(("bytes", 5, 1));
            if self.fail == Some("bytes") {
                return (events, Some(provider_failure("bytes")));
            }
        }
        events.push(("permit", 5 + events.len() as u64, 1));
        let error = if self.fail == Some("permit") {
            Some(provider_failure("permit"))
        } else if !self.permit {
            Some(r::CompactError::AssertionFailed("denied".into()))
        } else {
            None
        };
        (events, error)
    }
}
fn execute(s: Scenario<'_>) {
    let reference = row(s.id);
    assert_eq!(reference["circuit"], "witnessed");
    assert_eq!(reference["payload"], serde_json::json!(s.payload));
    assert_eq!(reference["fallback"], serde_json::json!(s.fallback));
    assert_eq!(reference["selected"], s.selected);
    assert_eq!(reference["permit"], s.permit);
    assert_eq!(reference["fail"], serde_json::json!(s.fail));
    assert_eq!(reference["envelope"], serde_json::Value::Null);
    assert_eq!(
        reference["args"],
        serde_json::json!([s.fallback, s.selected])
    );
    let (state, key) = setup();
    let [a, b, m, w] = std::array::from_fn(|_| Witness::new(s.payload, s.permit, s.fail));
    let argument = r::OpaqueBytes(s.fallback.to_vec());
    let native = c::witnessed(state.circuit_context(5), &a, argument.clone(), s.selected);
    let recorded =
        c::recorded::witnessed(state.circuit_context(5), &b, argument.clone(), s.selected);
    let manual = manual(
        state.circuit_context(5),
        &m,
        s.selected,
        if reference["status"] == "success" {
            expected(&reference)
        } else {
            empty()
        },
    );
    let contract = c::Contract::from(w.clone());
    let observed = contract
        .recording()
        .witnessed_call(&state, 5, argument.clone(), s.selected);
    let (events, error) = s.outcome();
    for witness in [&a, &b, &m] {
        assert_eq!(*witness.events.borrow(), events, "{}", s.id);
    }
    assert_eq!(
        reference["events"],
        serde_json::json!(
            events
                .iter()
                .map(|(name, p, count)| serde_json::json!([name, p, count.to_string()]))
                .collect::<Vec<_>>()
        )
    );
    let public_refusal = s.fallback.last() == Some(&0);
    assert_eq!(
        *w.events.borrow(),
        if public_refusal {
            vec![]
        } else {
            events.clone()
        }
    );
    assert_eq!(
        slots::calls
            .inspect(state.circuit_context(5).query.state.get_ref())
            .unwrap(),
        0
    );
    if let Some(error) = error {
        assert_eq!(reference["status"], "refused");
        assert_eq!(native.err().unwrap(), error);
        assert_eq!(recorded.err().unwrap(), error);
        assert_eq!(manual.err().unwrap(), error);
        assert_eq!(observed.err().unwrap(), error);
        return;
    }
    let (native, recorded, manual) = (native.unwrap(), recorded.unwrap(), manual.unwrap());
    compare(&native, &recorded, &manual);
    assert_ts_execution(&reference, &native.result.clone().into(), &native);
    assert_eq!(native.result, expected(&reference));
    assert_eq!(
        slots::calls
            .inspect(native.context.query.state.get_ref())
            .unwrap(),
        1
    );
    if public_refusal {
        assert_eq!(
            observed.err().unwrap(),
            r::CompactError::InvalidLedgerCell(
                "opaque ledger input has trailing zero bytes".into()
            )
        );
        assert_eq!(reference["serializedPreimage"]["status"], "refused");
        return;
    }
    let observed = observed.unwrap();
    compare(&native, observed.recorded(), &manual);
    if s.selected && s.payload.last() == Some(&0) {
        assert_eq!(reference["serializedPreimage"]["status"], "refused");
        assert_eq!(
            atoms(&native.private_transcript_outputs[0]),
            vec![s.payload.to_vec()]
        );
        assert!(matches!(
            observed.prepare(key, r::Field::from(0_u64)),
            Err(ObservedCallError::Prepare(
                PrepareCallError::NonCanonicalPrivateOutput {
                    output_index: 0,
                    atom_index: 0
                }
            ))
        ));
    } else {
        assert_eq!(reference["serializedPreimage"]["status"], "success");
        let prepared = observed.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(
            prepared.input,
            r::fab::AlignedValue::from((argument, s.selected))
        );
        assert_eq!(
            prepared.output,
            r::fab::AlignedValue::from(native.result.clone())
        );
        assert_ts(&reference, &prepared.input, &prepared.output, &native);
    }
}
#[test]
fn intermediate_bytes_keep_exact_hashes_and_private_outputs_without_reinvocation() {
    for (id, payload) in [
        ("empty", &[][..]),
        ("zero", &[0][..]),
        ("interior", &[1, 0, 2][..]),
        ("trailing", &[1, 0][..]),
    ] {
        execute(Scenario {
            id: &format!("witness-{id}"),
            payload,
            fallback: &[],
            selected: true,
            permit: true,
            fail: None,
        });
    }
    execute(Scenario {
        id: "lazy-provider-suppressed",
        payload: &[1, 0],
        fallback: &[1, 0, 2],
        selected: false,
        permit: true,
        fail: Some("bytes"),
    });
    execute(Scenario {
        id: "lazy-trailing-fallback",
        payload: &[],
        fallback: &[1, 0],
        selected: false,
        permit: true,
        fail: None,
    });
}
#[test]
fn witness_refusals_and_guard_keep_exact_counter_first_prefix() {
    for selected in [false, true] {
        execute(Scenario {
            id: &format!("denied-{selected}"),
            payload: &[],
            fallback: &[],
            selected,
            permit: false,
            fail: None,
        });
        execute(Scenario {
            id: &format!("permit-refusal-{selected}"),
            payload: &[],
            fallback: &[],
            selected,
            permit: true,
            fail: Some("permit"),
        });
    }
    execute(Scenario {
        id: "bytes-refusal",
        payload: &[],
        fallback: &[],
        selected: true,
        permit: true,
        fail: Some("bytes"),
    });
}
#[test]
fn nested_products_and_vectors_keep_exact_atoms_for_all_three_hashes() {
    for (id, text, bytes, nonce, samples) in [
        ("empty", "", &[][..], 0, [&[][..], &[][..]]),
        (
            "canonical",
            "λ\0key",
            &[1, 0, 2][..],
            1,
            [&[2][..], &[3, 0, 4][..]],
        ),
        ("text-trailing", "a\0", &[1][..], 2, [&[2][..], &[3][..]]),
        ("bytes-trailing", "a", &[1, 0][..], 2, [&[2][..], &[3][..]]),
        ("vector-trailing", "a", &[1][..], 2, [&[2][..], &[3, 0][..]]),
    ] {
        let reference = row(&format!("nested-{id}"));
        let argument = envelope(text, bytes, nonce, samples);
        let public = serde_json::json!({"payload":{"text":text,"bytes":bytes},"nonce":nonce.to_string(),"samples":samples});
        assert_eq!(reference["circuit"], "nested");
        assert_eq!(reference["envelope"], public);
        assert_eq!(reference["args"], serde_json::json!([public]));
        assert_eq!(reference["events"], serde_json::json!([]));
        let (state, key) = setup();
        let native = c::nested(state.circuit_context(5), argument.clone()).unwrap();
        let recorded = c::recorded::nested(state.circuit_context(5), argument.clone()).unwrap();
        let manual = slots::calls
            .record_increment(
                r::recording::RecordingFrame::new(state.circuit_context(5)),
                1,
            )
            .unwrap()
            .finish(expected(&reference));
        compare(&native, &recorded, &manual);
        assert_ts_execution(&reference, &native.result.clone().into(), &native);
        let contract = c::Contract::from(());
        let call = contract
            .recording()
            .nested_call(&state, 5, argument.clone());
        let refused = text.as_bytes().last() == Some(&0)
            || bytes.last() == Some(&0)
            || samples.iter().any(|v| v.last() == Some(&0));
        if refused {
            assert_eq!(
                call.err().unwrap(),
                r::CompactError::InvalidLedgerCell(
                    "opaque ledger input has trailing zero bytes".into()
                )
            );
            assert_eq!(reference["serializedPreimage"]["status"], "refused");
        } else {
            let call = call.unwrap();
            compare(&native, call.recorded(), &manual);
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(prepared.input, r::fab::AlignedValue::from(argument));
            assert_eq!(
                prepared.output,
                r::fab::AlignedValue::from(native.result.clone())
            );
            assert_ts(&reference, &prepared.input, &prepared.output, &native);
            assert_eq!(reference["serializedPreimage"]["status"], "success");
        }
        assert_eq!(
            slots::calls
                .inspect(state.circuit_context(5).query.state.get_ref())
                .unwrap(),
            0
        );
        assert_eq!(
            slots::calls
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            1
        );
    }
}
