use compact_rust_struct_field_projection_fixture::pure_circuits::{hash_projected, project};
use compact_rust_struct_field_projection_fixture::types::VecBox;
use midnight_compact_runtime::{Field, FixedVector};

#[test]
fn typed_struct_field_projection_matches_typescript_values_and_hash() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/struct-field-projection.json"
    ))
    .unwrap();
    let input = VecBox {
        values: FixedVector::new([Field::from(1_u64), Field::from(2_u64)]),
    };
    assert_eq!(project(input.clone()).unwrap(), input.values);
    assert_eq!(oracle["projected"], serde_json::json!(["1", "2"]));
    assert_eq!(
        hex::encode(hash_projected(input).unwrap().into_array()),
        oracle["hashHex"]
    );
}
