// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! A declared Boolean interface replaces the final closed Set permit.
use super::*;
use crate::ir::{Contract, Expr, LedgerFieldKind, Parameter, StateAction, StateReturn, Type};
use crate::recorded::{
    LegacyActionPrecision, admission_probe::trace, append_steps, closed_opaque_set_operation,
    legacy,
};
use std::collections::HashSet;
#[path = "opaque_boolean_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "opaque_boolean_migration_tests/inclusion.rs"]
mod inclusion;
fn fixture() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/opaque-string-set-schema20-ir.json"
    ))
    .unwrap()
}
fn root<'a>(c: &'a mut Contract, name: &str) -> &'a mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|r| r.name == name)
        .unwrap()
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn query() -> Expr {
    Expr::SetMember {
        field: "names".into(),
        index: 0,
        value: Box::new(parameter("name")),
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
    fn domain(&self, r: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_membership_domain(r, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn old(&self, r: &StatefulCircuit) -> Result<bool, RenderError> {
        if !closed_opaque_set_operation(r, &self.ledger) {
            return Ok(false);
        }
        let parameters = r
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
        match &r.return_value {
            StateReturn::Unit => Ok(!steps.is_empty()),
            StateReturn::SetMember {
                field,
                index,
                value,
            } => Ok(legacy::returns::collections::set::member(
                field,
                *index,
                value,
                &r.return_value,
                &parameters,
                &self.ledger,
                &self.witnesses,
                &self.circuits,
                &mut next_temp,
                &mut visiting,
            )?
            .is_supported()),
            _ => unreachable!("preserved old predicate has exactly two alternatives"),
        }
    }
}
fn current(c: &Contract, name: &str) -> String {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    let cx = Context::new(c);
    let r = cx.circuits[name];
    let plan = cx.domain(r).unwrap().expect("checked interface");
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
    assert_eq!(row["body_owner"], "algebra.membership");
    let guards = row["guards"].as_array().unwrap();
    assert!(
        !guards
            .iter()
            .any(|g| g["route"] == "guard.closed-opaque-set-operation")
    );
    for route in [
        "guard.closed-opaque-map-operation",
        "guard.proved-opaque-set-sequence",
    ] {
        assert!(
            guards.iter().any(|g| g["route"] == route),
            "preserved {route}"
        );
    }
    text
}
fn included(c: &Contract, name: &str) {
    let cx = Context::new(c);
    assert!(cx.old(cx.circuits[name]).unwrap(), "whole old {name}");
    let text = current(c, name);
    assert!(
        text.contains(
            if c.stateful_circuits
                .iter()
                .find(|r| r.name == name)
                .unwrap()
                .result
                == Type::Unit
            {
                "record_insert"
            } else {
                "record_member"
            }
        )
    );
}
