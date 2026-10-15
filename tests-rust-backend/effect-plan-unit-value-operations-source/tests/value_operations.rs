// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::setup, values::*, witness::*, *};
use compact_rust_effect_plan_unit_value_operations_source_fixture::{
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
    let selected = reference["circuit"] == "close" || reference["selected"] == true;
    let error = match id {
        "evaluate-x-refusal" => Some(provider_failure("coordinate_x")),
        "evaluate-y-refusal" => Some(provider_failure("coordinate_y")),
        "evaluate-permit-refusal" => Some(provider_failure("permit")),
        "evaluate-denied" => Some(r::CompactError::AssertionFailed("denied".into())),
        "evaluate-invalid-coordinates" => Some(r::CompactError::InvalidJubjubPoint),
        "close-wrong-x" => Some(r::CompactError::AssertionFailed("wrong x".into())),
        "close-wrong-default" => Some(r::CompactError::AssertionFailed("wrong default".into())),
        _ => None,
    };
    let mut events: Vec<Event> = vec![];
    if selected {
        events.push(("coordinate_x", 5, 1));
        if id != "evaluate-x-refusal" {
            events.push(("coordinate_y", 6, 1));
        }
    }
    if !matches!(
        id,
        "evaluate-x-refusal"
            | "evaluate-y-refusal"
            | "evaluate-invalid-coordinates"
            | "close-wrong-x"
            | "close-wrong-default"
    ) {
        events.push(("permit", 5 + events.len() as u64, 1));
    }
    for w in witnesses {
        assert_eq!(*w.events.borrow(), events, "{id}");
    }
    let json_events = serde_json::json!(
        events
            .iter()
            .map(|(n, p, c)| serde_json::json!([n, p, c.to_string()]))
            .collect::<Vec<_>>()
    );
    if id == "evaluate-invalid-coordinates" {
        // Existing native boundary: official TS only stores (1,1), so its later
        // permit runs and preimage serializes. Rust refuses before that callback.
        assert_eq!(reference["status"], "success");
        assert_eq!(reference["serializedPreimage"]["status"], "success");
        assert_eq!(reference["point"], serde_json::json!({"x":"1","y":"1"}));
        assert_eq!(
            reference["events"],
            serde_json::json!([
                ["coordinate_x", 5, "1"],
                ["coordinate_y", 6, "1"],
                ["permit", 7, "1"]
            ])
        );
    } else {
        assert_eq!(reference["events"], json_events, "{id}");
    }
    assert_eq!(
        slots::calls
            .inspect(state.circuit_context(5).query.state.get_ref())
            .unwrap(),
        0
    );
    if let Some(error) = error {
        if id != "evaluate-invalid-coordinates" {
            assert_eq!(reference["status"], "refused");
        }
        assert_eq!(executions.native.err().unwrap(), error, "{id}");
        assert_eq!(executions.recorded.err().unwrap(), error, "{id}");
        assert_eq!(executions.manual.err().unwrap(), error, "{id}");
        assert_eq!(executions.observed.err().unwrap(), error, "{id}");
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
        private.push(witnesses[0].x.into());
        private.push(witnesses[0].y.into());
    }
    private.push(true.into());
    assert_eq!(native.private_transcript_outputs, private, "{id}");
    assert_eq!(
        slots::calls
            .inspect(native.context.query.state.get_ref())
            .unwrap(),
        1
    );
    let prepared = observed.prepare(key, r::Field::from(0_u64)).unwrap();
    assert_eq!(prepared.input, input);
    assert_eq!(prepared.output, native.result.into());
}
#[test]
fn checked_value_operations_preserve_modular_arithmetic_laziness_and_error_prefixes() {
    for id in [
        "evaluate-identity",
        "evaluate-nonidentity",
        "evaluate-modular-wrap",
        "evaluate-lazy-suppressed",
        "evaluate-invalid-coordinates",
        "evaluate-x-refusal",
        "evaluate-y-refusal",
        "evaluate-permit-refusal",
        "evaluate-denied",
    ] {
        let reference = row(id);
        assert_eq!(reference["circuit"], "evaluate");
        let offset = field(&reference["offset"]);
        let fallback = point(&reference["fallback"]);
        let selected = reference["selected"].as_bool().unwrap();
        assert_eq!(
            reference["args"],
            serde_json::json!([reference["offset"], reference["fallback"], selected])
        );
        if id == "evaluate-modular-wrap" {
            assert_eq!(field(&reference["x"]), r::Field::from(0_u64));
            assert_ne!(offset, r::Field::from(0_u64));
        }
        let (state, key) = setup();
        let [a, b, m, w] = std::array::from_fn(|_| Witness::new(&reference));
        let contract = c::Contract::from(w.clone());
        let executions = Executions {
            native: c::evaluate(state.circuit_context(5), &a, offset, fallback, selected),
            recorded: c::recorded::evaluate(
                state.circuit_context(5),
                &b,
                offset,
                fallback,
                selected,
            ),
            manual: manual(state.circuit_context(5), &m, &reference, output(&reference)),
            observed: contract
                .recording()
                .evaluate_call(&state, 5, offset, fallback, selected),
        };
        qualify(
            &reference,
            [&a, &b, &m, &w],
            (offset, fallback, selected).into(),
            executions,
            key,
            &state,
        );
    }
}
#[test]
fn retained_unit_helper_keeps_constructor_assertions_and_default_guard_order() {
    for id in ["close-nonidentity", "close-wrong-x", "close-wrong-default"] {
        let reference = row(id);
        assert_eq!(reference["circuit"], "close");
        let offset = field(&reference["offset"]);
        let expected = point(&reference["expected"]);
        let blank = bytes(&reference["blank"]);
        assert_eq!(
            reference["args"],
            serde_json::json!([
                reference["offset"],
                reference["expected"],
                reference["blank"]
            ])
        );
        let (state, key) = setup();
        let [a, b, m, w] = std::array::from_fn(|_| Witness::new(&reference));
        let contract = c::Contract::from(w.clone());
        let executions = Executions {
            native: c::close(state.circuit_context(5), &a, offset, expected, blank),
            recorded: c::recorded::close(state.circuit_context(5), &b, offset, expected, blank),
            manual: manual(state.circuit_context(5), &m, &reference, ()),
            observed: contract
                .recording()
                .close_call(&state, 5, offset, expected, blank),
        };
        qualify(
            &reference,
            [&a, &b, &m, &w],
            (offset, expected, blank).into(),
            executions,
            key,
            &state,
        );
    }
}
