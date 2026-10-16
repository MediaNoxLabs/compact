#![cfg(unix)]
//! Real issuer/binder/Recorder lifecycle with explicit synthetic test frontend.
//! Opaque commitment IR is native-supported/recording-refused but frontend-excluded.
//! The physical one-character source and provided spans are fixture authority,
//! not genuine Compact parser ranges or consumed-source attestation.
use super::fixture::{self, Sandbox};
use crate::{MetadataStatus, RenderOptions, RenderedContract, ir::Contract};
use serde_json::{Value, json};

fn bytes() -> Value {
    json!({"kind":"bytes","length":32})
}
fn literal() -> Value {
    json!({"kind":"bytes_literal","bytes":vec![0u8;32]})
}
fn call(name: &str) -> Value {
    json!({"kind":"call","name":name,"arguments":[]})
}
fn refusal() -> Value {
    json!({"kind":"persistent_commit",
        "value":{"kind":"default","ty":{"kind":"opaque_string"}},
        "opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}})
}
fn circuit(name: &str, internal: bool, value: Value) -> Value {
    json!({"name":name,"internal":internal,"parameters":[],"actions":[],
        "result":bytes(),"return_value":{"kind":"expression","value":value}})
}
fn contract(circuits: Vec<Value>) -> Value {
    json!({"schema_version":20,"ledger_fields":[],"circuits":[],"stateful_circuits":circuits})
}
fn chain() -> Value {
    contract(vec![
        circuit("root", false, call("mid")),
        circuit("mid", true, call("bad")),
        circuit("bad", true, refusal()),
    ])
}
fn value_path(index: usize) -> Vec<Value> {
    vec![
        json!({"field":"stateful_circuits"}),
        json!({"index":index}),
        json!({"field":"return_value"}),
        json!({"field":"value"}),
    ]
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
            .expect("actual fixture parent/edge");
    }
    parent
}
fn plain(ir: &Value) -> RenderedContract {
    let typed: Contract = serde_json::from_value(ir.clone()).unwrap();
    crate::render_with_capabilities(&typed).unwrap()
}
fn assert_parity(rendered: &crate::FrontendRender, plain: &RenderedContract) {
    assert_eq!(rendered.rendered().source, plain.source);
    assert_eq!(
        serde_json::to_value(&rendered.rendered().capabilities).unwrap(),
        serde_json::to_value(&plain.capabilities).unwrap()
    );
    assert!(matches!(rendered.metadata(), MetadataStatus::Available(_)));
    rendered.finish_metadata().unwrap();
}
fn assert_gap(diagnostic: &crate::RecordingDiagnostic, plain: &RenderedContract, name: &str) {
    let capability = plain
        .capabilities
        .circuits
        .iter()
        .find(|capability| capability.name == name)
        .unwrap();
    assert!(!capability.recorded);
    assert_eq!(diagnostic.circuit, name);
    assert_eq!(
        &diagnostic.gap,
        capability.recording_unavailable.as_ref().unwrap()
    );
}
fn assert_physical(source: &crate::SourceDiagnostic, path: &std::path::Path) {
    assert_eq!(source.descriptor_path, path.to_str().unwrap());
    assert_eq!(source.provenance, "provided");
    assert!(source.end_exclusive);
    assert!(source.range_matches_post_invocation_snapshot);
    assert!(source.post_invocation_snapshot.is_some());
}

#[test]
fn real_recorder_nested_stateful_calls_keep_first_primary_and_unwind_addresses() {
    let ir = chain();
    let plain = plain(&ir);
    let seed = Sandbox::new(&ir, None, None, 0);
    let base: Value = serde_json::from_slice(&fixture::metadata(&ir, &seed.source, false)).unwrap();
    let bad = address(&base, &value_path(2));
    let mid = address(&base, &value_path(1));
    let root = address(&base, &value_path(0));
    assert_eq!(base["addresses"][bad][2], "persistent_commit");
    assert_eq!(base["addresses"][mid][2], "call");
    assert_ne!(root, mid);
    for primary_available in [true, false] {
        let mut raw = base.clone();
        if !primary_available {
            raw["addresses"][bad][3] = Value::Null;
        }
        let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
        let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
        assert_parity(&rendered, &plain);
        let diagnostics = rendered.recording_diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "internal failures are not exported sessions"
        );
        let diagnostic = &diagnostics[0];
        assert_gap(diagnostic, &plain, "root");
        assert_eq!(
            diagnostic
                .primary
                .as_ref()
                .map(|primary| primary.address_id),
            primary_available.then_some(bad)
        );
        if let Some(primary) = &diagnostic.primary {
            assert_eq!(primary.selected_slot, "expr_node");
            assert_physical(primary, &seed.source);
        }
        let related: Vec<_> = diagnostic
            .related_call_sites
            .iter()
            .map(|source| source.address_id)
            .collect();
        assert_eq!(
            related,
            [mid, root],
            "actual entered body unwinds inner then outer"
        );
        for related in &diagnostic.related_call_sites {
            assert_physical(related, &seed.source);
            assert_ne!(related.address_id, bad);
        }
        // Missing callee origin stays unavailable even though both actual
        // incoming calls have physical ranges; none becomes a guessed primary.
    }
}

#[test]
fn real_recorder_equal_call_branches_keep_selected_occurrence_not_value_equality() {
    let ir = contract(vec![
        circuit(
            "root",
            false,
            json!({"kind":"if",
        "condition":{"kind":"boolean","value":true},
        "then":call("bad"),"otherwise":call("bad")}),
        ),
        circuit("bad", true, refusal()),
    ]);
    let plain = plain(&ir);
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw: Value = serde_json::from_slice(&fixture::metadata(&ir, &seed.source, false)).unwrap();
    let mut then_path = value_path(0);
    then_path.push(json!({"field":"then"}));
    let mut else_path = value_path(0);
    else_path.push(json!({"field":"otherwise"}));
    let then = address(&raw, &then_path);
    let otherwise = address(&raw, &else_path);
    assert_ne!(then, otherwise);
    assert_eq!(
        ir["stateful_circuits"][0]["return_value"]["value"]["then"],
        ir["stateful_circuits"][0]["return_value"]["value"]["otherwise"]
    );
    let bad = address(&raw, &value_path(1));
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    assert_parity(&rendered, &plain);
    let diagnostics = rendered.recording_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_gap(&diagnostics[0], &plain, "root");
    assert_eq!(diagnostics[0].primary.as_ref().unwrap().address_id, bad);
    let related: Vec<_> = diagnostics[0]
        .related_call_sites
        .iter()
        .map(|source| source.address_id)
        .collect();
    assert_eq!(related, [then]);
    assert!(!related.contains(&otherwise));
}

#[test]
fn actual_export_sessions_clear_failed_edges_across_success_before_next_failure() {
    let ir = contract(vec![
        circuit("failed_first", false, call("bad")),
        circuit("supported", false, literal()),
        circuit("failed_later", false, call("bad")),
        circuit("bad", true, refusal()),
    ]);
    let plain = plain(&ir);
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw: Value = serde_json::from_slice(&fixture::metadata(&ir, &seed.source, false)).unwrap();
    let first = address(&raw, &value_path(0));
    let successful = address(&raw, &value_path(1));
    let later = address(&raw, &value_path(2));
    let bad = address(&raw, &value_path(3));
    assert_ne!(first, later);
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&raw).unwrap()), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    assert_parity(&rendered, &plain);
    let diagnostics = rendered.recording_diagnostics();
    assert_eq!(diagnostics.len(), 2);
    assert!(
        plain
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "supported")
            .unwrap()
            .recorded
    );
    for (diagnostic, name, edge) in [
        (&diagnostics[0], "failed_first", first),
        (&diagnostics[1], "failed_later", later),
    ] {
        assert_gap(diagnostic, &plain, name);
        assert_eq!(diagnostic.primary.as_ref().unwrap().address_id, bad);
        let related: Vec<_> = diagnostic
            .related_call_sites
            .iter()
            .map(|source| source.address_id)
            .collect();
        assert_eq!(related, [edge]);
        assert!(!related.contains(&successful));
    }
    // The real facade calls Recorder begin/finish for each root and discards
    // internal sessions. Tests do not invoke or imitate a reset themselves.
}
