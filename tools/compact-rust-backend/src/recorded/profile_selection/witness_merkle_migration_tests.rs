// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole-old witness-Merkle qualification, independent of its physical recipe.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION};
use crate::recorded::admission_probe::trace;
use crate::recorded::guarded_mutations::closed_witness_admitted_merkle_insert_steps;
use serde_json::{Value, json};

#[path = "witness_merkle_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "witness_merkle_migration_tests/inclusion.rs"]
mod inclusion;

const PATH_WITNESS: &str = "context$eligible_voters$path_of";
fn fixture() -> Value {
    let mut v: Value =
        serde_json::from_str(include_str!("../../../tests/election-schema13-ir.json")).unwrap();
    v["schema_version"] = json!(SCHEMA_VERSION);
    v
}
fn named_mut<'a>(v: &'a mut Value, family: &str, name: &str) -> &'a mut Value {
    v[family]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn root_actions(v: &mut Value) -> &mut Value {
    &mut named_mut(v, "stateful_circuits", "add_voter")["actions"][0]["actions"]
}
fn continuation(v: &mut Value) -> &mut Value {
    &mut root_actions(v)[1]["action"]["action"]["actions"]
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
fn renamed(mut v: Value, pairs: &[(&str, &str)]) -> Value {
    // Replace complete serialized string values, never JSON keys or substrings.
    let mut text = serde_json::to_string(&v).unwrap();
    for (from, to) in pairs {
        text = text.replace(
            &serde_json::to_string(from).unwrap(),
            &serde_json::to_string(to).unwrap(),
        );
    }
    v = serde_json::from_str(&text).unwrap();
    v
}
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn root(c: &Contract) -> &StatefulCircuit {
    c.stateful_circuits
        .iter()
        .find(|c| c.name == "add_voter")
        .unwrap()
}
fn old(c: &Contract) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    closed_witness_admitted_merkle_insert_steps(
        root(c),
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
    )
}
fn checked(c: &Contract) -> Result<TypedPlan, effect_plan::PlanError> {
    effect_plan::lower_with_calls(
        root(c),
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect(),
    )
}
fn assert_actual(c: &Contract) {
    let attempt = checked_kernel_domain(
        root(c),
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect(),
    )
    .unwrap();
    // The kernel interface adapter already admits public Unit roots with
    // Boolean/Bytes32/Uint64 parameters; the body need not contain a kernel op.
    assert!(attempt.is_some(), "actual earlier checked kernel interface");
    let (rendered, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = rendered.unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|c| c.name == "add_voter" && c.recorded && c.observed_call)
    );
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["circuit"] == "add_voter" && r["phase"] == "public_emission" && r["helper"] == false
        })
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.ordinary");
    let routes: Vec<_> = row["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["route"].as_str().unwrap())
        .collect();
    assert_eq!(routes[0], "algebra.ordinary");
    assert!(!routes.iter().any(|route| route.starts_with("organizer.")));
    assert!(!routes.contains(&"organizer.witness-merkle"));
}
