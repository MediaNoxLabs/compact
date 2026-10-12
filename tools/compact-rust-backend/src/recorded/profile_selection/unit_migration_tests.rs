// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite rule obligations start with the complete legacy evaluator, not its Audit.
use super::*;
use crate::ir::{
    Contract, Expr, LedgerFieldKind, LocalBinding, Parameter, StateAction, StateReturn,
    StructField, Type,
};
use quote::quote;

#[path = "unit_migration_tests/inclusion.rs"]
mod inclusion;
#[path = "unit_migration_tests/negative.rs"]
mod negative;
#[path = "unit_migration_tests/routing.rs"]
mod routing;

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn field() -> Expr {
    Expr::FieldLiteral { value: "7".into() }
}
fn boolean() -> Expr {
    Expr::Boolean { value: true }
}
fn bytes() -> Expr {
    Expr::BytesLiteral { bytes: vec![9; 32] }
}
fn bytes_type() -> Type {
    Type::Bytes { length: 32 }
}
fn phase() -> Type {
    Type::Enum {
        name: "Phase".into(),
        variants: vec!["Open".into(), "Closed".into()],
    }
}
fn inner() -> Type {
    Type::Struct {
        name: "Inner".into(),
        fields: vec![
            StructField {
                name: "flag".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "phase".into(),
                ty: phase(),
            },
        ],
    }
}
fn record() -> Type {
    Type::Struct {
        name: "Record".into(),
        fields: vec![
            StructField {
                name: "amount".into(),
                ty: Type::Field,
            },
            StructField {
                name: "inner".into(),
                ty: inner(),
            },
        ],
    }
}
fn variant() -> Expr {
    Expr::EnumVariant {
        ty: phase(),
        variant: "Open".into(),
    }
}
fn record_value() -> Expr {
    Expr::StructLiteral {
        ty: record(),
        fields: vec![
            field(),
            Expr::StructLiteral {
                ty: inner(),
                fields: vec![boolean(), variant()],
            },
        ],
    }
}
fn read(name: &str, index: u8) -> Expr {
    Expr::CellRead {
        field: name.into(),
        index,
    }
}
fn write(name: &str, index: u8, value: Expr) -> StateAction {
    StateAction::CellWrite {
        field: name.into(),
        index,
        value,
    }
}
fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
fn equal(left: Expr, right: Expr) -> Expr {
    Expr::Equal {
        left: Box::new(left),
        right: Box::new(right),
    }
}
fn witness(name: &str) -> Expr {
    Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    }
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
fn project(value: Expr, name: &str, index: usize) -> Expr {
    Expr::StructField {
        value: Box::new(value),
        field: name.into(),
        index,
    }
}
fn identity(name: &str, ty: Type) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: ty.clone(),
        }],
        result: ty,
        body: parameter("value"),
    }
}
fn subject() -> Contract {
    let slots = [
        ("amount", Type::Field),
        ("flag", Type::Boolean),
        ("digest", bytes_type()),
        ("phase", phase()),
        ("record", record()),
    ];
    Contract {
        schema_version: crate::ir::SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        ledger_fields: slots
            .iter()
            .enumerate()
            .map(|(index, (name, ty))| LedgerField {
                source: None,
                id: (*name).into(),
                index: index as u8,
                path: vec![index as u8],
                declaration: LedgerFieldKind::Cell { ty: ty.clone() },
            })
            .collect(),
        witnesses: slots
            .iter()
            .map(|(name, ty)| WitnessDeclaration {
                source: None,
                name: format!("w_{name}"),
                parameters: vec![],
                result: ty.clone(),
            })
            .collect(),
        circuits: slots
            .iter()
            .map(|(name, ty)| identity(&format!("p_{name}"), ty.clone()))
            .collect(),
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                name: "root".into(),
                internal: false,
                parameters: vec![Parameter {
                    name: "seed".into(),
                    ty: Type::Field,
                }],
                result: Type::Unit,
                actions: vec![],
                return_value: StateReturn::Expression {
                    value: call("store", vec![parameter("seed")]),
                },
            },
            StatefulCircuit {
                source: None,
                name: "store".into(),
                internal: true,
                parameters: vec![Parameter {
                    name: "input".into(),
                    ty: Type::Field,
                }],
                result: Type::Unit,
                actions: vec![write("amount", 0, parameter("input"))],
                return_value: StateReturn::Unit,
            },
        ],
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
        typed_plan::lower_unit_actions(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn algebra(&self, c: &StatefulCircuit) -> Result<TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn unit(&self, c: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_unit_domain(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    // Historical staged-priority oracle; production emission is checked separately.
    fn legacy_selected(&self, c: &StatefulCircuit) -> Result<ProfileSelection, RenderError> {
        legacy_select(
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
    let circuit = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == name)
        .unwrap();
    let context = Context::new(contract);
    assert!(
        context.old(circuit).is_some(),
        "old-FIRST admission failed for {label}"
    );
    let algebra = context
        .algebra(circuit)
        .unwrap_or_else(|error| panic!("algebra refused old {label}: {error:?}"));
    assert_eq!(
        tokens(&context.unit(circuit).unwrap().unwrap()),
        tokens(&algebra),
        "{label}"
    );
    let rendered = crate::render_with_capabilities(contract)
        .unwrap_or_else(|error| panic!("public render {label}: {error}"));
    let capabilities: Vec<_> = rendered
        .capabilities
        .circuits
        .iter()
        .filter(|entry| entry.name == name)
        .collect();
    assert_eq!(capabilities.len(), 1, "{label}");
    assert!(
        capabilities[0].recorded && capabilities[0].observed_call,
        "{label}"
    );
}
