// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete old read verifier, native validation, and checked admission are distinct.
use super::*;
use crate::ir::Contract;
use admission_probe::trace;
use serde_json::{Value, json};

#[path = "read_only_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "read_only_migration_tests/inclusion.rs"]
mod inclusion;

fn reduced() -> Value {
    serde_json::from_str(include_str!(
        "../../tests/did-read-only-reducer-schema20-ir.json"
    ))
    .unwrap()
}
fn parse(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn target<'a>(c: &'a Contract, name: &str) -> &'a StatefulCircuit {
    c.stateful_circuits.iter().find(|c| c.name == name).unwrap()
}
fn circuit<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn root(v: &mut Value) -> &mut Value {
    circuit(v, "verify")
}
fn key_let(v: &mut Value) -> &mut Value {
    &mut root(v)["actions"][0]["actions"][1]
}
fn lookup_let(v: &mut Value) -> &mut Value {
    &mut key_let(v)["action"]["actions"][1]
}
fn call(v: &mut Value) -> &mut Value {
    &mut lookup_let(v)["action"]
}
fn old(c: &Contract, name: &str) -> Result<Option<RecordingOutcome<syn::Item>>, RenderError> {
    read_only_verification::render(
        target(c, name),
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect(),
    )
}
fn checked(c: &Contract, name: &str) -> Result<typed_plan::TypedPlan, effect_plan::PlanError> {
    effect_plan::lower_with_calls(
        target(c, name),
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|f| (f.name.as_str(), f)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect(),
    )
}
fn current(c: &Contract, name: &str) -> typed_plan::TypedPlan {
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = result.unwrap();
    let capability = output
        .capabilities
        .circuits
        .iter()
        .find(|r| r.name == name)
        .unwrap();
    assert!(capability.recorded && capability.observed_call);
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.ordinary");
    assert!(
        !row["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["route"] == "early.read-only-verification"
                || a["route"] == "early.audited-local")
    );
    checked(c, name).unwrap()
}
fn included(c: &Contract, name: &str) -> typed_plan::TypedPlan {
    assert!(
        matches!(old(c, name).unwrap(), Some(RecordingOutcome::Supported(_))),
        "whole old renderer {name}"
    );
    current(c, name)
}
fn methods(plan: &typed_plan::TypedPlan) -> Vec<String> {
    struct Names(Vec<String>);
    impl syn::visit_mut::VisitMut for Names {
        fn visit_expr_method_call_mut(&mut self, n: &mut syn::ExprMethodCall) {
            self.0.push(n.method.to_string());
            syn::visit_mut::visit_expr_method_call_mut(self, n);
        }
    }
    let mut names = Names(Vec::new());
    for mut step in plan.steps.clone() {
        syn::visit_mut::VisitMut::visit_stmt_mut(&mut names, &mut step);
    }
    names.0
}
fn edit(v: &mut Value, f: &mut impl FnMut(&mut serde_json::Map<String, Value>)) {
    match v {
        Value::Object(m) => {
            f(m);
            m.values_mut().for_each(|v| edit(v, f));
        }
        Value::Array(a) => a.iter_mut().for_each(|v| edit(v, f)),
        _ => {}
    }
}
fn rename(v: &mut Value, from: &str, to: &str) {
    match v {
        Value::String(s) if s == from => *s = to.into(),
        Value::Object(m) => m.values_mut().for_each(|v| rename(v, from, to)),
        Value::Array(a) => a.iter_mut().for_each(|v| rename(v, from, to)),
        _ => {}
    }
}
fn local_binding(value: Value, ty: Value) -> Value {
    json!({"kind":"let","bindings":[{"name":"local_value","ty":ty,"value":value}],"action":{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"local evaluated"}})
}
