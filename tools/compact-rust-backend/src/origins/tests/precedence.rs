#![cfg(unix)]
use super::fixture::{self, Sandbox};
use crate::RenderError;
use crate::{MetadataStatus, NativeFailureCause, RenderOptions};
use serde_json::json;

#[test]
fn malformed_metadata_is_pending_when_native_semantics_succeeds() {
    let sandbox = Sandbox::new(&fixture::empty_ir(), Some(b"[]"), None, 0);
    let artifact = sandbox.invoke();
    let rendered = artifact.render(RenderOptions::default()).unwrap();
    assert!(matches!(rendered.metadata(), MetadataStatus::Invalid(_)));
    assert!(!rendered.rendered().source.is_empty());
    assert!(rendered.finish_metadata().is_err());
    assert!(artifact.promote_into(&rendered, &sandbox.root).is_err());
}

#[test]
fn semantic_typed_refusal_wins_over_malformed_origins_and_proof_metadata() {
    let ir = fixture::pure(
        json!({"kind":"parameter","name":"missing"}),
        json!({"kind":"field"}),
    );
    let sandbox = Sandbox::new(&ir, Some(b"[]"), Some(b"broken proof metadata"), 0);
    let artifact = sandbox.invoke();
    let failure = match artifact.render(RenderOptions {
        apply_frontend_proof_metadata: true,
        require_recording: true,
    }) {
        Ok(_) => panic!("unknown parameter must fail"),
        Err(error) => error,
    };
    assert!(
        matches!(failure.cause,NativeFailureCause::Render(RenderError::UnknownParameter(ref name)) if name=="missing")
    );
    assert!(matches!(failure.metadata, MetadataStatus::Invalid(_)));
}

#[test]
fn schema_refusal_is_not_masked_by_stale_but_structurally_valid_metadata() {
    let ir = fixture::empty_ir();
    let seed = Sandbox::new(&ir, None, None, 0);
    let mut raw: serde_json::Value =
        serde_json::from_slice(&fixture::metadata(&ir, &seed.source, false)).unwrap();
    raw["addresses"][1][1] = json!({"field":"not_a_real_field"});
    let mut changed = ir.clone();
    changed["schema_version"] = json!(99);
    let sandbox = Sandbox::new(&changed, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
    let failure = match sandbox.invoke().render(RenderOptions::default()) {
        Ok(_) => panic!("schema"),
        Err(error) => error,
    };
    assert!(matches!(
        failure.cause,
        NativeFailureCause::Render(RenderError::SchemaVersion(99))
    ));
    assert!(matches!(
        failure.metadata,
        MetadataStatus::Invalid(crate::MetadataProblem::Binding(_))
    ));
}

#[test]
fn require_recording_failure_wins_before_metadata_publication_problem() {
    // Exact current checked_interface_refusal_has_no_fallback_body_or_return_dispatch
    // control: CounterRead is native-supported, but an unused Vector<Field>
    // parameter exceeds the DEFAULT recording layout policy (32768 slots).
    // The layout limit is separate from native traversal of the compact Type.
    let ir = json!({
        "schema_version":20,
        "ledger_fields":[{"id":"round","index":0,"path":[0],"declaration":{"kind":"counter"}}],
        "circuits":[],
        "stateful_circuits":[{
            "name":"probe",
            "parameters":[{"name":"oversized","ty":{"kind":"vector","element":{"kind":"field"},"length":32769}}],
            "actions":[],
            "result":{"kind":"unsigned","max":"18446744073709551615"},
            "return_value":{"kind":"counter_read","field":"round","index":0}
        }]
    });
    let sandbox = Sandbox::new(
        &ir,
        Some(b"[]"),
        Some(br#"{"circuits":[{"name":"probe","proof":true}]}"#),
        0,
    );
    let failure = match sandbox.invoke().render(RenderOptions {
        apply_frontend_proof_metadata: true,
        require_recording: true,
    }) {
        Ok(_) => panic!("recorded fixed layout exceeds DEFAULT limit"),
        Err(error) => error,
    };
    assert!(
        matches!(failure.cause,NativeFailureCause::RequireRecording(ref requirements) if requirements.len()==1 && requirements[0].circuit=="probe")
    );
    assert!(matches!(failure.metadata, MetadataStatus::Invalid(_)));
}

#[test]
fn ordinary_originless_backend_api_output_is_exactly_preserved() {
    let ir = fixture::pure(
        json!({"kind":"field_literal","value":"1"}),
        json!({"kind":"field"}),
    );
    let expected = crate::render(&serde_json::from_value(ir.clone()).unwrap()).unwrap();
    let sandbox = Sandbox::new(&ir, None, None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    assert_eq!(rendered.rendered().source, expected);
}
