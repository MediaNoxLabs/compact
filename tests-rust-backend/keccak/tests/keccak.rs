use compact_rust_keccak_fixture::ledger_contract::{
    LedgerView, Witnesses, hash_echo, initial_state,
};
use compact_rust_keccak_fixture::pure_circuits::{hash_bytes, hash_field, hash_pair};
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;
use midnight_compact_runtime::{Field, FixedBytes, FixedVector};

struct Echo;

impl Witnesses<u64> for Echo {
    fn echo(&self, context: WitnessContext<'_, u64, LedgerView<'_>>, value: Field) -> (u64, Field) {
        (
            *context.private_state + 1,
            value + Field::from(*context.private_state),
        )
    }
}

#[test]
fn generated_keccak_matches_typescript_and_preserves_witness_transcript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/keccak.json"
    ))
    .unwrap();
    let cases = [
        ("field", hash_field(Field::from(42_u64)).unwrap()),
        ("bytes", hash_bytes(FixedBytes::new([1, 2, 3, 4])).unwrap()),
        (
            "pair",
            hash_pair(FixedVector::new([Field::from(3_u64), Field::from(5_u64)])).unwrap(),
        ),
    ];
    for (name, value) in cases {
        assert_eq!(
            hex::encode(value.0),
            oracle[name].as_str().unwrap(),
            "{name}"
        );
    }

    let context = initial_state(ConstructorContext::new(7_u64))
        .into_circuit_context(ContractAddress::default());
    let result = hash_echo(context, &Echo, Field::from(2_u64)).unwrap();
    let expected = &oracle["witness"];
    assert_eq!(hex::encode(result.result.0), expected["result"]);
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
