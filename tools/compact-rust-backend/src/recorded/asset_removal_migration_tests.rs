// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! The old asset-removal permit is only one premise of complete admission.
use super::*;
use crate::ir::Contract;
use admission_probe::trace;
use serde_json::{Value, json};
#[path = "asset_removal_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "asset_removal_migration_tests/inclusion.rs"]
mod inclusion;
const ROOT: &str = "removeRecord";
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../tests/asset-registry-schema20-ir.json")).unwrap()
}
fn isolated() -> Value {
    let mut v = fixture();
    // A complete synthetic closure, not a projection presented as genuine source.
    v.as_object_mut().unwrap().remove("constructor");
    v["stateful_circuits"].as_array_mut().unwrap().retain(|c| {
        [ROOT, "assertWritable", "recordWrite"]
            .iter()
            .any(|n| c["name"] == *n)
    });
    v["circuits"] = json!([]);
    v
}
fn named<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn actions(v: &mut Value) -> &mut Vec<Value> {
    named(v, ROOT)["actions"][0]["action"]["actions"]
        .as_array_mut()
        .unwrap()
}
fn rename(v: &mut Value, from: &str, to: &str) {
    match v {
        Value::String(s) if s == from => *s = to.into(),
        Value::Object(m) => m.values_mut().for_each(|v| rename(v, from, to)),
        Value::Array(a) => a.iter_mut().for_each(|v| rename(v, from, to)),
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
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
struct Context<'a> {
    c: &'a Contract,
    root: &'a StatefulCircuit,
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    circuits: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Context<'a> {
    fn new(c: &'a Contract) -> Self {
        Self {
            c,
            root: c.stateful_circuits.iter().find(|r| r.name == ROOT).unwrap(),
            ledger: c.ledger_fields.iter().map(|x| (x.id.as_str(), x)).collect(),
            witnesses: c.witnesses.iter().map(|x| (x.name.as_str(), x)).collect(),
            pure: c.circuits.iter().map(|x| (x.name.as_str(), x)).collect(),
            circuits: c
                .stateful_circuits
                .iter()
                .map(|x| (x.name.as_str(), x))
                .collect(),
        }
    }
    fn prefix(&self) -> bool {
        closed_opaque_asset_removal(self.root, &self.ledger)
    }
    fn old_complete(&self) -> Result<bool, RenderError> {
        if !self.prefix() {
            return Ok(false);
        }
        // Use actual discovery, then verify independently renderable helper bodies.
        // Frozen signed-599 representatives separately authenticate historical
        // discovery; this executable law also catches future discovery losses.
        let (shared, _) = plan_recorded_helpers(
            &self.c.stateful_circuits,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )?;
        for name in &shared {
            assert!(
                render_recorded_helper(
                    self.circuits[name.as_str()],
                    &self.ledger,
                    &self.witnesses,
                    &self.pure,
                    &self.circuits,
                    &HashSet::new()
                )?
                .is_supported()
            );
        }
        let parameters = self
            .root
            .parameters
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.as_str(), (&p.ty, ident(&format!("arg{i}")).unwrap())))
            .collect();
        let mut steps = Vec::new();
        let result = append_steps(
            &self.root.actions[0],
            "actions[0]",
            &HashMap::new(),
            &parameters,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
            &shared,
            &mut steps,
            &mut 0,
            &mut HashSet::from([self.root.name.clone()]),
            &mut LegacyActionPrecision::Concrete,
        )?;
        Ok(result.is_supported() && !steps.is_empty())
    }
    fn checked(&self) -> Result<typed_plan::TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(
            self.root,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
}
fn assert_current(c: &Contract) -> String {
    let plan = Context::new(c).checked().unwrap();
    let steps = &plan.steps;
    let text = quote::quote!(#(#steps)*).to_string();
    assert_eq!(text.matches("record_remove").count(), 1);
    assert_eq!(text.matches("record_insert").count(), 1);
    assert!(text.find("record does not exist").unwrap() < text.find("record_remove").unwrap());
    assert!(text.find("record is still watched").unwrap() < text.find("record_remove").unwrap());
    assert!(text.find("record_remove").unwrap() < text.find("record_insert").unwrap());
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == ROOT && r.recorded && r.observed_call)
    );
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["circuit"] == ROOT && r["phase"] == "public_emission" && r["helper"] == false)
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.membership");
    let guards = row["guards"].as_array().unwrap();
    assert!(!guards.iter().any(|g| matches!(
        g["route"].as_str(),
        Some(
            "guard.closed-opaque-asset-removal"
                | "guard.closed-opaque-set-operation"
                | "guard.closed-opaque-map-operation"
        )
    )));
    assert!(
        guards.is_empty(),
        "all explicit opaque shape permits retired"
    );
    text
}
fn assert_old_positive(c: &Contract, label: &str) -> String {
    assert!(
        Context::new(c).old_complete().unwrap(),
        "complete old: {label}"
    );
    assert_current(c)
}
