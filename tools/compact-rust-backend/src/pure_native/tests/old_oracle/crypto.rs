// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn pure_crypto_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    match expr {
        Expr::TransientHash { value } => {
            let (value, value_type) = expression_with_calls(value, parameters, circuits)?;
            native_representation::check_tuple_traits(
                native_representation::Operation::TransientHash,
                &value_type,
            )?;
            let value = hash_input::syntax(value, &value_type);
            Ok((
                syn::parse_quote!(runtime::transient_hash(#value)),
                Type::Field,
            ))
        }
        Expr::TransientCommit { value, opening } => {
            let (value, value_type) = expression_with_calls(value, parameters, circuits)?;
            let (opening, actual) = expression_with_calls(opening, parameters, circuits)?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            native_representation::check_tuple_traits(
                native_representation::Operation::TransientCommit,
                &value_type,
            )?;
            let value = hash_input::syntax(value, &value_type);
            Ok((
                syn::parse_quote!(runtime::transient_commit(#value, #opening)),
                Type::Field,
            ))
        }
        Expr::PersistentHash { value } => {
            let (value, value_type) = expression_with_calls(value, parameters, circuits)?;
            native_representation::check_tuple_traits(
                native_representation::Operation::PersistentHash,
                &value_type,
            )?;
            Ok((
                syn::parse_quote!(runtime::persistent_hash(#value)),
                Type::Bytes { length: 32 },
            ))
        }
        Expr::Keccak256 { value } => {
            let (value, value_type) = expression_with_calls(value, parameters, circuits)?;
            native_representation::check_tuple_traits(
                native_representation::Operation::Keccak256,
                &value_type,
            )?;
            Ok((
                syn::parse_quote!(runtime::keccak256(#value)),
                Type::Bytes { length: 32 },
            ))
        }
        Expr::PersistentCommit { value, opening } => {
            let (value, value_type) = expression_with_calls(value, parameters, circuits)?;
            let (opening, actual) = expression_with_calls(opening, parameters, circuits)?;
            if actual != (Type::Bytes { length: 32 }) {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Bytes { length: 32 },
                    actual,
                });
            }
            native_representation::check_tuple_traits(
                native_representation::Operation::PersistentCommit,
                &value_type,
            )?;
            Ok((
                syn::parse_quote!(runtime::persistent_commit(#value, #opening)),
                Type::Bytes { length: 32 },
            ))
        }
        Expr::DegradeToTransient { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != (Type::Bytes { length: 32 }) {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Bytes { length: 32 },
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::degrade_to_transient(#value)),
                Type::Field,
            ))
        }
        Expr::UpgradeFromTransient { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::upgrade_from_transient(#value)),
                Type::Bytes { length: 32 },
            ))
        }
        Expr::HashToCurve { value } => {
            let (value, value_type) = expression_with_calls(value, parameters, circuits)?;
            native_representation::check_tuple_traits(
                native_representation::Operation::HashToCurve,
                &value_type,
            )?;
            let value = hash_input::syntax(value, &value_type);
            Ok((
                syn::parse_quote!(runtime::hash_to_curve(#value)),
                Type::JubjubPoint,
            ))
        }
        Expr::JubjubPointX { value } | Expr::JubjubPointY { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != Type::JubjubPoint {
                return Err(RenderError::TypeMismatch {
                    expected: Type::JubjubPoint,
                    actual,
                });
            }
            let operation: syn::Path = match expr {
                Expr::JubjubPointX { .. } => syn::parse_quote!(runtime::jubjub_point_x),
                Expr::JubjubPointY { .. } => syn::parse_quote!(runtime::jubjub_point_y),
                _ => unreachable!(),
            };
            Ok((syn::parse_quote!(#operation(#value)), Type::Field))
        }
        Expr::EcAdd { left, right } => {
            let left = expression_with_calls(left, parameters, circuits)?;
            let right = expression_with_calls(right, parameters, circuits)?;
            jubjub_value::Operation::Add(left, right).lower()
        }
        Expr::ConstructJubjubPoint { x, y } => {
            let (x, x_ty) = expression_with_calls(x, parameters, circuits)?;
            let (y, y_ty) = expression_with_calls(y, parameters, circuits)?;
            for actual in [x_ty, y_ty] {
                if actual != Type::Field {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Field,
                        actual,
                    });
                }
            }
            Ok((
                syn::parse_quote!(runtime::construct_jubjub_point(#x, #y)?),
                Type::JubjubPoint,
            ))
        }
        Expr::EcNeg { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != Type::JubjubPoint {
                return Err(RenderError::TypeMismatch {
                    expected: Type::JubjubPoint,
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::ec_neg(#value)),
                Type::JubjubPoint,
            ))
        }
        Expr::EcMul { point, scalar } => {
            let point = expression_with_calls(point, parameters, circuits)?;
            let scalar = expression_with_calls(scalar, parameters, circuits)?;
            jubjub_value::Operation::Multiply(point, scalar).lower()
        }
        Expr::EcMulGenerator { scalar } => {
            let scalar = expression_with_calls(scalar, parameters, circuits)?;
            jubjub_value::Operation::Generator(scalar).lower()
        }
        Expr::JubjubScalarFromNative { value } => {
            let value = expression_with_calls(value, parameters, circuits)?;
            jubjub_value::Operation::Reduce(value).lower()
        }
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    }
}
