use compact_rust_truth_fixture::pure_circuits::truth;

#[test]
fn boolean_literal_survives_the_full_compiler_pipeline() {
    assert!(truth().unwrap());
}
