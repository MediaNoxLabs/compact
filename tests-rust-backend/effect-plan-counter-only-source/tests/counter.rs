// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source-matched native, recorded and independent public-program controls.
use compact_rust_effect_plan_counter_only_source_fixture::{
    ledger_contract as contract, ledger_slots, runtime,
};
use runtime::context::{CircuitContext, ConstructorContext, ConstructorResult};
use runtime::ledger::{ChargedState, ContractAddress, DefaultDB, StateValue, constructor_cell};
use runtime::recording::{RecordedCircuitResult, RecordingFrame};
use runtime::{BoundedUint, CompactError, Field};

type Amount = BoundedUint<65535>;

fn state_cells(round: u64, peer: u64) -> Vec<StateValue<DefaultDB>> {
    vec![
        constructor_cell::<_, DefaultDB>(round),
        constructor_cell::<_, DefaultDB>(peer),
    ]
}

fn record_outcome(
    frame: RecordingFrame<u64>,
    result: Field,
) -> Result<RecordingFrame<u64>, CompactError> {
    let _ = result;
    Ok(frame)
}

fn assert_outcome(state: &StateValue<DefaultDB>, expected: u64) {
    let _ = expected;
    let StateValue::Array(fields) = state else {
        panic!("expected contract array")
    };
    assert_eq!(
        fields.len(),
        2,
        "Counter-only contract has no artificial outcome Cell"
    );
}

include!("../../effect-plan-counter-source/tests/support/boundaries.rs");
