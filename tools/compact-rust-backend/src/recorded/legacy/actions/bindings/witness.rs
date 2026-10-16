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

//! Witness binding validation and metered witness invocation.
//!
//! The parent preserves type-policy ordering before entering this protocol.
//! Rejections retain the enclosing action's provenance, and result binding occurs
//! only after the original declaration, arity, type, and argument checks.

use crate::ir::{Expr, LocalBinding, StateAction, Type, WitnessDeclaration};
use crate::recorded::legacy::value_sources::cell_source;
use crate::recorded::pure_calls::identity_struct_call_argument;
use crate::recorded::{RecordingOutcome, recordable_cell_type, unavailable_action};
use crate::{RenderError, ident};
use proc_macro2::Span;
use std::collections::HashMap;

#[expect(
    clippy::too_many_arguments,
    reason = "binding protocol keeps operands, scope, declarations, and recording state explicit"
)]
pub(super) fn append_witness_binding(
    whole: &StateAction,
    binding: &LocalBinding,
    path: &str,
    name: &str,
    arguments: &[Expr],
    scoped: &mut HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
) -> Result<RecordingOutcome<()>, RenderError> {
    if !matches!(
        binding.ty,
        Type::Boolean | Type::Field | Type::Bytes { .. } | Type::Unsigned { .. }
    ) && !(matches!(binding.ty, Type::Struct { .. }) && recordable_cell_type(&binding.ty))
    {
        return Ok(unavailable_action(whole, path));
    }
    let declaration = witnesses
        .get(name)
        .ok_or_else(|| RenderError::UnknownWitness(name.to_owned()))?;
    if arguments.len() != declaration.parameters.len() {
        return Err(RenderError::ArgumentCount {
            circuit: name.to_owned(),
            expected: declaration.parameters.len(),
            actual: arguments.len(),
        });
    }
    if binding.ty != declaration.result {
        return Err(RenderError::TypeMismatch {
            expected: binding.ty.clone(),
            actual: declaration.result.clone(),
        });
    }
    let mut args = Vec::new();
    for (argument, parameter) in arguments.iter().zip(&declaration.parameters) {
        if matches!(binding.ty, Type::Struct { .. }) && !identity_struct_call_argument(argument) {
            return Ok(unavailable_action(whole, path));
        }
        let Some(arg) = cell_source(argument, &parameter.ty, scoped, parameters) else {
            return Ok(unavailable_action(whole, path));
        };
        args.push(arg);
    }
    let method = ident(name)?;
    let value = syn::Ident::new(
        &format!("__compact_witness_{}", *next_temp),
        Span::call_site(),
    );
    *next_temp += 1;
    steps.push(syn::parse_quote! {
        let (frame, #value) = frame.try_witness_metered(|context, meter| {
            witnesses.#method(
                context.witness_context_with(super::LedgerView {
                    state: context.query.state.get_ref(),
                    meter,
                }),
                #(#args),*
            )
        })?;
    });
    scoped.insert(binding.name.clone(), syn::parse_quote!(#value));
    Ok(RecordingOutcome::Supported(()))
}
