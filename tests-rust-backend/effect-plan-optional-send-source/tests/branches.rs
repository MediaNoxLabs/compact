// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod support;
use compact_rust_effect_plan_optional_send_source_fixture::{
    ledger_contract as c, runtime as r, types,
};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use r::fab::AlignedValue;
use r::transaction::{
    Observation, ObservedCallError, ObservedContractState, PrepareCallError, VerifierKey,
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use support::*;
fn setup() -> (ObservedContractState, VerifierKey) {
    let key: VerifierKey = StdRng::seed_from_u64(561).r#gen();
    let operations = midnight_storage::storage::HashMap::new().insert(
        EntryPointBuf(b"optional_send".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    (
        ObservedContractState::new(
            address(),
            r::ledger::ContractState::new(
                context().query.state.get_ref().clone(),
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
#[test]
fn optional_send_matches_manual_and_pinned_typescript_for_both_branches() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/typescript.json")).unwrap();
    assert_eq!(oracle["rows"].as_array().unwrap().len(), 18);
    for row in oracle["rows"].as_array().unwrap() {
        let selected = row["selected"].as_bool().unwrap();
        let kind = row["kind"].as_u64().unwrap() as u8;
        let amount = row["amount"].as_str().unwrap().parse().unwrap();
        let bound = r::BoundedUint::new(amount).unwrap();
        let native = c::optional_send(context(), selected, coin(), recipient(kind), bound);
        let recorded =
            c::recorded::optional_send(context(), selected, coin(), recipient(kind), bound);
        let manual = manual(context(), selected, kind, amount);
        if selected && amount > 42 {
            assert_eq!(native.err().unwrap(), r::CompactError::UnsignedUnderflow);
            assert_eq!(recorded.err().unwrap(), r::CompactError::UnsignedUnderflow);
            assert_eq!(manual.err().unwrap(), r::CompactError::UnsignedUnderflow);
            assert!(
                row["error"]
                    .as_str()
                    .unwrap()
                    .contains("result of subtraction would be negative")
            );
            continue;
        }
        let (native, recorded, manual) = (native.unwrap(), recorded.unwrap(), manual.unwrap());
        compare(&native, &recorded, &manual);
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            row["publicTranscript"]
        );
        assert_eq!(
            serde_json::to_value(&native.private_transcript_outputs).unwrap(),
            row["privateOutputs"]
        );
        let output: AlignedValue = native.result.clone().into();
        assert_eq!(serde_json::to_value(output).unwrap(), row["output"]);
        let gas = serde_json::to_value(native.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                gas[key].as_u64().unwrap().to_string(),
                row["queryCostSum"][key]
            );
        }
        assert_eq!(recorded.public.verify_ops().is_empty(), !selected);
        if !selected {
            assert_eq!(native.result, types::ShieldedSendResult::default());
            assert!(native.context.circuit_zswap().inputs().is_empty());
            assert!(native.context.circuit_zswap().outputs().is_empty());
            assert_eq!(native.context.circuit_zswap().next_index(), 1);
            assert_eq!(native.context.query.effects, context().query.effects);
            assert_eq!(native.gas_cost, r::context::RunningCost::default());
        } else {
            assert_eq!(native.context.circuit_zswap().inputs().len(), 1);
            assert_eq!(
                native.context.circuit_zswap().outputs().len(),
                if amount == 42 { 1 } else { 2 }
            );
        }
    }
}
#[test]
fn observed_noop_is_constructible_but_not_preparable_and_active_send_keeps_binding_boundary() {
    let (state, key) = setup();
    let contract = c::Contract::default();
    let before = state.circuit_context(()).query.state;
    for kind in [7, 9, 12] {
        for amount in [17, 42, 43] {
            let bound = r::BoundedUint::new(amount).unwrap();
            let call = contract
                .recording()
                .optional_send_call(&state, (), false, coin(), recipient(kind), bound)
                .unwrap();
            assert_eq!(
                call.recorded().execution.result,
                types::ShieldedSendResult::default()
            );
            assert!(call.recorded().public.verify_ops().is_empty());
            assert!(
                call.recorded()
                    .execution
                    .private_transcript_outputs
                    .is_empty()
            );
            assert!(
                call.recorded()
                    .execution
                    .context
                    .circuit_zswap()
                    .inputs()
                    .is_empty()
            );
            assert!(
                call.recorded()
                    .execution
                    .context
                    .circuit_zswap()
                    .outputs()
                    .is_empty()
            );
            assert_eq!(call.recorded().execution.context.query.state, before);
            assert!(matches!(
                call.prepare(key.clone(), r::Field::from(0_u64)),
                Err(ObservedCallError::Prepare(
                    PrepareCallError::EmptyTranscript
                ))
            ));
            // An ordinary observation carries no offer allocation. Preserve this exact body error.
            let expected = if amount > 42 {
                r::CompactError::UnsignedUnderflow
            } else {
                r::CompactError::ZswapAllocationLocked
            };
            assert_eq!(
                c::recorded::optional_send(
                    state.circuit_context(()),
                    true,
                    coin(),
                    recipient(kind),
                    bound
                )
                .err()
                .unwrap(),
                expected
            );
            assert_eq!(
                contract
                    .recording()
                    .optional_send_call(&state, (), true, coin(), recipient(kind), bound)
                    .err()
                    .unwrap(),
                expected
            );
        }
    }
    assert_eq!(state.circuit_context(()).query.state, before);
}
