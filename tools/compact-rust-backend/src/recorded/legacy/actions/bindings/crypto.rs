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

//! Hash and commitment bindings after the parent's ordered admission checks.
//!
//! Pair hashing and commitment opening reads retain their original operand,
//! temporary, statement, and scope-update order, including partial refusals.

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, LocalBinding, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::legacy::expressions::field_expression;
use crate::recorded::legacy::value_sources::literal_field_pair_hash_source;
use crate::recorded::{RecordingGap, RecordingOutcome};
use crate::{RenderError, ident};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

#[expect(
    clippy::too_many_arguments,
    reason = "binding protocol keeps operands, scope, declarations, and recording state explicit"
)]
pub(super) fn append_pair_hash_binding(
    binding: &LocalBinding,
    binding_index: usize,
    path: &str,
    operand: &Expr,
    persistent: bool,
    scoped: &mut HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
) -> Result<RecordingOutcome<()>, RenderError> {
    let Some(operand) = literal_field_pair_hash_source(operand, scoped, parameters) else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            &binding.value,
            format!("{path}.bindings[{binding_index}].value"),
        )));
    };
    let pair = syn::Ident::new(
        &format!("__compact_recorded_hash_arg_{}", *next_temp),
        Span::call_site(),
    );
    *next_temp += 1;
    let result = syn::Ident::new(
        &format!("__compact_recorded_hash_{}", *next_temp),
        Span::call_site(),
    );
    *next_temp += 1;
    steps.push(syn::parse_quote! {
        let #pair: (runtime::Field, runtime::Field) = #operand;
    });
    if persistent {
        steps.push(syn::parse_quote! {
            let #result: runtime::FixedBytes<32> = runtime::persistent_hash(#pair);
        });
    } else {
        steps.push(syn::parse_quote! {
            let #result: runtime::Field = runtime::transient_hash(#pair);
        });
    }
    scoped.insert(binding.name.clone(), syn::parse_quote!(#result));
    Ok(RecordingOutcome::Supported(()))
}

#[expect(
    clippy::too_many_arguments,
    reason = "binding protocol keeps operands, scope, declarations, and recording state explicit"
)]
pub(super) fn append_commitment_binding(
    binding: &LocalBinding,
    binding_index: usize,
    path: &str,
    value: &Expr,
    opening: &Expr,
    scoped: &mut HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<RecordingOutcome<()>, RenderError> {
    if !matches!(value, Expr::FieldLiteral { .. }) {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            value,
            format!("{path}.bindings[{binding_index}].value.value"),
        )));
    }
    let Expr::CellRead { field, index } = opening else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            opening,
            format!("{path}.bindings[{binding_index}].value.opening"),
        )));
    };
    let declaration = ledger_fields
        .get(field.as_str())
        .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
    if declaration.declaration
        != (LedgerFieldKind::Cell {
            ty: Type::Bytes { length: 32 },
        })
        || declaration.index != *index
    {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            opening,
            format!("{path}.bindings[{binding_index}].value.opening"),
        )));
    }
    let Some(field_value) = field_expression(
        value,
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
            value,
            format!("{path}.bindings[{binding_index}].value.value"),
        )));
    };
    let slot = ident(field)?;
    let opening_value = syn::Ident::new(
        &format!("__compact_recorded_opening_{}", *next_temp),
        Span::call_site(),
    );
    *next_temp += 1;
    let committed = syn::Ident::new(
        &format!("__compact_recorded_commitment_{}", *next_temp),
        Span::call_site(),
    );
    *next_temp += 1;
    steps.push(syn::parse_quote! {
        let (frame, #opening_value): (_, runtime::FixedBytes<32>) =
            crate::ledger_slots::#slot.record_read(frame)?;
    });
    steps.push(syn::parse_quote! {
        let #committed: runtime::FixedBytes<32> =
            runtime::persistent_commit(#field_value, #opening_value);
    });
    scoped.insert(binding.name.clone(), syn::parse_quote!(#committed));
    Ok(RecordingOutcome::Supported(()))
}
