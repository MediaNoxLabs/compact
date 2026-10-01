use compact_rust_witness_conditional_fixture::ledger_contract::{
    LedgerView, Witnesses, choose_secret, initial_state, local_secret, nested_secret, pair_secret,
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
        (
            *context.private_state + 1,
            seed + Field::from(*context.private_state),
        )
    }
}

fn initial() -> midnight_compact_runtime::context::CircuitContext<u64> {
    initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}

fn assert_oracle<T>(result: &CircuitResult<u64, T>, oracle: &serde_json::Value) {
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
fn conditional_witness_runs_only_in_selected_branch() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-conditional-ts-output.json"
    ))
    .unwrap();
    let selected = choose_secret(
        initial(),
        &Secret,
        true,
        Field::from(2_u64),
        Field::from(20_u64),
    )
    .unwrap();
    assert_eq!(selected.result, Field::from(9_u64));
    assert_oracle(&selected, &oracle["selected"]);

    let unselected = choose_secret(
        initial(),
        &Secret,
        false,
        Field::from(2_u64),
        Field::from(20_u64),
    )
    .unwrap();
    assert_eq!(unselected.result, Field::from(27_u64));
    assert_oracle(&unselected, &oracle["unselected"]);
}

#[test]
fn tuple_witnesses_preserve_element_and_transcript_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-conditional-ts-output.json"
    ))
    .unwrap();
    let pair = pair_secret(initial(), &Secret, Field::from(2_u64)).unwrap();
    assert_eq!(pair.result, (Field::from(9_u64), Field::from(10_u64)));
    assert_oracle(&pair, &oracle["pair"]);
}

#[test]
fn nested_witness_argument_observes_updated_private_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-conditional-ts-output.json"
    ))
    .unwrap();
    let nested = nested_secret(initial(), &Secret, Field::from(2_u64)).unwrap();
    assert_eq!(nested.result, Field::from(17_u64));
    assert_oracle(&nested, &oracle["nested"]);
}

#[test]
fn local_binding_preserves_single_witness_evaluation() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-conditional-ts-output.json"
    ))
    .unwrap();
    let local = local_secret(initial(), &Secret, Field::from(2_u64)).unwrap();
    assert_eq!(local.result, Field::from(10_u64));
    assert_oracle(&local, &oracle["local"]);
}
