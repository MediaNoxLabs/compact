// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn every_set_operation_independently_requires_declared_identity_kind_index_and_result() {
    let c = contract(owned());
    let original = checked(&c).unwrap().plan().clone();
    validate::check(original.clone(), declarations(&c)).unwrap();
    let positions: Vec<_> = original
        .body
        .bindings
        .iter()
        .enumerate()
        .filter_map(|(i, b)| set_slot(&mut b.operation.clone()).map(|_| i))
        .collect();
    assert_eq!(positions.len(), 6);
    for position in positions {
        // Isolate each actual lowered operation so another Set use cannot mask
        // its declaration validation, especially operations without operands.
        let mut isolated = original.clone();
        isolated.body.bindings = vec![
            original.body.bindings[0].clone(),
            original.body.bindings[position].clone(),
        ];
        isolated.body.result = isolated.body.bindings[1].value.clone();
        isolated.result_type = isolated.body.result.ty.clone();
        validate::check(isolated.clone(), declarations(&c)).unwrap();
        for mutation in ["missing", "index", "kind", "result"] {
            let mut plan = isolated.clone();
            let mut source = c.clone();
            let binding = &mut plan.body.bindings[1];
            match mutation {
                "missing" => set_slot(&mut binding.operation).unwrap().declaration = LedgerId(99),
                "index" => set_slot(&mut binding.operation).unwrap().index = 7,
                "kind" => source.ledger_fields[1].declaration = LedgerFieldKind::Counter,
                "result" => binding.value.ty = Type::Field,
                _ => unreachable!(),
            }
            let gap = validate::check(plan, declarations(&source)).unwrap_err();
            if mutation == "kind" {
                assert!(
                    gap.detail.contains("expected Set declaration"),
                    "operation {position}: {gap:?}"
                );
            }
        }
    }
}

#[test]
fn keyed_operations_reject_forged_operand_types_names_and_unavailable_ssa() {
    let c = contract(owned());
    let original = checked(&c).unwrap().plan().clone();
    for position in 0..original.body.bindings.len() {
        if !matches!(
            original.body.bindings[position].operation,
            Operation::SetMember { .. } | Operation::SetInsert { .. } | Operation::SetRemove { .. }
        ) {
            continue;
        }
        for mutation in ["missing", "element", "canonical"] {
            let mut plan = original.clone();
            let (Operation::SetMember { value, .. }
            | Operation::SetInsert { value, .. }
            | Operation::SetRemove { value, .. }) = &mut plan.body.bindings[position].operation
            else {
                unreachable!()
            };
            match mutation {
                "missing" => value.id = 999,
                "element" => value.ty = Type::OpaqueString,
                "canonical" => {
                    let Type::Struct { fields, .. } = &mut value.ty else {
                        unreachable!()
                    };
                    fields[0].name = "forged_member".into();
                }
                _ => unreachable!(),
            }
            assert!(
                validate::check(plan, declarations(&c)).is_err(),
                "operation {position}, {mutation}"
            );
        }
    }
    let mut wrong_payload = c.clone();
    wrong_payload.ledger_fields[1].declaration = LedgerFieldKind::Set {
        ty: Type::OpaqueBytes,
    };
    assert!(validate::check(original.clone(), declarations(&wrong_payload)).is_err());
    let mut before_definition = original.clone();
    before_definition.body.bindings.swap(0, 1);
    assert!(validate::check(before_definition, declarations(&c)).is_err());
}

#[test]
fn set_values_cannot_escape_a_sibling_block() {
    let mut c = contract(Type::Boolean);
    let root = &mut c.stateful_circuits[0];
    root.actions = vec![StateAction::If {
        condition: Expr::Boolean { value: true },
        then: Box::new(StateAction::Expression {
            value: Expr::CellRead {
                field: "key_cell".into(),
                index: 0,
            },
        }),
        otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
    }];
    root.return_value = StateReturn::Expression {
        value: member(Expr::Boolean { value: false }),
    };
    let original = checked(&c).unwrap().plan().clone();
    validate::check(original.clone(), declarations(&c)).unwrap();
    let hidden = original
        .body
        .bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::If { then, .. } => then
                .bindings
                .iter()
                .find(|b| matches!(b.operation, Operation::CellRead(_)))
                .map(|b| b.value.clone()),
            _ => None,
        })
        .unwrap();
    let mut forged = original;
    let Operation::SetMember { value, .. } =
        &mut forged.body.bindings.last_mut().unwrap().operation
    else {
        unreachable!()
    };
    *value = hidden;
    assert!(validate::check(forged, declarations(&c)).is_err());
}

#[test]
fn set_domain_refusals_are_independent_of_operand_and_interface_admission() {
    // Size has no operand: these fail on the declared Set domain itself.
    for ty in [
        Type::Unit,
        Type::Tuple {
            elements: vec![Type::Boolean],
        },
        Type::Vector {
            element: Box::new(Type::Boolean),
            length: 1,
        },
        Type::Unsigned {
            max: "340282366920938463463374607431768211456".into(),
        },
        structure("Empty", vec![]),
    ] {
        let mut c = contract(Type::Boolean);
        c.ledger_fields[1].declaration = LedgerFieldKind::Set { ty };
        c.stateful_circuits[0].actions.clear();
        c.stateful_circuits[0].result = Type::Unsigned {
            max: u64::MAX.to_string(),
        };
        c.stateful_circuits[0].return_value = StateReturn::SetSize {
            field: "seen".into(),
            index: 1,
        };
        assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
    }
    for ty in [Type::OpaqueString, Type::OpaqueBytes, owned()] {
        let mut c = contract(ty.clone());
        c.stateful_circuits[0].parameters.push(Parameter {
            name: "external".into(),
            ty: ty.clone(),
        });
        assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
        c.stateful_circuits[0].parameters.clear();
        c.witnesses.push(WitnessDeclaration {
            name: "external".into(),
            source: None,
            parameters: vec![],
            result: ty,
        });
        c.stateful_circuits[0].actions = vec![StateAction::Expression {
            value: member(Expr::WitnessCall {
                name: "external".into(),
                arguments: vec![],
            }),
        }];
        assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
    }
}
