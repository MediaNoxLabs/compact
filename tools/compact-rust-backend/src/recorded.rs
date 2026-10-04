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
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::ir::{
    CounterAmount, Expr, LedgerField, LedgerFieldKind, LocalBinding, PureCircuit, StateAction,
    StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::stateful::circuit_uses_witness;
use crate::{
    RenderError, expression_with_calls, ident, list_head_result_type, public_parameter_idents,
    retained_value, rust_type,
};

/// The first definite reason an exported circuit has no recorded Rust API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecordingGap {
    pub code: RecordingGapCode,
    pub ir_node: String,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingGapCode {
    UnsupportedAction,
    UnsupportedExpression,
    UnsupportedType,
    UnsupportedReturn,
    NoRecordedEffect,
    RecordingUnavailable,
    NameCollision,
}

impl RecordingGapCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedAction => "unsupported_action",
            Self::UnsupportedExpression => "unsupported_expression",
            Self::UnsupportedType => "unsupported_type",
            Self::UnsupportedReturn => "unsupported_return",
            Self::NoRecordedEffect => "no_recorded_effect",
            Self::RecordingUnavailable => "recording_unavailable",
            Self::NameCollision => "name_collision",
        }
    }
}

impl RecordingGap {
    fn action(action: &StateAction, path: String) -> Self {
        let ir_node = node_name(action, "StateAction");
        Self {
            code: RecordingGapCode::UnsupportedAction,
            ir_node: ir_node.clone(),
            path,
            detail: format!("{ir_node} is not supported by recorded Rust lowering"),
        }
    }

    fn returned(value: &StateReturn) -> Self {
        let ir_node = node_name(value, "StateReturn");
        Self {
            code: RecordingGapCode::UnsupportedReturn,
            ir_node: ir_node.clone(),
            path: "return_value".to_owned(),
            detail: format!("{ir_node} is not supported by recorded Rust lowering"),
        }
    }

    fn expression(value: &Expr, path: String) -> Self {
        let ir_node = node_name(value, "Expr");
        Self {
            code: RecordingGapCode::UnsupportedExpression,
            ir_node: ir_node.clone(),
            path,
            detail: format!("{ir_node} is not supported in this recorded expression"),
        }
    }

    fn unsupported_type(action: &StateAction, ty: &Type, path: String) -> Self {
        Self {
            code: RecordingGapCode::UnsupportedType,
            ir_node: node_name(action, "StateAction"),
            path,
            detail: format!("{ty:?} Cell recording is unsupported"),
        }
    }

    fn no_effect() -> Self {
        Self {
            code: RecordingGapCode::NoRecordedEffect,
            ir_node: "StatefulCircuit".to_owned(),
            path: "actions".to_owned(),
            detail: "the circuit has no replayable ledger read or write".to_owned(),
        }
    }

    pub(crate) fn recording_dependency(gap: Self) -> Self {
        Self {
            code: RecordingGapCode::RecordingUnavailable,
            ir_node: gap.ir_node,
            path: gap.path,
            detail: "the observed-call API requires a recorded circuit".to_owned(),
        }
    }

    pub(crate) fn name_collision(name: &str) -> Self {
        Self {
            code: RecordingGapCode::NameCollision,
            ir_node: "StatefulCircuit".to_owned(),
            path: "name".to_owned(),
            detail: format!("observed-call method {name:?} collides with an exported circuit"),
        }
    }
}

fn node_name(value: &impl Serialize, prefix: &str) -> String {
    // The tagged IR enum is the source of the stable machine discriminator.
    // This is only invoked on a failed lowering branch, never on supported output.
    let node = serde_json::to_value(value).expect("typed IR serializes");
    let kind = node["kind"].as_str().expect("tagged IR variant has kind");
    let variant = kind
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<String>();
    format!("{prefix}::{variant}")
}

pub(crate) enum RecordingOutcome<T> {
    Supported(T),
    Unsupported(RecordingGap),
}

impl<T> RecordingOutcome<T> {
    pub(crate) fn is_supported(&self) -> bool {
        matches!(self, Self::Supported(_))
    }

    pub(crate) fn gap(&self) -> Option<&RecordingGap> {
        match self {
            Self::Supported(_) => None,
            Self::Unsupported(gap) => Some(gap),
        }
    }
}

fn unavailable_action(action: &StateAction, path: &str) -> RecordingOutcome<()> {
    RecordingOutcome::Unsupported(RecordingGap::action(action, path.to_owned()))
}

/// Keep exported scalar-expression recording tied to a ledger read. Other
/// action-free expressions may still be lowered as private shared helpers.
fn contains_cell_read(value: &Expr) -> bool {
    match value {
        Expr::CellRead { .. } => true,
        Expr::Coerce { value, .. } | Expr::FieldCast { value } => contains_cell_read(value),
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right }
        | Expr::Equal { left, right }
        | Expr::NotEqual { left, right } => contains_cell_read(left) || contains_cell_read(right),
        Expr::Let { bindings, body } => {
            bindings
                .iter()
                .any(|binding| contains_cell_read(&binding.value))
                || contains_cell_read(body)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            contains_cell_read(condition)
                || contains_cell_read(then)
                || contains_cell_read(otherwise)
        }
        Expr::Sequence { steps, value } => {
            steps.iter().any(contains_cell_read) || contains_cell_read(value)
        }
        _ => false,
    }
}

/// These generated Rust types implement the runtime's existing `CellValue`
/// contract, including aligned FAB encoding and checked readback.
fn recordable_cell_type(ty: &Type) -> bool {
    match ty {
        Type::Boolean
        | Type::Field
        | Type::JubjubPoint
        | Type::Bytes { .. }
        | Type::Unsigned { .. }
        | Type::Enum { .. } => true,
        Type::Struct { fields, .. } => fields.iter().all(|field| recordable_cell_type(&field.ty)),
        Type::Tuple { elements } => {
            !elements.is_empty() && elements.len() <= 8 && elements.iter().all(recordable_cell_type)
        }
        Type::Vector { element, .. } => recordable_cell_type(element),
        Type::Unit | Type::OpaqueString | Type::OpaqueBytes | Type::LedgerMap { .. } => false,
    }
}

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
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
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
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
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
        if !recorded.is_supported() {
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
        .is_supported()
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
        if let RecordingOutcome::Supported(item) = item {
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
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
    if circuit.internal && !helper {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::no_effect()));
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
            Expr::UnsignedCast { max, value }
                if max == "65535" && matches!(value.as_ref(), Expr::If { .. }) =>
            {
                amount_source(value, locals, parameters)
            }
            Expr::Coerce { value, ty }
                if *ty
                    == (Type::Unsigned {
                        max: "65535".into(),
                    }) =>
            {
                amount_source(value, locals, parameters)
            }
            Expr::Coerce {
                value,
                ty: Type::Unsigned { max },
            } if max.parse::<u128>().ok()? <= u16::MAX as u128 => {
                amount_source(value, locals, parameters)
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                let condition = cell_source(condition, &Type::Boolean, locals, parameters)?;
                let then = amount_source(then, locals, parameters)?;
                let otherwise = amount_source(otherwise, locals, parameters)?;
                Some(syn::parse_quote!(if #condition { #then } else { #otherwise }))
            }
            Expr::UnsignedLiteral { value, max } if max == "65535" => {
                let value = value.parse::<u16>().ok()?;
                let literal = syn::LitInt::new(&format!("{value}u16"), Span::call_site());
                Some(syn::parse_quote!(#literal))
            }
            Expr::UnsignedLiteral { value, max } => {
                let value = value.parse::<u16>().ok()?;
                if value as u128 > max.parse::<u128>().ok()? {
                    return None;
                }
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
                | Type::JubjubPoint
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
                .map(|source| retained_value(source, ty)),
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
            Expr::UnsignedLiteral { .. } if matches!(ty, Type::Unsigned { .. }) => {
                let (rendered, actual) =
                    expression_with_calls(value, parameters, &HashMap::new()).ok()?;
                (actual == *ty).then_some(rendered)
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
    #[expect(
        clippy::too_many_arguments,
        reason = "recursive expression lowering threads typed lookups and ordered recording state explicitly"
    )]
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
            Expr::Add { left, right }
            | Expr::Subtract { left, right }
            | Expr::Multiply { left, right } => {
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
                let (name, arithmetic): (&str, syn::Expr) = match value {
                    Expr::Add { .. } => ("sum", syn::parse_quote!(#left + #right)),
                    Expr::Subtract { .. } => ("difference", syn::parse_quote!(#left - #right)),
                    Expr::Multiply { .. } => ("product", syn::parse_quote!(#left * #right)),
                    _ => unreachable!(),
                };
                let result = syn::Ident::new(
                    &format!("__compact_recorded_{name}_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(let #result: runtime::Field = #arithmetic;));
                Ok(Some(syn::parse_quote!(#result)))
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
                for (index, action) in callee.actions.iter().enumerate() {
                    if !append_steps(
                        action,
                        &format!("callee[{name}].actions[{index}]"),
                        &callee_locals,
                        &HashMap::new(),
                        ledger_fields,
                        witnesses,
                        circuits,
                        shared_callees,
                        steps,
                        next_temp,
                        visiting,
                    )?
                    .is_supported()
                    {
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
                if *value_ty != Type::Field || declaration.index != *index {
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

    #[expect(
        clippy::too_many_arguments,
        reason = "recursive expression lowering threads typed lookups and ordered recording state explicitly"
    )]
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
            Expr::CellRead { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(
                    declaration.declaration,
                    LedgerFieldKind::Cell { ty: Type::Boolean }
                ) || declaration.index != *index
                {
                    return Ok(None);
                }
                let slot = ident(field)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_bool_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(
                    let (frame, #observed): (_, bool) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                ));
                Ok(Some(syn::parse_quote!(#observed)))
            }
            Expr::SetIsEmpty { field, index } | Expr::MapIsEmpty { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let matching_kind = matches!(
                    (value, &declaration.declaration),
                    (Expr::SetIsEmpty { .. }, LedgerFieldKind::Set { .. })
                        | (Expr::MapIsEmpty { .. }, LedgerFieldKind::Map { .. })
                );
                if !matching_kind || declaration.index != *index {
                    return Ok(None);
                }
                let slot = ident(field)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_empty_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(
                    let (frame, #observed): (_, bool) =
                        crate::ledger_slots::#slot.record_is_empty(frame)?;
                ));
                Ok(Some(syn::parse_quote!(#observed)))
            }
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
                if declaration.index != *index {
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
                if declaration.index != *index {
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

    #[expect(
        clippy::too_many_arguments,
        reason = "recursive expression lowering threads typed lookups and ordered recording state explicitly"
    )]
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
    #[expect(
        clippy::too_many_arguments,
        reason = "shared callee arguments reuse recursive lowering context and ordered recording state"
    )]
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
    #[expect(
        clippy::too_many_arguments,
        reason = "shared callee lowering keeps declaration context and ordered recording state explicit"
    )]
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

    #[expect(
        clippy::too_many_arguments,
        reason = "recursive action lowering keeps typed declarations and ordered recording state explicit"
    )]
    fn append_steps(
        action: &StateAction,
        path: &str,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ledger_fields: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
        circuits: &HashMap<&str, &StatefulCircuit>,
        shared_callees: &HashSet<String>,
        steps: &mut Vec<syn::Stmt>,
        next_temp: &mut usize,
        visiting: &mut HashSet<String>,
    ) -> Result<RecordingOutcome<()>, RenderError> {
        match action {
            StateAction::Sequence { actions } => {
                for (index, action) in actions.iter().enumerate() {
                    if let RecordingOutcome::Unsupported(gap) = append_steps(
                        action,
                        &format!("{path}.actions[{index}]"),
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
                        return Ok(RecordingOutcome::Unsupported(gap));
                    }
                }
                Ok(RecordingOutcome::Supported(()))
            }
            whole @ StateAction::Let {
                bindings,
                action: nested_action,
            } => {
                let mut scoped = locals.clone();
                for (binding_index, binding) in bindings.iter().enumerate() {
                    if binding.ty
                        == (Type::Unsigned {
                            max: "65535".into(),
                        })
                    {
                        let Some(value) = amount_source(&binding.value, &scoped, parameters) else {
                            return Ok(unavailable_action(whole, path));
                        };
                        scoped.insert(binding.name.clone(), value);
                    } else if binding.ty == Type::Field {
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
                                return Ok(RecordingOutcome::Unsupported(
                                    RecordingGap::expression(
                                        &binding.value,
                                        format!("{path}.bindings[{binding_index}].value"),
                                    ),
                                ));
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
                    } else if let Expr::WitnessCall { name, arguments } = &binding.value {
                        if !matches!(
                            binding.ty,
                            Type::Boolean
                                | Type::Field
                                | Type::Bytes { .. }
                                | Type::Unsigned { .. }
                        ) {
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
                            let Some(arg) =
                                cell_source(argument, &parameter.ty, &scoped, parameters)
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
                    } else if let Expr::Call { name, arguments } = &binding.value {
                        if !matches!(binding.ty, Type::Bytes { .. })
                            || circuits.contains_key(name.as_str())
                        {
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
                    } else if matches!(binding.ty, Type::Bytes { .. }) {
                        let Some(value) =
                            cell_source(&binding.value, &binding.ty, &scoped, parameters)
                        else {
                            return Ok(unavailable_action(whole, path));
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
                    return Ok(unavailable_action(action, path));
                };
                steps.push(syn::parse_quote! {
                    if !(#condition) {
                        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                    }
                });
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::CircuitCall { name, arguments } => {
                let callee = circuits
                    .get(name.as_str())
                    .ok_or_else(|| RenderError::UnsupportedStatefulCall(name.clone()))?;
                if callee.result != Type::Unit || callee.return_value != StateReturn::Unit {
                    return Ok(unavailable_action(action, path));
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
                        let Some(value) = value else {
                            return Ok(unavailable_action(action, path));
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
                    return Ok(RecordingOutcome::Supported(()));
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
                        return Ok(unavailable_action(action, path));
                    };
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #arg = #value;));
                    callee_locals.insert(parameter.name.clone(), syn::parse_quote!(#arg));
                }
                let mut failure = None;
                for (index, action) in callee.actions.iter().enumerate() {
                    if let RecordingOutcome::Unsupported(gap) = append_steps(
                        action,
                        &format!("callee[{name}].actions[{index}]"),
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
                        failure = Some(gap);
                        break;
                    }
                }
                visiting.remove(name);
                Ok(failure.map_or(
                    RecordingOutcome::Supported(()),
                    RecordingOutcome::Unsupported,
                ))
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
                            return Ok(unavailable_action(action, path));
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
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::CounterReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_reset(frame)?;
                ));
                Ok(RecordingOutcome::Supported(()))
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
                    return Ok(unavailable_action(action, path));
                };
                if declaration.index != *index {
                    return Ok(unavailable_action(action, path));
                }
                if !matches!(
                    ty,
                    Type::Field
                        | Type::Boolean
                        | Type::Tuple { .. }
                        | Type::Struct { .. }
                        | Type::Vector { .. }
                ) {
                    return Ok(unavailable_action(action, path));
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
                let Some(value) = value else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                let method = if matches!(action, StateAction::SetInsert { .. }) {
                    syn::Ident::new("record_insert", Span::call_site())
                } else {
                    syn::Ident::new("record_remove", Span::call_site())
                };
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.#method(frame, #value)?;
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::SetReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                    || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(unavailable_action(action, path));
                }
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_reset(frame)?;
                ));
                Ok(RecordingOutcome::Supported(()))
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
                    return Ok(unavailable_action(action, path));
                };
                if declaration.index != *index {
                    return Ok(unavailable_action(action, path));
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
                let Some(value) = value else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_push_front(frame, #value)?;
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::ListPopFront { field, index }
            | StateAction::ListReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                    || declaration.index != *index
                {
                    return Ok(unavailable_action(action, path));
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
                Ok(RecordingOutcome::Supported(()))
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
                    return Ok(unavailable_action(action, path));
                };
                if declaration.index != *index {
                    return Ok(unavailable_action(action, path));
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
                let Some(key) = key else {
                    return Ok(unavailable_action(action, path));
                };
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
                let Some(value) = value else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert(frame, #key_name, #value)?;
                ));
                Ok(RecordingOutcome::Supported(()))
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
                    return Ok(unavailable_action(action, path));
                };
                if declaration.index != *index
                    || (matches!(action, StateAction::MapInsertDefault { .. })
                        && !matches!(value_ty, Type::Field | Type::Boolean))
                {
                    return Ok(unavailable_action(action, path));
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
                let Some(key) = key else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                let method = if matches!(action, StateAction::MapInsertDefault { .. }) {
                    syn::Ident::new("record_insert_default", Span::call_site())
                } else {
                    syn::Ident::new("record_remove", Span::call_site())
                };
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.#method(frame, #key)?;
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::MapReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                    || declaration.index != *index
                {
                    return Ok(unavailable_action(action, path));
                }
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_reset(frame)?;
                ));
                Ok(RecordingOutcome::Supported(()))
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
                    _ => {
                        return Ok(unavailable_action(action, path));
                    }
                };
                if declaration.index != *index
                    || declaration.physical_path().len() != 1
                    || historic != matches!(action, StateAction::HistoricMerkleInsert { .. })
                {
                    return Ok(unavailable_action(action, path));
                }
                let Some(value) = cell_source(value, ty, locals, parameters) else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert(frame, #value)?;
                ));
                Ok(RecordingOutcome::Supported(()))
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
                    return Ok(unavailable_action(action, path));
                };
                if !recordable_cell_type(ty) {
                    return Ok(RecordingOutcome::Unsupported(
                        RecordingGap::unsupported_type(action, ty, format!("{path}.value")),
                    ));
                }
                if declaration.index != *index {
                    return Ok(unavailable_action(action, path));
                }
                let expression = value;
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
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        expression,
                        format!("{path}.value"),
                    )));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_write(frame, #value)?;
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            _ => Ok(unavailable_action(action, path)),
        }
    }

    let mut steps = Vec::<syn::Stmt>::new();
    let mut next_temp = 0;
    let mut visiting = HashSet::from([circuit.name.clone()]);
    for (index, action) in circuit.actions.iter().enumerate() {
        if let RecordingOutcome::Unsupported(gap) = append_steps(
            action,
            &format!("actions[{index}]"),
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
            return Ok(RecordingOutcome::Unsupported(gap));
        }
    }
    let result_ty = rust_type(&circuit.result)?;
    let (return_steps, result): (Vec<syn::Stmt>, syn::Expr) = match &circuit.return_value {
        StateReturn::Unit if circuit.result == Type::Unit => {
            if steps.is_empty() {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::no_effect()));
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
        StateReturn::CellRead { field, index } if recordable_cell_type(&circuit.result) => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration
                != (LedgerFieldKind::Cell {
                    ty: circuit.result.clone(),
                })
                || declaration.index != *index
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
            if circuit.result == Type::Field
                && circuit.actions.is_empty()
                && (helper || (contains_cell_read(value) && !matches!(value, Expr::If { .. }))) =>
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    value,
                    "return_value.value".to_owned(),
                )));
            };
            (return_steps, result)
        }
        StateReturn::Expression { value }
            if contains_cell_read(value)
                && circuit.result == Type::Boolean
                && circuit.actions.is_empty()
                && !matches!(value, Expr::If { .. }) =>
        {
            let mut return_steps = Vec::new();
            let Some(result) = boolean_expression(
                value,
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    value,
                    "return_value.value".to_owned(),
                )));
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
            #[expect(
                clippy::too_many_arguments,
                reason = "branch lowering threads typed circuit context and recursive state explicitly"
            )]
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            let expected = list_head_result_type(ty, &circuit.result);
            if declaration.index != *index || circuit.result != expected {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            let member = matches!(&circuit.return_value, StateReturn::MapMember { .. });
            let expected = if member {
                Type::Boolean
            } else {
                value_ty.clone()
            };
            if declaration.index != *index
                || circuit.result != expected
                || !matches!(expected, Type::Boolean | Type::Field)
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
            let Some(key) = key else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
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
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
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
        StateReturn::MerkleIsFull { field, index }
        | StateReturn::HistoricMerkleIsFull { field, index }
            if circuit.result == Type::Boolean && circuit.actions.is_empty() =>
        {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = matches!(
                circuit.return_value,
                StateReturn::HistoricMerkleIsFull { .. }
            );
            let declaration_historic = match declaration.declaration {
                LedgerFieldKind::MerkleTree { .. } => false,
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            if declaration.index != *index || historic != declaration_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, bool) =
                        crate::ledger_slots::#slot.record_is_full(frame)?;
                )],
                syn::parse_quote!(observed),
            )
        }
        StateReturn::Expression {
            value: Expr::Let { bindings, body },
        } if circuit.result == Type::Boolean
            && circuit.actions.is_empty()
            && bindings.len() == 1
            && matches!(body.as_ref(), Expr::MerkleCheckRoot { .. }) =>
        {
            let binding = &bindings[0];
            let Expr::MerkleCheckRoot { field, index, root } = body.as_ref() else {
                unreachable!("guarded by MerkleCheckRoot expression")
            };
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::MerkleTree { .. })
                || declaration.index != *index
                || !matches!(root.as_ref(), Expr::Parameter { name } if *name == binding.name)
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    body,
                    "return_value.value".to_owned(),
                )));
            }
            let Expr::Call { name, arguments } = &binding.value else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    &binding.value,
                    "return_value.value.bindings[0].value".to_owned(),
                )));
            };
            let pure = pure_circuits
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
            if name != "merkleTreePathRoot"
                || pure.result != binding.ty
                || !matches!(&binding.ty, Type::Struct { name, .. } if name == "MerkleTreeDigest")
                || pure.parameters.len() != 1
                || arguments.len() != 1
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    &binding.value,
                    "return_value.value.bindings[0].value".to_owned(),
                )));
            }
            let path_ty = &pure.parameters[0].ty;
            let path = match &arguments[0] {
                Expr::Coerce { value, ty } if ty == path_ty => value.as_ref(),
                other => other,
            };
            let Expr::WitnessCall {
                name: witness_name,
                arguments: witness_args,
            } = path
            else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    path,
                    "return_value.value.bindings[0].value.arguments[0]".to_owned(),
                )));
            };
            let witness = witnesses
                .get(witness_name.as_str())
                .ok_or_else(|| RenderError::UnknownWitness(witness_name.clone()))?;
            if witness.result != *path_ty
                || !witness.parameters.is_empty()
                || !witness_args.is_empty()
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    path,
                    "return_value.value.bindings[0].value.arguments[0]".to_owned(),
                )));
            }
            let slot = ident(field)?;
            let witness_method = ident(witness_name)?;
            let pure_method = ident(name)?;
            let root_ty = rust_type(&binding.ty)?;
            (
                vec![
                    syn::parse_quote! {
                        let (frame, path) = frame.try_witness_metered(|context, meter| {
                            witnesses.#witness_method(
                                context.witness_context_with(super::LedgerView {
                                    state: context.query.state.get_ref(),
                                    meter,
                                }),
                            )
                        })?;
                    },
                    syn::parse_quote! {
                        let root: #root_ty = crate::pure_circuits::#pure_method(path)?;
                    },
                    syn::parse_quote! {
                        let (frame, observed): (_, bool) =
                            crate::ledger_slots::#slot.record_check_root(frame, root)?;
                    },
                ],
                syn::parse_quote!(observed),
            )
        }
        _ => {
            return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                &circuit.return_value,
            )));
        }
    };
    if steps.is_empty() && return_steps.is_empty() {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::no_effect()));
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
    Ok(RecordingOutcome::Supported(item))
}

/// A recording handle that borrows the user-supplied witness implementation.
pub(crate) fn render_borrowed_recorded_contract_method(
    circuit: &StatefulCircuit,
    uses_witness: bool,
) -> Result<syn::ImplItemFn, RenderError> {
    let name = ident(&circuit.name)?;
    let mut args = Vec::<syn::FnArg>::new();
    let mut call_args = Vec::<syn::Ident>::new();
    for (parameter, arg) in circuit
        .parameters
        .iter()
        .zip(public_parameter_idents(&circuit.parameters))
    {
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
    for (parameter, arg) in circuit
        .parameters
        .iter()
        .zip(public_parameter_idents(&circuit.parameters))
    {
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
    for (parameter, arg) in circuit
        .parameters
        .iter()
        .zip(public_parameter_idents(&circuit.parameters))
    {
        let ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#arg: #ty));
        input_args.push(retained_value(syn::parse_quote!(#arg), &parameter.ty));
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
