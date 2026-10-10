// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_effect_plan_opaque_public_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, EntryPointBuf,
};
use r::context::{CircuitResult, ConstructorContext, WitnessContext};
use r::ledger::ContractState;
use r::recording::RecordedCircuitResult;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct Witness {
    bytes: r::OpaqueBytes,
    calls: Rc<RefCell<Vec<(u64, u128)>>>,
}
impl Witness {
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec().into(),
            calls: Rc::default(),
        }
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn replacement(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::OpaqueBytes), r::CompactError> {
        self.calls
            .borrow_mut()
            .push((*ctx.private_state, ctx.ledger.calls()?.value()));
        Ok((*ctx.private_state + 1, self.bytes.clone()))
    }
}
fn setup() -> (ObservedContractState, VerifierKey) {
    let verifier: VerifierKey = StdRng::seed_from_u64(5551).r#gen();
    let initialized = c::initial_state(ConstructorContext::new(5_u64))
        .unwrap()
        .into_circuit_context(Default::default());
    let mut operations = midnight_storage::storage::HashMap::new();
    for entry in ["run", "text_roundtrip", "bytes_roundtrip"] {
        operations = operations.insert(
            EntryPointBuf(entry.as_bytes().to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        );
    }
    (
        ObservedContractState::new(
            Default::default(),
            ContractState::new(
                initialized.query.state.get_ref().clone(),
                operations,
                ContractMaintenanceAuthority::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        verifier,
    )
}
fn input(text: &str, bytes: &[u8]) -> types::Envelope {
    types::Envelope {
        message: types::Message {
            text: text.into(),
            bytes: bytes.to_vec().into(),
        },
    }
}
fn invalid() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("opaque ledger input has trailing zero bytes".into())
}
fn atoms(value: &r::fab::AlignedValue) -> Vec<Vec<u8>> {
    value.value.0.iter().map(|a| a.0.clone()).collect()
}
fn equal<T: std::fmt::Debug + PartialEq>(
    native: &CircuitResult<u64, T>,
    recorded: &RecordedCircuitResult<u64, T>,
) {
    assert_eq!(native.result, recorded.execution.result);
    assert_eq!(
        native.context.private_state,
        recorded.execution.context.private_state
    );
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert!(!recorded.public.verify_ops().is_empty());
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
}

#[test]
fn string_roots_preserve_raw_local_values_and_validate_observed_encoding() {
    for text in [
        "",
        "é文🦀",
        "hi\0world",
        "long message exceeding thirty-one encoded bytes",
        "\0",
        "hi\0",
        "é文\0\0",
    ] {
        let (state, key) = setup();
        let native = c::text_roundtrip(state.circuit_context(5), text.into()).unwrap();
        let recorded = c::recorded::text_roundtrip(state.circuit_context(5), text.into()).unwrap();
        equal(&native, &recorded);
        assert_eq!(native.result.value, r::OpaqueString::from(text));
        let call = c::recorded::Contract.text_roundtrip_call(&state, 5, text.into());
        if text.as_bytes().last() == Some(&0) {
            assert_eq!(call.err().unwrap(), invalid());
        } else {
            let prepared = call.unwrap().prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(atoms(&prepared.input), vec![text.as_bytes().to_vec()]);
            assert_eq!(atoms(&prepared.output), vec![text.as_bytes().to_vec()]);
        }
    }
}
#[test]
fn bytes_roots_preserve_raw_local_values_and_validate_observed_encoding() {
    for bytes in [
        vec![],
        vec![255, 254],
        vec![0, 2],
        vec![1, 0, 2],
        vec![3; 65],
        vec![0],
        vec![1, 0],
        vec![1, 0, 0],
    ] {
        let (state, key) = setup();
        let native = c::bytes_roundtrip(state.circuit_context(5), bytes.clone().into()).unwrap();
        let recorded =
            c::recorded::bytes_roundtrip(state.circuit_context(5), bytes.clone().into()).unwrap();
        equal(&native, &recorded);
        assert_eq!(native.result.value.0, bytes);
        let call = c::recorded::Contract.bytes_roundtrip_call(&state, 5, bytes.clone().into());
        if bytes.last() == Some(&0) {
            assert_eq!(call.err().unwrap(), invalid());
        } else {
            let prepared = call.unwrap().prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(atoms(&prepared.input), vec![bytes.clone()]);
            assert_eq!(atoms(&prepared.output), vec![bytes]);
        }
    }
}
#[test]
fn nested_helpers_reuse_owned_arguments_and_prepare_exact_atoms_for_both_branches() {
    for (text, bytes) in [
        ("", vec![]),
        ("é文🦀", vec![0, 255]),
        ("hi\0world", vec![1, 0, 2]),
    ] {
        for selected in [false, true] {
            let (state, key) = setup();
            let original = input(text, &bytes);
            // The unselected witness carries invalid bytes and must stay lazy.
            let replacement = if selected { vec![9, 0, 8] } else { vec![9, 0] };
            let (a, b, w) = (
                Witness::new(&replacement),
                Witness::new(&replacement),
                Witness::new(&replacement),
            );
            let native = c::run(state.circuit_context(5), &a, original.clone(), selected).unwrap();
            let recorded =
                c::recorded::run(state.circuit_context(5), &b, original.clone(), selected).unwrap();
            equal(&native, &recorded);
            let contract = c::Contract::from(w.clone());
            let call = contract
                .recording()
                .run_call(&state, 5, original.clone(), selected)
                .unwrap();
            assert_eq!(call.recorded().execution.result, native.result);
            assert_eq!(native.result.original, original);
            let chosen = if selected { replacement } else { bytes.clone() };
            assert_eq!(native.result.chosen, input(text, &chosen));
            let events = if selected { vec![(5, 1)] } else { vec![] };
            for witness in [&a, &b, &w] {
                assert_eq!(*witness.calls.borrow(), events);
            }
            assert_eq!(
                slots::calls
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                1
            );
            assert_eq!(native.context.private_state, if selected { 6 } else { 5 });
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(
                atoms(&prepared.input),
                vec![
                    text.as_bytes().to_vec(),
                    bytes.clone(),
                    if selected { vec![1] } else { vec![] }
                ]
            );
            assert_eq!(
                atoms(&prepared.output),
                vec![
                    text.as_bytes().to_vec(),
                    chosen,
                    text.as_bytes().to_vec(),
                    bytes.clone()
                ]
            );
        }
    }
}
#[test]
fn invalid_nested_input_stops_before_witness_while_invalid_output_stops_after_it() {
    for original in [
        input("bad\0", &[1]),
        input("good", &[1, 0]),
        input("bad\0", &[0]),
    ] {
        let (state, _) = setup();
        let witness = Witness::new(&[9]);
        let contract = c::Contract::from(witness.clone());
        assert_eq!(
            contract
                .recording()
                .run_call(&state, 5, original, true)
                .err()
                .unwrap(),
            invalid()
        );
        assert!(witness.calls.borrow().is_empty());
        assert_eq!(
            slots::calls
                .inspect(state.contract().data.get_ref())
                .unwrap(),
            0
        );
    }
    for bad in [vec![0], vec![9, 0], vec![9, 0, 0]] {
        let (state, _) = setup();
        let original = input("canonical", &[1]);
        let (a, b, w) = (Witness::new(&bad), Witness::new(&bad), Witness::new(&bad));
        let native = c::run(state.circuit_context(5), &a, original.clone(), true).unwrap();
        let recorded =
            c::recorded::run(state.circuit_context(5), &b, original.clone(), true).unwrap();
        equal(&native, &recorded);
        assert_eq!(native.result.chosen.message.bytes.0, bad);
        assert_eq!(recorded.execution.result.chosen.message.bytes.0, bad);
        let contract = c::Contract::from(w.clone());
        assert_eq!(
            contract
                .recording()
                .run_call(&state, 5, original, true)
                .err()
                .unwrap(),
            invalid()
        );
        for witness in [&a, &b, &w] {
            assert_eq!(*witness.calls.borrow(), vec![(5, 1)]);
        }
        // Public output guard runs after local execution. The existing witness
        // FAB conversion policy is unchanged and is not claimed lossless here.
        assert_eq!(
            native.private_transcript_outputs,
            vec![r::fab::AlignedValue::from(r::OpaqueBytes::from(bad))]
        );
    }
}
