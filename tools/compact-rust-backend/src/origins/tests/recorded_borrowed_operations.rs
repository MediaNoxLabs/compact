#![cfg(unix)]
//! Private configured frontend fixture authority, not genuine Compact source.
//! Opaque commitment is native-supported and recording-refused, frontend-excluded.
//! This exercises the real issuer/binder/Recorder, not a manufactured source map.
use super::fixture::{self, Sandbox};
use crate::{MetadataStatus, RenderOptions, RenderedContract, ir::Contract};
use serde_json::{Value, json};

fn refusal() -> Value {
    json!({"kind":"persistent_commit",
        "value":{"kind":"default","ty":{"kind":"opaque_string"}},
        "opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}})
}
fn assertion() -> Value {
    json!({"kind":"assert", "condition":{"kind":"equal",
        "left":refusal(), "right":{"kind":"bytes_literal","bytes":vec![0u8;32]}},
        "message":"original action"})
}
fn contract(actions: Vec<Value>) -> Value {
    json!({"schema_version":20,"ledger_fields":[],"circuits":[],
        "stateful_circuits":[{"name":"root","parameters":[],"actions":actions,
        "result":{"kind":"unit"},"return_value":{"kind":"unit"}}]})
}
fn address(raw: &Value, edges: &[Value]) -> usize {
    let mut parent = 0usize;
    for edge in edges {
        parent = raw["addresses"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .find_map(|(id, row)| (row[0] == json!(parent) && row[1] == *edge).then_some(id))
            .expect("fixture exact parent/edge");
    }
    parent
}
fn action_child(raw: &Value, index: usize) -> usize {
    address(
        raw,
        &[
            json!({"field":"stateful_circuits"}),
            json!({"index":0}),
            json!({"field":"actions"}),
            json!({"index":index}),
            json!({"field":"condition"}),
            json!({"field":"left"}),
        ],
    )
}
fn plain(ir: &Value) -> RenderedContract {
    let typed: Contract = serde_json::from_value(ir.clone()).unwrap();
    crate::render_with_capabilities(&typed).unwrap()
}
fn parity(rendered: &crate::FrontendRender, plain: &RenderedContract) {
    assert_eq!(rendered.rendered().source, plain.source);
    assert_eq!(
        serde_json::to_value(&rendered.rendered().capabilities).unwrap(),
        serde_json::to_value(&plain.capabilities).unwrap()
    );
    assert!(matches!(rendered.metadata(), MetadataStatus::Available(_)));
    rendered.finish_metadata().unwrap();
}

#[test]
fn real_recorder_action_assert_binds_original_failed_child_and_distinct_equal_occurrences() {
    let ir = contract(vec![assertion(), assertion()]);
    assert_eq!(
        ir["stateful_circuits"][0]["actions"][0],
        ir["stateful_circuits"][0]["actions"][1]
    );
    let plain = plain(&ir);
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw: Value = serde_json::from_slice(&fixture::metadata(&ir, &seed.source, false)).unwrap();
    let first = action_child(&raw, 0);
    let later = action_child(&raw, 1);
    assert_ne!(first, later);
    assert_eq!(raw["addresses"][first][2], "persistent_commit");
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    parity(&rendered, &plain);
    let diagnostics = rendered.recording_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    let diagnostic = &diagnostics[0];
    assert_eq!(
        diagnostic.gap,
        *plain.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
    );
    let primary = diagnostic.primary.as_ref().unwrap();
    assert_eq!(primary.address_id, first);
    assert_ne!(primary.address_id, later);
    assert_eq!(primary.selected_slot, "expr_node");
    assert_eq!(primary.provenance, "provided");
    assert!(primary.range_matches_post_invocation_snapshot);
    assert!(diagnostic.related_call_sites.is_empty());
}

#[test]
fn real_recorder_absent_assert_child_blocks_available_parent_and_action_origin() {
    let ir = contract(vec![assertion()]);
    let plain = plain(&ir);
    let seed = Sandbox::new(&ir, None, None, 0);
    let mut raw: Value =
        serde_json::from_slice(&fixture::metadata(&ir, &seed.source, false)).unwrap();
    let child = action_child(&raw, 0);
    let action = address(
        &raw,
        &[
            json!({"field":"stateful_circuits"}),
            json!({"index":0}),
            json!({"field":"actions"}),
            json!({"index":0}),
        ],
    );
    assert!(
        raw["addresses"][action][3].is_number(),
        "parent origin remains available"
    );
    raw["addresses"][child][3] = Value::Null;
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    parity(&rendered, &plain);
    let diagnostics = rendered.recording_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].primary.is_none());
    assert!(diagnostics[0].related_call_sites.is_empty());
    assert_eq!(
        diagnostics[0].gap,
        *plain.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
    );
}

#[test]
fn real_recorder_successful_assert_and_four_borrowed_leaf_returns_keep_public_output() {
    let mut ir = json!({"schema_version":20, "circuits":[],
      "ledger_fields":[
        {"id":"cell","index":0,"declaration":{"kind":"cell","ty":{"kind":"field"}}},
        {"id":"counter","index":1,"declaration":{"kind":"counter"}},
        {"id":"set","index":2,"declaration":{"kind":"set","ty":{"kind":"field"}}}],
      "stateful_circuits":[]});
    let rows = [
        ("cell_read", "cell", 0, json!({"kind":"field"})),
        (
            "counter_read",
            "counter",
            1,
            json!({"kind":"unsigned","max":"18446744073709551615"}),
        ),
        (
            "set_size",
            "set",
            2,
            json!({"kind":"unsigned","max":"18446744073709551615"}),
        ),
        ("set_is_empty", "set", 2, json!({"kind":"boolean"})),
    ];
    ir["stateful_circuits"] = Value::Array(
        rows.into_iter()
            .map(|(kind, field, index, ty)| {
                json!({"name":kind,"parameters":[],"actions":[
            {"kind":"assert","condition":{"kind":"boolean","value":true},"message":"supported"}],
            "result":ty,"return_value":{"kind":kind,"field":field,"index":index}})
            })
            .collect(),
    );
    let plain = plain(&ir);
    assert!(plain.capabilities.circuits.iter().all(|row| row.recorded));
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = fixture::metadata(&ir, &seed.source, false);
    let sandbox = Sandbox::new(&ir, Some(&raw), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    parity(&rendered, &plain);
    assert!(rendered.recording_diagnostics().is_empty());
}
