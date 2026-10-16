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

//! Map observation returns after parent admission.
//! Operand lowering preserves the original empty local scope and shared temporary state.

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::legacy::expressions::scalar_expression;
use crate::recorded::{RecordingGap, RecordingOutcome};
use crate::{RenderError, ident};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

#[expect(
    clippy::too_many_arguments,
    reason = "collection observation keeps operands, declarations, and mutable recording state explicit"
)]
pub(in crate::recorded) fn member_or_lookup(
    field: &str,
    index: u8,
    key: &Expr,
    return_value: &StateReturn,
    result_type: &Type,
    result_ty: &syn::Type,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let declaration = ledger_fields
        .get(field)
        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
    let LedgerFieldKind::Map {
        key: key_ty,
        value: value_ty,
    } = &declaration.declaration
    else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    };
    let member = matches!(return_value, StateReturn::MapMember { .. });
    let expected = if member {
        Type::Boolean
    } else {
        value_ty.clone()
    };
    if declaration.index != index
        || *result_type != expected
        || !matches!(expected, Type::Boolean | Type::Field)
    {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    }
    let mut return_steps = Vec::new();
    let key = scalar_expression(
        key,
        key_ty,
        &HashMap::new(),
        parameters,
        ledger_fields,
        witnesses,
        circuits,
        &mut return_steps,
        next_temp,
        visiting,
    )?;
    let Some(key) = key else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    };
    let slot = ident(field)?;
    let method = if member {
        "record_member"
    } else {
        "record_lookup"
    };
    let method = syn::Ident::new(method, Span::call_site());
    return_steps.push(syn::parse_quote!(
        let (frame, observed): (_, #result_ty) =
            crate::ledger_slots::#slot.#method(frame, #key)?;
    ));
    Ok(RecordingOutcome::Supported((
        return_steps,
        syn::parse_quote!(observed),
    )))
}

pub(in crate::recorded) fn size(
    field: &str,
    index: u8,
    return_value: &StateReturn,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let declaration = ledger_fields
        .get(field)
        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
    if !matches!(declaration.declaration, LedgerFieldKind::Map { .. }) || declaration.index != index
    {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    }
    let slot = ident(field)?;
    Ok(RecordingOutcome::Supported((
        vec![syn::parse_quote!(
            let (frame, observed): (_, u64) =
                crate::ledger_slots::#slot.record_size(frame)?;
        )],
        syn::parse_quote!(
            runtime::BoundedUint::<18446744073709551615>::new(observed as u128)
                .expect("ledger Map size fits Uint<64>")
        ),
    )))
}

pub(in crate::recorded) fn is_empty(
    field: &str,
    index: u8,
    return_value: &StateReturn,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let declaration = ledger_fields
        .get(field)
        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
    if !matches!(declaration.declaration, LedgerFieldKind::Map { .. }) || declaration.index != index
    {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    }
    let slot = ident(field)?;
    Ok(RecordingOutcome::Supported((
        vec![syn::parse_quote!(
            let (frame, observed): (_, bool) =
                crate::ledger_slots::#slot.record_is_empty(frame)?;
        )],
        syn::parse_quote!(observed),
    )))
}
