use compact_rust_witness_minimal_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, read_private,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::ContractAddress;

struct PrivateValue;

impl Witnesses<u64> for PrivateValue {
    fn private_value(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, Field) {
        assert_eq!(*context.private_state, 7);
        assert_eq!(*context.contract_address, ContractAddress::default());
        let _ledger = context.ledger;
        (8, Field::from(42_u64))
    }
}

#[test]
fn generated_witness_updates_private_state_and_records_aligned_output() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .into_circuit_context(ContractAddress::default());
    let result = read_private(context, &PrivateValue).unwrap();
    assert_eq!(result.result, Field::from(42_u64));
    assert_eq!(result.context.private_state, 8);
    let oracle_result: u64 = oracle["result"].as_str().unwrap().parse().unwrap();
    assert_eq!(result.result, Field::from(oracle_result));
    assert_eq!(oracle["privateState"], result.context.private_state);
    assert_eq!(
        result.private_transcript_outputs,
        vec![AlignedValue::from(Field::from(42_u64))]
    );
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
