// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Rust syntax is created only after semantic plan validation.
use super::super::typed_plan::TypedPlan;
use super::{RecordingGap, declarations::Declarations, invalid, model::*};
use crate::{ident, rust_type};
use proc_macro2::Span;
mod leaf;
mod merkle;
mod qualified_coins;
mod zswap;

fn name(value: &Value) -> syn::Ident {
    syn::Ident::new(&format!("__compact_effect_{}", value.id), Span::call_site())
}
fn value(value: &Value) -> syn::Expr {
    let name = name(value);
    crate::retained_value(syn::parse_quote!(#name), &value.ty)
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
        match &binding.operation {
            Operation::Call {
                arguments,
                parameters,
                body,
                ..
            } => {
                let mut formals = Vec::<syn::Stmt>::new();
                for (parameter, argument) in parameters.iter().zip(arguments) {
                    let parameter_name = name(parameter);
                    let argument = value(argument);
                    let parameter_type =
                        rust_type(&parameter.ty).map_err(|e| invalid(path, e.to_string()))?;
                    formals
                        .push(syn::parse_quote!(let #parameter_name: #parameter_type = #argument;));
                }
                let body_steps = block(body, declarations)?;
                let result = value(&body.result);
                steps.push(syn::parse_quote! {
                    let (frame, #output): (_, #ty) = {
                        #(#formals)*
                        #(#body_steps)*
                        (frame, #result)
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
                let then_value = value(&then.result);
                let else_value = value(&otherwise.result);
                steps.push(syn::parse_quote! {
                    let (frame,#output): (_,#ty) = if #condition {
                        #(#then_steps)*
                        (frame,#then_value)
                    } else {
                        #(#else_steps)*
                        (frame,#else_value)
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
