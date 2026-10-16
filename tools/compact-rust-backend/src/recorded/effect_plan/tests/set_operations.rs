// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite ordinary Set domain and independent checked-plan controls.
use super::super::type_environment::TypeEnvironment;
use super::*;
#[path = "set_operations/validation.rs"]
mod validation;

fn structure(name: &str, members: Vec<Type>) -> Type {
    Type::Struct {
        name: name.into(),
        fields: members
            .into_iter()
            .enumerate()
            .map(|(i, ty)| StructField {
                name: format!("member{i}"),
                ty,
            })
            .collect(),
    }
}
fn owned() -> Type {
    structure(
        "OwnedSetKey",
        vec![
            Type::Boolean,
            Type::OpaqueString,
            Type::OpaqueBytes,
            Type::Unit,
        ],
    )
}
fn key() -> Expr {
    Expr::Parameter { name: "key".into() }
}
fn member(value: Expr) -> Expr {
    Expr::SetMember {
        field: "seen".into(),
        index: 1,
        value: Box::new(value),
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
                id: "key_cell".into(),
                index: 0,
                path: vec![0],
                source: None,
                declaration: LedgerFieldKind::Cell { ty: ty.clone() },
            },
            LedgerField {
                id: "seen".into(),
                index: 1,
                path: vec![1],
                source: None,
                declaration: LedgerFieldKind::Set { ty: ty.clone() },
            },
        ],
        stateful_circuits: vec![StatefulCircuit {
            name: "lifecycle".into(),
            source: None,
            internal: false,
            parameters: vec![],
            result: Type::Boolean,
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "key".into(),
                    ty,
                    value: Expr::CellRead {
                        field: "key_cell".into(),
                        index: 0,
                    },
                }],
                action: Box::new(StateAction::Sequence {
                    actions: vec![
                        StateAction::SetInsert {
                            field: "seen".into(),
                            index: 1,
                            value: key(),
                        },
                        StateAction::Expression {
                            value: member(key()),
                        },
                        StateAction::Expression {
                            value: Expr::SetSize {
                                field: "seen".into(),
                                index: 1,
                            },
                        },
                        StateAction::SetRemove {
                            field: "seen".into(),
                            index: 1,
                            value: key(),
                        },
                        StateAction::SetReset {
                            field: "seen".into(),
                            index: 1,
                        },
                    ],
                }),
            }],
            return_value: StateReturn::Expression {
                value: Expr::SetIsEmpty {
                    field: "seen".into(),
                    index: 1,
                },
            },
        }],
    }
}
fn checked(c: &Contract) -> Result<CheckedPlan, PlanError> {
    check_with_calls(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|s| (s.name.as_str(), s))
            .collect(),
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
fn set_slot(operation: &mut Operation) -> Option<&mut SetSlot> {
    match operation {
        Operation::SetMember { slot, .. }
        | Operation::SetInsert { slot, .. }
        | Operation::SetRemove { slot, .. }
        | Operation::SetReset { slot }
        | Operation::SetSize { slot }
        | Operation::SetIsEmpty { slot } => Some(slot),
        _ => None,
    }
}

#[test]
fn six_ordinary_operations_use_the_storage_domain_and_preserve_order() {
    for ty in [
        Type::Boolean,
        Type::Field,
        Type::Unsigned { max: "255".into() },
        Type::JubjubPoint,
        Type::Bytes { length: 0 },
        Type::Bytes { length: 3 },
        Type::Bytes { length: 32 },
        Type::Enum {
            name: "Choice".into(),
            variants: vec!["First".into(), "Second".into()],
        },
        Type::OpaqueString,
        Type::OpaqueBytes,
        owned(),
        structure("Nested", vec![owned(), Type::Unit]),
    ] {
        let c = contract(ty.clone());
        let checked = checked(&c).unwrap_or_else(|e| panic!("{ty:?}: {e:?}"));
        let tags: Vec<_> = checked
            .plan()
            .body
            .bindings
            .iter()
            .filter_map(|b| match b.operation {
                Operation::SetInsert { .. } => Some("insert"),
                Operation::SetMember { .. } => Some("member"),
                Operation::SetSize { .. } => Some("size"),
                Operation::SetRemove { .. } => Some("remove"),
                Operation::SetReset { .. } => Some("reset"),
                Operation::SetIsEmpty { .. } => Some("empty"),
                _ => None,
            })
            .collect();
        assert_eq!(
            tags,
            ["insert", "member", "size", "remove", "reset", "empty"]
        );
        let size = checked
            .plan()
            .body
            .bindings
            .iter()
            .find(|b| matches!(b.operation, Operation::SetSize { .. }))
            .unwrap();
        assert_eq!(
            size.value.ty,
            Type::Unsigned {
                max: u64::MAX.to_string()
            }
        );
        render(&checked).unwrap();
        crate::render_with_capabilities(&c).unwrap();
    }
}

#[test]
fn specialized_queries_use_root_parameters_while_expression_return_uses_final_local() {
    for specialized in [true, false] {
        let mut c = contract(Type::Boolean);
        let root = &mut c.stateful_circuits[0];
        root.parameters = vec![Parameter {
            name: "key".into(),
            ty: Type::Boolean,
        }];
        root.return_value = if specialized {
            StateReturn::SetMember {
                field: "seen".into(),
                index: 1,
                value: key(),
            }
        } else {
            StateReturn::Expression {
                value: member(key()),
            }
        };
        let checked = checked(&c).unwrap();
        let plan = checked.plan();
        let read = plan
            .body
            .bindings
            .iter()
            .find(|b| matches!(b.operation, Operation::CellRead(_)))
            .unwrap();
        let Operation::SetMember { value, .. } = &plan.body.bindings.last().unwrap().operation
        else {
            panic!("member return")
        };
        assert_eq!(
            value.id,
            if specialized {
                plan.parameters[0].id
            } else {
                read.value.id
            }
        );
        render(&checked).unwrap();
    }
    for return_value in [
        StateReturn::SetSize {
            field: "seen".into(),
            index: 1,
        },
        StateReturn::SetIsEmpty {
            field: "seen".into(),
            index: 1,
        },
    ] {
        let mut c = contract(Type::Boolean);
        c.stateful_circuits[0].result = if matches!(return_value, StateReturn::SetSize { .. }) {
            Type::Unsigned {
                max: u64::MAX.to_string(),
            }
        } else {
            Type::Boolean
        };
        c.stateful_circuits[0].return_value = return_value;
        render(&checked(&c).unwrap()).unwrap();
    }
}

#[test]
fn real_source_exports_are_admitted_by_checked_composition() {
    let source: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-set-schema20-ir.json"
    ))
    .unwrap();
    for index in 0..source.stateful_circuits.len() {
        let mut candidate = source.clone();
        candidate.stateful_circuits.swap(0, index);
        let name = &candidate.stateful_circuits[0].name;
        checked(&candidate).unwrap_or_else(|gap| panic!("{name}: {gap:?}"));
    }
}
