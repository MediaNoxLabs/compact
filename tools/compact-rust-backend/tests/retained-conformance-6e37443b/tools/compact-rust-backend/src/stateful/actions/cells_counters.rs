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

//! Native Cell writes and Counter operations with typed operands.
//!
//! The traversal owns lexical scope and scheduling. This domain only appends
//! the selected action's statements and updates its explicit effect state.

use crate::ir::{
    CounterAmount, LedgerField, LedgerFieldKind, PureCircuit, StateAction, StatefulCircuit, Type,
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
            let slot = ident(&declaration.id)?;
            let amount: syn::Expr = match amount {
                CounterAmount::Literal { value } => {
                    let value = syn::LitInt::new(&value.to_string(), Span::call_site());
                    syn::parse_quote!(#value)
                }
                CounterAmount::Parameter { name } => {
                    let (ty, rust_name) = parameters
                        .get(name.as_str())
                        .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
                    let expected = Type::Unsigned {
                        max: "65535".into(),
                    };
                    if *ty != &expected {
                        return Err(RenderError::TypeMismatch {
                            expected,
                            actual: (*ty).clone(),
                        });
                    }
                    syn::parse_quote!(#rust_name.value() as u16)
                }
            };
            let method = if matches!(action, StateAction::CounterIncrement { .. }) {
                syn::Ident::new("increment", Span::call_site())
            } else {
                syn::Ident::new("decrement", Span::call_site())
            };
            statements.push(syn::parse_quote! {
                let step = crate::ledger_slots::#slot.#method(context, #amount)?;
            });
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
        }
        StateAction::CounterReset { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
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
        StateAction::CellWrite {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let mut value_statements = Vec::new();
            let mut query_effect = false;
            let (value, actual, witness_effect) = render_state_expression(
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
            if &actual != ty {
                return Err(RenderError::TypeMismatch {
                    expected: ty.clone(),
                    actual,
                });
            }
            if witness_effect {
                *uses_witness = true;
            }
            if witness_effect || query_effect {
                statements.push(syn::parse_quote!(let mut context = context;));
            }
            statements.extend(value_statements);
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote!(
                let step = crate::ledger_slots::#slot.write(context, #value)?;
            ));
            statements.push(syn::parse_quote! {
                let context = step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += step.gas_cost;
            });
        }
        _ => unreachable!("native cells_counters action dispatched to its declared domain"),
    }
    Ok(())
}
