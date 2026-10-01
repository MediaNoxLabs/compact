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

use compact_rust_witness_hash_fixture::ledger_contract::{
    LedgerView, Witnesses, commit_echo, degrade_hash_echo, hash_echo, initial_state,
    persistent_commit_echo, persistent_hash_echo, upgrade_echo,
};
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;
use midnight_compact_runtime::{Field, FixedBytes};

struct Echo;

impl Witnesses<u64> for Echo {
    fn echo(&self, context: WitnessContext<'_, u64, LedgerView<'_>>, value: Field) -> (u64, Field) {
        (
            *context.private_state + 1,
            value + Field::from(*context.private_state),
        )
    }

    fn echo_bytes(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        value: FixedBytes<32>,
    ) -> (u64, FixedBytes<32>) {
        (*context.private_state + 1, value)
    }
}

fn context() -> midnight_compact_runtime::context::CircuitContext<u64> {
    initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}

fn assert_oracle<T>(result: &CircuitResult<u64, T>, actual: String, oracle: &serde_json::Value) {
    assert_eq!(actual, oracle["result"].as_str().unwrap());
    assert_eq!(
        result.context.private_state,
        oracle["privateState"].as_u64().unwrap()
    );
    let outputs = oracle["privateTranscriptOutputs"].as_array().unwrap();
    assert_eq!(result.private_transcript_outputs.len(), outputs.len());
    for (output, expected) in result.private_transcript_outputs.iter().zip(outputs) {
        let atoms = output
            .value
            .0
            .iter()
            .map(|atom| &atom.0)
            .collect::<Vec<_>>();
        assert_eq!(serde_json::to_value(atoms).unwrap(), expected["valueAtoms"]);
        assert_eq!(
            serde_json::to_value(&output.alignment).unwrap(),
            expected["alignment"]
        );
    }
}

#[test]
fn witnessed_crypto_natives_preserve_values_and_transcript_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-hash.json"
    ))
    .unwrap();
    let value = Field::from(2_u64);
    let hash = hash_echo(context(), &Echo, value).unwrap();
    assert_oracle(
        &hash,
        hex::encode(hash.result.as_le_bytes()),
        &oracle["hash"],
    );

    let commit = commit_echo(context(), &Echo, value).unwrap();
    assert_oracle(
        &commit,
        hex::encode(commit.result.as_le_bytes()),
        &oracle["commit"],
    );

    let persistent = persistent_hash_echo(context(), &Echo, value).unwrap();
    assert_oracle(
        &persistent,
        hex::encode(persistent.result.0),
        &oracle["persistentHash"],
    );

    let degrade = degrade_hash_echo(context(), &Echo, value).unwrap();
    assert_oracle(
        &degrade,
        hex::encode(degrade.result.as_le_bytes()),
        &oracle["degradeHash"],
    );

    let upgrade = upgrade_echo(context(), &Echo, value).unwrap();
    assert_oracle(&upgrade, hex::encode(upgrade.result.0), &oracle["upgrade"]);

    let opening = FixedBytes::new(std::array::from_fn(|i| (i + 1) as u8));
    let commit = persistent_commit_echo(context(), &Echo, value, opening).unwrap();
    assert_oracle(
        &commit,
        hex::encode(commit.result.0),
        &oracle["persistentCommit"],
    );
}
