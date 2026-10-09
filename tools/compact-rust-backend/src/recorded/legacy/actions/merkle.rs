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

//! Plain and historic Merkle actions with existing path and operand checks.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::ir::{LedgerField, LedgerFieldKind, StateAction, Type};
use crate::recorded::legacy::value_sources::cell_source;
use crate::recorded::{RecordingOutcome, unavailable_action};
use crate::{RenderError, ident};
use std::collections::HashMap;

// This family retains the original lowering arms and ordered recording state.
pub(in crate::recorded) fn append_merkle_steps(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    steps: &mut Vec<syn::Stmt>,
) -> Result<RecordingOutcome<()>, RenderError> {
    match action {
        StateAction::HistoricMerkleResetHistory { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(
                declaration.declaration,
                LedgerFieldKind::HistoricMerkleTree { .. }
            ) || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(unavailable_action(action, path));
            }
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_reset_history(frame)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::HistoricMerkleResetToDefault { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(
                declaration.declaration,
                LedgerFieldKind::HistoricMerkleTree { .. }
            ) || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(unavailable_action(action, path));
            }
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_reset_to_default(frame)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::MerkleInsert {
            field,
            index,
            value,
        }
        | StateAction::HistoricMerkleInsert {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let (ty, historic) = match &declaration.declaration {
                LedgerFieldKind::MerkleTree { ty, .. } => (ty, false),
                LedgerFieldKind::HistoricMerkleTree { ty, .. } => (ty, true),
                _ => {
                    return Ok(unavailable_action(action, path));
                }
            };
            if declaration.index != *index
                || declaration.physical_path().len() != 1
                || historic != matches!(action, StateAction::HistoricMerkleInsert { .. })
            {
                return Ok(unavailable_action(action, path));
            }
            let Some(value) = cell_source(value, ty, locals, parameters) else {
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_insert(frame, #value)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::MerkleResetToDefault { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::MerkleTree { .. })
                || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(unavailable_action(action, path));
            }
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_reset_to_default(frame)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::MerkleInsertHash { field, index, hash } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::MerkleTree { .. })
                || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(unavailable_action(action, path));
            }
            let Some(hash) = cell_source(hash, &Type::Bytes { length: 32 }, locals, parameters)
            else {
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_insert_hash(frame, #hash)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::HistoricMerkleInsertHash { field, index, hash } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(
                declaration.declaration,
                LedgerFieldKind::HistoricMerkleTree { .. }
            ) || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(unavailable_action(action, path));
            }
            let Some(hash) = cell_source(hash, &Type::Bytes { length: 32 }, locals, parameters)
            else {
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_insert_hash(frame, #hash)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::MerkleInsertHashIndex {
            field,
            index,
            hash,
            position,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::MerkleTree { .. })
                || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(unavailable_action(action, path));
            }
            let position_ty = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            let (Some(hash), Some(position)) = (
                cell_source(hash, &Type::Bytes { length: 32 }, locals, parameters),
                cell_source(position, &position_ty, locals, parameters),
            ) else {
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_insert_hash_index(frame, #hash, #position)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::HistoricMerkleInsertHashIndex {
            field,
            index,
            hash,
            position,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(
                declaration.declaration,
                LedgerFieldKind::HistoricMerkleTree { .. }
            ) || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(unavailable_action(action, path));
            }
            let position_ty = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            let (Some(hash), Some(position)) = (
                cell_source(hash, &Type::Bytes { length: 32 }, locals, parameters),
                cell_source(position, &position_ty, locals, parameters),
            ) else {
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_insert_hash_index(frame, #hash, #position)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::MerkleInsertIndex {
            field,
            index,
            value,
            position,
        }
        | StateAction::HistoricMerkleInsertIndex {
            field,
            index,
            value,
            position,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let (ty, historic) = match &declaration.declaration {
                LedgerFieldKind::MerkleTree { ty, .. } => (ty, false),
                LedgerFieldKind::HistoricMerkleTree { ty, .. } => (ty, true),
                _ => return Ok(unavailable_action(action, path)),
            };
            if declaration.index != *index
                || declaration.physical_path().len() != 1
                || historic != matches!(action, StateAction::HistoricMerkleInsertIndex { .. })
            {
                return Ok(unavailable_action(action, path));
            }
            let position_ty = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            let (Some(value), Some(position)) = (
                cell_source(value, ty, locals, parameters),
                cell_source(position, &position_ty, locals, parameters),
            ) else {
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_insert_index(frame, #value, #position)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        StateAction::MerkleInsertIndexDefault {
            field,
            index,
            position,
        }
        | StateAction::HistoricMerkleInsertIndexDefault {
            field,
            index,
            position,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match &declaration.declaration {
                LedgerFieldKind::MerkleTree { .. } => false,
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                _ => return Ok(unavailable_action(action, path)),
            };
            if declaration.index != *index
                || declaration.physical_path().len() != 1
                || historic
                    != matches!(action, StateAction::HistoricMerkleInsertIndexDefault { .. })
            {
                return Ok(unavailable_action(action, path));
            }
            let position_ty = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            let Some(position) = cell_source(position, &position_ty, locals, parameters) else {
                return Ok(unavailable_action(action, path));
            };
            let slot = ident(field)?;
            steps.push(syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_insert_index_default(frame, #position)?;
            ));
            Ok(RecordingOutcome::Supported(()))
        }
        _ => Ok(unavailable_action(action, path)),
    }
}
