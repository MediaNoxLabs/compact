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

//! Native plain and historic Merkle mutations with typed values and positions.
//!
//! The traversal owns lexical scope and scheduling. This domain only appends
//! the selected action's statements and updates its explicit effect state.

use crate::ir::{
    LedgerField, LedgerFieldKind, PureCircuit, StateAction, StatefulCircuit, Type,
    WitnessDeclaration,
};
use crate::stateful::expression::render_state_expression;
use crate::{RenderError, ident};
use proc_macro2::Span;
use std::collections::HashMap;

#[expect(
    clippy::too_many_arguments,
    reason = "native action lowering keeps declaration scope and ordered effect state explicit"
)]
pub(super) fn lower(
    action: &StateAction,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
    statements: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    uses_witness: &mut bool,
) -> Result<(), RenderError> {
    match action {
        StateAction::HistoricMerkleResetHistory { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(
                declaration.declaration,
                LedgerFieldKind::HistoricMerkleTree { .. }
            ) || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.reset_history(context)?;
            });
            statements.push(syn::parse_quote!(let context = step.context;));
            statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
        }
        StateAction::HistoricMerkleResetToDefault { field, index }
        | StateAction::MerkleResetToDefault { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match declaration.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                LedgerFieldKind::MerkleTree { .. } => false,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            let action_historic =
                matches!(action, StateAction::HistoricMerkleResetToDefault { .. });
            if declaration.index != *index || historic != action_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.reset_to_default(context)?;
            });
            statements.push(syn::parse_quote!(let context = step.context;));
            statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
        }
        StateAction::HistoricMerkleInsert {
            field,
            index,
            value,
        }
        | StateAction::MerkleInsert {
            field,
            index,
            value,
        }
        | StateAction::HistoricMerkleInsertIndex {
            field,
            index,
            value,
            ..
        }
        | StateAction::MerkleInsertIndex {
            field,
            index,
            value,
            ..
        }
        | StateAction::HistoricMerkleInsertHash {
            field,
            index,
            hash: value,
        }
        | StateAction::MerkleInsertHash {
            field,
            index,
            hash: value,
        }
        | StateAction::HistoricMerkleInsertHashIndex {
            field,
            index,
            hash: value,
            ..
        }
        | StateAction::MerkleInsertHashIndex {
            field,
            index,
            hash: value,
            ..
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let (ty, historic) = match &declaration.declaration {
                LedgerFieldKind::HistoricMerkleTree { ty, .. } => (ty, true),
                LedgerFieldKind::MerkleTree { ty, .. } => (ty, false),
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            let action_historic = matches!(
                action,
                StateAction::HistoricMerkleInsert { .. }
                    | StateAction::HistoricMerkleInsertIndex { .. }
                    | StateAction::HistoricMerkleInsertHash { .. }
                    | StateAction::HistoricMerkleInsertHashIndex { .. }
            );
            if declaration.index != *index || historic != action_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let mut value_statements = Vec::new();
            let mut query_effect = false;
            let (value, actual, value_witness_effect) = render_state_expression(
                value,
                parameters,
                witnesses,
                &mut value_statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                &mut query_effect,
            )?;
            let is_hash = matches!(
                action,
                StateAction::HistoricMerkleInsertHash { .. }
                    | StateAction::HistoricMerkleInsertHashIndex { .. }
                    | StateAction::MerkleInsertHash { .. }
                    | StateAction::MerkleInsertHashIndex { .. }
            );
            let expected = if is_hash {
                Type::Bytes { length: 32 }
            } else {
                ty.clone()
            };
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            let mut witness_effect = value_witness_effect;
            let position = match action {
                StateAction::HistoricMerkleInsertIndex { position, .. }
                | StateAction::HistoricMerkleInsertHashIndex { position, .. }
                | StateAction::MerkleInsertIndex { position, .. }
                | StateAction::MerkleInsertHashIndex { position, .. } => {
                    let (rendered, actual, effect) = render_state_expression(
                        position,
                        parameters,
                        witnesses,
                        &mut value_statements,
                        next_temp,
                        circuits,
                        stateful_circuits,
                        ledger_fields,
                        &mut query_effect,
                    )?;
                    let expected = Type::Unsigned {
                        max: u64::MAX.to_string(),
                    };
                    if actual != expected {
                        return Err(RenderError::TypeMismatch { expected, actual });
                    }
                    witness_effect |= effect;
                    Some(rendered)
                }
                _ => None,
            };
            *uses_witness |= witness_effect;
            if witness_effect || query_effect {
                statements.push(syn::parse_quote!(let mut context = context;));
            }
            statements.extend(value_statements);
            let slot = ident(&declaration.id)?;
            let method = match (is_hash, position.is_some()) {
                (false, false) => "insert",
                (false, true) => "insert_index",
                (true, false) => "insert_hash",
                (true, true) => "insert_hash_index",
            };
            let method = syn::Ident::new(method, Span::call_site());
            if let Some(position) = position {
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.#method(context, #value, #position)?;
                });
            } else {
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.#method(context, #value)?;
                });
            }
            statements.push(syn::parse_quote!(let context = step.context;));
            statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
        }
        StateAction::HistoricMerkleInsertIndexDefault {
            field,
            index,
            position,
        }
        | StateAction::MerkleInsertIndexDefault {
            field,
            index,
            position,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match &declaration.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                LedgerFieldKind::MerkleTree { .. } => false,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            let action_historic =
                matches!(action, StateAction::HistoricMerkleInsertIndexDefault { .. });
            if declaration.index != *index || historic != action_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let mut value_statements = Vec::new();
            let mut query_effect = false;
            let (position, actual, witness_effect) = render_state_expression(
                position,
                parameters,
                witnesses,
                &mut value_statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                &mut query_effect,
            )?;
            let expected = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            if witness_effect {
                *uses_witness = true;
            }
            if witness_effect || query_effect {
                statements.push(syn::parse_quote!(let mut context = context;));
            }
            statements.extend(value_statements);
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.insert_index_default(context, #position)?;
            });
            statements.push(syn::parse_quote!(let context = step.context;));
            statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
        }
        _ => unreachable!("native merkle action dispatched to its declared domain"),
    }
    Ok(())
}
