use compact_rust_equality_fixture::ledger_contract::{
    LedgerView, Witnesses, equal_echo, initial_state,
};
use compact_rust_equality_fixture::pure_circuits::{
    equal_bytes, equal_field, equal_pair, not_equal_field,
};
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
fn typed_equality_matches_typescript_for_scalars_and_composites() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/equality.json"
    ))
    .unwrap();
    let a = Field::from(42_u64);
    let b = Field::from(43_u64);
    let pair = FixedVector::new([Field::from(3_u64), Field::from(5_u64)]);
    let pairs = [
        ("fieldEqual", equal_field(a, a).unwrap()),
        ("fieldDifferent", equal_field(a, b).unwrap()),
        ("notEqual", not_equal_field(a, b).unwrap()),
        (
            "bytesEqual",
            equal_bytes(FixedBytes::new([1, 2, 3, 4]), FixedBytes::new([1, 2, 3, 4])).unwrap(),
        ),
        (
            "bytesDifferent",
            equal_bytes(FixedBytes::new([1, 2, 3, 4]), FixedBytes::new([1, 2, 3, 5])).unwrap(),
        ),
        ("pairEqual", equal_pair(pair.clone(), pair).unwrap()),
        (
            "pairDifferent",
            equal_pair(
                FixedVector::new([Field::from(3_u64), Field::from(5_u64)]),
                FixedVector::new([Field::from(3_u64), Field::from(6_u64)]),
            )
            .unwrap(),
        ),
    ];
    for (name, actual) in pairs {
        assert_eq!(actual, oracle[name].as_bool().unwrap(), "{name}");
    }
    for (name, left, right) in [("witnessEqual", 2_u64, 1_u64), ("witnessDifferent", 2, 2)] {
        let context = initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let result = equal_echo(context, &Echo, Field::from(left), Field::from(right)).unwrap();
        let expected = &oracle[name];
        assert_eq!(
            result.result,
            expected["result"].as_bool().unwrap(),
            "{name}"
        );
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
}
