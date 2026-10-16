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

//! Typed source recognition for legacy recorded values and continuations.
//! These functions only admit the existing value shapes and construct syntax.
//! They do not own scopes, effect traversal, profile selection or mutable frames.

use crate::ir::{CounterAmount, Expr, LocalBinding, StateAction, Type};
use crate::{RenderError, expression_with_calls, ident, retained_value, rust_type};
use proc_macro2::Span;
use std::collections::HashMap;

pub(in crate::recorded) fn amount_source(
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
pub(in crate::recorded) fn conditional_uint64_source(
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
pub(in crate::recorded) fn closed_nested_uint4_source(
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

pub(in crate::recorded) fn closed_counter_one_continuation(action: &StateAction) -> bool {
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
pub(in crate::recorded) fn closed_conditional_field_pair(
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
pub(in crate::recorded) fn closed_annotated_uint8_source(
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
pub(in crate::recorded) fn closed_curve_field_argument(
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

pub(in crate::recorded) fn cell_source(
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
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            length,
        } if matches!(ty, Type::Vector { element, length: expected }
                if **element == *result && parameter.ty == *result && expected == length)
            && matches!(result, Type::Unsigned { .. })
            && matches!(body.as_ref(), Expr::Parameter { name } if name == &parameter.name)
            && matches!(source.as_ref(), Expr::Tuple { elements }
                    if elements.len() == *length
                        && elements.iter().all(|value| matches!(value, Expr::UnsignedLiteral { .. }))) =>
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
pub(in crate::recorded) fn literal_field_pair_hash_source(
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
    if !matches!(left, Expr::FieldLiteral { .. }) || !matches!(right, Expr::FieldLiteral { .. }) {
        return None;
    }
    let left = cell_source(left, &Type::Field, locals, parameters)?;
    let right = cell_source(right, &Type::Field, locals, parameters)?;
    Some(syn::parse_quote!((#left, #right)))
}

pub(in crate::recorded) fn static_bindings(
    bindings: &[LocalBinding],
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
) -> Result<Option<HashMap<String, syn::Expr>>, RenderError> {
    let mut scoped = locals.clone();
    for binding in bindings {
        if !matches!(binding.ty, Type::Vector { .. })
            && !(binding.ty == Type::Field && matches!(binding.value, Expr::FieldLiteral { .. }))
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
pub(in crate::recorded) fn closed_boolean_assertion_arm(
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
pub(in crate::recorded) fn closed_unsigned_ternary_comparison(
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
        (max == literal_max && max <= u8::MAX as u128 && literal <= max).then_some((literal, max))
    }

    fn selected_literal(
        value: &Expr,
        locals: &HashMap<String, syn::Expr>,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Option<(syn::Expr, u128)> {
        let (value, cast_max) = match value {
            Expr::UnsignedCast { max, value } => (value.as_ref(), Some(max.parse::<u128>().ok()?)),
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
            selected_literal(left, locals, parameters).map(|(selected, max)| (selected, max, right))
        })?;
    let other = comparable_operand(other, max, parameters)?;
    Some(if equal {
        syn::parse_quote!(#other == #selected)
    } else {
        syn::parse_quote!(#other != #selected)
    })
}
