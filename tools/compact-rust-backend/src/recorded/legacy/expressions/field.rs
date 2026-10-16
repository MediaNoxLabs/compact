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

//! Field values with ordered recording effects and recursive helper calls.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::LegacyActionPrecision;
use crate::recorded::legacy::actions::append_steps;
use crate::recorded::legacy::calls::shared_field_call;
use crate::recorded::legacy::expressions::{boolean_expression, scalar_expression};
use crate::recorded::legacy::value_sources::{amount_source, cell_source, static_bindings};
use crate::{RenderError, expression_with_calls, ident};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

/// Lower a Field expression together with its ordered recording effects.
/// The returned syntax refers only to values already evaluated in `steps`.
#[expect(
    clippy::too_many_arguments,
    reason = "recursive expression lowering threads typed lookups and ordered recording state explicitly"
)]
pub(in crate::recorded) fn field_expression(
    value: &Expr,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
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
            field_expression(
                body,
                &scoped,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )
        }
        Expr::Coerce { value, ty } if *ty == Type::Field => field_expression(
            value,
            locals,
            parameters,
            ledger_fields,
            witnesses,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
        ),
        Expr::FieldCast { value } => {
            // A bounded unsigned public parameter has no recording
            // effects. Retain its full u128 value before converting to
            // the ledger Field; this is the same typed cast as native
            // stateful lowering, including values above u64::MAX.
            if let Expr::Parameter { name } = value.as_ref()
                && !locals.contains_key(name)
                && let Some((ty @ Type::Unsigned { .. }, _)) = parameters.get(name.as_str())
                && let Some(unsigned) = cell_source(value, ty, locals, parameters)
            {
                let cast = syn::Ident::new(
                    &format!("__compact_recorded_field_cast_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote! {
                    let #cast: runtime::Field = runtime::Field::from((#unsigned).value());
                });
                return Ok(Some(syn::parse_quote!(#cast)));
            }

            fn bounded_arm(value: &Expr) -> Option<syn::Expr> {
                let Expr::Coerce {
                    value,
                    ty: Type::Unsigned { max },
                } = value
                else {
                    return None;
                };
                let Expr::UnsignedLiteral {
                    value: literal,
                    max: literal_max,
                } = value.as_ref()
                else {
                    return None;
                };
                if max != "2" || literal_max != max {
                    return None;
                }
                let literal = literal.parse::<u64>().ok()?;
                if literal > 2 {
                    return None;
                }
                let literal = syn::LitInt::new(&format!("{literal}u64"), Span::call_site());
                Some(syn::parse_quote!(runtime::Field::from(#literal)))
            }

            let Expr::If {
                condition,
                then,
                otherwise,
            } = value.as_ref()
            else {
                return Ok(None);
            };
            let (Some(then), Some(otherwise)) = (bounded_arm(then), bounded_arm(otherwise)) else {
                return Ok(None);
            };
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
            Ok(Some(syn::parse_quote!(if #condition {
                #then
            } else {
                #otherwise
            })))
        }
        Expr::Parameter { .. } => Ok(cell_source(value, &Type::Field, locals, parameters)),
        Expr::FieldLiteral { .. } => {
            let (literal, ty) = expression_with_calls(value, parameters, &HashMap::new())?;
            Ok((ty == Type::Field).then_some(literal))
        }
        // A pure conditional has no recording effects in either arm. Bind
        // the selected Field once, before the following ledger action.
        // Effectful arms stay unavailable until they can be lowered with
        // branch-local RecordingFrame state.
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let Some(condition) = cell_source(condition, &Type::Boolean, locals, parameters) else {
                return Ok(None);
            };
            let Some(then) = cell_source(then, &Type::Field, locals, parameters) else {
                return Ok(None);
            };
            let Some(otherwise) = cell_source(otherwise, &Type::Field, locals, parameters) else {
                return Ok(None);
            };
            let selected = syn::Ident::new(
                &format!("__compact_recorded_conditional_field_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote! {
                let #selected: runtime::Field = if #condition { #then } else { #otherwise };
            });
            Ok(Some(syn::parse_quote!(#selected)))
        }
        Expr::CellRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != (LedgerFieldKind::Cell { ty: Type::Field })
                || declaration.index != *index
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            let observed = syn::Ident::new(
                &format!("__compact_recorded_value_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote! {
                let (frame, #observed): (_, runtime::Field) =
                    crate::ledger_slots::#slot.record_read(frame)?;
            });
            Ok(Some(syn::parse_quote!(#observed)))
        }
        Expr::WitnessCall { name, arguments } => {
            let declaration = witnesses
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownWitness(name.clone()))?;
            if declaration.result != Type::Field {
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
                let value = if parameter.ty == Type::Field {
                    field_expression(
                        argument,
                        locals,
                        parameters,
                        ledger_fields,
                        witnesses,
                        circuits,
                        shared_callees,
                        steps,
                        next_temp,
                        visiting,
                    )?
                } else {
                    cell_source(argument, &parameter.ty, locals, parameters)
                };
                let Some(value) = value else {
                    return Ok(None);
                };
                let arg = syn::Ident::new(
                    &format!("__compact_recorded_arg_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #arg = #value;));
                args.push(arg);
            }
            let method = ident(name)?;
            let observed = syn::Ident::new(
                &format!("__compact_witness_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote! {
                let (frame, #observed) = frame.try_witness_metered(|context, meter| {
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
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right } => {
            let Some(left) = field_expression(
                left,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )?
            else {
                return Ok(None);
            };
            let Some(right) = field_expression(
                right,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )?
            else {
                return Ok(None);
            };
            let (name, arithmetic): (&str, syn::Expr) = match value {
                Expr::Add { .. } => ("sum", syn::parse_quote!(#left + #right)),
                Expr::Subtract { .. } => ("difference", syn::parse_quote!(#left - #right)),
                Expr::Multiply { .. } => ("product", syn::parse_quote!(#left * #right)),
                _ => unreachable!(),
            };
            let result = syn::Ident::new(
                &format!("__compact_recorded_{name}_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(let #result: runtime::Field = #arithmetic;));
            Ok(Some(syn::parse_quote!(#result)))
        }
        Expr::Call { name, arguments } => {
            let Some(callee) = circuits.get(name.as_str()) else {
                return Ok(None);
            };
            let StateReturn::Expression { value: result } = &callee.return_value else {
                return Ok(None);
            };
            if callee.result != Type::Field {
                return Ok(None);
            }
            if arguments.len() != callee.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: callee.parameters.len(),
                    actual: arguments.len(),
                });
            }
            if visiting.contains(name) {
                return Err(RenderError::UnsupportedStatefulCall(name.clone()));
            }
            if shared_callees.contains(name) {
                return shared_field_call(
                    name,
                    arguments,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                );
            }
            if !visiting.insert(name.clone()) {
                return Err(RenderError::UnsupportedStatefulCall(name.clone()));
            }
            let mut callee_locals = HashMap::new();
            for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                let value = if parameter.ty == Type::Field {
                    field_expression(
                        argument,
                        locals,
                        parameters,
                        ledger_fields,
                        witnesses,
                        circuits,
                        shared_callees,
                        steps,
                        next_temp,
                        visiting,
                    )?
                } else if parameter.ty
                    == (Type::Unsigned {
                        max: "65535".into(),
                    })
                {
                    amount_source(argument, locals, parameters)
                } else {
                    cell_source(argument, &parameter.ty, locals, parameters)
                };
                let Some(value) = value else {
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
            for (index, action) in callee.actions.iter().enumerate() {
                if !append_steps(
                    action,
                    &format!("callee[{name}].actions[{index}]"),
                    &callee_locals,
                    &HashMap::new(),
                    ledger_fields,
                    witnesses,
                    // This Field-returning stateful inlining path has no
                    // pure-circuit context. Keep nested pure calls
                    // unavailable until that context is threaded through.
                    &HashMap::new(),
                    circuits,
                    shared_callees,
                    steps,
                    next_temp,
                    visiting,
                    &mut LegacyActionPrecision::Concrete,
                )?
                .is_supported()
                {
                    visiting.remove(name);
                    return Ok(None);
                }
            }
            let value = field_expression(
                result,
                &callee_locals,
                &HashMap::new(),
                ledger_fields,
                witnesses,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )?;
            visiting.remove(name);
            let Some(value) = value else {
                return Ok(None);
            };
            let returned = syn::Ident::new(
                &format!("__compact_recorded_return_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(let #returned: runtime::Field = #value;));
            Ok(Some(syn::parse_quote!(#returned)))
        }
        Expr::MapLookup { field, index, key } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Map {
                key: key_ty,
                value: value_ty,
            } = &declaration.declaration
            else {
                return Ok(None);
            };
            if *value_ty != Type::Field || declaration.index != *index {
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
                &format!("__compact_recorded_lookup_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(
                let (frame, #observed): (_, runtime::Field) =
                    crate::ledger_slots::#slot.record_lookup(frame, #key)?;
            ));
            Ok(Some(syn::parse_quote!(#observed)))
        }
        _ => Ok(None),
    }
}
