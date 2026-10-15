// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent plan validation binds operations to original declarations and SSA scope.
use super::*;

#[test]
fn each_operator_rechecks_operand_ids_annotations_and_boolean_result() {
    for operator in OPERATORS {
        let source = root(operator, validate::uint("255"), validate::uint("65535"));
        let checked = checked(&source);
        let original = checked.plan();
        for side in [0, 1] {
            for mode in ["absent", "self", "annotation"] {
                let mut forged = original.clone();
                let binding = comparison(&mut forged);
                let self_id = binding.value.id;
                let input = operand(binding, side);
                match mode {
                    "absent" => input.id = usize::MAX,
                    "self" => input.id = self_id,
                    "annotation" => input.ty = validate::uint("127"),
                    _ => unreachable!(),
                }
                let gap = validate::validate(&forged, checked.declarations()).unwrap_err();
                assert_eq!(gap.path, "return_value.value", "{side}/{mode}: {gap:?}");
            }
        }
        for ty in [Type::Field, Type::Unit, validate::uint("1")] {
            let mut forged = original.clone();
            comparison(&mut forged).value.ty = ty;
            let gap = validate::validate(&forged, checked.declarations()).unwrap_err();
            assert_eq!(gap.path, "return_value.value");
            assert!(
                gap.detail.contains("operation input/output types"),
                "{gap:?}"
            );
        }
    }
}
#[test]
fn consistent_in_scope_wrong_domains_fail_the_ordering_law_on_either_side() {
    for operator in OPERATORS {
        let source = root(operator, validate::uint("255"), validate::uint("65535"));
        let original = checked(&source).plan().clone();
        for side in [0, 1] {
            for ty in unsupported_types() {
                // Both the declaration and use annotation match: this specifically
                // challenges the operation's bound law, not only SSA annotation checks.
                let mut different_source = source.clone();
                different_source.parameters[side].ty = ty.clone();
                let mut forged = original.clone();
                forged.parameters[side].ty = ty.clone();
                operand(comparison(&mut forged), side).ty = ty;
                let gap = validate::check(forged, authority(&different_source)).unwrap_err();
                assert_eq!(gap.path, "return_value.value");
                assert!(
                    gap.detail.contains("operation input/output types"),
                    "{gap:?}"
                );
            }
        }
    }
}
#[test]
fn original_root_bounds_cannot_be_replaced_with_different_small_carriers() {
    let source = root(
        ComparisonOperator::Less,
        validate::uint("255"),
        validate::uint("65535"),
    );
    let checked = checked(&source);
    for side in [0, 1] {
        let mut forged = checked.plan().clone();
        let substitute = validate::uint("127");
        forged.parameters[side].ty = substitute.clone();
        operand(comparison(&mut forged), side).ty = substitute;
        let gap = validate::validate(&forged, checked.declarations()).unwrap_err();
        assert!(gap.detail.contains("declared signature"), "{gap:?}");
    }
}
#[test]
fn a_comparison_cannot_capture_an_unsigned_value_from_a_finished_branch() {
    let mut source = root(
        ComparisonOperator::Less,
        validate::uint("255"),
        validate::uint("65535"),
    );
    let literal = |value: &str| Expr::UnsignedLiteral {
        value: value.into(),
        max: "255".into(),
    };
    source.return_value = StateReturn::Expression {
        value: compare(
            ComparisonOperator::Less,
            Expr::If {
                condition: Box::new(Expr::Boolean { value: true }),
                then: Box::new(literal("1")),
                otherwise: Box::new(literal("0")),
            },
            reference("right"),
        ),
    };
    let checked = checked(&source);
    let mut forged = checked.plan().clone();
    let inner = forged
        .body
        .bindings
        .iter()
        .find_map(|binding| match &binding.operation {
            Operation::If { then, .. } => Some(then.result.clone()),
            _ => None,
        })
        .unwrap();
    *operand(comparison(&mut forged), 0) = inner;
    let gap = validate::validate(&forged, checked.declarations()).unwrap_err();
    assert_eq!(gap.path, "return_value.value");
    assert!(gap.detail.contains("out of scope"), "{gap:?}");
}
