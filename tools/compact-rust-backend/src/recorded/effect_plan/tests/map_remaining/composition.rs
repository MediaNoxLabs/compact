// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Map metadata and default insertion compose without shape-specific routing.
use super::*;

#[test]
fn metadata_preserves_distinct_chunked_slots_and_scalar_types_are_not_read() {
    let mut c = contract(Type::Field, 3, false);
    c.ledger_fields[0].path = vec![0, 0];
    let mut second = c.ledger_fields[0].clone();
    second.id = "another".into();
    second.path = vec![0, 1];
    second.declaration = LedgerFieldKind::Map {
        key: Type::Boolean,
        value: Type::LedgerMap {
            key: Box::new(Type::Boolean),
            value: Box::new(Type::Field),
        },
    };
    c.ledger_fields.push(second);
    for index in 0..15 {
        c.ledger_fields.push(LedgerField {
            id: format!("padding{index}"),
            index: 1,
            path: vec![1, index],
            source: None,
            declaration: LedgerFieldKind::Counter,
        });
    }
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::MapIsEmpty {
            field: "another".into(),
            index: 0,
        },
    };
    let plan = checked(&c).unwrap();
    let Operation::MapIsEmpty { slot } = &plan.plan().body.bindings[index(plan.plan())].operation
    else {
        unreachable!()
    };
    assert_eq!(
        plan.declarations().field(slot.declaration).unwrap().path,
        vec![0, 1]
    );
    assert!(matches!(
        plan.declarations()
            .field(slot.declaration)
            .unwrap()
            .declaration,
        LedgerFieldKind::Map {
            value: Type::LedgerMap { .. },
            ..
        }
    ));
    crate::render_with_capabilities(&c).unwrap();
    // Direct size observes the same nested slot, without constructing a nested value.
    c.stateful_circuits[0].result = validate::uint("18446744073709551615");
    c.stateful_circuits[0].return_value = StateReturn::MapSize {
        field: "another".into(),
        index: 0,
    };
    let plan = checked(&c).unwrap();
    assert!(matches!(
        plan.plan().body.bindings[index(plan.plan())].operation,
        Operation::MapSize { .. }
    ));
    crate::render_with_capabilities(&c).unwrap();
}

#[test]
fn lazy_let_bound_witness_key_is_once_before_default_insert_and_reset() {
    let mut c = contract(Type::Field, 2, true);
    c.witnesses.push(WitnessDeclaration {
        name: "next_key".into(),
        source: None,
        parameters: vec![],
        result: Type::Boolean,
    });
    c.stateful_circuits[0].actions = vec![StateAction::If {
        condition: Expr::Boolean { value: false },
        then: Box::new(StateAction::MapInsertDefault {
            field: "table".into(),
            index: 0,
            key: Expr::WitnessCall {
                name: "next_key".into(),
                arguments: vec![],
            },
        }),
        otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
    }];
    assert!(matches!(
        crate::render_with_capabilities(&c),
        Err(crate::RenderError::EffectfulExpression)
    ));
    let StateAction::If { then, .. } = &mut c.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    **then = StateAction::Let {
        bindings: vec![LocalBinding {
            name: "key".into(),
            ty: Type::Boolean,
            value: Expr::WitnessCall {
                name: "next_key".into(),
                arguments: vec![],
            },
        }],
        action: Box::new(StateAction::Sequence {
            actions: vec![
                StateAction::MapInsertDefault {
                    field: "table".into(),
                    index: 0,
                    key: key(),
                },
                StateAction::MapReset {
                    field: "table".into(),
                    index: 0,
                },
            ],
        }),
    };
    let plan = checked(&c).unwrap();
    let bindings = &plan.plan().body.bindings;
    assert!(!bindings.iter().any(|b| matches!(
        b.operation,
        Operation::Witness { .. } | Operation::MapInsertDefault { .. } | Operation::MapReset { .. }
    )));
    let (then, otherwise) = bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::If {
                then, otherwise, ..
            } => Some((then, otherwise)),
            _ => None,
        })
        .unwrap();
    let witnesses = then
        .bindings
        .iter()
        .filter(|b| matches!(b.operation, Operation::Witness { .. }))
        .collect::<Vec<_>>();
    assert_eq!(witnesses.len(), 1);
    let map_ops = then
        .bindings
        .iter()
        .filter(|b| {
            matches!(
                b.operation,
                Operation::MapInsertDefault { .. } | Operation::MapReset { .. }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(map_ops.len(), 2);
    let Operation::MapInsertDefault { key, .. } = &map_ops[0].operation else {
        panic!("default insert before reset")
    };
    assert_eq!(*key, witnesses[0].value);
    assert!(witnesses[0].path.ends_with(".then.bindings[0].value"));
    assert!(matches!(map_ops[1].operation, Operation::MapReset { .. }));
    assert!(!otherwise.bindings.iter().any(|b| matches!(
        b.operation,
        Operation::Witness { .. } | Operation::MapInsertDefault { .. } | Operation::MapReset { .. }
    )));
    assert!(matches!(
        bindings.last().unwrap().operation,
        Operation::MapSize { .. }
    ));
    render(&plan).unwrap();
    crate::render_with_capabilities(&c).unwrap();
}
