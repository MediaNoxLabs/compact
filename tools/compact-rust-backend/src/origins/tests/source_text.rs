use crate::origins::{source_text::decode_compact_source, transport};

#[test]
fn decoded_text_preserves_actual_unicode_bom_replacement_and_lf_control_meaning() {
    let controls: [(&[u8], &[u32]); 4] = [
        (b"A\xc3\xa9\xf0\x9d\x84\x9eB", &[65, 233, 119070, 66]),
        (
            b"A\rB\r\nC\xc2\x85D\r\xc2\x85E\xe2\x80\xa8F\xe2\x80\xa9G\nH",
            &[65, 10, 66, 10, 67, 10, 68, 10, 69, 10, 70, 8233, 71, 10, 72],
        ),
        (b"\xef\xbb\xbfA\xef\xbb\xbfB", &[65, 65279, 66]),
        (
            b"A\xffB\xc0\xafC\xe2\x82D",
            &[65, 65533, 66, 65533, 65533, 67, 65533, 68],
        ),
    ];
    for (raw, expected) in controls {
        assert_eq!(
            decode_compact_source(raw)
                .chars()
                .map(u32::from)
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[cfg(unix)]
#[test]
fn compiler_context_is_never_a_physical_range_and_source_snapshot_is_explicitly_post_invocation() {
    let ir = super::fixture::empty_ir();
    let seed = super::fixture::Sandbox::new(&ir, None, None, 0);
    let raw = super::fixture::metadata(&ir, &seed.source, true);
    let sandbox = super::fixture::Sandbox::new(&ir, Some(&raw), None, 0);
    let artifact = sandbox.invoke();
    let rendered = artifact.render(crate::RenderOptions::default()).unwrap();
    let crate::MetadataStatus::Available(summary) = rendered.metadata() else {
        panic!("binding available")
    };
    assert_eq!(summary.compiler_context_ranges, 1);
    assert_eq!(summary.physical_ranges_matching_snapshot, 0);
    assert_eq!(summary.post_invocation_source_snapshots, 1);
    assert_eq!(
        summary.post_invocation_snapshot_fingerprints[0].raw_sha256,
        crate::origins::frontend::digest(b"x")
    );
    std::fs::write(&seed.source, b"different after frontend").unwrap();
    let rerendered = artifact.render(crate::RenderOptions::default()).unwrap();
    let crate::MetadataStatus::Available(changed) = rerendered.metadata() else {
        panic!("still binding available")
    };
    assert_ne!(
        changed.post_invocation_snapshot_fingerprints[0].raw_sha256,
        summary.post_invocation_snapshot_fingerprints[0].raw_sha256
    );
    assert_eq!(
        rerendered.rendered().source,
        rendered.rendered().source,
        "source snapshot changes do not rerun/change native admission"
    );
}

#[cfg(unix)]
#[test]
fn source_snapshot_budget_and_missing_file_do_not_reject_valid_metadata_or_native_code() {
    let ir = super::fixture::empty_ir();
    let seed = super::fixture::Sandbox::new(&ir, None, None, 0);
    let raw = super::fixture::metadata(&ir, &seed.source, false);
    std::fs::write(&seed.source, vec![b'x'; transport::MAX_BYTES + 1]).unwrap();
    let sandbox = super::fixture::Sandbox::new(&ir, Some(&raw), None, 0);
    let artifact = sandbox.invoke();
    for remove in [false, true] {
        if remove {
            std::fs::remove_file(&seed.source).unwrap();
        }
        let rendered = artifact.render(crate::RenderOptions::default()).unwrap();
        assert!(
            matches!(rendered.metadata(),crate::MetadataStatus::Available(summary) if summary.post_invocation_source_snapshots==0 && summary.source_snapshots_unavailable==1)
        );
        rendered.finish_metadata().unwrap();
    }
}

#[cfg(unix)]
#[test]
fn invalid_utf8_snapshot_is_encoding_unverified_without_endpoint_match_or_native_refusal() {
    let ir = super::fixture::empty_ir();
    let seed = super::fixture::Sandbox::new(&ir, None, None, 0);
    let raw = super::fixture::metadata(&ir, &seed.source, false);
    // The declared range covers only the valid leading "A". A lossy decoder
    // could match its endpoints despite the rest of this source being invalid.
    std::fs::write(&seed.source, b"A\xffB").unwrap();
    let sandbox = super::fixture::Sandbox::new(&ir, Some(&raw), None, 0);
    let rendered = sandbox
        .invoke()
        .render(crate::RenderOptions::default())
        .unwrap();
    let crate::MetadataStatus::Available(summary) = rendered.metadata() else {
        panic!("valid frontend metadata remains bound")
    };
    assert_eq!(summary.post_invocation_source_snapshots, 0);
    assert_eq!(summary.source_snapshots_unavailable, 1);
    assert_eq!(summary.physical_ranges_matching_snapshot, 0);
    assert_eq!(summary.physical_ranges_unverified, 1);
    assert!(summary.post_invocation_snapshot_fingerprints.is_empty());
    let expected = crate::render(&serde_json::from_value(ir).unwrap()).unwrap();
    assert_eq!(
        rendered.rendered().source,
        expected,
        "encoding-unverified snapshot cannot alter native success"
    );
    rendered.finish_metadata().unwrap();
}
