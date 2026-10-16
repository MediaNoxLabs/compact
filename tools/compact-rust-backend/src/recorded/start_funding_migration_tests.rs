// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete historical funding admission, native syntax and checked plans are separate premises.
use super::*;
use crate::ir::Contract;
use admission_probe::trace;
use serde_json::{Value, json};

#[path = "start_funding_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "start_funding_migration_tests/inclusion.rs"]
mod inclusion;

fn source() -> Value {
    serde_json::from_str(include_str!("../../tests/coracle-start-schema20-ir.json")).unwrap()
}
fn parse(value: Value) -> Contract {
    serde_json::from_value(value).unwrap()
}
fn root(value: &mut Value) -> &mut Value {
    value["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "start")
        .unwrap()
}
fn target<'a>(c: &'a Contract, name: &str) -> &'a StatefulCircuit {
    c.stateful_circuits.iter().find(|c| c.name == name).unwrap()
}
fn old(c: &Contract, name: &str) -> Option<typed_plan::TypedPlan> {
    typed_plan::lower_start_funding(
        target(c, name),
        &c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
        &c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
        &c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|v| (v.name.as_str(), v))
            .collect(),
    )
}
fn checked(c: &Contract, name: &str) -> Result<typed_plan::TypedPlan, effect_plan::PlanError> {
    effect_plan::lower_with_calls(
        target(c, name),
        &c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
        &c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
        &c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|v| (v.name.as_str(), v))
            .collect(),
    )
}
fn current(c: &Contract, name: &str) -> typed_plan::TypedPlan {
    let plan = checked(c, name).unwrap();
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = result.unwrap(); // Complete native validation is mandatory.
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|v| v.name == name)
        .unwrap();
    assert!(cap.recorded && cap.observed_call);
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .unwrap();
    assert_eq!(row["body_owner"], "effectful.checked-migration");
    assert!(row["action_dispatch"].is_null() && row["return_dispatch"].is_null());
    assert!(
        !row["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["route"] == "effectful.start-funding")
    );
    plan
}
fn included(c: &Contract, name: &str) -> typed_plan::TypedPlan {
    assert!(
        old(c, name).is_some(),
        "complete old lower must succeed for {name}"
    );
    current(c, name)
}
fn edit(value: &mut Value, f: &mut impl FnMut(&mut serde_json::Map<String, Value>)) {
    match value {
        Value::Object(m) => {
            f(m);
            m.values_mut().for_each(|v| edit(v, f));
        }
        Value::Array(a) => a.iter_mut().for_each(|v| edit(v, f)),
        _ => {}
    }
}
fn rename(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::String(s) if s == from => *s = to.into(),
        Value::Object(m) => m.values_mut().for_each(|v| rename(v, from, to)),
        Value::Array(a) => a.iter_mut().for_each(|v| rename(v, from, to)),
        _ => {}
    }
}
fn prefix_binding(value: &mut Value, name: &str, ty: Value, expression: Value) {
    let body = root(value)["return_value"]["body"].take();
    root(value)["return_value"]["body"] = json!({"kind":"let","bindings":[{
        "name":name,"ty":ty,"value":expression}],"result":body});
}
fn methods(plan: &typed_plan::TypedPlan) -> Vec<String> {
    struct Visitor(Vec<String>);
    impl syn::visit_mut::VisitMut for Visitor {
        fn visit_expr_method_call_mut(&mut self, node: &mut syn::ExprMethodCall) {
            self.0.push(node.method.to_string());
            syn::visit_mut::visit_expr_method_call_mut(self, node);
        }
    }
    let mut visitor = Visitor(vec![]);
    for mut stmt in plan.steps.clone() {
        syn::visit_mut::VisitMut::visit_stmt_mut(&mut visitor, &mut stmt);
    }
    visitor.0
}
fn assert_witness_order(plan: &typed_plan::TypedPlan) {
    let methods = methods(plan);
    let witnesses: Vec<_> = methods
        .iter()
        .filter(|n| ["local_secret_key", "fresh_nonce", "local_set_board"].contains(&n.as_str()))
        .collect();
    assert_eq!(
        witnesses,
        vec![
            "local_secret_key",
            "fresh_nonce",
            "local_set_board",
            "local_set_board"
        ]
    );
}
