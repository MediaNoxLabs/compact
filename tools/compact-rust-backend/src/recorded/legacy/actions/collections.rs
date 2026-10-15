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

//! Cell, Counter, Set, Map, and List action lowering with existing typed admission.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::ir::{
    CounterAmount, Expr, LedgerField, LedgerFieldKind, StateAction, StatefulCircuit, Type,
    WitnessDeclaration,
};
use crate::recorded::legacy::expressions::{
    boolean_expression, field_expression, scalar_expression,
};
use crate::recorded::legacy::value_sources::{amount_source, cell_source};
use crate::recorded::{RecordingGap, RecordingOutcome, recordable_cell_type, unavailable_action};
use crate::{RenderError, ident};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

// This family retains the original lowering arms and ordered recording state.
#[expect(
    clippy::too_many_arguments,
    reason = "legacy lowering keeps declarations, scope, and ordered recording state explicit"
)]
pub(in crate::recorded) fn append_collection_steps(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<RecordingOutcome<()>, RenderError> {
    match action {
        StateAction::CounterIncrement {
            field,
            index,
            amount,
        }
        | StateAction::CounterDecrement {
            field,
            index,
            amount,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(field)?;
            let amount: syn::Expr = match amount {
                CounterAmount::Literal { value } => {
                    let literal = syn::LitInt::new(&format!("{value}u16"), Span::call_site());
                    syn::parse_quote!(#literal)
                }
                CounterAmount::Parameter { name } => {
                    let Some(value) =
                        amount_source(&Expr::Parameter { name: name.clone() }, locals, parameters)
                    else {
                        return Ok(unavailable_action(action, path));
                    };
                    value
                }
            };
            let method = if matches!(action, StateAction::CounterIncrement { .. }) {
                syn::Ident::new("record_increment", Span::call_site())
            } else {
                syn::Ident::new("record_decrement", Span::call_site())
            };
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.#method(frame, #amount)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::CounterReset { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_reset(frame)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::SetInsert {
            field,
            index,
            value,
        }
        | StateAction::SetRemove {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                return Ok(unavailable_action(action, path));
            };
            if declaration.index != *index {
                return Ok(unavailable_action(action, path));
            }
            if !matches!(
                ty,
                Type::Field
                    | Type::Boolean
                    | Type::Enum { .. }
                    | Type::Tuple { .. }
                    | Type::Struct { .. }
                    | Type::Vector { .. }
            ) && !(*ty == Type::OpaqueString && matches!(action, StateAction::SetInsert { .. }))
            {
                return Ok(unavailable_action(action, path));
            }
            let value = scalar_expression(
                value,
                ty,
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
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            let method = if matches!(action, StateAction::SetInsert { .. }) {
                syn::Ident::new("record_insert", Span::call_site())
            } else {
                syn::Ident::new("record_remove", Span::call_site())
            };
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.#method(frame, #value)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::SetReset { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(unavailable_action(action, path));
            }
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_reset(frame)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::ListPushFront {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::List { ty } = &declaration.declaration else {
                return Ok(unavailable_action(action, path));
            };
            if declaration.index != *index {
                return Ok(unavailable_action(action, path));
            }
            let value = scalar_expression(
                value,
                ty,
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
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_push_front(frame, #value)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::ListPopFront { field, index } | StateAction::ListReset { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
            {
                return Ok(unavailable_action(action, path));
            }
            let slot = ident(field)?;
            let method = if matches!(action, StateAction::ListPopFront { .. }) {
                syn::Ident::new("record_pop_front", Span::call_site())
            } else {
                syn::Ident::new("record_reset", Span::call_site())
            };
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.#method(frame)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::MapInsert {
            field,
            index,
            key,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Map {
                key: key_ty,
                value: value_ty,
            } = &declaration.declaration
            else {
                return Ok(unavailable_action(action, path));
            };
            if declaration.index != *index {
                return Ok(unavailable_action(action, path));
            }
            let key = scalar_expression(
                key,
                key_ty,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            )?;
            let Some(key) = key else {
                return Ok(unavailable_action(action, path));
            };
            let key_name = syn::Ident::new(
                &format!("__compact_recorded_key_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(let #key_name = #key;));
            let value = scalar_expression(
                value,
                value_ty,
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
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_insert(frame, #key_name, #value)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::MapInsertDefault { field, index, key }
        | StateAction::MapRemove { field, index, key } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Map {
                key: key_ty,
                value: value_ty,
            } = &declaration.declaration
            else {
                return Ok(unavailable_action(action, path));
            };
            if declaration.index != *index
                || (matches!(action, StateAction::MapInsertDefault { .. })
                    && !matches!(value_ty, Type::Field | Type::Boolean))
            {
                return Ok(unavailable_action(action, path));
            }
            let key = scalar_expression(
                key,
                key_ty,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            )?;
            let Some(key) = key else {
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            let method = if matches!(action, StateAction::MapInsertDefault { .. }) {
                syn::Ident::new("record_insert_default", Span::call_site())
            } else {
                syn::Ident::new("record_remove", Span::call_site())
            };
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.#method(frame, #key)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::MapReset { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                || declaration.index != *index
            {
                return Ok(unavailable_action(action, path));
            }
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_reset(frame)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::CellWrite {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                return Ok(unavailable_action(action, path));
            };
            if !recordable_cell_type(ty) {
                return Ok(RecordingOutcome::Unsupported(
                    RecordingGap::unsupported_type(action, ty, format!("{path}.value")),
                ));
            }
            if declaration.index != *index {
                return Ok(unavailable_action(action, path));
            }
            let expression = value;
            let value = if *ty == Type::Field {
                field_expression(
                    value,
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
            } else if *ty == Type::Boolean {
                boolean_expression(
                    value,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?
            } else {
                cell_source(value, ty, locals, parameters)
            };
            let Some(value) = value else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    expression,
                    format!("{path}.value"),
                )));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_write(frame, #value)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        _ => Ok(unavailable_action(action, path)),
    }
}
