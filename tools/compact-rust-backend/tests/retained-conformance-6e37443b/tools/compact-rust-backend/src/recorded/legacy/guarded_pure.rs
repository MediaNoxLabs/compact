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

//! Closed pure-guard admission before an ordered stateful continuation.
//! These recognizers preserve the existing parameter, arithmetic and continuation
//! restrictions; they neither select profiles nor broaden recording support.

use super::value_sources::closed_counter_one_continuation;
use crate::ir::{Expr, PureCircuit, StateAction, StateReturn, StatefulCircuit, Type};
use crate::{RenderError, ident, rust_type};
use proc_macro2::Span;
use std::collections::HashMap;

// The asset and attestation freshness calls share a pure Unit guard
// before one closed stateful continuation. Keep the three typed source
// arguments and the pure assertion shape closed; the emitted pure Rust
// function owns its nested struct projection and bounded subtraction.
pub(in crate::recorded) fn closed_guarded_struct_pure_steps(
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

// A mixed-width product assertion lives in a pure Unit circuit because
// the stateful source cannot inline ordering comparisons. Admit only its
// exact two-Uint32, multiply-by-four, less-equal body before Counter-one.
pub(in crate::recorded) fn closed_guarded_unsigned_product_steps(
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
