// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! The final organizer oracle, native validation, and independent checked owners.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION};
use crate::recorded::admission_probe::trace;
use crate::recorded::guarded_mutations::closed_organizer_gate_steps;
use serde_json::{Value, json};

#[path = "organizer_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "organizer_migration_tests/inclusion.rs"]
mod inclusion;
const ROOTS: [&str; 2] = ["add_participant", "add_organizer"];
fn fixture() -> Value {
    let mut v: Value = serde_json::from_str(include_str!(
        "../../../tests/welcome-organizer-schema13-ir.json"
    ))
    .unwrap();
    v["schema_version"] = json!(SCHEMA_VERSION);
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
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn old(c: &Contract, name: &str) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    closed_organizer_gate_steps(
        c.stateful_circuits.iter().find(|c| c.name == name).unwrap(),
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect(),
    )
}
fn checked(c: &Contract, name: &str) -> Result<TypedPlan, effect_plan::PlanError> {
    effect_plan::lower_with_calls(
        c.stateful_circuits.iter().find(|c| c.name == name).unwrap(),
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
                *s = (*to).into()
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
fn assert_actual(c: &Contract) {
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = result.unwrap();
    for (name, owner) in [
        ("add_participant", "algebra.membership"),
        ("add_organizer", "algebra.kernel"),
    ] {
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
        assert_eq!(row["body_owner"], owner, "{name}");
        assert!(
            !row["attempts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["route"].as_str().unwrap().starts_with("organizer."))
        );
    }
}
fn assert_old_positive(c: &Contract, label: &str) {
    for name in ROOTS {
        assert!(old(c, name).unwrap().is_some(), "whole old {label} {name}");
        assert!(checked(c, name).is_ok(), "checked {label} {name}");
    }
    assert_actual(c);
}
