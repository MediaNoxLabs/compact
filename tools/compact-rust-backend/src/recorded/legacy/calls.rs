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

//! Shared recorded helper arguments and calls within the current recording frame.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::ir::{Expr, LedgerField, StatefulCircuit, Type, WitnessDeclaration};
use crate::recorded::helpers::helper_ident;
use crate::recorded::legacy::expressions::field_expression;
use crate::recorded::legacy::value_sources::{amount_source, cell_source};
use crate::stateful::circuit_uses_witness;
use crate::{RenderError, rust_type};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

/// Keep shared Unit and value-returning helpers on the same typed argument path.
/// `field_expression` appends effects before the caller binds each argument.
#[expect(
    clippy::too_many_arguments,
    reason = "shared callee arguments reuse recursive lowering context and ordered recording state"
)]
pub(in crate::recorded) fn shared_call_argument(
    argument: &Expr,
    ty: &Type,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<syn::Expr>, RenderError> {
    if *ty == Type::Field {
        return field_expression(
            argument,
            locals,
            parameters,
            ledger_fields,
            witnesses,
            circuits,
            &HashSet::new(),
            steps,
            next_temp,
            visiting,
        );
    }
    if *ty
        == (Type::Unsigned {
            max: "65535".into(),
        })
    {
        let uncoerced = match argument {
            Expr::Coerce { value, ty: target } if target == ty => value.as_ref(),
            _ => argument,
        };
        return Ok(match uncoerced {
            Expr::Parameter { name } if !locals.contains_key(name) => {
                cell_source(argument, ty, locals, parameters)
            }
            _ => amount_source(argument, locals, parameters).map(|value| {
                syn::parse_quote!(
                    runtime::BoundedUint::<65535>::new((#value) as u128)
                        .expect("Compact Uint argument fits its maximum")
                )
            }),
        });
    }
    Ok(cell_source(argument, ty, locals, parameters))
}

/// Reuse one typed callee body without opening or finishing another frame.
#[expect(
    clippy::too_many_arguments,
    reason = "shared callee lowering keeps declaration context and ordered recording state explicit"
)]
pub(in crate::recorded) fn shared_field_call(
    name: &str,
    arguments: &[Expr],
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<syn::Expr>, RenderError> {
    let callee = circuits
        .get(name)
        .ok_or_else(|| RenderError::UnknownCircuit(name.to_owned()))?;
    if callee.result != Type::Field || arguments.len() != callee.parameters.len() {
        return Ok(None);
    }
    let mut args = Vec::new();
    for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
        let Some(value) = shared_call_argument(
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
        )?
        else {
            return Ok(None);
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
    let observed = syn::Ident::new(
        &format!("__compact_recorded_value_{}", *next_temp),
        Span::call_site(),
    );
    *next_temp += 1;
    let helper = helper_ident(name, circuits)?;
    if circuit_uses_witness(callee, circuits, &mut HashSet::new())? {
        steps.push(syn::parse_quote!(
            let (frame, #observed) = #helper(frame, witnesses, #(#args),*)?;
        ));
    } else {
        steps.push(syn::parse_quote!(
            let (frame, #observed) = #helper(frame, #(#args),*)?;
        ));
    }
    Ok(Some(syn::parse_quote!(#observed)))
}
