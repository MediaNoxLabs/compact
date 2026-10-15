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

//! Native witness outputs, expression effects, calls, and assertions.
//!
//! The traversal owns lexical scope and scheduling. This domain only appends
//! the selected action's statements and updates its explicit effect state.

use super::ActionOutput;
use crate::circuit_analysis::{
    circuit_emits_native_private_output_observed, circuit_uses_witness_observed,
};
use crate::ir::{
    Expr, LedgerField, NativeWitnessBuiltin, PureCircuit, StateAction, StatefulCircuit, Type,
    WitnessDeclaration,
};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite};
use crate::stateful::{NativeExpressionContext, expression::render_state_expression_observed};
use crate::{RenderError, discard_expression, ident};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

#[expect(
    clippy::too_many_arguments,
    reason = "native action lowering keeps declaration scope and ordered effect state explicit"
)]
pub(super) fn lower(
    action: &StateAction,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
    statements: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    uses_witness: &mut bool,
) -> Result<(), RenderError> {
    let environment = NativeExpressionContext {
        witnesses,
        circuits,
        stateful_circuits,
        ledger_fields,
        observer: None,
    };
    lower_observed(
        action,
        parameters,
        &environment,
        ActionOutput {
            statements,
            next_temp,
            uses_witness,
        },
    )
}

pub(super) fn lower_observed(
    action: &StateAction,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    environment: &NativeExpressionContext<'_, '_>,
    output: ActionOutput<'_>,
) -> Result<(), RenderError> {
    let ActionOutput {
        statements,
        next_temp,
        uses_witness,
    } = output;
    let stateful_circuits = environment.stateful_circuits;
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
            let (rendered, ty, effect) = render_state_expression_observed(
                value,
                parameters,
                &mut effect_statements,
                next_temp,
                &mut query_effect,
                environment,
            )?;
            if effect {
                *uses_witness = true;
            }
            if effect || query_effect {
                statements.push(syn::parse_quote!(let mut context = context;));
            }
            statements.extend(effect_statements);
            statements.extend(discard_expression(rendered, &ty));
        }
        StateAction::PureCall { name, arguments } => {
            let rendered = pure_call::render(action, name, arguments, parameters, environment)?;
            statements.push(syn::parse_quote!(#rendered;));
        }
        StateAction::CircuitCall {
            name: callee_name,
            arguments,
        } => {
            let callee = stateful_circuits.get(callee_name.as_str()).ok_or_else(|| {
                refused_action_name(action, environment.observer);
                RenderError::UnknownCircuit(callee_name.clone())
            })?;
            let callee_uses_witness = circuit_uses_witness_observed(
                callee,
                stateful_circuits,
                &mut HashSet::new(),
                Some(RefusalSite::field(Owner::Action(action), "name")),
                environment.observer,
            )?;
            let callee_emits_native = circuit_emits_native_private_output_observed(
                callee,
                stateful_circuits,
                &mut HashSet::new(),
                Some(RefusalSite::field(Owner::Action(action), "name")),
                environment.observer,
            )?;
            if arguments.len() != callee.parameters.len() {
                refused_action_name(action, environment.observer);
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
                let (rendered, actual, effect) = render_state_expression_observed(
                    argument,
                    parameters,
                    &mut argument_statements,
                    next_temp,
                    &mut argument_query_effect,
                    environment,
                )?;
                if actual != parameter.ty {
                    refused_expression(argument, environment.observer);
                    return Err(RenderError::TypeMismatch {
                        expected: parameter.ty.clone(),
                        actual,
                    });
                }
                let argument_name = syn::Ident::new(
                    &format!("__compact_call_argument_{}", next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                argument_statements.push(syn::parse_quote!(let #argument_name = #rendered;));
                args.push(syn::parse_quote!(#argument_name));
                argument_witness_effect |= effect;
            }
            if argument_query_effect || argument_witness_effect {
                statements.push(syn::parse_quote!(let mut context = context;));
            }
            statements.extend(argument_statements);
            *uses_witness |= argument_witness_effect;
            let callee_name = ident(callee_name).inspect_err(|_| {
                refused_action_name(action, environment.observer);
            })?;
            if callee_uses_witness {
                statements.push(syn::parse_quote! {
                    let call_step = #callee_name(context, witnesses, #(#args),*)?;
                });
                *uses_witness = true;
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
            let original_condition = condition;
            let mut effect_statements = Vec::new();
            let mut query_effect = false;
            let (condition, actual, effect) = render_state_expression_observed(
                condition,
                parameters,
                &mut effect_statements,
                next_temp,
                &mut query_effect,
                environment,
            )?;
            if actual != Type::Boolean {
                refused_expression(original_condition, environment.observer);
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual,
                });
            }
            if effect {
                *uses_witness = true;
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
        _ => unreachable!("native effects action dispatched to its declared domain"),
    }
    Ok(())
}

fn refused_action_name(action: &StateAction, observer: Option<&dyn RefusalObserver>) {
    if let Some(observer) = observer {
        observer.refused(RefusalSite::field(Owner::Action(action), "name"));
    }
}
fn refused_expression(source: &Expr, observer: Option<&dyn RefusalObserver>) {
    if let Some(observer) = observer {
        observer.refused(RefusalSite::node(Owner::Expr(source)));
    }
}

mod pure_call;
