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

//! Ordered coin and recipient evaluation for qualified Cell and Set writes.
//!
//! The traversal owns lexical scope and scheduling. This domain only appends
//! the selected action's statements and updates its explicit effect state.

use super::ActionOutput;
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};
use crate::ir::{
    LedgerField, LedgerFieldKind, PureCircuit, StateAction, StatefulCircuit, Type,
    WitnessDeclaration,
};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite};
use crate::stateful::{NativeExpressionContext, render_state_expression_observed};
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
    let environment = NativeExpressionContext {
        witnesses,
        circuits,
        stateful_circuits,
        ledger_fields,
        observer: None,
    };
    lower_observed(
        action,
        parameters,
        &environment,
        ActionOutput {
            statements,
            next_temp,
            uses_witness,
        },
    )
}

pub(super) fn lower_observed(
    action: &StateAction,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    environment: &NativeExpressionContext<'_, '_>,
    output: ActionOutput<'_>,
) -> Result<(), RenderError> {
    let ActionOutput {
        statements,
        next_temp,
        uses_witness,
    } = output;
    let ledger_fields = environment.ledger_fields;
    match action {
        StateAction::SetInsertCoin {
            field,
            index,
            coin,
            recipient,
        }
        | StateAction::CellWriteCoin {
            field,
            index,
            coin,
            recipient,
        } => {
            let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                refused(
                    RefusalSite::field(Owner::Action(action), "field"),
                    environment.observer,
                );
                RenderError::UnknownLedgerField(field.clone())
            })?;
            let expected = qualified_coin_type();
            let cell_write = matches!(action, StateAction::CellWriteCoin { .. });
            let expected_kind = if cell_write {
                LedgerFieldKind::Cell { ty: expected }
            } else {
                LedgerFieldKind::Set { ty: expected }
            };
            if declaration.declaration != expected_kind || declaration.index != *index {
                refused(
                    RefusalSite::node(Owner::Action(action)),
                    environment.observer,
                );
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let mut operands = Vec::new();
            let mut query_effect = false;
            let (coin_value, coin_ty, coin_witness) = render_state_expression_observed(
                coin,
                parameters,
                &mut operands,
                next_temp,
                &mut query_effect,
                environment,
            )?;
            let expected_coin = shielded_coin_type();
            if coin_ty != expected_coin {
                refused(RefusalSite::node(Owner::Expr(coin)), environment.observer);
                return Err(RenderError::TypeMismatch {
                    expected: expected_coin,
                    actual: coin_ty,
                });
            }
            let coin_name =
                syn::Ident::new(&format!("__compact_coin_{}", next_temp), Span::call_site());
            *next_temp += 1;
            operands.push(syn::parse_quote!(let #coin_name = #coin_value;));
            let (recipient_value, recipient_ty, recipient_witness) =
                render_state_expression_observed(
                    recipient,
                    parameters,
                    &mut operands,
                    next_temp,
                    &mut query_effect,
                    environment,
                )?;
            let expected_recipient = shielded_recipient_type();
            if recipient_ty != expected_recipient {
                refused(
                    RefusalSite::node(Owner::Expr(recipient)),
                    environment.observer,
                );
                return Err(RenderError::TypeMismatch {
                    expected: expected_recipient,
                    actual: recipient_ty,
                });
            }
            let recipient_name = syn::Ident::new(
                &format!("__compact_recipient_{}", next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            operands.push(syn::parse_quote!(let #recipient_name = #recipient_value;));
            if coin_witness || recipient_witness {
                *uses_witness = true;
            }
            if coin_witness || recipient_witness || query_effect {
                statements.push(syn::parse_quote!(let mut context = context;));
            }
            statements.extend(operands);
            let slot = ledger_ident(declaration, environment.observer)?;
            let method = ident(if cell_write {
                "write_coin"
            } else {
                "insert_coin"
            })?;
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.#method(
                    context,
                    runtime::ledger::coin_info_from_compact(
                        #coin_name.nonce,
                        #coin_name.color,
                        #coin_name.value.value(),
                    ),
                    runtime::ledger::coin_recipient_from_compact(
                        #recipient_name.is_left,
                        #recipient_name.left.bytes,
                        #recipient_name.right.bytes,
                    ),
                )?;
            });
            statements.push(syn::parse_quote!(let context = step.context;));
            statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
        }
        _ => unreachable!("native coins action dispatched to its declared domain"),
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
