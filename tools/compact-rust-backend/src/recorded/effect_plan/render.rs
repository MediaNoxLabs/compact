// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Rust syntax is created only after semantic plan validation.
use super::super::typed_plan::TypedPlan;
use super::{RecordingGap, declarations::Declarations, invalid, model::*};
use crate::{ident, rust_type};
use proc_macro2::Span;

fn name(value: &Value) -> syn::Ident {
    syn::Ident::new(&format!("__compact_effect_{}", value.id), Span::call_site())
}
fn value(value: &Value) -> syn::Expr {
    let name = name(value);
    syn::parse_quote!(#name)
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
fn block(body: &Block, declarations: &Declarations) -> Result<Vec<syn::Stmt>, RecordingGap> {
    let mut steps = Vec::new();
    for binding in &body.bindings {
        let path = &binding.path;
        let output = name(&binding.value);
        let ty = rust_type(&binding.value.ty).map_err(|e| invalid(path, e.to_string()))?;
        let expression: syn::Expr = match &binding.operation {
            Operation::Literal(literal) => match literal {
                Literal::Unit => syn::parse_quote!(()),
                Literal::Boolean(b) => syn::parse_quote!(#b),
                Literal::Field(bytes) => {
                    syn::parse_quote!(runtime::Field::from_le_bytes(&[#(#bytes),*]).expect("validated Compact Field literal"))
                }
                Literal::WideUnsigned(bytes) => {
                    syn::parse_quote!(<#ty>::from_le_bytes(&[#(#bytes),*])?)
                }
                Literal::Unsigned(n) => {
                    let n = syn::LitInt::new(&format!("{n}u128"), Span::call_site());
                    syn::parse_quote!(<#ty>::new(#n)?)
                }
            },
            Operation::Add(a, b) => {
                let (a, b) = (value(a), value(b));
                syn::parse_quote!(#a + #b)
            }
            Operation::JubjubScalarFromNative(input) => {
                jubjub(crate::jubjub_value::Operation::Reduce(operand(input)), path)?
            }
            Operation::EcMulGenerator(input) => jubjub(
                crate::jubjub_value::Operation::Generator(operand(input)),
                path,
            )?,
            Operation::EcMul { point, scalar } => jubjub(
                crate::jubjub_value::Operation::Multiply(operand(point), operand(scalar)),
                path,
            )?,
            Operation::EcAdd { left, right } => jubjub(
                crate::jubjub_value::Operation::Add(operand(left), operand(right)),
                path,
            )?,
            Operation::Equal(a, b) => {
                let (a, b) = (value(a), value(b));
                syn::parse_quote!(#a == #b)
            }
            Operation::UnsignedAdd(a, b) => {
                let op = crate::ir::Expr::UnsignedAdd {
                    max: unsigned_max(&binding.value.ty).into(),
                    left: Box::new(crate::ir::Expr::Unit),
                    right: Box::new(crate::ir::Expr::Unit),
                };
                crate::unsigned_arithmetic_syntax(
                    &op,
                    value(a),
                    value(b),
                    unsigned_max(&a.ty),
                    unsigned_max(&b.ty),
                    unsigned_max(&binding.value.ty),
                )
                .map_err(|e| invalid(path, e.to_string()))?
            }
            Operation::Convert { value: input, kind } => match kind {
                Conversion::Coerce => {
                    crate::coerce_expression(value(input), &input.ty, &binding.value.ty, 0)
                }
                Conversion::UnsignedCast => crate::unsigned_cast_syntax(
                    value(input),
                    unsigned_max(&input.ty),
                    unsigned_max(&binding.value.ty),
                ),
            }
            .map_err(|e| invalid(path, e.to_string()))?,
            Operation::CellRead(slot) => {
                let slot = slot_name(declarations, slot.declaration, path)?;
                steps.push(syn::parse_quote!(let (frame,#output): (_,#ty) = crate::ledger_slots::#slot.record_read(frame)?;));
                continue;
            }
            Operation::CellWrite { slot, value: input } => {
                let slot = slot_name(declarations, slot.declaration, path)?;
                let input = value(input);
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_write(frame,#input)?;));
                syn::parse_quote!(())
            }
            Operation::CounterRead { slot } => {
                let slot = slot_name(declarations, slot.declaration, path)?;
                let raw = syn::Ident::new(
                    &format!("__compact_effect_raw_{}", binding.value.id),
                    Span::call_site(),
                );
                steps.push(syn::parse_quote!(let (frame,#raw) = crate::ledger_slots::#slot.record_read(frame)?;));
                syn::parse_quote!(<#ty>::new(#raw as u128)?)
            }
            Operation::CounterIncrement { slot, amount }
            | Operation::CounterDecrement { slot, amount } => {
                let slot = slot_name(declarations, slot.declaration, path)?;
                let amount = value(amount);
                let method = if matches!(&binding.operation, Operation::CounterIncrement { .. }) {
                    syn::Ident::new("record_increment", Span::call_site())
                } else {
                    syn::Ident::new("record_decrement", Span::call_site())
                };
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.#method(frame,(#amount).value() as u16)?;));
                syn::parse_quote!(())
            }
            Operation::CounterReset { slot } => {
                let slot = slot_name(declarations, slot.declaration, path)?;
                steps.push(
                    syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_reset(frame)?;),
                );
                syn::parse_quote!(())
            }
            Operation::CounterLessThan { slot, threshold } => {
                let slot = slot_name(declarations, slot.declaration, path)?;
                let threshold = value(threshold);
                steps.push(syn::parse_quote!(let (frame,#output): (_,#ty) = crate::ledger_slots::#slot.record_less_than(frame,(#threshold).value() as u64)?;));
                continue;
            }
            Operation::Witness {
                declaration,
                arguments,
            } => {
                let witness = declarations
                    .witness(*declaration)
                    .ok_or_else(|| invalid(path, "unknown checked witness"))?;
                let method = ident(&witness.name).map_err(|e| invalid(path, e.to_string()))?;
                let args: Vec<_> = arguments.iter().map(value).collect();
                steps.push(syn::parse_quote! {
                    let (frame,#output): (_,#ty) = frame.try_witness_metered(|context,meter| {
                        witnesses.#method(context.witness_context_with(super::LedgerView {
                            state: context.query.state.get_ref(),meter,
                        }),#(#args),*)
                    })?;
                });
                continue;
            }
            Operation::Assert { condition, message } => {
                let condition = value(condition);
                steps.push(syn::parse_quote!(if !#condition { return Err(runtime::CompactError::AssertionFailed(#message.to_owned())); }));
                syn::parse_quote!(())
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
                continue;
            }
        };
        steps.push(syn::parse_quote!(let #output: #ty = #expression;));
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
