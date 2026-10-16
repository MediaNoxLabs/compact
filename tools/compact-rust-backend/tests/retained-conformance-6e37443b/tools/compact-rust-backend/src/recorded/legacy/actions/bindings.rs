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

//! Lexical binding lowering after the ordered closed-prefix cases.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::ir::{
    Expr, LedgerField, PureCircuit, StateAction, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::recorded::legacy::actions::append_steps;
use crate::recorded::legacy::actions::bindings::closed::{
    try_annotated_uint8_field_cell_binding, try_conditional_field_pair_cell_binding,
    try_curve_field_cell_binding, try_literal_struct_field_projection_binding,
    try_nested_uint4_field_cell_binding, try_nested_uint64_cell_counter_binding,
    try_pure_struct_field_projection_binding,
};
use crate::recorded::legacy::expressions::boolean_expression;
use crate::recorded::legacy::value_sources::{
    amount_source, cell_source, conditional_uint64_source, static_bindings,
};
use crate::recorded::pure_calls::{
    closed_literal_field_vector_call, closed_pure_assert_call, closed_pure_unsigned_call,
    closed_struct_hash_value, identity_struct_call_argument,
};
use crate::recorded::{
    LegacyActionPrecision, RecordingGap, RecordingOutcome, recordable_cell_type, unavailable_action,
};
use crate::{RenderError, ident, rust_type};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

mod closed;
mod crypto;
mod field;
mod witness;

// This family retains the original lowering arms and ordered recording state.
#[expect(
    clippy::too_many_arguments,
    reason = "legacy lowering keeps declarations, scope, and ordered recording state explicit"
)]
pub(in crate::recorded) fn append_binding_steps(
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
) -> Result<RecordingOutcome<()>, RenderError> {
    match action {
        whole @ StateAction::Let {
            bindings,
            action: nested_action,
        } => {
            if let Some(outcome) = try_nested_uint64_cell_counter_binding(
                action,
                path,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )? {
                return Ok(outcome);
            }
            if let Some(outcome) = try_conditional_field_pair_cell_binding(
                action,
                path,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )? {
                return Ok(outcome);
            }
            if let Some(outcome) = try_annotated_uint8_field_cell_binding(
                action,
                path,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )? {
                return Ok(outcome);
            }
            if let Some(outcome) = try_curve_field_cell_binding(
                action,
                path,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )? {
                return Ok(outcome);
            }
            if let Some(outcome) = try_nested_uint4_field_cell_binding(
                action,
                path,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )? {
                return Ok(outcome);
            }
            if let Some(outcome) = try_literal_struct_field_projection_binding(
                action,
                path,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )? {
                return Ok(outcome);
            }
            if let Some(outcome) = try_pure_struct_field_projection_binding(
                action,
                path,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            )? {
                return Ok(outcome);
            }
            let mut scoped = locals.clone();
            for (binding_index, binding) in bindings.iter().enumerate() {
                if binding.ty == Type::OpaqueString
                    && let Expr::Parameter { name } = &binding.value
                    && parameters
                        .get(name.as_str())
                        .is_some_and(|(ty, _)| *ty == &Type::OpaqueString)
                    && let Some(value) =
                        cell_source(&binding.value, &binding.ty, &scoped, parameters)
                {
                    let local = syn::Ident::new(
                        &format!("__compact_recorded_opaque_key_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #local: runtime::OpaqueString = #value;));
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#local));
                    continue;
                }
                let hash = match (&binding.ty, &binding.value) {
                    (Type::Bytes { length: 32 }, Expr::PersistentHash { value }) => {
                        Some((value.as_ref(), true))
                    }
                    (Type::Field, Expr::TransientHash { value }) => Some((value.as_ref(), false)),
                    _ => None,
                };
                if let Some((operand, persistent)) = hash {
                    if let RecordingOutcome::Unsupported(gap) = crypto::append_pair_hash_binding(
                        binding,
                        binding_index,
                        path,
                        operand,
                        persistent,
                        &mut scoped,
                        parameters,
                        steps,
                        next_temp,
                    )? {
                        return Ok(RecordingOutcome::Unsupported(gap));
                    }
                    continue;
                }
                if matches!(binding.ty, Type::Unsigned { .. })
                    && let Expr::Call { name, arguments } = &binding.value
                {
                    let Some(callee) = pure_circuits.get(name.as_str()) else {
                        return Ok(unavailable_action(whole, path));
                    };
                    if callee.result != binding.ty
                        || !closed_pure_unsigned_call(name, pure_circuits, &mut HashSet::new())
                    {
                        return Ok(unavailable_action(whole, path));
                    }
                    if arguments.len() != callee.parameters.len() {
                        return Err(RenderError::ArgumentCount {
                            circuit: name.clone(),
                            expected: callee.parameters.len(),
                            actual: arguments.len(),
                        });
                    }
                    let mut args = Vec::new();
                    for (argument_index, (argument, parameter)) in
                        arguments.iter().zip(&callee.parameters).enumerate()
                    {
                        let Some(value) = cell_source(argument, &parameter.ty, &scoped, parameters)
                        else {
                            return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                                argument,
                                format!(
                                    "{path}.bindings[{binding_index}].value.arguments[{argument_index}]"
                                ),
                            )));
                        };
                        let arg = syn::Ident::new(
                            &format!("__compact_recorded_unsigned_arg_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        let ty = rust_type(&parameter.ty)?;
                        steps.push(syn::parse_quote!(let #arg: #ty = #value;));
                        args.push(arg);
                    }
                    let method = ident(name)?;
                    let local = syn::Ident::new(
                        &format!("__compact_recorded_pure_unsigned_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let ty = rust_type(&binding.ty)?;
                    steps.push(syn::parse_quote! {
                        let #local: #ty = crate::pure_circuits::#method(#(#args),*)?;
                    });
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#local));
                } else if binding.ty
                    == (Type::Unsigned {
                        max: "65535".into(),
                    })
                {
                    let Some(value) = amount_source(&binding.value, &scoped, parameters) else {
                        return Ok(unavailable_action(whole, path));
                    };
                    scoped.insert(binding.name.clone(), value);
                } else if binding.ty
                    == (Type::Unsigned {
                        max: "18446744073709551615".into(),
                    })
                    && matches!(binding.value, Expr::UnsignedLiteral { .. })
                {
                    // A compiler-introduced literal index is a pure typed
                    // local. Keep it scoped, then validate the nested
                    // ledger action through the ordinary recorder.
                    let Some(value) = cell_source(&binding.value, &binding.ty, &scoped, parameters)
                    else {
                        return Ok(unavailable_action(whole, path));
                    };
                    scoped.insert(binding.name.clone(), value);
                } else if binding.ty
                    == (Type::Unsigned {
                        max: "18446744073709551615".into(),
                    })
                    && !matches!(binding.value, Expr::WitnessCall { .. })
                {
                    let Some(value) =
                        conditional_uint64_source(&binding.value, &scoped, parameters)
                    else {
                        return Ok(unavailable_action(whole, path));
                    };
                    let local = syn::Ident::new(
                        &format!("__compact_recorded_uint64_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(
                        let #local = runtime::BoundedUint::<18446744073709551615>::new(#value)?;
                    ));
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#local));
                } else if binding.ty == Type::Field {
                    if let RecordingOutcome::Unsupported(gap) = field::append_field_binding(
                        binding,
                        binding_index,
                        path,
                        &mut scoped,
                        parameters,
                        ledger_fields,
                        witnesses,
                        pure_circuits,
                        circuits,
                        shared_callees,
                        steps,
                        next_temp,
                        visiting,
                    )? {
                        return Ok(RecordingOutcome::Unsupported(gap));
                    }
                } else if binding.ty == Type::Boolean
                    && let Expr::Call { name, arguments } = &binding.value
                    && let Some(callee) = pure_circuits.get(name.as_str())
                    && callee.result == Type::Boolean
                    && closed_pure_assert_call(callee)
                {
                    let [parameter] = callee.parameters.as_slice() else {
                        unreachable!("closed pure assertion has one parameter")
                    };
                    let [argument] = arguments.as_slice() else {
                        return Ok(unavailable_action(whole, path));
                    };
                    let Some(argument) = cell_source(argument, &parameter.ty, &scoped, parameters)
                    else {
                        return Ok(unavailable_action(whole, path));
                    };
                    let method = ident(name)?;
                    let typed_arg = syn::Ident::new(
                        &format!("__compact_recorded_pure_assert_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let value = syn::Ident::new(
                        &format!("__compact_recorded_pure_assert_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #typed_arg: bool = #argument;));
                    steps.push(syn::parse_quote!(
                        let #value: bool = crate::pure_circuits::#method(#typed_arg)?;
                    ));
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#value));
                } else if binding.ty == Type::Boolean
                    && !matches!(binding.value, Expr::WitnessCall { .. })
                {
                    let Some(value) = boolean_expression(
                        &binding.value,
                        &scoped,
                        parameters,
                        ledger_fields,
                        witnesses,
                        circuits,
                        steps,
                        next_temp,
                        visiting,
                    )?
                    else {
                        return Ok(unavailable_action(whole, path));
                    };
                    scoped.insert(binding.name.clone(), value);
                } else if matches!(binding.ty, Type::Enum { .. })
                    && matches!(
                        binding.value,
                        Expr::EnumVariant { .. } | Expr::Parameter { .. }
                    )
                {
                    let Some(value) = cell_source(&binding.value, &binding.ty, &scoped, parameters)
                    else {
                        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                            &binding.value,
                            format!("{path}.bindings[{binding_index}].value"),
                        )));
                    };
                    let value_ty = rust_type(&binding.ty)?;
                    let local = syn::Ident::new(
                        &format!("__compact_recorded_enum_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #local: #value_ty = #value;));
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#local));
                } else if let Expr::WitnessCall { name, arguments } = &binding.value {
                    if let RecordingOutcome::Unsupported(gap) = witness::append_witness_binding(
                        whole,
                        binding,
                        path,
                        name,
                        arguments,
                        &mut scoped,
                        parameters,
                        witnesses,
                        steps,
                        next_temp,
                    )? {
                        return Ok(RecordingOutcome::Unsupported(gap));
                    }
                } else if let (
                    Type::Bytes { length: 32 },
                    Expr::PersistentCommit { value, opening },
                ) = (&binding.ty, &binding.value)
                {
                    if let RecordingOutcome::Unsupported(gap) = crypto::append_commitment_binding(
                        binding,
                        binding_index,
                        path,
                        value,
                        opening,
                        &mut scoped,
                        parameters,
                        ledger_fields,
                        witnesses,
                        circuits,
                        shared_callees,
                        steps,
                        next_temp,
                        visiting,
                    )? {
                        return Ok(RecordingOutcome::Unsupported(gap));
                    }
                } else if matches!(binding.ty, Type::Bytes { .. })
                    && let Expr::Call { name, arguments } = &binding.value
                {
                    if circuits.contains_key(name.as_str()) {
                        return Ok(unavailable_action(whole, path));
                    }
                    let mut args = Vec::new();
                    for argument in arguments {
                        let Expr::Coerce { ty, .. } = argument else {
                            return Ok(unavailable_action(whole, path));
                        };
                        let Some(value) = cell_source(argument, ty, &scoped, parameters) else {
                            return Ok(unavailable_action(whole, path));
                        };
                        args.push(value);
                    }
                    let method = ident(name)?;
                    let value = syn::Ident::new(
                        &format!("__compact_recorded_pure_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let value_ty = rust_type(&binding.ty)?;
                    steps.push(syn::parse_quote! {
                        let #value: #value_ty = crate::pure_circuits::#method(#(#args),*)?;
                    });
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#value));
                } else if matches!(binding.ty, Type::Bytes { .. })
                    && matches!(binding.value, Expr::BytesLiteral { .. })
                {
                    let Some(value) = cell_source(&binding.value, &binding.ty, &scoped, parameters)
                    else {
                        return Ok(unavailable_action(whole, path));
                    };
                    let value_ty = rust_type(&binding.ty)?;
                    let local = syn::Ident::new(
                        &format!("__compact_recorded_bytes_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #local: #value_ty = #value;));
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#local));
                } else if matches!(binding.ty, Type::Bytes { .. }) {
                    let Some(value) = cell_source(&binding.value, &binding.ty, &scoped, parameters)
                    else {
                        return Ok(unavailable_action(whole, path));
                    };
                    scoped.insert(binding.name.clone(), value);
                } else if let (Type::Struct { .. }, Expr::Call { name, arguments }) =
                    (&binding.ty, &binding.value)
                {
                    let Some(callee) = pure_circuits.get(name.as_str()) else {
                        return Ok(unavailable_action(whole, path));
                    };
                    if callee.result != binding.ty
                        || !recordable_cell_type(&binding.ty)
                        || !closed_struct_hash_value(&callee.body)
                    {
                        return Ok(unavailable_action(whole, path));
                    }
                    if arguments.len() != callee.parameters.len() {
                        return Err(RenderError::ArgumentCount {
                            circuit: name.clone(),
                            expected: callee.parameters.len(),
                            actual: arguments.len(),
                        });
                    }
                    let mut args = Vec::new();
                    for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                        if !identity_struct_call_argument(argument) {
                            return Ok(unavailable_action(whole, path));
                        }
                        let Some(value) = cell_source(argument, &parameter.ty, &scoped, parameters)
                        else {
                            return Ok(unavailable_action(whole, path));
                        };
                        args.push(value);
                    }
                    let method = ident(name)?;
                    let value_ty = rust_type(&binding.ty)?;
                    let local = syn::Ident::new(
                        &format!("__compact_recorded_struct_value_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote! { let #local: #value_ty = crate::pure_circuits::#method(#(#args),*)?; });
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#local));
                } else if matches!(binding.ty, Type::Vector { .. }) {
                    if let Expr::Call { name, arguments } = &binding.value
                        && arguments.is_empty()
                        && closed_literal_field_vector_call(name, &binding.ty, pure_circuits)
                    {
                        let method = ident(name)?;
                        let value_ty = rust_type(&binding.ty)?;
                        let local = syn::Ident::new(
                            &format!("__compact_recorded_pure_vector_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        steps.push(syn::parse_quote! {
                            let #local: #value_ty = crate::pure_circuits::#method()?;
                        });
                        scoped.insert(binding.name.clone(), syn::parse_quote!(#local));
                        continue;
                    }
                    let Some(next) = static_bindings(
                        std::slice::from_ref(binding),
                        &scoped,
                        parameters,
                        steps,
                        next_temp,
                    )?
                    else {
                        return Ok(unavailable_action(whole, path));
                    };
                    scoped = next;
                } else {
                    return Ok(unavailable_action(action, path));
                }
            }
            append_steps(
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
        }
        _ => Ok(unavailable_action(action, path)),
    }
}
