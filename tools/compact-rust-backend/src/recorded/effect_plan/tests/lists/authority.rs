// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent plan checks must not trust lowering, IDs, annotations or result shapes.
use super::*;

#[test]
fn every_list_operation_rechecks_slot_kind_identity_and_exact_result() {
    for kind in 0..6 {
        let c = contract(Type::Field, kind, true);
        let original = checked(&c).unwrap().plan().clone();
        let i = index(&original);
        for mode in ["missing", "index", "kind", "result"] {
            let mut c = c.clone();
            let mut plan = original.clone();
            let Operation::List { slot, .. } = &mut plan.body.bindings[i].operation else {
                unreachable!()
            };
            match mode {
                "missing" => slot.declaration = LedgerId(999),
                "index" => slot.index = 1,
                "kind" => c.ledger_fields[0].declaration = LedgerFieldKind::Set { ty: Type::Field },
                _ => {
                    plan.body.bindings[i].value.ty = Type::OpaqueString;
                    plan.body.result = plan.body.bindings[i].value.clone();
                    plan.result_type = Type::OpaqueString;
                }
            }
            assert!(
                validate::check(plan, declarations(&c)).is_err(),
                "{kind}/{mode}"
            );
        }
    }
}

#[test]
fn push_operands_reject_missing_self_stale_wrong_live_and_escaped_values() {
    let mut c = contract(Type::Field, 0, true);
    c.stateful_circuits[0].parameters.push(Parameter {
        name: "flag".into(),
        ty: Type::Boolean,
    });
    let original = checked(&c).unwrap().plan().clone();
    let i = index(&original);
    for mode in ["missing", "self", "annotation", "live"] {
        let mut plan = original.clone();
        let output = plan.body.bindings[i].value.clone();
        let wrong = plan.parameters[1].clone();
        let Operation::List {
            operation: ListOperation::PushFront(value),
            ..
        } = &mut plan.body.bindings[i].operation
        else {
            unreachable!()
        };
        match mode {
            "missing" => value.id = 999,
            "self" => value.id = output.id,
            "annotation" => value.ty = Type::Boolean,
            _ => *value = wrong,
        }
        assert!(validate::check(plan, declarations(&c)).is_err(), "{mode}");
    }
    c.stateful_circuits[0].actions.insert(
        0,
        StateAction::If {
            condition: Expr::Boolean { value: true },
            then: Box::new(StateAction::Expression {
                value: Expr::FieldLiteral { value: "7".into() },
            }),
            otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
        },
    );
    let mut plan = checked(&c).unwrap().plan().clone();
    let hidden = plan
        .body
        .bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::If { then, .. } => then
                .bindings
                .iter()
                .find(|b| b.value.ty == Type::Field)
                .map(|b| b.value.clone()),
            _ => None,
        })
        .unwrap();
    let i = index(&plan);
    let Operation::List {
        operation: ListOperation::PushFront(value),
        ..
    } = &mut plan.body.bindings[i].operation
    else {
        unreachable!()
    };
    *value = hidden;
    assert!(validate::check(plan, declarations(&c)).is_err());
}

#[test]
fn codec_default_canonical_maybe_and_resource_boundaries_stay_independent() {
    for element in [
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Tuple { elements: vec![] },
        Type::Vector {
            element: Box::new(Type::Field),
            length: 0,
        },
        Type::Tuple {
            elements: vec![Type::Field; 8],
        },
    ] {
        let c = contract(element, 5, true);
        checked(&c).unwrap();
        crate::render_with_capabilities(&c).unwrap();
    }
    for element in [
        Type::Unit,
        Type::Tuple {
            elements: vec![Type::Field; 9],
        },
        Type::Vector {
            element: Box::new(Type::Tuple {
                elements: vec![Type::Field; 9],
            }),
            length: 0,
        },
        Type::Unsigned { max: "01".into() },
    ] {
        assert!(checked(&contract(element, 5, true)).is_err());
    }
    for name in ["Maybe", "MaybeCompact0", "MaybeCompact123"] {
        let mut c = contract(Type::Field, 5, true);
        c.stateful_circuits[0].result = maybe(Type::Field, name);
        checked(&c).unwrap();
    }
    for mode in ["name", "suffix", "order", "member", "element"] {
        let mut c = contract(Type::Field, 5, true);
        let Type::Struct { name, fields } = &mut c.stateful_circuits[0].result else {
            unreachable!()
        };
        match mode {
            "name" => *name = "Packet".into(),
            "suffix" => *name = "MaybeCompact".into(),
            "order" => fields.swap(0, 1),
            "member" => fields[0].name = "present".into(),
            _ => fields[1].ty = Type::Boolean,
        }
        assert!(checked(&c).is_err(), "{mode}");
    }
    let mut c = contract(Type::Field, 5, false);
    let StateReturn::Expression {
        value:
            Expr::ListHead {
                ty: Type::Struct { fields, .. },
                ..
            },
    } = &mut c.stateful_circuits[0].return_value
    else {
        unreachable!()
    };
    fields[1].ty = Type::Boolean;
    assert!(
        checked(&c).is_err(),
        "conflicting canonical Maybe declaration"
    );
    let huge = Type::Vector {
        element: Box::new(Type::Field),
        length: 32769,
    };
    let error = checked(&contract(huge, 3, true)).unwrap_err();
    let PlanError::Unsupported(gap) = error else {
        panic!("resource refusal must remain unsupported")
    };
    assert!(gap.resource.is_some());
}
