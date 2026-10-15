// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent authority rejects forged slots, results and SSA operands.
use super::*;

#[test]
fn all_four_effects_revalidate_slot_kind_index_result_and_source_signature() {
    for kind in 0..4 {
        let c = contract(Type::Field, kind, true);
        let original = checked(&c).unwrap().plan().clone();
        let i = index(&original);
        for mode in ["missing", "index", "kind", "result"] {
            let mut source = c.clone();
            let mut plan = original.clone();
            match mode {
                "missing" => slot(&mut plan.body.bindings[i].operation).declaration = LedgerId(99),
                "index" => slot(&mut plan.body.bindings[i].operation).index = 9,
                "kind" => {
                    source.ledger_fields[0].declaration = LedgerFieldKind::Set { ty: Type::Boolean }
                }
                _ => {
                    plan.body.bindings[i].value.ty = Type::OpaqueString;
                    plan.body.result = plan.body.bindings[i].value.clone();
                    plan.result_type = Type::OpaqueString;
                }
            }
            assert!(
                validate::check(plan, declarations(&source)).is_err(),
                "{kind}/{mode}"
            );
        }
        let mut bad = c;
        bad.ledger_fields[0].index = 1;
        assert!(checked(&bad).is_err());
        assert!(crate::render_with_capabilities(&bad).is_err());
    }
}

#[test]
fn default_insert_rejects_missing_self_stale_wrong_live_and_escaped_keys() {
    let mut c = contract(Type::Field, 0, true);
    c.stateful_circuits[0].parameters.push(Parameter {
        name: "wrong".into(),
        ty: Type::Field,
    });
    let original = checked(&c).unwrap().plan().clone();
    let i = index(&original);
    for mode in ["missing", "self", "annotation", "live"] {
        let mut plan = original.clone();
        let own = plan.body.bindings[i].value.clone();
        let wrong = plan.parameters[1].clone();
        let Operation::MapInsertDefault { key, .. } = &mut plan.body.bindings[i].operation else {
            unreachable!()
        };
        match mode {
            "missing" => key.id = 999,
            "self" => key.id = own.id,
            "annotation" => key.ty = Type::Field,
            _ => *key = wrong,
        }
        assert!(validate::check(plan, declarations(&c)).is_err(), "{mode}");
    }
    c.stateful_circuits[0].actions.insert(
        0,
        StateAction::If {
            condition: Expr::Boolean { value: true },
            then: Box::new(StateAction::Expression {
                value: Expr::Boolean { value: false },
            }),
            otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
        },
    );
    let mut plan = checked(&c).unwrap().plan().clone();
    let escaped = plan
        .body
        .bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::If { then, .. } => then
                .bindings
                .iter()
                .find(|b| b.value.ty == Type::Boolean)
                .map(|b| b.value.clone()),
            _ => None,
        })
        .unwrap();
    let i = index(&plan);
    let Operation::MapInsertDefault { key, .. } = &mut plan.body.bindings[i].operation else {
        unreachable!()
    };
    *key = escaped;
    assert!(validate::check(plan, declarations(&c)).is_err());
}

#[test]
fn metadata_is_key_only_but_default_and_reset_preserve_scalar_codec_and_resource_policy() {
    let nested = Type::LedgerMap {
        key: Box::new(Type::Boolean),
        value: Box::new(Type::Field),
    };
    for kind in 0..4 {
        let c = contract(nested.clone(), kind, true);
        if kind < 2 {
            assert!(
                checked(&c).is_err(),
                "nested Map mutation keeps V:CellValue requirement"
            );
            let original = checked(&contract(Type::Field, kind, true))
                .unwrap()
                .plan()
                .clone();
            assert!(validate::check(original, declarations(&c)).is_err());
        } else {
            let plan = checked(&c).unwrap();
            render(&plan).unwrap();
            crate::render_with_capabilities(&c).unwrap();
        }
    }
    for value in [
        Type::Boolean,
        Type::OpaqueString,
        Type::Tuple {
            elements: vec![Type::Field; 8],
        },
        Type::Vector {
            element: Box::new(Type::Boolean),
            length: 0,
        },
        Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
        Type::Unsigned {
            max: "680564733841876926926749214863536422911".into(),
        },
    ] {
        for kind in 0..2 {
            let c = contract(value.clone(), kind, true);
            checked(&c).unwrap();
            crate::render_with_capabilities(&c).unwrap();
        }
    }
    for value in [
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
    ] {
        for kind in 0..2 {
            assert!(checked(&contract(value.clone(), kind, true)).is_err());
        }
    }
    let huge = Type::Vector {
        element: Box::new(Type::Field),
        length: 32769,
    };
    let PlanError::Unsupported(gap) = checked(&contract(huge, 0, true)).unwrap_err() else {
        panic!("resource remains unsupported")
    };
    assert!(gap.resource.is_some());
    let mut malformed = contract(Type::Field, 0, true);
    malformed.stateful_circuits[0].parameters[0].ty = Type::Unsigned { max: "01".into() };
    let LedgerFieldKind::Map { key, .. } = &mut malformed.ledger_fields[0].declaration else {
        unreachable!()
    };
    *key = malformed.stateful_circuits[0].parameters[0].ty.clone();
    assert!(checked(&malformed).is_err());
}
