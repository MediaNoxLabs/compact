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

//! The existing closed OpaqueString map-write recording profile.
//! Keep exact enum/Map/Set/Counter shape admission visible; this module is not a
//! general map-write lowering API. Its local recognizers retain lexical ownership.

use crate::ir::{
    CounterAmount, Expr, LedgerField, LedgerFieldKind, LocalBinding, PureCircuit, StateAction,
    StateReturn, StatefulCircuit, Type,
};
use crate::recorded::helpers::helper_ident;
use crate::stateful::circuit_uses_witness;
use crate::{RenderError, ident, rust_type};
use std::collections::{HashMap, HashSet};

// Bind an opaque key and a concrete struct value once, then record the
// complete Insert/Update transition through the declared Map slot. The
// field and circuit names are data; the typed provenance, helper bodies,
// and ordered branch structure decide admission.
pub(in crate::recorded) fn closed_typed_opaque_map_write_steps(
    circuit: &StatefulCircuit,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    fn enum_is(expr: &Expr, bound: &str, ty: &Type, variant: &str) -> bool {
        matches!(expr, Expr::Equal { left, right }
            if matches!(left.as_ref(), Expr::Parameter { name } if name == bound)
                && matches!(right.as_ref(), Expr::EnumVariant { ty: variant_ty, variant: selected }
                    if variant_ty == ty && selected == variant))
    }
    fn map_member<'a>(
        expr: &'a Expr,
        key_name: &str,
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
            && matches!(key.as_ref(), Expr::Parameter { name } if name == key_name))
        .then_some((field.as_str(), *index))
    }
    let [key_parameter, value_parameter, mutation_parameter] = circuit.parameters.as_slice() else {
        return Ok(None);
    };
    let Type::Struct { .. } = &value_parameter.ty else {
        return Ok(None);
    };
    let Type::Enum { variants, .. } = &mutation_parameter.ty else {
        return Ok(None);
    };
    if key_parameter.ty != Type::OpaqueString
        || variants.as_slice() != ["Unspecified", "Insert", "Update"]
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
    {
        return Ok(None);
    }
    let [
        StateAction::Let {
            bindings: key_bindings,
            action: value_action,
        },
    ] = circuit.actions.as_slice()
    else {
        return Ok(None);
    };
    let [key_binding] = key_bindings.as_slice() else {
        return Ok(None);
    };
    let StateAction::Let {
        bindings: value_bindings,
        action: mutation_action,
    } = value_action.as_ref()
    else {
        return Ok(None);
    };
    let [value_binding] = value_bindings.as_slice() else {
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
        || value_binding.ty != value_parameter.ty
        || mutation_binding.ty != mutation_parameter.ty
        || !matches!(&key_binding.value, Expr::Parameter { name } if name == &key_parameter.name)
        || !matches!(&value_binding.value, Expr::Parameter { name } if name == &value_parameter.name)
        || !matches!(&mutation_binding.value, Expr::Parameter { name } if name == &mutation_parameter.name)
    {
        return Ok(None);
    }
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
        remaining @ ..,
    ] = actions.as_slice()
    else {
        return Ok(None);
    };
    let (class_guard, write_actions) = match remaining {
        [StateAction::PureCall { .. }, _, _, _] => (Some(&remaining[0]), &remaining[1..]),
        [_, _, _] => (None, remaining),
        _ => return Ok(None),
    };
    let [
        StateAction::If {
            condition: update_condition,
            then: update,
            otherwise: insert_branch,
        },
        StateAction::MapInsert {
            field,
            index,
            key: inserted_key,
            value: inserted_value,
        },
        StateAction::CircuitCall {
            name: write,
            arguments: write_args,
        },
    ] = write_actions
    else {
        return Ok(None);
    };
    let class_assert = if let Some(StateAction::PureCall { name, arguments }) = class_guard {
        let [Expr::Coerce { value, ty }] = arguments.as_slice() else {
            return Ok(None);
        };
        if ty != &value_binding.ty
            || !matches!(value.as_ref(), Expr::Parameter { name } if name == &value_binding.name)
        {
            return Ok(None);
        }
        let Some(callee) = pure_circuits.get(name.as_str()) else {
            return Ok(None);
        };
        let [formal] = callee.parameters.as_slice() else {
            return Ok(None);
        };
        let Expr::Sequence { steps, value } = &callee.body else {
            return Ok(None);
        };
        let [Expr::Assert { condition, .. }] = steps.as_slice() else {
            return Ok(None);
        };
        let Expr::NotEqual { left, right } = condition.as_ref() else {
            return Ok(None);
        };
        let Expr::StructField {
            value: selected,
            field: selected_field,
            index: selected_index,
        } = left.as_ref()
        else {
            return Ok(None);
        };
        let Expr::EnumVariant {
            ty: variant_type,
            variant,
        } = right.as_ref()
        else {
            return Ok(None);
        };
        let Type::Struct { fields, .. } = &value_binding.ty else {
            return Ok(None);
        };
        if callee.result != Type::Unit
            || formal.ty != value_binding.ty
            || !matches!(value.as_ref(), Expr::Unit)
            || !matches!(selected.as_ref(), Expr::Parameter { name } if name == &formal.name)
            || !matches!(fields.get(*selected_index), Some(member)
                if member.name == *selected_field && member.ty == *variant_type)
            || !matches!(variant_type, Type::Enum { variants, .. }
                if variants.first().is_some_and(|first| first == variant))
        {
            return Ok(None);
        }
        Some(ident(name)?)
    } else {
        None
    };
    let key_is_bound =
        |expr: &Expr| matches!(expr, Expr::Parameter { name } if name == &key_binding.name);
    if !writable_args.is_empty()
        || !write_args.is_empty()
        || !shared_callees.contains(writable)
        || !shared_callees.contains(write)
        || !key_is_bound(inserted_key)
        || !matches!(inserted_value, Expr::Parameter { name } if name == &value_binding.name)
        || !enum_is(
            update_condition,
            &mutation_binding.name,
            &mutation_binding.ty,
            "Update",
        )
    {
        return Ok(None);
    }
    let Expr::If {
        condition: insert_condition,
        then,
        otherwise,
    } = mutation_guard
    else {
        return Ok(None);
    };
    if !enum_is(
        insert_condition,
        &mutation_binding.name,
        &mutation_binding.ty,
        "Insert",
    ) || !matches!(then.as_ref(), Expr::Boolean { value: true })
        || !enum_is(
            otherwise,
            &mutation_binding.name,
            &mutation_binding.ty,
            "Update",
        )
    {
        return Ok(None);
    }
    let Some(map_slot) = ledger_fields.get(field.as_str()) else {
        return Ok(None);
    };
    if map_slot.index != *index
        || map_slot.declaration
            != (LedgerFieldKind::Map {
                key: Type::OpaqueString,
                value: value_binding.ty.clone(),
            })
    {
        return Ok(None);
    }
    let StateAction::Sequence {
        actions: update_actions,
    } = update.as_ref()
    else {
        return Ok(None);
    };
    let [
        StateAction::Assert {
            condition: member,
            message: missing_message,
        },
        StateAction::MapRemove {
            field: removed_field,
            index: removed_index,
            key: removed_key,
        },
    ] = update_actions.as_slice()
    else {
        return Ok(None);
    };
    if map_member(member, &key_binding.name, ledger_fields) != Some((field.as_str(), *index))
        || removed_field != field
        || removed_index != index
        || !key_is_bound(removed_key)
    {
        return Ok(None);
    }
    let StateAction::If {
        condition: insert_condition,
        then: uniqueness,
        otherwise: no_op,
    } = insert_branch.as_ref()
    else {
        return Ok(None);
    };
    if !enum_is(
        insert_condition,
        &mutation_binding.name,
        &mutation_binding.ty,
        "Insert",
    ) || !matches!(no_op.as_ref(), StateAction::Sequence { actions } if actions.is_empty())
    {
        return Ok(None);
    }
    let (uniqueness, increment) = match uniqueness.as_ref() {
        StateAction::Assert { .. } => (uniqueness.as_ref(), None),
        StateAction::Sequence { actions } => {
            let [
                guard @ StateAction::Assert { .. },
                count @ StateAction::Let { .. },
            ] = actions.as_slice()
            else {
                return Ok(None);
            };
            (guard, Some(count))
        }
        _ => return Ok(None),
    };
    if class_assert.is_some() != increment.is_some() {
        return Ok(None);
    }
    let StateAction::Assert {
        condition: absent_guard,
        message: duplicate_message,
    } = uniqueness
    else {
        return Ok(None);
    };
    let Expr::If {
        condition: exists_call,
        then: if_exists,
        otherwise: if_absent,
    } = absent_guard
    else {
        return Ok(None);
    };
    let Expr::Call {
        name: exists_name,
        arguments: exists_args,
    } = exists_call.as_ref()
    else {
        return Ok(None);
    };
    if !matches!(if_exists.as_ref(), Expr::Boolean { value: false })
        || !matches!(if_absent.as_ref(), Expr::Boolean { value: true })
        || !matches!(exists_args.as_slice(), [Expr::Coerce { value, ty }]
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
        then: first_found,
        otherwise: second_member,
    } = exists_value
    else {
        return Ok(None);
    };
    if exists_parameter.ty != Type::OpaqueString
        || exists.result != Type::Boolean
        || !exists.actions.is_empty()
        || !matches!(first_found.as_ref(), Expr::Boolean { value: true })
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
    if (first_field, first_index) == (second_field, second_index)
        || (first_field, first_index) != (field.as_str(), *index)
            && (second_field, second_index) != (field.as_str(), *index)
    {
        return Ok(None);
    }
    let Some(writable_callee) = circuits.get(writable.as_str()) else {
        return Ok(None);
    };
    let Some(write_callee) = circuits.get(write.as_str()) else {
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
    let (_, key_source) = parameters
        .get(key_parameter.name.as_str())
        .expect("checked key");
    let (_, value_source) = parameters
        .get(value_parameter.name.as_str())
        .expect("checked value");
    let (_, mutation_source) = parameters
        .get(mutation_parameter.name.as_str())
        .expect("checked enum");
    let map = ident(field)?;
    let first_map = ident(first_field)?;
    let second_map = ident(second_field)?;
    let insert_increment: syn::Expr = if let Some(StateAction::Let { bindings, action }) = increment
    {
        let [
            LocalBinding {
                name: amount_name,
                ty: Type::Unsigned { max },
                value:
                    Expr::UnsignedLiteral {
                        value,
                        max: literal_max,
                    },
            },
        ] = bindings.as_slice()
        else {
            return Ok(None);
        };
        let StateAction::CounterIncrement {
            field: counter_field,
            index: counter_index,
            amount: CounterAmount::Parameter { name },
        } = action.as_ref()
        else {
            return Ok(None);
        };
        let Some(counter_slot) = ledger_fields.get(counter_field.as_str()) else {
            return Ok(None);
        };
        if max != "65535"
            || literal_max != max
            || value != "1"
            || name != amount_name
            || counter_slot.index != *counter_index
            || counter_slot.declaration != LedgerFieldKind::Counter
        {
            return Ok(None);
        }
        let counter = ident(counter_field)?;
        syn::parse_quote!(crate::ledger_slots::#counter.record_increment(frame, 1_u16)?)
    } else {
        syn::parse_quote!(frame)
    };
    let value_ty = rust_type(&value_binding.ty)?;
    let mutation_ty = rust_type(&mutation_binding.ty)?;
    let writable_helper = helper_ident(writable, circuits)?;
    let write_helper = helper_ident(write, circuits)?;
    let mut steps = vec![
        syn::parse_quote!(let __compact_recorded_write_key: runtime::OpaqueString = (#key_source).clone();),
        syn::parse_quote!(let __compact_recorded_write_value: #value_ty = (#value_source).clone();),
        syn::parse_quote!(let __compact_recorded_write_mutation: #mutation_ty = #mutation_source;),
    ];
    if circuit_uses_witness(writable_callee, circuits, &mut HashSet::new())? {
        steps.push(syn::parse_quote!(let (frame, _) = #writable_helper(frame, witnesses)?;));
    } else {
        steps.push(syn::parse_quote!(let (frame, _) = #writable_helper(frame)?;));
    }
    steps.extend([syn::parse_quote!(
        if !(__compact_recorded_write_mutation == #mutation_ty::Insert
            || __compact_recorded_write_mutation == #mutation_ty::Update) {
            return Err(runtime::CompactError::AssertionFailed(#mutation_message.to_owned()));
        }
    )]);
    if let Some(class_assert) = class_assert {
        steps.push(syn::parse_quote!(crate::pure_circuits::#class_assert(__compact_recorded_write_value.clone())?;));
    }
    steps.extend([
        syn::parse_quote!(
            let frame = if __compact_recorded_write_mutation == #mutation_ty::Update {
                let (frame, __compact_recorded_present): (_, bool) =
                    crate::ledger_slots::#map.record_member(frame, __compact_recorded_write_key.clone())?;
                if !__compact_recorded_present {
                    return Err(runtime::CompactError::AssertionFailed(#missing_message.to_owned()));
                }
                crate::ledger_slots::#map.record_remove(frame, __compact_recorded_write_key.clone())?
            } else {
                let (frame, __compact_recorded_first_present): (_, bool) =
                    crate::ledger_slots::#first_map.record_member(frame, __compact_recorded_write_key.clone())?;
                if __compact_recorded_first_present {
                    return Err(runtime::CompactError::AssertionFailed(#duplicate_message.to_owned()));
                }
                let (frame, __compact_recorded_target_present): (_, bool) =
                    crate::ledger_slots::#second_map.record_member(frame, __compact_recorded_write_key.clone())?;
                if __compact_recorded_target_present {
                    return Err(runtime::CompactError::AssertionFailed(#duplicate_message.to_owned()));
                }
                #insert_increment
            };
        ),
        syn::parse_quote!(
            let frame = crate::ledger_slots::#map.record_insert(
                frame, __compact_recorded_write_key, __compact_recorded_write_value
            )?;
        ),
    ]);
    if circuit_uses_witness(write_callee, circuits, &mut HashSet::new())? {
        steps.push(syn::parse_quote!(let (frame, _) = #write_helper(frame, witnesses)?;));
    } else {
        steps.push(syn::parse_quote!(let (frame, _) = #write_helper(frame)?;));
    }
    Ok(Some(steps))
}
