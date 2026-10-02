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
