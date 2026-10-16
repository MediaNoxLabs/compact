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

//! Set observation returns after parent admission.
//! Operand lowering preserves the original empty local scope and shared temporary state.

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::legacy::expressions::boolean_expression;
use crate::recorded::{RecordingGap, RecordingOutcome};
use crate::{RenderError, ident};
use std::collections::{HashMap, HashSet};

#[expect(
    clippy::too_many_arguments,
    reason = "collection observation keeps operands, declarations, and mutable recording state explicit"
)]
pub(in crate::recorded) fn member(
    field: &str,
    index: u8,
    value: &Expr,
    return_value: &StateReturn,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let mut return_steps = Vec::new();
    let expression = Expr::SetMember {
        field: field.to_owned(),
        index,
        value: Box::new(value.clone()),
    };
    let Some(result) = boolean_expression(
        &expression,
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
    Ok(RecordingOutcome::Supported((return_steps, result)))
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
    if !matches!(declaration.declaration, LedgerFieldKind::Set { .. }) || declaration.index != index
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
                .expect("ledger Set size fits Uint<64>")
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
    if !matches!(declaration.declaration, LedgerFieldKind::Set { .. }) || declaration.index != index
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
