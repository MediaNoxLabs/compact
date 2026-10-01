use compact_rust_witness_ledger_list_fixture::ledger_contract::{
    LedgerView, Witnesses, drop_first, initial_state, prepend, private_first_is_42,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

struct FirstIs42;

impl Witnesses<u64> for FirstIs42 {
    fn first_is_42(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, bool) {
        let list = context.ledger.items().unwrap();
        let head = list.head().unwrap();
        let empty = list.is_empty();
        assert_eq!(empty, list.length().unwrap().value() == 0);
        assert_eq!(empty, head.is_none());
        (
            *context.private_state + 1,
            head == Some(Field::from(42_u64)),
        )
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
fn witness_reads_current_typed_list() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-ledger-list-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let before = private_first_is_42(context, &FirstIs42).unwrap();
    assert_oracle_output(&before, &oracle["before"]);
    let write = prepend(before.context, Field::from(42_u64)).unwrap();
    assert!(write.private_transcript_outputs.is_empty());
    let after = private_first_is_42(write.context, &FirstIs42).unwrap();
    assert_oracle_output(&after, &oracle["after"]);
    let write = prepend(after.context, Field::from(7_u64)).unwrap();
    let covered = private_first_is_42(write.context, &FirstIs42).unwrap();
    assert_oracle_output(&covered, &oracle["covered"]);
    let write = drop_first(covered.context).unwrap();
    let restored = private_first_is_42(write.context, &FirstIs42).unwrap();
    assert_oracle_output(&restored, &oracle["restored"]);
}
