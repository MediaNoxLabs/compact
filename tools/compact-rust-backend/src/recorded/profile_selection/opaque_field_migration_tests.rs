// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete old Map forms and the bounded declared Field interface.
use super::*;
use crate::ir::{Contract, Expr, Parameter, StateReturn, Type};
use crate::recorded::{
    ClosedOpaqueMapOperation, LegacyActionPrecision, admission_probe::trace, append_steps,
    closed_opaque_map_operation,
};
use serde_json::{Value, json};
use std::collections::HashSet;
#[path = "opaque_field_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "opaque_field_migration_tests/inclusion.rs"]
mod inclusion;
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/opaque-string-map-schema20-ir.json"
    ))
    .unwrap()
}
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn named<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["name"] == name)
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
    fn terminal(&self, r: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_terminal_domain(r, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn checked(&self, r: &StatefulCircuit) -> Result<TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(r, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn old(&self, r: &StatefulCircuit) -> Result<bool, RenderError> {
        let Some(operation) = closed_opaque_map_operation(r, &self.ledger) else {
            return Ok(false);
        };
        let parameters: HashMap<_, _> = r
            .parameters
            .iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    p.name.as_str(),
                    (&p.ty, crate::ident(&format!("arg{i}")).unwrap()),
                )
            })
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
        match operation {
            ClosedOpaqueMapOperation::Put => Ok(!steps.is_empty()),
            ClosedOpaqueMapOperation::Ensure { field, key } => {
                // Reproduce only the removed bridge's fallible construction.
                // Full prior native/public acceptance is independently frozen
                // against signed603; predicate truth alone is not that premise.
                let slot = crate::ident(&field)?;
                let (_, key_ident) = parameters
                    .get(key.as_str())
                    .expect("closed Map key is a declared parameter");
                let return_step: syn::Stmt = syn::parse_quote! {
                    let (frame, observed): (_, runtime::Field) =
                        crate::ledger_slots::#slot.record_lookup(frame, (#key_ident).clone())?;
                };
                steps.push(return_step);
                Ok(!steps.is_empty())
            }
        }
    }
}
fn current(c: &Contract, name: &str, owner: &str) -> String {
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
    assert_eq!(row["body_owner"], owner);
    assert!(
        !row["guards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["route"] == "guard.closed-opaque-map-operation")
    );
    assert!(
        row["guards"].as_array().unwrap().is_empty(),
        "all explicit opaque shape permits retired"
    );
    if r.result == Type::Field {
        assert!(cx.terminal(r).unwrap().is_some());
        let attempts = row["attempts"].as_array().unwrap();
        assert_eq!(
            attempts
                .iter()
                .filter(|a| a["route"] == "algebra.ordinary")
                .count(),
            1
        );
        assert!(
            !attempts
                .iter()
                .any(|a| a["route"] == "algebra.unit-actions")
        );
    }
    text
}
fn included(c: &Contract, name: &str) {
    let cx = Context::new(c);
    assert!(cx.old(cx.circuits[name]).unwrap(), "complete old {name}");
    let text = current(c, name, "algebra.ordinary");
    if cx.circuits[name].result == Type::Unit {
        assert!(text.contains("record_insert"));
    } else {
        assert_eq!(text.matches("record_lookup").count(), 1);
        assert!(text.find("missing key").unwrap() < text.find("record_lookup").unwrap());
    }
}
