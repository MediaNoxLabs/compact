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
    Expr, LedgerField, LedgerFieldKind, PureCircuit, StateAction, StateReturn, StatefulCircuit,
    Type, WitnessDeclaration,
};
use crate::recorded::legacy::actions::append_steps;
use crate::recorded::legacy::actions::bindings::closed::{
    try_annotated_uint8_field_cell_binding, try_conditional_field_pair_cell_binding,
    try_curve_field_cell_binding, try_literal_struct_field_projection_binding,
    try_nested_uint4_field_cell_binding, try_nested_uint64_cell_counter_binding,
    try_pure_struct_field_projection_binding,
};
use crate::recorded::legacy::calls::shared_field_call;
use crate::recorded::legacy::expressions::{boolean_expression, field_expression};
use crate::recorded::legacy::value_sources::{
    amount_source, cell_source, conditional_uint64_source, literal_field_pair_hash_source,
    static_bindings,
};
use crate::recorded::pure_calls::{
    closed_literal_field_vector_call, closed_pure_assert_call, closed_pure_field_call,
    closed_pure_field_pair_hash_call, closed_pure_unsigned_call, closed_struct_hash_value,
    identity_struct_call_argument,
};
use crate::recorded::{
    LegacyActionPrecision, RecordingGap, RecordingOutcome, recordable_cell_type, unavailable_action,
};
use crate::{RenderError, ident, rust_type};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

mod closed;

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
                    let Some(operand) =
                        literal_field_pair_hash_source(operand, &scoped, parameters)
                    else {
                        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                            &binding.value,
                            format!("{path}.bindings[{binding_index}].value"),
                        )));
                    };
                    let pair = syn::Ident::new(
                        &format!("__compact_recorded_hash_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let result = syn::Ident::new(
                        &format!("__compact_recorded_hash_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote! {
                        let #pair: (runtime::Field, runtime::Field) = #operand;
                    });
                    if persistent {
                        steps.push(syn::parse_quote! {
                            let #result: runtime::FixedBytes<32> = runtime::persistent_hash(#pair);
                        });
                    } else {
                        steps.push(syn::parse_quote! {
                            let #result: runtime::Field = runtime::transient_hash(#pair);
                        });
                    }
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#result));
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
                    // Admit an internal Field helper only when its complete body is
                    // a closed typed pair-hash call followed by one Field Cell read.
                    // The pure result must be evaluated before the recorded read,
                    // then the sum must be retained for the nested public action.
                    if let Expr::Call { name, arguments } = &binding.value
                        && let Some(callee) = circuits.get(name.as_str())
                        && callee.internal
                        && callee.actions.is_empty()
                        && callee.result == Type::Field
                        && let [callee_parameter] = callee.parameters.as_slice()
                        && let [argument] = arguments.as_slice()
                        && let StateReturn::Expression {
                            value: Expr::Add { left, right },
                        } = &callee.return_value
                        && let Expr::Call {
                            name: pure_name,
                            arguments: pure_arguments,
                        } = left.as_ref()
                        && let [
                            Expr::Coerce {
                                value: pure_argument,
                                ty: pure_argument_ty,
                            },
                        ] = pure_arguments.as_slice()
                        && let Expr::Parameter {
                            name: pure_parameter_name,
                        } = pure_argument.as_ref()
                        && pure_parameter_name == &callee_parameter.name
                        && pure_argument_ty == &callee_parameter.ty
                        && let Some(pure) = pure_circuits.get(pure_name.as_str())
                        && pure.result == Type::Field
                        && let [pure_parameter] = pure.parameters.as_slice()
                        && pure_parameter.ty == callee_parameter.ty
                        && closed_pure_field_pair_hash_call(pure_name, pure_circuits)
                        && let Expr::CellRead { field, index } = right.as_ref()
                        && let Some(declaration) = ledger_fields.get(field.as_str())
                        && declaration.index == *index
                        && declaration.declaration == (LedgerFieldKind::Cell { ty: Type::Field })
                    {
                        let Some(arg_value) =
                            cell_source(argument, &callee_parameter.ty, &scoped, parameters)
                        else {
                            return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                                argument,
                                format!("{path}.bindings[{binding_index}].value.arguments[0]"),
                            )));
                        };
                        let arg = syn::Ident::new(
                            &format!("__compact_recorded_helper_arg_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        let arg_ty = rust_type(&callee_parameter.ty)?;
                        steps.push(syn::parse_quote!(let #arg: #arg_ty = #arg_value;));
                        let pure_result = syn::Ident::new(
                            &format!("__compact_recorded_helper_pure_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        let pure_method = ident(pure_name)?;
                        steps.push(syn::parse_quote! {
                            let #pure_result: runtime::Field =
                                crate::pure_circuits::#pure_method(#arg)?;
                        });
                        let observed = syn::Ident::new(
                            &format!("__compact_recorded_helper_read_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        let slot = ident(field)?;
                        steps.push(syn::parse_quote! {
                            let (frame, #observed): (_, runtime::Field) =
                                crate::ledger_slots::#slot.record_read(frame)?;
                        });
                        let result = syn::Ident::new(
                            &format!("__compact_recorded_helper_result_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        steps.push(syn::parse_quote! {
                            let #result: runtime::Field = #pure_result + #observed;
                        });
                        scoped.insert(binding.name.clone(), syn::parse_quote!(#result));
                        continue;
                    }
                    if let Expr::Call { name, arguments } = &binding.value
                        && let Some(callee) = pure_circuits.get(name.as_str())
                        && callee.result == Type::Field
                        && closed_pure_field_call(name, pure_circuits, &mut HashSet::new())
                    {
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
                            if parameter.ty != Type::Field {
                                return Ok(RecordingOutcome::Unsupported(
                                    RecordingGap::expression(
                                        argument,
                                        format!(
                                            "{path}.bindings[{binding_index}].value.arguments[{argument_index}]"
                                        ),
                                    ),
                                ));
                            }
                            let Some(argument) = field_expression(
                                argument,
                                &scoped,
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
                                return Ok(RecordingOutcome::Unsupported(
                                    RecordingGap::expression(
                                        argument,
                                        format!(
                                            "{path}.bindings[{binding_index}].value.arguments[{argument_index}]"
                                        ),
                                    ),
                                ));
                            };
                            let arg = syn::Ident::new(
                                &format!("__compact_recorded_arg_{}", *next_temp),
                                Span::call_site(),
                            );
                            *next_temp += 1;
                            steps.push(syn::parse_quote!(let #arg: runtime::Field = #argument;));
                            args.push(arg);
                        }
                        let method = ident(name)?;
                        let value = syn::Ident::new(
                            &format!("__compact_recorded_pure_field_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        steps.push(syn::parse_quote! {
                            let #value: runtime::Field = crate::pure_circuits::#method(#(#args),*)?;
                        });
                        scoped.insert(binding.name.clone(), syn::parse_quote!(#value));
                        continue;
                    }
                    if let Expr::Call { name, arguments } = &binding.value
                        && closed_pure_field_pair_hash_call(name, pure_circuits)
                    {
                        let callee = pure_circuits
                            .get(name.as_str())
                            .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
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
                            let Some(argument_value) =
                                cell_source(argument, &parameter.ty, &scoped, parameters)
                            else {
                                return Ok(RecordingOutcome::Unsupported(
                                    RecordingGap::expression(
                                        argument,
                                        format!(
                                            "{path}.bindings[{binding_index}].value.arguments[{argument_index}]"
                                        ),
                                    ),
                                ));
                            };
                            let arg = syn::Ident::new(
                                &format!("__compact_recorded_pair_arg_{}", *next_temp),
                                Span::call_site(),
                            );
                            *next_temp += 1;
                            let ty = rust_type(&parameter.ty)?;
                            steps.push(syn::parse_quote!(let #arg: #ty = #argument_value;));
                            args.push(arg);
                        }
                        let method = ident(name)?;
                        let value = syn::Ident::new(
                            &format!("__compact_recorded_pure_pair_hash_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        steps.push(syn::parse_quote! {
                            let #value: runtime::Field = crate::pure_circuits::#method(#(#args),*)?;
                        });
                        scoped.insert(binding.name.clone(), syn::parse_quote!(#value));
                        continue;
                    }
                    if let Expr::Call { name, arguments } = &binding.value
                        && shared_callees.contains(name)
                    {
                        let Some(observed) = shared_field_call(
                            name,
                            arguments,
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
                            return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                                &binding.value,
                                format!("{path}.bindings[{binding_index}].value"),
                            )));
                        };
                        scoped.insert(binding.name.clone(), observed);
                        continue;
                    }
                    let Some(value) = field_expression(
                        &binding.value,
                        &scoped,
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
                        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                            &binding.value,
                            format!("{path}.bindings[{binding_index}].value"),
                        )));
                    };
                    scoped.insert(binding.name.clone(), value);
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
                    if !matches!(
                        binding.ty,
                        Type::Boolean | Type::Field | Type::Bytes { .. } | Type::Unsigned { .. }
                    ) && !(matches!(binding.ty, Type::Struct { .. })
                        && recordable_cell_type(&binding.ty))
                    {
                        return Ok(unavailable_action(whole, path));
                    }
                    let declaration = witnesses
                        .get(name.as_str())
                        .ok_or_else(|| RenderError::UnknownWitness(name.clone()))?;
                    if arguments.len() != declaration.parameters.len() {
                        return Err(RenderError::ArgumentCount {
                            circuit: name.clone(),
                            expected: declaration.parameters.len(),
                            actual: arguments.len(),
                        });
                    }
                    if binding.ty != declaration.result {
                        return Err(RenderError::TypeMismatch {
                            expected: binding.ty.clone(),
                            actual: declaration.result.clone(),
                        });
                    }
                    let mut args = Vec::new();
                    for (argument, parameter) in arguments.iter().zip(&declaration.parameters) {
                        if matches!(binding.ty, Type::Struct { .. })
                            && !identity_struct_call_argument(argument)
                        {
                            return Ok(unavailable_action(whole, path));
                        }
                        let Some(arg) = cell_source(argument, &parameter.ty, &scoped, parameters)
                        else {
                            return Ok(unavailable_action(whole, path));
                        };
                        args.push(arg);
                    }
                    let method = ident(name)?;
                    let value = syn::Ident::new(
                        &format!("__compact_witness_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote! {
                        let (frame, #value) = frame.try_witness_metered(|context, meter| {
                            witnesses.#method(
                                context.witness_context_with(super::LedgerView {
                                    state: context.query.state.get_ref(),
                                    meter,
                                }),
                                #(#args),*
                            )
                        })?;
                    });
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#value));
                } else if let (
                    Type::Bytes { length: 32 },
                    Expr::PersistentCommit { value, opening },
                ) = (&binding.ty, &binding.value)
                {
                    if !matches!(value.as_ref(), Expr::FieldLiteral { .. }) {
                        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                            value,
                            format!("{path}.bindings[{binding_index}].value.value"),
                        )));
                    }
                    let Expr::CellRead { field, index } = opening.as_ref() else {
                        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                            opening,
                            format!("{path}.bindings[{binding_index}].value.opening"),
                        )));
                    };
                    let declaration = ledger_fields
                        .get(field.as_str())
                        .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                    if declaration.declaration
                        != (LedgerFieldKind::Cell {
                            ty: Type::Bytes { length: 32 },
                        })
                        || declaration.index != *index
                    {
                        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                            opening,
                            format!("{path}.bindings[{binding_index}].value.opening"),
                        )));
                    }
                    let Some(field_value) = field_expression(
                        value,
                        &scoped,
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
                        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                            value,
                            format!("{path}.bindings[{binding_index}].value.value"),
                        )));
                    };
                    let slot = ident(field)?;
                    let opening_value = syn::Ident::new(
                        &format!("__compact_recorded_opening_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let committed = syn::Ident::new(
                        &format!("__compact_recorded_commitment_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote! {
                        let (frame, #opening_value): (_, runtime::FixedBytes<32>) =
                            crate::ledger_slots::#slot.record_read(frame)?;
                    });
                    steps.push(syn::parse_quote! {
                        let #committed: runtime::FixedBytes<32> =
                            runtime::persistent_commit(#field_value, #opening_value);
                    });
                    scoped.insert(binding.name.clone(), syn::parse_quote!(#committed));
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
