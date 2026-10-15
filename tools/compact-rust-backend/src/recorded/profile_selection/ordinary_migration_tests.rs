// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Declared-interface laws; full retained-corpus qualification is a separate gate.
use super::*;
use crate::ir::{Contract, Expr, Parameter, SCHEMA_VERSION, StateReturn, StructField, Type};
use crate::recorded::admission_probe::trace;

#[path = "ordinary_migration_tests/execution.rs"]
mod execution;
#[path = "ordinary_migration_tests/interfaces.rs"]
mod interfaces;
#[path = "ordinary_migration_tests/priority.rs"]
mod priority;

fn identity(ty: Type) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        ledger_fields: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "identity".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: ty.clone(),
            }],
            result: ty,
            actions: vec![],
            return_value: StateReturn::Expression {
                value: Expr::Parameter {
                    name: "value".into(),
                },
            },
        }],
    }
}
fn ordinary(c: &Contract, name: &str) -> Result<Option<TypedPlan>, RenderError> {
    let circuits: HashMap<_, _> = c
        .stateful_circuits
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();
    ordinary_domain::attempt(
        circuits[name],
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &circuits,
    )
}
fn public(c: &Contract, name: &str) -> (String, serde_json::Value) {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    let capability = output
        .capabilities
        .circuits
        .iter()
        .find(|r| r.name == name)
        .unwrap();
    assert!(capability.recorded && capability.observed_call, "{name}");
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .unwrap()
        .clone();
    (output.source, row)
}
fn methods(plan: &TypedPlan) -> Vec<String> {
    use syn::visit_mut::VisitMut;
    struct Calls(Vec<String>);
    impl VisitMut for Calls {
        fn visit_expr_method_call_mut(&mut self, node: &mut syn::ExprMethodCall) {
            self.0.push(node.method.to_string());
            syn::visit_mut::visit_expr_method_call_mut(self, node);
        }
    }
    let steps = &plan.steps;
    let mut block: syn::Block = syn::parse_quote!({ #(#steps)* });
    let mut calls = Calls(vec![]);
    calls.visit_block_mut(&mut block);
    calls.0
}
