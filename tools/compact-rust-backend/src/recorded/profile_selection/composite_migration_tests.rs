// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite old-first controls for all three composite admission domains.
use super::*;
use crate::ir::{Contract, Expr, LocalBinding, Parameter, StateReturn, StructField, Type};
use quote::quote;
#[path = "composite_migration_tests/intents.rs"]
mod intents;
#[path = "composite_migration_tests/observations.rs"]
mod observations;
#[path = "composite_migration_tests/routing.rs"]
mod routing;
#[path = "composite_migration_tests/values.rs"]
mod values;
fn unsigned(max: u128) -> Type {
    Type::Unsigned {
        max: max.to_string(),
    }
}
fn number(value: u128, max: u128) -> Expr {
    Expr::UnsignedLiteral {
        value: value.to_string(),
        max: max.to_string(),
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
fn structure(name: &str, fields: Vec<Type>) -> Type {
    Type::Struct {
        name: name.into(),
        fields: fields
            .into_iter()
            .enumerate()
            .map(|(i, ty)| StructField {
                name: format!("member{i}"),
                ty,
            })
            .collect(),
    }
}
fn record(ty: Type, fields: Vec<Expr>) -> Expr {
    Expr::StructLiteral { ty, fields }
}
fn local(name: &str, ty: Type, value: Expr, body: Expr) -> Expr {
    Expr::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty,
            value,
        }],
        body: Box::new(body),
    }
}
fn subject(result: Type, value: Expr) -> Contract {
    Contract {
        schema_version: crate::ir::SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "root".into(),
            internal: false,
            parameters: vec![],
            actions: vec![],
            result,
            return_value: StateReturn::Expression { value },
        }],
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
            ledger: c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
            witnesses: c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
            pure: c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
            circuits: c
                .stateful_circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
        }
    }
    fn old(&self, c: &StatefulCircuit) -> Option<TypedPlan> {
        typed_plan::lower_composite(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn algebra(&self, c: &StatefulCircuit) -> Result<TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn domain(&self, c: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_composite_domain(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn selected(&self, c: &StatefulCircuit) -> Result<ProfileSelection, RenderError> {
        select(
            c,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
            None,
        )
    }
}
fn tokens(plan: &TypedPlan) -> String {
    let (steps, result) = (&plan.steps, &plan.result);
    quote!(#(#steps)* #result).to_string()
}
fn included(contract: &Contract, name: &str, label: &str) {
    let root = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == name)
        .unwrap();
    let context = Context::new(contract);
    assert!(context.old(root).is_some(), "old-FIRST {label}");
    let candidate = context
        .algebra(root)
        .unwrap_or_else(|e| panic!("algebra {label}: {e:?}"));
    assert_eq!(
        tokens(&context.domain(root).unwrap().unwrap()),
        tokens(&candidate),
        "{label}"
    );
    let rendered =
        crate::render_with_capabilities(contract).unwrap_or_else(|e| panic!("render {label}: {e}"));
    let exports: Vec<_> = rendered
        .capabilities
        .circuits
        .iter()
        .filter(|c| c.name == name)
        .collect();
    if root.internal {
        assert!(exports.is_empty(), "internal {label}");
    } else {
        assert_eq!(exports.len(), 1, "{label}");
        assert!(exports[0].recorded && exports[0].observed_call, "{label}");
    }
}
