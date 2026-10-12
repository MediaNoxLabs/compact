// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete old Map-read rules, checked composition and native authority.
use super::*;
use crate::ir::Contract;
use admission_probe::trace;
use legacy::guarded_map::closed_guarded_struct_map_read_steps;
use serde_json::{Value, json};
#[path = "guarded_map_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "guarded_map_migration_tests/inclusion.rs"]
mod inclusion;
const ROOT: &str = "assertStoredRecordFresh";
const PURE: &str = "assertRecordFreshEnough";
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../tests/asset-registry-schema20-ir.json")).unwrap()
}
// A complete synthetic contract projected from the retained input. Keeping only
// the root and its pure callee makes domain variations independent of unrelated
// callsites. Remove the unrelated constructor with its helper dependencies, too.
// The genuine complete source is qualified separately, unchanged.
fn isolated() -> Value {
    let mut v = fixture();
    v.as_object_mut().unwrap().remove("constructor");
    v["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["name"] == ROOT);
    v["circuits"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["name"] == PURE);
    v
}
fn named<'a>(v: &'a mut Value, family: &str, name: &str) -> &'a mut Value {
    v[family]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn root(v: &mut Value) -> &mut Value {
    named(v, "stateful_circuits", ROOT)
}
fn pure(v: &mut Value) -> &mut Value {
    named(v, "circuits", PURE)
}
fn sequence(v: &mut Value) -> &mut Value {
    &mut root(v)["actions"][0]["action"]["actions"]
}
fn call(v: &mut Value) -> &mut Value {
    &mut sequence(v)[1]["action"]
}
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn target(c: &Contract) -> &StatefulCircuit {
    c.stateful_circuits.iter().find(|c| c.name == ROOT).unwrap()
}
fn old(c: &Contract) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    let root = target(c);
    // Orchestrator parameter names are generated; malformed source identifiers
    // are diagnosed by native rendering before any legacy attempt is evaluated.
    let params = root
        .parameters
        .iter()
        .enumerate()
        .map(|(n, p)| (p.name.as_str(), (&p.ty, ident(&format!("arg{n}")).unwrap())))
        .collect();
    closed_guarded_struct_map_read_steps(
        root,
        &params,
        &c.ledger_fields.iter().map(|x| (x.id.as_str(), x)).collect(),
        &c.circuits.iter().map(|x| (x.name.as_str(), x)).collect(),
    )
}
fn checked(c: &Contract) -> Result<typed_plan::TypedPlan, effect_plan::PlanError> {
    effect_plan::lower_with_calls(
        target(c),
        &c.ledger_fields.iter().map(|x| (x.id.as_str(), x)).collect(),
        &c.witnesses.iter().map(|x| (x.name.as_str(), x)).collect(),
        &c.circuits.iter().map(|x| (x.name.as_str(), x)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|x| (x.name.as_str(), x))
            .collect(),
    )
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
fn assert_current(c: &Contract) {
    let plan = checked(c).unwrap();
    let steps = &plan.steps;
    let text = quote::quote!(#(#steps)*).to_string();
    assert_eq!(text.matches("record_member").count(), 1);
    assert_eq!(text.matches("record_lookup").count(), 1);
    assert!(text.find("record_member").unwrap() < text.find("record does not exist").unwrap());
    assert!(text.find("record does not exist").unwrap() < text.find("record_lookup").unwrap());
    assert!(text.find("record_lookup").unwrap() < text.find("pure_circuits").unwrap());
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = result.unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|x| x.name == ROOT && x.recorded && x.observed_call)
    );
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["circuit"] == ROOT && x["phase"] == "public_emission" && x["helper"] == false)
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.unit-composition");
    let attempts = row["attempts"].as_array().unwrap();
    assert!(
        !attempts
            .iter()
            .any(|x| x["route"] == "guard.guarded-map-read")
    );
    assert!(
        !attempts
            .iter()
            .any(|x| x["route"] == "guard.opaque-map-write"),
        "Map-write admission is retired independently of this old oracle"
    );
    assert!(
        !attempts
            .iter()
            .any(|x| x["route"] == "guard.guarded-struct-pure")
    );
}
fn assert_old_positive(c: &Contract, label: &str) {
    assert!(old(c).unwrap().is_some(), "complete old {label}");
    assert_current(c);
}
