// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r, types::CommitPayload};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::fab::AlignedValue;
use r::recording::RecordingFrame;
use r::transaction::{Observation, ObservedContractState, VerifierKey};

#[test]
fn named_fixed_product_commitment_compiles_and_executes_in_all_lanes() {
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let operations = r::ledger::LedgerHashMap::new().insert(
        EntryPointBuf(b"commit_product".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    let state = ObservedContractState::new(
        Default::default(),
        r::ledger::ContractState::new(
            initial.ledger_state.get_ref().clone(),
            operations,
            Default::default(),
        ),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let payload = CommitPayload {
        fields: r::FixedVector::new([r::Field::from(5_u64), r::Field::from(9_u64)]),
        flag: true,
    };
    let native = c::commit_product(state.circuit_context(7), payload.clone()).unwrap();
    let recorded = c::recorded::commit_product(state.circuit_context(7), payload.clone()).unwrap();
    let contract = c::Contract::default();
    let call = contract
        .recording()
        .commit_product_call(&state, 7, payload.clone())
        .unwrap();
    let (frame, opening) = slots::opening
        .record_read(RecordingFrame::new(state.circuit_context(7)))
        .unwrap();
    let expected = r::persistent_commit(payload.clone(), opening);
    let manual = slots::commitCell
        .record_write(frame, expected)
        .unwrap()
        .finish(());
    assert_eq!(
        slots::commitCell
            .inspect(native.context.query.state.get_ref())
            .unwrap(),
        expected
    );
    // Exact generated struct FAB must preserve its vector/Boolean field order.
    let input = AlignedValue::from(payload);
    assert_eq!(
        input,
        AlignedValue::from((
            r::FixedVector::new([r::Field::from(5_u64), r::Field::from(9_u64)]),
            true
        ))
    );
    let reference = manual
        .public
        .initial()
        .query(manual.public.verify_ops(), None, &native.context.cost_model)
        .unwrap();
    for actual in [&recorded, call.recorded(), &manual] {
        assert_eq!(
            actual.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(
            actual.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(actual.execution.gas_cost, native.gas_cost);
        assert_eq!(actual.execution.context.private_state, 7);
        assert!(actual.execution.private_transcript_outputs.is_empty());
        assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
        let replay = actual
            .public
            .initial()
            .query(actual.public.verify_ops(), None, &native.context.cost_model)
            .unwrap();
        assert_eq!(replay.context.state, native.context.query.state);
        assert_eq!(replay.context.effects, native.context.query.effects);
        assert_eq!(replay.gas_cost, reference.gas_cost);
    }
    let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
    assert_eq!(prepared.input, input);
    assert_eq!(prepared.output, AlignedValue::from(()));
}
