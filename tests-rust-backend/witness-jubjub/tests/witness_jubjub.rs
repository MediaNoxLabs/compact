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

use compact_rust_witness_jubjub_fixture::ledger_contract::{
    LedgerView, Witnesses, combine, generator, hash_scalar, initial_state, negate, point_x,
    point_y, reveal, scale,
};
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;
use midnight_compact_runtime::{Field, JubjubPoint, hash_to_curve};

struct Secret;

impl Witnesses<u64> for Secret {
    fn secret_point(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> (u64, JubjubPoint) {
        (
            *context.private_state + 1,
            hash_to_curve(seed + Field::from(*context.private_state)),
        )
    }

    fn secret_scalar(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        (
            *context.private_state + 1,
            seed + Field::from(*context.private_state),
        )
    }
}

fn context() -> midnight_compact_runtime::context::CircuitContext<u64> {
    initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}

fn point_json(point: JubjubPoint) -> serde_json::Value {
    serde_json::json!({
        "x": hex::encode(point.x().unwrap().as_le_bytes()),
        "y": hex::encode(point.y().unwrap().as_le_bytes()),
    })
}

fn assert_oracle<T>(
    result: &CircuitResult<u64, T>,
    actual: serde_json::Value,
    oracle: &serde_json::Value,
) {
    assert_eq!(actual, oracle["result"]);
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
fn witness_points_compose_with_curve_natives_and_keep_transcript_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-jubjub.json"
    ))
    .unwrap();
    let seed = Field::from(2_u64);
    let result = combine(context(), &Secret, seed).unwrap();
    assert_oracle(&result, point_json(result.result), &oracle["combine"]);
    let result = scale(context(), &Secret, seed).unwrap();
    assert_oracle(&result, point_json(result.result), &oracle["scale"]);
    let result = point_x(context(), &Secret, seed).unwrap();
    assert_oracle(
        &result,
        serde_json::json!(hex::encode(result.result.as_le_bytes())),
        &oracle["pointX"],
    );
    let result = point_y(context(), &Secret, seed).unwrap();
    assert_oracle(
        &result,
        serde_json::json!(hex::encode(result.result.as_le_bytes())),
        &oracle["pointY"],
    );
    let result = negate(context(), &Secret, seed).unwrap();
    assert_oracle(&result, point_json(result.result), &oracle["negate"]);
    let result = reveal(context(), &Secret, seed).unwrap();
    assert_oracle(&result, point_json(result.result), &oracle["reveal"]);
    let result = hash_scalar(context(), &Secret, seed).unwrap();
    assert_oracle(&result, point_json(result.result), &oracle["hashScalar"]);
    let result = generator(context(), &Secret, seed).unwrap();
    assert_oracle(&result, point_json(result.result), &oracle["generator"]);
}
