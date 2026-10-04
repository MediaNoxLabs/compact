// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Typed stateful circuit syntax emission.

use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

use crate::ir::{
    ComparisonOperator, CounterAmount, Expr, LedgerField, LedgerFieldKind, NativeWitnessBuiltin,
    PureCircuit, StateAction, StateReturn, StatefulCircuit, StructField, Type, WitnessDeclaration,
};
use crate::{
    RenderError, UnsignedMaximum, coerce_expression, condition_needs_statement, discard_expression,
    expression_with_calls, ident, ledger_path_expr, list_head_result_type, map_slot_types,
    public_parameter_idents, retained_value, rust_type, unsigned_cast_syntax, unsigned_maximum,
};

#[expect(
    clippy::too_many_arguments,
    reason = "state expression lowering threads typed declarations and ordered query effects explicitly"
)]
pub(crate) fn render_state_expression(
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
        Expr::KernelSelf { ty } => {
            let expected = Type::Struct {
                name: "ContractAddress".into(),
                fields: vec![StructField {
                    name: "bytes".into(),
                    ty: Type::Bytes { length: 32 },
                }],
            };
            if *ty != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: ty.clone(),
                });
            }
            let ty_syntax = rust_type(ty)?;
            Ok((
                syn::parse_quote!(#ty_syntax {
                    bytes: runtime::ledger::contract_address_bytes(&context.query.address),
                }),
                ty.clone(),
                false,
            ))
        }
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
            let _value_ty = rust_type(ty)?;
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote!(
                let #step = crate::ledger_slots::#slot.read(context)?;
            ));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((syn::parse_quote!(#step.result), ty.clone(), false))
        }
        Expr::StructField {
            value,
            field,
            index,
        } => {
            let (value, ty, witness_effect) = render_state_expression(
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
            let Type::Struct { fields, .. } = ty else {
                return Err(RenderError::InvalidStructField(field.clone()));
            };
            let declaration = fields
                .get(*index)
                .filter(|declaration| declaration.name == *field)
                .ok_or_else(|| RenderError::InvalidStructField(field.clone()))?;
            let name = ident(field)?;
            let field_value = retained_value(syn::parse_quote!((#value).#name), &declaration.ty);
            Ok((field_value, declaration.ty.clone(), witness_effect))
        }
        Expr::TupleIndex { value, index } => {
            let (value, ty, witness_effect) = render_state_expression(
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
            let Type::Tuple { elements } = ty else {
                return Err(RenderError::ExpectedTuple(ty));
            };
            let result = elements
                .get(*index)
                .ok_or(RenderError::InvalidTupleIndex(*index))?
                .clone();
            let index = syn::Index::from(*index);
            Ok((syn::parse_quote!((#value).#index), result, witness_effect))
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
            let slot = ident(&declaration.id)?;
            let item = retained_value(item, ty);
            statements.push(syn::parse_quote!(
                let #step = crate::ledger_slots::#slot.member(context, #item)?;
            ));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((
                syn::parse_quote!(#step.result),
                Type::Boolean,
                witness_effect,
            ))
        }
        Expr::MapMember { field, index, key } | Expr::MapLookup { field, index, key } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.index != *index {
                return Err(RenderError::InvalidLedgerIndex(*index));
            }
            let LedgerFieldKind::Map {
                key: key_ty,
                value: value_ty,
            } = &declaration.declaration
            else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            let (key, actual, witness_effect) = render_state_expression(
                key,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            if actual != *key_ty {
                return Err(RenderError::TypeMismatch {
                    expected: key_ty.clone(),
                    actual,
                });
            }
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let slot = ident(&declaration.id)?;
            let key = retained_value(key, key_ty);
            let result_ty = if matches!(value, Expr::MapMember { .. }) {
                statements.push(syn::parse_quote!(
                    let #step = crate::ledger_slots::#slot.member(context, #key)?;
                ));
                Type::Boolean
            } else {
                statements.push(syn::parse_quote!(
                    let #step = crate::ledger_slots::#slot.lookup(context, #key)?;
                ));
                value_ty.clone()
            };
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((syn::parse_quote!(#step.result), result_ty, witness_effect))
        }
        Expr::MerkleCheckRoot { field, index, root }
        | Expr::HistoricMerkleCheckRoot { field, index, root } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match declaration.declaration {
                LedgerFieldKind::MerkleTree { .. } => false,
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            if declaration.index != *index
                || historic != matches!(value, Expr::HistoricMerkleCheckRoot { .. })
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let (root, actual, witness_effect) = render_state_expression(
                root,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let expected = Type::Struct {
                name: "MerkleTreeDigest".into(),
                fields: vec![StructField {
                    name: "field".into(),
                    ty: Type::Field,
                }],
            };
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote!(
                let #step = crate::ledger_slots::#slot.check_root(context, #root)?;
            ));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((
                syn::parse_quote!(#step.result),
                Type::Boolean,
                witness_effect,
            ))
        }
        Expr::SetSize { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote!(
                let #step = crate::ledger_slots::#slot.size(context)?;
            ));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((
                syn::parse_quote!(
                    runtime::BoundedUint::<18446744073709551615>::new(#step.result as u128)
                        .expect("ledger Set size fits Uint<64>")
                ),
                Type::Unsigned {
                    max: u64::MAX.to_string(),
                },
                false,
            ))
        }
        Expr::SetIsEmpty { field, index } | Expr::MapIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let (valid_kind, use_slot) = match (value, &declaration.declaration) {
                (Expr::SetIsEmpty { .. }, LedgerFieldKind::Set { .. }) => (true, true),
                (Expr::MapIsEmpty { .. }, LedgerFieldKind::Map { key, value }) => {
                    (true, map_slot_types(key, value)?.is_some())
                }
                _ => (false, false),
            };
            if !valid_kind || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            if use_slot {
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote!(
                    let #step = crate::ledger_slots::#slot.is_empty(context)?;
                ));
            } else {
                let path = ledger_path_expr(declaration);
                statements.push(syn::parse_quote!(
                    let #step = context.is_empty_map(#path)?;
                ));
            }
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((syn::parse_quote!(#step.result), Type::Boolean, false))
        }
        Expr::Call { name, arguments } => {
            let (formal_parameters, result, stateful, callee_uses_witness, callee_emits_native) =
                if let Some(callee) = stateful_circuits.get(name.as_str()) {
                    let effect =
                        circuit_uses_witness(callee, stateful_circuits, &mut HashSet::new())?;
                    let native = circuit_emits_native_private_output(
                        callee,
                        stateful_circuits,
                        &mut HashSet::new(),
                    )?;
                    (&callee.parameters, &callee.result, true, effect, native)
                } else {
                    let callee = circuits
                        .get(name.as_str())
                        .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
                    (&callee.parameters, &callee.result, false, false, false)
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
                if callee_uses_witness {
                    statements.push(syn::parse_quote!(
                        let #step = #name(context, witnesses, #(#rendered_arguments),*)?;
                    ));
                } else {
                    statements.push(syn::parse_quote!(
                        let #step = #name(context, #(#rendered_arguments),*)?;
                    ));
                }
                if callee_uses_witness || callee_emits_native {
                    statements.push(syn::parse_quote!(
                        private_transcript_outputs.extend(#step.private_transcript_outputs);
                    ));
                }
                statements.push(syn::parse_quote!(context = #step.context;));
                statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
                *query_effect = true;
                Ok((
                    syn::parse_quote!(#step.result),
                    result.clone(),
                    witness_effect || callee_uses_witness,
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
            let meter_name = syn::Ident::new(
                &format!("__compact_witness_meter_{index}"),
                Span::call_site(),
            );
            statements.push(syn::parse_quote! {
                let #meter_name = runtime::context::WitnessReadMeter::new(&context);
            });
            statements.push(syn::parse_quote! {
                let (#private_name, #value_name) = witnesses.#witness_name(
                    context.witness_context_with(LedgerView {
                        state: context.query.state.get_ref(),
                        meter: &#meter_name,
                    }),
                    #(#rendered_arguments),*
                )?;
            });
            statements.push(syn::parse_quote!(total_cost += #meter_name.gas_cost();));
            statements.push(syn::parse_quote!(context.private_state = #private_name;));
            let transcript_value =
                retained_value(syn::parse_quote!(#value_name), &declaration.result);
            statements.push(syn::parse_quote! {
                private_transcript_outputs.push(runtime::fab::AlignedValue::from(#transcript_value));
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
        Expr::Sequence { steps, value } => {
            let mut witness_effect = false;
            for step in steps {
                let (rendered, ty, effect) = render_state_expression(
                    step,
                    parameters,
                    witnesses,
                    statements,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    query_effect,
                )?;
                statements.extend(discard_expression(rendered, &ty));
                witness_effect |= effect;
            }
            let (rendered, ty, effect) = render_state_expression(
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
            Ok((rendered, ty, witness_effect || effect))
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let evaluate_condition = condition_needs_statement(condition);
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
            if then == otherwise {
                if evaluate_condition {
                    statements.push(syn::parse_quote!(let _ = #condition;));
                }
                let (value, ty, effect) = render_state_expression(
                    then,
                    parameters,
                    witnesses,
                    statements,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    query_effect,
                )?;
                return Ok((value, ty, condition_effect || effect));
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
        Expr::Coerce { value, ty } => {
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
            Ok((
                coerce_expression(rendered, &actual, ty, 0)?,
                ty.clone(),
                effect,
            ))
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
            let Type::Unsigned { max } = &actual else {
                return Err(RenderError::ExpectedUnsigned(actual));
            };
            let rendered = match unsigned_maximum(max)? {
                UnsignedMaximum::Small(_) => {
                    syn::parse_quote!(runtime::Field::from((#value).value()))
                }
                UnsignedMaximum::Wide { .. } => syn::parse_quote!((#value).as_field()),
            };
            Ok((rendered, Type::Field, effect))
        }
        Expr::UnsignedCast { max, value } => {
            unsigned_maximum(max)?;
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
            Ok((
                unsigned_cast_syntax(rendered, &source_max, max)?,
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

fn action_calls_named(action: &StateAction, name: &str) -> bool {
    match action {
        StateAction::CircuitCall {
            name: callee,
            arguments,
        } => {
            callee == name
                || arguments
                    .iter()
                    .any(|arg| expression_calls_named(arg, name))
        }
        StateAction::PureCall { arguments, .. } => arguments
            .iter()
            .any(|arg| expression_calls_named(arg, name)),
        StateAction::Let { bindings, action } => {
            bindings
                .iter()
                .any(|binding| expression_calls_named(&binding.value, name))
                || action_calls_named(action, name)
        }
        StateAction::Sequence { actions } => actions
            .iter()
            .any(|action| action_calls_named(action, name)),
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            expression_calls_named(condition, name)
                || action_calls_named(then, name)
                || action_calls_named(otherwise, name)
        }
        StateAction::Expression { value }
        | StateAction::CellWrite { value, .. }
        | StateAction::SetInsert { value, .. }
        | StateAction::SetRemove { value, .. }
        | StateAction::ListPushFront { value, .. }
        | StateAction::MerkleInsert { value, .. }
        | StateAction::MerkleInsertHash { hash: value, .. }
        | StateAction::MerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::HistoricMerkleInsert { value, .. }
        | StateAction::HistoricMerkleInsertHash { hash: value, .. }
        | StateAction::HistoricMerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::MapInsertDefault { key: value, .. }
        | StateAction::MapRemove { key: value, .. } => expression_calls_named(value, name),
        StateAction::Assert { condition, .. } => expression_calls_named(condition, name),
        StateAction::MapInsert { key, value, .. } => {
            expression_calls_named(key, name) || expression_calls_named(value, name)
        }
        StateAction::MerkleInsertIndex {
            value, position, ..
        }
        | StateAction::MerkleInsertHashIndex {
            hash: value,
            position,
            ..
        }
        | StateAction::HistoricMerkleInsertIndex {
            value, position, ..
        }
        | StateAction::HistoricMerkleInsertHashIndex {
            hash: value,
            position,
            ..
        } => expression_calls_named(value, name) || expression_calls_named(position, name),
        StateAction::CounterIncrement { .. }
        | StateAction::NativeWitnessCall { .. }
        | StateAction::CounterDecrement { .. }
        | StateAction::CounterReset { .. }
        | StateAction::SetReset { .. }
        | StateAction::ListPopFront { .. }
        | StateAction::ListReset { .. }
        | StateAction::MapReset { .. }
        | StateAction::HistoricMerkleResetHistory { .. }
        | StateAction::HistoricMerkleResetToDefault { .. }
        | StateAction::MerkleResetToDefault { .. } => false,
    }
}

fn expression_calls_named(expression: &Expr, name: &str) -> bool {
    expression_contains(
        expression,
        &|value| matches!(value, Expr::Call { name: callee, .. } if callee == name),
    )
}

fn return_calls_named(value: &StateReturn, name: &str) -> bool {
    match value {
        StateReturn::Expression { value }
        | StateReturn::SetMember { value, .. }
        | StateReturn::HistoricMerkleCheckRoot { root: value, .. }
        | StateReturn::MerkleCheckRoot { root: value, .. } => expression_calls_named(value, name),
        StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => {
            expression_calls_named(key, name)
        }
        _ => false,
    }
}

pub(crate) fn circuit_uses_witness(
    circuit: &StatefulCircuit,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
) -> Result<bool, RenderError> {
    if !visiting.insert(circuit.name.clone()) {
        return Err(RenderError::UnsupportedStatefulCall(circuit.name.clone()));
    }
    let mut effect = circuit_contains_witness(circuit);
    for (name, callee) in circuits {
        if circuit
            .actions
            .iter()
            .any(|action| action_calls_named(action, name))
            || return_calls_named(&circuit.return_value, name)
        {
            effect |= circuit_uses_witness(callee, circuits, visiting)?;
        }
    }
    visiting.remove(&circuit.name);
    Ok(effect)
}

fn action_emits_native_private_output(action: &StateAction) -> bool {
    match action {
        StateAction::NativeWitnessCall { .. } => true,
        StateAction::Sequence { actions } => actions.iter().any(action_emits_native_private_output),
        StateAction::If {
            then, otherwise, ..
        } => {
            action_emits_native_private_output(then)
                || action_emits_native_private_output(otherwise)
        }
        StateAction::Let { action, .. } => action_emits_native_private_output(action),
        _ => false,
    }
}

fn circuit_emits_native_private_output(
    circuit: &StatefulCircuit,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
) -> Result<bool, RenderError> {
    if !visiting.insert(circuit.name.clone()) {
        return Err(RenderError::UnsupportedStatefulCall(circuit.name.clone()));
    }
    let mut effect = circuit
        .actions
        .iter()
        .any(action_emits_native_private_output);
    for (name, callee) in circuits {
        if circuit
            .actions
            .iter()
            .any(|action| action_calls_named(action, name))
            || return_calls_named(&circuit.return_value, name)
        {
            effect |= circuit_emits_native_private_output(callee, circuits, visiting)?;
        }
    }
    visiting.remove(&circuit.name);
    Ok(effect)
}

/// A discoverable public method over the already-rendered circuit function.
/// This layer contains no VM operations or Compact semantics.
pub(crate) fn render_contract_method(
    circuit: &StatefulCircuit,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<syn::ImplItemFn>, RenderError> {
    if circuit.internal {
        return Ok(None);
    }
    let name = ident(&circuit.name)?;
    let uses_witness = circuit_uses_witness(circuit, circuits, &mut HashSet::new())?;
    let mut args = Vec::<syn::FnArg>::new();
    let mut call_args = Vec::<syn::Ident>::new();
    for (parameter, arg) in circuit
        .parameters
        .iter()
        .zip(public_parameter_idents(&circuit.parameters))
    {
        let ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#arg: #ty));
        call_args.push(arg);
    }
    let result_ty = rust_type(&circuit.result)?;
    let method = if uses_witness {
        syn::parse_quote! {
            pub fn #name<Private>(
                &self,
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::context::CircuitResult<Private, #result_ty>, runtime::CompactError>
            where
                W: TryWitnesses<Private>,
            {
                crate::ledger_contract::#name(context, &self.witnesses, #(#call_args),*)
            }
        }
    } else {
        syn::parse_quote! {
            pub fn #name<Private>(
                &self,
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::context::CircuitResult<Private, #result_ty>, runtime::CompactError> {
                crate::ledger_contract::#name(context, #(#call_args),*)
            }
        }
    };
    Ok(Some(method))
}

fn expression_contains(expression: &Expr, predicate: &impl Fn(&Expr) -> bool) -> bool {
    if predicate(expression) {
        return true;
    }
    let visit = |child: &Expr| expression_contains(child, predicate);
    match expression {
        Expr::WitnessCall { arguments, .. } | Expr::Call { arguments, .. } => {
            arguments.iter().any(visit)
        }
        Expr::Tuple { elements }
        | Expr::Vector { elements, .. }
        | Expr::StructLiteral {
            fields: elements, ..
        } => elements.iter().any(visit),
        Expr::VectorMap { source, body, .. } => visit(source) || visit(body),
        Expr::VectorFoldCall {
            initial, source, ..
        } => visit(initial) || visit(source),
        Expr::If {
            condition,
            then,
            otherwise,
        } => visit(condition) || visit(then) || visit(otherwise),
        Expr::Let { bindings, body } => {
            bindings.iter().any(|binding| visit(&binding.value)) || visit(body)
        }
        Expr::Sequence { steps, value } => steps.iter().any(visit) || visit(value),
        Expr::UnsignedCast { value, .. }
        | Expr::FieldCast { value }
        | Expr::Coerce { value, .. }
        | Expr::StructField { value, .. }
        | Expr::TupleIndex { value, .. }
        | Expr::SetMember { value, .. }
        | Expr::MapMember { key: value, .. }
        | Expr::MapLookup { key: value, .. }
        | Expr::MerkleCheckRoot { root: value, .. }
        | Expr::HistoricMerkleCheckRoot { root: value, .. }
        | Expr::Assert {
            condition: value, ..
        }
        | Expr::TransientHash { value }
        | Expr::PersistentHash { value }
        | Expr::Keccak256 { value }
        | Expr::DegradeToTransient { value }
        | Expr::UpgradeFromTransient { value }
        | Expr::HashToCurve { value }
        | Expr::JubjubPointX { value }
        | Expr::JubjubPointY { value }
        | Expr::EcNeg { value }
        | Expr::EcMulGenerator { scalar: value }
        | Expr::JubjubScalarFromNative { value } => visit(value),
        Expr::Equal { left, right }
        | Expr::NotEqual { left, right }
        | Expr::Compare { left, right, .. }
        | Expr::TransientCommit {
            value: left,
            opening: right,
        }
        | Expr::PersistentCommit {
            value: left,
            opening: right,
        }
        | Expr::EcAdd { left, right }
        | Expr::ConstructJubjubPoint { x: left, y: right }
        | Expr::EcMul {
            point: left,
            scalar: right,
        }
        | Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right }
        | Expr::UnsignedAdd { left, right, .. }
        | Expr::UnsignedSubtract { left, right, .. }
        | Expr::UnsignedMultiply { left, right, .. } => visit(left) || visit(right),
        Expr::Unit
        | Expr::Default { .. }
        | Expr::Boolean { .. }
        | Expr::FieldLiteral { .. }
        | Expr::BytesLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::EnumVariant { .. }
        | Expr::Parameter { .. }
        | Expr::CellRead { .. }
        | Expr::KernelSelf { .. }
        | Expr::SetSize { .. }
        | Expr::SetIsEmpty { .. }
        | Expr::MapIsEmpty { .. } => false,
    }
}

fn expression_contains_witness(expression: &Expr) -> bool {
    expression_contains(expression, &|value| {
        matches!(value, Expr::WitnessCall { .. })
    })
}

pub(crate) fn expression_requires_witness(
    expression: &Expr,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<bool, RenderError> {
    if expression_contains_witness(expression) {
        return Ok(true);
    }
    for (name, callee) in stateful_circuits {
        if expression_calls_named(expression, name)
            && circuit_uses_witness(callee, stateful_circuits, &mut HashSet::new())?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn expression_contains_stateful_call(
    expression: &Expr,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    expression_contains(
        expression,
        &|value| matches!(value, Expr::Call { name, .. } if stateful_circuits.contains_key(name.as_str())),
    )
}

fn action_contains_witness(action: &StateAction) -> bool {
    match action {
        StateAction::Sequence { actions } => actions.iter().any(action_contains_witness),
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            expression_contains_witness(condition)
                || action_contains_witness(then)
                || action_contains_witness(otherwise)
        }
        StateAction::Expression { value }
        | StateAction::CellWrite { value, .. }
        | StateAction::SetInsert { value, .. }
        | StateAction::SetRemove { value, .. }
        | StateAction::ListPushFront { value, .. }
        | StateAction::MerkleInsert { value, .. }
        | StateAction::MerkleInsertHash { hash: value, .. }
        | StateAction::MerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::HistoricMerkleInsert { value, .. }
        | StateAction::HistoricMerkleInsertHash { hash: value, .. }
        | StateAction::HistoricMerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::MapInsertDefault { key: value, .. }
        | StateAction::MapRemove { key: value, .. } => expression_contains_witness(value),
        StateAction::PureCall { arguments, .. } | StateAction::CircuitCall { arguments, .. } => {
            arguments.iter().any(expression_contains_witness)
        }
        StateAction::Assert { condition, .. } => expression_contains_witness(condition),
        StateAction::Let { bindings, action } => {
            bindings
                .iter()
                .any(|binding| expression_contains_witness(&binding.value))
                || action_contains_witness(action)
        }
        StateAction::MapInsert { key, value, .. } => {
            expression_contains_witness(key) || expression_contains_witness(value)
        }
        StateAction::MerkleInsertIndex {
            value, position, ..
        }
        | StateAction::MerkleInsertHashIndex {
            hash: value,
            position,
            ..
        }
        | StateAction::HistoricMerkleInsertIndex {
            value, position, ..
        }
        | StateAction::HistoricMerkleInsertHashIndex {
            hash: value,
            position,
            ..
        } => expression_contains_witness(value) || expression_contains_witness(position),
        StateAction::CounterIncrement { .. }
        | StateAction::NativeWitnessCall { .. }
        | StateAction::CounterDecrement { .. }
        | StateAction::CounterReset { .. }
        | StateAction::SetReset { .. }
        | StateAction::ListPopFront { .. }
        | StateAction::ListReset { .. }
        | StateAction::MapReset { .. }
        | StateAction::HistoricMerkleResetHistory { .. }
        | StateAction::HistoricMerkleResetToDefault { .. }
        | StateAction::MerkleResetToDefault { .. } => false,
    }
}

pub(crate) fn circuit_contains_witness(circuit: &StatefulCircuit) -> bool {
    circuit.actions.iter().any(action_contains_witness)
        || match &circuit.return_value {
            StateReturn::Expression { value }
            | StateReturn::SetMember { value, .. }
            | StateReturn::HistoricMerkleCheckRoot { root: value, .. }
            | StateReturn::MerkleCheckRoot { root: value, .. } => {
                expression_contains_witness(value)
            }
            StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => {
                expression_contains_witness(key)
            }
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
    if let Some(item) = crate::native_frame::render_if_supported(
        circuit,
        ledger_fields,
        witnesses,
        stateful_circuits,
    )? {
        return Ok(item);
    }
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
    let mut uses_witness = circuit_uses_witness(circuit, stateful_circuits, &mut HashSet::new())?;
    let has_native_private_output =
        circuit_emits_native_private_output(circuit, stateful_circuits, &mut HashSet::new())?;
    let mut next_temp = 0;
    let mut next_local = 0;
    enum Pending<'a> {
        Action(&'a StateAction),
        RestoreScope,
        EndThen(usize, &'a StateAction),
        EndElse(usize),
    }
    struct BranchFrame<'a> {
        condition: syn::Expr,
        parent_statements: Vec<syn::Stmt>,
        then_statements: Vec<syn::Stmt>,
        parameters: HashMap<&'a str, (&'a Type, syn::Ident)>,
    }
    let mut pending = circuit
        .actions
        .iter()
        .rev()
        .map(Pending::Action)
        .collect::<Vec<_>>();
    let mut scopes = Vec::new();
    let mut branches = Vec::<Option<BranchFrame>>::new();
    let mut local_parameters = parameters.clone();
    while let Some(pending_action) = pending.pop() {
        let action = match pending_action {
            Pending::Action(action) => action,
            Pending::RestoreScope => {
                local_parameters = scopes.pop().expect("scope marker has a matching scope");
                continue;
            }
            Pending::EndThen(index, otherwise) => {
                let frame = branches[index].as_mut().expect("open conditional branch");
                frame.then_statements = std::mem::take(&mut statements);
                local_parameters = frame.parameters.clone();
                pending.push(Pending::EndElse(index));
                pending.push(Pending::Action(otherwise));
                continue;
            }
            Pending::EndElse(index) => {
                let frame = branches[index].take().expect("open conditional branch");
                let else_statements = std::mem::take(&mut statements);
                local_parameters = frame.parameters;
                statements = frame.parent_statements;
                let condition = frame.condition;
                let then_statements = frame.then_statements;
                statements.push(syn::parse_quote! {
                    context = if #condition {
                        #[allow(unused_mut)]
                        let mut context = context;
                        #(#then_statements)*
                        context
                    } else {
                        #[allow(unused_mut)]
                        let mut context = context;
                        #(#else_statements)*
                        context
                    };
                });
                continue;
            }
        };
        match action {
            StateAction::Sequence { actions } => {
                pending.extend(actions.iter().rev().map(Pending::Action));
                continue;
            }
            StateAction::Let {
                bindings,
                action: inner,
            } => {
                scopes.push(local_parameters.clone());
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
                pending.push(Pending::RestoreScope);
                pending.push(Pending::Action(inner));
                continue;
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                let mut condition_statements = Vec::new();
                let mut query_effect = false;
                let (rendered, actual, effect) = render_state_expression(
                    condition,
                    &local_parameters,
                    witnesses,
                    &mut condition_statements,
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
                uses_witness |= effect;
                statements.push(syn::parse_quote!(let mut context = context;));
                statements.extend(condition_statements);
                let index = branches.len();
                branches.push(Some(BranchFrame {
                    condition: rendered,
                    parent_statements: std::mem::take(&mut statements),
                    then_statements: Vec::new(),
                    parameters: local_parameters.clone(),
                }));
                pending.push(Pending::EndThen(index, otherwise));
                pending.push(Pending::Action(then));
                continue;
            }
            _ => {}
        }
        let parameters = local_parameters.clone();
        match action {
            StateAction::NativeWitnessCall {
                builtin: NativeWitnessBuiltin::OwnPublicKey,
            } => {
                statements.push(syn::parse_quote! {
                    private_transcript_outputs.extend([runtime::fab::AlignedValue::from(
                        context.own_coin_public_key()?
                    )]);
                });
            }
            StateAction::Expression { value } => {
                let mut effect_statements = Vec::new();
                let mut query_effect = false;
                let (rendered, ty, effect) = render_state_expression(
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
                if effect {
                    uses_witness = true;
                }
                if effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(effect_statements);
                statements.extend(discard_expression(rendered, &ty));
            }
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
                let callee_uses_witness =
                    circuit_uses_witness(callee, stateful_circuits, &mut HashSet::new())?;
                let callee_emits_native = circuit_emits_native_private_output(
                    callee,
                    stateful_circuits,
                    &mut HashSet::new(),
                )?;
                if arguments.len() != callee.parameters.len() {
                    return Err(RenderError::ArgumentCount {
                        circuit: callee_name.clone(),
                        expected: callee.parameters.len(),
                        actual: arguments.len(),
                    });
                }
                let mut args = Vec::<syn::Expr>::new();
                let mut argument_statements = Vec::new();
                let mut argument_query_effect = false;
                let mut argument_witness_effect = false;
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let (rendered, actual, effect) = render_state_expression(
                        argument,
                        &parameters,
                        witnesses,
                        &mut argument_statements,
                        &mut next_temp,
                        circuits,
                        stateful_circuits,
                        ledger_fields,
                        &mut argument_query_effect,
                    )?;
                    if actual != parameter.ty {
                        return Err(RenderError::TypeMismatch {
                            expected: parameter.ty.clone(),
                            actual,
                        });
                    }
                    let argument_name = syn::Ident::new(
                        &format!("__compact_call_argument_{}", next_temp),
                        Span::call_site(),
                    );
                    next_temp += 1;
                    argument_statements.push(syn::parse_quote!(let #argument_name = #rendered;));
                    args.push(syn::parse_quote!(#argument_name));
                    argument_witness_effect |= effect;
                }
                if argument_query_effect || argument_witness_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(argument_statements);
                uses_witness |= argument_witness_effect;
                let callee_name = ident(callee_name)?;
                if callee_uses_witness {
                    statements.push(syn::parse_quote! {
                        let call_step = #callee_name(context, witnesses, #(#args),*)?;
                    });
                    uses_witness = true;
                } else {
                    statements.push(syn::parse_quote! {
                        let call_step = #callee_name(context, #(#args),*)?;
                    });
                }
                if callee_uses_witness || callee_emits_native {
                    statements.push(syn::parse_quote! {
                        private_transcript_outputs.extend(call_step.private_transcript_outputs);
                    });
                }
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
            StateAction::Let { .. } | StateAction::Sequence { .. } | StateAction::If { .. } => {
                unreachable!("control actions were expanded before rendering")
            }
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
                let slot = ident(&declaration.id)?;
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
                    syn::Ident::new("increment", Span::call_site())
                } else {
                    syn::Ident::new("decrement", Span::call_site())
                };
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.#method(context, #amount)?;
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
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset(context)?;
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
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote!(
                    let step = crate::ledger_slots::#slot.write(context, #value)?;
                ));
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
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.push_front(context, #value)?;
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
                let slot = ident(&declaration.id)?;
                let method = if matches!(action, StateAction::ListPopFront { .. }) {
                    "pop_front"
                } else {
                    "reset"
                };
                let method = syn::Ident::new(method, Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.#method(context)?;
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
                let slot = ident(&declaration.id)?;
                let method = if matches!(action, StateAction::SetInsert { .. }) {
                    syn::Ident::new("insert", Span::call_site())
                } else {
                    syn::Ident::new("remove", Span::call_site())
                };
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.#method(context, #value)?;
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
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset(context)?;
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
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.insert(context, #key, #value)?;
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
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.insert_default(context, #key)?;
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
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.remove(context, #key)?;
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
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset(context)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::HistoricMerkleResetHistory { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(
                    declaration.declaration,
                    LedgerFieldKind::HistoricMerkleTree { .. }
                ) || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset_history(context)?;
                });
                statements.push(syn::parse_quote!(let context = step.context;));
                statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
            }
            StateAction::HistoricMerkleResetToDefault { field, index }
            | StateAction::MerkleResetToDefault { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let historic = match declaration.declaration {
                    LedgerFieldKind::HistoricMerkleTree { .. } => true,
                    LedgerFieldKind::MerkleTree { .. } => false,
                    _ => return Err(RenderError::UnknownLedgerField(field.clone())),
                };
                let action_historic =
                    matches!(action, StateAction::HistoricMerkleResetToDefault { .. });
                if declaration.index != *index || historic != action_historic {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset_to_default(context)?;
                });
                statements.push(syn::parse_quote!(let context = step.context;));
                statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
            }
            StateAction::HistoricMerkleInsert {
                field,
                index,
                value,
            }
            | StateAction::MerkleInsert {
                field,
                index,
                value,
            }
            | StateAction::HistoricMerkleInsertIndex {
                field,
                index,
                value,
                ..
            }
            | StateAction::MerkleInsertIndex {
                field,
                index,
                value,
                ..
            }
            | StateAction::HistoricMerkleInsertHash {
                field,
                index,
                hash: value,
            }
            | StateAction::MerkleInsertHash {
                field,
                index,
                hash: value,
            }
            | StateAction::HistoricMerkleInsertHashIndex {
                field,
                index,
                hash: value,
                ..
            }
            | StateAction::MerkleInsertHashIndex {
                field,
                index,
                hash: value,
                ..
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let (ty, historic) = match &declaration.declaration {
                    LedgerFieldKind::HistoricMerkleTree { ty, .. } => (ty, true),
                    LedgerFieldKind::MerkleTree { ty, .. } => (ty, false),
                    _ => return Err(RenderError::UnknownLedgerField(field.clone())),
                };
                let action_historic = matches!(
                    action,
                    StateAction::HistoricMerkleInsert { .. }
                        | StateAction::HistoricMerkleInsertIndex { .. }
                        | StateAction::HistoricMerkleInsertHash { .. }
                        | StateAction::HistoricMerkleInsertHashIndex { .. }
                );
                if declaration.index != *index || historic != action_historic {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let mut value_statements = Vec::new();
                let mut query_effect = false;
                let (value, actual, value_witness_effect) = render_state_expression(
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
                let is_hash = matches!(
                    action,
                    StateAction::HistoricMerkleInsertHash { .. }
                        | StateAction::HistoricMerkleInsertHashIndex { .. }
                        | StateAction::MerkleInsertHash { .. }
                        | StateAction::MerkleInsertHashIndex { .. }
                );
                let expected = if is_hash {
                    Type::Bytes { length: 32 }
                } else {
                    ty.clone()
                };
                if actual != expected {
                    return Err(RenderError::TypeMismatch { expected, actual });
                }
                let mut witness_effect = value_witness_effect;
                let position = match action {
                    StateAction::HistoricMerkleInsertIndex { position, .. }
                    | StateAction::HistoricMerkleInsertHashIndex { position, .. }
                    | StateAction::MerkleInsertIndex { position, .. }
                    | StateAction::MerkleInsertHashIndex { position, .. } => {
                        let (rendered, actual, effect) = render_state_expression(
                            position,
                            &parameters,
                            witnesses,
                            &mut value_statements,
                            &mut next_temp,
                            circuits,
                            stateful_circuits,
                            ledger_fields,
                            &mut query_effect,
                        )?;
                        let expected = Type::Unsigned {
                            max: u64::MAX.to_string(),
                        };
                        if actual != expected {
                            return Err(RenderError::TypeMismatch { expected, actual });
                        }
                        witness_effect |= effect;
                        Some(rendered)
                    }
                    _ => None,
                };
                uses_witness |= witness_effect;
                if witness_effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(value_statements);
                let slot = ident(&declaration.id)?;
                let method = match (is_hash, position.is_some()) {
                    (false, false) => "insert",
                    (false, true) => "insert_index",
                    (true, false) => "insert_hash",
                    (true, true) => "insert_hash_index",
                };
                let method = syn::Ident::new(method, Span::call_site());
                if let Some(position) = position {
                    statements.push(syn::parse_quote! {
                        let step = crate::ledger_slots::#slot.#method(context, #value, #position)?;
                    });
                } else {
                    statements.push(syn::parse_quote! {
                        let step = crate::ledger_slots::#slot.#method(context, #value)?;
                    });
                }
                statements.push(syn::parse_quote!(let context = step.context;));
                statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
            }
            StateAction::HistoricMerkleInsertIndexDefault {
                field,
                index,
                position,
            }
            | StateAction::MerkleInsertIndexDefault {
                field,
                index,
                position,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let historic = match &declaration.declaration {
                    LedgerFieldKind::HistoricMerkleTree { .. } => true,
                    LedgerFieldKind::MerkleTree { .. } => false,
                    _ => return Err(RenderError::UnknownLedgerField(field.clone())),
                };
                let action_historic =
                    matches!(action, StateAction::HistoricMerkleInsertIndexDefault { .. });
                if declaration.index != *index || historic != action_historic {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let mut value_statements = Vec::new();
                let mut query_effect = false;
                let (position, actual, witness_effect) = render_state_expression(
                    position,
                    &parameters,
                    witnesses,
                    &mut value_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                let expected = Type::Unsigned {
                    max: u64::MAX.to_string(),
                };
                if actual != expected {
                    return Err(RenderError::TypeMismatch { expected, actual });
                }
                if witness_effect {
                    uses_witness = true;
                }
                if witness_effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(value_statements);
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.insert_index_default(context, #position)?;
                });
                statements.push(syn::parse_quote!(let context = step.context;));
                statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
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
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote!(
                let read_step = crate::ledger_slots::#slot.read(context)?;
            ));
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
            let slot = ident(&declaration.id)?;
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.read(context)?;
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
            let slot = ident(&declaration.id)?;
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.length(context)?;
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
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.is_empty(context)?;
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
            let expected = list_head_result_type(ty, &circuit.result);
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.head::<#result_ty, _, _>(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::HistoricMerkleIsFull { field, index }
        | StateReturn::MerkleIsFull { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match declaration.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                LedgerFieldKind::MerkleTree { .. } => false,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            let return_historic = matches!(
                circuit.return_value,
                StateReturn::HistoricMerkleIsFull { .. }
            );
            if declaration.index != *index || historic != return_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.is_full(context)?;
            });
            statements.push(syn::parse_quote!(let context = read_step.context;));
            statements.push(syn::parse_quote!(total_cost += read_step.gas_cost;));
            syn::parse_quote!(read_step.result)
        }
        StateReturn::HistoricMerkleCheckRoot { field, index, root }
        | StateReturn::MerkleCheckRoot { field, index, root } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match declaration.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                LedgerFieldKind::MerkleTree { .. } => false,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            let return_historic = matches!(
                circuit.return_value,
                StateReturn::HistoricMerkleCheckRoot { .. }
            );
            if declaration.index != *index || historic != return_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let expected = Type::Struct {
                name: "MerkleTreeDigest".into(),
                fields: vec![StructField {
                    name: "field".into(),
                    ty: Type::Field,
                }],
            };
            let (root, actual) = expression_with_calls(root, &parameters, circuits)?;
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.check_root(context, #root)?;
            });
            statements.push(syn::parse_quote!(let context = read_step.context;));
            statements.push(syn::parse_quote!(total_cost += read_step.gas_cost;));
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
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.member(context, #value)?;
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
            let slot = ident(&declaration.id)?;
            let method = if is_size { "size" } else { "is_empty" };
            let method = syn::Ident::new(method, Span::call_site());
            statements.push(syn::parse_quote!(
                let read_step = crate::ledger_slots::#slot.#method(context)?;
            ));
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
            let slot = ident(&declaration.id)?;
            if is_member {
                statements.push(syn::parse_quote! {
                    let read_step = crate::ledger_slots::#slot.member(context, #key)?;
                });
            } else {
                statements.push(syn::parse_quote! {
                    let read_step = crate::ledger_slots::#slot.lookup(context, #key)?;
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
    let transcript_init: syn::Stmt = if uses_witness || has_native_private_output {
        syn::parse_quote!(let mut private_transcript_outputs = Vec::new();)
    } else {
        syn::parse_quote!(let private_transcript_outputs = Vec::new();)
    };
    let cost_init: syn::Stmt =
        syn::parse_quote!(let mut total_cost = runtime::context::RunningCost::default(););
    let visibility: syn::Visibility = if circuit.internal {
        syn::parse_quote!(pub(crate))
    } else {
        syn::parse_quote!(pub)
    };
    let item: syn::Item = if uses_witness {
        syn::parse_quote! {
            #visibility fn #name<Private, W: TryWitnesses<Private>>(
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
