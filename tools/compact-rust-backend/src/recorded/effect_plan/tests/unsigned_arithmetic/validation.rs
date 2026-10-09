// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn validator_rejects_genuinely_wrong_operand_kinds_and_wide_values_not_only_stale_annotations() {
    for op in OPS {
        let original = checked(op.expression(zero("3"), zero("255"), "17"))
            .unwrap()
            .plan()
            .clone();
        independent(original.clone()).unwrap();
        for side in [0, 1] {
            for kind in ["boolean", "field", "wide"] {
                let mut plan = original.clone();
                let position = arithmetic_position(&plan);
                let mut operation = plan.body.bindings[position].operation.clone();
                let (left, right) = operands(&mut operation).unwrap();
                let input = if side == 0 { left } else { right };
                let definition = plan
                    .body
                    .bindings
                    .iter_mut()
                    .find(|b| b.value.id == input.id)
                    .unwrap();
                let (ty, literal) = match kind {
                    "boolean" => (Type::Boolean, Literal::Boolean(false)),
                    "field" => (Type::Field, Literal::Field([0; 32])),
                    "wide" => (unsigned(WIDE), Literal::WideUnsigned([0; 32])),
                    _ => unreachable!(),
                };
                definition.value.ty = ty.clone();
                definition.operation = Operation::Literal(literal);
                input.ty = ty;
                plan.body.bindings[position].operation = operation;
                let gap = independent(plan).unwrap_err();
                assert_eq!(
                    gap.path, original.body.bindings[position].path,
                    "{op:?}/{side}/{kind}: {gap:?}"
                );
            }
        }
    }
}

#[test]
fn validator_checks_result_domain_and_each_operand_identity_before_arithmetic() {
    for op in OPS {
        let original = checked(op.expression(zero("3"), zero("255"), "17"))
            .unwrap()
            .plan()
            .clone();
        let position = arithmetic_position(&original);
        for ty in [Type::Field, Type::Boolean, unsigned(WIDE), unsigned("-1")] {
            let mut plan = original.clone();
            plan.body.bindings[position].value.ty = ty;
            assert!(independent(plan).is_err(), "{op:?}: bad result");
        }
        for side in [0, 1] {
            for mutation in ["missing", "annotation", "forward"] {
                let mut plan = original.clone();
                let (left, right) = operands(&mut plan.body.bindings[position].operation).unwrap();
                let input = if side == 0 { left } else { right };
                match mutation {
                    "missing" => input.id = 999,
                    "annotation" => input.ty = unsigned("7"),
                    "forward" => {
                        let id = input.id;
                        let definition = plan
                            .body
                            .bindings
                            .iter()
                            .position(|b| b.value.id == id)
                            .unwrap();
                        plan.body.bindings.swap(definition, position);
                    }
                    _ => unreachable!(),
                }
                assert!(independent(plan).is_err(), "{op:?}/{side}/{mutation}");
            }
        }
    }
}

fn unit(id: usize) -> Binding {
    Binding {
        value: Value { id, ty: Type::Unit },
        operation: Operation::Literal(Literal::Unit),
        path: format!("unit{id}"),
    }
}
#[test]
fn arithmetic_cannot_capture_a_value_allocated_only_in_a_sibling_branch() {
    for op in OPS {
        let mut plan = checked(op.expression(zero("3"), zero("255"), "17"))
            .unwrap()
            .plan()
            .clone();
        let left = plan.body.bindings.remove(0);
        let condition = Value {
            id: 100,
            ty: Type::Boolean,
        };
        let branch = Binding {
            value: Value {
                id: 101,
                ty: Type::Unit,
            },
            path: "branch".into(),
            operation: Operation::If {
                condition: condition.clone(),
                then: Block {
                    bindings: vec![left, unit(102)],
                    result: unit(102).value,
                },
                otherwise: Block {
                    bindings: vec![unit(103)],
                    result: unit(103).value,
                },
            },
        };
        plan.body.bindings.insert(0, branch);
        plan.body.bindings.insert(
            0,
            Binding {
                value: condition,
                operation: Operation::Literal(Literal::Boolean(true)),
                path: "condition".into(),
            },
        );
        let gap = independent(plan).unwrap_err();
        assert!(gap.detail.contains("out of scope"), "{op:?}: {gap:?}");
    }
}
