// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Independent old owned Type checks and syntax prevent shared-helper drift.
use super::*;
use crate::pure_semantics::{
    jubjub::{Axis, CheckedJubjub, JubjubOperands, check_jubjub},
    sink::Typed,
};
fn operand(ty: Type) -> (syn::Expr, Type) {
    (syn::parse_quote!(probe_once()?), ty)
}
fn tokens(value: Result<(syn::Expr, Type), RenderError>) -> Result<(String, Type), RenderError> {
    value.map(|(output, ty)| (output.to_token_stream().to_string(), ty))
}
fn generic<V>(ty: Type, output: V) -> Typed<V> {
    Typed { output, ty }
}

#[test]
fn legacy_four_operations_and_two_coordinates_match_exact_old_helper_for_wrong_positions() {
    use crate::jubjub_value::{Coordinate as CurrentAxis, Operation as Current};
    use old_jubjub::{Coordinate as OldAxis, Operation as Old};
    let types = [
        Type::Boolean,
        Type::Field,
        Type::JubjubPoint,
        bytes(31),
        Type::Unsigned { max: "bad".into() },
        map(Type::OpaqueString),
    ];
    for left in &types {
        assert_eq!(
            tokens(Current::Reduce(operand(left.clone())).lower()),
            tokens(Old::Reduce(operand(left.clone())).lower())
        );
        assert_eq!(
            tokens(Current::Generator(operand(left.clone())).lower()),
            tokens(Old::Generator(operand(left.clone())).lower())
        );
        for right in &types {
            assert_eq!(
                tokens(Current::Multiply(operand(left.clone()), operand(right.clone())).lower()),
                tokens(Old::Multiply(operand(left.clone()), operand(right.clone())).lower())
            );
            assert_eq!(
                tokens(Current::Add(operand(left.clone()), operand(right.clone())).lower()),
                tokens(Old::Add(operand(left.clone()), operand(right.clone())).lower())
            );
        }
        assert_eq!(
            tokens(CurrentAxis::X.lower(operand(left.clone()))),
            tokens(OldAxis::X.lower(operand(left.clone())))
        );
        assert_eq!(
            tokens(CurrentAxis::Y.lower(operand(left.clone()))),
            tokens(OldAxis::Y.lower(operand(left.clone())))
        );
    }
}

#[test]
fn generic_jubjub_checks_move_actual_types_and_preserve_both_output_positions() {
    let first = Type::Unsigned { max: "bad".into() };
    let second = map(Type::OpaqueString);
    assert_eq!(
        check_jubjub(JubjubOperands::Multiply(
            generic(first.clone(), ()),
            generic(second, ())
        ))
        .err(),
        Some(RenderError::TypeMismatch {
            expected: Type::JubjubPoint,
            actual: first
        })
    );
    assert_eq!(
        check_jubjub(JubjubOperands::Construct(
            generic(Type::Field, ()),
            generic(Type::Boolean, ())
        ))
        .err(),
        Some(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    let checked = check_jubjub(JubjubOperands::Add(
        generic(Type::JubjubPoint, 7),
        generic(Type::JubjubPoint, 11),
    ))
    .unwrap();
    assert_eq!(checked.ty, Type::JubjubPoint);
    assert!(matches!(checked.output, CheckedJubjub::Add(7, 11)));
    let checked = check_jubjub(JubjubOperands::Coordinate(
        Axis::Y,
        generic(Type::JubjubPoint, 13),
    ))
    .unwrap();
    assert_eq!(checked.ty, Type::Field);
    assert!(matches!(
        checked.output,
        CheckedJubjub::Coordinate(Axis::Y, 13)
    ));
    let checked = check_jubjub(JubjubOperands::Negate(generic(Type::JubjubPoint, 17))).unwrap();
    assert!(matches!(checked.output, CheckedJubjub::Negate(17)));
}
