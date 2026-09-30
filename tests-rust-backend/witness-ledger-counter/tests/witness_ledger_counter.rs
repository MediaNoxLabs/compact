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
        .into_circuit_context(ContractAddress::default());
    let before = private_round(context, &ReadRound).unwrap();
    assert_oracle_output(&before, &oracle["before"]);
    let write = increment_round(before.context).unwrap();
    assert!(write.private_transcript_outputs.is_empty());
    let after = private_round(write.context, &ReadRound).unwrap();
    assert_oracle_output(&after, &oracle["after"]);
}
