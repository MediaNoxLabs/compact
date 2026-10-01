use compact_rust_witness_uint_arithmetic_fixture::ledger_contract::{
    LedgerView, Witnesses, add_echo, initial_state, multiply_echo, subtract_echo,
};
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

type Uint16 = BoundedUint<65535>;

struct Echo;

impl Witnesses<u64> for Echo {
    fn echo(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        value: Uint16,
    ) -> (u64, Uint16) {
        (*context.private_state + 1, value)
    }
}

fn initial() -> midnight_compact_runtime::context::CircuitContext<u64> {
    initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}

fn assert_oracle(result: &CircuitResult<u64, Uint16>, oracle: &serde_json::Value) {
    let expected: u128 = oracle["result"].as_str().unwrap().parse().unwrap();
    assert_eq!(result.result.value(), expected);
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
fn witnessed_unsigned_arithmetic_matches_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-uint-arithmetic-ts-output.json"
    ))
    .unwrap();
    let value = Uint16::new(2).unwrap();
    assert_oracle(&add_echo(initial(), &Echo, value).unwrap(), &oracle["add"]);
    assert_oracle(
        &subtract_echo(initial(), &Echo, value).unwrap(),
        &oracle["subtract"],
    );
    assert_oracle(
        &multiply_echo(initial(), &Echo, value).unwrap(),
        &oracle["multiply"],
    );
    let max = Uint16::new(65535).unwrap();
    assert_eq!(
        add_echo(initial(), &Echo, max).is_err(),
        oracle["addOverflowRejected"].as_bool().unwrap()
    );
    assert_eq!(
        subtract_echo(initial(), &Echo, Uint16::new(0).unwrap()).is_err(),
        oracle["subtractUnderflowRejected"].as_bool().unwrap()
    );
    assert_eq!(
        multiply_echo(initial(), &Echo, max).is_err(),
        oracle["multiplyOverflowRejected"].as_bool().unwrap()
    );
}
