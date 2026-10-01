use compact_rust_witness_ledger_cell_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, private_check, set_flag,
};
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

struct ReadFlag;

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
