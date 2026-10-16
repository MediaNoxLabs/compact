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

//! Closed guarded struct-map read recognition and pure guard inspection.
//! Admission, declaration checks and emitted statement order are retained from
//! the legacy renderer. Profile selection remains with the recording orchestrator.

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, PureCircuit, StateAction, StateReturn, StatefulCircuit,
    Type,
};
use crate::{RenderError, ident, rust_type};
use std::collections::HashMap;

// Type-check the complete pure guard against a closed scalar expression
// language. This rejects effects and calls even when they occur in a
// branch that a particular fixture does not exercise.
pub(in crate::recorded) fn guarded_map_pure_type(
    expr: &Expr,
    scope: &HashMap<String, Type>,
) -> Option<Type> {
    match expr {
        Expr::Unit => Some(Type::Unit),
        Expr::Boolean { .. } => Some(Type::Boolean),
        Expr::UnsignedLiteral { max, .. } => Some(Type::Unsigned { max: max.clone() }),
        Expr::Parameter { name } => scope.get(name).cloned(),
        Expr::StructField {
            value,
            field,
            index,
        } => {
            let Type::Struct { fields, .. } = guarded_map_pure_type(value, scope)? else {
                return None;
            };
            fields
                .get(*index)
                .filter(|part| part.name == *field)
                .map(|part| part.ty.clone())
        }
        Expr::Let { bindings, body } => {
            let mut local = scope.clone();
            for binding in bindings {
                if local.contains_key(&binding.name)
                    || guarded_map_pure_type(&binding.value, &local)? != binding.ty
                {
                    return None;
                }
                local.insert(binding.name.clone(), binding.ty.clone());
            }
            guarded_map_pure_type(body, &local)
        }
        Expr::Compare { left, right, .. } => {
            let left = guarded_map_pure_type(left, scope)?;
            (matches!(left, Type::Unsigned { .. }) && guarded_map_pure_type(right, scope)? == left)
                .then_some(Type::Boolean)
        }
        Expr::UnsignedSubtract { max, left, right } => {
            let ty = Type::Unsigned { max: max.clone() };
            (guarded_map_pure_type(left, scope)? == ty
                && guarded_map_pure_type(right, scope)? == ty)
                .then_some(ty)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            if guarded_map_pure_type(condition, scope)? != Type::Boolean {
                return None;
            }
            let then_ty = guarded_map_pure_type(then, scope)?;
            (guarded_map_pure_type(otherwise, scope)? == then_ty).then_some(then_ty)
        }
        Expr::Assert { condition, .. } => {
            (guarded_map_pure_type(condition, scope)? == Type::Boolean).then_some(Type::Unit)
        }
        Expr::Sequence { steps, value } => {
            for step in steps {
                if guarded_map_pure_type(step, scope)? != Type::Unit {
                    return None;
                }
            }
            guarded_map_pure_type(value, scope)
        }
        Expr::Coerce { value, ty } => {
            (guarded_map_pure_type(value, scope)? == *ty).then(|| ty.clone())
        }
        _ => None,
    }
}

pub(in crate::recorded) fn guarded_map_has_assert(expr: &Expr) -> bool {
    match expr {
        Expr::Assert { .. } => true,
        Expr::Sequence { steps, value } => {
            steps.iter().any(guarded_map_has_assert) || guarded_map_has_assert(value)
        }
        Expr::If {
            then, otherwise, ..
        } => guarded_map_has_assert(then) || guarded_map_has_assert(otherwise),
        Expr::Let { body, .. } => guarded_map_has_assert(body),
        _ => false,
    }
}

// This pattern is keyed by typed provenance and operation order rather
// than contract, Map, struct, or pure-circuit names.
pub(in crate::recorded) fn closed_guarded_struct_map_read_steps(
    circuit: &StatefulCircuit,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
    if circuit.result != Type::Unit || circuit.return_value != StateReturn::Unit {
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
    let Expr::Parameter {
        name: key_source_name,
    } = &key_binding.value
    else {
        return Ok(None);
    };
    if key_binding.ty != Type::OpaqueString
        || !matches!(
            parameters.get(key_source_name.as_str()),
            Some((Type::OpaqueString, _))
        )
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
                    field,
                    index,
                    key: member_key,
                },
            message,
        },
        StateAction::Let {
            bindings: value_bindings,
            action: pure_action,
        },
    ] = actions.as_slice()
    else {
        return Ok(None);
    };
    let [value_binding] = value_bindings.as_slice() else {
        return Ok(None);
    };
    let Expr::MapLookup {
        field: lookup_field,
        index: lookup_index,
        key: lookup_key,
    } = &value_binding.value
    else {
        return Ok(None);
    };
    let StateAction::PureCall {
        name: pure_name,
        arguments,
    } = pure_action.as_ref()
    else {
        return Ok(None);
    };
    if lookup_field != field
        || lookup_index != index
        || !matches!(member_key.as_ref(), Expr::Parameter { name } if name == &key_binding.name)
        || !matches!(lookup_key.as_ref(), Expr::Parameter { name } if name == &key_binding.name)
        || !matches!(value_binding.ty, Type::Struct { .. })
    {
        return Ok(None);
    }
    let Some(slot) = ledger_fields.get(field.as_str()) else {
        return Ok(None);
    };
    if slot.index != *index
        || slot.declaration
            != (LedgerFieldKind::Map {
                key: Type::OpaqueString,
                value: value_binding.ty.clone(),
            })
    {
        return Ok(None);
    }
    let Some(pure) = pure_circuits.get(pure_name.as_str()) else {
        return Ok(None);
    };
    if pure.result != Type::Unit
        || pure.parameters.len() != arguments.len()
        || !guarded_map_has_assert(&pure.body)
    {
        return Ok(None);
    }
    let scope: HashMap<String, Type> = pure
        .parameters
        .iter()
        .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
        .collect();
    if guarded_map_pure_type(&pure.body, &scope) != Some(Type::Unit) {
        return Ok(None);
    }
    let mut argument_sources = Vec::new();
    let mut looked_up_count = 0;
    for (argument, pure_parameter) in arguments.iter().zip(&pure.parameters) {
        let Expr::Coerce { value, ty } = argument else {
            return Ok(None);
        };
        if ty != &pure_parameter.ty {
            return Ok(None);
        }
        let Expr::Parameter { name } = value.as_ref() else {
            return Ok(None);
        };
        if name == &value_binding.name {
            if ty != &value_binding.ty {
                return Ok(None);
            }
            looked_up_count += 1;
            argument_sources.push(None);
        } else {
            let Some((source_ty, source)) = parameters.get(name.as_str()) else {
                return Ok(None);
            };
            if *source_ty != ty {
                return Ok(None);
            }
            argument_sources.push(Some(source.clone()));
        }
    }
    if looked_up_count != 1 {
        return Ok(None);
    }
    let (_, key_source) = parameters
        .get(key_source_name.as_str())
        .expect("checked key");
    let slot = ident(field)?;
    let method = ident(pure_name)?;
    let value_ty = rust_type(&value_binding.ty)?;
    let mut steps = vec![
        syn::parse_quote!(let __compact_recorded_map_key: runtime::OpaqueString = (#key_source).clone();),
        syn::parse_quote!(
            let (frame, __compact_recorded_map_member): (_, bool) =
                crate::ledger_slots::#slot.record_member(frame, __compact_recorded_map_key.clone())?;
        ),
        syn::parse_quote!(
            if !__compact_recorded_map_member {
                return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
            }
        ),
        syn::parse_quote!(
            let (frame, __compact_recorded_map_value): (_, #value_ty) =
                crate::ledger_slots::#slot.record_lookup(frame, __compact_recorded_map_key)?;
        ),
    ];
    let mut typed_arguments = Vec::new();
    for (index, (source, pure_parameter)) in
        argument_sources.iter().zip(&pure.parameters).enumerate()
    {
        let arg = ident(&format!("__compact_recorded_guard_arg_{index}"))?;
        let arg_ty = rust_type(&pure_parameter.ty)?;
        if let Some(source) = source {
            if matches!(
                pure_parameter.ty,
                Type::Unsigned { .. } | Type::Boolean | Type::Field
            ) {
                steps.push(syn::parse_quote!(let #arg: #arg_ty = #source;));
            } else {
                steps.push(syn::parse_quote!(let #arg: #arg_ty = (#source).clone();));
            }
        } else {
            steps.push(syn::parse_quote!(let #arg: #arg_ty = __compact_recorded_map_value;));
        }
        typed_arguments.push(arg);
    }
    steps.push(syn::parse_quote!(crate::pure_circuits::#method(#(#typed_arguments),*)?;));
    Ok(Some(steps))
}
