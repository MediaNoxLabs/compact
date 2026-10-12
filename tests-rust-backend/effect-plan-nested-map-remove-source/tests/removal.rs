// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_effect_plan_nested_map_remove_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use r::fab::AlignedValue;
use r::ledger::{DefaultDB, LedgerHashMap, StateValue, constructor_cell};
use r::recording::RecordingFrame;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};

fn state(present: bool) -> StateValue<DefaultDB> {
    let child = |value| {
        StateValue::Map(LedgerHashMap::new().insert(
            AlignedValue::from(r::Field::from(11_u64)),
            constructor_cell(r::BoundedUint::<{ u64::MAX as u128 }>::new(value).unwrap()),
        ))
    };
    let mut outer =
        LedgerHashMap::new().insert(AlignedValue::from(r::Field::from(2_u64)), child(90));
    if present {
        outer = outer.insert(AlignedValue::from(r::Field::from(1_u64)), child(10));
    }
    StateValue::Array(vec![StateValue::Map(outer)].into())
}

#[test]
fn source_generated_removal_matches_typescript_and_preserves_nested_siblings() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/typescript.json")).unwrap();
    assert_eq!(oracle["caseCount"], 2);
    assert_eq!(oracle["rows"].as_array().unwrap().len(), 2);
    for present in [true, false] {
        let key: VerifierKey = StdRng::seed_from_u64(602).r#gen();
        let operations = midnight_storage::storage::HashMap::new().insert(
            EntryPointBuf(b"remove".to_vec()),
            ContractOperation::new(Some(key.clone())),
        );
        // Supplied nested state uses official ledger nodes; no nested-value
        // scalar codec or generated nested insertion is assumed.
        let observed = ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(state(present), operations, Default::default()),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let input = r::Field::from(1_u64);
        let native = c::remove(observed.circuit_context(5_u64), input).unwrap();
        let recorded = c::recorded::remove(observed.circuit_context(5_u64), input).unwrap();
        let manual = slots::outer
            .record_remove(RecordingFrame::new(observed.circuit_context(5_u64)), input)
            .unwrap()
            .finish(());
        let contract = c::Contract::default();
        let call = contract
            .recording()
            .remove_call(&observed, 5_u64, input)
            .unwrap();
        assert_eq!(native.context.query.state.get_ref(), &state(false));
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
            assert_eq!(actual.execution.context.private_state, 5);
            assert!(actual.execution.private_transcript_outputs.is_empty());
            assert_eq!(
                serde_json::to_value(actual.public.verify_ops()).unwrap(),
                serde_json::to_value(manual.public.verify_ops()).unwrap()
            );
            let replay = actual
                .public
                .initial()
                .query(actual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, native.gas_cost);
        }
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(input));
        assert_eq!(prepared.output, AlignedValue::from(()));
        let id = if present { "present" } else { "absent" };
        let rows: Vec<_> = oracle["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["id"] == id)
            .collect();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(row["circuit"], "remove");
        assert_eq!(row["args"], serde_json::json!(["1"]));
        assert_eq!(row["present"], present);
        let remaining = serde_json::json!([["2", [["11", "90"]]]]);
        let supplied = if present {
            serde_json::json!([["1", [["11", "10"]]], ["2", [["11", "90"]]]])
        } else {
            remaining.clone()
        };
        assert_eq!(row["suppliedEntries"], supplied);
        assert_eq!(row["remainingEntries"], remaining);
        assert_eq!(row["status"], "success");
        assert_eq!(row["privateState"], 5);
        assert_eq!(
            row["proofData"]["privateTranscriptOutputs"],
            serde_json::json!([])
        );
        for (name, value) in [("input", &prepared.input), ("output", &prepared.output)] {
            assert_eq!(row["proofData"][name], serde_json::to_value(value).unwrap());
        }
        let gas = &native.gas_cost;
        assert_eq!(
            row["queryCostSum"]["readTime"],
            gas.read_time.into_picoseconds().to_string()
        );
        assert_eq!(
            row["queryCostSum"]["computeTime"],
            gas.compute_time.into_picoseconds().to_string()
        );
        assert_eq!(
            row["queryCostSum"]["bytesWritten"],
            gas.bytes_written.to_string()
        );
        assert_eq!(
            row["queryCostSum"]["bytesDeleted"],
            gas.bytes_deleted.to_string()
        );
    }
}
