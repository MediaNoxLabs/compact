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

//! Emit complete replayable traces from supported typed stateful IR.
//! Unsupported effect shapes have no generated recorded entry point.

use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

use crate::ir::{
    CounterAmount, Expr, LedgerField, LedgerFieldKind, LocalBinding, PureCircuit, StateAction,
    StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::stateful::circuit_uses_witness;
use crate::{RenderError, expression_with_calls, ident, list_head_result_type, rust_type};

/// Emit a replayable public VM trace for supported root Cell, Counter, Set, Map, List,
/// and plain/historic Merkle append
/// operations, including witnessed Cell values. Unsupported circuits have no
/// recorded entry point.
pub(crate) fn render_recorded_circuit(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
) -> Result<Option<syn::Item>, RenderError> {
    render_recorded_item(
        circuit,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        shared_callees,
        false,
    )
}

pub(crate) fn render_recorded_helper(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
) -> Result<Option<syn::Item>, RenderError> {
    render_recorded_item(
        circuit,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        shared_callees,
        true,
    )
}

fn helper_ident(
    name: &str,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<syn::Ident, RenderError> {
    let base = ident(&format!("__compact_recorded_body_{name}"))?;
    let mut duplicate_base = false;
    for other in circuits.keys() {
        if ident(other)? == base {
            duplicate_base = true;
            break;
        }
        if *other != name && ident(&format!("__compact_recorded_body_{other}"))? == base {
            duplicate_base = true;
            break;
        }
    }
    if !duplicate_base {
        return Ok(base);
    }

    // A Compact declaration may itself use our preferred helper name. Use a
    // stable index and the raw-name bytes so even `$`/`_` aliases stay distinct.
    let mut ordered: Vec<_> = circuits.keys().copied().collect();
    ordered.sort_unstable();
    let index = ordered
        .iter()
        .position(|candidate| *candidate == name)
        .ok_or_else(|| RenderError::UnknownCircuit(name.to_owned()))?;
    let mut fallback = format!("__compact_recorded_body_{index}_x");
    for byte in name.bytes() {
        fallback.push_str(&format!("{byte:02x}"));
    }
    loop {
        let candidate = ident(&fallback)?;
        let occupied = circuits.keys().any(|other| {
            ident(other).is_ok_and(|id| id == candidate)
                || ident(&format!("__compact_recorded_body_{other}"))
                    .is_ok_and(|id| id == candidate)
        });
        if !occupied {
            return Ok(candidate);
        }
        fallback.push('_');
    }
}

fn collect_field_callees(value: &Expr, names: &mut HashSet<String>) {
    match value {
        Expr::Call { name, .. } => {
            names.insert(name.clone());
        }
        Expr::Add { left, right } => {
            collect_field_callees(left, names);
            collect_field_callees(right, names);
        }
        Expr::Coerce { value, ty } if *ty == Type::Field => {
            collect_field_callees(value, names);
        }
        _ => {}
    }
}

fn collect_shared_callees(action: &StateAction, names: &mut HashSet<String>) {
    match action {
        StateAction::CircuitCall { name, .. } => {
            names.insert(name.clone());
        }
        StateAction::Sequence { actions } => {
            for action in actions {
                collect_shared_callees(action, names);
            }
        }
        StateAction::If {
            then, otherwise, ..
        } => {
            collect_shared_callees(then, names);
            collect_shared_callees(otherwise, names);
        }
        StateAction::Let { bindings, action } => {
            for binding in bindings {
                if binding.ty == Type::Field {
                    collect_field_callees(&binding.value, names);
                }
            }
            collect_shared_callees(action, names);
        }
        StateAction::CellWrite { value, .. } => collect_field_callees(value, names),
        _ => {}
    }
}

/// Find recordable Unit and Field-value callees before emitting public entry points. Rendering each
/// candidate without sharing also checks transitive support and rejects recursion.
pub(crate) fn plan_recorded_helpers(
    ordered_circuits: &[StatefulCircuit],
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<(HashSet<String>, Vec<syn::Item>), RenderError> {
    let mut roots_with_calls = HashSet::new();
    for circuit in ordered_circuits.iter().filter(|circuit| !circuit.internal) {
        let mut calls = HashSet::new();
        for action in &circuit.actions {
            collect_shared_callees(action, &mut calls);
        }
        if !calls.is_empty() {
            roots_with_calls.insert(circuit.name.as_str());
        }
    }
    if roots_with_calls.is_empty() {
        return Ok((HashSet::new(), Vec::new()));
    }

    let mut candidates = HashSet::new();
    for circuit in ordered_circuits
        .iter()
        .filter(|circuit| roots_with_calls.contains(circuit.name.as_str()))
    {
        let recorded = crate::located(circuit.source.as_ref(), || {
            render_recorded_circuit(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &HashSet::new(),
            )
        })?;
        if recorded.is_none() {
            continue;
        }
        for action in &circuit.actions {
            collect_shared_callees(action, &mut candidates);
        }
    }
    loop {
        let previous = candidates.len();
        let current = candidates.clone();
        for circuit in ordered_circuits
            .iter()
            .filter(|circuit| current.contains(&circuit.name))
        {
            for action in &circuit.actions {
                collect_shared_callees(action, &mut candidates);
            }
        }
        if candidates.len() == previous {
            break;
        }
    }
    let mut names = HashSet::new();
    for circuit in ordered_circuits {
        let unit_body = circuit.result == Type::Unit && circuit.return_value == StateReturn::Unit;
        let direct_field_body = circuit.result == Type::Field
            && circuit.actions.is_empty()
            && matches!(circuit.return_value, StateReturn::Expression { .. });
        if !candidates.contains(&circuit.name) || !(unit_body || direct_field_body) {
            continue;
        }
        if crate::located(circuit.source.as_ref(), || {
            render_recorded_helper(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &HashSet::new(),
            )
        })?
        .is_some()
        {
            names.insert(circuit.name.clone());
        }
    }
    let mut items = Vec::new();
    for circuit in ordered_circuits {
        if !names.contains(&circuit.name) {
            continue;
        }
        let item = crate::located(circuit.source.as_ref(), || {
            render_recorded_helper(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &names,
            )
        })?;
        if let Some(item) = item {
            items.push(item);
        }
    }
    Ok((names, items))
}

fn render_recorded_item(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    helper: bool,
) -> Result<Option<syn::Item>, RenderError> {
    if circuit.internal && !helper {
        return Ok(None);
    }

    let name = ident(&circuit.name)?;
    let mut parameters = HashMap::new();
    let mut args = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        ident(&parameter.name)?;
        let rust_name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if parameters
            .insert(parameter.name.as_str(), (&parameter.ty, rust_name.clone()))
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
        let arg_ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#rust_name: #arg_ty));
    }

    fn amount_source(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<syn::Expr> {
        match value {
            Expr::Coerce { value, ty }
                if *ty
                    == (Type::Unsigned {
                        max: "65535".into(),
                    }) =>
            {
                amount_source(value, locals, parameters)
            }
            Expr::UnsignedLiteral { value, max } if max == "65535" => {
                let value = value.parse::<u16>().ok()?;
                let literal = syn::LitInt::new(&format!("{value}u16"), Span::call_site());
                Some(syn::parse_quote!(#literal))
            }
            Expr::Parameter { name } => locals.get(name).cloned().or_else(|| {
                let (ty, rust_name) = parameters.get(name.as_str())?;
                if **ty
                    != (Type::Unsigned {
                        max: "65535".into(),
                    })
                {
                    return None;
                }
                Some(syn::parse_quote!(#rust_name.value() as u16))
            }),
            _ => None,
        }
    }

    fn cell_source(
        value: &Expr,
        ty: &Type,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<syn::Expr> {
        if !matches!(
            ty,
            Type::Boolean
                | Type::Field
                | Type::Bytes { .. }
                | Type::Unsigned { .. }
                | Type::Enum { .. }
                | Type::Struct { .. }
                | Type::Tuple { .. }
                | Type::Vector { .. }
        ) {
            return None;
        }
        match value {
            Expr::Coerce { value, ty: target } if target == ty => {
                cell_source(value, ty, locals, parameters)
            }
            Expr::Boolean { value } if *ty == Type::Boolean => Some(syn::parse_quote!(#value)),
            Expr::Parameter { name } => locals
                .get(name)
                .cloned()
                .or_else(|| {
                    let (actual, rust_name) = parameters.get(name.as_str())?;
                    (actual == &ty).then(|| syn::parse_quote!(#rust_name))
                })
                .map(|source| {
                    if matches!(
                        ty,
                        Type::Bytes { .. }
                            | Type::Enum { .. }
                            | Type::Struct { .. }
                            | Type::Tuple { .. }
                            | Type::Vector { .. }
                    ) {
                        syn::parse_quote!((#source).clone())
                    } else {
                        source
                    }
                }),
            Expr::EnumVariant {
                ty: variant_ty,
                variant,
            } if variant_ty == ty => {
                let rust_ty = rust_type(ty).ok()?;
                let variant = ident(variant).ok()?;
                Some(syn::parse_quote!(#rust_ty::#variant))
            }
            Expr::Default { ty: default_ty } if default_ty == ty => {
                let rust_ty = rust_type(ty).ok()?;
                Some(syn::parse_quote!(<#rust_ty as Default>::default()))
            }
            Expr::Vector { .. } if matches!(ty, Type::Vector { .. }) => {
                let (rendered, actual) =
                    expression_with_calls(value, parameters, &HashMap::new()).ok()?;
                (actual == *ty).then_some(rendered)
            }
            _ => None,
        }
    }

    fn vector_bindings(
        bindings: &[LocalBinding],
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        steps: &mut Vec<syn::Stmt>,
        next_temp: &mut usize,
    ) -> Result<Option<HashMap<String, syn::Expr>>, RenderError> {
        let mut scoped = locals.clone();
        for binding in bindings {
            if !matches!(binding.ty, Type::Vector { .. }) {
                return Ok(None);
            }
            let Some(value) = cell_source(&binding.value, &binding.ty, &scoped, parameters) else {
                return Ok(None);
            };
            let rust_ty = rust_type(&binding.ty)?;
            let name = syn::Ident::new(
                &format!("__compact_recorded_vector_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(let #name: #rust_ty = #value;));
            scoped.insert(binding.name.clone(), syn::parse_quote!(#name));
        }
        Ok(Some(scoped))
    }

    /// Lower a Field expression together with its ordered recording effects.
    /// The returned syntax refers only to values already evaluated in `steps`.
    fn field_expression(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ledger_fields: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
        circuits: &HashMap<&str, &StatefulCircuit>,
        shared_callees: &HashSet<String>,
        steps: &mut Vec<syn::Stmt>,
        next_temp: &mut usize,
        visiting: &mut HashSet<String>,
    ) -> Result<Option<syn::Expr>, RenderError> {
        match value {
            Expr::Let { bindings, body } => {
                let Some(scoped) = vector_bindings(bindings, locals, parameters, steps, next_temp)?
                else {
                    return Ok(None);
                };
                field_expression(
                    body,
                    &scoped,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    shared_callees,
                    steps,
                    next_temp,
                    visiting,
                )
            }
            Expr::Coerce { value, ty } if *ty == Type::Field => field_expression(
                value,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
            ),
            Expr::Parameter { .. } => Ok(cell_source(value, &Type::Field, locals, parameters)),
            Expr::FieldLiteral { .. } => {
                let (literal, ty) = expression_with_calls(value, parameters, &HashMap::new())?;
                Ok((ty == Type::Field).then_some(literal))
            }
            Expr::CellRead { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration != (LedgerFieldKind::Cell { ty: Type::Field })
                    || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(None);
                }
                let slot = ident(field)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_value_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote! {
                    let (frame, #observed): (_, runtime::Field) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                });
                Ok(Some(syn::parse_quote!(#observed)))
            }
            Expr::WitnessCall { name, arguments } => {
                let declaration = witnesses
                    .get(name.as_str())
                    .ok_or_else(|| RenderError::UnknownWitness(name.clone()))?;
                if declaration.result != Type::Field {
                    return Ok(None);
                }
                if arguments.len() != declaration.parameters.len() {
                    return Err(RenderError::ArgumentCount {
                        circuit: name.clone(),
                        expected: declaration.parameters.len(),
                        actual: arguments.len(),
                    });
                }
                let mut args = Vec::new();
                for (argument, parameter) in arguments.iter().zip(&declaration.parameters) {
                    let value = if parameter.ty == Type::Field {
                        field_expression(
                            argument,
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
                    } else {
                        cell_source(argument, &parameter.ty, locals, parameters)
                    };
                    let Some(value) = value else {
                        return Ok(None);
                    };
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #arg = #value;));
                    args.push(arg);
                }
                let method = ident(name)?;
                let observed = syn::Ident::new(
                    &format!("__compact_witness_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote! {
                    let (frame, #observed) = frame.try_witness_metered(|context, meter| {
                        witnesses.#method(
                            context.witness_context_with(super::LedgerView {
                                state: context.query.state.get_ref(),
                                meter,
                            }),
                            #(#args),*
                        )
                    })?;
                });
                Ok(Some(syn::parse_quote!(#observed)))
            }
            Expr::Add { left, right } => {
                let Some(left) = field_expression(
                    left,
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
                    return Ok(None);
                };
                let Some(right) = field_expression(
                    right,
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
                    return Ok(None);
                };
                let sum = syn::Ident::new(
                    &format!("__compact_recorded_sum_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #sum: runtime::Field = #left + #right;));
                Ok(Some(syn::parse_quote!(#sum)))
            }
            Expr::Call { name, arguments } => {
                let Some(callee) = circuits.get(name.as_str()) else {
                    return Ok(None);
                };
                let StateReturn::Expression { value: result } = &callee.return_value else {
                    return Ok(None);
                };
                if callee.result != Type::Field {
                    return Ok(None);
                }
                if arguments.len() != callee.parameters.len() {
                    return Err(RenderError::ArgumentCount {
                        circuit: name.clone(),
                        expected: callee.parameters.len(),
                        actual: arguments.len(),
                    });
                }
                if visiting.contains(name) {
                    return Err(RenderError::UnsupportedStatefulCall(name.clone()));
                }
                if shared_callees.contains(name) {
                    return shared_field_call(
                        name,
                        arguments,
                        locals,
                        parameters,
                        ledger_fields,
                        witnesses,
                        circuits,
                        steps,
                        next_temp,
                        visiting,
                    );
                }
                if !visiting.insert(name.clone()) {
                    return Err(RenderError::UnsupportedStatefulCall(name.clone()));
                }
                let mut callee_locals = HashMap::new();
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let value = if parameter.ty == Type::Field {
                        field_expression(
                            argument,
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
                    } else if parameter.ty
                        == (Type::Unsigned {
                            max: "65535".into(),
                        })
                    {
                        amount_source(argument, locals, parameters)
                    } else {
                        cell_source(argument, &parameter.ty, locals, parameters)
                    };
                    let Some(value) = value else {
                        visiting.remove(name);
                        return Ok(None);
                    };
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #arg = #value;));
                    callee_locals.insert(parameter.name.clone(), syn::parse_quote!(#arg));
                }
                for action in &callee.actions {
                    if !append_steps(
                        action,
                        &callee_locals,
                        &HashMap::new(),
                        ledger_fields,
                        witnesses,
                        circuits,
                        shared_callees,
                        steps,
                        next_temp,
                        visiting,
                    )? {
                        visiting.remove(name);
                        return Ok(None);
                    }
                }
                let value = field_expression(
                    result,
                    &callee_locals,
                    &HashMap::new(),
                    ledger_fields,
                    witnesses,
                    circuits,
                    shared_callees,
                    steps,
                    next_temp,
                    visiting,
                )?;
                visiting.remove(name);
                let Some(value) = value else {
                    return Ok(None);
                };
                let returned = syn::Ident::new(
                    &format!("__compact_recorded_return_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #returned: runtime::Field = #value;));
                Ok(Some(syn::parse_quote!(#returned)))
            }
            Expr::MapLookup { field, index, key } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Map {
                    key: key_ty,
                    value: value_ty,
                } = &declaration.declaration
                else {
                    return Ok(None);
                };
                if *value_ty != Type::Field
                    || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(None);
                }
                let Some(key) = scalar_expression(
                    key,
                    key_ty,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?
                else {
                    return Ok(None);
                };
                let slot = ident(field)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_lookup_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(
                    let (frame, #observed): (_, runtime::Field) =
                        crate::ledger_slots::#slot.record_lookup(frame, #key)?;
                ));
                Ok(Some(syn::parse_quote!(#observed)))
            }
            _ => Ok(None),
        }
    }

    fn boolean_expression(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ledger_fields: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
        circuits: &HashMap<&str, &StatefulCircuit>,
        steps: &mut Vec<syn::Stmt>,
        next_temp: &mut usize,
        visiting: &mut HashSet<String>,
    ) -> Result<Option<syn::Expr>, RenderError> {
        match value {
            Expr::Let { bindings, body } => {
                let Some(scoped) = vector_bindings(bindings, locals, parameters, steps, next_temp)?
                else {
                    return Ok(None);
                };
                boolean_expression(
                    body,
                    &scoped,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )
            }
            Expr::Coerce { value, ty } if *ty == Type::Boolean => boolean_expression(
                value,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            ),
            Expr::Equal { left, right } | Expr::NotEqual { left, right } => {
                let (field, index) = match (&**left, &**right) {
                    (Expr::CellRead { field, index }, _) | (_, Expr::CellRead { field, index }) => {
                        (field, index)
                    }
                    _ => return Ok(None),
                };
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                    return Ok(None);
                };
                if declaration.index != *index || declaration.physical_path().len() != 1 {
                    return Ok(None);
                }
                let mut operand = |value: &Expr| -> Result<Option<syn::Expr>, RenderError> {
                    if let Expr::CellRead {
                        field: read_field,
                        index: read_index,
                    } = value
                    {
                        if read_field != field || read_index != index {
                            return Ok(None);
                        }
                        let slot = ident(field)?;
                        let observed = syn::Ident::new(
                            &format!("__compact_recorded_value_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        let value_ty = rust_type(ty)?;
                        steps.push(syn::parse_quote! {
                            let (frame, #observed): (_, #value_ty) =
                                crate::ledger_slots::#slot.record_read(frame)?;
                        });
                        return Ok(Some(syn::parse_quote!(#observed)));
                    }
                    Ok(cell_source(value, ty, locals, parameters))
                };
                let Some(left) = operand(left)? else {
                    return Ok(None);
                };
                let Some(right) = operand(right)? else {
                    return Ok(None);
                };
                let compared = syn::Ident::new(
                    &format!("__compact_recorded_compare_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                if matches!(value, Expr::Equal { .. }) {
                    steps.push(syn::parse_quote!(let #compared = #left == #right;));
                } else {
                    steps.push(syn::parse_quote!(let #compared = #left != #right;));
                }
                Ok(Some(syn::parse_quote!(#compared)))
            }
            Expr::Call { name, arguments } => {
                let Some(callee) = circuits.get(name.as_str()) else {
                    return Ok(None);
                };
                let StateReturn::Expression { value: result } = &callee.return_value else {
                    return Ok(None);
                };
                if callee.result != Type::Boolean || !callee.actions.is_empty() {
                    return Ok(None);
                }
                if arguments.len() != callee.parameters.len() {
                    return Err(RenderError::ArgumentCount {
                        circuit: name.clone(),
                        expected: callee.parameters.len(),
                        actual: arguments.len(),
                    });
                }
                if !visiting.insert(name.clone()) {
                    return Err(RenderError::UnsupportedStatefulCall(name.clone()));
                }
                let mut callee_locals = HashMap::new();
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let Some(value) = cell_source(argument, &parameter.ty, locals, parameters)
                    else {
                        visiting.remove(name);
                        return Ok(None);
                    };
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #arg = #value;));
                    callee_locals.insert(parameter.name.clone(), syn::parse_quote!(#arg));
                }
                let value = boolean_expression(
                    result,
                    &callee_locals,
                    &HashMap::new(),
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                );
                visiting.remove(name);
                value
            }
            Expr::SetMember {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                    return Ok(None);
                };
                if declaration.index != *index {
                    return Ok(None);
                }
                if !matches!(
                    ty,
                    Type::Field
                        | Type::Boolean
                        | Type::Tuple { .. }
                        | Type::Struct { .. }
                        | Type::Vector { .. }
                ) {
                    return Ok(None);
                }
                let key = scalar_expression(
                    value,
                    ty,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?;
                let Some(key) = key else { return Ok(None) };
                let slot = ident(field)?;
                let key_name = syn::Ident::new(
                    &format!("__compact_recorded_key_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_member_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #key_name = #key;));
                steps.push(syn::parse_quote!(
                    let (frame, #observed): (_, bool) =
                        crate::ledger_slots::#slot.record_member(frame, #key_name)?;
                ));
                Ok(Some(syn::parse_quote!(#observed)))
            }
            Expr::MapMember { field, index, key } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Map { key: key_ty, .. } = &declaration.declaration else {
                    return Ok(None);
                };
                if declaration.index != *index || declaration.physical_path().len() != 1 {
                    return Ok(None);
                }
                let Some(key) = scalar_expression(
                    key,
                    key_ty,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?
                else {
                    return Ok(None);
                };
                let slot = ident(field)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_member_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(
                    let (frame, #observed): (_, bool) =
                        crate::ledger_slots::#slot.record_member(frame, #key)?;
                ));
                Ok(Some(syn::parse_quote!(#observed)))
            }
            _ => Ok(cell_source(value, &Type::Boolean, locals, parameters)),
        }
    }

    fn scalar_expression(
        value: &Expr,
        ty: &Type,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ledger_fields: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
        circuits: &HashMap<&str, &StatefulCircuit>,
        steps: &mut Vec<syn::Stmt>,
        next_temp: &mut usize,
        visiting: &mut HashSet<String>,
    ) -> Result<Option<syn::Expr>, RenderError> {
        match ty {
            Type::Field => field_expression(
                value,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                &HashSet::new(),
                steps,
                next_temp,
                visiting,
            ),
            Type::Boolean => boolean_expression(
                value,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            ),
            Type::Bytes { .. }
            | Type::Unsigned { .. }
            | Type::Enum { .. }
            | Type::Struct { .. }
            | Type::Tuple { .. }
            | Type::Vector { .. } => Ok(cell_source(value, ty, locals, parameters)),
            _ => Ok(None),
        }
    }

    /// Keep shared Unit and value-returning helpers on the same typed argument path.
    /// `field_expression` appends effects before the caller binds each argument.
    fn shared_call_argument(
        argument: &Expr,
        ty: &Type,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ledger_fields: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
        circuits: &HashMap<&str, &StatefulCircuit>,
        steps: &mut Vec<syn::Stmt>,
        next_temp: &mut usize,
        visiting: &mut HashSet<String>,
    ) -> Result<Option<syn::Expr>, RenderError> {
        if *ty == Type::Field {
            return field_expression(
                argument,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                &HashSet::new(),
                steps,
                next_temp,
                visiting,
            );
        }
        if *ty
            == (Type::Unsigned {
                max: "65535".into(),
            })
        {
            let uncoerced = match argument {
                Expr::Coerce { value, ty: target } if target == ty => value.as_ref(),
                _ => argument,
            };
            return Ok(match uncoerced {
                Expr::Parameter { name } if !locals.contains_key(name) => {
                    cell_source(argument, ty, locals, parameters)
                }
                _ => amount_source(argument, locals, parameters).map(|value| {
                    syn::parse_quote!(
                        runtime::BoundedUint::<65535>::new((#value) as u128)
                            .expect("Compact Uint argument fits its maximum")
                    )
                }),
            });
        }
        Ok(cell_source(argument, ty, locals, parameters))
    }

    /// Reuse one typed callee body without opening or finishing another frame.
    fn shared_field_call(
        name: &str,
        arguments: &[Expr],
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ledger_fields: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
        circuits: &HashMap<&str, &StatefulCircuit>,
        steps: &mut Vec<syn::Stmt>,
        next_temp: &mut usize,
        visiting: &mut HashSet<String>,
    ) -> Result<Option<syn::Expr>, RenderError> {
        let callee = circuits
            .get(name)
            .ok_or_else(|| RenderError::UnknownCircuit(name.to_owned()))?;
        if callee.result != Type::Field || arguments.len() != callee.parameters.len() {
            return Ok(None);
        }
        let mut args = Vec::new();
        for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
            let Some(value) = shared_call_argument(
                argument,
                &parameter.ty,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                steps,
                next_temp,
                visiting,
            )?
            else {
                return Ok(None);
            };
            let arg = syn::Ident::new(
                &format!("__compact_recorded_arg_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            let arg_ty = rust_type(&parameter.ty)?;
            steps.push(syn::parse_quote!(let #arg: #arg_ty = #value;));
            args.push(arg);
        }
        let observed = syn::Ident::new(
            &format!("__compact_recorded_value_{}", *next_temp),
            Span::call_site(),
        );
        *next_temp += 1;
        let helper = helper_ident(name, circuits)?;
        if circuit_uses_witness(callee, circuits, &mut HashSet::new())? {
            steps.push(syn::parse_quote!(
                let (frame, #observed) = #helper(frame, witnesses, #(#args),*)?;
            ));
        } else {
            steps.push(syn::parse_quote!(
                let (frame, #observed) = #helper(frame, #(#args),*)?;
            ));
        }
        Ok(Some(syn::parse_quote!(#observed)))
    }

    fn append_steps(
        action: &StateAction,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ledger_fields: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
        circuits: &HashMap<&str, &StatefulCircuit>,
        shared_callees: &HashSet<String>,
        steps: &mut Vec<syn::Stmt>,
        next_temp: &mut usize,
        visiting: &mut HashSet<String>,
    ) -> Result<bool, RenderError> {
        match action {
            StateAction::Sequence { actions } => {
                for action in actions {
                    if !append_steps(
                        action,
                        locals,
                        parameters,
                        ledger_fields,
                        witnesses,
                        circuits,
                        shared_callees,
                        steps,
                        next_temp,
                        visiting,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            StateAction::Let { bindings, action } => {
                let mut scoped = locals.clone();
                for binding in bindings {
                    if binding.ty
                        == (Type::Unsigned {
                            max: "65535".into(),
                        })
                    {
                        let Some(value) = amount_source(&binding.value, &scoped, parameters) else {
                            return Ok(false);
                        };
                        scoped.insert(binding.name.clone(), value);
                    } else if binding.ty == Type::Field {
                        if let Expr::Call { name, arguments } = &binding.value {
                            if shared_callees.contains(name) {
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
                                    return Ok(false);
                                };
                                scoped.insert(binding.name.clone(), observed);
                                continue;
                            }
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
                            return Ok(false);
                        };
                        scoped.insert(binding.name.clone(), value);
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
                            return Ok(false);
                        };
                        scoped.insert(binding.name.clone(), value);
                    } else if let Expr::WitnessCall { name, arguments } = &binding.value {
                        if !matches!(binding.ty, Type::Boolean | Type::Field | Type::Bytes { .. }) {
                            return Ok(false);
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
                            let Some(arg) =
                                cell_source(argument, &parameter.ty, &scoped, parameters)
                            else {
                                return Ok(false);
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
                    } else if let Expr::Call { name, arguments } = &binding.value {
                        if !matches!(binding.ty, Type::Bytes { .. })
                            || circuits.contains_key(name.as_str())
                        {
                            return Ok(false);
                        }
                        let mut args = Vec::new();
                        for argument in arguments {
                            let Expr::Coerce { ty, .. } = argument else {
                                return Ok(false);
                            };
                            let Some(value) = cell_source(argument, ty, &scoped, parameters) else {
                                return Ok(false);
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
                    } else if matches!(binding.ty, Type::Bytes { .. }) {
                        let Some(value) =
                            cell_source(&binding.value, &binding.ty, &scoped, parameters)
                        else {
                            return Ok(false);
                        };
                        scoped.insert(binding.name.clone(), value);
                    } else if matches!(binding.ty, Type::Vector { .. }) {
                        let Some(next) = vector_bindings(
                            std::slice::from_ref(binding),
                            &scoped,
                            parameters,
                            steps,
                            next_temp,
                        )?
                        else {
                            return Ok(false);
                        };
                        scoped = next;
                    } else {
                        return Ok(false);
                    }
                }
                append_steps(
                    action,
                    &scoped,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    shared_callees,
                    steps,
                    next_temp,
                    visiting,
                )
            }
            StateAction::Assert { condition, message } => {
                let Some(condition) = boolean_expression(
                    condition,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?
                else {
                    return Ok(false);
                };
                steps.push(syn::parse_quote! {
                    if !(#condition) {
                        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                    }
                });
                Ok(true)
            }
            StateAction::CircuitCall { name, arguments } => {
                let callee = circuits
                    .get(name.as_str())
                    .ok_or_else(|| RenderError::UnsupportedStatefulCall(name.clone()))?;
                if callee.result != Type::Unit || callee.return_value != StateReturn::Unit {
                    return Ok(false);
                }
                if arguments.len() != callee.parameters.len() {
                    return Err(RenderError::ArgumentCount {
                        circuit: name.clone(),
                        expected: callee.parameters.len(),
                        actual: arguments.len(),
                    });
                }
                if shared_callees.contains(name) {
                    let mut args = Vec::new();
                    for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                        let value = shared_call_argument(
                            argument,
                            &parameter.ty,
                            locals,
                            parameters,
                            ledger_fields,
                            witnesses,
                            circuits,
                            steps,
                            next_temp,
                            visiting,
                        )?;
                        let Some(value) = value else { return Ok(false) };
                        let arg = syn::Ident::new(
                            &format!("__compact_recorded_arg_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        let arg_ty = rust_type(&parameter.ty)?;
                        steps.push(syn::parse_quote!(let #arg: #arg_ty = #value;));
                        args.push(arg);
                    }
                    let helper = helper_ident(name, circuits)?;
                    if circuit_uses_witness(callee, circuits, &mut HashSet::new())? {
                        steps.push(syn::parse_quote!(
                            let (frame, _) = #helper(frame, witnesses, #(#args),*)?;
                        ));
                    } else {
                        steps.push(syn::parse_quote!(
                            let (frame, _) = #helper(frame, #(#args),*)?;
                        ));
                    }
                    return Ok(true);
                }
                if !visiting.insert(name.clone()) {
                    return Err(RenderError::UnsupportedStatefulCall(name.clone()));
                }
                let mut callee_locals = HashMap::new();
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let value = if parameter.ty == Type::Field {
                        field_expression(
                            argument,
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
                    } else if parameter.ty
                        == (Type::Unsigned {
                            max: "65535".into(),
                        })
                    {
                        amount_source(argument, locals, parameters)
                    } else {
                        cell_source(argument, &parameter.ty, locals, parameters)
                    };
                    let Some(value) = value else {
                        visiting.remove(name);
                        return Ok(false);
                    };
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #arg = #value;));
                    callee_locals.insert(parameter.name.clone(), syn::parse_quote!(#arg));
                }
                let mut complete = true;
                for action in &callee.actions {
                    if !append_steps(
                        action,
                        &callee_locals,
                        &HashMap::new(),
                        ledger_fields,
                        witnesses,
                        circuits,
                        shared_callees,
                        steps,
                        next_temp,
                        visiting,
                    )? {
                        complete = false;
                        break;
                    }
                }
                visiting.remove(name);
                Ok(complete)
            }
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
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(field)?;
                let amount: syn::Expr = match amount {
                    CounterAmount::Literal { value } => {
                        let literal = syn::LitInt::new(&format!("{value}u16"), Span::call_site());
                        syn::parse_quote!(#literal)
                    }
                    CounterAmount::Parameter { name } => {
                        let Some(value) = amount_source(
                            &Expr::Parameter { name: name.clone() },
                            locals,
                            parameters,
                        ) else {
                            return Ok(false);
                        };
                        value
                    }
                };
                let method = if matches!(action, StateAction::CounterIncrement { .. }) {
                    syn::Ident::new("record_increment", Span::call_site())
                } else {
                    syn::Ident::new("record_decrement", Span::call_site())
                };
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.#method(frame, #amount)?;
                ));
                Ok(true)
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
                    return Ok(false);
                };
                if declaration.index != *index {
                    return Ok(false);
                }
                if !matches!(
                    ty,
                    Type::Field
                        | Type::Boolean
                        | Type::Tuple { .. }
                        | Type::Struct { .. }
                        | Type::Vector { .. }
                ) {
                    return Ok(false);
                }
                let value = scalar_expression(
                    value,
                    ty,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?;
                let Some(value) = value else { return Ok(false) };
                let slot = ident(field)?;
                let method = if matches!(action, StateAction::SetInsert { .. }) {
                    syn::Ident::new("record_insert", Span::call_site())
                } else {
                    syn::Ident::new("record_remove", Span::call_site())
                };
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.#method(frame, #value)?;
                ));
                Ok(true)
            }
            StateAction::SetReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                    || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(false);
                }
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_reset(frame)?;
                ));
                Ok(true)
            }
            StateAction::ListPushFront {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::List { ty } = &declaration.declaration else {
                    return Ok(false);
                };
                if declaration.index != *index || declaration.physical_path().len() != 1 {
                    return Ok(false);
                }
                let value = scalar_expression(
                    value,
                    ty,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?;
                let Some(value) = value else { return Ok(false) };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_push_front(frame, #value)?;
                ));
                Ok(true)
            }
            StateAction::ListPopFront { field, index }
            | StateAction::ListReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                    || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(false);
                }
                let slot = ident(field)?;
                let method = if matches!(action, StateAction::ListPopFront { .. }) {
                    syn::Ident::new("record_pop_front", Span::call_site())
                } else {
                    syn::Ident::new("record_reset", Span::call_site())
                };
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.#method(frame)?;
                ));
                Ok(true)
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
                    return Ok(false);
                };
                if declaration.index != *index || declaration.physical_path().len() != 1 {
                    return Ok(false);
                }
                let key = scalar_expression(
                    key,
                    key_ty,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?;
                let Some(key) = key else { return Ok(false) };
                let key_name = syn::Ident::new(
                    &format!("__compact_recorded_key_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #key_name = #key;));
                let value = scalar_expression(
                    value,
                    value_ty,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?;
                let Some(value) = value else { return Ok(false) };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert(frame, #key_name, #value)?;
                ));
                Ok(true)
            }
            StateAction::MapInsertDefault { field, index, key }
            | StateAction::MapRemove { field, index, key } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Map {
                    key: key_ty,
                    value: value_ty,
                } = &declaration.declaration
                else {
                    return Ok(false);
                };
                if declaration.index != *index
                    || declaration.physical_path().len() != 1
                    || (matches!(action, StateAction::MapInsertDefault { .. })
                        && !matches!(value_ty, Type::Field | Type::Boolean))
                {
                    return Ok(false);
                }
                let key = scalar_expression(
                    key,
                    key_ty,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    steps,
                    next_temp,
                    visiting,
                )?;
                let Some(key) = key else { return Ok(false) };
                let slot = ident(field)?;
                let method = if matches!(action, StateAction::MapInsertDefault { .. }) {
                    syn::Ident::new("record_insert_default", Span::call_site())
                } else {
                    syn::Ident::new("record_remove", Span::call_site())
                };
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.#method(frame, #key)?;
                ));
                Ok(true)
            }
            StateAction::MapReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                    || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(false);
                }
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_reset(frame)?;
                ));
                Ok(true)
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
                    _ => return Ok(false),
                };
                if declaration.index != *index
                    || declaration.physical_path().len() != 1
                    || historic != matches!(action, StateAction::HistoricMerkleInsert { .. })
                {
                    return Ok(false);
                }
                let Some(value) = cell_source(value, ty, locals, parameters) else {
                    return Ok(false);
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert(frame, #value)?;
                ));
                Ok(true)
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
                    return Ok(false);
                };
                if !matches!(
                    ty,
                    Type::Boolean | Type::Field | Type::Bytes { .. } | Type::Enum { .. }
                ) {
                    return Ok(false);
                }
                if declaration.index != *index || declaration.physical_path().len() != 1 {
                    return Ok(false);
                }
                let value = if *ty == Type::Field {
                    field_expression(
                        value,
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
                } else if *ty == Type::Boolean {
                    boolean_expression(
                        value,
                        locals,
                        parameters,
                        ledger_fields,
                        witnesses,
                        circuits,
                        steps,
                        next_temp,
                        visiting,
                    )?
                } else {
                    cell_source(value, ty, locals, parameters)
                };
                let Some(value) = value else {
                    return Ok(false);
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_write(frame, #value)?;
                ));
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    let mut steps = Vec::<syn::Stmt>::new();
    let mut next_temp = 0;
    let mut visiting = HashSet::from([circuit.name.clone()]);
    for action in &circuit.actions {
        if !append_steps(
            action,
            &HashMap::new(),
            &parameters,
            ledger_fields,
            witnesses,
            circuits,
            shared_callees,
            &mut steps,
            &mut next_temp,
            &mut visiting,
        )? {
            return Ok(None);
        }
    }
    let result_ty = rust_type(&circuit.result)?;
    let (return_steps, result): (Vec<syn::Stmt>, syn::Expr) = match &circuit.return_value {
        StateReturn::Unit if circuit.result == Type::Unit => {
            if steps.is_empty() {
                return Ok(None);
            }
            (Vec::new(), syn::parse_quote!(()))
        }
        StateReturn::CounterRead { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, u64) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                )],
                syn::parse_quote!(
                    runtime::BoundedUint::<18446744073709551615>::new(observed as u128)
                        .expect("ledger Counter fits Uint<64>")
                ),
            )
        }
        StateReturn::CellRead { field, index }
            if matches!(
                circuit.result,
                Type::Boolean | Type::Field | Type::Bytes { .. } | Type::Enum { .. }
            ) =>
        {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration
                != (LedgerFieldKind::Cell {
                    ty: circuit.result.clone(),
                })
                || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, #result_ty) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                )],
                syn::parse_quote!(observed),
            )
        }
        StateReturn::Expression { value }
            if helper && circuit.result == Type::Field && circuit.actions.is_empty() =>
        {
            let mut return_steps = Vec::new();
            let Some(result) = field_expression(
                value,
                &HashMap::new(),
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                shared_callees,
                &mut return_steps,
                &mut next_temp,
                &mut visiting,
            )?
            else {
                return Ok(None);
            };
            (return_steps, result)
        }
        StateReturn::Expression {
            value:
                Expr::If {
                    condition,
                    then,
                    otherwise,
                },
        } => {
            fn pure_branch(
                value: &Expr,
                expected: &Type,
                parameters: &HashMap<&str, (&Type, syn::Ident)>,
                ledger_fields: &HashMap<&str, &LedgerField>,
                witnesses: &HashMap<&str, &WitnessDeclaration>,
                pure_circuits: &HashMap<&str, &PureCircuit>,
                circuits: &HashMap<&str, &StatefulCircuit>,
                next_temp: &mut usize,
                visiting: &mut HashSet<String>,
            ) -> Result<Option<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
                let Expr::Call { name, arguments } = value else {
                    return Ok(None);
                };
                let Some(callee) = pure_circuits.get(name.as_str()) else {
                    return Ok(None);
                };
                if callee.result != *expected || arguments.len() != callee.parameters.len() {
                    return Ok(None);
                }
                let mut steps = Vec::new();
                let mut args = Vec::new();
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let value = if parameter.ty == Type::Field {
                        field_expression(
                            argument,
                            &HashMap::new(),
                            parameters,
                            ledger_fields,
                            witnesses,
                            circuits,
                            &HashSet::new(),
                            &mut steps,
                            next_temp,
                            visiting,
                        )?
                    } else {
                        cell_source(argument, &parameter.ty, &HashMap::new(), parameters)
                    };
                    let Some(value) = value else { return Ok(None) };
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #arg = #value;));
                    args.push(arg);
                }
                let method = ident(name)?;
                Ok(Some((
                    steps,
                    syn::parse_quote!(crate::pure_circuits::#method(#(#args),*)?),
                )))
            }

            let mut return_steps = Vec::new();
            let Some(condition) = boolean_expression(
                condition,
                &HashMap::new(),
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut return_steps,
                &mut next_temp,
                &mut visiting,
            )?
            else {
                return Ok(None);
            };
            let Some((then_steps, then_result)) = pure_branch(
                then,
                &circuit.result,
                &parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &mut next_temp,
                &mut visiting,
            )?
            else {
                return Ok(None);
            };
            let Some((otherwise_steps, otherwise_result)) = pure_branch(
                otherwise,
                &circuit.result,
                &parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &mut next_temp,
                &mut visiting,
            )?
            else {
                return Ok(None);
            };
            return_steps.push(syn::parse_quote! {
                let (frame, observed): (_, #result_ty) = if #condition {
                    #(#then_steps)*
                    (frame, #then_result)
                } else {
                    #(#otherwise_steps)*
                    (frame, #otherwise_result)
                };
            });
            (return_steps, syn::parse_quote!(observed))
        }
        StateReturn::SetMember {
            field,
            index,
            value,
        } if circuit.result == Type::Boolean => {
            let mut return_steps = Vec::new();
            let expression = Expr::SetMember {
                field: field.clone(),
                index: *index,
                value: Box::new(value.clone()),
            };
            let Some(result) = boolean_expression(
                &expression,
                &HashMap::new(),
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut return_steps,
                &mut next_temp,
                &mut visiting,
            )?
            else {
                return Ok(None);
            };
            (return_steps, result)
        }
        StateReturn::SetSize { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                || declaration.index != *index
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, u64) =
                        crate::ledger_slots::#slot.record_size(frame)?;
                )],
                syn::parse_quote!(
                    runtime::BoundedUint::<18446744073709551615>::new(observed as u128)
                        .expect("ledger Set size fits Uint<64>")
                ),
            )
        }
        StateReturn::SetIsEmpty { field, index } if circuit.result == Type::Boolean => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                || declaration.index != *index
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, bool) =
                        crate::ledger_slots::#slot.record_is_empty(frame)?;
                )],
                syn::parse_quote!(observed),
            )
        }
        StateReturn::ListLength { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, u64) =
                        crate::ledger_slots::#slot.record_length(frame)?;
                )],
                syn::parse_quote!(
                    runtime::BoundedUint::<18446744073709551615>::new(observed as u128)
                        .expect("ledger List length fits Uint<64>")
                ),
            )
        }
        StateReturn::ListIsEmpty { field, index } if circuit.result == Type::Boolean => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, bool) =
                        crate::ledger_slots::#slot.record_is_empty(frame)?;
                )],
                syn::parse_quote!(observed),
            )
        }
        StateReturn::ListHead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::List { ty } = &declaration.declaration else {
                return Ok(None);
            };
            let expected = list_head_result_type(ty, &circuit.result);
            if declaration.index != *index
                || declaration.physical_path().len() != 1
                || circuit.result != expected
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, #result_ty) =
                        crate::ledger_slots::#slot.record_head::<#result_ty, _, _>(frame)?;
                )],
                syn::parse_quote!(observed),
            )
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
                return Ok(None);
            };
            let member = matches!(&circuit.return_value, StateReturn::MapMember { .. });
            let expected = if member {
                Type::Boolean
            } else {
                value_ty.clone()
            };
            if declaration.index != *index
                || declaration.physical_path().len() != 1
                || circuit.result != expected
                || !matches!(expected, Type::Boolean | Type::Field)
            {
                return Ok(None);
            }
            let mut return_steps = Vec::new();
            let key = scalar_expression(
                key,
                key_ty,
                &HashMap::new(),
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut return_steps,
                &mut next_temp,
                &mut visiting,
            )?;
            let Some(key) = key else { return Ok(None) };
            let slot = ident(field)?;
            let method = if member {
                "record_member"
            } else {
                "record_lookup"
            };
            let method = syn::Ident::new(method, Span::call_site());
            return_steps.push(syn::parse_quote!(
                let (frame, observed): (_, #result_ty) =
                    crate::ledger_slots::#slot.#method(frame, #key)?;
            ));
            (return_steps, syn::parse_quote!(observed))
        }
        StateReturn::MapSize { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, u64) =
                        crate::ledger_slots::#slot.record_size(frame)?;
                )],
                syn::parse_quote!(
                    runtime::BoundedUint::<18446744073709551615>::new(observed as u128)
                        .expect("ledger Map size fits Uint<64>")
                ),
            )
        }
        StateReturn::MapIsEmpty { field, index } if circuit.result == Type::Boolean => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                || declaration.index != *index
                || declaration.physical_path().len() != 1
            {
                return Ok(None);
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, bool) =
                        crate::ledger_slots::#slot.record_is_empty(frame)?;
                )],
                syn::parse_quote!(observed),
            )
        }
        _ => return Ok(None),
    };
    if steps.is_empty() && return_steps.is_empty() {
        return Ok(None);
    }

    let uses_witness = circuit_uses_witness(circuit, circuits, &mut HashSet::new())?;
    let item = if helper {
        let body_name = helper_ident(&circuit.name, circuits)?;
        if uses_witness {
            syn::parse_quote! {
                fn #body_name<Private, W: super::TryWitnesses<Private>>(
                    frame: runtime::recording::RecordingFrame<Private>,
                    witnesses: &W,
                    #(#args),*
                ) -> Result<(runtime::recording::RecordingFrame<Private>, #result_ty), runtime::CompactError> {
                    #(#steps)*
                    #(#return_steps)*
                    Ok((frame, #result))
                }
            }
        } else {
            syn::parse_quote! {
                fn #body_name<Private>(
                    frame: runtime::recording::RecordingFrame<Private>,
                    #(#args),*
                ) -> Result<(runtime::recording::RecordingFrame<Private>, #result_ty), runtime::CompactError> {
                    #(#steps)*
                    #(#return_steps)*
                    Ok((frame, #result))
                }
            }
        }
    } else if shared_callees.contains(&circuit.name) {
        let body_name = helper_ident(&circuit.name, circuits)?;
        // Parameter order must follow the Compact declaration, not HashMap order.
        let call_args: Vec<_> = circuit
            .parameters
            .iter()
            .enumerate()
            .map(|(index, _)| {
                syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site())
            })
            .collect();
        if uses_witness {
            syn::parse_quote! {
                pub fn #name<Private, W: super::TryWitnesses<Private>>(
                    context: runtime::context::CircuitContext<Private>,
                    witnesses: &W,
                    #(#args),*
                ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result_ty>, runtime::CompactError> {
                    let frame = runtime::recording::RecordingFrame::new(context);
                    let (frame, result) = #body_name(frame, witnesses, #(#call_args),*)?;
                    Ok(frame.finish(result))
                }
            }
        } else {
            syn::parse_quote! {
                pub fn #name<Private>(
                    context: runtime::context::CircuitContext<Private>,
                    #(#args),*
                ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result_ty>, runtime::CompactError> {
                    let frame = runtime::recording::RecordingFrame::new(context);
                    let (frame, result) = #body_name(frame, #(#call_args),*)?;
                    Ok(frame.finish(result))
                }
            }
        }
    } else if uses_witness {
        syn::parse_quote! {
            pub fn #name<Private, W: super::TryWitnesses<Private>>(
                context: runtime::context::CircuitContext<Private>,
                witnesses: &W,
                #(#args),*
            ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result_ty>, runtime::CompactError> {
                let frame = runtime::recording::RecordingFrame::new(context);
                #(#steps)*
                #(#return_steps)*
                Ok(frame.finish(#result))
            }
        }
    } else {
        syn::parse_quote! {
        pub fn #name<Private>(
            context: runtime::context::CircuitContext<Private>,
            #(#args),*
        ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result_ty>, runtime::CompactError> {
            let frame = runtime::recording::RecordingFrame::new(context);
            #(#steps)*
            #(#return_steps)*
            Ok(frame.finish(#result))
        }
        }
    };
    Ok(Some(item))
}

/// A recording handle that borrows the user-supplied witness implementation.
pub(crate) fn render_borrowed_recorded_contract_method(
    circuit: &StatefulCircuit,
    uses_witness: bool,
) -> Result<syn::ImplItemFn, RenderError> {
    let name = ident(&circuit.name)?;
    let mut args = Vec::<syn::FnArg>::new();
    let mut call_args = Vec::<syn::Ident>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        let arg = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        let ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#arg: #ty));
        call_args.push(arg);
    }
    let result = rust_type(&circuit.result)?;
    let method = if uses_witness {
        syn::parse_quote! {
            pub fn #name<Private>(
                &self,
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result>, runtime::CompactError>
            where W: super::TryWitnesses<Private> {
                #name(context, self.witnesses, #(#call_args),*)
            }
        }
    } else {
        syn::parse_quote! {
            pub fn #name<Private>(
                &self,
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result>, runtime::CompactError> {
                #name(context, #(#call_args),*)
            }
        }
    };
    Ok(method)
}

/// A typed method on the generated recording handle. The ledger program is
/// still produced by the corresponding `recorded` module function.
pub(crate) fn render_recorded_contract_method(
    circuit: &StatefulCircuit,
) -> Result<syn::ImplItemFn, RenderError> {
    let name = ident(&circuit.name)?;
    let mut args = Vec::<syn::FnArg>::new();
    let mut call_args = Vec::<syn::Ident>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        let arg = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        let ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#arg: #ty));
        call_args.push(arg);
    }
    let result = rust_type(&circuit.result)?;
    Ok(syn::parse_quote! {
        pub fn #name<Private>(
            &self,
            context: runtime::context::CircuitContext<Private>,
            #(#args),*
        ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result>, runtime::CompactError> {
            crate::ledger_contract::recorded::#name(context, #(#call_args),*)
        }
    })
}

/// A generated call handle carries the circuit's declared input and entry
/// point into ledger preparation, so consumers cannot repeat them incorrectly.
pub(crate) fn render_observed_call_method(
    circuit: &StatefulCircuit,
    uses_witness: bool,
) -> Result<syn::ImplItemFn, RenderError> {
    let name = ident(&circuit.name)?;
    let call_name = ident(&format!("{}_call", circuit.name))?;
    let entry_point = syn::LitStr::new(&circuit.name, Span::call_site());
    let mut args = Vec::<syn::FnArg>::new();
    let mut call_args = Vec::<syn::Ident>::new();
    let mut input_args = Vec::<syn::Expr>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        let arg = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        let ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#arg: #ty));
        input_args.push(syn::parse_quote!(#arg.clone()));
        call_args.push(arg);
    }
    let input: syn::Expr = match input_args.as_slice() {
        [] => syn::parse_quote!(runtime::fab::AlignedValue::from(())),
        [single] => syn::parse_quote!(runtime::fab::AlignedValue::from(#single)),
        [first, second] => {
            syn::parse_quote!(runtime::fab::AlignedValue::from((#first, #second)))
        }
        _ => syn::parse_quote!(runtime::fab::AlignedValue::concat(&[
            #(runtime::fab::AlignedValue::from(#input_args)),*
        ])),
    };
    let result = rust_type(&circuit.result)?;
    let method = if uses_witness {
        syn::parse_quote! {
            #[cfg(feature = "ledger-transaction")]
            pub fn #call_name<'observed, Private>(
                &self,
                observed: &'observed runtime::transaction::ObservedContractState,
                private_state: Private,
                #(#args),*
            ) -> Result<runtime::transaction::RecordedCall<'observed, Private, #result>, runtime::CompactError>
            where W: super::TryWitnesses<Private> {
                let input = #input;
                let recorded = self.#name(observed.circuit_context(private_state), #(#call_args),*)?;
                Ok(runtime::transaction::RecordedCall::new(observed, recorded, #entry_point, input))
            }
        }
    } else {
        syn::parse_quote! {
            #[cfg(feature = "ledger-transaction")]
            pub fn #call_name<'observed, Private>(
                &self,
                observed: &'observed runtime::transaction::ObservedContractState,
                private_state: Private,
                #(#args),*
            ) -> Result<runtime::transaction::RecordedCall<'observed, Private, #result>, runtime::CompactError> {
                let input = #input;
                let recorded = self.#name(observed.circuit_context(private_state), #(#call_args),*)?;
                Ok(runtime::transaction::RecordedCall::new(observed, recorded, #entry_point, input))
            }
        }
    };
    Ok(method)
}
