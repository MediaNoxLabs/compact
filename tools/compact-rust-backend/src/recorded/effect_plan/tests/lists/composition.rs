// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Slot identity, callee return authority and lazy evaluation are compositional laws.
use super::*;

#[test]
fn chunked_slots_keep_full_identity_and_head_uses_each_callees_result() {
    let mut c = contract(Type::Field, 5, true);
    c.ledger_fields[0].path = vec![0, 0];
    let mut second = c.ledger_fields[0].clone();
    second.id = "another".into();
    second.path = vec![0, 1];
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
    c.stateful_circuits[0].return_value = StateReturn::ListHead {
        field: "another".into(),
        index: 0,
    };
    let plan = checked(&c).unwrap();
    let Operation::List {
        slot,
        operation: ListOperation::Head,
    } = &plan.plan().body.bindings[index(plan.plan())].operation
    else {
        unreachable!()
    };
    assert_eq!(
        plan.declarations().field(slot.declaration).unwrap().path,
        vec![0, 1]
    );
    crate::render_with_capabilities(&c).unwrap();

    let mut helper = c.stateful_circuits[0].clone();
    helper.name = "head_inner".into();
    helper.internal = true;
    c.stateful_circuits[0].result = Type::Field;
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::StructField {
            value: Box::new(Expr::Call {
                name: "head_inner".into(),
                arguments: vec![parameter()],
            }),
            field: "value".into(),
            index: 1,
        },
    };
    c.stateful_circuits.push(helper);
    let plan = checked(&c).unwrap();
    let body = plan
        .plan()
        .body
        .bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::Call { body, .. } => Some(body),
            _ => None,
        })
        .unwrap();
    let head = body
        .bindings
        .iter()
        .find(|b| {
            matches!(
                b.operation,
                Operation::List {
                    operation: ListOperation::Head,
                    ..
                }
            )
        })
        .unwrap();
    assert_eq!(head.value.ty, c.stateful_circuits[1].result);
    assert_ne!(head.value.ty, c.stateful_circuits[0].result);
    crate::render_with_capabilities(&c).unwrap();
}

#[test]
fn witness_push_is_once_and_branch_local_before_following_mutations() {
    let mut c = contract(Type::Field, 3, true);
    c.witnesses.push(WitnessDeclaration {
        source: None,
        name: "next".into(),
        parameters: vec![],
        result: Type::Field,
    });
    c.stateful_circuits[0].actions = vec![
        StateAction::If {
            condition: Expr::Boolean { value: false },
            then: Box::new(StateAction::ListPushFront {
                field: "items".into(),
                index: 0,
                value: Expr::WitnessCall {
                    name: "next".into(),
                    arguments: vec![],
                },
            }),
            otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
        },
        StateAction::ListPopFront {
            field: "items".into(),
            index: 0,
        },
        StateAction::ListReset {
            field: "items".into(),
            index: 0,
        },
    ];
    // The checked effect vocabulary can express the nested witness, but the
    // public native frontend owns its existing pure List operand restriction.
    assert!(matches!(
        crate::render_with_capabilities(&c),
        Err(crate::RenderError::EffectfulExpression)
    ));
    let StateAction::If { then, .. } = &mut c.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    **then = StateAction::Let {
        bindings: vec![LocalBinding {
            name: "witness_value".into(),
            ty: Type::Field,
            value: Expr::WitnessCall {
                name: "next".into(),
                arguments: vec![],
            },
        }],
        action: Box::new(StateAction::ListPushFront {
            field: "items".into(),
            index: 0,
            value: Expr::Parameter {
                name: "witness_value".into(),
            },
        }),
    };
    let plan = checked(&c).unwrap();
    let bindings = &plan.plan().body.bindings;
    assert!(
        !bindings
            .iter()
            .any(|b| matches!(b.operation, Operation::Witness { .. }))
    );
    let branch = bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::If {
                then, otherwise, ..
            } => Some((then, otherwise)),
            _ => None,
        })
        .unwrap();
    let witnesses = branch
        .0
        .bindings
        .iter()
        .filter(|b| matches!(b.operation, Operation::Witness { .. }))
        .collect::<Vec<_>>();
    assert_eq!(witnesses.len(), 1);
    let pushed = branch
        .0
        .bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::List {
                operation: ListOperation::PushFront(value),
                ..
            } => Some(value),
            _ => None,
        })
        .unwrap();
    assert_eq!(*pushed, witnesses[0].value);
    assert!(witnesses[0].path.ends_with(".then.bindings[0].value"));
    assert!(!branch.1.bindings.iter().any(|b| matches!(
        b.operation,
        Operation::Witness { .. } | Operation::List { .. }
    )));
    let kinds = bindings
        .iter()
        .filter_map(|b| match &b.operation {
            Operation::List { operation, .. } => Some(operation),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(matches!(
        kinds.as_slice(),
        [
            ListOperation::PopFront,
            ListOperation::Reset,
            ListOperation::Length
        ]
    ));
    render(&plan).unwrap();
    crate::render_with_capabilities(&c).unwrap();
}
