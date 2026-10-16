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

//! Admitted witness calls, pure guards, and assertions in recording order.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::ir::{
    Expr, LedgerField, PureCircuit, StateAction, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::legacy::expressions::boolean_expression;
use crate::recorded::legacy::value_sources::cell_source;
use crate::recorded::pure_calls::closed_pure_assert_call;
use crate::recorded::{RecordingGap, RecordingOutcome, recordable_cell_type, unavailable_action};
use crate::{RenderError, ident, rust_type};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

// This family retains the original lowering arms and ordered recording state.
#[expect(
    clippy::too_many_arguments,
    reason = "legacy lowering keeps declarations, scope, and ordered recording state explicit"
)]
pub(in crate::recorded) fn append_assertion_steps(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<RecordingOutcome<()>, RenderError> {
    match action {
        StateAction::Expression {
            value: Expr::WitnessCall { name, arguments },
        } => {
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
            if declaration.result != Type::Unit && declaration.result != Type::Field {
                return Ok(unavailable_action(action, path));
            }
            if declaration.result == Type::Unit
                && !(arguments.is_empty()
                    || matches!(declaration.parameters.as_slice(), [parameter]
                        if parameter.ty == Type::OpaqueString
                            || (matches!(parameter.ty, Type::Struct { .. })
                                && recordable_cell_type(&parameter.ty))))
            {
                return Ok(unavailable_action(action, path));
            }
            let mut args = Vec::new();
            for (index, (argument, parameter)) in
                arguments.iter().zip(&declaration.parameters).enumerate()
            {
                let Some(value) = cell_source(argument, &parameter.ty, locals, parameters) else {
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        argument,
                        format!("{path}.value.arguments[{index}]"),
                    )));
                };
                let ty = rust_type(&parameter.ty)?;
                let arg = syn::Ident::new(
                    &format!("__compact_recorded_witness_arg_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #arg: #ty = #value;));
                args.push(arg);
            }
            let method = ident(name)?;
            steps.push(syn::parse_quote! {
                let (frame, _) = frame.try_witness_metered(|context, meter| {
                    witnesses.#method(context.witness_context_with(super::LedgerView {
                        state: context.query.state.get_ref(),
                        meter,
                    }), #(#args),*)
                })?;
            });
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::PureCall { name, arguments } => {
            let Some(callee) = pure_circuits.get(name.as_str()) else {
                return Ok(unavailable_action(action, path));
            };
            if !closed_pure_assert_call(callee) || callee.result != Type::Unit {
                return Ok(unavailable_action(action, path));
            }
            let [parameter] = callee.parameters.as_slice() else {
                unreachable!("closed pure assertion has one parameter")
            };
            let [argument] = arguments.as_slice() else {
                return Ok(unavailable_action(action, path));
            };
            let Some(argument) = cell_source(argument, &parameter.ty, locals, parameters) else {
                return Ok(unavailable_action(action, path));
            };
            let method = ident(name)?;
            let typed_arg = syn::Ident::new(
                &format!("__compact_recorded_pure_assert_arg_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let arg_ty = rust_type(&parameter.ty)?;
            steps.push(syn::parse_quote!(let #typed_arg: #arg_ty = #argument;));
            steps.push(syn::parse_quote!(crate::pure_circuits::#method(#typed_arg)?;));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::Assert { condition, message } => {
            // A typed public Uint<8> may be widened to Uint<32> for an
            // equality guard. Keep the checked cast and assertion before
            // any later public action; no operand may observe ledger or
            // witness state on this path.
            if let Expr::Equal { left, right } = condition
                && let Expr::UnsignedCast {
                    max,
                    value: small_value,
                } = left.as_ref()
                && max == "4294967295"
                && let Expr::Parameter { name: small_name } = small_value.as_ref()
                && let Expr::Parameter { name: big_name } = right.as_ref()
                && !locals.contains_key(small_name)
                && !locals.contains_key(big_name)
                && matches!(
                    parameters.get(small_name.as_str()),
                    Some((Type::Unsigned { max }, _)) if max == "255"
                )
                && matches!(
                    parameters.get(big_name.as_str()),
                    Some((Type::Unsigned { max }, _)) if max == "4294967295"
                )
            {
                let Some(small) = cell_source(
                    small_value,
                    &Type::Unsigned { max: "255".into() },
                    locals,
                    parameters,
                ) else {
                    return Ok(unavailable_action(action, path));
                };
                let Some(big) = cell_source(
                    right,
                    &Type::Unsigned {
                        max: "4294967295".into(),
                    },
                    locals,
                    parameters,
                ) else {
                    return Ok(unavailable_action(action, path));
                };
                let widened = syn::Ident::new(
                    &format!("__compact_recorded_widened_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote! {
                    let #widened: runtime::BoundedUint<4294967295> =
                        runtime::cast_unsigned::<255, 4294967295>(#small)?;
                });
                steps.push(syn::parse_quote! {
                    if !(#widened == #big) {
                        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                    }
                });
                return Ok(RecordingOutcome::Supported(()));
            }
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
                return Ok(unavailable_action(action, path));
            };
            steps.push(syn::parse_quote! {
                if !(#condition) {
                    return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                }
            });
            Ok(RecordingOutcome::Supported(()))
        }
        _ => Ok(unavailable_action(action, path)),
    }
}
