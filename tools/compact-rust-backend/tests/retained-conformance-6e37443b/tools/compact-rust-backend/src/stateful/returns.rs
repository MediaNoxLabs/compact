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

//! Final native return lowering after lexical action traversal has completed.
//!
//! Expression returns use the restored return scope. Specialized ledger queries
//! intentionally use the original parameter scope. Append effects to the current
//! statement stream and preserve its temporary counter and witness requirement.

use super::expression::render_state_expression;
use crate::ir::{
    LedgerField, LedgerFieldKind, PureCircuit, StateReturn, StatefulCircuit, StructField, Type,
    WitnessDeclaration,
};
use crate::{RenderError, expression_with_calls, ident, list_head_result_type};
use proc_macro2::Span;
use std::collections::HashMap;

#[expect(
    clippy::too_many_arguments,
    reason = "native returns keep declaration scopes and ordered statement, temporary, and witness state explicit"
)]
pub(super) fn lower(
    circuit: &StatefulCircuit,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    return_parameters: &HashMap<&str, (&Type, syn::Ident)>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    result_ty: &syn::Type,
    statements: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    uses_witness: &mut bool,
) -> Result<syn::Expr, RenderError> {
    let return_expr: syn::Expr = match &circuit.return_value {
        StateReturn::Effectful { .. } => syn::parse_quote!(__compact_effectful_result),
        StateReturn::Expression { value } => {
            let mut effect_statements = Vec::new();
            let mut query_effect = false;
            let (rendered, actual, effect) = render_state_expression(
                value,
                return_parameters,
                witnesses,
                &mut effect_statements,
                next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                &mut query_effect,
            )?;
            if actual != circuit.result {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual,
                });
            }
            if effect {
                *uses_witness = true;
            }
            if effect || query_effect {
                statements.push(syn::parse_quote!(let mut context = context;));
            }
            statements.extend(effect_statements);
            rendered
        }
        StateReturn::Unit => {
            if circuit.result != Type::Unit {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual: Type::Unit,
                });
            }
            syn::parse_quote!(())
        }
        StateReturn::CellRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if ty != &circuit.result {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual: ty.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote!(
                let read_step = crate::ledger_slots::#slot.read(context)?;
            ));
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::CounterRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.read(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger Counter fits Uint<64>"))
        }
        StateReturn::ListLength { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.length(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger List length fits Uint<64>"))
        }
        StateReturn::ListIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.is_empty(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::ListHead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::List { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = list_head_result_type(ty, &circuit.result);
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.head::<#result_ty, _, _>(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::HistoricMerkleIsFull { field, index }
        | StateReturn::MerkleIsFull { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match declaration.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                LedgerFieldKind::MerkleTree { .. } => false,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            let return_historic = matches!(
                circuit.return_value,
                StateReturn::HistoricMerkleIsFull { .. }
            );
            if declaration.index != *index || historic != return_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.is_full(context)?;
            });
            statements.push(syn::parse_quote!(let context = read_step.context;));
            statements.push(syn::parse_quote!(total_cost += read_step.gas_cost;));
            syn::parse_quote!(read_step.result)
        }
        StateReturn::HistoricMerkleCheckRoot { field, index, root }
        | StateReturn::MerkleCheckRoot { field, index, root } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match declaration.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                LedgerFieldKind::MerkleTree { .. } => false,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            let return_historic = matches!(
                circuit.return_value,
                StateReturn::HistoricMerkleCheckRoot { .. }
            );
            if declaration.index != *index || historic != return_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let expected = Type::Struct {
                name: "MerkleTreeDigest".into(),
                fields: vec![StructField {
                    name: "field".into(),
                    ty: Type::Field,
                }],
            };
            let (root, actual) = expression_with_calls(root, parameters, circuits)?;
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.check_root(context, #root)?;
            });
            statements.push(syn::parse_quote!(let context = read_step.context;));
            statements.push(syn::parse_quote!(total_cost += read_step.gas_cost;));
            syn::parse_quote!(read_step.result)
        }
        StateReturn::SetMember {
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
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
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
                let read_step = crate::ledger_slots::#slot.member(context, #value)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::SetSize { field, index }
        | StateReturn::SetIsEmpty { field, index }
        | StateReturn::MapSize { field, index }
        | StateReturn::MapIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let is_map = matches!(
                &circuit.return_value,
                StateReturn::MapSize { .. } | StateReturn::MapIsEmpty { .. }
            );
            let declaration_matches = if is_map {
                matches!(declaration.declaration, LedgerFieldKind::Map { .. })
            } else {
                matches!(declaration.declaration, LedgerFieldKind::Set { .. })
            };
            if !declaration_matches || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let is_size = matches!(
                &circuit.return_value,
                StateReturn::SetSize { .. } | StateReturn::MapSize { .. }
            );
            let expected = if is_size {
                Type::Unsigned {
                    max: u64::MAX.to_string(),
                }
            } else {
                Type::Boolean
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            let method = if is_size { "size" } else { "is_empty" };
            let method = syn::Ident::new(method, Span::call_site());
            statements.push(syn::parse_quote!(
                let read_step = crate::ledger_slots::#slot.#method(context)?;
            ));
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            if is_size {
                let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
                syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger collection size fits Uint<64>"))
            } else {
                syn::parse_quote!(read_step.result)
            }
        }
        StateReturn::MapMember { field, index, key }
        | StateReturn::MapLookup { field, index, key } => {
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
            let is_member = matches!(&circuit.return_value, StateReturn::MapMember { .. });
            let expected = if is_member {
                Type::Boolean
            } else {
                value_ty.clone()
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            if is_member {
                statements.push(syn::parse_quote! {
                    let read_step = crate::ledger_slots::#slot.member(context, #key)?;
                });
            } else {
                statements.push(syn::parse_quote! {
                    let read_step = crate::ledger_slots::#slot.lookup(context, #key)?;
                });
            }
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
    };
    Ok(return_expr)
}
