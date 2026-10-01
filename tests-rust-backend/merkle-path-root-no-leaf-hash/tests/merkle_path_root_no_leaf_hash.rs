use compact_rust_merkle_path_root_no_leaf_hash_fixture::pure_circuits::root_of_hash;
use compact_rust_merkle_path_root_no_leaf_hash_fixture::types::{
    MerkleTreeDigest, MerkleTreePath, MerkleTreePathEntry,
};
use midnight_compact_runtime as runtime;

fn decimal_field(value: &serde_json::Value) -> runtime::Field {
    let decimal = value.as_str().unwrap();
    let integer = decimal.parse::<num_bigint::BigUint>().unwrap();
    runtime::Field::from_le_bytes(&integer.to_bytes_le()).unwrap()
}

fn path(value: &serde_json::Value) -> MerkleTreePath {
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
        leaf: runtime::FixedBytes::new([1; 32]),
        path: runtime::FixedVector::new(entries.try_into().unwrap()),
    }
}

#[test]
fn compact_stdlib_raw_hash_path_root_matches_typescript() {
    let paths: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
    ))
    .unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-path-root-no-leaf-hash.json"
    ))
    .unwrap();
    for (path_key, root_key) in [
        ("pathFor7At0", "rootWithZeroSiblings"),
        ("pathFor9At3", "rootWithNonzeroSibling"),
    ] {
        let compact_path = path(&paths[path_key]);
        let root = root_of_hash(compact_path.clone()).unwrap();
        let mut expected = runtime::degrade_to_transient(compact_path.leaf);
        for entry in compact_path.path.0 {
            expected = runtime::transient_hash(if entry.goes_left {
                (expected, entry.sibling.field)
            } else {
                (entry.sibling.field, expected)
            });
        }
        assert_eq!(root.field, expected);
        let decimal = num_bigint::BigUint::from_bytes_le(&root.field.as_le_bytes());
        assert_eq!(decimal.to_string(), oracle[root_key]);
    }
}
