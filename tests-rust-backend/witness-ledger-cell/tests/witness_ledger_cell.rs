// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_witness_ledger_cell_fixture::ledger_contract::{
    LedgerView, TryWitnesses, Witnesses, initial_state, private_check, set_flag,
};
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::RunningCost;
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

struct ReadFlag;

struct TryReadFlag;

impl TryWitnesses<u64> for TryReadFlag {
    fn read_flag(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, bool), CompactError> {
        let flag = context.ledger.flag()?;
        Ok((*context.private_state + 1, flag))
    }
}

impl Witnesses<u64> for ReadFlag {
    fn read_flag(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, bool) {
        assert!(matches!(*context.private_state, 7 | 8));
        (*context.private_state + 1, context.ledger.flag().unwrap())
    }
}

fn assert_oracle_output(result: &CircuitResult<u64, bool>, oracle: &serde_json::Value) {
    assert_eq!(result.result, oracle["result"].as_bool().unwrap());
    assert_eq!(
        result.context.private_state,
        oracle["privateState"].as_u64().unwrap()
    );
    assert_eq!(result.private_transcript_outputs.len(), 1);
    let output = &result.private_transcript_outputs[0];
    let atoms = output
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        oracle["privateTranscriptOutputs"][0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&output.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
}

#[test]
fn witness_reads_current_typed_ledger_cell() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-ledger-cell-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let before = private_check(context, &ReadFlag).unwrap();
    assert_oracle_output(&before, &oracle["before"]);
    let write = set_flag(before.context).unwrap();
    assert!(write.private_transcript_outputs.is_empty());
    let after = private_check(write.context, &ReadFlag).unwrap();
    assert_oracle_output(&after, &oracle["after"]);
}

#[test]
fn fallible_witness_matches_legacy_general_emitter_result() {
    let context = || {
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default())
    };
    let old = private_check(context(), &ReadFlag).unwrap();
    let new = private_check(context(), &TryReadFlag).unwrap();
    assert_eq!(new.result, old.result);
    assert_eq!(new.context.private_state, old.context.private_state);
    assert_eq!(new.gas_cost, old.gas_cost);
    assert_eq!(
        new.private_transcript_outputs,
        old.private_transcript_outputs
    );
}

#[test]
fn fallible_witness_rejects_general_circuit_without_panic() {
    let mut context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    context.gas_limit = Some(RunningCost::ZERO);
    assert!(matches!(
        private_check(context, &TryReadFlag),
        Err(CompactError::LedgerQueryRejected(_))
    ));
}

// Qualify newly recorded witness-only exports against the retained TS oracle.
mod recording_parity {
    use super::*;
    use compact_rust_witness_ledger_cell_fixture::ledger_contract as contract;
    use midnight_compact_runtime as runtime;
    use std::cell::RefCell;

    struct Logged<'a>(&'a RefCell<Vec<u64>>);
    impl Witnesses<u64> for Logged<'_> {
        fn read_flag(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, bool) {
            self.0.borrow_mut().push(*context.private_state);
            Witnesses::read_flag(&ReadFlag, context)
        }
    }
    fn same_execution<T: std::fmt::Debug + PartialEq>(
        native: &CircuitResult<u64, T>,
        actual: &CircuitResult<u64, T>,
    ) {
        assert_eq!(native.result, actual.result);
        assert_eq!(native.context.private_state, actual.context.private_state);
        assert_eq!(
            native.context.query.state.get_ref(),
            actual.context.query.state.get_ref()
        );
        assert_eq!(native.context.query.effects, actual.context.query.effects);
        assert_eq!(native.gas_cost, actual.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            actual.private_transcript_outputs
        );
    }

    #[test]
    fn recorded_and_observed_witness_reads_match_native_and_ts() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../runtime-rs/tests/fixtures/witness-ledger-cell-ts-output.json"
        ))
        .unwrap();
        for (populated, key) in [(false, "before"), (true, "after")] {
            let private = if populated { 8_u64 } else { 7_u64 };
            let context = initial_state(ConstructorContext::new(private))
                .unwrap()
                .into_circuit_context(ContractAddress::default());
            let context = if populated {
                set_flag(context).unwrap().context
            } else {
                context
            };
            let state = runtime::ledger::ContractState::new(
                context.query.state.get_ref().clone(),
                Default::default(),
                Default::default(),
            );
            let context = || {
                runtime::context::CircuitContext::from_contract_state(
                    private,
                    ContractAddress::default(),
                    &state,
                )
            };
            let native_calls = RefCell::new(Vec::new());
            let native = private_check(context(), &Logged(&native_calls)).unwrap();
            assert_eq!(*native_calls.borrow(), [private]);
            assert_oracle_output(&native, &oracle[key]);
            let recorded_calls = RefCell::new(Vec::new());
            let recorded =
                contract::recorded::private_check(context(), &Logged(&recorded_calls)).unwrap();
            assert_eq!(*recorded_calls.borrow(), [private]);
            same_execution(&native, &recorded.execution);
            assert_oracle_output(&recorded.execution, &oracle[key]);
            // Witness ledger queries are metered privately; they do not invent public VM ops.
            assert!(recorded.public.verify_ops().is_empty());
            #[cfg(feature = "ledger-transaction")]
            {
                use runtime::transaction::{Observation, ObservedContractState};
                let observed = ObservedContractState::new(
                    ContractAddress::default(),
                    state.clone(),
                    Observation {
                        transaction_hash: [0; 32],
                        block_hash: [1; 32],
                        block_height: 1,
                    },
                );
                let observed_calls = RefCell::new(Vec::new());
                let contract = contract::Contract::from(Logged(&observed_calls));
                let call = contract
                    .recording()
                    .private_check_call(&observed, private)
                    .unwrap();
                assert_eq!(*observed_calls.borrow(), [private]);
                same_execution(&native, &call.recorded().execution);
                assert_oracle_output(&call.recorded().execution, &oracle[key]);
                assert!(call.recorded().public.verify_ops().is_empty());
            }
        }
    }
}

#[test]
fn recorded_fallible_witness_preserves_zero_gas_rejection() {
    use compact_rust_witness_ledger_cell_fixture::ledger_contract::recorded;
    let context = || {
        let mut context = initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        context.gas_limit = Some(RunningCost::ZERO);
        context
    };
    let native = private_check(context(), &TryReadFlag)
        .err()
        .expect("native rejects query");
    let recorded = recorded::private_check(context(), &TryReadFlag)
        .err()
        .expect("recorded rejects query");
    assert!(matches!(native, CompactError::LedgerQueryRejected(_)));
    assert!(matches!(recorded, CompactError::LedgerQueryRejected(_)));
    assert_eq!(native.to_string(), recorded.to_string());
}
