// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_map_lambda_oracle_fixture::{ledger_contract as c, ledger_slots};
use midnight_compact_runtime as r;
#[path = "../../support/witness_only_recording.rs"]
#[allow(dead_code)]
mod support;

#[test]
fn observed_ping_matches_original_ts_manual_write_and_prepared_unit_fab() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/map-lambda-oracle.json"
    ))
    .unwrap();
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, key) = support::observation(initial.ledger_state.get_ref().clone(), &["ping"]);
    let native = c::ping(state.circuit_context(7)).unwrap();
    let recorded = c::recorded::ping(state.circuit_context(7)).unwrap();
    let contract = c::Contract::default();
    let call = contract.recording().ping_call(&state, 7).unwrap();
    type U64 = r::BoundedUint<{ u64::MAX as u128 }>;
    let expected = r::FixedVector::new([U64::new(0).unwrap(); 3]);
    let manual = ledger_slots::doubled
        .record_write(
            r::recording::RecordingFrame::new(state.circuit_context(7)),
            expected.clone(),
        )
        .unwrap()
        .finish(());
    assert_eq!(
        ledger_slots::doubled
            .inspect(native.context.query.state.get_ref())
            .unwrap(),
        expected
    );
    let manual_replay = manual
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
        assert_eq!(actual.execution.context.private_state, 7);
        assert!(actual.execution.private_transcript_outputs.is_empty());
        assert_eq!(actual.execution.gas_cost, native.gas_cost);
        assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
        assert_eq!(
            serde_json::to_value(actual.public.verify_ops()).unwrap(),
            oracle["ping"]["queries"][0]["program"]
        );
        let replay = actual
            .public
            .initial()
            .query(actual.public.verify_ops(), None, &native.context.cost_model)
            .unwrap();
        assert_eq!(replay.context.state, native.context.query.state);
        assert_eq!(replay.context.effects, native.context.query.effects);
        assert_eq!(replay.gas_cost, manual_replay.gas_cost);
    }
    let gas = serde_json::to_value(native.gas_cost).unwrap();
    for metric in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            gas[metric].as_u64().unwrap().to_string(),
            oracle["ping"]["gasCost"][metric].as_str().unwrap()
        );
    }
    assert!(native.private_transcript_outputs.is_empty());
    // This empty verifier carrier only qualifies preparation, never proof verification.
    let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
    assert_eq!(prepared.input, r::fab::AlignedValue::from(()));
    assert_eq!(prepared.output, r::fab::AlignedValue::from(()));
}
