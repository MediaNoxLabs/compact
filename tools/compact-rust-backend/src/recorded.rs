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
    ComparisonOperator, CounterAmount, Expr, LedgerField, LedgerFieldKind, LocalBinding,
    PureCircuit, StateAction, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
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

fn set_size_operand(value: &Expr) -> Option<(&str, u8)> {
    match value {
        Expr::SetSize { field, index } => Some((field, *index)),
        Expr::Coerce { value, ty }
            if *ty
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            set_size_operand(value)
        }
        _ => None,
    }
}

fn list_length_operand(value: &Expr) -> Option<(&str, u8)> {
    match value {
        Expr::ListLength { field, index } => Some((field, *index)),
        Expr::Coerce { value, ty }
            if *ty
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            list_length_operand(value)
        }
        _ => None,
    }
}

fn list_head_field<'a>(
    value: &'a Expr,
    expected_field: &str,
    expected_index: usize,
) -> Option<(&'a str, u8, &'a Type)> {
    let Expr::StructField {
        value,
        field,
        index,
    } = value
    else {
        return None;
    };
    if field != expected_field || *index != expected_index {
        return None;
    }
    let Expr::ListHead { field, index, ty } = value.as_ref() else {
        return None;
    };
    Some((field, *index, ty))
}

fn uint64_literal(value: &Expr) -> Option<u64> {
    match value {
        Expr::UnsignedLiteral { value, max } => {
            let value = value.parse::<u64>().ok()?;
            (value as u128 <= max.parse::<u128>().ok()?).then_some(value)
        }
        Expr::Coerce { value, ty }
            if *ty
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            uint64_literal(value)
        }
        _ => None,
    }
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

/// Opaque string recording is admitted for the proved assertion → insertion
/// → Unit witness sequence. Other opaque APIs need their own TS/ledger proof
/// parity before the capability report can advertise them.
fn proved_opaque_set_sequence(circuit: &StatefulCircuit) -> bool {
    let [parameter] = circuit.parameters.as_slice() else {
        return false;
    };
    if parameter.ty != Type::OpaqueString
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
    {
        return false;
    }
    let [
        StateAction::Assert {
            condition: Expr::SetMember { value: member, .. },
            ..
        },
        StateAction::SetInsert {
            value: inserted, ..
        },
        StateAction::Expression {
            value: Expr::WitnessCall { arguments, .. },
        },
    ] = circuit.actions.as_slice()
    else {
        return false;
    };
    let [
        Expr::Coerce {
            value: witnessed,
            ty: Type::OpaqueString,
        },
    ] = arguments.as_slice()
    else {
        return false;
    };
    [member.as_ref(), inserted, witnessed.as_ref()]
        .iter()
        .all(|value| matches!(value, Expr::Parameter { name } if name == &parameter.name))
}

/// The one-operation OpaqueString Set shapes proved by the set oracle. Keep
/// the field, physical index, and parameter source tied to the typed IR so
/// general opaque expressions remain outside the recording surface.
fn closed_opaque_set_operation(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> bool {
    let [parameter] = circuit.parameters.as_slice() else {
        return false;
    };
    if parameter.ty != Type::OpaqueString {
        return false;
    }
    let operation = match (&circuit.actions[..], &circuit.return_value, &circuit.result) {
        (
            [
                StateAction::SetInsert {
                    field,
                    index,
                    value,
                },
            ],
            StateReturn::Unit,
            Type::Unit,
        ) => Some((field, index, value)),
        (
            [],
            StateReturn::SetMember {
                field,
                index,
                value,
            },
            Type::Boolean,
        ) => Some((field, index, value)),
        _ => None,
    };
    let Some((field, index, value)) = operation else {
        return false;
    };
    let Some(declaration) = ledger_fields.get(field.as_str()) else {
        return false;
    };
    declaration.index == *index
        && matches!(&declaration.declaration, LedgerFieldKind::Set { ty } if *ty == Type::OpaqueString)
        && matches!(value, Expr::Parameter { name } if name == &parameter.name)
}

/// The original opaque-key Map oracle has two closed shapes. The Map key is
/// only encoded as an input; the lookup result is the fixed-width Field value.
enum ClosedOpaqueMapOperation {
    Put,
    Ensure { field: String, key: String },
}

fn closed_opaque_map_operation(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Option<ClosedOpaqueMapOperation> {
    let map_matches = |field: &str, index| {
        let declaration = ledger_fields.get(field)?;
        (declaration.index == index
            && matches!(
                &declaration.declaration,
                LedgerFieldKind::Map {
                    key: Type::OpaqueString,
                    value: Type::Field,
                }
            ))
        .then_some(())
    };
    match (
        circuit.parameters.as_slice(),
        &circuit.result,
        &circuit.return_value,
        circuit.actions.as_slice(),
    ) {
        (
            [key, value],
            Type::Unit,
            StateReturn::Unit,
            [
                StateAction::MapInsert {
                    field,
                    index,
                    key: Expr::Parameter { name: key_name },
                    value: Expr::Parameter { name: value_name },
                },
            ],
        ) if key.ty == Type::OpaqueString
            && value.ty == Type::Field
            && key_name == &key.name
            && value_name == &value.name =>
        {
            map_matches(field, *index)?;
            Some(ClosedOpaqueMapOperation::Put)
        }
        (
            [key],
            Type::Field,
            StateReturn::Expression {
                value: Expr::Let { bindings, body },
            },
            [
                StateAction::Assert {
                    condition:
                        Expr::MapMember {
                            field: member_field,
                            index: member_index,
                            key: member_key,
                        },
                    ..
                },
            ],
        ) if key.ty == Type::OpaqueString => {
            let [binding] = bindings.as_slice() else {
                return None;
            };
            let Expr::MapLookup {
                field,
                index,
                key: lookup_key,
            } = &binding.value
            else {
                return None;
            };
            if binding.ty != Type::Field
                || field != member_field
                || index != member_index
                || !matches!(member_key.as_ref(), Expr::Parameter { name } if name == &key.name)
                || !matches!(lookup_key.as_ref(), Expr::Parameter { name } if name == &key.name)
                || !matches!(body.as_ref(), Expr::Parameter { name } if name == &binding.name)
            {
                return None;
            }
            map_matches(field, *index)?;
            Some(ClosedOpaqueMapOperation::Ensure {
                field: field.clone(),
                key: key.name.clone(),
            })
        }
        _ => None,
    }
}

/// The asset removal path keeps a disclosed OpaqueString key scoped across
/// ordered guards and mutations. Only this exact shape has an oracle and a
/// ledger proof; unrelated OpaqueString Lets remain unavailable.
fn closed_opaque_asset_removal(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> bool {
    let [parameter] = circuit.parameters.as_slice() else {
        return false;
    };
    if parameter.ty != Type::OpaqueString
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
    {
        return false;
    }
    let [StateAction::Let { bindings, action }] = circuit.actions.as_slice() else {
        return false;
    };
    let [binding] = bindings.as_slice() else {
        return false;
    };
    if binding.ty != Type::OpaqueString
        || !matches!(&binding.value, Expr::Parameter { name } if name == &parameter.name)
    {
        return false;
    }
    let StateAction::Sequence { actions } = action.as_ref() else {
        return false;
    };
    let [
        StateAction::CircuitCall {
            name: writable,
            arguments: writable_args,
        },
        StateAction::Assert {
            condition:
                Expr::MapMember {
                    field: records_member,
                    index: records_member_index,
                    key: member_key,
                },
            ..
        },
        StateAction::Assert {
            condition:
                Expr::If {
                    condition: watched,
                    then: unguarded,
                    otherwise: guarded,
                },
            ..
        },
        StateAction::MapRemove {
            field: records_remove,
            index: records_remove_index,
            key: removed_key,
        },
        StateAction::SetInsert {
            field: retired,
            index: retired_index,
            value: retired_key,
        },
        StateAction::CircuitCall {
            name: write,
            arguments: write_args,
        },
    ] = actions.as_slice()
    else {
        return false;
    };
    let Expr::SetMember {
        field: watch_field,
        index: watch_index,
        value: watched_key,
    } = watched.as_ref()
    else {
        return false;
    };
    let key_is_bound =
        |value: &Expr| matches!(value, Expr::Parameter { name } if name == &binding.name);
    if writable != "assertWritable"
        || !writable_args.is_empty()
        || write != "recordWrite"
        || !write_args.is_empty()
        || records_member != "records"
        || records_remove != records_member
        || records_member_index != records_remove_index
        || watch_field != "watchList"
        || retired != "retiredKeys"
        || !matches!(unguarded.as_ref(), Expr::Boolean { value: false })
        || !matches!(guarded.as_ref(), Expr::Boolean { value: true })
        || ![
            member_key.as_ref(),
            watched_key.as_ref(),
            removed_key,
            retired_key,
        ]
        .into_iter()
        .all(key_is_bound)
    {
        return false;
    }
    matches!(ledger_fields.get(records_member.as_str()), Some(field)
        if field.index == *records_member_index
            && matches!(&field.declaration, LedgerFieldKind::Map { key: Type::OpaqueString, .. }))
        && matches!(ledger_fields.get(watch_field.as_str()), Some(field)
            if field.index == *watch_index
                && matches!(&field.declaration, LedgerFieldKind::Set { ty: Type::OpaqueString }))
        && matches!(ledger_fields.get(retired.as_str()), Some(field)
            if field.index == *retired_index
                && matches!(&field.declaration, LedgerFieldKind::Set { ty: Type::OpaqueString }))
}

/// A closed typed witness → Bytes32 hash → Set authorization followed by a
/// single Set insertion. The matched callees are inspected transitively so
/// recording never silently skips an assertion or private output.
fn closed_organizer_gate_steps(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    let bytes32 = Type::Bytes { length: 32 };
    let [parameter] = circuit.parameters.as_slice() else {
        return Ok(None);
    };
    if !matches!(
        parameter.ty,
        Type::OpaqueString | Type::Bytes { length: 32 }
    ) || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
    {
        return Ok(None);
    }
    let [
        StateAction::Assert {
            condition: Expr::Let { bindings, body },
            message: organizer_message,
        },
        StateAction::SetInsert {
            field: insert_field,
            index: insert_index,
            value: inserted,
        },
    ] = circuit.actions.as_slice()
    else {
        return Ok(None);
    };
    let [binding] = bindings.as_slice() else {
        return Ok(None);
    };
    if binding.ty != bytes32
        || !matches!(inserted, Expr::Parameter { name } if name == &parameter.name)
    {
        return Ok(None);
    }
    let Expr::SetMember {
        field: member_field,
        index: member_index,
        value: member_value,
    } = body.as_ref()
    else {
        return Ok(None);
    };
    if !matches!(member_value.as_ref(), Expr::Parameter { name } if name == &binding.name) {
        return Ok(None);
    }
    let Some(member_slot) = ledger_fields.get(member_field.as_str()) else {
        return Ok(None);
    };
    let Some(insert_slot) = ledger_fields.get(insert_field.as_str()) else {
        return Ok(None);
    };
    if member_slot.index != *member_index
        || member_slot.declaration
            != (LedgerFieldKind::Set {
                ty: bytes32.clone(),
            })
        || insert_slot.index != *insert_index
        || insert_slot.declaration
            != (LedgerFieldKind::Set {
                ty: parameter.ty.clone(),
            })
    {
        return Ok(None);
    }
    let Expr::Call {
        name: hash_name,
        arguments: hash_arguments,
    } = &binding.value
    else {
        return Ok(None);
    };
    let [
        Expr::Coerce {
            value: helper_call,
            ty: hash_argument_ty,
        },
    ] = hash_arguments.as_slice()
    else {
        return Ok(None);
    };
    if *hash_argument_ty != bytes32 {
        return Ok(None);
    }
    let Expr::Call {
        name: helper_name,
        arguments: helper_arguments,
    } = helper_call.as_ref()
    else {
        return Ok(None);
    };
    if !helper_arguments.is_empty() {
        return Ok(None);
    }
    let Some(hash) = pure_circuits.get(hash_name.as_str()) else {
        return Ok(None);
    };
    let [hash_parameter] = hash.parameters.as_slice() else {
        return Ok(None);
    };
    if hash_parameter.ty != bytes32 || hash.result != bytes32 {
        return Ok(None);
    }
    let Expr::PersistentHash { value: hash_value } = &hash.body else {
        return Ok(None);
    };
    let Expr::Tuple {
        elements: hash_elements,
    } = hash_value.as_ref()
    else {
        return Ok(None);
    };
    let [
        Expr::BytesLiteral { bytes: prefix },
        Expr::Parameter { name: hashed_name },
    ] = hash_elements.as_slice()
    else {
        return Ok(None);
    };
    if prefix.len() != 32 || hashed_name != &hash_parameter.name {
        return Ok(None);
    }

    let Some(helper) = circuits.get(helper_name.as_str()) else {
        return Ok(None);
    };
    if !helper.internal
        || !helper.parameters.is_empty()
        || !helper.actions.is_empty()
        || helper.result != bytes32
    {
        return Ok(None);
    }
    let StateReturn::Expression {
        value:
            Expr::Let {
                bindings: helper_bindings,
                body: helper_body,
            },
    } = &helper.return_value
    else {
        return Ok(None);
    };
    let [maybe_binding] = helper_bindings.as_slice() else {
        return Ok(None);
    };
    let Type::Struct {
        fields: maybe_fields,
        ..
    } = &maybe_binding.ty
    else {
        return Ok(None);
    };
    let [present_field, value_field] = maybe_fields.as_slice() else {
        return Ok(None);
    };
    if present_field.name != "is_some"
        || present_field.ty != Type::Boolean
        || value_field.name != "value"
        || value_field.ty != bytes32
    {
        return Ok(None);
    }
    let Expr::WitnessCall {
        name: witness_name,
        arguments: witness_arguments,
    } = &maybe_binding.value
    else {
        return Ok(None);
    };
    let Some(witness) = witnesses.get(witness_name.as_str()) else {
        return Ok(None);
    };
    if !witness_arguments.is_empty()
        || !witness.parameters.is_empty()
        || witness.result != maybe_binding.ty
    {
        return Ok(None);
    }
    let Expr::Sequence {
        steps: helper_steps,
        value: helper_result,
    } = helper_body.as_ref()
    else {
        return Ok(None);
    };
    let [
        Expr::Assert {
            condition: present_condition,
            message: key_message,
        },
    ] = helper_steps.as_slice()
    else {
        return Ok(None);
    };
    let Expr::StructField {
        value: present_source,
        field: present_name,
        index: 0,
    } = present_condition.as_ref()
    else {
        return Ok(None);
    };
    let Expr::StructField {
        value: key_source,
        field: value_name,
        index: 1,
    } = helper_result.as_ref()
    else {
        return Ok(None);
    };
    if present_name != "is_some"
        || value_name != "value"
        || !matches!(present_source.as_ref(), Expr::Parameter { name } if name == &maybe_binding.name)
        || !matches!(key_source.as_ref(), Expr::Parameter { name } if name == &maybe_binding.name)
    {
        return Ok(None);
    }

    let witness_method = ident(witness_name)?;
    let hash_method = ident(hash_name)?;
    let member_slot = ident(member_field)?;
    let insert_slot = ident(insert_field)?;
    let maybe_ty = rust_type(&maybe_binding.ty)?;
    let inserted = retained_value(syn::parse_quote!(__compact_param_0), &parameter.ty);
    Ok(Some(vec![
        syn::parse_quote! {
            let (frame, __compact_recorded_maybe_sk): (_, #maybe_ty) = frame.try_witness_metered(|context, meter| {
                witnesses.#witness_method(context.witness_context_with(super::LedgerView {
                    state: context.query.state.get_ref(), meter,
                }))
            })?;
        },
        syn::parse_quote! {
            if !__compact_recorded_maybe_sk.is_some {
                return Err(runtime::CompactError::AssertionFailed(#key_message.to_owned()));
            }
        },
        syn::parse_quote! {
            let __compact_recorded_organizer_pk: runtime::FixedBytes<32> =
                crate::pure_circuits::#hash_method(__compact_recorded_maybe_sk.value)?;
        },
        syn::parse_quote! {
            let (frame, __compact_recorded_is_organizer): (_, bool) =
                crate::ledger_slots::#member_slot.record_member(frame, __compact_recorded_organizer_pk)?;
        },
        syn::parse_quote! {
            if !__compact_recorded_is_organizer {
                return Err(runtime::CompactError::AssertionFailed(#organizer_message.to_owned()));
            }
        },
        syn::parse_quote! {
            let frame = crate::ledger_slots::#insert_slot.record_insert(frame, #inserted)?;
        },
    ]))
}

/// A pure Field call may be evaluated while recording only when its whole
/// transitive body is scalar arithmetic. Hashes and other primitives need
/// their own VM/gas parity decision before they can join this path.
fn closed_pure_field_call(
    name: &str,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    visiting: &mut HashSet<String>,
) -> bool {
    fn scalar_body(
        value: &Expr,
        parameters: &HashSet<&str>,
        pure_circuits: &HashMap<&str, &PureCircuit>,
        visiting: &mut HashSet<String>,
    ) -> bool {
        match value {
            Expr::FieldLiteral { .. } => true,
            Expr::Parameter { name } => parameters.contains(name.as_str()),
            Expr::Coerce { value, ty } if *ty == Type::Field => {
                scalar_body(value, parameters, pure_circuits, visiting)
            }
            Expr::Add { left, right }
            | Expr::Subtract { left, right }
            | Expr::Multiply { left, right } => {
                scalar_body(left, parameters, pure_circuits, visiting)
                    && scalar_body(right, parameters, pure_circuits, visiting)
            }
            Expr::Call { name, arguments } => {
                let Some(callee) = pure_circuits.get(name.as_str()) else {
                    return false;
                };
                arguments.len() == callee.parameters.len()
                    && arguments
                        .iter()
                        .zip(&callee.parameters)
                        .all(|(argument, parameter)| {
                            parameter.ty == Type::Field
                                && scalar_body(argument, parameters, pure_circuits, visiting)
                        })
                    && closed_pure_field_call(name, pure_circuits, visiting)
            }
            _ => false,
        }
    }

    let Some(callee) = pure_circuits.get(name) else {
        return false;
    };
    if callee.result != Type::Field
        || callee
            .parameters
            .iter()
            .any(|parameter| parameter.ty != Type::Field)
        || !visiting.insert(name.to_owned())
    {
        return false;
    }
    let parameters = callee
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect();
    let allowed = scalar_body(&callee.body, &parameters, pure_circuits, visiting);
    visiting.remove(name);
    allowed
}

// Calling the generated pure Rust function retains its exact assertion
// message and propagates failure before any recorded ledger operation. Keep
// this admission bounded to one typed argument, one assertion, and a direct
// Unit or Boolean return; other pure bodies need their own effect audit.
fn closed_pure_assert_call(callee: &PureCircuit) -> bool {
    let [parameter] = callee.parameters.as_slice() else {
        return false;
    };
    let Expr::Sequence { steps, value } = &callee.body else {
        return false;
    };
    let [Expr::Assert { condition, .. }] = steps.as_slice() else {
        return false;
    };
    match (
        &parameter.ty,
        &callee.result,
        condition.as_ref(),
        value.as_ref(),
    ) {
        (
            Type::Boolean,
            Type::Boolean,
            Expr::Parameter { name: asserted },
            Expr::Parameter { name: returned },
        ) => asserted == &parameter.name && returned == &parameter.name,
        (Type::Field, Type::Unit, Expr::NotEqual { left, right }, Expr::Unit) => {
            matches!(left.as_ref(), Expr::Parameter { name } if name == &parameter.name)
                && matches!(right.as_ref(), Expr::FieldLiteral { value } if value == "0")
        }
        _ => false,
    }
}

/// Admit a pure unsigned helper only when its complete body consists of
/// bounded unsigned arithmetic and casts. Other pure primitives may carry
/// VM effects or different gas and need an explicit recording decision.
fn closed_pure_unsigned_call(
    name: &str,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    visiting: &mut HashSet<String>,
) -> bool {
    fn unsigned_body(
        value: &Expr,
        parameters: &HashSet<&str>,
        pure_circuits: &HashMap<&str, &PureCircuit>,
        visiting: &mut HashSet<String>,
    ) -> bool {
        match value {
            Expr::UnsignedLiteral { .. } => true,
            Expr::Parameter { name } => parameters.contains(name.as_str()),
            Expr::Coerce {
                value,
                ty: Type::Unsigned { .. },
            } => unsigned_body(value, parameters, pure_circuits, visiting),
            Expr::UnsignedCast { value, .. } => {
                unsigned_body(value, parameters, pure_circuits, visiting)
            }
            Expr::UnsignedAdd { left, right, .. }
            | Expr::UnsignedSubtract { left, right, .. }
            | Expr::UnsignedMultiply { left, right, .. } => {
                unsigned_body(left, parameters, pure_circuits, visiting)
                    && unsigned_body(right, parameters, pure_circuits, visiting)
            }
            Expr::Call { name, arguments } => {
                let Some(callee) = pure_circuits.get(name.as_str()) else {
                    return false;
                };
                arguments.len() == callee.parameters.len()
                    && arguments
                        .iter()
                        .zip(&callee.parameters)
                        .all(|(argument, parameter)| {
                            matches!(parameter.ty, Type::Unsigned { .. })
                                && unsigned_body(argument, parameters, pure_circuits, visiting)
                        })
                    && closed_pure_unsigned_call(name, pure_circuits, visiting)
            }
            _ => false,
        }
    }

    let Some(callee) = pure_circuits.get(name) else {
        return false;
    };
    if !matches!(callee.result, Type::Unsigned { .. })
        || callee
            .parameters
            .iter()
            .any(|parameter| !matches!(parameter.ty, Type::Unsigned { .. }))
        || !visiting.insert(name.to_owned())
    {
        return false;
    }
    let parameters = callee
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect();
    let allowed = unsigned_body(&callee.body, &parameters, pure_circuits, visiting);
    visiting.remove(name);
    allowed
}

fn field_pair_type(ty: &Type) -> bool {
    matches!(ty, Type::Vector { element, length } if **element == Type::Field && *length == 2)
        || matches!(ty, Type::Tuple { elements } if elements == &[Type::Field, Type::Field])
}

/// A pure Field helper may be evaluated during recording when its entire
/// transitive body only constructs and hashes a pair of Fields. Its declared
/// input is either empty or one typed pair; the caller must separately lower
/// that input through the typed Cell source. Tuple/vector coercions are checked
/// at their declared types instead of assuming the representations match.
fn closed_pure_field_pair_hash_call(
    name: &str,
    pure_circuits: &HashMap<&str, &PureCircuit>,
) -> bool {
    fn body_type(
        value: &Expr,
        locals: &HashMap<String, Type>,
        pure_circuits: &HashMap<&str, &PureCircuit>,
        visiting: &mut HashSet<String>,
        saw_hash: &mut bool,
    ) -> Option<Type> {
        match value {
            Expr::FieldLiteral { .. } => Some(Type::Field),
            Expr::Parameter { name } => locals.get(name).cloned(),
            Expr::Vector { element, elements }
                if *element == Type::Field && elements.len() == 2 =>
            {
                elements
                    .iter()
                    .all(|element| {
                        body_type(element, locals, pure_circuits, visiting, saw_hash)
                            == Some(Type::Field)
                    })
                    .then(|| Type::Vector {
                        element: Box::new(Type::Field),
                        length: 2,
                    })
            }
            Expr::Tuple { elements } if elements.len() == 2 => elements
                .iter()
                .all(|element| {
                    body_type(element, locals, pure_circuits, visiting, saw_hash)
                        == Some(Type::Field)
                })
                .then(|| Type::Tuple {
                    elements: vec![Type::Field, Type::Field],
                }),
            Expr::Coerce { value, ty } => {
                let source = body_type(value, locals, pure_circuits, visiting, saw_hash)?;
                (source == *ty || (field_pair_type(&source) && field_pair_type(ty)))
                    .then(|| ty.clone())
            }
            Expr::Let { bindings, body } => {
                let mut scoped = locals.clone();
                for binding in bindings {
                    let actual =
                        body_type(&binding.value, &scoped, pure_circuits, visiting, saw_hash)?;
                    if actual != binding.ty {
                        return None;
                    }
                    scoped.insert(binding.name.clone(), actual);
                }
                body_type(body, &scoped, pure_circuits, visiting, saw_hash)
            }
            Expr::Call { name, arguments } => {
                let callee = pure_circuits.get(name.as_str())?;
                if arguments.len() != callee.parameters.len() || !visiting.insert(name.clone()) {
                    return None;
                }
                let valid_arguments =
                    arguments
                        .iter()
                        .zip(&callee.parameters)
                        .all(|(arg, formal)| {
                            body_type(arg, locals, pure_circuits, visiting, saw_hash)
                                == Some(formal.ty.clone())
                        });
                let mut callee_locals = HashMap::new();
                for parameter in &callee.parameters {
                    callee_locals.insert(parameter.name.clone(), parameter.ty.clone());
                }
                let result = if valid_arguments {
                    body_type(
                        &callee.body,
                        &callee_locals,
                        pure_circuits,
                        visiting,
                        saw_hash,
                    )
                    .filter(|actual| *actual == callee.result)
                } else {
                    None
                };
                visiting.remove(name);
                result
            }
            Expr::TransientHash { value } => {
                let ty = body_type(value, locals, pure_circuits, visiting, saw_hash)?;
                if !field_pair_type(&ty) {
                    return None;
                }
                *saw_hash = true;
                Some(Type::Field)
            }
            _ => None,
        }
    }

    let Some(callee) = pure_circuits.get(name) else {
        return false;
    };
    if callee.result != Type::Field
        || !(callee.parameters.is_empty()
            || (callee.parameters.len() == 1 && field_pair_type(&callee.parameters[0].ty)))
    {
        return false;
    }
    let locals = callee
        .parameters
        .iter()
        .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
        .collect();
    let mut saw_hash = false;
    let mut visiting = HashSet::from([name.to_owned()]);
    body_type(
        &callee.body,
        &locals,
        pure_circuits,
        &mut visiting,
        &mut saw_hash,
    ) == Some(Type::Field)
        && saw_hash
}

/// A closed literal Vector helper has no ledger, witness, or VM effects. Keep
/// this narrower than general pure calls until their recording parity is known.
fn closed_literal_field_vector_call(
    name: &str,
    ty: &Type,
    pure_circuits: &HashMap<&str, &PureCircuit>,
) -> bool {
    let Type::Vector { element, length } = ty else {
        return false;
    };
    if **element != Type::Field {
        return false;
    }
    let Some(callee) = pure_circuits.get(name) else {
        return false;
    };
    if !callee.parameters.is_empty() || callee.result != *ty {
        return false;
    }
    let Expr::Vector {
        element: body_element,
        elements,
    } = &callee.body
    else {
        return false;
    };
    *body_element == Type::Field
        && elements.len() == *length
        && elements
            .iter()
            .all(|element| matches!(element, Expr::FieldLiteral { .. }))
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

    // The asset and attestation freshness calls share a pure Unit guard
    // before one closed stateful continuation. Keep the three typed source
    // arguments and the pure assertion shape closed; the emitted pure Rust
    // function owns its nested struct projection and bounded subtraction.
    fn closed_guarded_struct_pure_steps(
        circuit: &StatefulCircuit,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        pure_circuits: &HashMap<&str, &PureCircuit>,
        circuits: &HashMap<&str, &StatefulCircuit>,
    ) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
        let [policy, record, current_time] = circuit.parameters.as_slice() else {
            return Ok(None);
        };
        if !matches!(policy.ty, Type::Struct { .. })
            || !matches!(record.ty, Type::Struct { .. })
            || !matches!(&current_time.ty, Type::Unsigned { max } if max == "18446744073709551615")
            || circuit.result != Type::Unit
            || circuit.return_value != StateReturn::Unit
        {
            return Ok(None);
        }
        let [StateAction::PureCall { name, arguments }, continuation] = circuit.actions.as_slice()
        else {
            return Ok(None);
        };
        let Some(pure) = pure_circuits.get(name.as_str()) else {
            return Ok(None);
        };
        let closed_continuation = match continuation {
            StateAction::CircuitCall {
                name: helper_name,
                arguments: helper_arguments,
            } => circuits.get(helper_name.as_str()).is_some_and(|helper| {
                helper.result == Type::Unit
                    && helper.return_value == StateReturn::Unit
                    && helper.parameters.is_empty()
                    && helper_arguments.is_empty()
            }),
            action => closed_counter_one_continuation(action),
        };
        if pure.result != Type::Unit
            || pure.parameters.len() != 3
            || !closed_continuation
            || arguments.len() != 3
        {
            return Ok(None);
        }
        let Expr::Sequence {
            steps: pure_steps,
            value,
        } = &pure.body
        else {
            return Ok(None);
        };
        let [
            Expr::Assert { .. },
            Expr::If {
                then, otherwise, ..
            },
        ] = pure_steps.as_slice()
        else {
            return Ok(None);
        };
        if !matches!(value.as_ref(), Expr::Unit)
            || !matches!(then.as_ref(), Expr::Assert { .. })
            || !matches!(otherwise.as_ref(), Expr::Unit)
        {
            return Ok(None);
        }
        let mut steps = Vec::new();
        let mut typed_arguments = Vec::new();
        for (index, ((argument, source), target)) in arguments
            .iter()
            .zip(&circuit.parameters)
            .zip(&pure.parameters)
            .enumerate()
        {
            if source.ty != target.ty
                || !matches!(argument, Expr::Coerce { value, ty }
                    if ty == &source.ty && matches!(value.as_ref(), Expr::Parameter { name } if name == &source.name))
            {
                return Ok(None);
            }
            let (_, rust_name) = parameters
                .get(source.name.as_str())
                .expect("closed guard argument is a declared parameter");
            let arg_ty = rust_type(&source.ty)?;
            let arg = syn::Ident::new(
                &format!("__compact_recorded_guard_arg_{index}"),
                Span::call_site(),
            );
            if index == 2 {
                steps.push(syn::parse_quote!(let #arg: #arg_ty = #rust_name;));
            } else {
                steps.push(syn::parse_quote!(let #arg: #arg_ty = (#rust_name).clone();));
            }
            typed_arguments.push(arg);
        }
        let method = ident(name)?;
        steps.push(syn::parse_quote!(
            crate::pure_circuits::#method(#(#typed_arguments),*)?;
        ));
        Ok(Some(steps))
    }

    // A custody grant read has a disclosed opaque key, one Map membership
    // guard, one typed composite lookup, and a closed pure time assertion.
    // Keep this exact asset shape separate from general OpaqueString Lets and
    // general struct Map lookups until their own proof parity is established.
    fn closed_custody_grant_steps(
        circuit: &StatefulCircuit,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ledger_fields: &HashMap<&str, &LedgerField>,
        pure_circuits: &HashMap<&str, &PureCircuit>,
    ) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
        let [key_parameter, time_parameter] = circuit.parameters.as_slice() else {
            return Ok(None);
        };
        let uint64 = Type::Unsigned {
            max: u64::MAX.to_string(),
        };
        if circuit.name != "assertGrantEffective"
            || key_parameter.ty != Type::OpaqueString
            || time_parameter.ty != uint64
            || circuit.result != Type::Unit
            || circuit.return_value != StateReturn::Unit
        {
            return Ok(None);
        }
        let [
            StateAction::Let {
                bindings: key_bindings,
                action,
            },
        ] = circuit.actions.as_slice()
        else {
            return Ok(None);
        };
        let [key_binding] = key_bindings.as_slice() else {
            return Ok(None);
        };
        if key_binding.ty != Type::OpaqueString
            || !matches!(&key_binding.value, Expr::Parameter { name } if name == &key_parameter.name)
        {
            return Ok(None);
        }
        let StateAction::Sequence { actions } = action.as_ref() else {
            return Ok(None);
        };
        let [
            StateAction::Assert {
                condition:
                    Expr::MapMember {
                        field: member_field,
                        index: member_index,
                        key: member_key,
                    },
                message,
            },
            StateAction::Let {
                bindings: grant_bindings,
                action: grant_action,
            },
        ] = actions.as_slice()
        else {
            return Ok(None);
        };
        let [grant_binding] = grant_bindings.as_slice() else {
            return Ok(None);
        };
        let Expr::MapLookup {
            field: lookup_field,
            index: lookup_index,
            key: lookup_key,
        } = &grant_binding.value
        else {
            return Ok(None);
        };
        let StateAction::PureCall {
            name: pure_name,
            arguments,
        } = grant_action.as_ref()
        else {
            return Ok(None);
        };
        let [grant_argument, time_argument] = arguments.as_slice() else {
            return Ok(None);
        };
        if member_field != "custodyGrants"
            || lookup_field != member_field
            || member_index != lookup_index
            || !matches!(member_key.as_ref(), Expr::Parameter { name } if name == &key_binding.name)
            || !matches!(lookup_key.as_ref(), Expr::Parameter { name } if name == &key_binding.name)
            || pure_name != "assertGrantNotFuture"
            || !matches!(grant_argument, Expr::Coerce { value, ty }
                if ty == &grant_binding.ty && matches!(value.as_ref(), Expr::Parameter { name } if name == &grant_binding.name))
            || !matches!(time_argument, Expr::Coerce { value, ty }
                if ty == &uint64 && matches!(value.as_ref(), Expr::Parameter { name } if name == &time_parameter.name))
        {
            return Ok(None);
        }
        let Type::Struct {
            name: grant_name,
            fields: grant_fields,
        } = &grant_binding.ty
        else {
            return Ok(None);
        };
        let [code, holder, granted_at] = grant_fields.as_slice() else {
            return Ok(None);
        };
        let Type::Struct {
            name: holder_name,
            fields: holder_fields,
        } = &holder.ty
        else {
            return Ok(None);
        };
        if grant_name != "CustodyGrant"
            || code.name != "code"
            || code.ty != (Type::Bytes { length: 32 })
            || holder.name != "holder"
            || holder_name != "ContractAddress"
            || !matches!(holder_fields.as_slice(), [field]
                if field.name == "bytes" && field.ty == (Type::Bytes { length: 32 }))
            || granted_at.name != "grantedAt"
            || granted_at.ty != uint64
        {
            return Ok(None);
        }
        let Some(slot) = ledger_fields.get(member_field.as_str()) else {
            return Ok(None);
        };
        if slot.index != *member_index
            || slot.physical_path() != [1, 11]
            || slot.declaration
                != (LedgerFieldKind::Map {
                    key: Type::OpaqueString,
                    value: grant_binding.ty.clone(),
                })
        {
            return Ok(None);
        }
        let Some(pure) = pure_circuits.get(pure_name.as_str()) else {
            return Ok(None);
        };
        let [pure_grant, pure_time] = pure.parameters.as_slice() else {
            return Ok(None);
        };
        let Expr::Sequence {
            steps: pure_steps,
            value: pure_value,
        } = &pure.body
        else {
            return Ok(None);
        };
        let [
            Expr::Assert {
                condition: pure_condition,
                ..
            },
        ] = pure_steps.as_slice()
        else {
            return Ok(None);
        };
        let Expr::Let {
            bindings: pure_bindings,
            body,
        } = pure_condition.as_ref()
        else {
            return Ok(None);
        };
        let [projected] = pure_bindings.as_slice() else {
            return Ok(None);
        };
        if pure.result != Type::Unit
            || pure_grant.ty != grant_binding.ty
            || pure_time.ty != uint64
            || !matches!(pure_value.as_ref(), Expr::Unit)
            || projected.ty != uint64
            || !matches!(&projected.value, Expr::StructField { value, field, index: 2 }
                if field == "grantedAt" && matches!(value.as_ref(), Expr::Parameter { name } if name == &pure_grant.name))
            || !matches!(body.as_ref(), Expr::Compare { operator: ComparisonOperator::LessEqual, left, right }
                if matches!(left.as_ref(), Expr::Parameter { name } if name == &projected.name)
                    && matches!(right.as_ref(), Expr::Parameter { name } if name == &pure_time.name))
        {
            return Ok(None);
        }
        let (_, key_source) = parameters
            .get(key_parameter.name.as_str())
            .expect("closed grant key is declared");
        let (_, time_source) = parameters
            .get(time_parameter.name.as_str())
            .expect("closed grant time is declared");
        let slot = ident(member_field)?;
        let method = ident(pure_name)?;
        let grant_ty = rust_type(&grant_binding.ty)?;
        let steps = vec![
            syn::parse_quote!(let __compact_recorded_grant_key: runtime::OpaqueString = (#key_source).clone();),
            syn::parse_quote!(
                let (frame, __compact_recorded_grant_member): (_, bool) =
                    crate::ledger_slots::#slot.record_member(frame, __compact_recorded_grant_key.clone())?;
            ),
            syn::parse_quote!(
                if !__compact_recorded_grant_member {
                    return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                }
            ),
            syn::parse_quote!(
                let (frame, __compact_recorded_grant): (_, #grant_ty) =
                    crate::ledger_slots::#slot.record_lookup(frame, __compact_recorded_grant_key)?;
            ),
            syn::parse_quote!(
                let __compact_recorded_as_of: runtime::BoundedUint<18446744073709551615> =
                    #time_source;
            ),
            syn::parse_quote!(
                crate::pure_circuits::#method(__compact_recorded_grant, __compact_recorded_as_of)?;
            ),
        ];
        Ok(Some(steps))
    }

    // A mixed-width product assertion lives in a pure Unit circuit because
    // the stateful source cannot inline ordering comparisons. Admit only its
    // exact two-Uint32, multiply-by-four, less-equal body before Counter-one.
    fn closed_guarded_unsigned_product_steps(
        circuit: &StatefulCircuit,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        pure_circuits: &HashMap<&str, &PureCircuit>,
    ) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
        const UINT32: &str = "4294967295";
        const PRODUCT: &str = "17179869180";
        let [left, right] = circuit.parameters.as_slice() else {
            return Ok(None);
        };
        if ![left, right]
            .iter()
            .all(|parameter| matches!(&parameter.ty, Type::Unsigned { max } if max == UINT32))
            || circuit.result != Type::Unit
            || circuit.return_value != StateReturn::Unit
        {
            return Ok(None);
        }
        let [StateAction::PureCall { name, arguments }, continuation] = circuit.actions.as_slice()
        else {
            return Ok(None);
        };
        if !closed_counter_one_continuation(continuation) {
            return Ok(None);
        }
        let Some(pure) = pure_circuits.get(name.as_str()) else {
            return Ok(None);
        };
        if pure.result != Type::Unit
            || pure.parameters.len() != 2
            || arguments.len() != 2
            || pure
                .parameters
                .iter()
                .any(|parameter| !matches!(&parameter.ty, Type::Unsigned { max } if max == UINT32))
        {
            return Ok(None);
        }
        let Expr::Sequence { steps, value } = &pure.body else {
            return Ok(None);
        };
        let [Expr::Assert { condition, .. }] = steps.as_slice() else {
            return Ok(None);
        };
        if !matches!(value.as_ref(), Expr::Unit) {
            return Ok(None);
        }
        let Expr::Let { bindings, body } = condition.as_ref() else {
            return Ok(None);
        };
        let [binding] = bindings.as_slice() else {
            return Ok(None);
        };
        if binding.ty
            != (Type::Unsigned {
                max: PRODUCT.into(),
            })
        {
            return Ok(None);
        }
        let Expr::UnsignedMultiply {
            max,
            left: product_left,
            right: product_right,
        } = &binding.value
        else {
            return Ok(None);
        };
        if max != PRODUCT
            || !matches!(product_left.as_ref(), Expr::UnsignedCast { max, value }
                if max == PRODUCT && matches!(value.as_ref(), Expr::Parameter { name } if name == &pure.parameters[0].name))
            || !matches!(product_right.as_ref(), Expr::UnsignedLiteral { value, max }
                if value == "4" && max == PRODUCT)
            || !matches!(body.as_ref(), Expr::Compare { operator: crate::ir::ComparisonOperator::LessEqual, left, right }
                if matches!(left.as_ref(), Expr::Parameter { name } if name == &binding.name)
                    && matches!(right.as_ref(), Expr::UnsignedCast { max, value }
                        if max == PRODUCT && matches!(value.as_ref(), Expr::Parameter { name } if name == &pure.parameters[1].name)))
        {
            return Ok(None);
        }
        let mut typed_arguments = Vec::new();
        let mut output = Vec::new();
        for (index, ((argument, source), target)) in arguments
            .iter()
            .zip(&circuit.parameters)
            .zip(&pure.parameters)
            .enumerate()
        {
            if source.ty != target.ty
                || !matches!(argument, Expr::Coerce { value, ty }
                    if ty == &source.ty && matches!(value.as_ref(), Expr::Parameter { name } if name == &source.name))
            {
                return Ok(None);
            }
            let (_, rust_name) = parameters
                .get(source.name.as_str())
                .expect("closed unsigned guard argument is a declared parameter");
            let typed = syn::Ident::new(
                &format!("__compact_recorded_product_arg_{index}"),
                Span::call_site(),
            );
            output.push(syn::parse_quote!(
                let #typed: runtime::BoundedUint<4294967295> = #rust_name;
            ));
            typed_arguments.push(typed);
        }
        let method = ident(name)?;
        output.push(syn::parse_quote!(
            crate::pure_circuits::#method(#(#typed_arguments),*)?;
        ));
        Ok(Some(output))
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

    // The Uint64 Cell write after a conditional Counter increment uses this
    // exact cast shape. Keep it separate from Counter amounts: a Cell retains
    // the Compact bounded value, while Counter increments consume a u16.
    fn conditional_uint64_source(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<syn::Expr> {
        const UINT64_MAX: &str = "18446744073709551615";

        fn literal_arm(value: &Expr, ceiling: u64) -> Option<syn::Expr> {
            match value {
                Expr::Coerce {
                    value,
                    ty: Type::Unsigned { max },
                } => {
                    let max = max.parse::<u64>().ok()?;
                    literal_arm(value, ceiling.min(max))
                }
                Expr::UnsignedLiteral { value, max } => {
                    let max = max.parse::<u64>().ok()?;
                    let value = value.parse::<u64>().ok()?;
                    (value <= ceiling.min(max)).then(|| {
                        let literal = syn::LitInt::new(&format!("{value}u128"), Span::call_site());
                        syn::parse_quote!(#literal)
                    })
                }
                _ => None,
            }
        }

        let Expr::UnsignedCast { max, value } = value else {
            return None;
        };
        if max != UINT64_MAX {
            return None;
        }
        let Expr::If {
            condition,
            then,
            otherwise,
        } = value.as_ref()
        else {
            return None;
        };
        let condition = cell_source(condition, &Type::Boolean, locals, parameters)?;
        let then = literal_arm(then, u64::MAX)?;
        let otherwise = literal_arm(otherwise, u64::MAX)?;
        Some(syn::parse_quote!(if #condition { #then } else { #otherwise }))
    }

    // This is the compiler's closed two-level Uint<4> conditional used by
    // walkerNestedIf and streamNestedIf. Each arm is a bounded literal and
    // each predicate is a Boolean parameter or an already recorded local.
    // In particular, no branch can perform a witness or ledger operation.
    fn closed_nested_uint4_source(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<syn::Expr> {
        fn lower(
            value: &Expr,
            ceiling: u64,
            depth: usize,
            locals: &HashMap<String, syn::Expr>,
            parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ) -> Option<(syn::Expr, bool)> {
            match value {
                Expr::Coerce {
                    value,
                    ty: Type::Unsigned { max },
                }
                | Expr::UnsignedCast { value, max } => {
                    let max = max.parse::<u64>().ok()?;
                    (max <= ceiling && max <= 4)
                        .then_some(())
                        .and_then(|()| lower(value, max, depth, locals, parameters))
                }
                Expr::If {
                    condition,
                    then,
                    otherwise,
                } if depth < 2 => {
                    let condition = cell_source(condition, &Type::Boolean, locals, parameters)?;
                    let (then, nested_then) = lower(then, ceiling, depth + 1, locals, parameters)?;
                    let (otherwise, nested_otherwise) =
                        lower(otherwise, ceiling, depth + 1, locals, parameters)?;
                    if depth == 0 && !(nested_then && nested_otherwise) {
                        return None;
                    }
                    Some((
                        syn::parse_quote!(if #condition { #then } else { #otherwise }),
                        depth > 0 || nested_then || nested_otherwise,
                    ))
                }
                Expr::UnsignedLiteral { value, max } => {
                    let max = max.parse::<u64>().ok()?;
                    let value = value.parse::<u64>().ok()?;
                    (max <= ceiling && value <= max).then(|| {
                        let literal = syn::LitInt::new(&format!("{value}u64"), Span::call_site());
                        (syn::parse_quote!(#literal), false)
                    })
                }
                _ => None,
            }
        }
        let Expr::If { .. } = value else { return None };
        let (selected, nested) = lower(value, 4, 0, locals, parameters)?;
        nested.then_some(selected)
    }

    fn closed_counter_one_continuation(action: &StateAction) -> bool {
        let StateAction::Let { bindings, action } = action else {
            return false;
        };
        let [binding] = bindings.as_slice() else {
            return false;
        };
        binding.ty
            == (Type::Unsigned {
                max: "65535".into(),
            })
            && matches!(
                &binding.value,
                Expr::UnsignedLiteral { value, max }
                    if value == "1" && max == "65535"
            )
            && matches!(
                action.as_ref(),
                StateAction::CounterIncrement { amount: CounterAmount::Parameter { name }, .. }
                    if name == &binding.name
            )
    }

    // The compiler emits each element of the closed two-Field vector as a
    // ternary over a Boolean already retained by recording. Both arms must
    // be small, checked literals; no element can perform a VM or witness
    // operation while the vector is built.
    fn closed_conditional_field_pair(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<(syn::Expr, syn::Expr)> {
        fn field_arm(value: &Expr) -> Option<syn::Expr> {
            let Expr::Coerce {
                value,
                ty: Type::Field,
            } = value
            else {
                return None;
            };
            let number = match value.as_ref() {
                Expr::UnsignedLiteral { value, max } => {
                    let max = max.parse::<u64>().ok()?;
                    let value = value.parse::<u64>().ok()?;
                    (max <= 4 && value <= max).then_some(value)?
                }
                Expr::FieldLiteral { value } => {
                    let value = value.parse::<u64>().ok()?;
                    (value <= 4).then_some(value)?
                }
                _ => return None,
            };
            let number = syn::LitInt::new(&format!("{number}u64"), Span::call_site());
            Some(syn::parse_quote!(runtime::Field::from(#number)))
        }

        fn element(
            value: &Expr,
            locals: &HashMap<String, syn::Expr>,
            parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ) -> Option<(String, syn::Expr)> {
            let Expr::If {
                condition,
                then,
                otherwise,
            } = value
            else {
                return None;
            };
            let Expr::Parameter { name } = condition.as_ref() else {
                return None;
            };
            if !locals.contains_key(name) {
                return None;
            }
            let predicate = cell_source(condition, &Type::Boolean, locals, parameters)?;
            let then = field_arm(then)?;
            let otherwise = field_arm(otherwise)?;
            Some((
                name.clone(),
                syn::parse_quote!(if #predicate { #then } else { #otherwise }),
            ))
        }

        let Expr::Vector {
            element: ty,
            elements,
        } = value
        else {
            return None;
        };
        if *ty != Type::Field {
            return None;
        }
        let [first, second] = elements.as_slice() else {
            return None;
        };
        let (first_source, first) = element(first, locals, parameters)?;
        let (second_source, second) = element(second, locals, parameters)?;
        (first_source == second_source).then_some((first, second))
    }

    // A Uint<8> annotation widens the compiler's closed Uint<2> literal
    // ternary. Keep the outer cast and both arm bounds explicit here: the
    // recorded local may not evaluate an unrecorded operation.
    fn closed_annotated_uint8_source(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<syn::Expr> {
        fn arm(value: &Expr) -> Option<syn::Expr> {
            let Expr::Coerce {
                value,
                ty: Type::Unsigned { max },
            } = value
            else {
                return None;
            };
            let Expr::UnsignedLiteral {
                value: literal,
                max: literal_max,
            } = value.as_ref()
            else {
                return None;
            };
            if max != "2" || literal_max != max {
                return None;
            }
            let literal = literal.parse::<u64>().ok()?;
            if literal > 2 {
                return None;
            }
            let literal = syn::LitInt::new(&format!("{literal}u128"), Span::call_site());
            Some(syn::parse_quote!(#literal))
        }

        let Expr::UnsignedCast { max, value } = value else {
            return None;
        };
        if max != "255" {
            return None;
        }
        let Expr::If {
            condition,
            then,
            otherwise,
        } = value.as_ref()
        else {
            return None;
        };
        let condition = cell_source(condition, &Type::Boolean, locals, parameters)?;
        let then = arm(then)?;
        let otherwise = arm(otherwise)?;
        Some(syn::parse_quote!(if #condition { #then } else { #otherwise }))
    }

    // Hash-to-curve is pure, but only admit the compiler's closed two-arm
    // Uint<2> -> Field input. The predicate must already be available as a
    // Boolean parameter or recorded local; neither arm may observe state.
    fn closed_curve_field_argument(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<syn::Expr> {
        fn arm(value: &Expr) -> Option<syn::Expr> {
            let Expr::Coerce {
                value,
                ty: Type::Unsigned { max },
            } = value
            else {
                return None;
            };
            let Expr::UnsignedLiteral {
                value: literal,
                max: literal_max,
            } = value.as_ref()
            else {
                return None;
            };
            if max != "2" || literal_max != max {
                return None;
            }
            let literal = literal.parse::<u64>().ok()?;
            if literal > 2 {
                return None;
            }
            let literal = syn::LitInt::new(&format!("{literal}u64"), Span::call_site());
            Some(syn::parse_quote!(runtime::Field::from(#literal)))
        }

        let Expr::HashToCurve { value } = value else {
            return None;
        };
        let Expr::FieldCast { value } = value.as_ref() else {
            return None;
        };
        let Expr::If {
            condition,
            then,
            otherwise,
        } = value.as_ref()
        else {
            return None;
        };
        let condition = cell_source(condition, &Type::Boolean, locals, parameters)?;
        let then = arm(then)?;
        let otherwise = arm(otherwise)?;
        Some(syn::parse_quote!(if #condition { #then } else { #otherwise }))
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
                | Type::OpaqueString
                | Type::Enum { .. }
                | Type::Struct { .. }
                | Type::Tuple { .. }
                | Type::Vector { .. }
        ) {
            return None;
        }
        match value {
            Expr::Coerce {
                value: source,
                ty: target,
            } if target == ty => {
                if let (Type::Vector { length, element }, Expr::Tuple { elements }) =
                    (ty, source.as_ref())
                    && **element == Type::Field
                    && *length == elements.len()
                    && elements
                        .iter()
                        .all(|element| matches!(element, Expr::FieldLiteral { .. }))
                {
                    let (rendered, actual) =
                        expression_with_calls(value, parameters, &HashMap::new()).ok()?;
                    return (actual == *ty).then_some(rendered);
                }
                cell_source(source, ty, locals, parameters)
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
            Expr::Default { ty: default_ty }
                if default_ty == ty && !matches!(ty, Type::OpaqueString) =>
            {
                let rust_ty = rust_type(ty).ok()?;
                Some(syn::parse_quote!(<#rust_ty as Default>::default()))
            }
            Expr::UnsignedLiteral { .. } if matches!(ty, Type::Unsigned { .. }) => {
                let (rendered, actual) =
                    expression_with_calls(value, parameters, &HashMap::new()).ok()?;
                (actual == *ty).then_some(rendered)
            }
            Expr::FieldLiteral { .. } if *ty == Type::Field => {
                let (rendered, actual) =
                    expression_with_calls(value, parameters, &HashMap::new()).ok()?;
                (actual == *ty).then_some(rendered)
            }
            Expr::BytesLiteral { bytes }
                if *ty
                    == (Type::Bytes {
                        length: bytes.len(),
                    }) =>
            {
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

    /// Hashing itself is pure, but its FAB encoding depends on the exact
    /// operand type. Admit the compiler's closed two-Field tuple only; a
    /// ledger read, witness, or call inside the operand needs ordered lowering.
    fn literal_field_pair_hash_source(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<syn::Expr> {
        let Expr::Tuple { elements } = value else {
            return None;
        };
        let [left, right] = elements.as_slice() else {
            return None;
        };
        if !matches!(left, Expr::FieldLiteral { .. }) || !matches!(right, Expr::FieldLiteral { .. })
        {
            return None;
        }
        let left = cell_source(left, &Type::Field, locals, parameters)?;
        let right = cell_source(right, &Type::Field, locals, parameters)?;
        Some(syn::parse_quote!((#left, #right)))
    }

    fn static_bindings(
        bindings: &[LocalBinding],
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        steps: &mut Vec<syn::Stmt>,
        next_temp: &mut usize,
    ) -> Result<Option<HashMap<String, syn::Expr>>, RenderError> {
        let mut scoped = locals.clone();
        for binding in bindings {
            if !matches!(binding.ty, Type::Vector { .. })
                && !(binding.ty == Type::Field
                    && matches!(binding.value, Expr::FieldLiteral { .. }))
            {
                return Ok(None);
            }
            let Some(value) = cell_source(&binding.value, &binding.ty, &scoped, parameters) else {
                return Ok(None);
            };
            let rust_ty = rust_type(&binding.ty)?;
            let name = syn::Ident::new(
                &format!("__compact_recorded_static_{}", *next_temp),
                Span::call_site(),
            );
            *next_temp += 1;
            steps.push(syn::parse_quote!(let #name: #rust_ty = #value;));
            scoped.insert(binding.name.clone(), syn::parse_quote!(#name));
        }
        Ok(Some(scoped))
    }

    /// A Boolean If arm can fold a closed, same-type unsigned equality.
    /// This never records an effect from the unselected branch.
    fn closed_boolean_assertion_arm(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<syn::Expr> {
        if let Expr::Equal { left, right } = value
            && let (
                Expr::UnsignedLiteral {
                    value: left_value,
                    max: left_max,
                },
                Expr::UnsignedLiteral {
                    value: right_value,
                    max: right_max,
                },
            ) = (left.as_ref(), right.as_ref())
            && left_max == right_max
        {
            let maximum = left_max.parse::<u128>().ok()?;
            let left = left_value.parse::<u128>().ok()?;
            let right = right_value.parse::<u128>().ok()?;
            if left <= maximum && right <= maximum {
                let folded = syn::LitBool::new(left == right, Span::call_site());
                return Some(syn::parse_quote!(#folded));
            }
        }
        cell_source(value, &Type::Boolean, locals, parameters)
    }

    // A compiler-typed Uint comparison may select one of two closed literal
    // arms. Keep its selection pure; any observation must already be bound in
    // `locals`, so this cannot move a VM read across the comparison.
    fn closed_unsigned_ternary_comparison(
        left: &Expr,
        right: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
        equal: bool,
    ) -> Option<syn::Expr> {
        fn literal_arm(value: &Expr) -> Option<(u128, u128)> {
            let Expr::Coerce {
                value,
                ty: Type::Unsigned { max },
            } = value
            else {
                return None;
            };
            let Expr::UnsignedLiteral {
                value: literal,
                max: literal_max,
            } = value.as_ref()
            else {
                return None;
            };
            let max = max.parse::<u128>().ok()?;
            let literal_max = literal_max.parse::<u128>().ok()?;
            let literal = literal.parse::<u128>().ok()?;
            (max == literal_max && max <= u8::MAX as u128 && literal <= max)
                .then_some((literal, max))
        }

        fn selected_literal(
            value: &Expr,
            locals: &HashMap<String, syn::Expr>,
            parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ) -> Option<(syn::Expr, u128)> {
            let (value, cast_max) = match value {
                Expr::UnsignedCast { max, value } => {
                    (value.as_ref(), Some(max.parse::<u128>().ok()?))
                }
                other => (other, None),
            };
            let Expr::If {
                condition,
                then,
                otherwise,
            } = value
            else {
                return None;
            };
            let (then_value, arm_max) = literal_arm(then)?;
            let (otherwise_value, other_max) = literal_arm(otherwise)?;
            if arm_max != other_max {
                return None;
            }
            let target_max = cast_max.unwrap_or(arm_max);
            if target_max < arm_max || target_max > u8::MAX as u128 {
                return None;
            }
            let condition = cell_source(condition, &Type::Boolean, locals, parameters)?;
            let then_value = syn::LitInt::new(&format!("{then_value}u128"), Span::call_site());
            let otherwise_value =
                syn::LitInt::new(&format!("{otherwise_value}u128"), Span::call_site());
            Some((
                syn::parse_quote!(if #condition { #then_value } else { #otherwise_value }),
                target_max,
            ))
        }

        fn comparable_operand(
            value: &Expr,
            max: u128,
            parameters: &HashMap<&str, (&Type, syn::Ident)>,
        ) -> Option<syn::Expr> {
            match value {
                Expr::Parameter { name } => {
                    let (ty, rust_name) = parameters.get(name.as_str())?;
                    let Type::Unsigned { max: actual_max } = ty else {
                        return None;
                    };
                    (actual_max.parse::<u128>().ok()? == max)
                        .then(|| syn::parse_quote!(#rust_name.value()))
                }
                Expr::UnsignedLiteral {
                    value,
                    max: actual_max,
                } => {
                    let actual_max = actual_max.parse::<u128>().ok()?;
                    let value = value.parse::<u128>().ok()?;
                    if actual_max != max || value > max {
                        return None;
                    }
                    let value = syn::LitInt::new(&format!("{value}u128"), Span::call_site());
                    Some(syn::parse_quote!(#value))
                }
                _ => None,
            }
        }

        let (selected, max, other) = selected_literal(right, locals, parameters)
            .map(|(selected, max)| (selected, max, left))
            .or_else(|| {
                selected_literal(left, locals, parameters)
                    .map(|(selected, max)| (selected, max, right))
            })?;
        let other = comparable_operand(other, max, parameters)?;
        Some(if equal {
            syn::parse_quote!(#other == #selected)
        } else {
            syn::parse_quote!(#other != #selected)
        })
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
                let Some(scoped) = static_bindings(bindings, locals, parameters, steps, next_temp)?
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
            Expr::FieldCast { value } => {
                // A bounded unsigned public parameter has no recording
                // effects. Retain its full u128 value before converting to
                // the ledger Field; this is the same typed cast as native
                // stateful lowering, including values above u64::MAX.
                if let Expr::Parameter { name } = value.as_ref()
                    && !locals.contains_key(name)
                    && let Some((ty @ Type::Unsigned { .. }, _)) = parameters.get(name.as_str())
                    && let Some(unsigned) = cell_source(value, ty, locals, parameters)
                {
                    let cast = syn::Ident::new(
                        &format!("__compact_recorded_field_cast_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote! {
                        let #cast: runtime::Field = runtime::Field::from((#unsigned).value());
                    });
                    return Ok(Some(syn::parse_quote!(#cast)));
                }

                fn bounded_arm(value: &Expr) -> Option<syn::Expr> {
                    let Expr::Coerce {
                        value,
                        ty: Type::Unsigned { max },
                    } = value
                    else {
                        return None;
                    };
                    let Expr::UnsignedLiteral {
                        value: literal,
                        max: literal_max,
                    } = value.as_ref()
                    else {
                        return None;
                    };
                    if max != "2" || literal_max != max {
                        return None;
                    }
                    let literal = literal.parse::<u64>().ok()?;
                    if literal > 2 {
                        return None;
                    }
                    let literal = syn::LitInt::new(&format!("{literal}u64"), Span::call_site());
                    Some(syn::parse_quote!(runtime::Field::from(#literal)))
                }

                let Expr::If {
                    condition,
                    then,
                    otherwise,
                } = value.as_ref()
                else {
                    return Ok(None);
                };
                let (Some(then), Some(otherwise)) = (bounded_arm(then), bounded_arm(otherwise))
                else {
                    return Ok(None);
                };
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
                    return Ok(None);
                };
                Ok(Some(syn::parse_quote!(if #condition {
                    #then
                } else {
                    #otherwise
                })))
            }
            Expr::Parameter { .. } => Ok(cell_source(value, &Type::Field, locals, parameters)),
            Expr::FieldLiteral { .. } => {
                let (literal, ty) = expression_with_calls(value, parameters, &HashMap::new())?;
                Ok((ty == Type::Field).then_some(literal))
            }
            // A pure conditional has no recording effects in either arm. Bind
            // the selected Field once, before the following ledger action.
            // Effectful arms stay unavailable until they can be lowered with
            // branch-local RecordingFrame state.
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                let Some(condition) = cell_source(condition, &Type::Boolean, locals, parameters)
                else {
                    return Ok(None);
                };
                let Some(then) = cell_source(then, &Type::Field, locals, parameters) else {
                    return Ok(None);
                };
                let Some(otherwise) = cell_source(otherwise, &Type::Field, locals, parameters)
                else {
                    return Ok(None);
                };
                let selected = syn::Ident::new(
                    &format!("__compact_recorded_conditional_field_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote! {
                    let #selected: runtime::Field = if #condition { #then } else { #otherwise };
                });
                Ok(Some(syn::parse_quote!(#selected)))
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
                        // This Field-returning stateful inlining path has no
                        // pure-circuit context. Keep nested pure calls
                        // unavailable until that context is threaded through.
                        &HashMap::new(),
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
                let Some(scoped) = static_bindings(bindings, locals, parameters, steps, next_temp)?
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
            // The condition may record an existing Boolean observation, but
            // both branches must be effect-free scalar sources. Bind the
            // selected value after that observation and before the Assert.
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
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
                    return Ok(None);
                };
                let Some(then) = closed_boolean_assertion_arm(then, locals, parameters) else {
                    return Ok(None);
                };
                let Some(otherwise) = closed_boolean_assertion_arm(otherwise, locals, parameters)
                else {
                    return Ok(None);
                };
                let selected = syn::Ident::new(
                    &format!("__compact_recorded_conditional_bool_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                let literal = |value: &syn::Expr| match value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Bool(value),
                        ..
                    }) => Some(value.value),
                    _ => None,
                };
                if let (Some(then_value), Some(otherwise_value)) =
                    (literal(&then), literal(&otherwise))
                    && then_value == otherwise_value
                {
                    // The condition may have already recorded its effects.
                    // Evaluate its remaining scalar expression exactly once.
                    steps.push(syn::parse_quote!(let _ = #condition;));
                    steps.push(syn::parse_quote!(let #selected: bool = #then_value;));
                } else {
                    steps.push(syn::parse_quote! {
                        let #selected: bool = if #condition { #then } else { #otherwise };
                    });
                }
                Ok(Some(syn::parse_quote!(#selected)))
            }
            Expr::WitnessCall { name, arguments } => {
                let declaration = witnesses
                    .get(name.as_str())
                    .ok_or_else(|| RenderError::UnknownWitness(name.clone()))?;
                if declaration.result != Type::Boolean {
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
                    let Some(value) = cell_source(argument, &parameter.ty, locals, parameters)
                    else {
                        return Ok(None);
                    };
                    let ty = rust_type(&parameter.ty)?;
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #arg: #ty = #value;));
                    args.push(arg);
                }
                let method = ident(name)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_bool_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote! {
                    let (frame, #observed): (_, bool) = frame.try_witness_metered(|context, meter| {
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
            Expr::SetIsEmpty { field, index }
            | Expr::MapIsEmpty { field, index }
            | Expr::ListIsEmpty { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let matching_kind = matches!(
                    (value, &declaration.declaration),
                    (Expr::SetIsEmpty { .. }, LedgerFieldKind::Set { .. })
                        | (Expr::MapIsEmpty { .. }, LedgerFieldKind::Map { .. })
                        | (Expr::ListIsEmpty { .. }, LedgerFieldKind::List { .. })
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
            Expr::StructField {
                value,
                field,
                index: 0,
            } if field == "is_some" => {
                let Expr::ListHead {
                    field: list_field,
                    index,
                    ty,
                } = value.as_ref()
                else {
                    return Ok(None);
                };
                let declaration = ledger_fields
                    .get(list_field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(list_field.clone()))?;
                let LedgerFieldKind::List { ty: element } = &declaration.declaration else {
                    return Ok(None);
                };
                if declaration.index != *index || *ty != list_head_result_type(element, ty) {
                    return Ok(None);
                }
                let slot = ident(list_field)?;
                let result_ty = rust_type(ty)?;
                let observed = syn::Ident::new(
                    &format!("__compact_recorded_head_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.push(syn::parse_quote!(
                    let (frame, #observed): (_, #result_ty) =
                        crate::ledger_slots::#slot.record_head::<#result_ty, _, _>(frame)?;
                ));
                Ok(Some(syn::parse_quote!(#observed.is_some)))
            }
            Expr::Equal { left, right } | Expr::NotEqual { left, right } => {
                if let Some(comparison) = closed_unsigned_ternary_comparison(
                    left,
                    right,
                    locals,
                    parameters,
                    matches!(value, Expr::Equal { .. }),
                ) {
                    return Ok(Some(comparison));
                }
                let boolean_literal = match (&**left, &**right) {
                    (Expr::Boolean { value }, other) => Some((other, *value)),
                    (other, Expr::Boolean { value }) => Some((other, *value)),
                    _ => None,
                };
                if let Some((other, expected)) = boolean_literal {
                    let Some(observed) = boolean_expression(
                        other,
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
                    return if matches!(value, Expr::Equal { .. }) {
                        Ok(Some(syn::parse_quote!(#observed == #expected)))
                    } else {
                        Ok(Some(syn::parse_quote!(#observed != #expected)))
                    };
                }
                let size_comparison = set_size_operand(left)
                    .and_then(|(field, index)| {
                        uint64_literal(right).map(|value| (field, index, value))
                    })
                    .or_else(|| {
                        set_size_operand(right).and_then(|(field, index)| {
                            uint64_literal(left).map(|value| (field, index, value))
                        })
                    });
                if let Some((field, index, expected)) = size_comparison {
                    let declaration = ledger_fields
                        .get(field)
                        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
                    if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                        || declaration.index != index
                    {
                        return Ok(None);
                    }
                    let slot = ident(&declaration.id)?;
                    let observed = syn::Ident::new(
                        &format!("__compact_recorded_size_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(
                        let (frame, #observed): (_, u64) =
                            crate::ledger_slots::#slot.record_size(frame)?;
                    ));
                    return if matches!(value, Expr::Equal { .. }) {
                        Ok(Some(syn::parse_quote!(#observed == #expected)))
                    } else {
                        Ok(Some(syn::parse_quote!(#observed != #expected)))
                    };
                }
                let length_comparison = list_length_operand(left)
                    .and_then(|(field, index)| {
                        uint64_literal(right).map(|value| (field, index, value))
                    })
                    .or_else(|| {
                        list_length_operand(right).and_then(|(field, index)| {
                            uint64_literal(left).map(|value| (field, index, value))
                        })
                    });
                if let Some((field, index, expected)) = length_comparison {
                    let declaration = ledger_fields
                        .get(field)
                        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
                    if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                        || declaration.index != index
                    {
                        return Ok(None);
                    }
                    let slot = ident(&declaration.id)?;
                    let observed = syn::Ident::new(
                        &format!("__compact_recorded_length_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(
                        let (frame, #observed): (_, u64) =
                            crate::ledger_slots::#slot.record_length(frame)?;
                    ));
                    return if matches!(value, Expr::Equal { .. }) {
                        Ok(Some(syn::parse_quote!(#observed == #expected)))
                    } else {
                        Ok(Some(syn::parse_quote!(#observed != #expected)))
                    };
                }
                let head_comparison = list_head_field(left, "value", 1)
                    .map(|head| (head, right.as_ref()))
                    .or_else(|| {
                        list_head_field(right, "value", 1).map(|head| (head, left.as_ref()))
                    });
                if let Some(((field, index, ty), expected)) = head_comparison {
                    let declaration = ledger_fields
                        .get(field)
                        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
                    let LedgerFieldKind::List { ty: element } = &declaration.declaration else {
                        return Ok(None);
                    };
                    let comparable_element = match element {
                        Type::Field | Type::Enum { .. } | Type::Bytes { .. } => true,
                        Type::Vector { element, .. } => **element == Type::Field,
                        _ => false,
                    };
                    if declaration.index != index
                        || !comparable_element
                        || *ty != list_head_result_type(element, ty)
                    {
                        return Ok(None);
                    }
                    let Some(expected) = cell_source(expected, element, locals, parameters) else {
                        return Ok(None);
                    };
                    let slot = ident(field)?;
                    let result_ty = rust_type(ty)?;
                    let observed = syn::Ident::new(
                        &format!("__compact_recorded_head_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(
                        let (frame, #observed): (_, #result_ty) =
                            crate::ledger_slots::#slot.record_head::<#result_ty, _, _>(frame)?;
                    ));
                    return if matches!(value, Expr::Equal { .. }) {
                        Ok(Some(syn::parse_quote!(#observed.value == #expected)))
                    } else {
                        Ok(Some(syn::parse_quote!(#observed.value != #expected)))
                    };
                }
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
                // A closed Field-pair hash is a pure value operation. Keep it
                // before the declared Cell read in the Boolean helper and let
                // that read alone contribute to the public VM transcript.
                let comparison = match result {
                    Expr::Equal { left, right } => Some((left.as_ref(), right.as_ref(), true)),
                    Expr::NotEqual { left, right } => Some((left.as_ref(), right.as_ref(), false)),
                    _ => None,
                };
                if let (Some((left, right, equal)), [parameter], [argument]) = (
                    comparison,
                    callee.parameters.as_slice(),
                    arguments.as_slice(),
                ) && field_pair_type(&parameter.ty)
                    && let Expr::TransientHash { value: hashed } = left
                    && matches!(hashed.as_ref(), Expr::Parameter { name } if name == &parameter.name)
                    && let Expr::CellRead { field, index } = right
                {
                    let declaration = ledger_fields
                        .get(field.as_str())
                        .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                    if declaration.declaration == (LedgerFieldKind::Cell { ty: Type::Field })
                        && declaration.index == *index
                    {
                        let Some(argument) =
                            cell_source(argument, &parameter.ty, locals, parameters)
                        else {
                            return Ok(None);
                        };
                        let arg = syn::Ident::new(
                            &format!("__compact_recorded_hash_arg_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        let ty = rust_type(&parameter.ty)?;
                        let hash = syn::Ident::new(
                            &format!("__compact_recorded_hash_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        let observed = syn::Ident::new(
                            &format!("__compact_recorded_value_{}", *next_temp),
                            Span::call_site(),
                        );
                        *next_temp += 1;
                        let slot = ident(field)?;
                        steps.push(syn::parse_quote!(let #arg: #ty = #argument;));
                        steps.push(syn::parse_quote!(
                            let #hash: runtime::Field = runtime::transient_hash(#arg);
                        ));
                        steps.push(syn::parse_quote!(
                            let (frame, #observed): (_, runtime::Field) =
                                crate::ledger_slots::#slot.record_read(frame)?;
                        ));
                        return if equal {
                            Ok(Some(syn::parse_quote!(#hash == #observed)))
                        } else {
                            Ok(Some(syn::parse_quote!(#hash != #observed)))
                        };
                    }
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
                        | Type::OpaqueString
                        | Type::Enum { .. }
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
            | Type::OpaqueString
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
        pure_circuits: &HashMap<&str, &PureCircuit>,
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
                        pure_circuits,
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
            StateAction::Expression {
                value: Expr::WitnessCall { name, arguments },
            } => {
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
                if declaration.result != Type::Unit && declaration.result != Type::Field {
                    return Ok(unavailable_action(action, path));
                }
                if declaration.result == Type::Unit
                    && !(arguments.is_empty()
                        || matches!(declaration.parameters.as_slice(), [parameter] if parameter.ty == Type::OpaqueString))
                {
                    return Ok(unavailable_action(action, path));
                }
                let mut args = Vec::new();
                for (index, (argument, parameter)) in
                    arguments.iter().zip(&declaration.parameters).enumerate()
                {
                    let Some(value) = cell_source(argument, &parameter.ty, locals, parameters)
                    else {
                        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                            argument,
                            format!("{path}.value.arguments[{index}]"),
                        )));
                    };
                    let ty = rust_type(&parameter.ty)?;
                    let arg = syn::Ident::new(
                        &format!("__compact_recorded_witness_arg_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote!(let #arg: #ty = #value;));
                    args.push(arg);
                }
                let method = ident(name)?;
                steps.push(syn::parse_quote! {
                    let (frame, _) = frame.try_witness_metered(|context, meter| {
                        witnesses.#method(context.witness_context_with(super::LedgerView {
                            state: context.query.state.get_ref(),
                            meter,
                        }), #(#args),*)
                    })?;
                });
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                let mut condition_steps = Vec::new();
                let Some(condition) = boolean_expression(
                    condition,
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    &mut condition_steps,
                    next_temp,
                    visiting,
                )?
                else {
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        condition,
                        format!("{path}.condition"),
                    )));
                };
                let mut then_steps = Vec::new();
                if let RecordingOutcome::Unsupported(gap) = append_steps(
                    then,
                    &format!("{path}.then"),
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                    shared_callees,
                    &mut then_steps,
                    next_temp,
                    visiting,
                )? {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
                let mut otherwise_steps = Vec::new();
                if let RecordingOutcome::Unsupported(gap) = append_steps(
                    otherwise,
                    &format!("{path}.otherwise"),
                    locals,
                    parameters,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                    shared_callees,
                    &mut otherwise_steps,
                    next_temp,
                    visiting,
                )? {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
                let selected = syn::Ident::new(
                    &format!("__compact_recorded_branch_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                steps.extend(condition_steps);
                steps.push(syn::parse_quote!(let #selected: bool = #condition;));
                steps.push(syn::parse_quote!(
                    #[allow(clippy::let_and_return)]
                    let frame = if #selected {
                        #(#then_steps)*
                        frame
                    } else {
                        #(#otherwise_steps)*
                        frame
                    };
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::PureCall { name, arguments } => {
                let Some(callee) = pure_circuits.get(name.as_str()) else {
                    return Ok(unavailable_action(action, path));
                };
                if !closed_pure_assert_call(callee) || callee.result != Type::Unit {
                    return Ok(unavailable_action(action, path));
                }
                let [parameter] = callee.parameters.as_slice() else {
                    unreachable!("closed pure assertion has one parameter")
                };
                let [argument] = arguments.as_slice() else {
                    return Ok(unavailable_action(action, path));
                };
                let Some(argument) = cell_source(argument, &parameter.ty, locals, parameters)
                else {
                    return Ok(unavailable_action(action, path));
                };
                let method = ident(name)?;
                let typed_arg = syn::Ident::new(
                    &format!("__compact_recorded_pure_assert_arg_{}", *next_temp),
                    Span::call_site(),
                );
                *next_temp += 1;
                let arg_ty = rust_type(&parameter.ty)?;
                steps.push(syn::parse_quote!(let #typed_arg: #arg_ty = #argument;));
                steps.push(syn::parse_quote!(crate::pure_circuits::#method(#typed_arg)?;));
                Ok(RecordingOutcome::Supported(()))
            }
            whole @ StateAction::Let {
                bindings,
                action: nested_action,
            } => {
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
                    )?;
                    if let RecordingOutcome::Unsupported(_) = first {
                        return Ok(first);
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
                    );
                }
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
                    );
                }
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
                    );
                }
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
                    );
                }
                // Preserve the typed Uint<4> local and its immediate Field
                // projection. A Sequence may put a Counter increment after
                // that projection, but the projection itself remains first.
                if let [unsigned_binding] = bindings.as_slice()
                    && unsigned_binding.ty == (Type::Unsigned { max: "4".into() })
                    && let Some(selected) =
                        closed_nested_uint4_source(&unsigned_binding.value, locals, parameters)
                    && let Some((projected_bindings, projected_action, continuation)) =
                        (match nested_action.as_ref() {
                            StateAction::Let { bindings, action } => {
                                Some((bindings, action.as_ref(), None))
                            }
                            StateAction::Sequence { actions } => {
                                if let [StateAction::Let { bindings, action }, continuation] =
                                    actions.as_slice()
                                {
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
                    )?;
                    if let RecordingOutcome::Unsupported(_) = first {
                        return Ok(first);
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
                        );
                    }
                    return Ok(RecordingOutcome::Supported(()));
                }
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
                        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                            &literal_fields[0],
                            format!("{path}.bindings[0].value.fields[0]"),
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
                    );
                }
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
                    );
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
                        (Type::Field, Expr::TransientHash { value }) => {
                            Some((value.as_ref(), false))
                        }
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
                            let Some(value) =
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
                        let Some(value) =
                            cell_source(&binding.value, &binding.ty, &scoped, parameters)
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
                            && declaration.declaration
                                == (LedgerFieldKind::Cell { ty: Type::Field })
                        {
                            let Some(arg_value) =
                                cell_source(argument, &callee_parameter.ty, &scoped, parameters)
                            else {
                                return Ok(RecordingOutcome::Unsupported(
                                    RecordingGap::expression(
                                        argument,
                                        format!(
                                            "{path}.bindings[{binding_index}].value.arguments[0]"
                                        ),
                                    ),
                                ));
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
                                steps
                                    .push(syn::parse_quote!(let #arg: runtime::Field = #argument;));
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
                        let Some(argument) =
                            cell_source(argument, &parameter.ty, &scoped, parameters)
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
                        let Some(value) =
                            cell_source(&binding.value, &binding.ty, &scoped, parameters)
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
                        let Some(value) =
                            cell_source(&binding.value, &binding.ty, &scoped, parameters)
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
                        let Some(value) =
                            cell_source(&binding.value, &binding.ty, &scoped, parameters)
                        else {
                            return Ok(unavailable_action(whole, path));
                        };
                        scoped.insert(binding.name.clone(), value);
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
                )
            }
            StateAction::Assert { condition, message } => {
                // A typed public Uint<8> may be widened to Uint<32> for an
                // equality guard. Keep the checked cast and assertion before
                // any later public action; no operand may observe ledger or
                // witness state on this path.
                if let Expr::Equal { left, right } = condition
                    && let Expr::UnsignedCast {
                        max,
                        value: small_value,
                    } = left.as_ref()
                    && max == "4294967295"
                    && let Expr::Parameter { name: small_name } = small_value.as_ref()
                    && let Expr::Parameter { name: big_name } = right.as_ref()
                    && !locals.contains_key(small_name)
                    && !locals.contains_key(big_name)
                    && matches!(
                        parameters.get(small_name.as_str()),
                        Some((Type::Unsigned { max }, _)) if max == "255"
                    )
                    && matches!(
                        parameters.get(big_name.as_str()),
                        Some((Type::Unsigned { max }, _)) if max == "4294967295"
                    )
                {
                    let Some(small) = cell_source(
                        small_value,
                        &Type::Unsigned { max: "255".into() },
                        locals,
                        parameters,
                    ) else {
                        return Ok(unavailable_action(action, path));
                    };
                    let Some(big) = cell_source(
                        right,
                        &Type::Unsigned {
                            max: "4294967295".into(),
                        },
                        locals,
                        parameters,
                    ) else {
                        return Ok(unavailable_action(action, path));
                    };
                    let widened = syn::Ident::new(
                        &format!("__compact_recorded_widened_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    steps.push(syn::parse_quote! {
                        let #widened: runtime::BoundedUint<4294967295> =
                            runtime::cast_unsigned::<255, 4294967295>(#small)?;
                    });
                    steps.push(syn::parse_quote! {
                        if !(#widened == #big) {
                            return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                        }
                    });
                    return Ok(RecordingOutcome::Supported(()));
                }
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
                        pure_circuits,
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
                        | Type::Enum { .. }
                        | Type::Tuple { .. }
                        | Type::Struct { .. }
                        | Type::Vector { .. }
                ) && !(*ty == Type::OpaqueString
                    && matches!(action, StateAction::SetInsert { .. }))
                {
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
            StateAction::MerkleInsertHash { field, index, hash } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::MerkleTree { .. })
                    || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(unavailable_action(action, path));
                }
                let Some(hash) = cell_source(hash, &Type::Bytes { length: 32 }, locals, parameters)
                else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert_hash(frame, #hash)?;
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::HistoricMerkleInsertHash { field, index, hash } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(
                    declaration.declaration,
                    LedgerFieldKind::HistoricMerkleTree { .. }
                ) || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(unavailable_action(action, path));
                }
                let Some(hash) = cell_source(hash, &Type::Bytes { length: 32 }, locals, parameters)
                else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert_hash(frame, #hash)?;
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::MerkleInsertHashIndex {
                field,
                index,
                hash,
                position,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::MerkleTree { .. })
                    || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(unavailable_action(action, path));
                }
                let position_ty = Type::Unsigned {
                    max: u64::MAX.to_string(),
                };
                let (Some(hash), Some(position)) = (
                    cell_source(hash, &Type::Bytes { length: 32 }, locals, parameters),
                    cell_source(position, &position_ty, locals, parameters),
                ) else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert_hash_index(frame, #hash, #position)?;
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::HistoricMerkleInsertHashIndex {
                field,
                index,
                hash,
                position,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(
                    declaration.declaration,
                    LedgerFieldKind::HistoricMerkleTree { .. }
                ) || declaration.index != *index
                    || declaration.physical_path().len() != 1
                {
                    return Ok(unavailable_action(action, path));
                }
                let position_ty = Type::Unsigned {
                    max: u64::MAX.to_string(),
                };
                let (Some(hash), Some(position)) = (
                    cell_source(hash, &Type::Bytes { length: 32 }, locals, parameters),
                    cell_source(position, &position_ty, locals, parameters),
                ) else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert_hash_index(frame, #hash, #position)?;
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::MerkleInsertIndex {
                field,
                index,
                value,
                position,
            }
            | StateAction::HistoricMerkleInsertIndex {
                field,
                index,
                value,
                position,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let (ty, historic) = match &declaration.declaration {
                    LedgerFieldKind::MerkleTree { ty, .. } => (ty, false),
                    LedgerFieldKind::HistoricMerkleTree { ty, .. } => (ty, true),
                    _ => return Ok(unavailable_action(action, path)),
                };
                if declaration.index != *index
                    || declaration.physical_path().len() != 1
                    || historic != matches!(action, StateAction::HistoricMerkleInsertIndex { .. })
                {
                    return Ok(unavailable_action(action, path));
                }
                let position_ty = Type::Unsigned {
                    max: u64::MAX.to_string(),
                };
                let (Some(value), Some(position)) = (
                    cell_source(value, ty, locals, parameters),
                    cell_source(position, &position_ty, locals, parameters),
                ) else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert_index(frame, #value, #position)?;
                ));
                Ok(RecordingOutcome::Supported(()))
            }
            StateAction::MerkleInsertIndexDefault {
                field,
                index,
                position,
            }
            | StateAction::HistoricMerkleInsertIndexDefault {
                field,
                index,
                position,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let historic = match &declaration.declaration {
                    LedgerFieldKind::MerkleTree { .. } => false,
                    LedgerFieldKind::HistoricMerkleTree { .. } => true,
                    _ => return Ok(unavailable_action(action, path)),
                };
                if declaration.index != *index
                    || declaration.physical_path().len() != 1
                    || historic
                        != matches!(action, StateAction::HistoricMerkleInsertIndexDefault { .. })
                {
                    return Ok(unavailable_action(action, path));
                }
                let position_ty = Type::Unsigned {
                    max: u64::MAX.to_string(),
                };
                let Some(position) = cell_source(position, &position_ty, locals, parameters) else {
                    return Ok(unavailable_action(action, path));
                };
                let slot = ident(field)?;
                steps.push(syn::parse_quote!(
                    let frame = crate::ledger_slots::#slot.record_insert_index_default(frame, #position)?;
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

    let organizer_steps =
        closed_organizer_gate_steps(circuit, ledger_fields, witnesses, pure_circuits, circuits)?;
    let organizer_gate = organizer_steps.is_some();
    let opaque_map_operation = closed_opaque_map_operation(circuit, ledger_fields);
    let opaque_asset_removal = closed_opaque_asset_removal(circuit, ledger_fields);
    let custody_steps =
        closed_custody_grant_steps(circuit, &parameters, ledger_fields, pure_circuits)?;
    let custody_gate = custody_steps.is_some();
    let guarded_pure_steps =
        closed_guarded_struct_pure_steps(circuit, &parameters, pure_circuits, circuits)?.or(
            closed_guarded_unsigned_product_steps(circuit, &parameters, pure_circuits)?,
        );
    let guarded_pure_call = guarded_pure_steps.is_some();
    let mut steps = organizer_steps
        .or(custody_steps)
        .or(guarded_pure_steps)
        .unwrap_or_default();
    let mut next_temp = 0;
    let mut visiting = HashSet::from([circuit.name.clone()]);
    // A root Let encloses both its ordered actions and its final return.
    // Capture this exact Field Cell read once, before the nested actions,
    // so the returned value cannot accidentally observe the post-write state.
    let root_field_return = if !organizer_gate && !custody_gate && !guarded_pure_call {
        match (circuit.actions.as_slice(), &circuit.return_value) {
            (
                [StateAction::Let { bindings, action }],
                StateReturn::Expression {
                    value: Expr::Parameter { name },
                },
            ) if circuit.result == Type::Field => match bindings.as_slice() {
                [binding]
                    if binding.name == *name
                        && binding.ty == Type::Field
                        && matches!(&binding.value, Expr::CellRead { .. }) =>
                {
                    if let Expr::CellRead { field, index } = &binding.value {
                        let declaration = ledger_fields
                            .get(field.as_str())
                            .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                        if declaration.declaration != (LedgerFieldKind::Cell { ty: Type::Field })
                            || declaration.index != *index
                        {
                            return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                                &circuit.return_value,
                            )));
                        }
                        Some((binding.name.as_str(), field.as_str(), action.as_ref()))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    };
    let mut recorded_root_return: Option<(&str, syn::Expr)> = None;
    if !organizer_gate && !custody_gate {
        if let Some((name, field, action)) = root_field_return {
            let slot = ident(field)?;
            let saved = syn::Ident::new("__compact_recorded_root_return", Span::call_site());
            steps.push(syn::parse_quote! {
                let (frame, #saved): (_, runtime::Field) =
                    crate::ledger_slots::#slot.record_read(frame)?;
            });
            let locals = HashMap::from([(name.to_owned(), syn::parse_quote!(#saved))]);
            if let RecordingOutcome::Unsupported(gap) = append_steps(
                action,
                "actions[0].action",
                &locals,
                &parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                &mut steps,
                &mut next_temp,
                &mut visiting,
            )? {
                return Ok(RecordingOutcome::Unsupported(gap));
            }
            recorded_root_return = Some((name, syn::parse_quote!(#saved)));
        } else {
            for (index, action) in circuit.actions.iter().enumerate() {
                if guarded_pure_call && index == 0 {
                    continue;
                }
                if let RecordingOutcome::Unsupported(gap) = append_steps(
                    action,
                    &format!("actions[{index}]"),
                    &HashMap::new(),
                    &parameters,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                    shared_callees,
                    &mut steps,
                    &mut next_temp,
                    &mut visiting,
                )? {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
    }
    let result_ty = rust_type(&circuit.result)?;
    let (return_steps, result): (Vec<syn::Stmt>, syn::Expr) = match &circuit.return_value {
        StateReturn::Expression {
            value: Expr::Parameter { name },
        } if recorded_root_return
            .as_ref()
            .is_some_and(|(bound_name, _)| *bound_name == name.as_str()) =>
        {
            (
                Vec::new(),
                recorded_root_return.expect("matching root local").1,
            )
        }
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
        StateReturn::Expression {
            value: Expr::Call { name, arguments },
        } if circuit.result == Type::Field && !circuit.actions.is_empty() => {
            let Some(callee) = pure_circuits.get(name.as_str()) else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            if callee.result != Type::Field
                || !closed_pure_field_call(name, pure_circuits, &mut HashSet::new())
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            }
            if arguments.len() != callee.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: callee.parameters.len(),
                    actual: arguments.len(),
                });
            }
            let mut return_steps = Vec::new();
            let mut args = Vec::new();
            for (index, (argument, parameter)) in
                arguments.iter().zip(&callee.parameters).enumerate()
            {
                if parameter.ty != Type::Field {
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        argument,
                        format!("return_value.value.arguments[{index}]"),
                    )));
                }
                let Some(value) = field_expression(
                    argument,
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
                        argument,
                        format!("return_value.value.arguments[{index}]"),
                    )));
                };
                let arg = syn::Ident::new(
                    &format!("__compact_recorded_arg_{}", next_temp),
                    Span::call_site(),
                );
                next_temp += 1;
                return_steps.push(syn::parse_quote!(let #arg: runtime::Field = #value;));
                args.push(arg);
            }
            let method = ident(name)?;
            let result = syn::Ident::new(
                &format!("__compact_recorded_pure_return_{}", next_temp),
                Span::call_site(),
            );
            return_steps.push(syn::parse_quote! {
                let #result: runtime::Field = crate::pure_circuits::#method(#(#args),*)?;
            });
            (return_steps, syn::parse_quote!(#result))
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
        StateReturn::Expression { .. }
            if matches!(
                opaque_map_operation,
                Some(ClosedOpaqueMapOperation::Ensure { .. })
            ) =>
        {
            let Some(ClosedOpaqueMapOperation::Ensure { field, key }) =
                opaque_map_operation.as_ref()
            else {
                unreachable!()
            };
            let slot = ident(field)?;
            let (_, key_ident) = parameters
                .get(key.as_str())
                .expect("closed Map key is a declared parameter");
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, runtime::Field) =
                        crate::ledger_slots::#slot.record_lookup(frame, (#key_ident).clone())?;
                )],
                syn::parse_quote!(observed),
            )
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
        StateReturn::MerkleCheckRoot { field, index, root }
            if circuit.result == Type::Boolean && circuit.actions.is_empty() =>
        {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let Expr::Parameter { name } = root else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            let Some((root_ty, _)) = parameters.get(name.as_str()) else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            if declaration.index != *index
                || !matches!(declaration.declaration, LedgerFieldKind::MerkleTree { .. })
                || !matches!(root_ty, Type::Struct { name, fields }
                    if name == "MerkleTreeDigest" && fields.len() == 1
                        && fields[0].name == "field" && fields[0].ty == Type::Field)
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            }
            let Some(root) = cell_source(root, root_ty, &HashMap::new(), &parameters) else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, bool) =
                        crate::ledger_slots::#slot.record_check_root(frame, #root)?;
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
    if circuit
        .parameters
        .iter()
        .any(|parameter| parameter.ty == Type::OpaqueString)
        && !proved_opaque_set_sequence(circuit)
        && !closed_opaque_set_operation(circuit, ledger_fields)
        && opaque_map_operation.is_none()
        && !opaque_asset_removal
        && !custody_gate
        && !organizer_gate
    {
        return Ok(RecordingOutcome::Unsupported(
            match circuit.actions.first() {
                Some(action) => RecordingGap::action(action, "actions[0]".to_owned()),
                None => RecordingGap::returned(&circuit.return_value),
            },
        ));
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

#[cfg(test)]
mod unsigned_call_tests {
    use super::*;
    use crate::ir::Parameter;

    fn uint(max: &str) -> Type {
        Type::Unsigned { max: max.into() }
    }

    #[test]
    fn unsigned_pure_call_whitelist_is_transitive_and_rejects_other_primitives() {
        let leaf = PureCircuit {
            source: None,
            name: "product".into(),
            internal: false,
            parameters: vec![Parameter {
                name: "x".into(),
                ty: uint("65535"),
            }],
            result: uint("4294967295"),
            body: Expr::UnsignedCast {
                max: "4294967295".into(),
                value: Box::new(Expr::UnsignedMultiply {
                    max: "4294836225".into(),
                    left: Box::new(Expr::Parameter { name: "x".into() }),
                    right: Box::new(Expr::Parameter { name: "x".into() }),
                }),
            },
        };
        let wrapper = PureCircuit {
            source: None,
            name: "wrapper".into(),
            internal: false,
            parameters: vec![Parameter {
                name: "y".into(),
                ty: uint("65535"),
            }],
            result: uint("4294967295"),
            body: Expr::Call {
                name: "product".into(),
                arguments: vec![Expr::Parameter { name: "y".into() }],
            },
        };
        let mut circuits = HashMap::new();
        circuits.insert("product", &leaf);
        circuits.insert("wrapper", &wrapper);
        assert!(closed_pure_unsigned_call(
            "wrapper",
            &circuits,
            &mut HashSet::new()
        ));

        let effectful = PureCircuit {
            body: Expr::TransientHash {
                value: Box::new(Expr::Parameter { name: "x".into() }),
            },
            ..leaf.clone()
        };
        circuits.insert("product", &effectful);
        assert!(!closed_pure_unsigned_call(
            "wrapper",
            &circuits,
            &mut HashSet::new()
        ));

        let recursive = PureCircuit {
            body: Expr::Call {
                name: "wrapper".into(),
                arguments: vec![Expr::Parameter { name: "x".into() }],
            },
            ..leaf.clone()
        };
        circuits.insert("product", &recursive);
        assert!(!closed_pure_unsigned_call(
            "wrapper",
            &circuits,
            &mut HashSet::new()
        ));
    }
}
