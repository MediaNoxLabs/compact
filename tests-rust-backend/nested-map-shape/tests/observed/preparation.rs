// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use midnight_compact_runtime as r;
use midnight_serialize::{Deserializable, Serializable};
use r::fab::AlignedValue;
use r::ledger::{LedgerHashMap, constructor_cell};
use r::recording::RecordingFrame;
use r::transaction::{Observation, ObservedContractState, VerifierKey};

#[test]
fn nested_metadata_observes_full_children_without_a_scalar_value_codec() {
    for populated in [false, true] {
        let child = |value| {
            StateValue::Map(LedgerHashMap::new().insert(
                AlignedValue::from(r::Field::from(11_u64)),
                constructor_cell(r::BoundedUint::<{ u64::MAX as u128 }>::new(value).unwrap()),
            ))
        };
        let outer = if populated {
            LedgerHashMap::new()
                .insert(AlignedValue::from(r::Field::from(1_u64)), child(10))
                .insert(AlignedValue::from(r::Field::from(2_u64)), child(90))
        } else {
            LedgerHashMap::new()
        };
        let data = StateValue::Array(vec![StateValue::Map(outer)].into());
        // Empty serialized key is only a preparation-order carrier, not a proof.
        let mut bytes = Vec::new();
        Vec::<u8>::new().serialize(&mut bytes).unwrap();
        let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
        let operations = LedgerHashMap::new().insert(
            EntryPointBuf(b"check_nested_empty".to_vec()),
            ContractOperation::new(Some(key.clone())),
        );
        let state = ObservedContractState::new(
            Default::default(),
            ContractState::new(data.clone(), operations, Default::default()),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let contract = Contract::default();
        let native = contract
            .check_nested_empty(state.circuit_context(7_u64))
            .unwrap();
        let recorded = contract
            .recording()
            .check_nested_empty(state.circuit_context(7))
            .unwrap();
        let call = contract
            .recording()
            .check_nested_empty_call(&state, 7)
            .unwrap();
        let (frame, empty) = users_by_org
            .record_is_empty(RecordingFrame::new(state.circuit_context(7)))
            .unwrap();
        let manual = frame.finish(empty);
        assert_eq!(native.result, !populated);
        assert_eq!(native.context.query.state.get_ref(), &data);
        assert_eq!(native.context.private_state, 7);
        assert!(native.private_transcript_outputs.is_empty());
        let reference = manual
            .public
            .initial()
            .query(manual.public.verify_ops(), None, &native.context.cost_model)
            .unwrap();
        for actual in [&recorded, call.recorded(), &manual] {
            assert_eq!(actual.execution.result, native.result);
            assert_eq!(actual.execution.gas_cost, native.gas_cost);
            assert_eq!(actual.execution.context.query.state.get_ref(), &data);
            assert_eq!(
                actual.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(actual.execution.context.private_state, 7);
            assert!(actual.execution.private_transcript_outputs.is_empty());
            assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
            let replay = actual
                .public
                .initial()
                .query(actual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            assert_eq!(replay.context.state.get_ref(), &data);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, reference.gas_cost);
        }
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(()));
        assert_eq!(prepared.output, AlignedValue::from(!populated));
    }
}
