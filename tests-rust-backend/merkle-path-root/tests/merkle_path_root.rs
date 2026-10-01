use compact_rust_merkle_path_root_fixture::pure_circuits::root_of;
use compact_rust_merkle_path_root_fixture::types::{
    MerkleTreeDigest, MerkleTreePath, MerkleTreePathEntry,
};
use midnight_compact_runtime as runtime;

fn decimal_field(value: &serde_json::Value) -> runtime::Field {
    let decimal = value.as_str().unwrap();
    let integer = decimal.parse::<num_bigint::BigUint>().unwrap();
    runtime::Field::from_le_bytes(&integer.to_bytes_le()).unwrap()
}

fn path(value: &serde_json::Value) -> MerkleTreePath {
    let leaf = value["leaf"].as_str().unwrap().parse::<u128>().unwrap();
    let entries = value["path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| MerkleTreePathEntry {
            sibling: MerkleTreeDigest {
                field: decimal_field(&entry["sibling"]),
            },
            goes_left: entry["goesLeft"].as_bool().unwrap(),
        })
        .collect::<Vec<_>>();
    MerkleTreePath {
        leaf: runtime::BoundedUint::<255>::new(leaf).unwrap(),
        path: runtime::FixedVector::new(entries.try_into().unwrap()),
    }
}

#[test]
fn compact_stdlib_merkle_path_root_matches_typescript_and_ledger_primitive() {
    let paths: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
    ))
    .unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-path-root.json"
    ))
    .unwrap();
    for (path_key, root_key) in [
        ("pathFor7At0", "rootFor7At0"),
        ("pathFor9At3", "rootFor9At3"),
    ] {
        let compact_path = path(&paths[path_key]);
        let root = root_of(compact_path.clone()).unwrap();
        assert_eq!(root.field, compact_path.into_ledger_path().root().0);
        let decimal = num_bigint::BigUint::from_bytes_le(&root.field.as_le_bytes());
        assert_eq!(decimal.to_string(), oracle[root_key]);
    }
}
