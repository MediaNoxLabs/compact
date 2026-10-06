// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Selective native frame lowering for witnessed Field Cells and simple Unit leaves.
//!
//! The typed IR remains authoritative. Unsupported shapes return `None` and
//! use the existing general emitter; this path never rewrites rendered text.

use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

use crate::ir::{
    CounterAmount, Expr, LedgerField, LedgerFieldKind, StateAction, StateReturn, StatefulCircuit,
    Type, WitnessDeclaration,
};
use crate::stateful::circuit_uses_witness;
use crate::{RenderError, expression_with_calls, ident};

fn field_expression(
    value: &Expr,
    bindings: &HashMap<String, syn::Ident>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    steps: &mut Vec<syn::Stmt>,
    next_id: &mut usize,
) -> Result<Option<syn::Expr>, RenderError> {
    match value {
        Expr::Parameter { name } => Ok(bindings.get(name).map(|name| syn::parse_quote!(#name))),
        Expr::Coerce { value, ty } if *ty == Type::Field => {
            field_expression(value, bindings, witnesses, steps, next_id)
        }
        Expr::FieldLiteral { .. } => {
            let (syntax, ty) = expression_with_calls(value, &HashMap::new(), &HashMap::new())?;
            Ok((ty == Type::Field).then_some(syntax))
        }
        Expr::Add { left, right } => {
            let Some(left) = field_expression(left, bindings, witnesses, steps, next_id)? else {
                return Ok(None);
            };
            let Some(right) = field_expression(right, bindings, witnesses, steps, next_id)? else {
                return Ok(None);
            };
            Ok(Some(syn::parse_quote!((#left) + (#right))))
        }
        Expr::WitnessCall { name, arguments } => {
            let Some(declaration) = witnesses.get(name.as_str()) else {
                return Ok(None);
            };
            if declaration.result != Type::Field
                || arguments.len() != declaration.parameters.len()
                || declaration
                    .parameters
                    .iter()
                    .any(|parameter| parameter.ty != Type::Field)
            {
                return Ok(None);
            }
            let mut args = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let Some(arg) = field_expression(argument, bindings, witnesses, steps, next_id)?
                else {
                    return Ok(None);
                };
                args.push(arg);
            }
            let witness_name = ident(name)?;
            let value_name = syn::Ident::new(
                &format!("__compact_frame_witness_{}", *next_id),
                Span::call_site(),
            );
            *next_id += 1;
            steps.push(syn::parse_quote! {
                let (frame, #value_name) = frame.try_witness_metered(|context, meter| {
                    witnesses.#witness_name(
                        context.witness_context_with(LedgerView {
                            state: context.query.state.get_ref(),
                            meter,
                        }),
                        #(#args),*
                    )
                })?;
            });
            Ok(Some(syn::parse_quote!(#value_name)))
        }
        _ => Ok(None),
    }
}

fn append_action(
    action: &StateAction,
    bindings: &HashMap<String, syn::Ident>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    steps: &mut Vec<syn::Stmt>,
    next_id: &mut usize,
) -> Result<bool, RenderError> {
    match action {
        StateAction::Sequence { actions } => {
            for action in actions {
                if !append_action(
                    action,
                    bindings,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_id,
                )? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        StateAction::Let {
            bindings: locals,
            action,
        } => {
            let mut scope = bindings.clone();
            for binding in locals {
                if binding.ty != Type::Field {
                    return Ok(false);
                }
                let Some(value) =
                    field_expression(&binding.value, &scope, witnesses, steps, next_id)?
                else {
                    return Ok(false);
                };
                let local = syn::Ident::new(
                    &format!("__compact_frame_local_{}", *next_id),
                    Span::call_site(),
                );
                *next_id += 1;
                steps.push(syn::parse_quote!(let #local: runtime::Field = #value;));
                scope.insert(binding.name.clone(), local);
            }
            append_action(
                action,
                &scope,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_id,
            )
        }
        StateAction::CellWrite {
            field,
            index,
            value,
        } => {
            let Some(declaration) = ledger_fields.get(field.as_str()) else {
                return Ok(false);
            };
            if declaration.index != *index
                || declaration.declaration != (LedgerFieldKind::Cell { ty: Type::Field })
            {
                return Ok(false);
            }
            let Some(value) = field_expression(value, bindings, witnesses, steps, next_id)? else {
                return Ok(false);
            };
            let slot = ident(&declaration.id)?;
            steps.push(syn::parse_quote! {
                let (frame, ()) = frame.apply(|context| {
                    crate::ledger_slots::#slot.write(context, #value)
                })?;
            });
            Ok(true)
        }
        StateAction::CircuitCall { name, arguments } => {
            let Some(callee) = circuits.get(name.as_str()) else {
                return Ok(false);
            };
            if callee.result != Type::Unit
                || arguments.len() != callee.parameters.len()
                || callee
                    .parameters
                    .iter()
                    .any(|parameter| parameter.ty != Type::Field)
            {
                return Ok(false);
            }
            let mut args = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let Some(arg) = field_expression(argument, bindings, witnesses, steps, next_id)?
                else {
                    return Ok(false);
                };
                args.push(arg);
            }
            let callee_name = ident(name)?;
            if circuit_uses_witness(callee, circuits, &mut HashSet::new())? {
                steps.push(syn::parse_quote! {
                    let (frame, ()) = frame.apply(|context| {
                        self::#callee_name(context, witnesses, #(#args),*)
                    })?;
                });
            } else {
                steps.push(syn::parse_quote! {
                    let (frame, ()) = frame.apply(|context| {
                        self::#callee_name(context, #(#args),*)
                    })?;
                });
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Reuse runtime result ownership for two closed, parameterless native leaves.
/// A miss (including malformed literals/names) leaves diagnostics to the general
/// native emitter. No expression evaluator or source-name whitelist lives here.
fn render_simple_leaf(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Option<syn::Item> {
    if !circuit.parameters.is_empty()
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
    {
        return None;
    }
    let [action] = circuit.actions.as_slice() else {
        return None;
    };
    let (field, index, expected, preparation, method, argument) = match action {
        StateAction::CellWrite {
            field,
            index,
            value: Expr::Boolean { value },
        } => (
            field,
            *index,
            LedgerFieldKind::Cell { ty: Type::Boolean },
            None,
            "write",
            syn::parse_quote!(#value),
        ),
        StateAction::Let { bindings, action } => {
            let [binding] = bindings.as_slice() else {
                return None;
            };
            let expected = Type::Unsigned {
                max: "65535".into(),
            };
            if binding.ty != expected {
                return None;
            }
            let Expr::UnsignedLiteral { max, .. } = &binding.value else {
                return None;
            };
            if max != "65535" {
                return None;
            }
            let StateAction::CounterIncrement {
                field,
                index,
                amount: CounterAmount::Parameter { name },
            } = action.as_ref()
            else {
                return None;
            };
            if name != &binding.name {
                return None;
            }
            ident(&binding.name).ok()?;
            let (literal, ty) =
                expression_with_calls(&binding.value, &HashMap::new(), &HashMap::new()).ok()?;
            if ty != expected {
                return None;
            }
            let preparation: syn::Stmt = syn::parse_quote!(
                let __compact_action_local_0: runtime::BoundedUint<65535> = #literal;
            );
            (
                field,
                *index,
                LedgerFieldKind::Counter,
                Some(preparation),
                "increment",
                syn::parse_quote!(__compact_action_local_0.value() as u16),
            )
        }
        _ => return None,
    };
    let declaration = ledger_fields.get(field.as_str())?;
    if declaration.index != index || declaration.declaration != expected {
        return None;
    }
    let slot = ident(&declaration.id).ok()?;
    let argument: syn::Expr = argument;
    let method = syn::Ident::new(method, Span::call_site());
    let name = ident(&circuit.name).ok()?;
    let visibility: syn::Visibility = if circuit.internal {
        syn::parse_quote!(pub(crate))
    } else {
        syn::parse_quote!(pub)
    };
    Some(syn::parse_quote! {
        #visibility fn #name<Private>(
            context: runtime::context::CircuitContext<Private>,
        ) -> Result<runtime::context::CircuitResult<Private, ()>, runtime::CompactError> {
            #preparation
            let frame = runtime::context::CircuitFrame::new(context);
            let (frame, ()) = frame.apply(|context| {
                crate::ledger_slots::#slot.#method(context, #argument)
            })?;
            Ok(frame.finish(()))
        }
    })
}

pub(crate) fn render_if_supported(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<syn::Item>, RenderError> {
    if let Some(item) = render_simple_leaf(circuit, ledger_fields) {
        return Ok(Some(item));
    }
    if circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
        || circuit
            .parameters
            .iter()
            .any(|parameter| parameter.ty != Type::Field)
        || !circuit_uses_witness(circuit, circuits, &mut HashSet::new())?
    {
        return Ok(None);
    }
    let mut bindings = HashMap::new();
    let mut args = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        ident(&parameter.name)?;
        let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if bindings
            .insert(parameter.name.clone(), name.clone())
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
        args.push(syn::parse_quote!(#name: runtime::Field));
    }
    let mut steps = Vec::new();
    let mut next_id = 0;
    for action in &circuit.actions {
        if !append_action(
            action,
            &bindings,
            ledger_fields,
            witnesses,
            circuits,
            &mut steps,
            &mut next_id,
        )? {
            return Ok(None);
        }
    }
    if steps.is_empty() {
        return Ok(None);
    }
    let name = ident(&circuit.name)?;
    let visibility: syn::Visibility = if circuit.internal {
        syn::parse_quote!(pub(crate))
    } else {
        syn::parse_quote!(pub)
    };
    Ok(Some(syn::parse_quote! {
        #visibility fn #name<Private, W: TryWitnesses<Private>>(
            context: runtime::context::CircuitContext<Private>,
            witnesses: &W,
            #(#args),*
        ) -> Result<runtime::context::CircuitResult<Private, ()>, runtime::CompactError> {
            let frame = runtime::context::CircuitFrame::new(context);
            #(#steps)*
            Ok(frame.finish(()))
        }
    }))
}

#[cfg(test)]
mod tests;
