// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn pure_conversion_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    match expr {
        Expr::FieldCast { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            let Type::Unsigned { max } = &actual else {
                return Err(RenderError::ExpectedUnsigned(actual));
            };
            let rendered = match unsigned_maximum(max)? {
                UnsignedMaximum::Small(_) => {
                    syn::parse_quote!(runtime::Field::from((#value).value()))
                }
                UnsignedMaximum::Wide { .. } => syn::parse_quote!((#value).as_field()),
            };
            Ok((rendered, Type::Field))
        }
        Expr::FieldToBytes32 { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            Ok((field_to_bytes_32_syntax(value), Type::Bytes { length: 32 }))
        }
        Expr::Coerce { value, ty } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            Ok((coerce_expression(value, &actual, ty, 0)?, ty.clone()))
        }
        Expr::UnsignedCast { max, value } => {
            unsigned_maximum(max)?;
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            let Type::Unsigned { max: source_max } = actual else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual,
                });
            };
            Ok((
                unsigned_cast_syntax(value, &source_max, max)?,
                Type::Unsigned { max: max.clone() },
            ))
        }
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    }
}
