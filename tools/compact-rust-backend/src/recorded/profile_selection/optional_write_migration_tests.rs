// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole-old optional-write rules, actual ownership and native error precedence.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION};
use crate::recorded::admission_probe::trace;
use crate::recorded::guarded_mutations::closed_authorized_optional_write_steps;
use serde_json::{Value, json};

#[path = "optional_write_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "optional_write_migration_tests/inclusion.rs"]
mod inclusion;

fn fixture() -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("../../../tests/election-schema13-ir.json")).unwrap();
    value["schema_version"] = json!(SCHEMA_VERSION);
    value
}
fn circuit_mut<'a>(value: &'a mut Value, family: &str, name: &str) -> &'a mut Value {
    value[family]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn root_mut(value: &mut Value) -> &mut Value {
    circuit_mut(value, "stateful_circuits", "set_topic")
}
fn continuation(value: &mut Value) -> &mut Value {
    &mut root_mut(value)["actions"][0]["action"]["action"]["actions"]
}
fn contract(value: Value) -> Contract {
    serde_json::from_value(value).unwrap()
}
fn root(c: &Contract) -> &StatefulCircuit {
    c.stateful_circuits
        .iter()
        .find(|c| c.name == "set_topic")
        .unwrap()
}
fn old(c: &Contract) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    closed_authorized_optional_write_steps(
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
fn replace_strings(value: &mut Value, replacements: &[(&str, &str)]) {
    match value {
        Value::String(s) => {
            if let Some((_, replacement)) =
                replacements.iter().find(|(from, _)| *from == s.as_str())
            {
                *s = (*replacement).into();
            }
        }
        Value::Object(map) => map
            .values_mut()
            .for_each(|v| replace_strings(v, replacements)),
        Value::Array(items) => items
            .iter_mut()
            .for_each(|v| replace_strings(v, replacements)),
        _ => {}
    }
}
fn edit_nodes(value: &mut Value, edit: &mut impl FnMut(&mut serde_json::Map<String, Value>)) {
    match value {
        Value::Object(map) => {
            edit(map);
            map.values_mut().for_each(|v| edit_nodes(v, edit));
        }
        Value::Array(items) => items.iter_mut().for_each(|v| edit_nodes(v, edit)),
        _ => {}
    }
}

#[test]
fn actual_owner_and_eager_sibling_order_are_preserved() {
    let c = contract(fixture());
    assert!(old(&c).unwrap().is_some(), "complete old premise first");
    assert!(checked(&c).is_ok());
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    let output = result.unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|c| c.name == "set_topic" && c.recorded && c.observed_call)
    );
    for (name, owner) in [
        ("set_topic", "algebra.membership"),
        ("advance", "algebra.membership"),
        ("add_voter", "algebra.kernel"),
    ] {
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["circuit"] == name
                    && row["phase"] == "public_emission"
                    && row["helper"] == false
            })
            .unwrap();
        assert_eq!(row["body_owner"], owner, "{name}");
        let attempts: Vec<_> = row["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["route"].as_str().unwrap())
            .collect();
        assert_eq!(attempts[0], "algebra.membership");
        assert!(!attempts.iter().any(|route| route.starts_with("organizer.")));
        assert!(!attempts.contains(&"organizer.optional-write"));
        assert!(!attempts.contains(&"organizer.enum-advance"));
        assert!(!attempts.contains(&"organizer.witness-merkle"));
        if name == "set_topic" {
            assert_eq!(attempts[0], "algebra.membership");
        }
    }
}
