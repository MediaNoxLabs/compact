// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete old enum-advance admission before checked composition and native errors.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION};
use crate::recorded::admission_probe::trace;
use crate::recorded::guarded_mutations::closed_authorized_enum_advance_steps;
use serde_json::{Value, json};

#[path = "enum_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "enum_migration_tests/inclusion.rs"]
mod inclusion;

fn fixture() -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("../../../tests/election-schema13-ir.json")).unwrap();
    value["schema_version"] = json!(SCHEMA_VERSION);
    value
}
fn named_mut<'a>(v: &'a mut Value, family: &str, name: &str) -> &'a mut Value {
    v[family]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn actions(v: &mut Value) -> &mut Value {
    &mut named_mut(v, "stateful_circuits", "advance")["actions"][0]["action"]["action"]["actions"]
}
fn root(c: &Contract) -> &StatefulCircuit {
    c.stateful_circuits
        .iter()
        .find(|c| c.name == "advance")
        .unwrap()
}
fn old(c: &Contract) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    closed_authorized_enum_advance_steps(
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
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn assert_actual_owner(c: &Contract) {
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = result.unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|c| c.name == "advance" && c.recorded && c.observed_call)
    );
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["circuit"] == "advance" && r["phase"] == "public_emission" && r["helper"] == false
        })
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.ordinary");
    let attempts: Vec<_> = row["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["route"].as_str().unwrap())
        .collect();
    assert_eq!(attempts[0], "algebra.ordinary");
    assert!(!attempts.iter().any(|route| route.starts_with("organizer.")));
    assert!(!attempts.contains(&"organizer.enum-advance"));
    assert!(!attempts.contains(&"organizer.witness-merkle"));
}
