use crate::origins::{
    status::{MetadataState, MetadataStatus, MetadataUnavailable},
    transport,
};
use serde_json::json;

#[test]
fn fixed_producer_budget_and_numeric_width_refusals_are_unavailable_not_invalid() {
    for (error, expected) in [
        (
            json!({"resource":"origin_bytes","observed":4194305,"limit":4194304}),
            MetadataUnavailable::Budget {
                resource: "origin_bytes",
                limit: 4194304,
                observed_at_least: 4194305,
            },
        ),
        (
            json!({"resource":"origin_numeric_width","observed":"outside_u64","limit":"18446744073709551615"}),
            MetadataUnavailable::NumericWidth,
        ),
    ] {
        let raw = serde_json::to_vec(&json!({"schema_version":3,"coordinate_system":"compact-decoded-source","end_exclusive":true,"error":error})).unwrap();
        assert_eq!(
            MetadataState::read(raw.as_slice()).prior_status(),
            MetadataStatus::Unavailable(expected)
        );
    }
    let forged = br#"{"schema_version":3,"coordinate_system":"compact-decoded-source","end_exclusive":true,"error":{"resource":"origin_bytes","observed":3,"limit":4}}"#;
    assert!(matches!(
        MetadataState::read(forged.as_slice()).prior_status(),
        MetadataStatus::Invalid(_)
    ));
}

#[test]
fn reader_byte_cap_consumes_only_one_sentinel_without_changing_metadata_budget() {
    let bytes = vec![b' '; transport::MAX_BYTES + 16];
    let mut cursor = std::io::Cursor::new(bytes);
    let state = MetadataState::read(&mut cursor);
    assert_eq!(cursor.position(), transport::MAX_BYTES as u64 + 1);
    assert!(matches!(
        state.prior_status(),
        MetadataStatus::Unavailable(MetadataUnavailable::Budget {
            resource: "origin_bytes",
            ..
        })
    ));
}

#[test]
fn decoder_table_cap_is_typed_and_cannot_be_forged_by_an_unknown_tag() {
    let rows = vec![json!([0,{"index":0},null,null]); transport::MAX_ENTRIES + 1];
    let raw = serde_json::to_vec(&json!({"schema_version":3,"coordinate_system":"compact-decoded-source","end_exclusive":true,
        "files":[],"spans":[],"origins":[],"addresses":rows})).unwrap();
    assert!(
        raw.len() < transport::MAX_BYTES,
        "test reaches row cap, not byte cap"
    );
    assert!(matches!(
        MetadataState::read(raw.as_slice()).prior_status(),
        MetadataStatus::Unavailable(MetadataUnavailable::Budget {
            resource: "origin_table_entries",
            ..
        })
    ));
    let forged = br#"{"schema_version":3,"coordinate_system":"origin table entry limit exceeded","end_exclusive":true,"files":[],"spans":[],"origins":[],"addresses":[]}"#;
    assert!(matches!(
        MetadataState::read(forged.as_slice()).prior_status(),
        MetadataStatus::Invalid(_)
    ));
}

#[cfg(unix)]
#[test]
fn oversized_optional_companion_keeps_native_success_and_cannot_claim_a_pair_hash() {
    let sandbox = super::fixture::Sandbox::new(
        &super::fixture::empty_ir(),
        Some(&vec![b' '; transport::MAX_BYTES + 1]),
        None,
        0,
    );
    let artifact = sandbox.invoke();
    let rendered = artifact.render(crate::RenderOptions::default()).unwrap();
    assert!(matches!(
        rendered.metadata(),
        MetadataStatus::Unavailable(MetadataUnavailable::Budget {
            resource: "origin_bytes",
            ..
        })
    ));
    assert!(artifact.fingerprint().companion_sha256.is_none());
    rendered.finish_metadata().unwrap();
}
