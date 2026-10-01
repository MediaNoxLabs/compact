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

use compact_rust_jubjub_construct_fixture::ledger_contract::{
    LedgerView, Witnesses, construct_echo, initial_state,
};
use compact_rust_jubjub_construct_fixture::pure_circuits::{construct_point, construct_y};
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;
use midnight_compact_runtime::{CompactError, Field, JubjubPoint};

struct Echo;

impl Witnesses<u64> for Echo {
    fn echo(&self, context: WitnessContext<'_, u64, LedgerView<'_>>, value: Field) -> (u64, Field) {
        (*context.private_state + 1, value)
    }
}

fn field(value: &serde_json::Value) -> Field {
    let bytes: [u8; 32] = hex::decode(value.as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    Field::from_le_bytes(&bytes).unwrap()
}

fn assert_coords(point: JubjubPoint, expected: &serde_json::Value) {
    let x = point.x().unwrap_or_else(|| Field::from(0_u64));
    let y = point.y().unwrap_or_else(|| Field::from(1_u64));
    assert_eq!(hex::encode(x.as_le_bytes()), expected["x"]);
    assert_eq!(hex::encode(y.as_le_bytes()), expected["y"]);
}

#[test]
fn constructed_points_match_typescript_and_invalid_coordinates_are_rejected() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/jubjub-construct.json"
    ))
    .unwrap();
    let x = field(&oracle["input"]["x"]);
    let y = field(&oracle["input"]["y"]);
    assert_coords(construct_point(x, y).unwrap(), &oracle["constructed"]);
    assert_eq!(
        hex::encode(construct_y(x, y).unwrap().as_le_bytes()),
        oracle["constructedY"]
    );

    let zero = Field::from(0_u64);
    let one = Field::from(1_u64);
    assert_coords(construct_point(zero, one).unwrap(), &oracle["identity"]);
    assert_eq!(
        hex::encode(construct_y(zero, one).unwrap().as_le_bytes()),
        oracle["identityY"]
    );
    assert_eq!(
        construct_point(zero, Field::from(2_u64)),
        Err(CompactError::InvalidJubjubPoint)
    );

    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let result = construct_echo(context, &Echo, x, y).unwrap();
    let expected = &oracle["witness"];
    assert_coords(result.result, &expected["result"]);
    assert_eq!(result.context.private_state, expected["privateState"]);
    let outputs = expected["privateTranscriptOutputs"].as_array().unwrap();
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
