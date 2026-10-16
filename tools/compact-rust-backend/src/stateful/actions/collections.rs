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

use super::ActionOutput;
use crate::ir::{LedgerField, LedgerFieldKind, PureCircuit, StateAction, Type};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite};
use crate::stateful::NativeExpressionContext;
use crate::{RenderError, ident};
use proc_macro2::Span;
use std::collections::HashMap;

pub(super) fn lower(
    action: &StateAction,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    circuits: &HashMap<&str, &PureCircuit>,
    statements: &mut Vec<syn::Stmt>,
) -> Result<(), RenderError> {
    let witnesses = HashMap::new();
    let stateful_circuits = HashMap::new();
    let environment = NativeExpressionContext {
        witnesses: &witnesses,
        circuits,
        stateful_circuits: &stateful_circuits,
        ledger_fields,
        observer: None,
    };
    lower_observed(
        action,
        parameters,
        &environment,
        ActionOutput {
            statements,
            next_temp: &mut 0,
            uses_witness: &mut false,
        },
    )
}

pub(super) fn lower_observed(
    action: &StateAction,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    environment: &NativeExpressionContext<'_, '_>,
    output: ActionOutput<'_>,
) -> Result<(), RenderError> {
    let ActionOutput { statements, .. } = output;
    let ledger_fields = environment.ledger_fields;
    match action {
        StateAction::ListPushFront {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                RenderError::UnknownLedgerField(field.clone())
            })?;
            let LedgerFieldKind::List { ty } = &declaration.declaration else {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                refused(
                    RefusalSite::field(Owner::Action(action), "index"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let original_value = value;
            let (value, actual) = environment.pure_expression(value, parameters)?;
            if &actual != ty {
                refused(
                    RefusalSite::node(Owner::Expr(original_value)),
                    environment.observer,
                );
                return Err(RenderError::TypeMismatch {
                    expected: ty.clone(),
                    actual,
                });
            }
            let slot = ledger_ident(declaration, environment.observer)?;
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
            let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                RenderError::UnknownLedgerField(field.clone())
            })?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
            {
                refused(
                    RefusalSite::node(Owner::Action(action)),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ledger_ident(declaration, environment.observer)?;
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
            let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                RenderError::UnknownLedgerField(field.clone())
            })?;
            let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                refused(
                    RefusalSite::field(Owner::Action(action), "index"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let original_value = value;
            let (value, actual) = environment.pure_expression(value, parameters)?;
            if &actual != ty {
                refused(
                    RefusalSite::node(Owner::Expr(original_value)),
                    environment.observer,
                );
                return Err(RenderError::TypeMismatch {
                    expected: ty.clone(),
                    actual,
                });
            }
            let slot = ledger_ident(declaration, environment.observer)?;
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
            let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                RenderError::UnknownLedgerField(field.clone())
            })?;
            if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                || declaration.index != *index
            {
                refused(
                    RefusalSite::node(Owner::Action(action)),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ledger_ident(declaration, environment.observer)?;
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
            let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                RenderError::UnknownLedgerField(field.clone())
            })?;
            let LedgerFieldKind::Map {
                key: key_ty,
                value: value_ty,
            } = &declaration.declaration
            else {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                refused(
                    RefusalSite::field(Owner::Action(action), "index"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let original_key = key;
            let (key, actual_key) = environment.pure_expression(key, parameters)?;
            if &actual_key != key_ty {
                refused(
                    RefusalSite::node(Owner::Expr(original_key)),
                    environment.observer,
                );
                return Err(RenderError::TypeMismatch {
                    expected: key_ty.clone(),
                    actual: actual_key,
                });
            }
            let original_value = value;
            let (value, actual_value) = environment.pure_expression(value, parameters)?;
            if &actual_value != value_ty {
                refused(
                    RefusalSite::node(Owner::Expr(original_value)),
                    environment.observer,
                );
                return Err(RenderError::TypeMismatch {
                    expected: value_ty.clone(),
                    actual: actual_value,
                });
            }
            let slot = ledger_ident(declaration, environment.observer)?;
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
            let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                RenderError::UnknownLedgerField(field.clone())
            })?;
            let LedgerFieldKind::Map { key: key_ty, .. } = &declaration.declaration else {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                refused(
                    RefusalSite::field(Owner::Action(action), "index"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let original_key = key;
            let (key, actual_key) = environment.pure_expression(key, parameters)?;
            if &actual_key != key_ty {
                refused(
                    RefusalSite::node(Owner::Expr(original_key)),
                    environment.observer,
                );
                return Err(RenderError::TypeMismatch {
                    expected: key_ty.clone(),
                    actual: actual_key,
                });
            }
            let slot = ledger_ident(declaration, environment.observer)?;
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
            let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                RenderError::UnknownLedgerField(field.clone())
            })?;
            let LedgerFieldKind::Map { key: key_ty, .. } = &declaration.declaration else {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                refused(
                    RefusalSite::field(Owner::Action(action), "index"),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let original_key = key;
            let (key, actual_key) = environment.pure_expression(key, parameters)?;
            if &actual_key != key_ty {
                refused(
                    RefusalSite::node(Owner::Expr(original_key)),
                    environment.observer,
                );
                return Err(RenderError::TypeMismatch {
                    expected: key_ty.clone(),
                    actual: actual_key,
                });
            }
            let slot = ledger_ident(declaration, environment.observer)?;
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
            let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                RenderError::UnknownLedgerField(field.clone())
            })?;
            if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                || declaration.index != *index
            {
                refused(
                    RefusalSite::node(Owner::Action(action)),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ledger_ident(declaration, environment.observer)?;
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

fn ledger_ident(
    declaration: &LedgerField,
    observer: Option<&dyn RefusalObserver>,
) -> Result<syn::Ident, RenderError> {
    ident(&declaration.id).inspect_err(|_| {
        if let Some(observer) = observer {
            observer.refused(RefusalSite::field(Owner::Ledger(declaration), "id"));
        }
    })
}

fn refused(site: RefusalSite<'_>, observer: Option<&dyn RefusalObserver>) {
    if let Some(observer) = observer {
        observer.refused(site);
    }
}

#[cfg(test)]
mod guard_tests;
