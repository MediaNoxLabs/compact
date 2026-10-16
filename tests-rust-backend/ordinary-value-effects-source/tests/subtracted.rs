// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_ordinary_value_effects_source_fixture::{ledger_contract as c, ledger_slots};
use midnight_compact_runtime as r;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::transaction::{Observation, ObservedContractState, VerifierKey};
type Uint16 = r::BoundedUint<65535>;

fn setup(count: u16) -> (ObservedContractState, VerifierKey) {
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    // Supplied state isolates the source branch; this is not a constructor proof.
    let seeded = ledger_slots::count
        .increment(initial.into_circuit_context(Default::default()), count)
        .unwrap();
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let operations = r::ledger::LedgerHashMap::new().insert(
        EntryPointBuf(b"subtracted".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                seeded.context.query.state.get_ref().clone(),
                operations,
                Default::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        key,
    )
}

fn manual(
    state: &ObservedContractState,
    selected: bool,
    value: Uint16,
) -> Result<r::recording::RecordedCircuitResult<u64, Uint16>, r::CompactError> {
    let (frame, below) = ledger_slots::count.record_less_than(
        r::recording::RecordingFrame::new(state.circuit_context(7)),
        10,
    )?;
    let value = if below && selected {
        Uint16::new(
            value
                .value()
                .checked_sub(1)
                .ok_or_else(|| r::CompactError::InvalidLedgerCell("manual underflow".into()))?,
        )?
    } else {
        value
    };
    Ok(frame.finish(value))
}

#[test]
fn conditional_unsigned_return_preserves_public_query_and_prepared_fab() {
    // Independent integer expectations, including both paths that suppress an
    // otherwise invalid zero subtraction. No new TypeScript or proof claim.
    for (count, selected, input, expected) in [
        (0, true, 1, 0),
        (0, false, 0, 0),
        (9, true, 65535, 65534),
        (10, true, 0, 0),
        (10, false, 65535, 65535),
    ] {
        let (state, key) = setup(count);
        let value = Uint16::new(input).unwrap();
        let native = c::subtracted(state.circuit_context(7), selected, value).unwrap();
        let recorded = c::recorded::subtracted(state.circuit_context(7), selected, value).unwrap();
        let contract = c::Contract::default();
        let call = contract
            .recording()
            .subtracted_call(&state, 7, selected, value)
            .unwrap();
        let manual = manual(&state, selected, value).unwrap();
        assert_eq!(native.result.value(), expected);
        let manual_replay = manual
            .public
            .initial()
            .query(manual.public.verify_ops(), None, &native.context.cost_model)
            .unwrap();
        for actual in [&recorded, call.recorded(), &manual] {
            assert_eq!(actual.execution.result, native.result);
            assert_eq!(actual.execution.context.private_state, 7);
            assert!(actual.execution.private_transcript_outputs.is_empty());
            assert_eq!(
                actual.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                actual.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(actual.execution.gas_cost, native.gas_cost);
            assert_eq!(
                serde_json::json!(actual.public.verify_ops()),
                serde_json::json!(manual.public.verify_ops())
            );
            assert!(!actual.public.verify_ops().is_empty());
            let replay = actual
                .public
                .initial()
                .query(actual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, manual_replay.gas_cost);
        }
        assert!(native.private_transcript_outputs.is_empty());
        assert_eq!(
            native.context.query.state.get_ref(),
            state.contract().data.get_ref()
        );
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(
            prepared.input,
            r::fab::AlignedValue::from((selected, value))
        );
        assert_eq!(
            prepared.output,
            r::fab::AlignedValue::from(Uint16::new(expected).unwrap())
        );
    }
}

#[test]
fn selected_subtraction_underflow_is_identical_across_generated_lanes() {
    for count in [0, 9] {
        let (state, _) = setup(count);
        let value = Uint16::new(0).unwrap();
        let native = c::subtracted(state.circuit_context(7), true, value)
            .err()
            .unwrap();
        let recorded = c::recorded::subtracted(state.circuit_context(7), true, value)
            .err()
            .unwrap();
        let contract = c::Contract::default();
        let observed = contract
            .recording()
            .subtracted_call(&state, 7, true, value)
            .err()
            .unwrap();
        assert_eq!(native, recorded);
        assert_eq!(native, observed);
        assert!(manual(&state, true, value).is_err());
    }
}
