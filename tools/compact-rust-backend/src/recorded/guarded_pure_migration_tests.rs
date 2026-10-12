// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete old guard plus continuation, separately checked against native authority.
use super::*;
use crate::ir::Contract;
use admission_probe::trace;
use legacy::guarded_pure::closed_guarded_struct_pure_steps;
use serde_json::{Value, json};

#[path = "guarded_pure_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "guarded_pure_migration_tests/inclusion.rs"]
mod inclusion;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../tests/guarded-assert-arith-schema20-ir.json"
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
    named(v, "stateful_circuits", "recordFreshEnough")
}
fn pure(v: &mut Value) -> &mut Value {
    named(v, "circuits", "assertFreshEnough")
}
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn target<'a>(c: &'a Contract, name: &str) -> &'a StatefulCircuit {
    c.stateful_circuits.iter().find(|c| c.name == name).unwrap()
}
fn old_prefix(c: &Contract, name: &str) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    let root = target(c, name);
    closed_guarded_struct_pure_steps(
        root,
        &root
            .parameters
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.as_str(), (&p.ty, ident(&format!("arg{i}")).unwrap())))
            .collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect(),
    )
}
// Preserve complete continuation qualification: the prefix alone never certifies
// arbitrary helper bodies. Discovery and the generic continuation remain live,
// unchanged production algorithms; signed baseline source controls are archived.
fn old_complete(c: &Contract, name: &str) -> Result<bool, RenderError> {
    let Some(mut steps) = old_prefix(c, name)? else {
        return Ok(false);
    };
    let root = target(c, name);
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let pure = c.circuits.iter().map(|p| (p.name.as_str(), p)).collect();
    let calls = c
        .stateful_circuits
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    let (shared, _) =
        plan_recorded_helpers(&c.stateful_circuits, &ledger, &witnesses, &pure, &calls)?;
    let parameters = root
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), (&p.ty, ident(&format!("arg{i}")).unwrap())))
        .collect();
    let mut next = 0;
    let result = append_steps(
        &root.actions[1],
        "actions[1]",
        &HashMap::new(),
        &parameters,
        &ledger,
        &witnesses,
        &pure,
        &calls,
        &shared,
        &mut steps,
        &mut next,
        &mut HashSet::from([name.to_owned()]),
        &mut LegacyActionPrecision::Concrete,
    )?;
    Ok(result.is_supported())
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
fn assert_current(c: &Contract, name: &str) -> String {
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = result.unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == name && r.recorded && r.observed_call)
    );
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.unit-composition");
    assert!(
        !row["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["route"] == "guard.guarded-struct-pure")
    );
    let plan = checked(c, name).unwrap();
    let steps = &plan.steps;
    quote::quote!(#(#steps)*).to_string()
}
fn assert_old_positive(c: &Contract, name: &str, following: &str) {
    // Render the entire contract, not just the prefix or selected circuit.
    crate::render_with_capabilities(c).unwrap();
    assert!(
        old_complete(c, name).unwrap(),
        "complete old continuation {name}"
    );
    let text = assert_current(c, name);
    assert!(
        text.find("pure_circuits").unwrap() < text.find(following).unwrap(),
        "pure guard precedes {following}"
    );
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
fn helper(v: &mut Value, name: &str, actions: Value) {
    v["stateful_circuits"].as_array_mut().unwrap().push(json!({"name":name,"parameters":[],"result":{"kind":"unit"},"return_value":{"kind":"unit"},"actions":actions,"internal":true}));
}
fn call(name: &str) -> Value {
    json!({"kind":"circuit_call","name":name,"arguments":[]})
}
