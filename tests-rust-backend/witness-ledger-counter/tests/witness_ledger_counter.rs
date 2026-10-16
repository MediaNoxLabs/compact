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

use compact_rust_witness_ledger_counter_fixture::ledger_contract::{
    LedgerView, Witnesses, increment_round, initial_state, private_round,
};
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

type CounterValue = BoundedUint<18446744073709551615>;

struct ReadRound;

impl Witnesses<u64> for ReadRound {
    fn read_round(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, CounterValue) {
        assert!(matches!(*context.private_state, 7 | 8));
        (*context.private_state + 1, context.ledger.round().unwrap())
    }
}

fn assert_oracle_output(result: &CircuitResult<u64, CounterValue>, oracle: &serde_json::Value) {
    let queries = oracle["queries"].as_array().unwrap();
    assert_eq!(queries.len(), 1);
    assert_eq!(oracle["publicTranscript"].as_array().unwrap().len(), 0);
    let actual_gas = serde_json::to_value(result.gas_cost).unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected: u64 = queries
            .iter()
            .map(|query| {
                query["gasCost"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(
            actual_gas[key].as_u64().unwrap(),
            expected,
            "{key} total gas"
        );
        // The TypeScript wrapper reports no public query for a witness-only
        // circuit, although the witness ledger read still runs a VM query.
        assert_eq!(oracle["reportedGas"][key], "0");
    }
    let expected: u128 = oracle["result"].as_str().unwrap().parse().unwrap();
    assert_eq!(result.result.value(), expected);
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
fn witness_reads_current_typed_counter() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-ledger-counter-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let before = private_round(context, &ReadRound).unwrap();
    assert_oracle_output(&before, &oracle["before"]);
    let write = increment_round(before.context).unwrap();
    assert!(write.private_transcript_outputs.is_empty());
    let after = private_round(write.context, &ReadRound).unwrap();
    assert_oracle_output(&after, &oracle["after"]);
}

// Qualify newly recorded witness-only exports against the retained TS oracle.
mod recording_parity {
    use super::*;
    use compact_rust_witness_ledger_counter_fixture::ledger_contract as contract;
    use midnight_compact_runtime as runtime;
    use std::cell::RefCell;

    struct Logged<'a>(&'a RefCell<Vec<u64>>);
    impl Witnesses<u64> for Logged<'_> {
        fn read_round(
            &self,
            context: WitnessContext<'_, u64, LedgerView<'_>>,
        ) -> (u64, CounterValue) {
            self.0.borrow_mut().push(*context.private_state);
            Witnesses::read_round(&ReadRound, context)
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
            "../../../runtime-rs/tests/fixtures/witness-ledger-counter-ts-output.json"
        ))
        .unwrap();
        for (populated, key) in [(false, "before"), (true, "after")] {
            let private = if populated { 8_u64 } else { 7_u64 };
            let context = initial_state(ConstructorContext::new(private))
                .unwrap()
                .into_circuit_context(ContractAddress::default());
            let context = if populated {
                increment_round(context).unwrap().context
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
            let native = private_round(context(), &Logged(&native_calls)).unwrap();
            assert_eq!(*native_calls.borrow(), [private]);
            assert_oracle_output(&native, &oracle[key]);
            let recorded_calls = RefCell::new(Vec::new());
            let recorded =
                contract::recorded::private_round(context(), &Logged(&recorded_calls)).unwrap();
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
                    .private_round_call(&observed, private)
                    .unwrap();
                assert_eq!(*observed_calls.borrow(), [private]);
                same_execution(&native, &call.recorded().execution);
                assert_oracle_output(&call.recorded().execution, &oracle[key]);
                assert!(call.recorded().public.verify_ops().is_empty());
            }
        }
    }
}
