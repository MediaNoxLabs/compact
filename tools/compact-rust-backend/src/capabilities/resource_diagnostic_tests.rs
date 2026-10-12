// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Public capability serialization and priority for a bounded recording refusal.
use super::*;
use crate::{RenderError, render_with_capabilities};
use serde_json::json;

const ROOT: &str = "assertStoredRecordFresh";
const LOOKUP: &str = "actions[0].action.actions[1].bindings[0].value";

pub(crate) fn oversized() -> Value {
    let mut contract: Value =
        serde_json::from_str(include_str!("../../tests/asset-registry-schema20-ir.json")).unwrap();
    fn extend(value: &mut Value) {
        match value {
            Value::Object(object) => {
                if object.get("kind") == Some(&json!("struct"))
                    && object.get("name") == Some(&json!("AssetRecord"))
                {
                    object["fields"].as_array_mut().unwrap().push(json!({"name":"resource_boundary","ty":{"kind":"vector","element":{"kind":"field"},"length":32769}}));
                }
                for child in object.values_mut() {
                    extend(child);
                }
            }
            Value::Array(values) => values.iter_mut().for_each(extend),
            _ => {}
        }
    }
    extend(&mut contract);
    contract
}
fn root(value: &mut Value) -> &mut Value {
    value["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["name"] == ROOT)
        .unwrap()
}
fn render(value: Value) -> crate::RenderedContract {
    render_with_capabilities(&serde_json::from_value::<ir::Contract>(value).unwrap()).unwrap()
}
fn capability(report: &RustCapabilityReport) -> &RustCircuitCapability {
    report.circuits.iter().find(|row| row.name == ROOT).unwrap()
}
fn persistent_commit() -> Value {
    json!({"kind":"persistent_commit","value":{"kind":"field_literal","value":"0"},"opening":{"kind":"bytes_literal","bytes":vec![0_u8;32]}})
}

fn assert_resource(gap: &Value) {
    assert_eq!(
        gap["resource"],
        json!({"kind":"fixed_layout_slots","phase":"fixed_layout_validation","limit":32768})
    );
    assert_eq!(gap["detail"], "fixed aggregate layout exceeds 32768 slots");
}

#[test]
fn fixed_layout_resource_reaches_public_and_observed_capabilities() {
    let input = oversized();
    let expected_source = input["stateful_circuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == ROOT)
        .unwrap()["source"]
        .clone();
    let mut output = render(input);
    assert_eq!(
        output.capabilities.schema_version, 2,
        "lowering remains an unclassified draft"
    );
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|row| row.name == "setCustodian" && row.recorded && row.observed_call),
        "unrelated supported export must retain its APIs"
    );
    let row = capability(&output.capabilities);
    assert_eq!(serde_json::to_value(&row.source).unwrap(), expected_source);
    assert!(!row.recorded && !row.observed_call);
    assert!(row.proof_required.is_none() && row.recording_status.is_none());
    let gap = serde_json::to_value(row.recording_unavailable.as_ref().unwrap()).unwrap();
    assert_eq!(gap["path"], LOOKUP);
    assert_eq!(gap["ir_node"], "RecordedPlan");
    assert_eq!(gap["code"], "unsupported_expression");
    assert_resource(&gap);
    let observed = serde_json::to_value(row.observed_call_unavailable.as_ref().unwrap()).unwrap();
    assert_eq!(observed["code"], "recording_unavailable");
    assert_eq!(observed["path"], LOOKUP);
    assert_eq!(observed["ir_node"], gap["ir_node"]);
    assert_eq!(observed["resource"], gap["resource"]);
    assert_eq!(
        observed["detail"],
        "the observed-call API requires a recorded circuit: fixed aggregate layout exceeds 32768 slots"
    );
    // Exact Rust namespaces matter: a native function is retained; recorded
    // functions and observed methods are absent, rather than just marked false.
    let syntax = syn::parse_file(&output.source).unwrap();
    let ledger = syntax
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "ledger_contract" => {
                module.content.as_ref().map(|(_, items)| items)
            }
            _ => None,
        })
        .unwrap();
    assert!(
        ledger
            .iter()
            .any(|item| matches!(item, syn::Item::Fn(function) if function.sig.ident == ROOT))
    );
    let recorded = ledger
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "recorded" => {
                module.content.as_ref().map(|(_, items)| items)
            }
            _ => None,
        })
        .unwrap();
    assert!(
        !recorded
            .iter()
            .any(|item| matches!(item, syn::Item::Fn(function) if function.sig.ident == ROOT))
    );
    assert!(!output.source.contains("fn assertStoredRecordFresh_call("));
    let info = json!({"circuits":output.capabilities.circuits.iter().map(|row| json!({"name":row.name,"proof":true})).collect::<Vec<_>>()});
    output.capabilities.apply_contract_info(&info).unwrap();
    assert_eq!(output.capabilities.schema_version, 3);
    let row = capability(&output.capabilities);
    assert_eq!(row.recording_status, Some(RecordingStatus::Unavailable));
    assert_eq!(
        serde_json::to_value(row.recording_unavailable.as_ref().unwrap()).unwrap(),
        gap
    );
}

#[test]
fn earlier_root_and_let_failures_do_not_inherit_later_resource_payload() {
    for earlier_binding in [false, true] {
        let mut input = oversized();
        if earlier_binding {
            root(&mut input)["actions"][0]["action"]["actions"][1]["bindings"].as_array_mut().unwrap().insert(0,
                json!({"name":"unsupported_before_lookup","ty":{"kind":"bytes","length":32},"value":persistent_commit()}));
        } else {
            root(&mut input)["actions"]
                .as_array_mut()
                .unwrap()
                .insert(0, json!({"kind":"expression","value":persistent_commit()}));
        }
        let output = render(input);
        let row = capability(&output.capabilities);
        assert!(!row.recorded && !row.observed_call);
        for gap in [
            row.recording_unavailable.as_ref().unwrap(),
            row.observed_call_unavailable.as_ref().unwrap(),
        ] {
            let gap = serde_json::to_value(gap).unwrap();
            assert!(
                gap.get("resource").is_none(),
                "earlier_binding={earlier_binding}: {gap}"
            );
            assert!(!gap["detail"].as_str().unwrap().contains("32768"));
        }
    }
}

#[test]
fn mandatory_native_errors_precede_recording_resource_metadata() {
    let mut input = oversized();
    root(&mut input)["name"] = json!("invalid circuit name");
    let contract: ir::Contract = serde_json::from_value(input).unwrap();
    let error = match render_with_capabilities(&contract) {
        Ok(_) => panic!("invalid native identifier accepted"),
        Err(error) => error,
    };
    assert!(matches!(error, RenderError::Located { .. }));
    assert!(error.to_string().contains("invalid circuit name"));
}

#[test]
fn ordinary_gap_serialization_omits_resource_and_preserves_schema_join() {
    let gap = RecordingGap::name_collision("reserved_call");
    let expected = json!({"code":"name_collision","ir_node":"StatefulCircuit","path":"name","detail":"observed-call method \"reserved_call\" collides with an exported circuit"});
    assert_eq!(serde_json::to_value(&gap).unwrap(), expected);
    let mut report = RustCapabilityReport {
        schema_version: 2,
        circuits: vec![RustCircuitCapability {
            name: "normal".into(),
            source: None,
            recorded: false,
            observed_call: false,
            proof_required: None,
            recording_status: None,
            recording_unavailable: Some(gap.clone()),
            observed_call_unavailable: Some(RecordingGap::recording_dependency(gap)),
        }],
    };
    let before = serde_json::to_value(&report).unwrap();
    assert_eq!(before["schema_version"], 2);
    assert!(before["circuits"][0].get("proof_required").is_none());
    assert!(
        before["circuits"][0]["observed_call_unavailable"]
            .get("resource")
            .is_none()
    );
    report
        .apply_contract_info(&json!({"circuits":[{"name":"normal","proof":true}]}))
        .unwrap();
    let after = serde_json::to_value(&report).unwrap();
    assert_eq!(after["schema_version"], 3);
    assert_eq!(after["circuits"][0]["recording_unavailable"], expected);
    assert!(
        after["circuits"][0]["observed_call_unavailable"]
            .get("resource")
            .is_none()
    );
}
