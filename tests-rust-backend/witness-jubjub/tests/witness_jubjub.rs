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

#[derive(Clone)]
struct GeneratorWitness(std::rc::Rc<std::cell::RefCell<Vec<(Field, u64)>>>);

impl Witnesses<u64> for GeneratorWitness {
    fn secret_point(
        &self,
        _context: WitnessContext<'_, u64, LedgerView<'_>>,
        _seed: Field,
    ) -> (u64, JubjubPoint) {
        panic!("generator must call only the Field witness")
    }

    fn secret_scalar(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        self.0.borrow_mut().push((seed, *context.private_state));
        (
            *context.private_state + 1,
            seed + Field::from(*context.private_state),
        )
    }
}

#[test]
fn recorded_generator_preserves_single_field_witness_and_private_transcript() {
    use compact_rust_witness_jubjub_fixture::ledger_contract::recorded;
    use midnight_compact_runtime::{ec_mul_generator, jubjub_scalar_from_native};
    let seed = Field::from(2_u64);
    let native_witness = GeneratorWitness(Default::default());
    let recorded_witness = GeneratorWitness(Default::default());
    let native = generator(context(), &native_witness, seed).unwrap();
    let recorded = recorded::generator(context(), &recorded_witness, seed).unwrap();
    assert_eq!(*native_witness.0.borrow(), vec![(seed, 7)]);
    assert_eq!(*recorded_witness.0.borrow(), *native_witness.0.borrow());
    assert_eq!(
        native.result,
        ec_mul_generator(jubjub_scalar_from_native(Field::from(9_u64))).unwrap()
    );
    assert_eq!(recorded.execution.result, native.result);
    assert_eq!(recorded.execution.context.private_state, 8);
    assert_eq!(
        recorded.execution.context.private_state,
        native.context.private_state
    );
    assert_eq!(
        recorded.execution.context.query.state,
        native.context.query.state
    );
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        vec![midnight_compact_runtime::fab::AlignedValue::from(
            Field::from(9_u64)
        )]
    );
    assert!(recorded.public.verify_ops().is_empty());
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-jubjub.json"
    ))
    .unwrap();
    assert_oracle(
        &recorded.execution,
        point_json(recorded.execution.result),
        &oracle["generator"],
    );
}

#[cfg(feature = "ledger-transaction")]
#[test]
fn recorded_generator_empty_public_program_is_not_a_prepared_call() {
    use compact_rust_witness_jubjub_fixture::ledger_contract::recorded;
    use midnight_compact_runtime::transaction::{
        CallSpec, PrepareCallError, VerifierKey, prepare_call,
    };
    use midnight_serialize::{Deserializable, Serializable};
    let witness = GeneratorWitness(Default::default());
    let seed = Field::from(2_u64);
    let recorded = recorded::generator(context(), &witness, seed).unwrap();
    assert!(recorded.public.verify_ops().is_empty());
    // Empty-program refusal precedes key inspection; this carrier is not a proof key.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let carrier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let spec = CallSpec::new("generator", carrier, seed, Field::from(0_u64));
    assert!(matches!(
        prepare_call(recorded, spec),
        Err(PrepareCallError::EmptyTranscript)
    ));
    assert_eq!(*witness.0.borrow(), vec![(seed, 7)]);
}

#[cfg(feature = "ledger-transaction")]
#[path = "witness_jubjub/hash_recording.rs"]
mod hash_recording;

#[cfg(feature = "ledger-transaction")]
#[path = "witness_jubjub/negation_recording.rs"]
mod negation_recording;
