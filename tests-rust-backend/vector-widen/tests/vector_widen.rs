use compact_rust_vector_widen_fixture::pure_circuits::{
    hash_nested, hash_widened, widen, widen_elements,
};
use midnight_compact_runtime::{BoundedUint, Field, FixedVector};

#[test]
fn vector_uint_to_field_coercion_matches_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/vector-widen.json"
    ))
    .unwrap();
    let values = FixedVector::new([
        BoundedUint::<255>::new(7).unwrap(),
        BoundedUint::<255>::new(255).unwrap(),
    ]);
    let widened = widen(values.clone()).unwrap();
    assert_eq!(
        widened,
        FixedVector::new([Field::from(7_u64), Field::from(255_u64)])
    );
    assert_eq!(oracle["widened"], serde_json::json!(["7", "255"]));

    let elements = widen_elements(BoundedUint::<4294967295>::new(4294967295).unwrap()).unwrap();
    assert_eq!(elements, FixedVector::new([Field::from(4294967295_u64); 2]));
    assert_eq!(
        oracle["elements"],
        serde_json::json!(["4294967295", "4294967295"])
    );
    assert_eq!(
        hex::encode(hash_widened(values.clone()).unwrap().into_array()),
        oracle["hashHex"]
    );
    assert_eq!(
        hex::encode(hash_nested(values).unwrap().into_array()),
        oracle["nestedHashHex"]
    );
}
