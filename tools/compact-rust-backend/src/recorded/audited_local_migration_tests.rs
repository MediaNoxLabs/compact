// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Full old renderer admission, native authority, and checked ownership are separate laws.
use super::*;
use crate::ir::Contract;
use admission_probe::trace;
use serde_json::{Value, json};

#[path = "audited_local_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "audited_local_migration_tests/inclusion.rs"]
mod inclusion;

const VERIFY: &str = "verifyAttestation";
const ACCEPT: &str = "acceptAttestation";
const LOCAL: &str = "schnorrVerify";
fn fixture() -> Contract {
    serde_json::from_str(include_str!(
        "../../tests/schnorr-attestation-schema20-ir.json"
    ))
    .unwrap()
}
fn target<'a>(c: &'a Contract, name: &str) -> &'a StatefulCircuit {
    c.stateful_circuits.iter().find(|c| c.name == name).unwrap()
}
fn target_mut<'a>(c: &'a mut Contract, name: &str) -> &'a mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|c| c.name == name)
        .unwrap()
}
fn old(c: &Contract, name: &str) -> Result<Option<RecordingOutcome<syn::Item>>, RenderError> {
    audited_local::render(
        target(c, name),
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect(),
    )
}
fn checked(c: &Contract, name: &str) -> Result<typed_plan::TypedPlan, effect_plan::PlanError> {
    effect_plan::lower_with_calls(
        target(c, name),
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect(),
    )
}
fn current(c: &Contract, name: &str, expected_owner: &str) -> typed_plan::TypedPlan {
    // Full rendering validates every native body/declaration before recording.
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
    assert_eq!(row["body_owner"], expected_owner);
    assert!(
        !row["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["route"] == "early.audited-local")
    );
    checked(c, name).unwrap()
}
fn included(c: &Contract, name: &str) -> typed_plan::TypedPlan {
    included_as(c, name, "algebra.unit-composition")
}
fn included_as(c: &Contract, name: &str, expected_owner: &str) -> typed_plan::TypedPlan {
    assert!(
        matches!(old(c, name).unwrap(), Some(RecordingOutcome::Supported(_))),
        "complete old renderer: {name}"
    );
    current(c, name, expected_owner)
}
fn calls(plan: &typed_plan::TypedPlan) -> Vec<String> {
    struct Names(Vec<String>);
    impl syn::visit_mut::VisitMut for Names {
        fn visit_expr_method_call_mut(&mut self, node: &mut syn::ExprMethodCall) {
            self.0.push(node.method.to_string());
            syn::visit_mut::visit_expr_method_call_mut(self, node);
        }
    }
    let mut names = Names(Vec::new());
    for mut statement in plan.steps.clone() {
        syn::visit_mut::VisitMut::visit_stmt_mut(&mut names, &mut statement);
    }
    names.0
}
fn change(c: Contract, f: impl FnOnce(&mut Value)) -> Contract {
    let mut v = serde_json::to_value(c).unwrap();
    f(&mut v);
    serde_json::from_value(v).unwrap()
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
fn rename(v: &mut Value, pairs: &[(&str, &str)]) {
    match v {
        Value::String(s) => {
            if let Some((_, to)) = pairs.iter().find(|(from, _)| *from == s.as_str()) {
                *s = (*to).into();
            }
        }
        Value::Object(m) => m.values_mut().for_each(|v| rename(v, pairs)),
        Value::Array(a) => a.iter_mut().for_each(|v| rename(v, pairs)),
        _ => {}
    }
}
fn statement(value: Expr, ty: Type) -> StateAction {
    StateAction::Let {
        bindings: vec![crate::ir::LocalBinding {
            name: "local_value".into(),
            ty,
            value,
        }],
        action: Box::new(StateAction::Assert {
            condition: Expr::Boolean { value: true },
            message: "value evaluated".into(),
        }),
    }
}
fn local(c: &mut Contract, actions: Vec<StateAction>) {
    target_mut(c, LOCAL).actions = actions;
}
fn param(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
