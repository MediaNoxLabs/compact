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

//! Native typed expression lowering with ordered query and witness effects.
//!
//! This component emits the existing expression language. Circuit action/return
//! assembly belongs to the parent; public convenience methods belong to facade.

use crate::circuit_analysis::{circuit_emits_native_private_output, circuit_uses_witness};
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};
use crate::ir::{
    ComparisonOperator, Expr, LedgerField, LedgerFieldKind, NativeWitnessBuiltin, PureCircuit,
    StatefulCircuit, StructField, Type, WitnessDeclaration,
};
use crate::{
    RenderError, UnsignedMaximum, coerce_expression, condition_needs_statement, discard_expression,
    expression_with_calls, field_to_bytes_32_syntax, ident, ledger_path_expr,
    list_head_result_type, map_slot_types, retained_value, rust_type, unsigned_arithmetic_syntax,
    unsigned_cast_syntax, unsigned_maximum,
};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

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
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #step = context.kernel_self()?;));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((
                syn::parse_quote!(#ty_syntax {
                    bytes: runtime::ledger::contract_address_bytes(&#step.result),
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
        Expr::CounterLessThan {
            field,
            index,
            threshold,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let (threshold, actual, _) = render_state_expression(
                threshold,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let expected = Type::Unsigned {
                max: u64::MAX.to_string(),
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
            statements.push(syn::parse_quote!(let #step = crate::ledger_slots::#slot.less_than(context, (#threshold).value() as u64)?;));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((syn::parse_quote!(#step.result), Type::Boolean, false))
        }
        Expr::CounterRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
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
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            Ok((
                syn::parse_quote!(runtime::BoundedUint::<#max>::new(#step.result as u128)
                    .expect("ledger Counter fits Uint<64>")),
                Type::Unsigned {
                    max: u64::MAX.to_string(),
                },
                false,
            ))
        }
        Expr::StructLiteral { ty, fields } => {
            let Type::Struct {
                name,
                fields: declarations,
            } = ty
            else {
                return Err(RenderError::InvalidStructField("<literal>".into()));
            };
            if fields.len() != declarations.len() {
                return Err(RenderError::InvalidStructField(name.clone()));
            }
            let mut members = Vec::<syn::FieldValue>::with_capacity(fields.len());
            let mut witness_effect = false;
            for (value, declaration) in fields.iter().zip(declarations) {
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
                if actual != declaration.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: declaration.ty.clone(),
                        actual,
                    });
                }
                let temporary = syn::Ident::new(
                    &format!("__compact_struct_member_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                let member_ty = rust_type(&declaration.ty)?;
                statements.push(syn::parse_quote!(let #temporary: #member_ty = #rendered;));
                let member = ident(&declaration.name)?;
                members.push(syn::parse_quote!(#member: #temporary));
                witness_effect |= effect;
            }
            let name = ident(name)?;
            Ok((
                syn::parse_quote!(crate::types::#name {#(#members),*}),
                ty.clone(),
                witness_effect,
            ))
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
        Expr::ListLength { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
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
                let #step = crate::ledger_slots::#slot.length(context)?;
            ));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((
                syn::parse_quote!(
                    runtime::BoundedUint::<18446744073709551615>::new(#step.result as u128)
                        .expect("ledger List length fits Uint<64>")
                ),
                Type::Unsigned {
                    max: u64::MAX.to_string(),
                },
                false,
            ))
        }
        Expr::ListIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
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
                let #step = crate::ledger_slots::#slot.is_empty(context)?;
            ));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((syn::parse_quote!(#step.result), Type::Boolean, false))
        }
        Expr::ListHead { field, index, ty } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::List { ty: element } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index || *ty != list_head_result_type(element, ty) {
                return Err(RenderError::TypeMismatch {
                    expected: list_head_result_type(element, ty),
                    actual: ty.clone(),
                });
            }
            let step = syn::Ident::new(
                &format!("__compact_query_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let slot = ident(&declaration.id)?;
            let result_ty = rust_type(ty)?;
            statements.push(syn::parse_quote!(
                let #step = crate::ledger_slots::#slot.head::<#result_ty, _, _>(context)?;
            ));
            statements.push(syn::parse_quote!(context = #step.context;));
            statements.push(syn::parse_quote!(total_cost += #step.gas_cost;));
            *query_effect = true;
            Ok((syn::parse_quote!(#step.result), ty.clone(), false))
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
        Expr::KernelClaim {
            value: argument, ..
        }
        | Expr::KernelMintShielded {
            domain: argument, ..
        } => {
            let (rendered, actual, mut witness_effect) = render_state_expression(
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
            let expected = Type::Bytes { length: 32 };
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            let name = syn::Ident::new(
                &format!("__compact_kernel_arg_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #name = #rendered;));
            let call: syn::Expr = match value {
                Expr::KernelClaim { claim, .. } => {
                    use crate::ir::KernelClaimKind;
                    let (method, carrier) = match claim {
                        KernelClaimKind::Nullifier => {
                            ("kernel_claim_zswap_nullifier", "CoinNullifier")
                        }
                        KernelClaimKind::CoinSpend => {
                            ("kernel_claim_zswap_coin_spend", "CoinCommitment")
                        }
                        KernelClaimKind::CoinReceive => {
                            ("kernel_claim_zswap_coin_receive", "CoinCommitment")
                        }
                    };
                    let method = ident(method)?;
                    let carrier = ident(carrier)?;
                    syn::parse_quote!(context.#method(runtime::ledger::#carrier(runtime::ledger::HashOutput(#name.into_array())))?)
                }
                Expr::KernelMintShielded { amount, .. } => {
                    let (rendered, actual, effect) = render_state_expression(
                        amount,
                        parameters,
                        witnesses,
                        statements,
                        next_temp,
                        circuits,
                        stateful_circuits,
                        ledger_fields,
                        query_effect,
                    )?;
                    let expected = Type::Unsigned {
                        max: u64::MAX.to_string(),
                    };
                    if actual != expected {
                        return Err(RenderError::TypeMismatch { expected, actual });
                    }
                    witness_effect |= effect;
                    syn::parse_quote!(context.kernel_mint_shielded(runtime::ledger::HashOutput(#name.into_array()),(#rendered).value() as u64)?)
                }
                _ => unreachable!(),
            };
            statements.push(syn::parse_quote!(let step = #call;));
            statements.push(syn::parse_quote!(context = step.context;));
            statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
            *query_effect = true;
            Ok((syn::parse_quote!(()), Type::Unit, witness_effect))
        }
        Expr::CreateZswapInput { coin } | Expr::CreateZswapOutput { coin, .. } => {
            let is_input = matches!(value, Expr::CreateZswapInput { .. });
            let (rendered, actual, mut witness_effect) = render_state_expression(
                coin,
                parameters,
                witnesses,
                statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                query_effect,
            )?;
            let expected = if is_input {
                qualified_coin_type()
            } else {
                shielded_coin_type()
            };
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            let coin_name = syn::Ident::new(
                &format!("__compact_zswap_coin_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote!(let #coin_name = #rendered;));
            let info: syn::Expr = syn::parse_quote!(runtime::ledger::coin_info_from_compact(#coin_name.nonce,#coin_name.color,#coin_name.value.value()));
            if let Expr::CreateZswapOutput { recipient, .. } = value {
                let (rendered, actual, effect) = render_state_expression(
                    recipient,
                    parameters,
                    witnesses,
                    statements,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    query_effect,
                )?;
                let expected = shielded_recipient_type();
                if actual != expected {
                    return Err(RenderError::TypeMismatch { expected, actual });
                }
                witness_effect |= effect;
                let recipient_name = syn::Ident::new(
                    &format!("__compact_zswap_recipient_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                statements.push(syn::parse_quote!(let #recipient_name = #rendered;));
                statements.push(syn::parse_quote!(context.create_zswap_output(#info, runtime::ledger::coin_recipient_from_compact(#recipient_name.is_left,#recipient_name.left.bytes,#recipient_name.right.bytes))?;));
            } else {
                statements.push(syn::parse_quote!(context.create_zswap_input((#info).qualify(#coin_name.mt_index.value() as u64));));
            }
            statements.push(syn::parse_quote!(private_transcript_outputs.push(runtime::fab::AlignedValue::from(()));));
            *query_effect = true;
            Ok((syn::parse_quote!(()), Type::Unit, witness_effect))
        }
        Expr::NativeWitnessCall {
            builtin: NativeWitnessBuiltin::OwnPublicKey,
        } => {
            let value_name = syn::Ident::new(
                &format!("__compact_native_key_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            statements.push(syn::parse_quote! {
                let #value_name = context.own_coin_public_key()?;
            });
            statements.push(syn::parse_quote! {
                private_transcript_outputs.extend([runtime::fab::AlignedValue::from(#value_name)]);
            });
            Ok((
                syn::parse_quote!(crate::types::ZswapCoinPublicKey { bytes: runtime::FixedBytes::new(#value_name) }),
                NativeWitnessBuiltin::OwnPublicKey.result_type(),
                false,
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
        Expr::Assert { condition, message } => {
            let (condition, actual, effect) = render_state_expression(
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
            if actual != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual,
                });
            }
            Ok((
                syn::parse_quote!({
                    if !(#condition) {
                        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                    }
                }),
                Type::Unit,
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
            // A block already evaluates to Unit without an explicit `()` tail.
            // Remove only that syntax; effectful Unit-valued expressions remain.
            let then_value = (!matches!(&then_value, syn::Expr::Tuple(t) if t.elems.is_empty()))
                .then_some(then_value);
            let else_value = (!matches!(&else_value, syn::Expr::Tuple(t) if t.elems.is_empty()))
                .then_some(else_value);
            let otherwise = (!else_statements.is_empty() || else_value.is_some())
                .then(|| quote::quote!(else { #(#else_statements)* #else_value }));
            Ok((
                syn::parse_quote!(if #condition {
                    #(#then_statements)*
                    #then_value
                } #otherwise),
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
        Expr::FieldToBytes32 { value } => {
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
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            Ok((
                field_to_bytes_32_syntax(rendered),
                Type::Bytes { length: 32 },
                effect,
            ))
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
            Ok((
                unsigned_arithmetic_syntax(
                    value,
                    syn::parse_quote!(#left_name),
                    syn::parse_quote!(#right_name),
                    &left_max,
                    &right_max,
                    max,
                )?,
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
            let Type::Unsigned { max } = &left_ty else {
                return Err(RenderError::ExpectedUnsigned(left_ty));
            };
            if matches!(unsigned_maximum(max)?, UnsignedMaximum::Wide { .. }) {
                return Err(RenderError::InvalidUnsignedMaximum(max.clone()));
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
            let Type::Unsigned { max } = &right_ty else {
                return Err(RenderError::ExpectedUnsigned(right_ty));
            };
            if matches!(unsigned_maximum(max)?, UnsignedMaximum::Wide { .. }) {
                return Err(RenderError::InvalidUnsignedMaximum(max.clone()));
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
