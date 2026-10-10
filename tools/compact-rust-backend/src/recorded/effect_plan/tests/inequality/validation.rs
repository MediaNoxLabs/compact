// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Forged plans are checked against immutable declarations and actual SSA scope.
use super::*;
#[test]
fn independent_validation_checks_each_operand_identity_and_boolean_result() {
    let source = root(Type::OpaqueBytes, Type::OpaqueBytes);
    let original = checked(&source).plan().clone();
    for side in [0, 1] {
        for mode in ["absent", "annotation", "self"] {
            let mut forged = original.clone();
            let binding = comparison(&mut forged);
            let Operation::NotEqual(a, b) = &mut binding.operation else {
                unreachable!()
            };
            let operand = if side == 0 { a } else { b };
            match mode {
                "absent" => operand.id = 999,
                "annotation" => operand.ty = Type::OpaqueString,
                "self" => operand.id = binding.value.id,
                _ => unreachable!(),
            }
            assert_eq!(
                validate::check(forged, authority(&source))
                    .unwrap_err()
                    .path,
                "actions[0].value"
            );
        }
    }
    for ty in [
        Type::Field,
        Type::Unit,
        Type::OpaqueBytes,
        validate::uint("1"),
    ] {
        let mut forged = original.clone();
        comparison(&mut forged).value.ty = ty;
        let error = validate::check(forged, authority(&source)).unwrap_err();
        assert_eq!(error.path, "actions[0].value");
        assert!(error.detail.contains("operation input/output types"));
    }
    // A real in-scope differently typed operand must fail the comparison law,
    // independently of use-site annotation matching.
    let mut different_source = source.clone();
    different_source.parameters[1].ty = Type::OpaqueString;
    let mut forged = original;
    forged.parameters[1].ty = Type::OpaqueString;
    let Operation::NotEqual(_, right) = &mut comparison(&mut forged).operation else {
        unreachable!()
    };
    right.ty = Type::OpaqueString;
    let gap = validate::check(forged, authority(&different_source)).unwrap_err();
    assert!(gap.detail.contains("operation input/output types"));
}

#[test]
fn independent_tuple_limit_rejects_consistently_forged_arity_thirteen() {
    let source = root(tuple(12), tuple(12));
    let mut forged = checked(&source).plan().clone();
    let wider_source = root(tuple(13), tuple(13));
    for value in &mut forged.parameters {
        value.ty = tuple(13);
    }
    let Operation::NotEqual(a, b) = &mut comparison(&mut forged).operation else {
        unreachable!()
    };
    a.ty = tuple(13);
    b.ty = tuple(13);
    let gap = validate::check(forged, authority(&wider_source)).unwrap_err();
    assert_eq!(gap.path, "actions[0].value");
    assert!(gap.detail.contains("operation input/output types"));
}

#[test]
fn original_canonical_product_and_signature_cannot_be_replaced() {
    let ty = product(Type::OpaqueBytes);
    let source = root(ty.clone(), ty);
    let original = checked(&source).plan().clone();
    let substitute = product(Type::OpaqueString);
    let mut forged = original.clone();
    for value in &mut forged.parameters {
        value.ty = substitute.clone();
    }
    let Operation::NotEqual(a, b) = &mut comparison(&mut forged).operation else {
        unreachable!()
    };
    a.ty = substitute.clone();
    b.ty = substitute;
    assert!(validate::check(forged, authority(&source)).is_err());
    let mut forged = original;
    forged.parameters.pop();
    assert!(
        validate::check(forged, authority(&source))
            .unwrap_err()
            .detail
            .contains("declared signature")
    );
}

#[test]
fn comparison_cannot_escape_branch_scope_or_static_resource_limits() {
    let source = root(Type::Field, Type::Field);
    let original = checked(&source).plan().clone();
    let mut forged = original.clone();
    let local = Value {
        id: 99,
        ty: Type::Field,
    };
    let condition = Value {
        id: 98,
        ty: Type::Boolean,
    };
    let unit = |id| Binding {
        value: Value { id, ty: Type::Unit },
        operation: Operation::Literal(Literal::Unit),
        path: format!("unit{id}"),
    };
    forged.body.bindings.insert(
        0,
        Binding {
            value: condition.clone(),
            operation: Operation::Literal(Literal::Boolean(true)),
            path: "condition".into(),
        },
    );
    forged.body.bindings.insert(
        1,
        Binding {
            value: Value {
                id: 97,
                ty: Type::Unit,
            },
            path: "branch".into(),
            operation: Operation::If {
                condition,
                then: Block {
                    bindings: vec![
                        Binding {
                            value: local.clone(),
                            operation: Operation::Literal(Literal::Field([0; 32])),
                            path: "local".into(),
                        },
                        unit(96),
                    ],
                    result: unit(96).value,
                },
                otherwise: Block {
                    bindings: vec![unit(95)],
                    result: unit(95).value,
                },
            },
        },
    );
    let Operation::NotEqual(left, _) = &mut comparison(&mut forged).operation else {
        unreachable!()
    };
    *left = local;
    assert_eq!(
        validate::check(forged, authority(&source))
            .unwrap_err()
            .path,
        "actions[0].value"
    );
    let oversized = vector(Type::Field, 32769);
    assert!(matches!(
        check(
            &root(oversized.clone(), oversized.clone()),
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(PlanError::Unsupported(_))
    ));
    let mut forged = original;
    for value in &mut forged.parameters {
        value.ty = oversized.clone();
    }
    let gap = validate::check(forged, authority(&source)).unwrap_err();
    assert!(
        gap.detail
            .contains("fixed aggregate layout exceeds 32768 slots"),
        "{gap:?}"
    );
}
