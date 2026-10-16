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

//! Native List, Set, and Map mutations with their existing pure operand rules.
//!
//! The traversal owns lexical scope and scheduling. This domain only appends
//! the selected action's statements and updates its explicit effect state.

use crate::ir::{LedgerField, LedgerFieldKind, PureCircuit, StateAction, Type};
use crate::{RenderError, expression_with_calls, ident};
use proc_macro2::Span;
use std::collections::HashMap;

pub(super) fn lower(
    action: &StateAction,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    circuits: &HashMap<&str, &PureCircuit>,
    statements: &mut Vec<syn::Stmt>,
) -> Result<(), RenderError> {
    match action {
        StateAction::ListPushFront {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::List { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if &actual != ty {
                return Err(RenderError::TypeMismatch {
                    expected: ty.clone(),
                    actual,
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.push_front(context, #value)?;
            });
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
        }
        StateAction::ListPopFront { field, index } | StateAction::ListReset { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(&declaration.id)?;
            let method = if matches!(action, StateAction::ListPopFront { .. }) {
                "pop_front"
            } else {
                "reset"
            };
            let method = syn::Ident::new(method, Span::call_site());
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.#method(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
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
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if &actual != ty {
                return Err(RenderError::TypeMismatch {
                    expected: ty.clone(),
                    actual,
                });
            }
            let slot = ident(&declaration.id)?;
            let method = if matches!(action, StateAction::SetInsert { .. }) {
                syn::Ident::new("insert", Span::call_site())
            } else {
                syn::Ident::new("remove", Span::call_site())
            };
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.#method(context, #value)?;
            });
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
        }
        StateAction::SetReset { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.reset(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
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
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let (key, actual_key) = expression_with_calls(key, parameters, circuits)?;
            if &actual_key != key_ty {
                return Err(RenderError::TypeMismatch {
                    expected: key_ty.clone(),
                    actual: actual_key,
                });
            }
            let (value, actual_value) = expression_with_calls(value, parameters, circuits)?;
            if &actual_value != value_ty {
                return Err(RenderError::TypeMismatch {
                    expected: value_ty.clone(),
                    actual: actual_value,
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.insert(context, #key, #value)?;
            });
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
        }
        StateAction::MapInsertDefault { field, index, key } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Map { key: key_ty, .. } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let (key, actual_key) = expression_with_calls(key, parameters, circuits)?;
            if &actual_key != key_ty {
                return Err(RenderError::TypeMismatch {
                    expected: key_ty.clone(),
                    actual: actual_key,
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.insert_default(context, #key)?;
            });
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
        }
        StateAction::MapRemove { field, index, key } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Map { key: key_ty, .. } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let (key, actual_key) = expression_with_calls(key, parameters, circuits)?;
            if &actual_key != key_ty {
                return Err(RenderError::TypeMismatch {
                    expected: key_ty.clone(),
                    actual: actual_key,
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.remove(context, #key)?;
            });
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
        }
        StateAction::MapReset { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.reset(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
        }
        _ => unreachable!("native collections action dispatched to its declared domain"),
    }
    Ok(())
}
