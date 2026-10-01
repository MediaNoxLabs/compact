use compact_rust_merkle_path_verify_fixture::ledger_contract::{
    LedgerView, Witnesses, append, initial_state, replace, verify,
};
use compact_rust_merkle_path_verify_fixture::types::MerkleTreePath;
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use runtime::ledger::ContractAddress;

struct PathWitness;

impl Witnesses<()> for PathWitness {
    fn leaf_path(&self, context: WitnessContext<'_, (), LedgerView<'_>>) -> ((), MerkleTreePath) {
        let path = context
            .ledger
            .t()
            .unwrap()
            .path_for_leaf(0, runtime::BoundedUint::<255>::new(7).unwrap())
            .unwrap();
        ((), MerkleTreePath::from_ledger_path(path).unwrap())
    }
}

fn assert_oracle_output(output: &CircuitResult<(), bool>, oracle: &serde_json::Value) {
    assert_eq!(output.result, oracle["result"]);
    assert_eq!(output.private_transcript_outputs.len(), 1);
    let transcript = &output.private_transcript_outputs[0];
    let atoms = transcript
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
        serde_json::to_value(&transcript.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
}

#[test]
fn witness_merkle_root_check_matches_typescript_before_and_after_replacement() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-path-verify.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let after_append = append(context, runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();
    let valid = verify(after_append.context, &PathWitness).unwrap();
    assert_oracle_output(&valid, &oracle["afterAppend"]);
    let after_replace =
        replace(valid.context, runtime::BoundedUint::<255>::new(8).unwrap()).unwrap();
    let invalid = verify(after_replace.context, &PathWitness).unwrap();
    assert_oracle_output(&invalid, &oracle["afterReplace"]);
}
