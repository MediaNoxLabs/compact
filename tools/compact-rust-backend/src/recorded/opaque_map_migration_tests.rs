// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole Map-write admission includes actual shared-helper discovery.
use super::*;
use crate::ir::Contract;
use admission_probe::trace;
use legacy::opaque_map::closed_typed_opaque_map_write_steps;
use serde_json::{Value, json};
#[path = "opaque_map_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "opaque_map_migration_tests/inclusion.rs"]
mod inclusion;
const ROOTS: [&str; 2] = ["setRecord", "setCustodyGrant"];
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../tests/asset-registry-schema20-ir.json")).unwrap()
}
// Explicit synthetic complete contract: remove unrelated constructor and roots,
// but retain every helper required by the selected public entry point.
fn isolated(root: &str) -> Value {
    let mut v = fixture();
    v.as_object_mut().unwrap().remove("constructor");
    v["stateful_circuits"].as_array_mut().unwrap().retain(|c| {
        [root, "assertWritable", "recordWrite", "recordExists"]
            .iter()
            .any(|name| c["name"] == *name)
    });
    v["circuits"]
        .as_array_mut()
        .unwrap()
        .retain(|c| root == "setRecord" && c["name"] == "assertRecordClassKnown");
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
fn actions<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    &mut named(v, name)["actions"][0]["action"]["action"]["action"]["actions"]
}
fn branch<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    let a = actions(v, name).as_array_mut().unwrap();
    a.iter_mut().find(|x| x["kind"] == "if").unwrap()
}
fn insertion<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    actions(v, name)
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|x| x["kind"] == "map_insert")
        .unwrap()
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
    fn new(c: &'a Contract, name: &str) -> Self {
        Self {
            c,
            root: c.stateful_circuits.iter().find(|r| r.name == name).unwrap(),
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
    fn discovered(&self) -> Result<HashSet<String>, RenderError> {
        let (names, _) = plan_recorded_helpers(
            &self.c.stateful_circuits,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )?;
        for name in &names {
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
        Ok(names)
    }
    fn old(&self) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
        let names = self.discovered()?;
        let params = self
            .root
            .parameters
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.as_str(), (&p.ty, ident(&format!("arg{i}")).unwrap())))
            .collect();
        closed_typed_opaque_map_write_steps(
            self.root,
            &params,
            &self.ledger,
            &self.pure,
            &self.circuits,
            &names,
        )
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
fn assert_current(c: &Contract, name: &str) -> String {
    let plan = Context::new(c, name).checked().unwrap();
    let steps = &plan.steps;
    let text = quote::quote!(#(#steps)*).to_string();
    assert_eq!(text.matches("record_remove").count(), 1);
    assert_eq!(text.matches("record_insert").count(), 1);
    assert_eq!(
        text.matches("record_member").count(),
        3,
        "one Update query, two lazy Insert queries"
    );
    assert!(
        text.find("mutation must be Insert or Update").unwrap()
            < text.find("record_remove").unwrap()
    );
    assert!(text.find("record_remove").unwrap() < text.find("record_insert").unwrap());
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
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
    let attempts = row["attempts"].as_array().unwrap();
    assert!(
        !attempts
            .iter()
            .any(|a| a["route"] == "guard.opaque-map-write")
    );
    assert!(
        !attempts
            .iter()
            .any(|a| a["route"] == "guard.guarded-struct-pure")
    );
    text
}
fn assert_old_positive(c: &Contract, name: &str, label: &str) -> String {
    assert!(
        Context::new(c, name).old().unwrap().is_some(),
        "complete old {name}/{label}"
    );
    assert_current(c, name)
}
