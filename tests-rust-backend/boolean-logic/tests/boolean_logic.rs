use compact_rust_boolean_logic_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, witnessed_both, witnessed_either, witnessed_not,
};
use compact_rust_boolean_logic_fixture::pure_circuits::{both, either, invert};
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

struct Echo;

impl Witnesses<u64> for Echo {
    fn echo(&self, context: WitnessContext<'_, u64, LedgerView<'_>>, value: bool) -> (u64, bool) {
        (*context.private_state + 1, value)
    }
}

fn context() -> midnight_compact_runtime::context::CircuitContext<u64> {
    initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}

fn assert_case(result: CircuitResult<u64, bool>, expected: &serde_json::Value) {
    assert_eq!(result.result, expected["result"]);
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

#[test]
fn normalized_boolean_conditionals_preserve_short_circuit_witness_effects() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/boolean-logic.json"
    ))
    .unwrap();
    let pure = &oracle["pure"];
    for (name, actual) in [
        ("invertTrue", invert(true).unwrap()),
        ("invertFalse", invert(false).unwrap()),
        ("bothTrue", both(true, true).unwrap()),
        ("bothFalse", both(false, true).unwrap()),
        ("eitherTrue", either(true, false).unwrap()),
        ("eitherFalse", either(false, false).unwrap()),
    ] {
        assert_eq!(actual, pure[name], "{name}");
    }
    assert_case(
        witnessed_both(context(), &Echo, false, true).unwrap(),
        &oracle["andSkip"],
    );
    assert_case(
        witnessed_both(context(), &Echo, true, false).unwrap(),
        &oracle["andBoth"],
    );
    assert_case(
        witnessed_either(context(), &Echo, true, false).unwrap(),
        &oracle["orSkip"],
    );
    assert_case(
        witnessed_either(context(), &Echo, false, true).unwrap(),
        &oracle["orBoth"],
    );
    assert_case(
        witnessed_not(context(), &Echo, true).unwrap(),
        &oracle["notTrue"],
    );
}
