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

//! Ordered action sequences, branch frames, and recursive Unit circuit calls.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::ir::{
    LedgerField, PureCircuit, StateAction, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::helpers::helper_ident;
use crate::recorded::legacy::actions::append_steps;
use crate::recorded::legacy::calls::shared_call_argument;
use crate::recorded::legacy::expressions::{boolean_expression, field_expression};
use crate::recorded::legacy::value_sources::{amount_source, cell_source};
use crate::recorded::{LegacyActionPrecision, RecordingGap, RecordingOutcome, unavailable_action};
use crate::stateful::circuit_uses_witness;
use crate::{RenderError, rust_type};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

// This family retains the original lowering arms and ordered recording state.
#[expect(
    clippy::too_many_arguments,
    reason = "legacy lowering keeps declarations, scope, and ordered recording state explicit"
)]
pub(in crate::recorded) fn append_control_steps(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
    failure_precision: &mut LegacyActionPrecision,
) -> Result<RecordingOutcome<()>, RenderError> {
    match action {
        StateAction::Sequence { actions } => {
            for (index, action) in actions.iter().enumerate() {
                if let RecordingOutcome::Unsupported(gap) = append_steps(
                    action,
                    &format!("{path}.actions[{index}]"),
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                    shared_callees,
                    steps,
                    next_temp,
                    visiting,
                    &mut LegacyActionPrecision::Concrete,
                )? {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            let mut condition_steps = Vec::new();
            let Some(condition) = boolean_expression(
                condition,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut condition_steps,
                next_temp,
                visiting,
            )?
            else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    condition,
                    format!("{path}.condition"),
                )));
            };
            let mut then_steps = Vec::new();
            if let RecordingOutcome::Unsupported(gap) = append_steps(
                then,
                &format!("{path}.then"),
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                &mut then_steps,
                next_temp,
                visiting,
                &mut LegacyActionPrecision::Concrete,
            )? {
                return Ok(RecordingOutcome::Unsupported(gap));
            }
            let mut otherwise_steps = Vec::new();
            if let RecordingOutcome::Unsupported(gap) = append_steps(
                otherwise,
                &format!("{path}.otherwise"),
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                &mut otherwise_steps,
                next_temp,
                visiting,
                &mut LegacyActionPrecision::Concrete,
            )? {
                return Ok(RecordingOutcome::Unsupported(gap));
            }
            let selected = syn::Ident::new(
                &format!("__compact_recorded_branch_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.extend(condition_steps);
            steps.push(syn::parse_quote!(let #selected: bool = #condition;));
            steps.push(syn::parse_quote!(
                #[allow(clippy::let_and_return)]
                let frame = if #selected {
                    #(#then_steps)*
                    frame
                } else {
                    #(#otherwise_steps)*
                    frame
                };
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::CircuitCall { name, arguments } => {
            let callee = circuits
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnsupportedStatefulCall(name.clone()))?;
            if callee.result != Type::Unit || callee.return_value != StateReturn::Unit {
                *failure_precision = LegacyActionPrecision::Coarse;
                return Ok(unavailable_action(action, path));
            }
            if arguments.len() != callee.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: callee.parameters.len(),
                    actual: arguments.len(),
                });
            }
            if shared_callees.contains(name) {
                let mut args = Vec::new();
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let value = shared_call_argument(
                        argument,
                        &parameter.ty,
                        locals,
                        parameters,
                        ledger_fields,
                        witnesses,
                        circuits,
                        steps,
                        next_temp,
                        visiting,
                    )?;
                    let Some(value) = value else {
                        *failure_precision = LegacyActionPrecision::Coarse;
                        return Ok(unavailable_action(action, path));
                    };
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let arg_ty = rust_type(&parameter.ty)?;
                    steps.push(syn::parse_quote!(let #arg: #arg_ty = #value;));
                    args.push(arg);
                }
                let helper = helper_ident(name, circuits)?;
                if circuit_uses_witness(callee, circuits, &mut HashSet::new())? {
                    steps.push(syn::parse_quote!(
                        let (frame, _) = #helper(frame, witnesses, #(#args),*)?;
                    ));
                } else {
                    steps.push(syn::parse_quote!(
                        let (frame, _) = #helper(frame, #(#args),*)?;
                    ));
                }
                return Ok(RecordingOutcome::Supported(()));
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
                    *failure_precision = LegacyActionPrecision::Coarse;
                    return Ok(unavailable_action(action, path));
                };
                let arg = syn::Ident::new(
                    &format!("__compact_recorded_arg_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #arg = #value;));
                callee_locals.insert(parameter.name.clone(), syn::parse_quote!(#arg));
            }
            let mut failure = None;
            for (index, action) in callee.actions.iter().enumerate() {
                if let RecordingOutcome::Unsupported(gap) = append_steps(
                    action,
                    &format!("callee[{name}].actions[{index}]"),
                    &callee_locals,
                    &HashMap::new(),
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                    shared_callees,
                    steps,
                    next_temp,
                    visiting,
                    &mut LegacyActionPrecision::Concrete,
                )? {
                    failure = Some(gap);
                    break;
                }
            }
            visiting.remove(name);
            Ok(failure.map_or(
                RecordingOutcome::Supported(()),
                RecordingOutcome::Unsupported,
            ))
        }
        _ => Ok(unavailable_action(action, path)),
    }
}
