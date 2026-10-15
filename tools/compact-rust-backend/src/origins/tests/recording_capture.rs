#![cfg(unix)]
//! Native-IR controls retained by existing checked recording tests. Fake process
//! issuance/binding is exercised; this is not genuine frontend source acceptance.
use super::fixture::{self, Sandbox};
use crate::{MetadataStatus, RenderOptions, ir::Contract};
use serde_json::{Value, json};

fn returned(value: Value, result: Value) -> Value {
    json!({"schema_version":20,"ledger_fields":[],"circuits":[],"stateful_circuits":[{
        "name":"probe","parameters":[],"actions":[],"result":result,"return_value":{"kind":"expression","value":value}
    }]})
}
fn opaque_commit() -> Value {
    returned(
        json!({"kind":"persistent_commit","value":{"kind":"default","ty":{"kind":"opaque_string"}},
        "opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}),
        json!({"kind":"bytes","length":32}),
    )
}

#[test]
fn native_valid_recording_refusal_captures_actual_expression_and_preserves_report() {
    // Retained typed_plan/composition opaque-input commitment control is
    // frontend-excluded. Native rendering supports it, checked recording refuses.
    let ir = opaque_commit();
    let contract: Contract = serde_json::from_value(ir.clone()).unwrap();
    let plain = crate::render_with_capabilities(&contract).unwrap();
    assert!(!plain.capabilities.circuits[0].recorded);
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = fixture::metadata(&ir, &seed.source, false);
    let sandbox = Sandbox::new(&ir, Some(&raw), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    assert_eq!(rendered.rendered().source, plain.source);
    assert_eq!(
        serde_json::to_value(&rendered.rendered().capabilities).unwrap(),
        serde_json::to_value(&plain.capabilities).unwrap()
    );
    assert!(matches!(rendered.metadata(), MetadataStatus::Available(_)));
    let gap = plain.capabilities.circuits[0]
        .recording_unavailable
        .as_ref()
        .unwrap();
    let diagnostics = rendered.recording_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(&diagnostics[0].gap, gap);
    let primary = diagnostics[0]
        .primary
        .as_ref()
        .expect("actual persistent commitment source captured");
    let raw: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(raw["addresses"][primary.address_id][2], "persistent_commit");
    assert_eq!(primary.selected_slot, "expr_node");
    rendered.finish_metadata().unwrap();
}

#[test]
fn certifier_target_type_is_primary_and_actual_direct_call_site_is_related() {
    let ty = json!({"kind":"tuple","elements":vec![json!({"kind":"field"});13]});
    let mut ir = returned(
        json!({"kind":"call","name":"helper","arguments":[]}),
        ty.clone(),
    );
    ir["circuits"] = json!([{"name":"helper","internal":true,"parameters":[],"result":ty.clone(),"body":{"kind":"default","ty":ty}}]);
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = fixture::metadata(&ir, &seed.source, false);
    let sandbox = Sandbox::new(&ir, Some(&raw), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    assert!(!rendered.rendered().capabilities.circuits[0].recorded);
    let diagnostics = rendered.recording_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0]
            .gap
            .detail
            .contains("pure Default requires tuple arity")
    );
    let primary = diagnostics[0]
        .primary
        .as_ref()
        .expect("original checked target Type");
    assert_eq!(primary.selected_slot, "node");
    assert_eq!(diagnostics[0].related_call_sites.len(), 1);
    let caller = &diagnostics[0].related_call_sites[0];
    assert_ne!(primary.address_id, caller.address_id);
    let raw: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(raw["addresses"][primary.address_id][2], "tuple");
    assert_eq!(raw["addresses"][caller.address_id][2], "call");
}

#[test]
fn unavailable_or_compiler_context_origin_does_not_become_a_parent_recording_range() {
    let ir = opaque_commit();
    for context in [None, Some(true)] {
        let seed = Sandbox::new(&ir, None, None, 0);
        let raw = context.map(|context| fixture::metadata(&ir, &seed.source, context));
        let sandbox = Sandbox::new(&ir, raw.as_deref(), None, 0);
        let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
        assert!(!rendered.rendered().capabilities.circuits[0].recorded);
        assert_eq!(rendered.recording_diagnostics().len(), 1);
        assert!(rendered.recording_diagnostics()[0].primary.is_none());
        assert!(
            rendered.recording_diagnostics()[0]
                .related_call_sites
                .is_empty()
        );
        rendered.finish_metadata().unwrap();
    }
}

#[test]
fn recording_sessions_do_not_leak_a_prior_refusal_into_the_next_export() {
    let mut ir = opaque_commit();
    ir["stateful_circuits"].as_array_mut().unwrap().push(json!({"name":"supported","parameters":[],"actions":[],"result":{"kind":"unit"},"return_value":{"kind":"unit"}}));
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = fixture::metadata(&ir, &seed.source, false);
    let sandbox = Sandbox::new(&ir, Some(&raw), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    assert_eq!(
        rendered
            .recording_diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.circuit == "probe")
            .count(),
        1
    );
    assert!(
        rendered
            .recording_diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.circuit == "supported")
            .all(|diagnostic| diagnostic.primary.is_none())
    );
}

#[test]
fn recording_layout_refusal_captures_original_parameter_type_before_lowered_clones() {
    // Fake configured process control of a genuine-source-compatible Counter
    // interface. The parent separately compiles the supplied Compact source.
    let ir = json!({"schema_version":20,
        "ledger_fields":[{"id":"count","index":0,"path":[0],"declaration":{"kind":"counter"}}],
        "circuits":[],"stateful_circuits":[{"name":"read",
            "parameters":[{"name":"unused","ty":{"kind":"vector","element":{"kind":"field"},"length":32769}}],
            "actions":[],"result":{"kind":"unsigned","max":"18446744073709551615"},
            "return_value":{"kind":"counter_read","field":"count","index":0}}]});
    let contract: Contract = serde_json::from_value(ir.clone()).unwrap();
    let plain = crate::render_with_capabilities(&contract).unwrap();
    let gap = plain.capabilities.circuits[0]
        .recording_unavailable
        .as_ref()
        .expect("existing recording layout refusal");
    assert_eq!(gap.path, "parameters[0]");
    assert!(
        gap.detail
            .contains("fixed aggregate layout exceeds 32768 slots")
    );
    let seed = Sandbox::new(&ir, None, None, 0);
    let raw = fixture::metadata(&ir, &seed.source, false);
    let sandbox = Sandbox::new(&ir, Some(&raw), None, 0);
    let rendered = sandbox.invoke().render(RenderOptions::default()).unwrap();
    assert_eq!(rendered.rendered().source, plain.source);
    assert_eq!(
        serde_json::to_value(&rendered.rendered().capabilities).unwrap(),
        serde_json::to_value(&plain.capabilities).unwrap()
    );
    assert_eq!(rendered.recording_diagnostics().len(), 1);
    let diagnostic = &rendered.recording_diagnostics()[0];
    assert_eq!(&diagnostic.gap, gap);
    assert_eq!(diagnostic.gap.ir_node, gap.ir_node);
    let resource = diagnostic
        .gap
        .resource
        .as_ref()
        .expect("typed layout policy retained");
    assert_eq!(
        resource.kind,
        crate::recorded::RecordingResourceKind::FixedLayoutSlots
    );
    assert_eq!(
        resource.phase,
        crate::recorded::RecordingResourcePhase::FixedLayoutValidation
    );
    assert_eq!(resource.limit, 32768);
    let primary = diagnostic
        .primary
        .as_ref()
        .expect("original root parameter Type captured");
    assert_eq!(primary.selected_slot, "node");
    let raw: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(raw["addresses"][primary.address_id][2], "vector");
    assert!(diagnostic.related_call_sites.is_empty());
}
