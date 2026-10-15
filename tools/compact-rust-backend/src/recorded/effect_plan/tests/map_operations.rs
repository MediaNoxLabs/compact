// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Map composition laws; immutable authority and runtime execution have separate tests.
use super::super::type_environment::TypeEnvironment;
use super::*;
#[path = "map_operations/removal.rs"]
mod removal;
#[path = "map_operations/source.rs"]
mod source;
#[path = "map_operations/validation.rs"]
mod validation;
#[path = "map_operations/value_codecs.rs"]
mod value_codecs;

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn query(lookup: bool) -> Expr {
    if lookup {
        Expr::MapLookup {
            field: "table".into(),
            index: 0,
            key: Box::new(parameter("key")),
        }
    } else {
        Expr::MapMember {
            field: "table".into(),
            index: 0,
            key: Box::new(parameter("key")),
        }
    }
}
fn contract(key: Type, value: Type) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        circuits: vec![],
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "table".into(),
            index: 0,
            path: vec![0],
            source: None,
            declaration: LedgerFieldKind::Map {
                key: key.clone(),
                value: value.clone(),
            },
        }],
        stateful_circuits: vec![StatefulCircuit {
            name: "lifecycle".into(),
            source: None,
            internal: false,
            parameters: vec![
                Parameter {
                    name: "key".into(),
                    ty: key,
                },
                Parameter {
                    name: "input".into(),
                    ty: value,
                },
                Parameter {
                    name: "wrong".into(),
                    ty: Type::Boolean,
                },
            ],
            result: Type::Boolean,
            actions: vec![
                StateAction::MapInsert {
                    field: "table".into(),
                    index: 0,
                    key: parameter("key"),
                    value: parameter("input"),
                },
                StateAction::Expression { value: query(true) },
                StateAction::MapRemove {
                    field: "table".into(),
                    index: 0,
                    key: parameter("key"),
                },
            ],
            return_value: StateReturn::Expression {
                value: query(false),
            },
        }],
    }
}
fn checked(c: &Contract) -> Result<CheckedPlan, PlanError> {
    check(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
    )
}
fn declarations(c: &Contract) -> declarations::Declarations {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let types = TypeEnvironment::from_sources(
        &c.stateful_circuits[0],
        &ledger,
        &witnesses,
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::new(&ledger, &witnesses).with_type_environment(types)
}
fn map_slot(operation: &mut Operation) -> &mut MapSlot {
    match operation {
        Operation::MapMember { slot, .. }
        | Operation::MapLookup { slot, .. }
        | Operation::MapInsert { slot, .. }
        | Operation::MapRemove { slot, .. } => slot,
        _ => panic!("Map operation"),
    }
}
fn map_key(operation: &mut Operation) -> &mut Value {
    match operation {
        Operation::MapMember { key, .. }
        | Operation::MapLookup { key, .. }
        | Operation::MapInsert { key, .. }
        | Operation::MapRemove { key, .. } => key,
        _ => panic!("Map operation"),
    }
}
fn rendered(checked: &CheckedPlan) -> String {
    let steps = render(checked).unwrap().steps;
    quote::quote!(#(#steps)*).to_string()
}
#[test]
fn four_map_operations_compose_in_order_with_exact_declared_types() {
    for value in [
        Type::Field,
        Type::Boolean,
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::JubjubPoint,
        Type::Bytes { length: 32 },
        validate::uint("255"),
        Type::Enum {
            name: "Choice".into(),
            variants: vec!["First".into(), "Last".into()],
        },
        Type::Struct {
            name: "Owned".into(),
            fields: vec![
                StructField {
                    name: "name".into(),
                    ty: Type::OpaqueString,
                },
                StructField {
                    name: "point".into(),
                    ty: Type::JubjubPoint,
                },
            ],
        },
    ] {
        let c = contract(Type::OpaqueString, value.clone());
        let checked = checked(&c).unwrap();
        let bindings = &checked.plan().body.bindings;
        assert!(matches!(bindings[0].operation, Operation::MapInsert { .. }));
        assert!(matches!(bindings[1].operation, Operation::MapLookup { .. }));
        assert_eq!(bindings[1].value.ty, value);
        assert!(matches!(bindings[2].operation, Operation::MapRemove { .. }));
        assert!(matches!(bindings[3].operation, Operation::MapMember { .. }));
        let source = rendered(&checked);
        for method in [
            "record_insert",
            "record_lookup",
            "record_remove",
            "record_member",
        ] {
            assert!(source.contains(method));
        }
        crate::render_with_capabilities(&c).unwrap();
    }
}
#[test]
fn direct_map_returns_preserve_root_scope_and_share_query_operations() {
    for lookup in [false, true] {
        for direct in [false, true] {
            let mut c = contract(Type::Field, Type::Field);
            let root = &mut c.stateful_circuits[0];
            root.result = if lookup { Type::Field } else { Type::Boolean };
            root.actions = vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "key".into(),
                    ty: Type::Field,
                    value: Expr::FieldLiteral { value: "9".into() },
                }],
                action: Box::new(StateAction::Sequence { actions: vec![] }),
            }];
            root.return_value = match (direct, lookup) {
                (true, true) => StateReturn::MapLookup {
                    field: "table".into(),
                    index: 0,
                    key: parameter("key"),
                },
                (true, false) => StateReturn::MapMember {
                    field: "table".into(),
                    index: 0,
                    key: parameter("key"),
                },
                (false, _) => StateReturn::Expression {
                    value: query(lookup),
                },
            };
            let checked = checked(&c).unwrap();
            let plan = checked.plan();
            let mut operation = plan.body.bindings.last().unwrap().operation.clone();
            assert_eq!(
                map_key(&mut operation).id,
                if direct {
                    plan.parameters[0].id
                } else {
                    plan.body.bindings[0].value.id
                }
            );
            render(&checked).unwrap();
        }
    }
}
#[test]
fn key_only_nested_membership_and_removal_do_not_authorize_scalar_map_values() {
    let nested = Type::LedgerMap {
        key: Box::new(Type::Field),
        value: Box::new(validate::uint("18446744073709551615")),
    };
    let mut c = contract(Type::Field, nested);
    let root = &mut c.stateful_circuits[0];
    root.parameters.remove(1);
    root.actions.clear();
    let plan = checked(&c).unwrap();
    assert!(rendered(&plan).contains("record_member"));
    crate::render_with_capabilities(&c).unwrap();
    c.stateful_circuits[0].actions = vec![StateAction::MapRemove {
        field: "table".into(),
        index: 0,
        key: parameter("key"),
    }];
    let plan = checked(&c).unwrap();
    let text = rendered(&plan);
    assert_eq!(text.matches("record_remove").count(), 1);
    assert!(text.find("record_remove").unwrap() < text.find("record_member").unwrap());
    crate::render_with_capabilities(&c).unwrap();
    for action in [
        StateAction::MapInsert {
            field: "table".into(),
            index: 0,
            key: parameter("key"),
            value: Expr::Boolean { value: false },
        },
        StateAction::Expression { value: query(true) },
    ] {
        let mut invalid = c.clone();
        invalid.stateful_circuits[0].actions = vec![action];
        assert!(matches!(checked(&invalid), Err(PlanError::Unsupported(_))));
    }
}
#[test]
fn map_insertion_evaluates_key_then_value_once_before_mutation() {
    let mut c = contract(Type::Field, Type::Field);
    c.witnesses = ["key_input", "value_input"]
        .into_iter()
        .map(|name| WitnessDeclaration {
            source: None,
            name: name.into(),
            parameters: vec![],
            result: Type::Field,
        })
        .collect();
    c.stateful_circuits[0].actions = vec![StateAction::MapInsert {
        field: "table".into(),
        index: 0,
        key: Expr::WitnessCall {
            name: "key_input".into(),
            arguments: vec![],
        },
        value: Expr::WitnessCall {
            name: "value_input".into(),
            arguments: vec![],
        },
    }];
    let checked = checked(&c).unwrap();
    let plan = checked.plan();
    assert!(matches!(
        plan.body.bindings[0].operation,
        Operation::Witness {
            declaration: WitnessId(0),
            ..
        }
    ));
    assert!(matches!(
        plan.body.bindings[1].operation,
        Operation::Witness {
            declaration: WitnessId(1),
            ..
        }
    ));
    let Operation::MapInsert { key, value, .. } = &plan.body.bindings[2].operation else {
        panic!("ordered mutation")
    };
    assert_eq!(key.id, plan.body.bindings[0].value.id);
    assert_eq!(value.id, plan.body.bindings[1].value.id);
}
