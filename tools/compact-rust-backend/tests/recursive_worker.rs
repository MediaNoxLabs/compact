// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Finite regression controls on the Rust test harness's ordinary worker stack.
//! These bound the measured repair; they do not promise arbitrary nesting.
use compact_rust_backend::{ir::Contract, render_with_capabilities};
use serde_json::{Value, json};

fn nested(family: &str, depth: usize) -> Contract {
    let mut value = json!({"kind": "parameter", "name": "x"});
    for level in 0..depth {
        let operation = if family == "mixed" {
            ["if", "add", "hash"][level % 3]
        } else {
            family
        };
        value = match operation {
            "if" => json!({"kind":"if", "condition":{"kind":"parameter","name":"flag"},
                "then":value,"otherwise":{"kind":"field_literal","value":"1"}}),
            "add" => json!({"kind":"add","left":value,
                "right":{"kind":"field_literal","value":"1"}}),
            "hash" => json!({"kind":"transient_hash","value":value}),
            _ => unreachable!(),
        };
    }
    let source: Value = json!({"schema_version":20,"ledger_fields":[],"witnesses":[],
        "stateful_circuits":[],"circuits":[{"name":"nested","parameters":[
            {"name":"x","ty":{"kind":"field"}},
            {"name":"flag","ty":{"kind":"boolean"}}],
            "result":{"kind":"field"},"body":value}]});
    serde_json::from_value(source).expect("valid constructed IR")
}

fn check(family: &str) {
    for depth in [8, 16, 24] {
        let contract = nested(family, depth);
        let rendered = render_with_capabilities(&contract).expect("supported nested expression");
        assert!(rendered.source.contains("pub fn nested"));
        assert!(rendered.capabilities.circuits.is_empty());
    }
}

#[test]
fn nested_dynamic_conditions_on_default_worker() {
    check("if");
}
#[test]
fn nested_arithmetic_on_default_worker() {
    check("add");
}
#[test]
fn nested_hashes_on_default_worker() {
    check("hash");
}
#[test]
fn mixed_control_and_operations_on_default_worker() {
    check("mixed");
}

#[test]
fn recorded_nested_conditions_on_default_worker() {
    // Reduced from the existing witnessed Boolean Cell assertion profile. Twenty
    // dynamic branches overflowed the old recorded Plan::expression frame even
    // with the pure-only frame repair, so native-only admission is insufficient.
    let mut source: Value =
        serde_json::from_str(include_str!("stateful-assert-schema20-ir.json")).unwrap();
    source["stateful_circuits"] = json!([source["stateful_circuits"][1].clone()]);
    source["ledger_fields"] = json!([source["ledger_fields"][0].clone()]);
    source["constructor"] = Value::Null;
    let mut condition = json!({"kind":"cell_read","field":"open","index":0});
    for _ in 0..20 {
        condition = json!({"kind":"if",
            "condition":{"kind":"parameter","name":"selected"},
            "then":condition,"otherwise":{"kind":"boolean","value":false}});
    }
    source["stateful_circuits"][0]["return_value"]["value"]["body"]["steps"][0]["condition"] =
        condition;
    let contract: Contract = serde_json::from_value(source).unwrap();
    let rendered = render_with_capabilities(&contract).expect("recorded nested assertion");
    assert_eq!(rendered.capabilities.circuits.len(), 1);
    let circuit = &rendered.capabilities.circuits[0];
    assert_eq!(circuit.name, "unit_result");
    assert!(circuit.recorded);
    assert!(circuit.observed_call);
}

fn local_call_graph(depth: usize, repeated: bool, mixed: bool) -> Contract {
    let circuits = (0..depth).map(|i| {
        let actions = if i + 1 == depth {
            vec![json!({"kind":"assert", "condition":{"kind":"parameter","name":"selected"}, "message":"selected"})]
        } else {
            let call = json!({"kind":"circuit_call", "name":format!("chain{}",i+1), "arguments":[{"kind":"parameter","name":"selected"}]});
            if mixed && i % 2 == 0 {
                vec![json!({"kind":"if", "condition":{"kind":"parameter","name":"selected"}, "then":call, "otherwise":{"kind":"sequence","actions":[]}})]
            } else if repeated { vec![call.clone(),call] } else { vec![call] }
        };
        json!({"name":format!("chain{i}"), "parameters":[{"name":"selected","ty":{"kind":"boolean"}}], "internal":i>0, "result":{"kind":"unit"}, "return_value":{"kind":"unit"}, "actions":actions})
    }).collect::<Vec<_>>();
    serde_json::from_value(json!({"schema_version":20,"ledger_fields":[],"witnesses":[],"circuits":[],"stateful_circuits":circuits})).expect("valid finite call graph")
}

fn check_local_call_graph(depth: usize, repeated: bool, mixed: bool) {
    let contract = local_call_graph(depth, repeated, mixed);
    let rendered =
        render_with_capabilities(&contract).expect("supported finite recorded call graph");
    let entry = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "chain0")
        .expect("entry capability");
    assert!(
        entry.recorded,
        "the regression must exercise recording, not only native emission"
    );
    assert!(entry.observed_call);
    assert!(rendered.source.contains("pub fn chain0"));
}

#[test]
fn recorded_local_call_chain_on_default_worker() {
    check_local_call_graph(12, false, false);
}

#[test]
fn recorded_repeated_call_diamond_on_default_worker() {
    check_local_call_graph(12, true, false);
}

#[test]
fn recorded_calls_and_branches_on_default_worker() {
    check_local_call_graph(12, false, true);
}

#[test]
fn malformed_local_call_diagnostics_precede_recorded_expansion() {
    use compact_rust_backend::RenderError;
    for cyclic in [false, true] {
        let mut contract = local_call_graph(12, false, false);
        let mut bad = contract.stateful_circuits.last().unwrap().clone();
        bad.name = "bad".into();
        bad.actions = vec![compact_rust_backend::ir::StateAction::CircuitCall {
            name: if cyclic { "bad" } else { "missing" }.into(),
            arguments: vec![compact_rust_backend::ir::Expr::Boolean { value: true }],
        }];
        contract.stateful_circuits.push(bad);
        let error = match render_with_capabilities(&contract) {
            Err(error) => error,
            Ok(_) => panic!("malformed graph unexpectedly admitted"),
        };
        match error {
            RenderError::UnknownCircuit(name) if !cyclic => assert_eq!(name, "missing"),
            RenderError::UnsupportedStatefulCall(name) if cyclic => assert_eq!(name, "bad"),
            other => panic!("unexpected diagnostic: {other:?}"),
        }
    }
}

#[test]
fn recorded_local_bindings_across_calls_on_default_worker() {
    let contract = local_call_graph(12, false, false);
    let mut value = serde_json::to_value(contract).unwrap();
    for circuit in value["stateful_circuits"].as_array_mut().unwrap() {
        let mut actions = circuit["actions"].take();
        for action in actions.as_array_mut().unwrap() {
            if action["kind"] == "circuit_call" {
                action["arguments"][0]["name"] = json!("chosen");
            } else {
                action["condition"]["name"] = json!("chosen");
            }
        }
        circuit["actions"] = json!([{"kind":"let", "bindings":[{
            "name":"chosen","ty":{"kind":"boolean"},
            "value":{"kind":"parameter","name":"selected"}}],
            "action":{"kind":"sequence","actions":actions}}]);
    }
    let contract = serde_json::from_value(value).unwrap();
    let rendered =
        render_with_capabilities(&contract).expect("scoped bindings across finite calls");
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .any(|c| c.name == "chain0" && c.recorded && c.observed_call)
    );
}
