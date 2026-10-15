// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::setup, values::*, witness::*, *};
use compact_rust_effect_plan_aligned_witness_results_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::context::CircuitResult;
use r::recording::RecordedCircuitResult;
use r::transaction::{
    ObservedCallError, ObservedContractState, PrepareCallError, RecordedCall, VerifierKey,
};
struct Executions<'a, T> {
    native: Result<CircuitResult<u64, T>, r::CompactError>,
    recorded: Result<RecordedCircuitResult<u64, T>, r::CompactError>,
    manual: Result<RecordedCircuitResult<u64, T>, r::CompactError>,
    observed: Result<RecordedCall<'a, u64, T>, r::CompactError>,
}
fn qualify<T: Clone + std::fmt::Debug + PartialEq + Into<r::fab::AlignedValue>>(
    reference: &serde_json::Value,
    witnesses: [&Witness; 4],
    selected: bool,
    input: r::fab::AlignedValue,
    executions: Executions<'_, T>,
    key: VerifierKey,
    state: &ObservedContractState,
) {
    let id = reference["id"].as_str().unwrap();
    let witness = witnesses[0];
    let mut events = vec![];
    let mut error = None;
    if selected {
        events.push((witness.payload.name(), 5, 1));
        if witness.fail == Some(witness.payload.name()) {
            error = Some(provider_failure(witness.payload.name()));
        }
    }
    if error.is_none() {
        events.push(("permit", 5 + events.len() as u64, 1));
        if witness.fail == Some("permit") {
            error = Some(provider_failure("permit"));
        } else if !witness.permit {
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
        private.push(witness.payload.aligned());
    }
    private.push(r::fab::AlignedValue::from(true));
    assert_eq!(native.private_transcript_outputs, private);
    assert_eq!(
        slots::calls
            .inspect(native.context.query.state.get_ref())
            .unwrap(),
        1
    );
    let refused_atom = match id {
        "text-trailing" | "entry-trailing" => Some(0),
        "nested-trailing-later" => Some(3),
        _ => None,
    };
    if let Some(atom_index) = refused_atom {
        assert_eq!(reference["serializedPreimage"]["status"], "refused");
        assert!(
            reference["serializedPreimage"]["error"]
                .as_str()
                .unwrap()
                .contains("not in normal form")
        );
        assert_eq!(
            atoms(&native.private_transcript_outputs[0])[atom_index],
            b"a\0"
        );
        assert!(
            matches!(observed.prepare(key,r::Field::from(0_u64)),Err(ObservedCallError::Prepare(PrepareCallError::NonCanonicalPrivateOutput{output_index:0,atom_index:i})) if i==atom_index)
        );
    } else {
        assert_eq!(reference["serializedPreimage"]["status"], "success");
        let prepared = observed.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, input);
        assert_eq!(prepared.output, native.result.into());
    }
}
fn execute_hash(id: &str) {
    let reference = row(id);
    let circuit = reference["circuit"].as_str().unwrap();
    let selected = reference["selected"].as_bool().unwrap();
    assert_eq!(
        reference["args"],
        serde_json::json!([reference["fallback"], selected])
    );
    let fallback = Payload::from_json(circuit, &reference["fallback"]);
    let input = r::fab::AlignedValue::concat(&[fallback.aligned(), selected.into()]);
    let (state, key) = setup();
    let [a, b, m, w] = std::array::from_fn(|_| Witness::new(&reference));
    let contract = c::Contract::from(w.clone());
    let handle = contract.recording();
    let (native, recorded, observed) = match fallback {
        Payload::Text(v) => (
            c::text(state.circuit_context(5), &a, v.clone(), selected),
            c::recorded::text(state.circuit_context(5), &b, v.clone(), selected),
            handle.text_call(&state, 5, v, selected),
        ),
        Payload::Entry(v) => (
            c::entry(state.circuit_context(5), &a, v.clone(), selected),
            c::recorded::entry(state.circuit_context(5), &b, v.clone(), selected),
            handle.entry_call(&state, 5, v, selected),
        ),
        Payload::Nested(v) => (
            c::nested(state.circuit_context(5), &a, v.clone(), selected),
            c::recorded::nested(state.circuit_context(5), &b, v.clone(), selected),
            handle.nested_call(&state, 5, v, selected),
        ),
        Payload::Tuple(_) => panic!("fixed tuple has its own typed output"),
    };
    let output = if reference["status"] == "success" {
        hashes(&reference)
    } else {
        types::Hashes::default()
    };
    let manual = manual(state.circuit_context(5), &m, selected, output);
    qualify(
        &reference,
        [&a, &b, &m, &w],
        selected,
        input,
        Executions {
            native,
            recorded,
            manual,
            observed,
        },
        key,
        &state,
    );
}
#[test]
fn opaque_result_shapes_preserve_exact_private_atoms_and_indexed_preparation_refusals() {
    for id in [
        "text-empty",
        "text-ascii",
        "text-unicode-interior",
        "text-trailing",
        "entry-canonical",
        "entry-trailing",
        "nested-canonical",
        "nested-trailing-later",
    ] {
        execute_hash(id);
    }
}
#[test]
fn counter_first_witnesses_preserve_lazy_selection_and_failure_prefixes() {
    for id in [
        "lazy-provider-suppressed",
        "selected-provider-refusal",
        "marker-provider-refusal",
        "marker-denied",
    ] {
        execute_hash(id);
    }
}
#[test]
fn standalone_tuple11_and_old_positive_local_unit_match_ts_and_prepare() {
    for id in ["tuple-zero", "tuple-mixed"] {
        let reference = row(id);
        assert_eq!(reference["circuit"], "tuple11");
        assert_eq!(reference["selected"], true);
        let Payload::Tuple(value) = Payload::from_json("tuple11", &reference["fallback"]) else {
            unreachable!()
        };
        let value = *value;
        assert_eq!(
            reference["args"],
            serde_json::json!([reference["fallback"], true])
        );
        let (state, key) = setup();
        let [a, b, m, w] = std::array::from_fn(|_| Witness::new(&reference));
        let contract = c::Contract::from(w.clone());
        let input = r::fab::AlignedValue::from((value, true));
        let executions = Executions {
            native: c::tuple11(state.circuit_context(5), &a, value, true),
            recorded: c::recorded::tuple11(state.circuit_context(5), &b, value, true),
            manual: manual(state.circuit_context(5), &m, true, digest(&reference)),
            observed: contract.recording().tuple11_call(&state, 5, value, true),
        };
        qualify(
            &reference,
            [&a, &b, &m, &w],
            true,
            input,
            executions,
            key,
            &state,
        );
    }
    let reference = row("unit-local-canonical");
    assert_eq!(reference["circuit"], "unit_text");
    assert_eq!(reference["args"], serde_json::json!([]));
    assert_eq!(reference["payload"], "unit");
    let (state, key) = setup();
    let [a, b, m, w] = std::array::from_fn(|_| Witness::new(&reference));
    let contract = c::Contract::from(w.clone());
    let executions = Executions {
        native: c::unit_text(state.circuit_context(5), &a),
        recorded: c::recorded::unit_text(state.circuit_context(5), &b),
        manual: manual(state.circuit_context(5), &m, true, ()),
        observed: contract.recording().unit_text_call(&state, 5),
    };
    qualify(
        &reference,
        [&a, &b, &m, &w],
        true,
        ().into(),
        executions,
        key,
        &state,
    );
}
