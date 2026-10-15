// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent validation rejects forged annotations and escaped values.
use super::*;

#[test]
fn every_operation_independently_rechecks_declaration_identity_kind_depth_and_result() {
    for historic in [false, true] {
        for c in candidates(historic) {
            let original = checked(&c).unwrap().plan().clone();
            let i = merkle_binding(&original);
            validate::check(original.clone(), declarations(&c)).unwrap();
            for mode in [
                "missing",
                "index",
                "historic",
                "slot_depth",
                "source_kind",
                "source_depth",
                "invalid_depth",
                "result",
            ] {
                let mut source = c.clone();
                let mut plan = original.clone();
                let Operation::Merkle { slot, .. } = &mut plan.body.bindings[i].operation else {
                    unreachable!()
                };
                match mode {
                    "missing" => slot.declaration = LedgerId(99),
                    "index" => slot.index = 9,
                    "historic" => slot.historic = !historic,
                    "slot_depth" => slot.depth = 4,
                    "source_kind" => source.ledger_fields[0].declaration = LedgerFieldKind::Counter,
                    "source_depth" => {
                        source.ledger_fields[0].declaration =
                            declaration(historic, 4, Type::Boolean)
                    }
                    "invalid_depth" => {
                        slot.depth = 33;
                        source.ledger_fields[0].declaration =
                            declaration(historic, 33, Type::Boolean);
                    }
                    "result" => {
                        plan.body.bindings[i].value.ty = Type::Field;
                        plan.body.result = plan.body.bindings[i].value.clone();
                        plan.result_type = Type::Field;
                    }
                    _ => unreachable!(),
                }
                assert!(
                    validate::check(plan, declarations(&source)).is_err(),
                    "{historic}/{i}/{mode}"
                );
            }
            if !historic {
                let mut forged = original.clone();
                let Operation::Merkle { operation, .. } = &mut forged.body.bindings[i].operation
                else {
                    unreachable!()
                };
                *operation = MerkleOperation::ResetHistory;
                forged.body.bindings[i].value.ty = Type::Unit;
                forged.body.result = forged.body.bindings[i].value.clone();
                forged.result_type = Type::Unit;
                assert!(validate::check(forged, declarations(&c)).is_err());
            }
        }
    }
}

#[test]
fn operand_contracts_reject_missing_stale_and_genuinely_wrong_live_definitions() {
    for historic in [false, true] {
        for c in candidates(historic) {
            let original = checked(&c).unwrap().plan().clone();
            let i = merkle_binding(&original);
            let Operation::Merkle { mut operation, .. } =
                original.body.bindings[i].operation.clone()
            else {
                unreachable!()
            };
            for n in 0..operands(&mut operation).len() {
                for mode in ["missing", "stale", "live"] {
                    let mut plan = original.clone();
                    let mut op = plan.body.bindings[i].operation.clone();
                    let Operation::Merkle { operation, .. } = &mut op else {
                        unreachable!()
                    };
                    let operand = operands(operation).remove(n);
                    if mode == "missing" {
                        operand.id = 999;
                    } else if mode == "stale" {
                        operand.ty = Type::Field;
                    } else {
                        let definition = plan
                            .body
                            .bindings
                            .iter_mut()
                            .find(|b| b.value.id == operand.id)
                            .unwrap();
                        definition.value.ty = Type::Field;
                        definition.operation = Operation::Literal(Literal::Field([0; 32]));
                        operand.ty = Type::Field;
                    }
                    plan.body.bindings[i].operation = op;
                    assert!(
                        validate::check(plan, declarations(&c)).is_err(),
                        "{historic}/{i}/{n}/{mode}"
                    );
                }
            }
            // Valid operands still fail if the declared leaf changes independently.
            if matches!(operation, MerkleOperation::Insert { .. }) {
                let mut source = c.clone();
                source.ledger_fields[0].declaration = declaration(historic, 3, Type::Field);
                assert!(validate::check(original, declarations(&source)).is_err());
            }
        }
    }
}

#[test]
fn indexed_inputs_are_lowered_left_to_right_once_and_cannot_escape_branch_scope() {
    let mut c = contract(false, Type::Boolean);
    c.stateful_circuits[0]
        .actions
        .push(action(false, 1, sample(Type::Boolean), sample(uint64())));
    let original = checked(&c).unwrap().plan().clone();
    let i = merkle_binding(&original);
    let Operation::Merkle {
        operation:
            MerkleOperation::Insert {
                value,
                position: Some(position),
            },
        ..
    } = &original.body.bindings[i].operation
    else {
        unreachable!()
    };
    assert!(value.id < position.id);
    assert!(
        original
            .body
            .bindings
            .iter()
            .find(|b| b.value.id == value.id)
            .unwrap()
            .path
            .ends_with(".value")
    );
    assert!(
        original
            .body
            .bindings
            .iter()
            .find(|b| b.value.id == position.id)
            .unwrap()
            .path
            .ends_with(".position")
    );
    let mut reordered = original;
    reordered.body.bindings.swap(0, i);
    assert!(validate::check(reordered, declarations(&c)).is_err());

    c.stateful_circuits[0].actions.insert(
        0,
        StateAction::If {
            condition: Expr::Boolean { value: true },
            then: Box::new(StateAction::Expression {
                value: sample(Type::Boolean),
            }),
            otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
        },
    );
    let mut forged = checked(&c).unwrap().plan().clone();
    let hidden = forged
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
    let i = merkle_binding(&forged);
    let Operation::Merkle {
        operation: MerkleOperation::Insert { value, .. },
        ..
    } = &mut forged.body.bindings[i].operation
    else {
        unreachable!()
    };
    *value = hidden;
    assert!(validate::check(forged, declarations(&c)).is_err());
}

#[test]
fn root_checks_reject_forged_digest_identity_even_when_definition_and_use_agree() {
    for historic in [false, true] {
        let mut c = contract(historic, Type::Boolean);
        c.stateful_circuits[0].result = Type::Boolean;
        c.stateful_circuits[0].return_value = returned(historic, false, sample(digest()));
        let original = checked(&c).unwrap().plan().clone();
        for ty in [
            Type::Struct {
                name: "OtherDigest".into(),
                fields: vec![StructField {
                    name: "field".into(),
                    ty: Type::Field,
                }],
            },
            Type::Struct {
                name: "MerkleTreeDigest".into(),
                fields: vec![StructField {
                    name: "forged".into(),
                    ty: Type::Field,
                }],
            },
        ] {
            let mut plan = original.clone();
            let i = merkle_binding(&plan);
            let mut operation = plan.body.bindings[i].operation.clone();
            let Operation::Merkle {
                operation: MerkleOperation::CheckRoot { root },
                ..
            } = &mut operation
            else {
                unreachable!()
            };
            plan.body
                .bindings
                .iter_mut()
                .find(|b| b.value.id == root.id)
                .unwrap()
                .value
                .ty = ty.clone();
            root.ty = ty;
            plan.body.bindings[i].operation = operation;
            assert!(validate::check(plan, declarations(&c)).is_err());
        }
    }
}

#[test]
fn every_merkle_operand_rejects_same_typed_values_from_a_sibling_branch() {
    for historic in [false, true] {
        for kind in 0..6 {
            let mut c = contract(historic, Type::Boolean);
            let step = if kind == 5 {
                StateAction::Expression {
                    value: root_check(historic, sample(digest())),
                }
            } else {
                action(
                    historic,
                    kind,
                    sample(if kind == 2 || kind == 3 {
                        Type::Bytes { length: 32 }
                    } else {
                        Type::Boolean
                    }),
                    sample(uint64()),
                )
            };
            c.stateful_circuits[0].actions.push(StateAction::If {
                condition: Expr::Boolean { value: true },
                then: Box::new(step.clone()),
                otherwise: Box::new(step),
            });
            let original = checked(&c).unwrap().plan().clone();
            validate::check(original.clone(), declarations(&c)).unwrap();
            let i = original
                .body
                .bindings
                .iter()
                .position(|b| matches!(b.operation, Operation::If { .. }))
                .unwrap();
            let Operation::If { then, .. } = &original.body.bindings[i].operation else {
                unreachable!()
            };
            let mut then_operation = then
                .bindings
                .iter()
                .find_map(|b| match &b.operation {
                    Operation::Merkle { operation, .. } => Some(operation.clone()),
                    _ => None,
                })
                .unwrap();
            let hidden: Vec<_> = operands(&mut then_operation)
                .into_iter()
                .map(|v| v.clone())
                .collect();
            for (n, hidden) in hidden.into_iter().enumerate() {
                let mut plan = original.clone();
                let Operation::If { otherwise, .. } = &mut plan.body.bindings[i].operation else {
                    unreachable!()
                };
                let sibling = otherwise
                    .bindings
                    .iter_mut()
                    .find_map(|b| match &mut b.operation {
                        Operation::Merkle { operation, .. } => Some(operation),
                        _ => None,
                    })
                    .unwrap();
                *operands(sibling).remove(n) = hidden;
                assert!(
                    validate::check(plan, declarations(&c)).is_err(),
                    "{historic}/{kind}/{n}"
                );
            }
        }
    }
}
