use compact_rust_uint_compare_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, witnessed_less,
};
use compact_rust_uint_compare_fixture::pure_circuits::{
    greater, greater_equal, less, less_equal, mixed_width,
};
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

type U16 = BoundedUint<65535>;
type U8 = BoundedUint<255>;

struct Echo;

impl Witnesses<u64> for Echo {
    fn echo(&self, context: WitnessContext<'_, u64, LedgerView<'_>>, value: U16) -> (u64, U16) {
        (
            *context.private_state + 1,
            U16::new(value.value() + u128::from(*context.private_state)).unwrap(),
        )
    }
}

#[test]
fn generated_uint_comparisons_match_typescript_including_witness_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/uint-compare.json"
    ))
    .unwrap();
    let u = |value| U16::new(value).unwrap();
    for (name, actual) in [
        ("less", less(u(7), u(9)).unwrap()),
        ("lessEqualSame", less_equal(u(7), u(7)).unwrap()),
        ("lessEqualDifferent", less_equal(u(9), u(7)).unwrap()),
        ("greater", greater(u(9), u(7)).unwrap()),
        ("greaterEqualSame", greater_equal(u(7), u(7)).unwrap()),
        ("greaterEqualDifferent", greater_equal(u(7), u(9)).unwrap()),
        (
            "mixedWidth",
            mixed_width(U8::new(8).unwrap(), u(300)).unwrap(),
        ),
    ] {
        assert_eq!(actual, oracle[name].as_bool().unwrap(), "{name}");
    }

    for (name, left, right) in [("witnessTrue", 2, 3), ("witnessFalse", 3, 2)] {
        let context = initial_state(ConstructorContext::new(7_u64))
            .into_circuit_context(ContractAddress::default());
        let result = witnessed_less(context, &Echo, u(left), u(right)).unwrap();
        let expected = &oracle[name];
        assert_eq!(result.result, expected["result"].as_bool().unwrap());
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
