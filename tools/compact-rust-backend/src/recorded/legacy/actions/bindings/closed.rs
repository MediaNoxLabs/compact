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

//! Closed binding prefixes with typed projections and ordered continuations.
//! A nonmatching case returns None before mutation. A matched refusal is retained.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::ir::{
    Expr, LedgerField, PureCircuit, StateAction, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::legacy::actions::append_steps;
use crate::recorded::legacy::expressions::field_expression;
use crate::recorded::legacy::value_sources::{
    cell_source, closed_annotated_uint8_source, closed_conditional_field_pair,
    closed_counter_one_continuation, closed_curve_field_argument, closed_nested_uint4_source,
};
use crate::recorded::{LegacyActionPrecision, RecordingGap, RecordingOutcome};
use crate::{RenderError, ident, rust_type};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

#[expect(
    clippy::too_many_arguments,
    reason = "closed binding lowering preserves explicit declaration, scope, and recording state"
)]
pub(in crate::recorded) fn try_nested_uint64_cell_counter_binding(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<RecordingOutcome<()>>, RenderError> {
    let StateAction::Let {
        bindings,
        action: nested_action,
    } = action
    else {
        return Ok(None);
    };
    // The nested Uint<4> source also occurs before an exact
    // Uint<64> Cell projection and one Counter increment. Keep
    // this sibling of the Field projection bounded and ordered.
    if let [unsigned_binding] = bindings.as_slice()
        && unsigned_binding.ty == (Type::Unsigned { max: "4".into() })
        && let Some(selected) =
            closed_nested_uint4_source(&unsigned_binding.value, locals, parameters)
        && let StateAction::Sequence { actions } = nested_action.as_ref()
        && let [
            StateAction::Let {
                bindings: projected_bindings,
                action: projected_action,
            },
            continuation,
        ] = actions.as_slice()
        && let [projected_binding] = projected_bindings.as_slice()
        && projected_binding.ty
            == (Type::Unsigned {
                max: "18446744073709551615".into(),
            })
        && matches!(
            &projected_binding.value,
            Expr::UnsignedCast { max, value }
                if max == "18446744073709551615"
                    && matches!(value.as_ref(), Expr::Parameter { name } if name == &unsigned_binding.name)
        )
        && matches!(
            projected_action.as_ref(),
            StateAction::CellWrite { value: Expr::Parameter { name }, .. }
                if name == &projected_binding.name
        )
        && closed_counter_one_continuation(continuation)
    {
        let selected_name = syn::Ident::new(
            &format!("__compact_recorded_nested_uint4_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let widened_name = syn::Ident::new(
            &format!("__compact_recorded_nested_uint64_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote! {
            let #selected_name: runtime::BoundedUint<4> =
                runtime::BoundedUint::<4>::new((#selected) as u128)?;
        });
        steps.push(syn::parse_quote! {
            let #widened_name: runtime::BoundedUint<18446744073709551615> =
                runtime::cast_unsigned::<4, 18446744073709551615>(#selected_name)?;
        });
        let mut scoped = locals.clone();
        scoped.insert(
            unsigned_binding.name.clone(),
            syn::parse_quote!(#selected_name),
        );
        scoped.insert(
            projected_binding.name.clone(),
            syn::parse_quote!(#widened_name),
        );
        let first = append_steps(
            projected_action,
            &format!("{path}.action.actions[0].action"),
            &scoped,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
            &mut LegacyActionPrecision::Concrete,
        )?;
        if let RecordingOutcome::Unsupported(_) = first {
            return Ok(Some(first));
        }
        return append_steps(
            continuation,
            &format!("{path}.action.actions[1]"),
            &scoped,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
            &mut LegacyActionPrecision::Concrete,
        )
        .map(Some);
    }
    Ok(None)
}

#[expect(
    clippy::too_many_arguments,
    reason = "closed binding lowering preserves explicit declaration, scope, and recording state"
)]
pub(in crate::recorded) fn try_conditional_field_pair_cell_binding(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<RecordingOutcome<()>>, RenderError> {
    let StateAction::Let {
        bindings,
        action: nested_action,
    } = action
    else {
        return Ok(None);
    };
    if let [vector_binding] = bindings.as_slice()
        && vector_binding.ty
            == (Type::Vector {
                element: Box::new(Type::Field),
                length: 2,
            })
        && let Some((first, second)) =
            closed_conditional_field_pair(&vector_binding.value, locals, parameters)
        && matches!(
            nested_action.as_ref(),
            StateAction::CellWrite { value: Expr::Parameter { name }, .. }
                if name == &vector_binding.name
        )
    {
        let vector = syn::Ident::new(
            &format!("__compact_recorded_conditional_pair_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote! {
            let #vector: runtime::FixedVector<runtime::Field, 2> =
                runtime::FixedVector::new([#first, #second]);
        });
        let mut scoped = locals.clone();
        scoped.insert(vector_binding.name.clone(), syn::parse_quote!(#vector));
        return append_steps(
            nested_action,
            &format!("{path}.action"),
            &scoped,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
            &mut LegacyActionPrecision::Concrete,
        )
        .map(Some);
    }
    Ok(None)
}

#[expect(
    clippy::too_many_arguments,
    reason = "closed binding lowering preserves explicit declaration, scope, and recording state"
)]
pub(in crate::recorded) fn try_annotated_uint8_field_cell_binding(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<RecordingOutcome<()>>, RenderError> {
    let StateAction::Let {
        bindings,
        action: nested_action,
    } = action
    else {
        return Ok(None);
    };
    if let [unsigned_binding] = bindings.as_slice()
        && unsigned_binding.ty == (Type::Unsigned { max: "255".into() })
        && let Some(selected) =
            closed_annotated_uint8_source(&unsigned_binding.value, locals, parameters)
        && let StateAction::Let {
            bindings: projected_bindings,
            action: projected_action,
        } = nested_action.as_ref()
        && let [projected_binding] = projected_bindings.as_slice()
        && projected_binding.ty == Type::Field
        && matches!(
            &projected_binding.value,
            Expr::FieldCast { value }
                if matches!(value.as_ref(), Expr::Parameter { name } if name == &unsigned_binding.name)
        )
        && matches!(
            projected_action.as_ref(),
            StateAction::CellWrite { value: Expr::Parameter { name }, .. }
                if name == &projected_binding.name
        )
    {
        let unsigned = syn::Ident::new(
            &format!("__compact_recorded_annotated_uint8_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let field = syn::Ident::new(
            &format!("__compact_recorded_annotated_field_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote! {
            let #unsigned: runtime::BoundedUint<255> =
                runtime::BoundedUint::<255>::new(#selected)?;
        });
        steps.push(syn::parse_quote! {
            let #field: runtime::Field = runtime::Field::from(#unsigned.value());
        });
        let mut scoped = locals.clone();
        scoped.insert(unsigned_binding.name.clone(), syn::parse_quote!(#unsigned));
        scoped.insert(projected_binding.name.clone(), syn::parse_quote!(#field));
        return append_steps(
            projected_action,
            &format!("{path}.action.action"),
            &scoped,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
            &mut LegacyActionPrecision::Concrete,
        )
        .map(Some);
    }
    Ok(None)
}

#[expect(
    clippy::too_many_arguments,
    reason = "closed binding lowering preserves explicit declaration, scope, and recording state"
)]
pub(in crate::recorded) fn try_curve_field_cell_binding(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<RecordingOutcome<()>>, RenderError> {
    let StateAction::Let {
        bindings,
        action: nested_action,
    } = action
    else {
        return Ok(None);
    };
    if let [point_binding] = bindings.as_slice()
        && point_binding.ty == Type::JubjubPoint
        && let Some(argument) =
            closed_curve_field_argument(&point_binding.value, locals, parameters)
        && let StateAction::Let {
            bindings: projected_bindings,
            action: projected_action,
        } = nested_action.as_ref()
        && let [projected_binding] = projected_bindings.as_slice()
        && projected_binding.ty == Type::Field
        && matches!(
            &projected_binding.value,
            Expr::JubjubPointX { value }
                if matches!(value.as_ref(), Expr::Parameter { name } if name == &point_binding.name)
        )
        && matches!(
            projected_action.as_ref(),
            StateAction::CellWrite { value: Expr::Parameter { name }, .. }
                if name == &projected_binding.name
        )
    {
        let point = syn::Ident::new(
            &format!("__compact_recorded_curve_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let x = syn::Ident::new(
            &format!("__compact_recorded_curve_x_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote! {
            let #point: runtime::JubjubPoint = runtime::hash_to_curve(#argument);
        });
        steps.push(syn::parse_quote! {
            let #x: runtime::Field = runtime::jubjub_point_x(#point);
        });
        let mut scoped = locals.clone();
        scoped.insert(projected_binding.name.clone(), syn::parse_quote!(#x));
        return append_steps(
            projected_action,
            &format!("{path}.action.action"),
            &scoped,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
            &mut LegacyActionPrecision::Concrete,
        )
        .map(Some);
    }
    Ok(None)
}

#[expect(
    clippy::too_many_arguments,
    reason = "closed binding lowering preserves explicit declaration, scope, and recording state"
)]
pub(in crate::recorded) fn try_nested_uint4_field_cell_binding(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<RecordingOutcome<()>>, RenderError> {
    let StateAction::Let {
        bindings,
        action: nested_action,
    } = action
    else {
        return Ok(None);
    };
    // Preserve the typed Uint<4> local and its immediate Field
    // projection. A Sequence may put a Counter increment after
    // that projection, but the projection itself remains first.
    if let [unsigned_binding] = bindings.as_slice()
        && unsigned_binding.ty == (Type::Unsigned { max: "4".into() })
        && let Some(selected) =
            closed_nested_uint4_source(&unsigned_binding.value, locals, parameters)
        && let Some((projected_bindings, projected_action, continuation)) = (match nested_action
            .as_ref()
        {
            StateAction::Let { bindings, action } => Some((bindings, action.as_ref(), None)),
            StateAction::Sequence { actions } => {
                if let [StateAction::Let { bindings, action }, continuation] = actions.as_slice() {
                    Some((bindings, action.as_ref(), Some(continuation)))
                } else {
                    None
                }
            }
            _ => None,
        })
        && let [projected_binding] = projected_bindings.as_slice()
        && projected_binding.ty == Type::Field
        && matches!(
            &projected_binding.value,
            Expr::FieldCast { value }
                if matches!(value.as_ref(), Expr::Parameter { name } if name == &unsigned_binding.name)
        )
    {
        let selected_name = syn::Ident::new(
            &format!("__compact_recorded_nested_uint4_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let field_name = syn::Ident::new(
            &format!("__compact_recorded_nested_field_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote! {
            let #selected_name = runtime::BoundedUint::<4>::new(#selected as u128)?;
        });
        steps.push(syn::parse_quote! {
            let #field_name: runtime::Field =
                runtime::Field::from(#selected_name.value() as u64);
        });
        let mut scoped = locals.clone();
        scoped.insert(
            unsigned_binding.name.clone(),
            syn::parse_quote!(#selected_name),
        );
        scoped.insert(
            projected_binding.name.clone(),
            syn::parse_quote!(#field_name),
        );
        let projected_path = if continuation.is_some() {
            format!("{path}.action.actions[0].action")
        } else {
            format!("{path}.action.action")
        };
        let first = append_steps(
            projected_action,
            &projected_path,
            &scoped,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
            &mut LegacyActionPrecision::Concrete,
        )?;
        if let RecordingOutcome::Unsupported(_) = first {
            return Ok(Some(first));
        }
        if let Some(continuation) = continuation {
            return append_steps(
                continuation,
                &format!("{path}.action.actions[1]"),
                &scoped,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
                &mut LegacyActionPrecision::Concrete,
            )
            .map(Some);
        }
        return Ok(Some(RecordingOutcome::Supported(())));
    }
    Ok(None)
}

#[expect(
    clippy::too_many_arguments,
    reason = "closed binding lowering preserves explicit declaration, scope, and recording state"
)]
pub(in crate::recorded) fn try_literal_struct_field_projection_binding(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<RecordingOutcome<()>>, RenderError> {
    let StateAction::Let {
        bindings,
        action: nested_action,
    } = action
    else {
        return Ok(None);
    };
    // A one-Field struct built from a closed literal ternary can
    // be retained as a typed Rust value before projecting the
    // same member in the immediately nested Let. Do not turn an
    // arbitrary StructLiteral or effectful arm into a pure local.
    if let [struct_binding] = bindings.as_slice()
        && let Type::Struct {
            fields: struct_fields,
            ..
        } = &struct_binding.ty
        && let [struct_field] = struct_fields.as_slice()
        && struct_field.ty == Type::Field
        && let Expr::StructLiteral {
            ty: literal_ty,
            fields: literal_fields,
        } = &struct_binding.value
        && literal_ty == &struct_binding.ty
        && let [field_value] = literal_fields.as_slice()
        && let Expr::FieldCast { value: cast_value } = field_value
        && let Expr::If { condition, .. } = cast_value.as_ref()
        && cell_source(condition, &Type::Boolean, locals, parameters).is_some()
        && let StateAction::Let {
            bindings: projected_bindings,
            action: projected_action,
        } = nested_action.as_ref()
        && let [projected_binding] = projected_bindings.as_slice()
        && projected_binding.ty == Type::Field
        && let Expr::StructField {
            value: projected_source,
            field: projected_field,
            index: 0,
        } = &projected_binding.value
        && projected_field == &struct_field.name
        && matches!(projected_source.as_ref(), Expr::Parameter { name } if name == &struct_binding.name)
        && matches!(projected_action.as_ref(), StateAction::CellWrite { value: Expr::Parameter { name }, .. } if name == &projected_binding.name)
    {
        let Some(field_value) = field_expression(
            field_value,
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
        else {
            return Ok(Some(RecordingOutcome::Unsupported(
                RecordingGap::expression(
                    &literal_fields[0],
                    format!("{path}.bindings[0].value.fields[0]"),
                ),
            )));
        };
        let struct_ty = rust_type(&struct_binding.ty)?;
        let member = ident(&struct_field.name)?;
        let retained = syn::Ident::new(
            &format!("__compact_recorded_struct_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let projected = syn::Ident::new(
            &format!("__compact_recorded_struct_field_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote! {
            let #retained: #struct_ty = #struct_ty { #member: #field_value };
        });
        steps.push(syn::parse_quote! {
            let #projected: runtime::Field = #retained.#member;
        });
        let mut scoped = locals.clone();
        scoped.insert(
            projected_binding.name.clone(),
            syn::parse_quote!(#projected),
        );
        return append_steps(
            projected_action,
            &format!("{path}.action.action"),
            &scoped,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
            &mut LegacyActionPrecision::Concrete,
        )
        .map(Some);
    }
    Ok(None)
}

#[expect(
    clippy::too_many_arguments,
    reason = "closed binding lowering preserves explicit declaration, scope, and recording state"
)]
pub(in crate::recorded) fn try_pure_struct_field_projection_binding(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<RecordingOutcome<()>>, RenderError> {
    let StateAction::Let {
        bindings,
        action: nested_action,
    } = action
    else {
        return Ok(None);
    };
    // Keep a pure one-field constructor's exact result type at
    // the call site. Two expanded modules may give otherwise
    // similar structs different Rust names and member names.
    if let [struct_binding] = bindings.as_slice()
        && let Type::Struct {
            fields: struct_fields,
            ..
        } = &struct_binding.ty
        && let [struct_field] = struct_fields.as_slice()
        && struct_field.ty == Type::Field
        && let Expr::Call { name, arguments } = &struct_binding.value
        && let Some(callee) = pure_circuits.get(name.as_str())
        && callee.result == struct_binding.ty
        && let [callee_parameter] = callee.parameters.as_slice()
        && callee_parameter.ty == Type::Field
        && let Expr::StructLiteral {
            ty: literal_ty,
            fields: literal_fields,
        } = &callee.body
        && literal_ty == &callee.result
        && matches!(literal_fields.as_slice(), [Expr::Parameter { name }] if name == &callee_parameter.name)
        && let [argument] = arguments.as_slice()
        && let Some(argument) = cell_source(argument, &Type::Field, locals, parameters)
        && let StateAction::Let {
            bindings: projected_bindings,
            action: projected_action,
        } = nested_action.as_ref()
        && let [projected_binding] = projected_bindings.as_slice()
        && projected_binding.ty == Type::Field
        && let Expr::StructField {
            value: projected_source,
            field: projected_field,
            index: 0,
        } = &projected_binding.value
        && projected_field == &struct_field.name
        && matches!(projected_source.as_ref(), Expr::Parameter { name } if name == &struct_binding.name)
        && matches!(projected_action.as_ref(), StateAction::CellWrite { value: Expr::Parameter { name }, .. } if name == &projected_binding.name)
    {
        let struct_ty = rust_type(&struct_binding.ty)?;
        let method = ident(name)?;
        let member = ident(&struct_field.name)?;
        let arg = syn::Ident::new(
            &format!("__compact_recorded_struct_arg_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let retained = syn::Ident::new(
            &format!("__compact_recorded_struct_call_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let projected = syn::Ident::new(
            &format!("__compact_recorded_struct_field_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        steps.push(syn::parse_quote!(let #arg: runtime::Field = #argument;));
        steps.push(syn::parse_quote! {
            let #retained: #struct_ty = crate::pure_circuits::#method(#arg)?;
        });
        steps.push(syn::parse_quote! {
            let #projected: runtime::Field = #retained.#member;
        });
        let mut scoped = locals.clone();
        scoped.insert(
            projected_binding.name.clone(),
            syn::parse_quote!(#projected),
        );
        return append_steps(
            projected_action,
            &format!("{path}.action.action"),
            &scoped,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
            &mut LegacyActionPrecision::Concrete,
        )
        .map(Some);
    }
    Ok(None)
}
