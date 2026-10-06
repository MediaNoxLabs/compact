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

//! Closed guarded mutation profiles, separate from recording assembly.
//!
//! Each entry checks its entire supported source shape and emits ordered steps.
//! The shared authority continuation stays private; profile selection and
//! recording diagnostics remain owned by the parent module.

use std::collections::HashMap;

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, PureCircuit, StateAction, StateReturn, StatefulCircuit,
    Type, WitnessDeclaration,
};
use crate::{RenderError, expression_with_calls, ident, retained_value, rust_type};

pub(super) fn closed_organizer_gate_steps(
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

struct AuthorizedContinuation<'a> {
    actions: &'a [StateAction],
    steps: Vec<syn::Stmt>,
}

/// Shared closed authority prefix. The continuation receives the unconsumed
/// source actions only after witness/hash/Cell provenance has been checked.
fn closed_authority_prefix<'a>(
    circuit: &StatefulCircuit,
    source_actions: &'a [StateAction],
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
) -> Result<Option<AuthorizedContinuation<'a>>, RenderError> {
    let bytes32 = Type::Bytes { length: 32 };
    if circuit.result != Type::Unit || circuit.return_value != StateReturn::Unit {
        return Ok(None);
    }
    let [
        StateAction::Let {
            bindings: secret_bindings,
            action,
        },
    ] = source_actions
    else {
        return Ok(None);
    };
    let [secret] = secret_bindings.as_slice() else {
        return Ok(None);
    };
    let Expr::WitnessCall {
        name: witness_name,
        arguments,
    } = &secret.value
    else {
        return Ok(None);
    };
    let Some(witness) = witnesses.get(witness_name.as_str()) else {
        return Ok(None);
    };
    if secret.ty != bytes32
        || witness.result != bytes32
        || !arguments.is_empty()
        || !witness.parameters.is_empty()
    {
        return Ok(None);
    }
    let StateAction::Let {
        bindings: hash_bindings,
        action,
    } = action.as_ref()
    else {
        return Ok(None);
    };
    let [hash_binding] = hash_bindings.as_slice() else {
        return Ok(None);
    };
    let Expr::Call {
        name: hash_name,
        arguments,
    } = &hash_binding.value
    else {
        return Ok(None);
    };
    let [
        Expr::Coerce {
            value: hash_arg,
            ty: arg_ty,
        },
    ] = arguments.as_slice()
    else {
        return Ok(None);
    };
    if hash_binding.ty != bytes32
        || *arg_ty != bytes32
        || !matches!(hash_arg.as_ref(), Expr::Parameter { name } if name == &secret.name)
        || hash_binding.name == secret.name
        || circuit
            .parameters
            .iter()
            .any(|parameter| parameter.name == secret.name || parameter.name == hash_binding.name)
    {
        return Ok(None);
    }
    let Some(hash) = pure_circuits.get(hash_name.as_str()) else {
        return Ok(None);
    };
    let [hash_formal] = hash.parameters.as_slice() else {
        return Ok(None);
    };
    if hash_formal.ty != bytes32 || hash.result != bytes32 {
        return Ok(None);
    }
    let Expr::PersistentHash { value } = &hash.body else {
        return Ok(None);
    };
    let Expr::Tuple { elements } = value.as_ref() else {
        return Ok(None);
    };
    if !matches!(elements.as_slice(), [Expr::BytesLiteral { bytes }, Expr::Parameter { name }]
        if bytes.len() == 32 && name == &hash_formal.name)
    {
        return Ok(None);
    }
    let StateAction::Sequence { actions } = action.as_ref() else {
        return Ok(None);
    };

    let [
        StateAction::Assert {
            condition:
                Expr::Equal {
                    left: authority_value,
                    right: authority_read,
                },
            message: authority_message,
        },
        rest @ ..,
    ] = actions.as_slice()
    else {
        return Ok(None);
    };
    if !matches!(authority_value.as_ref(), Expr::Parameter { name } if name == &hash_binding.name) {
        return Ok(None);
    }
    let Expr::CellRead {
        field: authority_field,
        index: authority_index,
    } = authority_read.as_ref()
    else {
        return Ok(None);
    };
    if !matches!(ledger_fields.get(authority_field.as_str()), Some(declaration)
        if declaration.index == *authority_index && declaration.declaration == (LedgerFieldKind::Cell { ty: bytes32 }))
    {
        return Ok(None);
    }
    let witness_method = ident(witness_name)?;
    let hash_method = ident(hash_name)?;
    let authority_slot = ident(authority_field)?;
    let steps = vec![
        syn::parse_quote! {
            let (frame, __compact_authority_secret): (_, runtime::FixedBytes<32>) =
                frame.try_witness_metered(|context, meter| {
                    witnesses.#witness_method(context.witness_context_with(super::LedgerView {
                        state: context.query.state.get_ref(), meter,
                    }))
                })?;
        },
        syn::parse_quote! {
            let __compact_authority_hash = crate::pure_circuits::#hash_method(__compact_authority_secret)?;
        },
        syn::parse_quote! {
            let (frame, __compact_authority): (_, runtime::FixedBytes<32>) =
                crate::ledger_slots::#authority_slot.record_read(frame)?;
        },
        syn::parse_quote! {
            if __compact_authority_hash != __compact_authority {
                return Err(runtime::CompactError::AssertionFailed(#authority_message.to_owned()));
            }
        },
    ];
    Ok(Some(AuthorizedContinuation {
        actions: rest,
        steps,
    }))
}

/// A typed authority witness/hash and enum phase guard before a single optional
/// opaque Cell write. Every binding and ledger access is checked by provenance;
/// names carry no semantic meaning in this closed proof-supported shape.
pub(super) fn closed_authorized_optional_write_steps(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    let [parameter] = circuit.parameters.as_slice() else {
        return Ok(None);
    };
    if parameter.ty != Type::OpaqueString
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
    {
        return Ok(None);
    }
    let Some(AuthorizedContinuation { actions, mut steps }) = closed_authority_prefix(
        circuit,
        &circuit.actions,
        ledger_fields,
        witnesses,
        pure_circuits,
    )?
    else {
        return Ok(None);
    };
    let [
        StateAction::Assert {
            condition:
                Expr::Equal {
                    left: phase_read,
                    right: phase_value,
                },
            message: phase_message,
        },
        StateAction::Let {
            bindings: value_bindings,
            action: write,
        },
    ] = actions
    else {
        return Ok(None);
    };
    let Expr::CellRead {
        field: phase_field,
        index: phase_index,
    } = phase_read.as_ref()
    else {
        return Ok(None);
    };
    let Expr::EnumVariant { ty: phase_ty, .. } = phase_value.as_ref() else {
        return Ok(None);
    };
    if !matches!(phase_ty, Type::Enum { .. }) {
        return Ok(None);
    }
    let [value_binding] = value_bindings.as_slice() else {
        return Ok(None);
    };
    let Type::Struct {
        fields: members, ..
    } = &value_binding.ty
    else {
        return Ok(None);
    };
    let [present_member, value_member] = members.as_slice() else {
        return Ok(None);
    };
    if present_member.ty != Type::Boolean || value_member.ty != Type::OpaqueString {
        return Ok(None);
    }
    let Expr::StructLiteral {
        ty: literal_ty,
        fields: values,
    } = &value_binding.value
    else {
        return Ok(None);
    };
    if literal_ty != &value_binding.ty
        || !matches!(values.as_slice(), [Expr::Boolean { value: true }, Expr::Parameter { name }]
            if name == &parameter.name)
    {
        return Ok(None);
    }
    let StateAction::CellWrite {
        field: write_field,
        index: write_index,
        value: written,
    } = write.as_ref()
    else {
        return Ok(None);
    };
    if !matches!(written, Expr::Parameter { name } if name == &value_binding.name) {
        return Ok(None);
    }
    for (field, index, ty) in [
        (phase_field, phase_index, phase_ty),
        (write_field, write_index, &value_binding.ty),
    ] {
        if !matches!(ledger_fields.get(field.as_str()), Some(declaration)
            if declaration.index == *index && declaration.declaration == (LedgerFieldKind::Cell { ty: ty.clone() }))
        {
            return Ok(None);
        }
    }
    let phase_slot = ident(phase_field)?;
    let write_slot = ident(write_field)?;
    let phase_type = rust_type(phase_ty)?;
    let (phase_value, _) = expression_with_calls(phase_value, &HashMap::new(), &HashMap::new())?;
    let write_type = rust_type(&value_binding.ty)?;
    let present_member = ident(&present_member.name)?;
    let value_member = ident(&value_member.name)?;
    steps.extend([
        syn::parse_quote! {
            let (frame, __compact_phase): (_, #phase_type) = crate::ledger_slots::#phase_slot.record_read(frame)?;
        },
        syn::parse_quote! {
            if __compact_phase != #phase_value {
                return Err(runtime::CompactError::AssertionFailed(#phase_message.to_owned()));
            }
        },
        syn::parse_quote! {
            let __compact_optional_value: #write_type = #write_type {
                #present_member: true, #value_member: __compact_param_0.clone(),
            };
        },
        syn::parse_quote! {
            let frame = crate::ledger_slots::#write_slot.record_write(frame, __compact_optional_value)?;
        },
    ]);
    Ok(Some(steps))
}

/// Closed optional-presence gate followed by a pure enum decision tree. Calling
/// the existing pure emitter keeps enum variant semantics in one implementation.
pub(super) fn closed_authorized_enum_advance_steps(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    if !circuit.parameters.is_empty() {
        return Ok(None);
    }
    let Some(AuthorizedContinuation { actions, mut steps }) = closed_authority_prefix(
        circuit,
        &circuit.actions,
        ledger_fields,
        witnesses,
        pure_circuits,
    )?
    else {
        return Ok(None);
    };
    let [
        StateAction::Assert {
            condition:
                Expr::StructField {
                    value: optional_read,
                    field: member,
                    index: 0,
                },
            message,
        },
        StateAction::Let {
            bindings,
            action: write,
        },
    ] = actions
    else {
        return Ok(None);
    };
    let Expr::CellRead {
        field: optional_field,
        index: optional_index,
    } = optional_read.as_ref()
    else {
        return Ok(None);
    };
    let Some(optional) = ledger_fields.get(optional_field.as_str()) else {
        return Ok(None);
    };
    let LedgerFieldKind::Cell {
        ty: optional_ty @ Type::Struct { fields, .. },
    } = &optional.declaration
    else {
        return Ok(None);
    };
    let [presence, payload] = fields.as_slice() else {
        return Ok(None);
    };
    if optional.index != *optional_index
        || presence.name != *member
        || presence.ty != Type::Boolean
        || payload.ty != Type::OpaqueString
    {
        return Ok(None);
    }
    let [binding] = bindings.as_slice() else {
        return Ok(None);
    };
    if !matches!(&binding.ty, Type::Enum { .. }) {
        return Ok(None);
    }
    let Expr::Call { name, arguments } = &binding.value else {
        return Ok(None);
    };
    let [
        Expr::Coerce {
            value: phase_read,
            ty: argument_ty,
        },
    ] = arguments.as_slice()
    else {
        return Ok(None);
    };
    let Expr::CellRead {
        field: phase_field,
        index: phase_index,
    } = phase_read.as_ref()
    else {
        return Ok(None);
    };
    let StateAction::CellWrite {
        field: written_field,
        index: written_index,
        value: written,
    } = write.as_ref()
    else {
        return Ok(None);
    };
    if written_field != phase_field
        || written_index != phase_index
        || argument_ty != &binding.ty
        || !matches!(written, Expr::Parameter { name } if name == &binding.name)
        || !matches!(ledger_fields.get(phase_field.as_str()), Some(field) if field.index == *phase_index && field.declaration == (LedgerFieldKind::Cell { ty: binding.ty.clone() }))
    {
        return Ok(None);
    }
    let Some(successor) = pure_circuits.get(name.as_str()) else {
        return Ok(None);
    };
    let [formal] = successor.parameters.as_slice() else {
        return Ok(None);
    };
    if !successor.internal || successor.result != binding.ty || formal.ty != binding.ty {
        return Ok(None);
    }
    fn enum_tree(value: &Expr, formal: &str, ty: &Type) -> bool {
        match value {
            Expr::EnumVariant { ty: result, .. } => result == ty,
            Expr::Coerce { value, ty: result } => result == ty && enum_tree(value, formal, ty),
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                matches!(condition.as_ref(), Expr::Equal { left, right }
                    if matches!(left.as_ref(), Expr::Parameter { name } if name == formal)
                    && matches!(right.as_ref(), Expr::EnumVariant { ty: variant_ty, .. } if variant_ty == ty))
                    && enum_tree(then, formal, ty)
                    && enum_tree(otherwise, formal, ty)
            }
            _ => false,
        }
    }
    if !enum_tree(&successor.body, &formal.name, &formal.ty) {
        return Ok(None);
    }
    let optional_slot = ident(optional_field)?;
    let optional_type = rust_type(optional_ty)?;
    let presence_member = ident(member)?;
    let phase_slot = ident(phase_field)?;
    let phase_type = rust_type(&binding.ty)?;
    let successor_method = ident(name)?;
    steps.extend([
        syn::parse_quote! {
            let (frame, __compact_optional): (_, #optional_type) = crate::ledger_slots::#optional_slot.record_read(frame)?;
        },
        syn::parse_quote! {
            if !__compact_optional.#presence_member {
                return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
            }
        },
        syn::parse_quote! {
            let (frame, __compact_phase): (_, #phase_type) = crate::ledger_slots::#phase_slot.record_read(frame)?;
        },
        syn::parse_quote! {
            let __compact_successor: #phase_type = crate::pure_circuits::#successor_method(__compact_phase)?;
        },
        syn::parse_quote! {
            let frame = crate::ledger_slots::#phase_slot.record_write(frame, __compact_successor)?;
        },
    ]);
    Ok(Some(steps))
}

/// A negative optional-path witness guard followed by the shared authority
/// prefix, enum phase assertion and one typed Bytes32 Merkle insertion.
pub(super) fn closed_witness_admitted_merkle_insert_steps(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    let bytes32 = Type::Bytes { length: 32 };
    let [parameter] = circuit.parameters.as_slice() else {
        return Ok(None);
    };
    if parameter.ty != bytes32 {
        return Ok(None);
    }
    let [StateAction::Sequence { actions }] = circuit.actions.as_slice() else {
        return Ok(None);
    };
    let [
        StateAction::Assert {
            condition:
                Expr::If {
                    condition,
                    then,
                    otherwise,
                },
            message: admission_message,
        },
        authority,
    ] = actions.as_slice()
    else {
        return Ok(None);
    };
    if !matches!(then.as_ref(), Expr::Boolean { value: false })
        || !matches!(otherwise.as_ref(), Expr::Boolean { value: true })
    {
        return Ok(None);
    }
    let Expr::StructField {
        value,
        field: presence_member,
        index: 0,
    } = condition.as_ref()
    else {
        return Ok(None);
    };
    let Expr::WitnessCall {
        name: admission_name,
        arguments,
    } = value.as_ref()
    else {
        return Ok(None);
    };
    let [
        Expr::Coerce {
            value: argument,
            ty: argument_ty,
        },
    ] = arguments.as_slice()
    else {
        return Ok(None);
    };
    if *argument_ty != bytes32
        || !matches!(argument.as_ref(), Expr::Parameter { name } if name == &parameter.name)
    {
        return Ok(None);
    }
    let Some(admission) = witnesses.get(admission_name.as_str()) else {
        return Ok(None);
    };
    let [formal] = admission.parameters.as_slice() else {
        return Ok(None);
    };
    if formal.ty != bytes32 {
        return Ok(None);
    }
    let Type::Struct {
        fields: optional_fields,
        ..
    } = &admission.result
    else {
        return Ok(None);
    };
    let [presence, path] = optional_fields.as_slice() else {
        return Ok(None);
    };
    if presence.name != *presence_member || presence.ty != Type::Boolean {
        return Ok(None);
    }
    let Type::Struct {
        fields: path_fields,
        ..
    } = &path.ty
    else {
        return Ok(None);
    };
    let [leaf, entries] = path_fields.as_slice() else {
        return Ok(None);
    };
    if leaf.ty != bytes32 {
        return Ok(None);
    }
    let Type::Vector {
        element,
        length: depth,
    } = &entries.ty
    else {
        return Ok(None);
    };
    let Type::Struct {
        fields: entry_fields,
        ..
    } = element.as_ref()
    else {
        return Ok(None);
    };
    let [digest, direction] = entry_fields.as_slice() else {
        return Ok(None);
    };
    let Type::Struct {
        fields: digest_fields,
        ..
    } = &digest.ty
    else {
        return Ok(None);
    };
    if direction.ty != Type::Boolean
        || !matches!(digest_fields.as_slice(), [field] if field.ty == Type::Field)
    {
        return Ok(None);
    }
    let Some(AuthorizedContinuation {
        actions,
        steps: authority_steps,
    }) = closed_authority_prefix(
        circuit,
        std::slice::from_ref(authority),
        ledger_fields,
        witnesses,
        pure_circuits,
    )?
    else {
        return Ok(None);
    };
    let [
        StateAction::Assert {
            condition:
                Expr::Equal {
                    left: phase_read,
                    right: phase_value,
                },
            message: phase_message,
        },
        StateAction::MerkleInsert {
            field: tree_field,
            index: tree_index,
            value: inserted,
        },
    ] = actions
    else {
        return Ok(None);
    };
    let Expr::CellRead {
        field: phase_field,
        index: phase_index,
    } = phase_read.as_ref()
    else {
        return Ok(None);
    };
    let Expr::EnumVariant { ty: phase_ty, .. } = phase_value.as_ref() else {
        return Ok(None);
    };
    if !matches!(phase_ty, Type::Enum { .. })
        || !matches!(inserted, Expr::Parameter { name } if name == &parameter.name)
    {
        return Ok(None);
    }
    if !matches!(ledger_fields.get(phase_field.as_str()), Some(field) if field.index == *phase_index && field.declaration == (LedgerFieldKind::Cell { ty: phase_ty.clone() }))
    {
        return Ok(None);
    }
    let Some(tree) = ledger_fields.get(tree_field.as_str()) else {
        return Ok(None);
    };
    if tree.index != *tree_index
        || tree.physical_path().len() != 1
        || !matches!(&tree.declaration, LedgerFieldKind::MerkleTree { ty, depth: tree_depth } if *ty == bytes32 && usize::from(*tree_depth) == *depth)
    {
        return Ok(None);
    }
    let admission_method = ident(admission_name)?;
    let admission_ty = rust_type(&admission.result)?;
    let presence_member = ident(presence_member)?;
    let phase_slot = ident(phase_field)?;
    let phase_type = rust_type(phase_ty)?;
    let (phase_value, _) = expression_with_calls(phase_value, &HashMap::new(), &HashMap::new())?;
    let tree_slot = ident(tree_field)?;
    let mut steps = vec![
        syn::parse_quote! {
            let (frame, __compact_admission): (_, #admission_ty) = frame.try_witness_metered(|context, meter| {
                witnesses.#admission_method(context.witness_context_with(super::LedgerView {
                    state: context.query.state.get_ref(), meter,
                }), __compact_param_0)
            })?;
        },
        syn::parse_quote! {
            if __compact_admission.#presence_member {
                return Err(runtime::CompactError::AssertionFailed(#admission_message.to_owned()));
            }
        },
    ];
    steps.extend(authority_steps);
    steps.extend([
        syn::parse_quote! {
            let (frame, __compact_phase): (_, #phase_type) = crate::ledger_slots::#phase_slot.record_read(frame)?;
        },
        syn::parse_quote! {
            if __compact_phase != #phase_value {
                return Err(runtime::CompactError::AssertionFailed(#phase_message.to_owned()));
            }
        },
        syn::parse_quote! {
            let frame = crate::ledger_slots::#tree_slot.record_insert(frame, __compact_param_0)?;
        },
    ]);
    Ok(Some(steps))
}
