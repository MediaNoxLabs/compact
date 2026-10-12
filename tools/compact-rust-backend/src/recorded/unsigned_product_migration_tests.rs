// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete old product admission versus native validation and checked composition.
use super::*;
use crate::ir::Contract;
use admission_probe::trace;
use legacy::guarded_pure::closed_guarded_unsigned_product_steps;
use serde_json::{Value, json};

#[path = "unsigned_product_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "unsigned_product_migration_tests/inclusion.rs"]
mod inclusion;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../tests/mixed-width-operand-schema20-ir.json"
    ))
    .unwrap()
}
fn named<'a>(v: &'a mut Value, family: &str, name: &str) -> &'a mut Value {
    v[family]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|v| v["name"] == name)
        .unwrap()
}
fn root(v: &mut Value) -> &mut Value {
    named(v, "stateful_circuits", "recordPinned")
}
fn pure(v: &mut Value) -> &mut Value {
    named(v, "circuits", "assertProductLE")
}
fn condition(v: &mut Value) -> &mut Value {
    &mut pure(v)["body"]["steps"][0]["condition"]
}
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn target(c: &Contract) -> &StatefulCircuit {
    c.stateful_circuits
        .iter()
        .find(|c| c.name == "recordPinned")
        .unwrap()
}
fn old(c: &Contract) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    let root = target(c);
    closed_guarded_unsigned_product_steps(
        root,
        &root
            .parameters
            .iter()
            .map(|p| (p.name.as_str(), (&p.ty, ident(&p.name).unwrap())))
            .collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
    )
}
fn checked(c: &Contract) -> Result<typed_plan::TypedPlan, effect_plan::PlanError> {
    effect_plan::lower_with_calls(
        target(c),
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect(),
    )
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
fn assert_current(c: &Contract) {
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = result.unwrap();
    for name in ["recordPinned", "recordMatching"] {
        assert!(
            output
                .capabilities
                .circuits
                .iter()
                .any(|c| c.name == name && c.recorded && c.observed_call)
        );
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false
            })
            .unwrap();
        assert_eq!(row["body_owner"], "algebra.unit-composition", "{name}");
        let attempts = row["attempts"].as_array().unwrap();
        assert!(
            !attempts
                .iter()
                .any(|a| a["route"] == "guard.guarded-unsigned-product")
        );
        assert!(
            attempts
                .iter()
                .any(|a| a["route"] == "guard.guarded-struct-pure"),
            "sibling is still evaluated"
        );
    }
}
fn assert_old_positive(c: &Contract, label: &str) {
    assert!(old(c).unwrap().is_some(), "whole old {label}");
    let plan = checked(c).unwrap();
    let steps = &plan.steps;
    let text = quote::quote!(#(#steps)*).to_string();
    assert!(
        text.find("pure_circuits").unwrap() < text.find("record_increment").unwrap(),
        "pure guard precedes the public Counter query"
    );
    assert_current(c);
}
