use compact_rust_asset_registry_oracle_fixture::pure_circuits::registrationGap;
use compact_rust_asset_registry_oracle_fixture::types::AssetRecord;
use midnight_compact_runtime as runtime;

fn record(registered_at: u128) -> AssetRecord {
    let mut record = AssetRecord::default();
    record.provenance.registeredAt = runtime::BoundedUint::new(registered_at).unwrap();
    record
}

#[test]
fn nested_provenance_subtraction_keeps_distinct_compiler_temporaries() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-registry-pure.json"
    ))
    .unwrap();
    let gap = registrationGap(record(120), record(100)).unwrap();
    assert_eq!(gap.value().to_string(), oracle["gap"]);
    let error = registrationGap(record(100), record(120))
        .err()
        .expect("reversed order should fail");
    assert!(matches!(error, runtime::CompactError::AssertionFailed(_)));
    assert_eq!(error.to_string(), oracle["reverseError"]);
}
