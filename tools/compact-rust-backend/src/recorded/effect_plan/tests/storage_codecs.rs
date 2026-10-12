// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Recursive ledger codecs; public FAB and Merkle policy remain independent.
use super::super::type_environment::TypeEnvironment;
use super::*;
#[path = "storage_codecs/authority.rs"]
mod authority;
#[path = "storage_codecs/domain.rs"]
mod domain;

fn vector(ty: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(ty),
        length,
    }
}
fn tuple(length: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::Field; length],
    }
}
fn product(ty: Type) -> Type {
    Type::Struct {
        name: "Stored".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty,
        }],
    }
}
fn parameter() -> Expr {
    Expr::Parameter {
        name: "value".into(),
    }
}
fn contract(ty: Type) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        circuits: vec![],
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "cell".into(),
                index: 0,
                path: vec![0],
                declaration: LedgerFieldKind::Cell { ty: ty.clone() },
            },
            LedgerField {
                source: None,
                id: "set".into(),
                index: 1,
                path: vec![1],
                declaration: LedgerFieldKind::Set { ty: ty.clone() },
            },
            LedgerField {
                source: None,
                id: "map".into(),
                index: 2,
                path: vec![2],
                declaration: LedgerFieldKind::Map {
                    key: ty.clone(),
                    value: Type::Field,
                },
            },
        ],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "roundtrip".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: ty.clone(),
            }],
            result: ty,
            actions: vec![
                StateAction::CellWrite {
                    field: "cell".into(),
                    index: 0,
                    value: parameter(),
                },
                StateAction::SetInsert {
                    field: "set".into(),
                    index: 1,
                    value: parameter(),
                },
                StateAction::Expression {
                    value: Expr::SetMember {
                        field: "set".into(),
                        index: 1,
                        value: Box::new(parameter()),
                    },
                },
                StateAction::MapInsert {
                    field: "map".into(),
                    index: 2,
                    key: parameter(),
                    value: Expr::FieldLiteral { value: "1".into() },
                },
                StateAction::Expression {
                    value: Expr::MapLookup {
                        field: "map".into(),
                        index: 2,
                        key: Box::new(parameter()),
                    },
                },
                StateAction::MapRemove {
                    field: "map".into(),
                    index: 2,
                    key: parameter(),
                },
                StateAction::SetRemove {
                    field: "set".into(),
                    index: 1,
                    value: parameter(),
                },
            ],
            return_value: StateReturn::CellRead {
                field: "cell".into(),
                index: 0,
            },
        }],
    }
}
fn checked(c: &Contract) -> Result<CheckedPlan, PlanError> {
    check(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &HashMap::new(),
    )
}
fn declarations(c: &Contract) -> declarations::Declarations {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let types = TypeEnvironment::from_sources(
        &c.stateful_circuits[0],
        &ledger,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::new(&ledger, &HashMap::new()).with_type_environment(types)
}
fn input(operation: &mut Operation) -> Option<&mut Value> {
    match operation {
        Operation::CellWrite { value, .. }
        | Operation::SetInsert { value, .. }
        | Operation::SetRemove { value, .. }
        | Operation::SetMember { value, .. } => Some(value),
        Operation::MapInsert { key, .. }
        | Operation::MapLookup { key, .. }
        | Operation::MapRemove { key, .. }
        | Operation::MapMember { key, .. } => Some(key),
        _ => None,
    }
}
