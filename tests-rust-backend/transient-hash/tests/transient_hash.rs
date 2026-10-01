use compact_rust_transient_hash_fixture::pure_circuits::{
    commit_field, commit_pair, hash_bytes, hash_field, hash_pair,
};
use midnight_compact_runtime::{Field, FixedBytes, FixedVector};

#[test]
fn generated_hashes_match_generated_typescript_and_ledger_wasm() {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/transient-hash.json"
    ))
    .unwrap();
    let pair = FixedVector::new([Field::from(3_u64), Field::from(5_u64)]);
    let cases = [
        ("field", hash_field(Field::from(42_u64)).unwrap()),
        (
            "commitField",
            commit_field(Field::from(42_u64), Field::from(7_u64)).unwrap(),
        ),
        ("pair", hash_pair(pair.clone()).unwrap()),
        ("commitPair", commit_pair(pair, Field::from(7_u64)).unwrap()),
        ("bytes", hash_bytes(FixedBytes::new([1, 2, 0, 0])).unwrap()),
    ];
    for (name, actual) in cases {
        let bytes = hex::decode(expected[name].as_str().unwrap()).unwrap();
        assert_eq!(actual.as_le_bytes().as_slice(), bytes, "{name}");
    }
}
