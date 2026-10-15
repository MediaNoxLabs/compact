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

//! The existing closed guarded OpaqueString set-mutation recording profile.
//! Exact guard and continuation checks remain part of this supported profile.
//! Selection order and fallback diagnostics belong to the recording orchestrator.

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, StateAction, StateReturn, StatefulCircuit, Type,
};
use crate::recorded::helpers::helper_ident;
use crate::stateful::circuit_uses_witness;
use crate::{RenderError, ident, rust_type};
use std::collections::{HashMap, HashSet};

// Record the closed Add/Drop Set mutation only when both the cross-Map
// existence guard and opposite Set membership guards retain their typed
// source order. This shares ledger slots and helper planning with the
// asset Map writes above, without depending on the exported circuit name.
pub(in crate::recorded) fn closed_guarded_opaque_set_mutation_steps(
    circuit: &StatefulCircuit,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    fn enum_is(expr: &Expr, name: &str, ty: &Type, variant: &str) -> bool {
        matches!(expr, Expr::Equal { left, right }
            if matches!(left.as_ref(), Expr::Parameter { name: selected } if selected == name)
                && matches!(right.as_ref(), Expr::EnumVariant { ty: selected_ty, variant: selected }
                    if selected_ty == ty && selected == variant))
    }
    fn map_member<'a>(
        expr: &'a Expr,
        parameter: &str,
        ledger_fields: &HashMap<&str, &LedgerField>,
    ) -> Option<(&'a str, u8)> {
        let Expr::MapMember { field, index, key } = expr else {
            return None;
        };
        let slot = ledger_fields.get(field.as_str())?;
        (slot.index == *index
            && matches!(
                &slot.declaration,
                LedgerFieldKind::Map {
                    key: Type::OpaqueString,
                    ..
                }
            )
            && matches!(key.as_ref(), Expr::Parameter { name } if name == parameter))
        .then_some((field.as_str(), *index))
    }
    let [key_parameter, mutation_parameter] = circuit.parameters.as_slice() else {
        return Ok(None);
    };
    let Type::Enum { variants, .. } = &mutation_parameter.ty else {
        return Ok(None);
    };
    if key_parameter.ty != Type::OpaqueString
        || variants.as_slice() != ["Unspecified", "Add", "Drop"]
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
    {
        return Ok(None);
    }
    let [
        StateAction::Let {
            bindings: key_bindings,
            action: mutation_action,
        },
    ] = circuit.actions.as_slice()
    else {
        return Ok(None);
    };
    let [key_binding] = key_bindings.as_slice() else {
        return Ok(None);
    };
    let StateAction::Let {
        bindings: mutation_bindings,
        action: body,
    } = mutation_action.as_ref()
    else {
        return Ok(None);
    };
    let [mutation_binding] = mutation_bindings.as_slice() else {
        return Ok(None);
    };
    if key_binding.ty != key_parameter.ty
        || mutation_binding.ty != mutation_parameter.ty
        || !matches!(&key_binding.value, Expr::Parameter { name } if name == &key_parameter.name)
        || !matches!(&mutation_binding.value, Expr::Parameter { name } if name == &mutation_parameter.name)
    {
        return Ok(None);
    }
    let key_is_bound =
        |expr: &Expr| matches!(expr, Expr::Parameter { name } if name == &key_binding.name);
    let StateAction::Sequence { actions } = body.as_ref() else {
        return Ok(None);
    };
    let [
        StateAction::CircuitCall {
            name: writable,
            arguments: writable_args,
        },
        StateAction::Assert {
            condition: mutation_guard,
            message: mutation_message,
        },
        StateAction::Assert {
            condition: exists_guard,
            message: missing_record_message,
        },
        StateAction::If {
            condition: add_condition,
            then: add,
            otherwise: drop_branch,
        },
        StateAction::CircuitCall {
            name: write,
            arguments: write_args,
        },
    ] = actions.as_slice()
    else {
        return Ok(None);
    };
    let Expr::If {
        condition: add_guard,
        then: allowed,
        otherwise: drop_guard,
    } = mutation_guard
    else {
        return Ok(None);
    };
    if !writable_args.is_empty()
        || !write_args.is_empty()
        || !shared_callees.contains(writable)
        || !shared_callees.contains(write)
        || !enum_is(
            add_guard,
            &mutation_binding.name,
            &mutation_binding.ty,
            "Add",
        )
        || !matches!(allowed.as_ref(), Expr::Boolean { value: true })
        || !enum_is(
            drop_guard,
            &mutation_binding.name,
            &mutation_binding.ty,
            "Drop",
        )
        || !enum_is(
            add_condition,
            &mutation_binding.name,
            &mutation_binding.ty,
            "Add",
        )
    {
        return Ok(None);
    }
    let Expr::Call {
        name: exists_name,
        arguments: exists_args,
    } = exists_guard
    else {
        return Ok(None);
    };
    if !matches!(exists_args.as_slice(), [Expr::Coerce { value, ty }]
        if ty == &Type::OpaqueString && key_is_bound(value))
    {
        return Ok(None);
    }
    let Some(exists) = circuits.get(exists_name.as_str()) else {
        return Ok(None);
    };
    let [exists_parameter] = exists.parameters.as_slice() else {
        return Ok(None);
    };
    let StateReturn::Expression {
        value: exists_value,
    } = &exists.return_value
    else {
        return Ok(None);
    };
    let Expr::If {
        condition: first_member,
        then: found,
        otherwise: second_member,
    } = exists_value
    else {
        return Ok(None);
    };
    if exists_parameter.ty != Type::OpaqueString
        || exists.result != Type::Boolean
        || !exists.actions.is_empty()
        || !matches!(found.as_ref(), Expr::Boolean { value: true })
    {
        return Ok(None);
    }
    let Some((first_field, first_index)) =
        map_member(first_member, &exists_parameter.name, ledger_fields)
    else {
        return Ok(None);
    };
    let Some((second_field, second_index)) =
        map_member(second_member, &exists_parameter.name, ledger_fields)
    else {
        return Ok(None);
    };
    if (first_field, first_index) == (second_field, second_index) {
        return Ok(None);
    }
    let StateAction::Sequence {
        actions: add_actions,
    } = add.as_ref()
    else {
        return Ok(None);
    };
    let [
        StateAction::Assert {
            condition: absent_guard,
            message: duplicate_message,
        },
        StateAction::SetInsert {
            field,
            index,
            value: inserted,
        },
    ] = add_actions.as_slice()
    else {
        return Ok(None);
    };
    let Expr::If {
        condition: member,
        then: present,
        otherwise: absent,
    } = absent_guard
    else {
        return Ok(None);
    };
    if !matches!(present.as_ref(), Expr::Boolean { value: false })
        || !matches!(absent.as_ref(), Expr::Boolean { value: true })
        || !key_is_bound(inserted)
        || !matches!(member.as_ref(), Expr::SetMember { field: member_field, index: member_index, value }
            if member_field == field && member_index == index && key_is_bound(value))
    {
        return Ok(None);
    }
    let StateAction::If {
        condition: drop_condition,
        then: drop,
        otherwise: no_op,
    } = drop_branch.as_ref()
    else {
        return Ok(None);
    };
    if !enum_is(
        drop_condition,
        &mutation_binding.name,
        &mutation_binding.ty,
        "Drop",
    ) || !matches!(no_op.as_ref(), StateAction::Sequence { actions } if actions.is_empty())
    {
        return Ok(None);
    }
    let StateAction::Sequence {
        actions: drop_actions,
    } = drop.as_ref()
    else {
        return Ok(None);
    };
    let [
        StateAction::Assert {
            condition: present_guard,
            message: absent_message,
        },
        StateAction::SetRemove {
            field: removed_field,
            index: removed_index,
            value: removed,
        },
    ] = drop_actions.as_slice()
    else {
        return Ok(None);
    };
    let Some(slot) = ledger_fields.get(field.as_str()) else {
        return Ok(None);
    };
    if slot.index != *index
        || slot.declaration
            != (LedgerFieldKind::Set {
                ty: Type::OpaqueString,
            })
        || removed_field != field
        || removed_index != index
        || !key_is_bound(removed)
        || !matches!(present_guard, Expr::SetMember { field: member_field, index: member_index, value }
            if member_field == field && member_index == index && key_is_bound(value))
    {
        return Ok(None);
    }
    let (Some(writable_callee), Some(write_callee)) = (
        circuits.get(writable.as_str()),
        circuits.get(write.as_str()),
    ) else {
        return Ok(None);
    };
    if writable_callee.result != Type::Unit
        || write_callee.result != Type::Unit
        || writable_callee.return_value != StateReturn::Unit
        || write_callee.return_value != StateReturn::Unit
        || !writable_callee.parameters.is_empty()
        || !write_callee.parameters.is_empty()
    {
        return Ok(None);
    }
    let first = ident(first_field)?;
    let second = ident(second_field)?;
    let set = ident(field)?;
    let mutation_ty = rust_type(&mutation_binding.ty)?;
    let writable_helper = helper_ident(writable, circuits)?;
    let write_helper = helper_ident(write, circuits)?;
    let (_, key_source) = parameters
        .get(key_parameter.name.as_str())
        .expect("checked key");
    let (_, mutation_source) = parameters
        .get(mutation_parameter.name.as_str())
        .expect("checked mutation");
    let mut steps = vec![
        syn::parse_quote!(let __compact_recorded_watch_key: runtime::OpaqueString = (#key_source).clone();),
        syn::parse_quote!(let __compact_recorded_watch_mutation: #mutation_ty = #mutation_source;),
    ];
    if circuit_uses_witness(writable_callee, circuits, &mut HashSet::new())? {
        steps.push(syn::parse_quote!(let (frame, _) = #writable_helper(frame, witnesses)?;));
    } else {
        steps.push(syn::parse_quote!(let (frame, _) = #writable_helper(frame)?;));
    }
    steps.extend([
        syn::parse_quote!(
            if !(__compact_recorded_watch_mutation == #mutation_ty::Add
                || __compact_recorded_watch_mutation == #mutation_ty::Drop) {
                return Err(runtime::CompactError::AssertionFailed(#mutation_message.to_owned()));
            }
        ),
        syn::parse_quote!(
            let (frame, __compact_recorded_first_exists): (_, bool) =
                crate::ledger_slots::#first.record_member(frame, __compact_recorded_watch_key.clone())?;
        ),
        syn::parse_quote!(
            let (frame, __compact_recorded_exists): (_, bool) = if __compact_recorded_first_exists {
                (frame, true)
            } else {
                crate::ledger_slots::#second.record_member(frame, __compact_recorded_watch_key.clone())?
            };
        ),
        syn::parse_quote!(
            if !__compact_recorded_exists {
                return Err(runtime::CompactError::AssertionFailed(#missing_record_message.to_owned()));
            }
        ),
        syn::parse_quote!(
            let frame = if __compact_recorded_watch_mutation == #mutation_ty::Add {
                let (frame, __compact_recorded_watched): (_, bool) =
                    crate::ledger_slots::#set.record_member(frame, __compact_recorded_watch_key.clone())?;
                if __compact_recorded_watched {
                    return Err(runtime::CompactError::AssertionFailed(#duplicate_message.to_owned()));
                }
                crate::ledger_slots::#set.record_insert(frame, __compact_recorded_watch_key)?
            } else {
                let (frame, __compact_recorded_watched): (_, bool) =
                    crate::ledger_slots::#set.record_member(frame, __compact_recorded_watch_key.clone())?;
                if !__compact_recorded_watched {
                    return Err(runtime::CompactError::AssertionFailed(#absent_message.to_owned()));
                }
                crate::ledger_slots::#set.record_remove(frame, __compact_recorded_watch_key)?
            };
        ),
    ]);
    if circuit_uses_witness(write_callee, circuits, &mut HashSet::new())? {
        steps.push(syn::parse_quote!(let (frame, _) = #write_helper(frame, witnesses)?;));
    } else {
        steps.push(syn::parse_quote!(let (frame, _) = #write_helper(frame)?;));
    }
    Ok(Some(steps))
}
