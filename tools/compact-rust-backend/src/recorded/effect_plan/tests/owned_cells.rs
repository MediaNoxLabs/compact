// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Owned storage has its own domain; declaration truth and public interfaces stay independent.
use super::super::type_environment::TypeEnvironment;
use super::*;
#[path = "owned_cells/validation.rs"]
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
        "Owned",
        vec![Type::Boolean, Type::OpaqueString, Type::OpaqueBytes],
    )
}
fn read() -> Expr {
    Expr::CellRead {
        field: "storage".into(),
        index: 0,
    }
}
fn contract(ty: Type) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        circuits: vec![],
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "storage".into(),
            index: 0,
            path: vec![],
            source: None,
            declaration: LedgerFieldKind::Cell { ty },
        }],
        stateful_circuits: vec![StatefulCircuit {
            name: "copy".into(),
            source: None,
            internal: false,
            parameters: vec![],
            result: Type::Unit,
            actions: vec![StateAction::CellWrite {
                field: "storage".into(),
                index: 0,
                value: read(),
            }],
            return_value: StateReturn::Unit,
        }],
    }
}
fn maps_check(contract: &Contract) -> Result<CheckedPlan, PlanError> {
    check_with_calls(
        &contract.stateful_circuits[0],
        &contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect(),
        &contract
            .witnesses
            .iter()
            .map(|w| (w.name.as_str(), w))
            .collect(),
        &contract
            .circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect(),
        &contract
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect(),
    )
}
fn declarations(contract: &Contract) -> declarations::Declarations {
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let types = TypeEnvironment::from_sources(
        &contract.stateful_circuits[0],
        &ledger,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::new(&ledger, &HashMap::new()).with_type_environment(types)
}

#[test]
fn cell_domain_preserves_prior_members_and_qualifies_recursive_codecs() {
    for ty in [
        Type::Field,
        Type::Boolean,
        Type::JubjubPoint,
        Type::Bytes { length: 3 },
        Type::Unsigned { max: "255".into() },
        structure("Prior", vec![Type::Unit, Type::Boolean]),
        Type::OpaqueString,
        Type::OpaqueBytes,
        owned(),
        structure("Nested", vec![owned(), Type::Unit]),
        Type::Tuple {
            elements: vec![Type::Boolean],
        },
        Type::Vector {
            element: Box::new(Type::OpaqueString),
            length: 1,
        },
        structure("Empty", vec![]),
        structure("ContainsEmpty", vec![structure("InnerEmpty", vec![])]),
        structure(
            "ContainsVector",
            vec![Type::Vector {
                element: Box::new(Type::OpaqueBytes),
                length: 1,
            }],
        ),
        Type::Unsigned {
            max: "340282366920938463463374607431768211456".into(),
        },
    ] {
        assert!(validate::cell_type(&ty), "{ty:?}");
        let plan = maps_check(&contract(ty)).unwrap();
        assert!(
            plan.plan()
                .body
                .bindings
                .iter()
                .any(|b| matches!(b.operation, Operation::CellRead(_)))
        );
        render(&plan).unwrap();
    }
    for ty in [
        Type::Unit,
        Type::Tuple {
            elements: vec![Type::Boolean; 9],
        },
        Type::Vector {
            element: Box::new(Type::Tuple {
                elements: vec![Type::Field; 9],
            }),
            length: 0,
        },
        Type::Unsigned {
            max: "452312848583266388373324160190187140051835877600158453279131187530910662656"
                .into(),
        },
    ] {
        assert!(!validate::cell_type(&ty), "{ty:?}");
        assert!(matches!(
            maps_check(&contract(ty)),
            Err(PlanError::Unsupported(_))
        ));
    }
}

#[test]
fn read_once_retains_owned_local_for_multiple_writes() {
    let mut contract = contract(owned());
    contract.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![LocalBinding {
            name: "saved".into(),
            ty: owned(),
            value: read(),
        }],
        action: Box::new(StateAction::Sequence {
            actions: vec![
                StateAction::CellWrite {
                    field: "storage".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "saved".into(),
                    },
                },
                StateAction::CellWrite {
                    field: "storage".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "saved".into(),
                    },
                },
            ],
        }),
    }];
    let checked = maps_check(&contract).unwrap();
    let body = &checked.plan().body.bindings;
    assert_eq!(
        body.iter()
            .filter(|b| matches!(b.operation, Operation::CellRead(_)))
            .count(),
        1
    );
    assert_eq!(
        body.iter()
            .filter(|b| matches!(b.operation, Operation::CellWrite { .. }))
            .count(),
        2
    );
    let rendered = render(&checked).unwrap();
    let steps = rendered.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert_eq!(tokens.matches("record_read").count(), 1);
    assert_eq!(tokens.matches("record_write").count(), 2);
    assert_eq!(tokens.matches("clone").count(), 2);
}
