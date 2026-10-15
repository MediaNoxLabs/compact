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

//! Boolean ledger observations, comparisons, and admitted Boolean helper calls.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::legacy::expressions::scalar_expression;
use crate::recorded::legacy::value_sources::{
    cell_source, closed_boolean_assertion_arm, closed_unsigned_ternary_comparison, static_bindings,
};
use crate::recorded::pure_calls::field_pair_type;
use crate::recorded::{list_head_field, list_length_operand, set_size_operand, uint64_literal};
use crate::{RenderError, ident, list_head_result_type, rust_type};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

#[expect(
    clippy::too_many_arguments,
    reason = "recursive expression lowering threads typed lookups and ordered recording state explicitly"
)]
pub(in crate::recorded) fn boolean_expression(
    value: &Expr,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<syn::Expr>, RenderError> {
    match value {
        Expr::Let { bindings, body } => {
            let Some(scoped) = static_bindings(bindings, locals, parameters, steps, next_temp)?
            else {
                return Ok(None);
            };
            boolean_expression(
                body,
                &scoped,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            )
        }
        Expr::Coerce { value, ty } if *ty == Type::Boolean => boolean_expression(
            value,
            locals,
            parameters,
            ledger_fields,
            witnesses,
            circuits,
            steps,
            next_temp,
            visiting,
        ),
        // The condition may record an existing Boolean observation, but
        // both branches must be effect-free scalar sources. Bind the
        // selected value after that observation and before the Assert.
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let Some(condition) = boolean_expression(
                condition,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            )?
            else {
                return Ok(None);
            };
            let Some(then) = closed_boolean_assertion_arm(then, locals, parameters) else {
                return Ok(None);
            };
            let Some(otherwise) = closed_boolean_assertion_arm(otherwise, locals, parameters)
            else {
                return Ok(None);
            };
            let selected = syn::Ident::new(
                &format!("__compact_recorded_conditional_bool_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let literal = |value: &syn::Expr| match value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Bool(value),
                    ..
                }) => Some(value.value),
                _ => None,
            };
            if let (Some(then_value), Some(otherwise_value)) = (literal(&then), literal(&otherwise))
                && then_value == otherwise_value
            {
                // The condition may have already recorded its effects.
                // Evaluate its remaining scalar expression exactly once.
                steps.push(syn::parse_quote!(let _ = #condition;));
                steps.push(syn::parse_quote!(let #selected: bool = #then_value;));
            } else {
                steps.push(syn::parse_quote! {
                    let #selected: bool = if #condition { #then } else { #otherwise };
                });
            }
            Ok(Some(syn::parse_quote!(#selected)))
        }
        Expr::WitnessCall { name, arguments } => {
            let declaration = witnesses
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownWitness(name.clone()))?;
            if declaration.result != Type::Boolean {
                return Ok(None);
            }
            if arguments.len() != declaration.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: declaration.parameters.len(),
                    actual: arguments.len(),
                });
            }
            let mut args = Vec::new();
            for (argument, parameter) in arguments.iter().zip(&declaration.parameters) {
                let Some(value) = cell_source(argument, &parameter.ty, locals, parameters) else {
                    return Ok(None);
                };
                let ty = rust_type(&parameter.ty)?;
                let arg = syn::Ident::new(
                    &format!("__compact_recorded_arg_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #arg: #ty = #value;));
                args.push(arg);
            }
            let method = ident(name)?;
            let observed = syn::Ident::new(
                &format!("__compact_recorded_bool_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote! {
                let (frame, #observed): (_, bool) = frame.try_witness_metered(|context, meter| {
                    witnesses.#method(
                        context.witness_context_with(super::LedgerView {
                            state: context.query.state.get_ref(),
                            meter,
                        }),
                        #(#args),*
                    )
                })?;
            });
            Ok(Some(syn::parse_quote!(#observed)))
        }
        Expr::CellRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(
                declaration.declaration,
                LedgerFieldKind::Cell { ty: Type::Boolean }
            ) || declaration.index != *index
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            let observed = syn::Ident::new(
                &format!("__compact_recorded_bool_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(
                let (frame, #observed): (_, bool) =
                    crate::ledger_slots::#slot.record_read(frame)?;
            ));
            Ok(Some(syn::parse_quote!(#observed)))
        }
        Expr::SetIsEmpty { field, index }
        | Expr::MapIsEmpty { field, index }
        | Expr::ListIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let matching_kind = matches!(
                (value, &declaration.declaration),
                (Expr::SetIsEmpty { .. }, LedgerFieldKind::Set { .. })
                    | (Expr::MapIsEmpty { .. }, LedgerFieldKind::Map { .. })
                    | (Expr::ListIsEmpty { .. }, LedgerFieldKind::List { .. })
            );
            if !matching_kind || declaration.index != *index {
                return Ok(None);
            }
            let slot = ident(field)?;
            let observed = syn::Ident::new(
                &format!("__compact_recorded_empty_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(
                let (frame, #observed): (_, bool) =
                    crate::ledger_slots::#slot.record_is_empty(frame)?;
            ));
            Ok(Some(syn::parse_quote!(#observed)))
        }
        Expr::StructField {
            value,
            field,
            index: 0,
        } if field == "is_some" => {
            let Expr::ListHead {
                field: list_field,
                index,
                ty,
            } = value.as_ref()
            else {
                return Ok(None);
            };
            let declaration = ledger_fields
                .get(list_field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(list_field.clone()))?;
            let LedgerFieldKind::List { ty: element } = &declaration.declaration else {
                return Ok(None);
            };
            if declaration.index != *index || *ty != list_head_result_type(element, ty) {
                return Ok(None);
            }
            let slot = ident(list_field)?;
            let result_ty = rust_type(ty)?;
            let observed = syn::Ident::new(
                &format!("__compact_recorded_head_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(
                let (frame, #observed): (_, #result_ty) =
                    crate::ledger_slots::#slot.record_head::<#result_ty, _, _>(frame)?;
            ));
            Ok(Some(syn::parse_quote!(#observed.is_some)))
        }
        Expr::Equal { left, right } | Expr::NotEqual { left, right } => {
            if let Some(comparison) = closed_unsigned_ternary_comparison(
                left,
                right,
                locals,
                parameters,
                matches!(value, Expr::Equal { .. }),
            ) {
                return Ok(Some(comparison));
            }
            let boolean_literal = match (&**left, &**right) {
                (Expr::Boolean { value }, other) => Some((other, *value)),
                (other, Expr::Boolean { value }) => Some((other, *value)),
                _ => None,
            };
            if let Some((other, expected)) = boolean_literal {
                let Some(observed) = boolean_expression(
                    other,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?
                else {
                    return Ok(None);
                };
                return if matches!(value, Expr::Equal { .. }) {
                    Ok(Some(syn::parse_quote!(#observed == #expected)))
                } else {
                    Ok(Some(syn::parse_quote!(#observed != #expected)))
                };
            }
            let size_comparison = set_size_operand(left)
                .and_then(|(field, index)| uint64_literal(right).map(|value| (field, index, value)))
                .or_else(|| {
                    set_size_operand(right).and_then(|(field, index)| {
                        uint64_literal(left).map(|value| (field, index, value))
                    })
                });
            if let Some((field, index, expected)) = size_comparison {
                let declaration = ledger_fields
                    .get(field)
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                    || declaration.index != index
                {
                    return Ok(None);
                }
                let slot = ident(&declaration.id)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_size_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(
                    let (frame, #observed): (_, u64) =
                        crate::ledger_slots::#slot.record_size(frame)?;
                ));
                return if matches!(value, Expr::Equal { .. }) {
                    Ok(Some(syn::parse_quote!(#observed == #expected)))
                } else {
                    Ok(Some(syn::parse_quote!(#observed != #expected)))
                };
            }
            let length_comparison = list_length_operand(left)
                .and_then(|(field, index)| uint64_literal(right).map(|value| (field, index, value)))
                .or_else(|| {
                    list_length_operand(right).and_then(|(field, index)| {
                        uint64_literal(left).map(|value| (field, index, value))
                    })
                });
            if let Some((field, index, expected)) = length_comparison {
                let declaration = ledger_fields
                    .get(field)
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                    || declaration.index != index
                {
                    return Ok(None);
                }
                let slot = ident(&declaration.id)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_length_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(
                    let (frame, #observed): (_, u64) =
                        crate::ledger_slots::#slot.record_length(frame)?;
                ));
                return if matches!(value, Expr::Equal { .. }) {
                    Ok(Some(syn::parse_quote!(#observed == #expected)))
                } else {
                    Ok(Some(syn::parse_quote!(#observed != #expected)))
                };
            }
            let head_comparison = list_head_field(left, "value", 1)
                .map(|head| (head, right.as_ref()))
                .or_else(|| list_head_field(right, "value", 1).map(|head| (head, left.as_ref())));
            if let Some(((field, index, ty), expected)) = head_comparison {
                let declaration = ledger_fields
                    .get(field)
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
                let LedgerFieldKind::List { ty: element } = &declaration.declaration else {
                    return Ok(None);
                };
                let comparable_element = match element {
                    Type::Field | Type::Enum { .. } | Type::Bytes { .. } => true,
                    Type::Vector { element, .. } => **element == Type::Field,
                    _ => false,
                };
                if declaration.index != index
                    || !comparable_element
                    || *ty != list_head_result_type(element, ty)
                {
                    return Ok(None);
                }
                let Some(expected) = cell_source(expected, element, locals, parameters) else {
                    return Ok(None);
                };
                let slot = ident(field)?;
                let result_ty = rust_type(ty)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_head_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(
                    let (frame, #observed): (_, #result_ty) =
                        crate::ledger_slots::#slot.record_head::<#result_ty, _, _>(frame)?;
                ));
                return if matches!(value, Expr::Equal { .. }) {
                    Ok(Some(syn::parse_quote!(#observed.value == #expected)))
                } else {
                    Ok(Some(syn::parse_quote!(#observed.value != #expected)))
                };
            }
            let (field, index) = match (&**left, &**right) {
                (Expr::CellRead { field, index }, _) | (_, Expr::CellRead { field, index }) => {
                    (field, index)
                }
                _ => return Ok(None),
            };
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                return Ok(None);
            };
            if declaration.index != *index {
                return Ok(None);
            }
            let mut operand = |value: &Expr| -> Result<Option<syn::Expr>, RenderError> {
                if let Expr::CellRead {
                    field: read_field,
                    index: read_index,
                } = value
                {
                    if read_field != field || read_index != index {
                        return Ok(None);
                    }
                    let slot = ident(field)?;
                    let observed = syn::Ident::new(
                        &format!("__compact_recorded_value_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let value_ty = rust_type(ty)?;
                    steps.push(syn::parse_quote! {
                        let (frame, #observed): (_, #value_ty) =
                            crate::ledger_slots::#slot.record_read(frame)?;
                    });
                    return Ok(Some(syn::parse_quote!(#observed)));
                }
                Ok(cell_source(value, ty, locals, parameters))
            };
            let Some(left) = operand(left)? else {
                return Ok(None);
            };
            let Some(right) = operand(right)? else {
                return Ok(None);
            };
            let compared = syn::Ident::new(
                &format!("__compact_recorded_compare_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            if matches!(value, Expr::Equal { .. }) {
                steps.push(syn::parse_quote!(let #compared = #left == #right;));
            } else {
                steps.push(syn::parse_quote!(let #compared = #left != #right;));
            }
            Ok(Some(syn::parse_quote!(#compared)))
        }
        Expr::Call { name, arguments } => {
            let Some(callee) = circuits.get(name.as_str()) else {
                return Ok(None);
            };
            let StateReturn::Expression { value: result } = &callee.return_value else {
                return Ok(None);
            };
            if callee.result != Type::Boolean || !callee.actions.is_empty() {
                return Ok(None);
            }
            if arguments.len() != callee.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: callee.parameters.len(),
                    actual: arguments.len(),
                });
            }
            // Hash a value at the helper formal type before observing its
            // comparison Cell. Caller names and shorter vector types must
            // never change the helper's persistent encoding.
            let comparison = match result {
                Expr::Equal { left, right } => Some((left.as_ref(), right.as_ref(), true)),
                Expr::NotEqual { left, right } => Some((left.as_ref(), right.as_ref(), false)),
                _ => None,
            };
            if let (Some((left, right, equal)), [parameter], [argument]) = (
                comparison,
                callee.parameters.as_slice(),
                arguments.as_slice(),
            ) && let Some((hashed, hash_ty, hash_function)) = (match left {
                Expr::TransientHash { value } if field_pair_type(&parameter.ty) => {
                    Some((value, Type::Field, ident("transient_hash")?))
                }
                Expr::PersistentHash { value }
                    if callee.internal
                        && equal
                        && (parameter.ty == Type::Field
                            || matches!(&parameter.ty, Type::Vector { element, .. } if **element == Type::Field)) =>
                {
                    Some((value, Type::Bytes { length: 32 }, ident("persistent_hash")?))
                }
                _ => None,
            }) && matches!(hashed.as_ref(), Expr::Parameter { name } if name == &parameter.name)
                && let Expr::CellRead { field, index } = right
            {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration
                    == (LedgerFieldKind::Cell {
                        ty: hash_ty.clone(),
                    })
                    && declaration.index == *index
                {
                    let Some(argument) = cell_source(argument, &parameter.ty, locals, parameters)
                    else {
                        return Ok(None);
                    };
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_hash_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let ty = rust_type(&parameter.ty)?;
                    let hash = syn::Ident::new(
                        &format!("__compact_recorded_hash_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let observed = syn::Ident::new(
                        &format!("__compact_recorded_value_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let slot = ident(field)?;
                    let hash_ty = rust_type(&hash_ty)?;
                    steps.push(syn::parse_quote!(let #arg: #ty = #argument;));
                    steps.push(syn::parse_quote!(
                        let #hash: #hash_ty = runtime::#hash_function(#arg);
                    ));
                    steps.push(syn::parse_quote!(
                        let (frame, #observed): (_, #hash_ty) =
                            crate::ledger_slots::#slot.record_read(frame)?;
                    ));
                    return if equal {
                        Ok(Some(syn::parse_quote!(#hash == #observed)))
                    } else {
                        Ok(Some(syn::parse_quote!(#hash != #observed)))
                    };
                }
            }
            if !visiting.insert(name.clone()) {
                return Err(RenderError::UnsupportedStatefulCall(name.clone()));
            }
            let mut callee_locals = HashMap::new();
            for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                let Some(value) = cell_source(argument, &parameter.ty, locals, parameters) else {
                    visiting.remove(name);
                    return Ok(None);
                };
                let arg = syn::Ident::new(
                    &format!("__compact_recorded_arg_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #arg = #value;));
                callee_locals.insert(parameter.name.clone(), syn::parse_quote!(#arg));
            }
            let value = boolean_expression(
                result,
                &callee_locals,
                &HashMap::new(),
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            );
            visiting.remove(name);
            value
        }
        Expr::SetMember {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                return Ok(None);
            };
            if declaration.index != *index {
                return Ok(None);
            }
            if !matches!(
                ty,
                Type::Field
                    | Type::Boolean
                    | Type::OpaqueString
                    | Type::Enum { .. }
                    | Type::Tuple { .. }
                    | Type::Struct { .. }
                    | Type::Vector { .. }
            ) {
                return Ok(None);
            }
            let key = scalar_expression(
                value,
                ty,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            )?;
            let Some(key) = key else { return Ok(None) };
            let slot = ident(field)?;
            let key_name = syn::Ident::new(
                &format!("__compact_recorded_key_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let observed = syn::Ident::new(
                &format!("__compact_recorded_member_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(let #key_name = #key;));
            steps.push(syn::parse_quote!(
                let (frame, #observed): (_, bool) =
                    crate::ledger_slots::#slot.record_member(frame, #key_name)?;
            ));
            Ok(Some(syn::parse_quote!(#observed)))
        }
        Expr::MapMember { field, index, key } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Map { key: key_ty, .. } = &declaration.declaration else {
                return Ok(None);
            };
            if declaration.index != *index {
                return Ok(None);
            }
            let Some(key) = scalar_expression(
                key,
                key_ty,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            )?
            else {
                return Ok(None);
            };
            let slot = ident(field)?;
            let observed = syn::Ident::new(
                &format!("__compact_recorded_member_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(
                let (frame, #observed): (_, bool) =
                    crate::ledger_slots::#slot.record_member(frame, #key)?;
            ));
            Ok(Some(syn::parse_quote!(#observed)))
        }
        _ => Ok(cell_source(value, &Type::Boolean, locals, parameters)),
    }
}
