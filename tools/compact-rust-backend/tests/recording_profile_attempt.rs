// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_backend::ir::Contract;
use compact_rust_backend::{RecordingGapCode, RenderError, render_with_capabilities};
use serde_json::{Value, json};

fn source() -> Value {
    serde_json::from_str(include_str!("shielded-send-schema20-ir.json")).unwrap()
}

fn rendered(value: Value) -> compact_rust_backend::RenderedContract {
    // Keep this historical nested source on the ordinary test worker: it is a
    // regression control for recursive renderer stack use.
    let contract: Contract = serde_json::from_value(value).unwrap();
    render_with_capabilities(&contract).unwrap()
}

fn send_gap(value: Value) -> compact_rust_backend::RecordingGap {
    let report = rendered(value).capabilities;
    assert!(
        report
            .circuits
            .iter()
            .filter(|c| c.name != "send_to_self")
            .all(|c| c.recorded)
    );
    let send = report
        .circuits
        .iter()
        .find(|c| c.name == "send_to_self")
        .unwrap();
    assert!(!send.recorded && !send.observed_call);
    send.recording_unavailable.clone().unwrap()
}

#[test]
fn admitted_send_profiles_keep_native_and_recorded_surface() {
    let source = rendered(source());
    assert_eq!(source.capabilities.circuits.len(), 4);
    assert!(source.capabilities.circuits.iter().all(|c| c.recorded));
    assert!(source.source.contains("create_zswap_input"));
    assert!(source.source.contains("create_zswap_output"));
}

#[test]
fn action_gap_keeps_precedence_over_return_profile_diagnostic() {
    let mut value = source();
    value["stateful_circuits"][1]["actions"] = json!([{
        "kind": "expression", "value": {"kind":"transient_hash", "value":{"kind":"default", "ty":{"kind":"opaque_string"}}}
    }]);
    let original = value["stateful_circuits"][1]["return_value"]["value"].take();
    value["stateful_circuits"][1]["return_value"]["value"] = json!({
        "kind": "let", "bindings": [{
            "name": "hidden", "ty": {"kind": "field"},
            "value": {"kind": "transient_hash", "value": {"kind":"default", "ty":{"kind":"opaque_string"}}}
        }], "body": original
    });
    let gap = send_gap(value);
    assert_eq!(gap.code, RecordingGapCode::UnsupportedAction);
    assert_eq!(gap.path, "actions[0]");
}

#[test]
fn rejected_unused_dynamic_hash_reports_its_exact_binding() {
    let mut value = source();
    let original = value["stateful_circuits"][1]["return_value"]["value"].take();
    value["stateful_circuits"][1]["return_value"]["value"] = json!({
        "kind": "let",
        "bindings": [{
            "name": "hidden", "ty": {"kind": "field"},
            "value": {"kind": "transient_hash", "value": {"kind":"default", "ty":{"kind":"opaque_string"}}}
        }],
        "body": original
    });
    let gap = send_gap(value);
    assert_eq!(gap.code, RecordingGapCode::UnsupportedExpression);
    assert_eq!(gap.ir_node, "Expr::Default");
    assert_eq!(gap.path, "return_value.value.bindings[0].value.value");
}

#[test]
fn rejected_unselected_branch_and_nested_argument_keep_source_paths() {
    let mut branch = source();
    let original = branch["stateful_circuits"][1]["return_value"]["value"].take();
    let result_ty = branch["stateful_circuits"][1]["result"].clone();
    branch["stateful_circuits"][1]["return_value"]["value"] = json!({
        "kind": "if", "condition": {"kind": "boolean", "value": true},
        "then": original,
        "otherwise": {"kind": "let", "bindings": [{
            "name": "hidden", "ty": {"kind": "field"},
            "value": {"kind": "transient_hash", "value": {"kind":"default", "ty":{"kind":"opaque_string"}}}
        }], "body": {"kind": "default", "ty": result_ty}}
    });
    let gap = send_gap(branch);
    assert_eq!(gap.code, RecordingGapCode::UnsupportedExpression);
    assert_eq!(
        gap.path,
        "return_value.value.otherwise.bindings[0].value.value"
    );

    let mut argument = source();
    let first = argument["stateful_circuits"][1]["return_value"]["value"]["arguments"][0].take();
    argument["stateful_circuits"][1]["return_value"]["value"]["arguments"][0] = json!({
        "kind": "let", "bindings": [{
            "name": "hidden", "ty": {"kind": "field"},
            "value": {"kind":"transient_hash", "value":{"kind":"default", "ty":{"kind":"opaque_string"}}}
        }], "body": first
    });
    let gap = send_gap(argument);
    assert_eq!(gap.code, RecordingGapCode::UnsupportedExpression);
    assert_eq!(gap.ir_node, "Expr::Default");
    assert_eq!(
        gap.path,
        "return_value.value.arguments[0].bindings[0].value.value"
    );
}

#[test]
fn native_error_precedes_recording_profile_diagnostic() {
    let mut value = source();
    value["stateful_circuits"][1]["name"] = "bad-name".into();
    let contract: Contract = serde_json::from_value(value).unwrap();
    assert!(matches!(
        render_with_capabilities(&contract).err(),
        Some(RenderError::Located { error, .. })
            if *error == RenderError::InvalidIdentifier("bad-name".into())
    ));
}
