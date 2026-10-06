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
    // This is the complete historical send fixture, whose expression nesting
    // exceeds the default stack of Rust's test threads during rendering.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let contract: Contract = serde_json::from_value(value).unwrap();
            render_with_capabilities(&contract).unwrap()
        })
        .unwrap()
        .join()
        .unwrap()
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

fn set_slot(value: &mut Value) {
    value["ledger_fields"] = json!([{
        "id": "coins",
        "index": 0,
        "path": [0],
        "declaration": {"kind": "set", "ty": value["stateful_circuits"][0]["parameters"][0]["ty"]}
    }]);
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
        "kind": "expression", "value": {"kind": "boolean", "value": true}
    }]);
    let original = value["stateful_circuits"][1]["return_value"]["value"].take();
    value["stateful_circuits"][1]["return_value"]["value"] = json!({
        "kind": "let", "bindings": [{
            "name": "hidden", "ty": {"kind": "unsigned", "max": "18446744073709551615"},
            "value": {"kind": "set_size", "field": "coins", "index": 0}
        }], "body": original
    });
    set_slot(&mut value);
    let gap = send_gap(value);
    assert_eq!(gap.code, RecordingGapCode::UnsupportedAction);
    assert_eq!(gap.path, "actions[0]");
}

#[test]
fn rejected_unused_set_size_reports_its_exact_binding() {
    let mut value = source();
    let original = value["stateful_circuits"][1]["return_value"]["value"].take();
    value["stateful_circuits"][1]["return_value"]["value"] = json!({
        "kind": "let",
        "bindings": [{
            "name": "hidden", "ty": {"kind": "unsigned", "max": "18446744073709551615"},
            "value": {"kind": "set_size", "field": "coins", "index": 0}
        }],
        "body": original
    });
    set_slot(&mut value);
    let gap = send_gap(value);
    assert_eq!(gap.code, RecordingGapCode::UnsupportedExpression);
    assert_eq!(gap.ir_node, "Expr::SetSize");
    assert_eq!(gap.path, "return_value.value.bindings[0].value");
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
            "name": "hidden", "ty": {"kind": "unsigned", "max": "18446744073709551615"},
            "value": {"kind": "set_size", "field": "coins", "index": 0}
        }], "body": {"kind": "default", "ty": result_ty}}
    });
    set_slot(&mut branch);
    let gap = send_gap(branch);
    assert_eq!(gap.code, RecordingGapCode::UnsupportedExpression);
    assert_eq!(gap.path, "return_value.value.otherwise.bindings[0].value");

    let mut argument = source();
    let first = argument["stateful_circuits"][1]["return_value"]["value"]["arguments"][0].take();
    argument["stateful_circuits"][1]["return_value"]["value"]["arguments"][0] = json!({
        "kind": "let", "bindings": [{
            "name": "hidden", "ty": {"kind": "boolean"},
            "value": {"kind": "set_member", "field": "coins", "index": 0,
                "value": {"kind": "parameter", "name": "input"}}
        }], "body": first
    });
    set_slot(&mut argument);
    let gap = send_gap(argument);
    assert_eq!(gap.code, RecordingGapCode::UnsupportedExpression);
    assert_eq!(gap.ir_node, "Expr::SetMember");
    assert_eq!(
        gap.path,
        "return_value.value.arguments[0].bindings[0].value"
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
