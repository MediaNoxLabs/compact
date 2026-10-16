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

//! Conditional returns with ordered condition and pure-call branch lowering.
//!
//! Both branches are lowered after the condition, sharing temporary and recursion
//! state. A refused branch may have already advanced that state; this boundary
//! preserves those mutations and constructs diagnostics only on refusal.

use crate::ir::{
    Expr, LedgerField, PureCircuit, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::legacy::expressions::{boolean_expression, field_expression};
use crate::recorded::legacy::value_sources::cell_source;
use crate::recorded::{RecordingGap, RecordingOutcome};
use crate::{RenderError, ident};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

#[expect(
    clippy::too_many_arguments,
    reason = "conditional lowering keeps branch inputs, declarations, and shared recording state explicit"
)]
pub(in crate::recorded) fn lower(
    condition: &Expr,
    then: &Expr,
    otherwise: &Expr,
    expected: &Type,
    return_value: &StateReturn,
    result_ty: &syn::Type,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let mut return_steps = Vec::new();
    let Some(condition) = boolean_expression(
        condition,
        &HashMap::new(),
        parameters,
        ledger_fields,
        witnesses,
        circuits,
        &mut return_steps,
        next_temp,
        visiting,
    )?
    else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    };
    let Some((then_steps, then_result)) = pure_branch(
        then,
        expected,
        parameters,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        next_temp,
        visiting,
    )?
    else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    };
    let Some((otherwise_steps, otherwise_result)) = pure_branch(
        otherwise,
        expected,
        parameters,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        next_temp,
        visiting,
    )?
    else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    };
    return_steps.push(syn::parse_quote! {
        let (frame, observed): (_, #result_ty) = if #condition {
            #(#then_steps)*
            (frame, #then_result)
        } else {
            #(#otherwise_steps)*
            (frame, #otherwise_result)
        };
    });
    Ok(RecordingOutcome::Supported((
        return_steps,
        syn::parse_quote!(observed),
    )))
}

#[expect(
    clippy::too_many_arguments,
    reason = "branch lowering threads typed circuit context and recursive state explicitly"
)]
fn pure_branch(
    value: &Expr,
    expected: &Type,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let Expr::Call { name, arguments } = value else {
        return Ok(None);
    };
    let Some(callee) = pure_circuits.get(name.as_str()) else {
        return Ok(None);
    };
    if callee.result != *expected || arguments.len() != callee.parameters.len() {
        return Ok(None);
    }
    let mut steps = Vec::new();
    let mut args = Vec::new();
    for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
        let value = if parameter.ty == Type::Field {
            field_expression(
                argument,
                &HashMap::new(),
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                &HashSet::new(),
                &mut steps,
                next_temp,
                visiting,
            )?
        } else {
            cell_source(argument, &parameter.ty, &HashMap::new(), parameters)
        };
        let Some(value) = value else { return Ok(None) };
        let arg = syn::Ident::new(
            &format!("__compact_recorded_arg_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote!(let #arg = #value;));
        args.push(arg);
    }
    let method = ident(name)?;
    Ok(Some((
        steps,
        syn::parse_quote!(crate::pure_circuits::#method(#(#args),*)?),
    )))
}
