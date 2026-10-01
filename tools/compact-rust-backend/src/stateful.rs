//! Typed stateful circuit syntax emission.

use proc_macro2::Span;
use std::collections::HashMap;

use crate::ir::{
    ComparisonOperator, CounterAmount, Expr, LedgerField, LedgerFieldKind, PureCircuit,
    StateAction, StateReturn, StatefulCircuit, StructField, Type, WitnessDeclaration,
};
use crate::{RenderError, expression_with_calls, ident, rust_type};

fn render_state_expression(
    value: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    statements: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    query_effect: &mut bool,
) -> Result<(syn::Expr, Type, bool), RenderError> {
    match value {
        Expr::CellRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.index != *index {
                return Err(RenderError::InvalidLedgerIndex(*index));
            }
            let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            let value_ty = rust_type(ty)?;
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            statements
                .push(syn::parse_quote!(let #step = context.read_cell::<#value_ty>(#index)?;));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((syn::parse_quote!(#step.result), ty.clone(), false))
        }
        Expr::SetMember {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.index != *index {
                return Err(RenderError::InvalidLedgerIndex(*index));
            }
            let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            let (item, actual, witness_effect) = render_state_expression(
                value,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if actual != *ty {
                return Err(RenderError::TypeMismatch {
                    expected: ty.clone(),
                    actual,
                });
            }
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            statements
                .push(syn::parse_quote!(let #step = context.member_set(#index, (#item).clone())?;));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((
                syn::parse_quote!(#step.result),
                Type::Boolean,
                witness_effect,
            ))
        }
        Expr::SetIsEmpty { field, index } | Expr::MapIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let (valid_kind, method) = if matches!(value, Expr::SetIsEmpty { .. }) {
                (
                    matches!(declaration.declaration, LedgerFieldKind::Set { .. }),
                    syn::Ident::new("is_empty_set", Span::call_site()),
                )
            } else {
                (
                    matches!(declaration.declaration, LedgerFieldKind::Map { .. }),
                    syn::Ident::new("is_empty_map", Span::call_site()),
                )
            };
            if !valid_kind || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            statements.push(syn::parse_quote!(let #step = context.#method(#index)?;));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((syn::parse_quote!(#step.result), Type::Boolean, false))
        }
        Expr::Call { name, arguments } => {
            let (formal_parameters, result, stateful) =
                if let Some(callee) = stateful_circuits.get(name.as_str()) {
                    if !witnesses.is_empty() || callee.actions.iter().any(action_contains_call) {
                        return Err(RenderError::UnsupportedStatefulCall(name.clone()));
                    }
                    (&callee.parameters, &callee.result, true)
                } else {
                    let callee = circuits
                        .get(name.as_str())
                        .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
                    (&callee.parameters, &callee.result, false)
                };
            if arguments.len() != formal_parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: formal_parameters.len(),
                    actual: arguments.len(),
                });
            }
            let mut rendered_arguments = Vec::<syn::Expr>::with_capacity(arguments.len());
            let mut witness_effect = false;
            for (argument, parameter) in arguments.iter().zip(formal_parameters) {
                let (rendered, actual, effect) = render_state_expression(
                    argument,
                    parameters,
                    witnesses,
                    statements,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    query_effect,
                )?;
                if actual != parameter.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: parameter.ty.clone(),
                        actual,
                    });
                }
                let argument_name = syn::Ident::new(
                    &format!("__compact_call_argument_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                statements.push(syn::parse_quote!(let #argument_name = #rendered;));
                rendered_arguments.push(syn::parse_quote!(#argument_name));
                witness_effect |= effect;
            }
            let name = ident(name)?;
            if stateful {
                let step =
                    syn::Ident::new(&format!("__compact_call_{}", *next_temp), Span::call_site());
                *next_temp += 1;
                statements.push(
                    syn::parse_quote!(let #step = #name(context, #(#rendered_arguments),*)?;),
                );
                statements.push(syn::parse_quote!(context = #step.context;));
                statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
                *query_effect = true;
                Ok((
                    syn::parse_quote!(#step.result),
                    result.clone(),
                    witness_effect,
                ))
            } else {
                Ok((
                    syn::parse_quote!(crate::pure_circuits::#name(#(#rendered_arguments),*)?),
                    result.clone(),
                    witness_effect,
                ))
            }
        }
        Expr::WitnessCall { name, arguments } => {
            let declaration = witnesses
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownWitness(name.clone()))?;
            if arguments.len() != declaration.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: declaration.parameters.len(),
                    actual: arguments.len(),
                });
            }
            let mut rendered_arguments = Vec::<syn::Expr>::with_capacity(arguments.len());
            for (argument, parameter) in arguments.iter().zip(&declaration.parameters) {
                let (rendered, actual, _) = render_state_expression(
                    argument,
                    parameters,
                    witnesses,
                    statements,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    query_effect,
                )?;
                if actual != parameter.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: parameter.ty.clone(),
                        actual,
                    });
                }
                let argument_name = syn::Ident::new(
                    &format!("__compact_argument_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                statements.push(syn::parse_quote!(let #argument_name = #rendered;));
                rendered_arguments.push(syn::parse_quote!(#argument_name));
            }
            let index = *next_temp;
            *next_temp += 1;
            let witness_name = ident(name)?;
            let private_name = syn::Ident::new(
                &format!("__compact_next_private_{index}"),
                Span::call_site(),
            );
            let value_name =
                syn::Ident::new(&format!("__compact_witness_{index}"), Span::call_site());
            statements.push(syn::parse_quote! {
                let (#private_name, #value_name) = witnesses.#witness_name(
                    context.witness_context_with(LedgerView {
                        state: context.query.state.get_ref(),
                    }),
                    #(#rendered_arguments),*
                );
            });
            statements.push(syn::parse_quote!(context.private_state = #private_name;));
            statements.push(syn::parse_quote! {
                private_transcript_outputs.push(runtime::fab::AlignedValue::from(#value_name.clone()));
            });
            Ok((
                syn::parse_quote!(#value_name),
                declaration.result.clone(),
                true,
            ))
        }
        Expr::TransientHash { value: input }
        | Expr::PersistentHash { value: input }
        | Expr::Keccak256 { value: input }
        | Expr::DegradeToTransient { value: input }
        | Expr::UpgradeFromTransient { value: input }
        | Expr::HashToCurve { value: input }
        | Expr::JubjubPointX { value: input }
        | Expr::JubjubPointY { value: input }
        | Expr::EcNeg { value: input }
        | Expr::JubjubScalarFromNative { value: input } => {
            let (rendered, actual, effect) = render_state_expression(
                input,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let (operation, result): (syn::Path, Type) = match value {
                Expr::TransientHash { .. } => {
                    (syn::parse_quote!(runtime::transient_hash), Type::Field)
                }
                Expr::PersistentHash { .. } => (
                    syn::parse_quote!(runtime::persistent_hash),
                    Type::Bytes { length: 32 },
                ),
                Expr::Keccak256 { .. } => (
                    syn::parse_quote!(runtime::keccak256),
                    Type::Bytes { length: 32 },
                ),
                Expr::DegradeToTransient { .. } => {
                    if actual != (Type::Bytes { length: 32 }) {
                        return Err(RenderError::TypeMismatch {
                            expected: Type::Bytes { length: 32 },
                            actual,
                        });
                    }
                    (
                        syn::parse_quote!(runtime::degrade_to_transient),
                        Type::Field,
                    )
                }
                Expr::UpgradeFromTransient { .. } => {
                    if actual != Type::Field {
                        return Err(RenderError::TypeMismatch {
                            expected: Type::Field,
                            actual,
                        });
                    }
                    (
                        syn::parse_quote!(runtime::upgrade_from_transient),
                        Type::Bytes { length: 32 },
                    )
                }
                Expr::HashToCurve { .. } => {
                    (syn::parse_quote!(runtime::hash_to_curve), Type::JubjubPoint)
                }
                Expr::JubjubPointX { .. } | Expr::JubjubPointY { .. } | Expr::EcNeg { .. } => {
                    if actual != Type::JubjubPoint {
                        return Err(RenderError::TypeMismatch {
                            expected: Type::JubjubPoint,
                            actual,
                        });
                    }
                    match value {
                        Expr::JubjubPointX { .. } => {
                            (syn::parse_quote!(runtime::jubjub_point_x), Type::Field)
                        }
                        Expr::JubjubPointY { .. } => {
                            (syn::parse_quote!(runtime::jubjub_point_y), Type::Field)
                        }
                        _ => (syn::parse_quote!(runtime::ec_neg), Type::JubjubPoint),
                    }
                }
                Expr::JubjubScalarFromNative { .. } => {
                    if actual != Type::Field {
                        return Err(RenderError::TypeMismatch {
                            expected: Type::Field,
                            actual,
                        });
                    }
                    (
                        syn::parse_quote!(runtime::jubjub_scalar_from_native),
                        Type::Field,
                    )
                }
                _ => unreachable!(),
            };
            Ok((syn::parse_quote!(#operation(#rendered)), result, effect))
        }
        Expr::TransientCommit {
            value: input,
            opening,
        }
        | Expr::PersistentCommit {
            value: input,
            opening,
        } => {
            let (input, _, input_effect) = render_state_expression(
                input,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let input_name = syn::Ident::new(
                &format!("__compact_value_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #input_name = #input;));
            let (opening, actual, opening_effect) = render_state_expression(
                opening,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let (expected, operation, result): (Type, syn::Path, Type) = match value {
                Expr::TransientCommit { .. } => (
                    Type::Field,
                    syn::parse_quote!(runtime::transient_commit),
                    Type::Field,
                ),
                Expr::PersistentCommit { .. } => (
                    Type::Bytes { length: 32 },
                    syn::parse_quote!(runtime::persistent_commit),
                    Type::Bytes { length: 32 },
                ),
                _ => unreachable!(),
            };
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            Ok((
                syn::parse_quote!(#operation(#input_name, #opening)),
                result,
                input_effect || opening_effect,
            ))
        }
        Expr::EcMulGenerator { scalar } => {
            let (scalar, actual, effect) = render_state_expression(
                scalar,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::ec_mul_generator(#scalar)?),
                Type::JubjubPoint,
                effect,
            ))
        }
        Expr::ConstructJubjubPoint { x: left, y: right }
        | Expr::EcAdd { left, right }
        | Expr::EcMul {
            point: left,
            scalar: right,
        } => {
            let (left, actual, left_effect) = render_state_expression(
                left,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let expected_left = if matches!(value, Expr::ConstructJubjubPoint { .. }) {
                Type::Field
            } else {
                Type::JubjubPoint
            };
            if actual != expected_left {
                return Err(RenderError::TypeMismatch {
                    expected: expected_left,
                    actual,
                });
            }
            let left_name = syn::Ident::new(
                &format!("__compact_value_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #left_name = #left;));
            let (right, actual, right_effect) = render_state_expression(
                right,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let (expected, operation, fallible): (Type, syn::Path, bool) = match value {
                Expr::ConstructJubjubPoint { .. } => (
                    Type::Field,
                    syn::parse_quote!(runtime::construct_jubjub_point),
                    true,
                ),
                Expr::EcAdd { .. } => {
                    (Type::JubjubPoint, syn::parse_quote!(runtime::ec_add), false)
                }
                Expr::EcMul { .. } => (Type::Field, syn::parse_quote!(runtime::ec_mul), true),
                _ => unreachable!(),
            };
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            let rendered = if fallible {
                syn::parse_quote!(#operation(#left_name, #right)?)
            } else {
                syn::parse_quote!(#operation(#left_name, #right))
            };
            Ok((rendered, Type::JubjubPoint, left_effect || right_effect))
        }
        Expr::Tuple { elements } => {
            let mut rendered_elements = Vec::<syn::Expr>::new();
            let mut types = Vec::new();
            let mut effect = false;
            for element in elements {
                let (rendered, ty, element_effect) = render_state_expression(
                    element,
                    parameters,
                    witnesses,
                    statements,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    query_effect,
                )?;
                let element_name = syn::Ident::new(
                    &format!("__compact_element_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                statements.push(syn::parse_quote!(let #element_name = #rendered;));
                rendered_elements.push(syn::parse_quote!(#element_name));
                types.push(ty);
                effect |= element_effect;
            }
            Ok((
                syn::parse_quote!((#(#rendered_elements,)*)),
                Type::Tuple { elements: types },
                effect,
            ))
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let (condition, condition_ty, condition_effect) = render_state_expression(
                condition,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if condition_ty != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: condition_ty,
                });
            }
            let mut then_statements = Vec::new();
            let (then_value, then_ty, then_effect) = render_state_expression(
                then,
                parameters,
                witnesses,
                &mut then_statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let mut else_statements = Vec::new();
            let (else_value, else_ty, else_effect) = render_state_expression(
                otherwise,
                parameters,
                witnesses,
                &mut else_statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if then_ty != else_ty {
                return Err(RenderError::TypeMismatch {
                    expected: then_ty,
                    actual: else_ty,
                });
            }
            Ok((
                syn::parse_quote!(if #condition {
                    #(#then_statements)*
                    #then_value
                } else {
                    #(#else_statements)*
                    #else_value
                }),
                then_ty,
                condition_effect || then_effect || else_effect,
            ))
        }
        Expr::Let { bindings, body } => {
            let mut locals = parameters.clone();
            let mut effect = false;
            for binding in bindings {
                ident(&binding.name)?;
                let (rendered, actual, binding_effect) = render_state_expression(
                    &binding.value,
                    &locals,
                    witnesses,
                    statements,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    query_effect,
                )?;
                if actual != binding.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: binding.ty.clone(),
                        actual,
                    });
                }
                let local_name = syn::Ident::new(
                    &format!("__compact_expression_local_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                let ty = rust_type(&binding.ty)?;
                statements.push(syn::parse_quote!(let #local_name: #ty = #rendered;));
                locals.insert(binding.name.as_str(), (&binding.ty, local_name));
                effect |= binding_effect;
            }
            let (rendered, ty, body_effect) = render_state_expression(
                body,
                &locals,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            Ok((rendered, ty, effect || body_effect))
        }
        Expr::FieldCast { value } => {
            let (value, actual, effect) = render_state_expression(
                value,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if !matches!(actual, Type::Unsigned { .. }) {
                return Err(RenderError::ExpectedUnsigned(actual));
            }
            Ok((
                syn::parse_quote!(runtime::Field::from((#value).value())),
                Type::Field,
                effect,
            ))
        }
        Expr::UnsignedCast { max, value } => {
            let target_max = max
                .parse::<u128>()
                .map_err(|_| RenderError::InvalidUnsignedMaximum(max.clone()))?;
            if target_max.to_string() != *max {
                return Err(RenderError::InvalidUnsignedMaximum(max.clone()));
            }
            let (rendered, actual, effect) = render_state_expression(
                value,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let Type::Unsigned { max: source_max } = actual else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual,
                });
            };
            let parsed_source = source_max
                .parse::<u128>()
                .map_err(|_| RenderError::InvalidUnsignedMaximum(source_max.clone()))?;
            if parsed_source.to_string() != source_max {
                return Err(RenderError::InvalidUnsignedMaximum(source_max));
            }
            let source_max = syn::LitInt::new(&source_max, Span::call_site());
            let target_max = syn::LitInt::new(max, Span::call_site());
            Ok((
                syn::parse_quote!(runtime::cast_unsigned::<#source_max, #target_max>(#rendered)?),
                Type::Unsigned { max: max.clone() },
                effect,
            ))
        }
        Expr::UnsignedAdd { max, left, right }
        | Expr::UnsignedSubtract { max, left, right }
        | Expr::UnsignedMultiply { max, left, right } => {
            let result_max = max
                .parse::<u128>()
                .map_err(|_| RenderError::InvalidUnsignedMaximum(max.clone()))?;
            if result_max.to_string() != *max {
                return Err(RenderError::InvalidUnsignedMaximum(max.clone()));
            }
            let (left, left_ty, left_effect) = render_state_expression(
                left,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let Type::Unsigned { max: left_max } = left_ty else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual: left_ty,
                });
            };
            let left_name = syn::Ident::new(
                &format!("__compact_value_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #left_name = #left;));
            let (right, right_ty, right_effect) = render_state_expression(
                right,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let Type::Unsigned { max: right_max } = right_ty else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual: right_ty,
                });
            };
            let right_name = syn::Ident::new(
                &format!("__compact_value_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #right_name = #right;));
            let left_max = syn::LitInt::new(&left_max, Span::call_site());
            let right_max = syn::LitInt::new(&right_max, Span::call_site());
            let result_max = syn::LitInt::new(max, Span::call_site());
            let operation: syn::Path = match value {
                Expr::UnsignedAdd { .. } => syn::parse_quote!(runtime::add_unsigned),
                Expr::UnsignedSubtract { .. } => syn::parse_quote!(runtime::subtract_unsigned),
                Expr::UnsignedMultiply { .. } => syn::parse_quote!(runtime::multiply_unsigned),
                _ => unreachable!(),
            };
            Ok((
                syn::parse_quote!(#operation::<#left_max, #right_max, #result_max>(#left_name, #right_name)?),
                Type::Unsigned { max: max.clone() },
                left_effect || right_effect,
            ))
        }
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right } => {
            let (left, left_ty, left_effect) = render_state_expression(
                left,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if left_ty != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: left_ty,
                });
            }
            let left_name = syn::Ident::new(
                &format!("__compact_value_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #left_name = #left;));
            let (right, right_ty, right_effect) = render_state_expression(
                right,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if right_ty != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: right_ty,
                });
            }
            let right_name = syn::Ident::new(
                &format!("__compact_value_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #right_name = #right;));
            let rendered = match value {
                Expr::Add { .. } => syn::parse_quote!(#left_name + #right_name),
                Expr::Subtract { .. } => syn::parse_quote!(#left_name - #right_name),
                Expr::Multiply { .. } => syn::parse_quote!(#left_name * #right_name),
                _ => unreachable!(),
            };
            Ok((rendered, Type::Field, left_effect || right_effect))
        }
        Expr::Equal { left, right } | Expr::NotEqual { left, right } => {
            let (left, left_ty, left_effect) = render_state_expression(
                left,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let left_name = syn::Ident::new(
                &format!("__compact_value_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #left_name = #left;));
            let (right, right_ty, right_effect) = render_state_expression(
                right,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if right_ty != left_ty {
                return Err(RenderError::TypeMismatch {
                    expected: left_ty,
                    actual: right_ty,
                });
            }
            let rendered = if matches!(value, Expr::Equal { .. }) {
                syn::parse_quote!(#left_name == #right)
            } else {
                syn::parse_quote!(#left_name != #right)
            };
            Ok((rendered, Type::Boolean, left_effect || right_effect))
        }
        Expr::Compare {
            operator,
            left,
            right,
        } => {
            let (left, left_ty, left_effect) = render_state_expression(
                left,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if !matches!(left_ty, Type::Unsigned { .. }) {
                return Err(RenderError::ExpectedUnsigned(left_ty));
            }
            let left_name = syn::Ident::new(
                &format!("__compact_value_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #left_name = #left;));
            let (right, right_ty, right_effect) = render_state_expression(
                right,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if !matches!(right_ty, Type::Unsigned { .. }) {
                return Err(RenderError::ExpectedUnsigned(right_ty));
            }
            let rendered = match operator {
                ComparisonOperator::Less => syn::parse_quote!(#left_name.value() < #right.value()),
                ComparisonOperator::LessEqual => {
                    syn::parse_quote!(#left_name.value() <= #right.value())
                }
                ComparisonOperator::Greater => {
                    syn::parse_quote!(#left_name.value() > #right.value())
                }
                ComparisonOperator::GreaterEqual => {
                    syn::parse_quote!(#left_name.value() >= #right.value())
                }
            };
            Ok((rendered, Type::Boolean, left_effect || right_effect))
        }
        _ => expression_with_calls(value, parameters, circuits)
            .map(|(rendered, ty)| (rendered, ty, false)),
    }
}

fn action_contains_call(action: &StateAction) -> bool {
    match action {
        StateAction::CircuitCall { .. } => true,
        StateAction::Let { action, .. } => action_contains_call(action),
        _ => false,
    }
}

pub(crate) fn render_stateful_circuit(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<syn::Item, RenderError> {
    let name = ident(&circuit.name)?;
    let mut parameters = HashMap::new();
    let mut args = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        ident(&parameter.name)?;
        let rust_name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if parameters
            .insert(parameter.name.as_str(), (&parameter.ty, rust_name.clone()))
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
        let arg_ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#rust_name: #arg_ty));
    }
    let mut statements = Vec::<syn::Stmt>::new();
    let mut uses_witness = false;
    let mut next_temp = 0;
    let mut next_local = 0;
    for action in &circuit.actions {
        let mut local_parameters = parameters.clone();
        let mut action = action;
        while let StateAction::Let {
            bindings,
            action: inner,
        } = action
        {
            for binding in bindings {
                ident(&binding.name)?;
                let mut binding_statements = Vec::new();
                let mut query_effect = false;
                let (value, actual, effect) = render_state_expression(
                    &binding.value,
                    &local_parameters,
                    witnesses,
                    &mut binding_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                if actual != binding.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: binding.ty.clone(),
                        actual,
                    });
                }
                if effect {
                    uses_witness = true;
                }
                if effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(binding_statements);
                let local_name = syn::Ident::new(
                    &format!("__compact_action_local_{next_local}"),
                    Span::call_site(),
                );
                next_local += 1;
                let ty = rust_type(&binding.ty)?;
                statements.push(syn::parse_quote!(let #local_name: #ty = #value;));
                local_parameters.insert(binding.name.as_str(), (&binding.ty, local_name));
            }
            action = inner;
        }
        let parameters = local_parameters;
        match action {
            StateAction::PureCall { name, arguments } => {
                let call = Expr::Call {
                    name: name.clone(),
                    arguments: arguments.clone(),
                };
                let (rendered, _) = expression_with_calls(&call, &parameters, circuits)?;
                statements.push(syn::parse_quote!(#rendered;));
            }
            StateAction::CircuitCall {
                name: callee_name,
                arguments,
            } => {
                let callee = stateful_circuits
                    .get(callee_name.as_str())
                    .ok_or_else(|| RenderError::UnknownCircuit(callee_name.clone()))?;
                // The first call slice keeps transcript handling local to each
                // generated function. Witness and nested calls need an explicit
                // composition model before they can safely share a context.
                if !witnesses.is_empty()
                    || callee_name == &circuit.name
                    || callee.actions.iter().any(action_contains_call)
                {
                    return Err(RenderError::UnsupportedStatefulCall(callee_name.clone()));
                }
                if arguments.len() != callee.parameters.len() {
                    return Err(RenderError::ArgumentCount {
                        circuit: callee_name.clone(),
                        expected: callee.parameters.len(),
                        actual: arguments.len(),
                    });
                }
                let mut args = Vec::new();
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let (rendered, actual) =
                        expression_with_calls(argument, &parameters, circuits)?;
                    if actual != parameter.ty {
                        return Err(RenderError::TypeMismatch {
                            expected: parameter.ty.clone(),
                            actual,
                        });
                    }
                    args.push(rendered);
                }
                let callee_name = ident(callee_name)?;
                statements.push(syn::parse_quote! {
                    let call_step = #callee_name(context, #(#args),*)?;
                });
                statements.push(syn::parse_quote!(let context = call_step.context;));
                statements.push(syn::parse_quote!(total_cost += call_step.gas_cost;));
            }
            StateAction::Assert { condition, message } => {
                let mut effect_statements = Vec::new();
                let mut query_effect = false;
                let (condition, actual, effect) = render_state_expression(
                    condition,
                    &parameters,
                    witnesses,
                    &mut effect_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                if actual != Type::Boolean {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Boolean,
                        actual,
                    });
                }
                if effect {
                    uses_witness = true;
                }
                if effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(effect_statements);
                statements.push(syn::parse_quote! {
                    if !(#condition) {
                        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                    }
                });
            }
            StateAction::Let { .. } => unreachable!("action Let wrappers were unwrapped"),
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            }
            | StateAction::CounterDecrement {
                field,
                index,
                amount,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = syn::LitInt::new(&declaration.index.to_string(), Span::call_site());
                let amount: syn::Expr = match amount {
                    CounterAmount::Literal { value } => {
                        let value = syn::LitInt::new(&value.to_string(), Span::call_site());
                        syn::parse_quote!(#value)
                    }
                    CounterAmount::Parameter { name } => {
                        let (ty, rust_name) = parameters
                            .get(name.as_str())
                            .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
                        let expected = Type::Unsigned {
                            max: "65535".into(),
                        };
                        if *ty != &expected {
                            return Err(RenderError::TypeMismatch {
                                expected,
                                actual: (*ty).clone(),
                            });
                        }
                        syn::parse_quote!(#rust_name.value() as u16)
                    }
                };
                let method = if matches!(action, StateAction::CounterIncrement { .. }) {
                    syn::Ident::new("increment_counter", Span::call_site())
                } else {
                    syn::Ident::new("decrement_counter", Span::call_site())
                };
                statements.push(syn::parse_quote! {
                    let step = context.#method(#index, #amount)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::CounterReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.write_cell(#index, 0_u64)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::CellWrite {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let mut value_statements = Vec::new();
                let mut query_effect = false;
                let (value, actual, witness_effect) = render_state_expression(
                    value,
                    &parameters,
                    witnesses,
                    &mut value_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                if &actual != ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                if witness_effect {
                    uses_witness = true;
                }
                if witness_effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(value_statements);
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.write_cell(#index, #value)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::ListPushFront {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::List { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (value, actual) = expression_with_calls(value, &parameters, circuits)?;
                if &actual != ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.push_front_list(#index, #value)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::ListPopFront { field, index }
            | StateAction::ListReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                let method = if matches!(action, StateAction::ListPopFront { .. }) {
                    "pop_front_list"
                } else {
                    "reset_list"
                };
                let method = syn::Ident::new(method, Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.#method(#index)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::SetInsert {
                field,
                index,
                value,
            }
            | StateAction::SetRemove {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (value, actual) = expression_with_calls(value, &parameters, circuits)?;
                if &actual != ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                let method = if matches!(action, StateAction::SetInsert { .. }) {
                    syn::Ident::new("insert_set", Span::call_site())
                } else {
                    syn::Ident::new("remove_set", Span::call_site())
                };
                statements.push(syn::parse_quote! {
                    let step = context.#method(#index, #value)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::SetReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.reset_set(#index)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::MapInsert {
                field,
                index,
                key,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Map {
                    key: key_ty,
                    value: value_ty,
                } = &declaration.declaration
                else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (key, actual_key) = expression_with_calls(key, &parameters, circuits)?;
                if &actual_key != key_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let (value, actual_value) = expression_with_calls(value, &parameters, circuits)?;
                if &actual_value != value_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: value_ty.clone(),
                        actual: actual_value,
                    });
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.insert_map(#index, #key, #value)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::MapInsertDefault { field, index, key } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Map {
                    key: key_ty,
                    value: value_ty,
                } = &declaration.declaration
                else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (key, actual_key) = expression_with_calls(key, &parameters, circuits)?;
                if &actual_key != key_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let value_ty = rust_type(value_ty)?;
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.insert_map(#index, #key, <#value_ty as Default>::default())?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::MapRemove { field, index, key } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Map { key: key_ty, .. } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (key, actual_key) = expression_with_calls(key, &parameters, circuits)?;
                if &actual_key != key_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.remove_map(#index, #key)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::MapReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.reset_map(#index)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
        }
    }
    let result_ty = rust_type(&circuit.result)?;
    let return_expr: syn::Expr = match &circuit.return_value {
        StateReturn::Expression { value } => {
            let mut effect_statements = Vec::new();
            let mut query_effect = false;
            let (rendered, actual, effect) = render_state_expression(
                value,
                &parameters,
                witnesses,
                &mut effect_statements,
                &mut next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                &mut query_effect,
            )?;
            if actual != circuit.result {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual,
                });
            }
            if effect {
                uses_witness = true;
            }
            if effect || query_effect {
                statements.push(syn::parse_quote!(let mut context = context;));
            }
            statements.extend(effect_statements);
            rendered
        }
        StateReturn::Unit => {
            if circuit.result != Type::Unit {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual: Type::Unit,
                });
            }
            syn::parse_quote!(())
        }
        StateReturn::CellRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if ty != &circuit.result {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual: ty.clone(),
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.read_cell::<#result_ty>(#index)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::CounterRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.read_cell::<u64>(#index)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger Counter fits Uint<64>"))
        }
        StateReturn::ListLength { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.length_list(#index)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger List length fits Uint<64>"))
        }
        StateReturn::ListIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.is_empty_list(#index)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::ListHead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::List { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = Type::Struct {
                name: "Maybe".into(),
                fields: vec![
                    StructField {
                        name: "is_some".into(),
                        ty: Type::Boolean,
                    },
                    StructField {
                        name: "value".into(),
                        ty: ty.clone(),
                    },
                ],
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let value_ty = rust_type(ty)?;
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.head_list::<#value_ty, #result_ty>(#index)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::SetMember {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let (value, actual) = expression_with_calls(value, &parameters, circuits)?;
            if &actual != ty {
                return Err(RenderError::TypeMismatch {
                    expected: ty.clone(),
                    actual,
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.member_set(#index, #value)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::SetSize { field, index }
        | StateReturn::SetIsEmpty { field, index }
        | StateReturn::MapSize { field, index }
        | StateReturn::MapIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let is_map = matches!(
                &circuit.return_value,
                StateReturn::MapSize { .. } | StateReturn::MapIsEmpty { .. }
            );
            let declaration_matches = if is_map {
                matches!(declaration.declaration, LedgerFieldKind::Map { .. })
            } else {
                matches!(declaration.declaration, LedgerFieldKind::Set { .. })
            };
            if !declaration_matches || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let is_size = matches!(
                &circuit.return_value,
                StateReturn::SetSize { .. } | StateReturn::MapSize { .. }
            );
            let expected = if is_size {
                Type::Unsigned {
                    max: u64::MAX.to_string(),
                }
            } else {
                Type::Boolean
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            let method = match (is_map, is_size) {
                (false, true) => "size_set",
                (false, false) => "is_empty_set",
                (true, true) => "size_map",
                (true, false) => "is_empty_map",
            };
            let method = syn::Ident::new(method, Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.#method(#index)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            if is_size {
                let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
                syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger collection size fits Uint<64>"))
            } else {
                syn::parse_quote!(read_step.result)
            }
        }
        StateReturn::MapMember { field, index, key }
        | StateReturn::MapLookup { field, index, key } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Map {
                key: key_ty,
                value: value_ty,
            } = &declaration.declaration
            else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let (key, actual_key) = expression_with_calls(key, &parameters, circuits)?;
            if &actual_key != key_ty {
                return Err(RenderError::TypeMismatch {
                    expected: key_ty.clone(),
                    actual: actual_key,
                });
            }
            let is_member = matches!(&circuit.return_value, StateReturn::MapMember { .. });
            let expected = if is_member {
                Type::Boolean
            } else {
                value_ty.clone()
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            if is_member {
                statements.push(syn::parse_quote! {
                    let read_step = context.member_map(#index, #key)?;
                });
            } else {
                statements.push(syn::parse_quote! {
                    let read_step = context.lookup_map::<_, #result_ty>(#index, #key)?;
                });
            }
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
    };
    let transcript_init: syn::Stmt = if uses_witness {
        syn::parse_quote!(let mut private_transcript_outputs = Vec::new();)
    } else {
        syn::parse_quote!(let private_transcript_outputs = Vec::new();)
    };
    let cost_init: syn::Stmt = if uses_witness
        && circuit
            .actions
            .iter()
            .all(|action| matches!(action, StateAction::Assert { .. }))
    {
        syn::parse_quote!(let total_cost = runtime::context::RunningCost::default();)
    } else {
        syn::parse_quote!(let mut total_cost = runtime::context::RunningCost::default();)
    };
    let visibility: syn::Visibility = if circuit.internal {
        syn::parse_quote!(pub(crate))
    } else {
        syn::parse_quote!(pub)
    };
    let item: syn::Item = if uses_witness {
        syn::parse_quote! {
            #visibility fn #name<Private, W: Witnesses<Private>>(
                context: runtime::context::CircuitContext<Private>,
                witnesses: &W,
                #(#args),*
            ) -> Result<runtime::context::CircuitResult<Private, #result_ty>, runtime::CompactError> {
                #cost_init
                #transcript_init
                #(#statements)*
                let result = #return_expr;
                Ok(runtime::context::CircuitResult {
                    context,
                    result,
                    gas_cost: total_cost,
                    private_transcript_outputs,
                })
            }
        }
    } else {
        syn::parse_quote! {
            #visibility fn #name<Private>(
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::context::CircuitResult<Private, #result_ty>, runtime::CompactError> {
                #cost_init
                #transcript_init
                #(#statements)*
                let result = #return_expr;
                Ok(runtime::context::CircuitResult {
                    context,
                    result,
                    gas_cost: total_cost,
                    private_transcript_outputs,
                })
            }
        }
    };
    Ok(item)
}
