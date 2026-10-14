#![cfg(unix)]
//! Actual issuer/binder/Recorder lifecycle using a private configured test
//! frontend. The normalized duplicate enum IR is a negative recording fixture,
//! not an application crate or genuine Compact/proof acceptance claim.
use super::fixture::{self, Sandbox};
use crate::{MetadataStatus, RenderOptions, ir::Contract};
use serde_json::{Value, json};

#[test]
fn owned_named_table_refusal_stays_unavailable_in_real_recorder_with_physical_metadata() {
    let ir = json!({"schema_version":20,"ledger_fields":[],"circuits":[],"stateful_circuits":[{
        "name":"root","parameters":[{"name":"unused","ty":{"kind":"enum","name":"Status","variants":["a$b","a_b"]}}],
        "actions":[],"result":{"kind":"unit"},"return_value":{"kind":"unit"}
    }]});
    let typed: Contract = serde_json::from_value(ir.clone()).unwrap();
    let plain = crate::render_with_capabilities(&typed).unwrap();
    assert!(!plain.capabilities.circuits[0].recorded);
    let gap = plain.capabilities.circuits[0]
        .recording_unavailable
        .as_ref()
        .unwrap();
    assert_eq!(
        gap.detail,
        "named members collide after Rust identifier normalization"
    );
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = fixture::metadata(&ir, &seed.source, false);
    let parsed: Value = serde_json::from_slice(&raw).unwrap();
    assert!(
        parsed["addresses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row[2] == "enum" && !row[3].is_null())
    );
    let sandbox = Sandbox::new(&ir, Some(&raw), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    assert!(matches!(rendered.metadata(), MetadataStatus::Available(_)));
    rendered.finish_metadata().unwrap();
    assert_eq!(rendered.rendered().source, plain.source);
    assert_eq!(
        serde_json::to_value(&rendered.rendered().capabilities).unwrap(),
        serde_json::to_value(&plain.capabilities).unwrap()
    );
    let diagnostics = rendered.recording_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(&diagnostics[0].gap, gap);
    assert!(
        diagnostics[0].primary.is_none(),
        "late owned-table failure cannot guess a source from enum names"
    );
    assert!(diagnostics[0].related_call_sites.is_empty());
}
