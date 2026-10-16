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

//! List observation returns after parent admission.

use crate::ir::{LedgerField, LedgerFieldKind, StateReturn, Type};
use crate::recorded::{RecordingGap, RecordingOutcome};
use crate::{RenderError, ident, list_head_result_type};
use std::collections::HashMap;

pub(in crate::recorded) fn length(
    field: &str,
    index: u8,
    return_value: &StateReturn,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let declaration = ledger_fields
        .get(field)
        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
    if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
        || declaration.index != index
    {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    }
    let slot = ident(field)?;
    Ok(RecordingOutcome::Supported((
        vec![syn::parse_quote!(
            let (frame, observed): (_, u64) =
                crate::ledger_slots::#slot.record_length(frame)?;
        )],
        syn::parse_quote!(
            runtime::BoundedUint::<18446744073709551615>::new(observed as u128)
                .expect("ledger List length fits Uint<64>")
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
    if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
        || declaration.index != index
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

pub(in crate::recorded) fn head(
    field: &str,
    index: u8,
    return_value: &StateReturn,
    result_type: &Type,
    result_ty: &syn::Type,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let declaration = ledger_fields
        .get(field)
        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
    let LedgerFieldKind::List { ty } = &declaration.declaration else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    };
    let expected = list_head_result_type(ty, result_type);
    if declaration.index != index || *result_type != expected {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    }
    let slot = ident(field)?;
    Ok(RecordingOutcome::Supported((
        vec![syn::parse_quote!(
            let (frame, observed): (_, #result_ty) =
                crate::ledger_slots::#slot.record_head::<#result_ty, _, _>(frame)?;
        )],
        syn::parse_quote!(observed),
    )))
}
