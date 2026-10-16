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

//! Field-valued lexical bindings and their ordered recording policies.
//!
//! The caller has already admitted the Field type and applied prefix/hash rules.
//! Each policy either binds the value or returns its concrete refusal. A refusal
//! may retain statements or temporary identifiers emitted while lowering arguments.

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, LocalBinding, PureCircuit, StateReturn, StatefulCircuit,
    Type, WitnessDeclaration,
};
use crate::recorded::legacy::calls::shared_field_call;
use crate::recorded::legacy::expressions::field_expression;
use crate::recorded::legacy::value_sources::cell_source;
use crate::recorded::pure_calls::{closed_pure_field_call, closed_pure_field_pair_hash_call};
use crate::recorded::{RecordingGap, RecordingOutcome};
use crate::{RenderError, ident, rust_type};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

#[expect(
    clippy::too_many_arguments,
    reason = "binding policy keeps declaration dependencies and ordered mutable recording state explicit"
)]
pub(super) fn append_field_binding(
    binding: &LocalBinding,
    binding_index: usize,
    path: &str,
    scoped: &mut HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<RecordingOutcome<()>, RenderError> {
    // Admit an internal Field helper only when its complete body is
    // a closed typed pair-hash call followed by one Field Cell read.
    // The pure result must be evaluated before the recorded read,
    // then the sum must be retained for the nested public action.
    if let Expr::Call { name, arguments } = &binding.value
        && let Some(callee) = circuits.get(name.as_str())
        && callee.internal
        && callee.actions.is_empty()
        && callee.result == Type::Field
        && let [callee_parameter] = callee.parameters.as_slice()
        && let [argument] = arguments.as_slice()
        && let StateReturn::Expression {
            value: Expr::Add { left, right },
        } = &callee.return_value
        && let Expr::Call {
            name: pure_name,
            arguments: pure_arguments,
        } = left.as_ref()
        && let [
            Expr::Coerce {
                value: pure_argument,
                ty: pure_argument_ty,
            },
        ] = pure_arguments.as_slice()
        && let Expr::Parameter {
            name: pure_parameter_name,
        } = pure_argument.as_ref()
        && pure_parameter_name == &callee_parameter.name
        && pure_argument_ty == &callee_parameter.ty
        && let Some(pure) = pure_circuits.get(pure_name.as_str())
        && pure.result == Type::Field
        && let [pure_parameter] = pure.parameters.as_slice()
        && pure_parameter.ty == callee_parameter.ty
        && closed_pure_field_pair_hash_call(pure_name, pure_circuits)
        && let Expr::CellRead { field, index } = right.as_ref()
        && let Some(declaration) = ledger_fields.get(field.as_str())
        && declaration.index == *index
        && declaration.declaration == (LedgerFieldKind::Cell { ty: Type::Field })
    {
        let Some(arg_value) = cell_source(argument, &callee_parameter.ty, scoped, parameters)
        else {
            return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                argument,
                format!("{path}.bindings[{binding_index}].value.arguments[0]"),
            )));
        };
        let arg = syn::Ident::new(
            &format!("__compact_recorded_helper_arg_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let arg_ty = rust_type(&callee_parameter.ty)?;
        steps.push(syn::parse_quote!(let #arg: #arg_ty = #arg_value;));
        let pure_result = syn::Ident::new(
            &format!("__compact_recorded_helper_pure_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let pure_method = ident(pure_name)?;
        steps.push(syn::parse_quote! {
            let #pure_result: runtime::Field =
                crate::pure_circuits::#pure_method(#arg)?;
        });
        let observed = syn::Ident::new(
            &format!("__compact_recorded_helper_read_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let slot = ident(field)?;
        steps.push(syn::parse_quote! {
            let (frame, #observed): (_, runtime::Field) =
                crate::ledger_slots::#slot.record_read(frame)?;
        });
        let result = syn::Ident::new(
            &format!("__compact_recorded_helper_result_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote! {
            let #result: runtime::Field = #pure_result + #observed;
        });
        scoped.insert(binding.name.clone(), syn::parse_quote!(#result));
        return Ok(RecordingOutcome::Supported(()));
    }
    if let Expr::Call { name, arguments } = &binding.value
        && let Some(callee) = pure_circuits.get(name.as_str())
        && callee.result == Type::Field
        && closed_pure_field_call(name, pure_circuits, &mut HashSet::new())
    {
        if arguments.len() != callee.parameters.len() {
            return Err(RenderError::ArgumentCount {
                circuit: name.clone(),
                expected: callee.parameters.len(),
                actual: arguments.len(),
            });
        }
        let mut args = Vec::new();
        for (argument_index, (argument, parameter)) in
            arguments.iter().zip(&callee.parameters).enumerate()
        {
            if parameter.ty != Type::Field {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    argument,
                    format!("{path}.bindings[{binding_index}].value.arguments[{argument_index}]"),
                )));
            }
            let Some(argument) = field_expression(
                argument,
                scoped,
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    argument,
                    format!("{path}.bindings[{binding_index}].value.arguments[{argument_index}]"),
                )));
            };
            let arg = syn::Ident::new(
                &format!("__compact_recorded_arg_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(let #arg: runtime::Field = #argument;));
            args.push(arg);
        }
        let method = ident(name)?;
        let value = syn::Ident::new(
            &format!("__compact_recorded_pure_field_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote! {
            let #value: runtime::Field = crate::pure_circuits::#method(#(#args),*)?;
        });
        scoped.insert(binding.name.clone(), syn::parse_quote!(#value));
        return Ok(RecordingOutcome::Supported(()));
    }
    if let Expr::Call { name, arguments } = &binding.value
        && closed_pure_field_pair_hash_call(name, pure_circuits)
    {
        let callee = pure_circuits
            .get(name.as_str())
            .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
        if arguments.len() != callee.parameters.len() {
            return Err(RenderError::ArgumentCount {
                circuit: name.clone(),
                expected: callee.parameters.len(),
                actual: arguments.len(),
            });
        }
        let mut args = Vec::new();
        for (argument_index, (argument, parameter)) in
            arguments.iter().zip(&callee.parameters).enumerate()
        {
            let Some(argument_value) = cell_source(argument, &parameter.ty, scoped, parameters)
            else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    argument,
                    format!("{path}.bindings[{binding_index}].value.arguments[{argument_index}]"),
                )));
            };
            let arg = syn::Ident::new(
                &format!("__compact_recorded_pair_arg_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let ty = rust_type(&parameter.ty)?;
            steps.push(syn::parse_quote!(let #arg: #ty = #argument_value;));
            args.push(arg);
        }
        let method = ident(name)?;
        let value = syn::Ident::new(
            &format!("__compact_recorded_pure_pair_hash_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote! {
            let #value: runtime::Field = crate::pure_circuits::#method(#(#args),*)?;
        });
        scoped.insert(binding.name.clone(), syn::parse_quote!(#value));
        return Ok(RecordingOutcome::Supported(()));
    }
    if let Expr::Call { name, arguments } = &binding.value
        && shared_callees.contains(name)
    {
        let Some(observed) = shared_field_call(
            name,
            arguments,
            scoped,
            parameters,
            ledger_fields,
            witnesses,
            circuits,
            steps,
            next_temp,
            visiting,
        )?
        else {
            return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                &binding.value,
                format!("{path}.bindings[{binding_index}].value"),
            )));
        };
        scoped.insert(binding.name.clone(), observed);
        return Ok(RecordingOutcome::Supported(()));
    }
    let Some(value) = field_expression(
        &binding.value,
        scoped,
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
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            &binding.value,
            format!("{path}.bindings[{binding_index}].value"),
        )));
    };
    scoped.insert(binding.name.clone(), value);
    Ok(RecordingOutcome::Supported(()))
}
