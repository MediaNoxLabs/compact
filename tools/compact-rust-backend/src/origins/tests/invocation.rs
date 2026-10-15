#![cfg(unix)]
use super::fixture::{self, Sandbox};
use crate::{
    ConfiguredFrontend, FrontendError, MetadataStatus, MetadataUnavailable, RenderOptions,
};
use std::fs;

#[test]
fn successful_configured_invocation_uses_exact_args_private_receipt_and_validated_binding() {
    let ir = fixture::empty_ir();
    let sandbox = Sandbox::new(&ir, None, Some(br#"{"circuits":[]}"#), 0);
    let raw = fixture::metadata(&ir, &sandbox.source, false);
    // The test program is configured before invocation; production can issue
    // only after actually running it into a fresh child stage.
    let sandbox = Sandbox::new(&ir, Some(&raw), Some(br#"{"circuits":[]}"#), 0);
    let artifact = sandbox.invoke();
    assert_ne!(artifact.output_dir(), sandbox.root);
    let args = fs::read_to_string(sandbox.root.join("args.txt")).unwrap();
    assert!(args.contains("--skip-zk\n--emit-rust-ir\n"));
    assert!(args.contains(sandbox.source.to_str().unwrap()));
    let fingerprint = artifact.fingerprint();
    assert_eq!(
        fingerprint.exact_ir_sha256,
        crate::origins::frontend::digest(&serde_json::to_vec(&ir).unwrap())
    );
    assert_eq!(
        fingerprint.companion_sha256,
        Some(crate::origins::frontend::digest(&raw))
    );
    assert!(fingerprint.configured_executable_sha256.is_some());
    let rendered = artifact
        .render(RenderOptions {
            apply_frontend_proof_metadata: true,
            require_recording: false,
        })
        .unwrap();
    assert!(
        matches!(rendered.metadata(), MetadataStatus::Available(summary) if summary.addresses > 1 && summary.typed_slots > 1)
    );
    assert_eq!(rendered.rendered().capabilities.schema_version, 3);
    rendered.finish_metadata().unwrap();
}

#[test]
fn invocation_never_adopts_stale_companion_from_the_parent_output() {
    let sandbox = Sandbox::new(&fixture::empty_ir(), None, None, 0);
    fs::create_dir(sandbox.root.join("contract")).unwrap();
    fs::write(
        sandbox.root.join("contract/compact-rust-origins.raw.json"),
        b"stale",
    )
    .unwrap();
    let artifact = sandbox.invoke();
    let rendered = artifact.render(RenderOptions::default()).unwrap();
    assert_eq!(
        rendered.metadata(),
        &MetadataStatus::Unavailable(MetadataUnavailable::Missing)
    );
    assert!(artifact.fingerprint().companion_sha256.is_none());
    rendered.finish_metadata().unwrap();
}

#[test]
fn failed_process_never_issues_and_fresh_stage_is_cleaned() {
    let sandbox = Sandbox::new(&fixture::empty_ir(), Some(b"stale"), None, 19);
    let result = ConfiguredFrontend::new(sandbox.executable.as_os_str()).invoke(sandbox.request());
    assert!(matches!(
        result,
        Err(FrontendError::ProcessExit { code: Some(19) })
    ));
    assert!(!fs::read_dir(&sandbox.root).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".compact-origin-invocation-")
    }));
}

#[test]
fn dropping_artifacts_removes_the_unpublished_invocation_output() {
    let sandbox = Sandbox::new(&fixture::empty_ir(), None, None, 0);
    let artifact = sandbox.invoke();
    let stage = artifact.output_dir().to_path_buf();
    assert!(stage.is_dir());
    drop(artifact);
    assert!(!stage.exists());
}

#[test]
fn changed_pair_cannot_be_promoted_but_retained_render_bytes_stay_exact() {
    let ir = fixture::empty_ir();
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = fixture::metadata(&ir, &seed.source, false);
    let sandbox = Sandbox::new(&ir, Some(&raw), None, 0);
    let artifact = sandbox.invoke();
    let rendered = artifact.render(RenderOptions::default()).unwrap();
    let path = artifact
        .output_dir()
        .join("contract/compact-rust-origins.raw.json");
    let mut changed = raw.clone();
    changed.push(b' ');
    fs::write(path, &changed).unwrap();
    assert!(
        artifact.render(RenderOptions::default()).is_ok(),
        "in-memory pair retains its exact issuance bytes"
    );
    assert!(
        matches!(artifact.promote_into(&rendered,&sandbox.root),Err(FrontendError::Publication(message)) if message.contains("PairChanged"))
    );
}

#[test]
fn successful_promote_requires_matching_render_and_preserves_pair() {
    let sandbox = Sandbox::new(&fixture::empty_ir(), None, Some(br#"{"circuits":[]}"#), 0);
    let artifact = sandbox.invoke();
    let rendered = artifact.render(RenderOptions::default()).unwrap();
    fs::write(
        artifact.output_dir().join("contract/lib.rs"),
        &rendered.rendered().source,
    )
    .unwrap();
    artifact.promote_into(&rendered, &sandbox.root).unwrap();
    assert!(sandbox.root.join("contract/lib.rs").is_file());
    assert!(sandbox.root.join("contract/compact-rust-ir.json").is_file());
}

#[test]
fn foreign_render_and_destination_conflicts_fail_before_promotion() {
    let first = Sandbox::new(&fixture::empty_ir(), None, None, 0);
    let second = Sandbox::new(&fixture::empty_ir(), None, None, 0);
    let a = first.invoke();
    let b = second.invoke();
    let rendered = a.render(RenderOptions::default()).unwrap();
    assert!(
        matches!(b.promote_into(&rendered,&second.root),Err(FrontendError::Publication(message)) if message.contains("another invocation"))
    );
    fs::create_dir(first.root.join("contract")).unwrap();
    fs::write(first.root.join("contract/marker"), "old").unwrap();
    assert!(a.promote_into(&rendered, &first.root).is_err());
    assert_eq!(
        fs::read_to_string(first.root.join("contract/marker")).unwrap(),
        "old"
    );
}
