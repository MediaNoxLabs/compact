use compact_rust_merkle_path_witness_fixture::ledger_contract::{
    LedgerView, Witnesses, append, get_path, initial_state,
};
use compact_rust_merkle_path_witness_fixture::types::MerkleTreePath;
use midnight_compact_runtime as runtime;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::ContractAddress;

struct PathWitness;

impl Witnesses<()> for PathWitness {
    fn leaf_path(&self, context: WitnessContext<'_, (), LedgerView<'_>>) -> ((), MerkleTreePath) {
        let tree = context.ledger.t().unwrap();
        let path = tree
            .path_for_leaf(0, runtime::BoundedUint::<255>::new(7).unwrap())
            .unwrap();
        assert_eq!(Some(path.root()), tree.root());
        let compact = MerkleTreePath::from_ledger_path(path).unwrap();
        assert_eq!(Some(compact.clone().into_ledger_path().root()), tree.root());
        ((), compact)
    }
}

#[test]
fn generated_path_conversions_match_typescript_witness_output() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-path-witness.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let after_insert = append(context, runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();
    let output = get_path(after_insert.context, &PathWitness).unwrap();
    assert_eq!(output.result.leaf.value().to_string(), oracle["leaf"]);
    assert_eq!(
        output.result.path.0.len(),
        oracle["path"].as_array().unwrap().len()
    );
    for (actual, expected) in output
        .result
        .path
        .0
        .iter()
        .zip(oracle["path"].as_array().unwrap())
    {
        let sibling = num_bigint::BigUint::from_bytes_le(&actual.sibling.field.as_le_bytes());
        assert_eq!(sibling.to_string(), expected["sibling"]);
        assert_eq!(actual.goes_left, expected["goesLeft"]);
    }
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
    assert_eq!(output.private_transcript_outputs.len(), 1);

    let invalid = runtime::ledger::MerklePath {
        leaf: runtime::BoundedUint::<255>::new(7).unwrap(),
        path: Vec::new(),
    };
    assert!(MerkleTreePath::from_ledger_path(invalid).is_err());
}
