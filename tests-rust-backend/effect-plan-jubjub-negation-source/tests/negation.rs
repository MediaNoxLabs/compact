// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::setup, values::*, witness::*, *};
use compact_rust_effect_plan_jubjub_negation_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use r::context::CircuitResult;
use r::recording::RecordedCircuitResult;
use r::transaction::{ObservedContractState, RecordedCall, VerifierKey};
struct Executions<'a, T> {
    native: Result<CircuitResult<u64, T>, r::CompactError>,
    recorded: Result<RecordedCircuitResult<u64, T>, r::CompactError>,
    manual: Result<RecordedCircuitResult<u64, T>, r::CompactError>,
    observed: Result<RecordedCall<'a, u64, T>, r::CompactError>,
}
fn qualify<T: Clone + std::fmt::Debug + PartialEq + Into<r::fab::AlignedValue>>(
    reference: &serde_json::Value,
    witnesses: [&Witness; 4],
    input: r::fab::AlignedValue,
    executions: Executions<'_, T>,
    key: VerifierKey,
    state: &ObservedContractState,
) {
    let id = reference["id"].as_str().unwrap();
    let is_close = reference["circuit"] == "close";
    let selected = is_close || reference["selected"].as_bool().unwrap();
    let w = witnesses[0];
    let mut events = vec![];
    let mut error = None;
    if selected {
        events.push(("point", 5, 1));
        if w.fail == Some("point") {
            error = Some(provider_failure("point"));
        }
    }
    if error.is_none() && is_close {
        let negated = coordinates(&reference["negated"]);
        let expected = point(&reference["expected"]);
        if negated.x != expected.x().unwrap_or(r::Field::from(0_u64)) {
            error = Some(r::CompactError::AssertionFailed("wrong x".into()));
        } else if negated.y != expected.y().unwrap_or(r::Field::from(1_u64)) {
            error = Some(r::CompactError::AssertionFailed("wrong y".into()));
        }
    }
    if error.is_none() {
        events.push(("permit", 5 + events.len() as u64, 1));
        if w.fail == Some("permit") {
            error = Some(provider_failure("permit"));
        } else if !w.permit {
            error = Some(r::CompactError::AssertionFailed("denied".into()));
        }
    }
    for w in witnesses {
        assert_eq!(*w.events.borrow(), events, "{id}");
    }
    assert_eq!(
        reference["events"],
        serde_json::json!(
            events
                .iter()
                .map(|(n, p, c)| serde_json::json!([n, p, c.to_string()]))
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(
        slots::calls
            .inspect(state.circuit_context(5).query.state.get_ref())
            .unwrap(),
        0
    );
    if let Some(error) = error {
        assert_eq!(reference["status"], "refused");
        assert_eq!(executions.native.err().unwrap(), error);
        assert_eq!(executions.recorded.err().unwrap(), error);
        assert_eq!(executions.manual.err().unwrap(), error);
        assert_eq!(executions.observed.err().unwrap(), error);
        return;
    }
    let native = executions.native.unwrap();
    let recorded = executions.recorded.unwrap();
    let manual = executions.manual.unwrap();
    let observed = executions.observed.unwrap();
    compare(&native, &recorded, &manual);
    compare(&native, observed.recorded(), &manual);
    assert_ts(reference, &input, &native.result.clone().into(), &native);
    let mut private = vec![];
    if selected {
        private.push(r::fab::AlignedValue::from(w.payload));
    }
    private.push(true.into());
    assert_eq!(native.private_transcript_outputs, private);
    assert_eq!(
        slots::calls
            .inspect(native.context.query.state.get_ref())
            .unwrap(),
        1
    );
    assert_eq!(reference["serializedPreimage"]["status"], "success");
    let prepared = observed.prepare(key, r::Field::from(0_u64)).unwrap();
    assert_eq!(prepared.input, input);
    assert_eq!(prepared.output, native.result.into());
}
#[test]
fn selected_coordinates_preserve_negation_laziness_and_callback_failure_order() {
    for id in [
        "coordinates-identity",
        "coordinates-nonidentity",
        "coordinates-lazy-provider-suppressed",
        "coordinates-point-refusal",
        "coordinates-permit-refusal",
        "coordinates-denied",
    ] {
        let reference = row(id);
        assert_eq!(reference["circuit"], "coordinates");
        let selected = reference["selected"].as_bool().unwrap();
        let fallback = point(&reference["fallback"]);
        assert_eq!(
            reference["args"],
            serde_json::json!([reference["fallback"], selected])
        );
        let (state, key) = setup();
        let [a, b, m, w] = std::array::from_fn(|_| Witness::new(&reference));
        let contract = c::Contract::from(w.clone());
        let executions = Executions {
            native: c::coordinates(state.circuit_context(5), &a, fallback, selected),
            recorded: c::recorded::coordinates(state.circuit_context(5), &b, fallback, selected),
            manual: manual(
                state.circuit_context(5),
                &m,
                selected,
                None,
                coordinates(&reference["negated"]),
            ),
            observed: contract
                .recording()
                .coordinates_call(&state, 5, fallback, selected),
        };
        qualify(
            &reference,
            [&a, &b, &m, &w],
            (fallback, selected).into(),
            executions,
            key,
            &state,
        );
    }
}
#[test]
fn old_positive_unit_helper_preserves_both_coordinates_and_assertion_prefix() {
    for id in [
        "close-identity",
        "close-nonidentity",
        "close-wrong-x",
        "close-point-refusal",
    ] {
        let reference = row(id);
        assert_eq!(reference["circuit"], "close");
        let expected = point(&reference["expected"]);
        assert_eq!(
            reference["args"],
            serde_json::json!([reference["expected"]])
        );
        let (state, key) = setup();
        let [a, b, m, w] = std::array::from_fn(|_| Witness::new(&reference));
        let contract = c::Contract::from(w.clone());
        let executions = Executions {
            native: c::close(state.circuit_context(5), &a, expected),
            recorded: c::recorded::close(state.circuit_context(5), &b, expected),
            manual: manual(
                state.circuit_context(5),
                &m,
                true,
                Some((coordinates(&reference["negated"]), expected)),
                (),
            ),
            observed: contract.recording().close_call(&state, 5, expected),
        };
        qualify(
            &reference,
            [&a, &b, &m, &w],
            expected.into(),
            executions,
            key,
            &state,
        );
    }
}
