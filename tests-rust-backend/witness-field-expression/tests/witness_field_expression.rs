use compact_rust_witness_field_expression_fixture::ledger_contract::{
    LedgerView, Witnesses, add_secret, initial_state, sum_secrets,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

struct Secret;

impl Witnesses<u64> for Secret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        assert_eq!(seed, Field::from(2_u64));
        (
            *context.private_state + 1,
            seed + Field::from(*context.private_state),
        )
    }
}

fn assert_oracle_output(result: &CircuitResult<u64, Field>, oracle: &serde_json::Value) {
    let expected: u64 = oracle["result"].as_str().unwrap().parse().unwrap();
    assert_eq!(result.result, Field::from(expected));
    assert_eq!(
        result.context.private_state,
        oracle["privateState"].as_u64().unwrap()
    );
    let expected_outputs = oracle["privateTranscriptOutputs"].as_array().unwrap();
    assert_eq!(
        result.private_transcript_outputs.len(),
        expected_outputs.len()
    );
    for (output, expected) in result
        .private_transcript_outputs
        .iter()
        .zip(expected_outputs)
    {
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
fn witness_result_composes_with_field_addition() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-field-expression-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let result = add_secret(context, &Secret, Field::from(2_u64)).unwrap();
    assert_oracle_output(&result, &oracle);
}

#[test]
fn two_witnesses_preserve_source_order_and_transcript_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-field-expression-pair-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let result = sum_secrets(context, &Secret, Field::from(2_u64)).unwrap();
    assert_oracle_output(&result, &oracle);
}
