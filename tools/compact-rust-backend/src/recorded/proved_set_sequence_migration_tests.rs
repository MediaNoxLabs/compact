// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete legacy witness-sequence admission, not predicate-only inclusion.
use super::*;
use crate::ir::Contract;
use admission_probe::trace;
use serde_json::{Value, json};
#[path = "proved_set_sequence_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "proved_set_sequence_migration_tests/inclusion.rs"]
mod inclusion;
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../tests/welcome-schema20-ir.json")).unwrap()
}
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn named<'a>(v: &'a mut Value, collection: &str, name: &str) -> &'a mut Value {
    v[collection]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["name"] == name)
        .unwrap()
}
fn root(v: &mut Value) -> &mut Value {
    named(v, "stateful_circuits", "check_in")
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
struct Context<'a> {
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    circuits: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Context<'a> {
    fn new(c: &'a Contract) -> Self {
        Self {
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
    fn checked(
        &self,
        r: &StatefulCircuit,
    ) -> Result<typed_plan::TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(r, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn old(&self, r: &StatefulCircuit) -> Result<bool, RenderError> {
        if !proved_opaque_set_sequence(r) {
            return Ok(false);
        }
        let parameters = r
            .parameters
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.as_str(), (&p.ty, ident(&format!("arg{i}")).unwrap())))
            .collect();
        let mut steps = Vec::new();
        let mut next_temp = 0;
        let mut visiting = HashSet::from([r.name.clone()]);
        for (i, action) in r.actions.iter().enumerate() {
            if !append_steps(
                action,
                &format!("actions[{i}]"),
                &HashMap::new(),
                &parameters,
                &self.ledger,
                &self.witnesses,
                &self.pure,
                &self.circuits,
                &HashSet::new(),
                &mut steps,
                &mut next_temp,
                &mut visiting,
                &mut LegacyActionPrecision::Concrete,
            )?
            .is_supported()
            {
                return Ok(false);
            }
        }
        Ok(!steps.is_empty())
    }
}
fn current(c: &Contract, name: &str) -> String {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    let cx = Context::new(c);
    let r = cx.circuits[name];
    let plan = cx.checked(r).unwrap();
    let steps = &plan.steps;
    let text = quote::quote!(#(#steps)*).to_string();
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
    assert_eq!(row["body_owner"], "algebra.ordinary");
    assert!(
        row["guards"].as_array().unwrap().is_empty(),
        "all explicit shape permits are now retired; generic rejection remains"
    );
    assert_eq!(text.matches("record_member").count(), 1);
    assert_eq!(text.matches("record_insert").count(), 1);
    assert_eq!(text.matches("try_witness_metered").count(), 1);
    assert!(text.find("record_member").unwrap() < text.find("Not eligible participant").unwrap());
    assert!(text.find("Not eligible participant").unwrap() < text.find("record_insert").unwrap());
    assert!(text.find("record_insert").unwrap() < text.find("try_witness_metered").unwrap());
    text
}
fn included(c: &Contract, name: &str) {
    let cx = Context::new(c);
    assert!(cx.old(cx.circuits[name]).unwrap(), "complete old {name}");
    current(c, name);
}
