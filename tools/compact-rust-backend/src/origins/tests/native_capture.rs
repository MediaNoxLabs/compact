#![cfg(unix)]
//! Configured-process issuer controls using explicit test frontend artifacts.
//! These are not genuine Compact source-origin completeness claims.
use super::fixture::{self, Sandbox};
use crate::{
    MetadataStatus, NativeCompileFailure, NativeFailureCause, RenderOptions, SourceDiagnostic,
    ir::Contract,
};
use serde_json::{Value, json};

mod declaration_preflight;
mod header_carrier;
mod named_provenance;
mod workers;

fn field() -> Value {
    json!({"kind":"field_literal","value":"1"})
}
fn boolean() -> Value {
    json!({"kind":"boolean","value":true})
}
fn missing(name: &str) -> Value {
    json!({"kind":"parameter","name":name})
}
fn pure(body: Value) -> Value {
    fixture::pure(body, json!({"kind":"field"}))
}
fn raw(ir: &Value, source: &std::path::Path) -> Value {
    serde_json::from_slice(&fixture::metadata(ir, source, false)).unwrap()
}
fn body() -> Vec<Value> {
    vec![
        json!({"field":"circuits"}),
        json!({"index":0}),
        json!({"field":"body"}),
    ]
}
fn child(field: &str) -> Vec<Value> {
    let mut path = body();
    path.push(json!({"field":field}));
    path
}
fn address(raw: &Value, path: &[Value]) -> usize {
    let mut parent = 0usize;
    for edge in path {
        parent = raw["addresses"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .find_map(|(id, row)| (row[0] == json!(parent) && row[1] == *edge).then_some(id))
            .expect("test fixture edge");
    }
    parent
}
fn failure(sandbox: &Sandbox) -> Box<NativeCompileFailure> {
    let contract: Contract =
        serde_json::from_slice(&std::fs::read(sandbox.root.join("ir.json")).unwrap()).unwrap();
    let expected = match crate::render(&contract) {
        Ok(_) => panic!("originless refusal expected"),
        Err(error) => error,
    };
    let failure = match sandbox.invoke().render(RenderOptions::default()) {
        Ok(_) => panic!("native refusal expected"),
        Err(error) => error,
    };
    assert_eq!(failure.to_string(), expected.to_string());
    let NativeFailureCause::Render(cause) = &failure.cause else {
        panic!("typed render cause")
    };
    assert_eq!(cause, &expected);
    failure
}
fn diagnostic(failure: &NativeCompileFailure) -> &SourceDiagnostic {
    assert!(matches!(failure.metadata, MetadataStatus::Available(_)));
    failure
        .diagnostic
        .as_deref()
        .expect("physical source diagnostic")
}

#[test]
fn early_control_selects_parent_node_and_retains_each_provenance_label() {
    let ir = pure(
        json!({"kind":"if","condition":field(),"then":missing("then"),"otherwise":missing("else")}),
    );
    let seed = Sandbox::new(&ir, None, None, 0);
    for label in ["provided", "derived", "inherited"] {
        let mut raw = raw(&ir, &seed.source);
        raw["origins"][0][1] = json!(label);
        let selected = address(&raw, &body());
        let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
        let failure = failure(&sandbox);
        let diagnostic = diagnostic(&failure);
        assert_eq!(diagnostic.address_id, selected);
        assert_eq!(diagnostic.provenance, label);
        assert_eq!(diagnostic.category, "expression");
        assert_eq!(diagnostic.selected_slot, "expr_node");
        assert!(diagnostic.end_exclusive);
        assert!(diagnostic.range_matches_post_invocation_snapshot);
        assert!(diagnostic.post_invocation_snapshot.is_some());
    }
}

#[test]
fn two_bad_children_select_left_before_right_and_parent() {
    let ir = pure(json!({"kind":"add","left":missing("left"),"right":missing("right")}));
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = raw(&ir, &seed.source);
    let left = address(&raw, &child("left"));
    let right = address(&raw, &child("right"));
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
    let failure = failure(&sandbox);
    assert_eq!(diagnostic(&failure).address_id, left);
    assert_ne!(left, right);
}

#[test]
fn successful_children_then_operand_relation_failure_select_parent() {
    let ir = pure(json!({"kind":"add","left":boolean(),"right":field()}));
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = raw(&ir, &seed.source);
    let parent = address(&raw, &body());
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
    let failure = failure(&sandbox);
    assert_eq!(diagnostic(&failure).address_id, parent);
}

#[test]
fn structurally_equal_children_keep_distinct_occurrence_identity() {
    let ir = pure(
        json!({"kind":"if","condition":boolean(),"then":missing("same"),"otherwise":missing("same")}),
    );
    assert_eq!(
        ir["circuits"][0]["body"]["then"],
        ir["circuits"][0]["body"]["otherwise"]
    );
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = raw(&ir, &seed.source);
    let then = address(&raw, &child("then"));
    let otherwise = address(&raw, &child("otherwise"));
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
    let failure = failure(&sandbox);
    assert_eq!(diagnostic(&failure).address_id, then);
    assert_ne!(then, otherwise);
}

#[test]
fn unavailable_child_origin_blocks_available_parent_annotation() {
    let ir = pure(json!({"kind":"add","left":missing("left"),"right":field()}));
    let seed = Sandbox::new(&ir, None, None, 0);
    let mut raw = raw(&ir, &seed.source);
    let left = address(&raw, &child("left"));
    raw["addresses"][left][3] = Value::Null;
    assert!(!raw["addresses"][address(&raw, &body())][3].is_null());
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
    let failure = failure(&sandbox);
    assert!(matches!(failure.metadata, MetadataStatus::Available(_)));
    assert!(failure.diagnostic.is_none());
}

#[test]
fn compiler_context_is_never_attached_and_located_cause_is_preserved() {
    let mut ir = pure(missing("missing"));
    ir["circuits"][0]["source"] = json!({"file":"declaration.compact","line":2,"column":3});
    let seed = Sandbox::new(&ir, None, None, 0);
    let metadata = fixture::metadata(&ir, &seed.source, true);
    let sandbox = Sandbox::new(&ir, Some(&metadata), None, 0);
    let failure = failure(&sandbox);
    assert!(failure.diagnostic.is_none());
    assert!(matches!(
        failure.cause,
        NativeFailureCause::Render(crate::RenderError::Located { .. })
    ));
}

#[test]
fn same_basename_descriptor_files_remain_distinct_in_selected_diagnostic() {
    let ir = pure(missing("missing"));
    let seed = Sandbox::new(&ir, None, None, 0);
    let left = seed.root.join("left/shared.compact");
    let right = seed.root.join("right/shared.compact");
    for path in [&left, &right] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x").unwrap();
    }
    let mut raw = raw(&ir, &left);
    raw["files"] = json!([{"id":0,"path":left},{"id":1,"path":right}]);
    let mut selected = Vec::new();
    for file_id in [0, 1] {
        raw["spans"][0][0] = json!(file_id);
        let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
        let failure = failure(&sandbox);
        let diagnostic = diagnostic(&failure);
        assert_eq!(diagnostic.file_id, file_id);
        selected.push(diagnostic.descriptor_path.clone());
    }
    assert_ne!(selected[0], selected[1]);
    assert_eq!(
        std::path::Path::new(&selected[0]).file_name(),
        std::path::Path::new(&selected[1]).file_name()
    );
}

#[test]
fn successful_origin_aware_artifact_is_byte_equal_to_originless_output() {
    let ir = pure(json!({"kind":"add","left":field(),"right":field()}));
    let expected = crate::render(&serde_json::from_value(ir.clone()).unwrap()).unwrap();
    let seed = Sandbox::new(&ir, None, None, 0);
    let metadata = fixture::metadata(&ir, &seed.source, false);
    let sandbox = Sandbox::new(&ir, Some(&metadata), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    assert_eq!(rendered.rendered().source, expected);
    assert!(matches!(rendered.metadata(), MetadataStatus::Available(_)));
    rendered.finish_metadata().unwrap();
}
