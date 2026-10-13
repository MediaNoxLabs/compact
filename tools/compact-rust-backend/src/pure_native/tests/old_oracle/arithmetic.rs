// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn pure_arithmetic_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    match expr {
        Expr::UnsignedAdd { max, left, right }
        | Expr::UnsignedSubtract { max, left, right }
        | Expr::UnsignedMultiply { max, left, right } => {
            let (left, left_type) = expression_with_calls(left, parameters, circuits)?;
            let (right, right_type) = expression_with_calls(right, parameters, circuits)?;
            let Type::Unsigned { max: left_max } = left_type else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual: left_type,
                });
            };
            let Type::Unsigned { max: right_max } = right_type else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual: right_type,
                });
            };
            Ok((
                unsigned_arithmetic_syntax(expr, left, right, &left_max, &right_max, max)?,
                Type::Unsigned { max: max.clone() },
            ))
        }
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right } => {
            let (left, left_type) = expression_with_calls(left, parameters, circuits)?;
            let (right, right_type) = expression_with_calls(right, parameters, circuits)?;
            if left_type != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: left_type,
                });
            }
            if right_type != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: right_type,
                });
            }
            let op = match expr {
                Expr::Add { .. } => syn::BinOp::Add(syn::token::Plus::default()),
                Expr::Subtract { .. } => syn::BinOp::Sub(syn::token::Minus::default()),
                Expr::Multiply { .. } => syn::BinOp::Mul(syn::token::Star::default()),
                _ => unreachable!(),
            };
            Ok((
                syn::Expr::Binary(syn::ExprBinary {
                    attrs: vec![],
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                }),
                Type::Field,
            ))
        }
        Expr::Equal { left, right } | Expr::NotEqual { left, right } => {
            let (left, left_ty) = expression_with_calls(left, parameters, circuits)?;
            let (right, right_ty) = expression_with_calls(right, parameters, circuits)?;
            if right_ty != left_ty {
                return Err(RenderError::TypeMismatch {
                    expected: left_ty,
                    actual: right_ty,
                });
            }
            let operation = if matches!(expr, Expr::Equal { .. }) {
                native_representation::Operation::Equal
            } else {
                native_representation::Operation::NotEqual
            };
            native_representation::check_tuple_traits(operation, &left_ty)?;
            let rendered = if matches!(expr, Expr::Equal { .. }) {
                syn::parse_quote!(#left == #right)
            } else {
                syn::parse_quote!(#left != #right)
            };
            Ok((rendered, Type::Boolean))
        }
        Expr::Compare {
            operator,
            left,
            right,
        } => {
            let (left, left_ty) = expression_with_calls(left, parameters, circuits)?;
            let (right, right_ty) = expression_with_calls(right, parameters, circuits)?;
            for actual in [left_ty, right_ty] {
                let Type::Unsigned { max } = &actual else {
                    return Err(RenderError::ExpectedUnsigned(actual));
                };
                if matches!(unsigned_maximum(max)?, UnsignedMaximum::Wide { .. }) {
                    return Err(RenderError::UnsupportedWideUnsignedOperation {
                        operation: "ordered comparison",
                        max: max.clone(),
                    });
                }
            }
            let rendered = match operator {
                ComparisonOperator::Less => syn::parse_quote!(#left.value() < #right.value()),
                ComparisonOperator::LessEqual => syn::parse_quote!(#left.value() <= #right.value()),
                ComparisonOperator::Greater => syn::parse_quote!(#left.value() > #right.value()),
                ComparisonOperator::GreaterEqual => {
                    syn::parse_quote!(#left.value() >= #right.value())
                }
            };
            Ok((rendered, Type::Boolean))
        }
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    }
}
