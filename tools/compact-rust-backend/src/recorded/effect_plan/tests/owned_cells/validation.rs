// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Declaration, scope, canonical shape and interface boundaries.
use super::*;

#[test]
fn forged_owned_cell_plans_require_declaration_payload_and_live_operand_identity() {
    let contract = contract(owned());
    let checked = maps_check(&contract).unwrap();
    let original = checked.plan();
    validate::check(original.clone(), declarations(&contract)).unwrap();
    for mutation in 0..8 {
        let mut plan = original.clone();
        match mutation {
            0 => {
                let Operation::CellRead(slot) = &mut plan.body.bindings[0].operation else {
                    unreachable!()
                };
                slot.declaration = LedgerId(99);
            }
            1 => {
                let Operation::CellRead(slot) = &mut plan.body.bindings[0].operation else {
                    unreachable!()
                };
                slot.index = 9;
            }
            2 => {
                plan.body.bindings[0].value.ty = Type::OpaqueString;
            }
            3 => {
                let Operation::CellWrite { value, .. } = &mut plan.body.bindings[1].operation
                else {
                    unreachable!()
                };
                value.id = 99;
            }
            4 => {
                let Operation::CellWrite { value, .. } = &mut plan.body.bindings[1].operation
                else {
                    unreachable!()
                };
                value.ty = Type::OpaqueBytes;
            }
            5 => {
                let Type::Struct { fields, .. } = &mut plan.body.bindings[0].value.ty else {
                    unreachable!()
                };
                fields[1].name = "forged".into();
            }
            6 => {
                let Operation::CellWrite { slot, .. } = &mut plan.body.bindings[1].operation else {
                    unreachable!()
                };
                slot.index = 1;
            }
            7 => {
                plan.body.bindings.swap(0, 1);
            }
            _ => unreachable!(),
        }
        assert!(
            validate::check(plan, declarations(&contract)).is_err(),
            "forgery {mutation}"
        );
    }
    let mut wrong_kind = contract.clone();
    let mut canonical = contract.ledger_fields[0].clone();
    canonical.id = "zz_canonical".into();
    canonical.index = 1;
    wrong_kind.ledger_fields.push(canonical.clone());
    wrong_kind.ledger_fields[0].declaration = LedgerFieldKind::Counter;
    let gap = validate::check(original.clone(), declarations(&wrong_kind)).unwrap_err();
    assert!(gap.detail.contains("expected Cell declaration"), "{gap:?}");
    let mut mismatched = contract.clone();
    mismatched.ledger_fields.push(canonical);
    mismatched.ledger_fields[0].declaration = LedgerFieldKind::Cell {
        ty: Type::OpaqueString,
    };
    assert!(validate::check(original.clone(), declarations(&mismatched)).is_err());
}

#[test]
fn owned_cells_and_public_opaque_interfaces_keep_witness_results_separate() {
    for ty in [Type::OpaqueString, Type::OpaqueBytes, owned()] {
        assert!(validate::cell_type(&ty));
        assert!(!validate::surface_type(&ty));
        let mut source = contract(ty.clone());
        source.stateful_circuits[0].parameters = vec![Parameter {
            name: "arg".into(),
            ty: ty.clone(),
        }];
        render(&maps_check(&source).unwrap()).unwrap();
        source.stateful_circuits[0].parameters.clear();
        source.stateful_circuits[0].result = ty.clone();
        source.stateful_circuits[0].return_value = StateReturn::Expression { value: read() };
        render(&maps_check(&source).unwrap()).unwrap();

        let mut source = contract(ty.clone());
        source.witnesses.push(WitnessDeclaration {
            source: None,
            name: "provide".into(),
            parameters: vec![],
            result: ty.clone(),
        });
        source.stateful_circuits[0].actions = vec![StateAction::Expression {
            value: Expr::WitnessCall {
                name: "provide".into(),
                arguments: vec![],
            },
        }];
        if ty == Type::OpaqueBytes {
            // Top-level owned bytes also have separately qualified witness results.
            render(&maps_check(&source).unwrap()).unwrap();
        } else {
            // OpaqueString and products with opaque members remain refused.
            assert!(matches!(
                maps_check(&source),
                Err(PlanError::Unsupported(_))
            ));
        }
        let mut helper = contract(ty.clone()).stateful_circuits.remove(0);
        helper.name = "helper".into();
        helper.internal = true;
        helper.result = ty;
        helper.return_value = StateReturn::Expression { value: read() };
        source.stateful_circuits.push(helper);
        source.stateful_circuits[0].actions = vec![StateAction::CircuitCall {
            name: "helper".into(),
            arguments: vec![],
        }];
        // Internal calls use their owned domain independently of the public
        // opaque policy and the narrower witness-result policy above.
        render(&maps_check(&source).unwrap()).unwrap();
    }
}

#[test]
fn canonical_storage_and_scopes_are_not_inferred_from_local_annotations() {
    let mut source = contract(owned());
    let mut forged = owned();
    let Type::Struct { fields, .. } = &mut forged else {
        unreachable!()
    };
    fields[1].name = "wrong".into();
    source.stateful_circuits[0].actions = vec![StateAction::Expression {
        value: Expr::Default { ty: forged },
    }];
    assert!(matches!(
        maps_check(&source),
        Err(PlanError::Unsupported(_))
    ));
    let mut source = contract(owned());
    source.stateful_circuits[0].actions = vec![
        StateAction::If {
            condition: Expr::Boolean { value: true },
            then: Box::new(StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "hidden".into(),
                    ty: owned(),
                    value: read(),
                }],
                action: Box::new(StateAction::Sequence { actions: vec![] }),
            }),
            otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
        },
        StateAction::CellWrite {
            field: "storage".into(),
            index: 0,
            value: Expr::Parameter {
                name: "hidden".into(),
            },
        },
    ];
    assert!(matches!(
        maps_check(&source),
        Err(PlanError::Unsupported(_))
    ));
}

#[test]
fn cell_identity_retains_declared_nested_physical_path() {
    let mut source = contract(owned());
    source.ledger_fields[0].path = vec![1, 0];
    let checked = maps_check(&source).unwrap();
    let Operation::CellRead(slot) = checked.plan().body.bindings[0].operation else {
        unreachable!()
    };
    assert_eq!(
        checked
            .declarations()
            .field(slot.declaration)
            .unwrap()
            .physical_path(),
        vec![1, 0]
    );
    render(&checked).unwrap();
    // This is an independent slot-identity test. The fixture lane qualifies a
    // complete valid nested public ledger layout through native/runtime paths.
}
