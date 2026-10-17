// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Rust syntax is created only after semantic plan validation.
use super::super::recording_plan::TypedPlan;
use super::{RecordingGap, declarations::Declarations, invalid, model::*};
use crate::{ident, rust_type};
use proc_macro2::Span;
mod leaf;
mod lists;
mod maps;
mod merkle;
mod qualified_coins;
#[cfg(test)]
mod tests;
mod vector_map;
mod zswap;

fn name(value: &Value) -> syn::Ident {
    syn::Ident::new(&format!("__compact_effect_{}", value.id), Span::call_site())
}
fn value(value: &Value) -> syn::Expr {
    // A checked Unit value has no payload. Its producer remains an ordered
    // operation; reading the result need not name a synthetic temporary.
    if value.ty == crate::ir::Type::Unit {
        return syn::parse_quote!(());
    }
    let name = name(value);
    crate::retained_value(syn::parse_quote!(#name), &value.ty)
}
// Control blocks transfer frame ownership. Only checked Unit has no payload;
// even an empty IR tuple keeps its ordinary typed result carrier.
fn frame_result(result: &Value) -> syn::Expr {
    if result.ty == crate::ir::Type::Unit {
        syn::parse_quote!(frame)
    } else {
        let result = value(result);
        syn::parse_quote!((frame, #result))
    }
}

fn unsigned_max(ty: &crate::ir::Type) -> &str {
    let crate::ir::Type::Unsigned { max } = ty else {
        unreachable!("checked unsigned type")
    };
    max
}
pub(super) fn render(checked: &CheckedPlan) -> Result<TypedPlan, RecordingGap> {
    let plan = checked.plan();
    let mut steps = Vec::new();
    for (i, parameter) in plan.parameters.iter().enumerate() {
        // Unit reads already render as (). Keep public source parameters, but
        // do not introduce an internal alias for their absent payload.
        if parameter.ty == crate::ir::Type::Unit {
            continue;
        }
        let target = name(parameter);
        let source = syn::Ident::new(&format!("__compact_param_{i}"), Span::call_site());
        let ty = rust_type(&parameter.ty).map_err(|e| invalid("parameters", e.to_string()))?;
        steps.push(syn::parse_quote!(let #target: #ty = #source;));
    }
    steps.extend(block(&plan.body, checked.declarations())?);
    Ok(TypedPlan {
        steps,
        result: value(&plan.body.result),
    })
}
// Keep only control-flow syntax on recursive frames. Leaf opcode quote temporaries
// must not accumulate at each nesting level on an ordinary compiler worker stack.
fn block(body: &Block, declarations: &Declarations) -> Result<Vec<syn::Stmt>, RecordingGap> {
    let mut steps = Vec::new();
    for binding in &body.bindings {
        let path = &binding.path;
        let output = name(&binding.value);
        let ty = rust_type(&binding.value.ty).map_err(|e| invalid(path, e.to_string()))?;
        let target = if binding.value.ty == crate::ir::Type::Unit {
            quote::quote!(frame)
        } else {
            quote::quote!((frame, #output): (_, #ty))
        };
        let frame_attribute = if binding.value.ty == crate::ir::Type::Unit {
            quote::quote!(#[allow(clippy::let_and_return, reason = "uniform Unit frames preserve ordered recording steps")])
        } else {
            proc_macro2::TokenStream::new()
        };
        match &binding.operation {
            Operation::VectorMap { .. } => {
                steps.push(vector_map::render(binding, declarations)?);
            }
            Operation::Call {
                arguments,
                parameters,
                body,
                ..
            } => {
                let mut formals = Vec::<syn::Stmt>::new();
                for (parameter, argument) in parameters.iter().zip(arguments) {
                    // Argument producers belong to the enclosing ordered block.
                    // Only their checked Unit payload alias is unnecessary here.
                    if parameter.ty == crate::ir::Type::Unit {
                        continue;
                    }
                    let parameter_name = name(parameter);
                    let argument = value(argument);
                    let parameter_type =
                        rust_type(&parameter.ty).map_err(|e| invalid(path, e.to_string()))?;
                    formals
                        .push(syn::parse_quote!(let #parameter_name: #parameter_type = #argument;));
                }
                let body_steps = block(body, declarations)?;
                let result = frame_result(&body.result);
                steps.push(syn::parse_quote! {
                    #frame_attribute
                    let #target = {
                        #(#formals)*
                        #(#body_steps)*
                        #result
                    };
                });
            }
            Operation::If {
                condition,
                then,
                otherwise,
            } => {
                let condition = value(condition);
                let then_steps = block(then, declarations)?;
                let else_steps = block(otherwise, declarations)?;
                let then_value = frame_result(&then.result);
                let else_value = frame_result(&otherwise.result);
                steps.push(syn::parse_quote! {
                    #frame_attribute
                    let #target = if #condition {
                        #(#then_steps)*
                        #then_value
                    } else {
                        #(#else_steps)*
                        #else_value
                    };
                });
            }
            _ => leaf::render(binding, declarations, &mut steps)?,
        }
    }
    Ok(steps)
}

fn slot_name(
    declarations: &Declarations,
    id: LedgerId,
    path: &str,
) -> Result<syn::Ident, RecordingGap> {
    let declaration = declarations
        .field(id)
        .ok_or_else(|| invalid(path, "unknown checked ledger declaration"))?;
    ident(&declaration.id).map_err(|e| invalid(path, e.to_string()))
}

fn operand(input: &Value) -> (syn::Expr, crate::ir::Type) {
    (value(input), input.ty.clone())
}
fn jubjub(
    operation: crate::jubjub_value::Operation,
    path: &str,
) -> Result<syn::Expr, RecordingGap> {
    operation
        .lower()
        .map(|(syntax, _)| syntax)
        .map_err(|e| invalid(path, e.to_string()))
}
