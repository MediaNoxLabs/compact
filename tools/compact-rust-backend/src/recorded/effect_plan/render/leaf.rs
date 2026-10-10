// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Nonrecursive opcode syntax. Large quote temporaries stay off recursive block frames.
use super::*;
use crate::ir::Type;

pub(super) fn render(
    binding: &Binding,
    declarations: &Declarations,
    steps: &mut Vec<syn::Stmt>,
) -> Result<(), RecordingGap> {
    let path = &binding.path;
    let output = name(&binding.value);
    let ty = rust_type(&binding.value.ty).map_err(|e| invalid(path, e.to_string()))?;
    let expression: syn::Expr = match &binding.operation {
        Operation::Literal(literal) => match literal {
            Literal::Unit => syn::parse_quote!(()),
            Literal::Bytes(bytes) => syn::parse_quote!(runtime::FixedBytes::new([#(#bytes),*])),
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
        Operation::EnumVariant { variant } => {
            let variant = ident(variant).map_err(|e| invalid(path, e.to_string()))?;
            syn::parse_quote!(#ty::#variant)
        }
        Operation::Default => syn::parse_quote!(<#ty as ::core::default::Default>::default()),
        Operation::StructConstruct { fields: inputs } => {
            let crate::ir::Type::Struct { fields, .. } = &binding.value.ty else {
                unreachable!("checked struct construction")
            };
            let members = fields
                .iter()
                .map(|f| ident(&f.name))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(path, e.to_string()))?;
            let inputs: Vec<_> = inputs.iter().map(value).collect();
            syn::parse_quote!(#ty { #(#members: #inputs),* })
        }
        Operation::StructProject {
            value: input,
            field,
            ..
        } => {
            let member = ident(field).map_err(|e| invalid(path, e.to_string()))?;
            let source = name(input);
            crate::retained_value(syn::parse_quote!(#source.#member), &binding.value.ty)
        }
        Operation::Vector { elements } => {
            let Type::Vector { element, .. } = &binding.value.ty else {
                unreachable!("checked vector output");
            };
            if elements.is_empty() {
                let element =
                    crate::rust_type(element).map_err(|error| invalid(path, error.to_string()))?;
                syn::parse_quote!(runtime::FixedVector::<#element, 0>::new([]))
            } else {
                let elements: Vec<_> = elements.iter().map(value).collect();
                syn::parse_quote!(runtime::FixedVector::new([#(#elements),*]))
            }
        }
        Operation::Tuple { elements } => {
            let elements: Vec<_> = elements.iter().map(value).collect();
            syn::parse_quote!((#(#elements,)*))
        }
        Operation::CreateZswapInput { coin } => {
            steps.push(zswap::input(coin));
            syn::parse_quote!(())
        }
        Operation::CreateZswapOutput { coin, recipient } => {
            steps.push(zswap::output(coin, recipient));
            syn::parse_quote!(())
        }
        Operation::KernelClaim { kind, value: input } => {
            use crate::ir::KernelClaimKind;
            let input = value(input);
            let claim: syn::Expr = match kind {
                KernelClaimKind::Nullifier => {
                    syn::parse_quote!(runtime::ledger::KernelClaim::Nullifier(
                        runtime::ledger::CoinNullifier(runtime::ledger::HashOutput((#input).into_array()))))
                }
                KernelClaimKind::CoinSpend => {
                    syn::parse_quote!(runtime::ledger::KernelClaim::CoinSpend(
                        runtime::ledger::CoinCommitment(runtime::ledger::HashOutput((#input).into_array()))))
                }
                KernelClaimKind::CoinReceive => {
                    syn::parse_quote!(runtime::ledger::KernelClaim::CoinReceive(
                        runtime::ledger::CoinCommitment(runtime::ledger::HashOutput((#input).into_array()))))
                }
            };
            steps.push(syn::parse_quote!(let frame = frame.kernel_claim(#claim)?;));
            syn::parse_quote!(())
        }
        Operation::KernelMintShielded { domain, amount } => {
            let (domain, amount) = (value(domain), value(amount));
            // The checked exact Uint64 signature proves this conversion fits.
            steps.push(syn::parse_quote!(let frame = frame.kernel_mint_shielded(
                    runtime::ledger::HashOutput((#domain).into_array()), (#amount).value() as u64,
                )?;));
            syn::parse_quote!(())
        }
        Operation::KernelSelf => {
            let observed = syn::Ident::new(
                &format!("__compact_effect_address_{}", binding.value.id),
                Span::call_site(),
            );
            steps.push(syn::parse_quote!(let (frame, #observed) = frame.kernel_self()?;));
            syn::parse_quote!(#ty { bytes: runtime::ledger::contract_address_bytes(&#observed) })
        }
        Operation::PersistentCommit {
            value: input,
            opening,
        } => {
            let (input, opening) = (value(input), value(opening));
            syn::parse_quote!(runtime::persistent_commit(#input, #opening))
        }
        Operation::FieldToBytes32(input) => crate::field_to_bytes_32_syntax(value(input)),
        Operation::PersistentHash(input) => {
            let input = value(input);
            syn::parse_quote!(runtime::persistent_hash(#input))
        }
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
        Operation::DegradeToTransient(input) => {
            let input = value(input);
            syn::parse_quote!(runtime::degrade_to_transient(#input))
        }
        Operation::UpgradeFromTransient(input) => {
            let input = value(input);
            syn::parse_quote!(runtime::upgrade_from_transient(#input))
        }
        Operation::TransientHash(input) => {
            let input = value(input);
            syn::parse_quote!(runtime::transient_hash(#input))
        }
        Operation::TransientCommit {
            value: input,
            opening,
        } => {
            let (input, opening) = (value(input), value(opening));
            syn::parse_quote!(runtime::transient_commit(#input, #opening))
        }
        Operation::UnsignedAdd(a, b)
        | Operation::UnsignedSubtract(a, b)
        | Operation::UnsignedMultiply(a, b) => {
            // The native syntax helper only uses the variant as an operation tag;
            // operands have already been evaluated once in SSA source order.
            let max = unsigned_max(&binding.value.ty).into();
            let left = Box::new(crate::ir::Expr::Unit);
            let right = Box::new(crate::ir::Expr::Unit);
            let op = match &binding.operation {
                Operation::UnsignedAdd(..) => crate::ir::Expr::UnsignedAdd { max, left, right },
                Operation::UnsignedSubtract(..) => {
                    crate::ir::Expr::UnsignedSubtract { max, left, right }
                }
                Operation::UnsignedMultiply(..) => {
                    crate::ir::Expr::UnsignedMultiply { max, left, right }
                }
                _ => unreachable!("unsigned arithmetic arm"),
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
        Operation::Merkle { slot, operation } => {
            return merkle::render(binding, *slot, operation, declarations, steps);
        }
        Operation::CellRead(slot) => {
            let slot = slot_name(declarations, slot.declaration, path)?;
            steps.push(syn::parse_quote!(let (frame,#output): (_,#ty) = crate::ledger_slots::#slot.record_read(frame)?;));
            return Ok(());
        }
        Operation::CellWrite { slot, value: input } => {
            let slot = slot_name(declarations, slot.declaration, path)?;
            let input = value(input);
            steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_write(frame,#input)?;));
            syn::parse_quote!(())
        }
        Operation::CellWriteCoin {
            slot,
            coin,
            recipient,
        } => {
            let slot = slot_name(declarations, slot.declaration, path)?;
            steps.push(qualified_coins::write(&slot, coin, recipient));
            syn::parse_quote!(())
        }
        Operation::SetInsertCoin {
            slot,
            coin,
            recipient,
        } => {
            let slot = slot_name(declarations, slot.declaration, path)?;
            steps.push(qualified_coins::insert(&slot, coin, recipient));
            syn::parse_quote!(())
        }
        Operation::SetMember { slot, value: input } => {
            let slot = slot_name(declarations, slot.declaration, path)?;
            let input = value(input);
            steps.push(syn::parse_quote!(let (frame, #output): (_, #ty) = crate::ledger_slots::#slot.record_member(frame, #input)?;));
            return Ok(());
        }
        Operation::SetInsert { slot, value: input }
        | Operation::SetRemove { slot, value: input } => {
            let slot = slot_name(declarations, slot.declaration, path)?;
            let input = value(input);
            let method = ident(
                if matches!(binding.operation, Operation::SetInsert { .. }) {
                    "record_insert"
                } else {
                    "record_remove"
                },
            )
            .map_err(|e| invalid(path, e.to_string()))?;
            steps.push(
                syn::parse_quote!(let frame = crate::ledger_slots::#slot.#method(frame, #input)?;),
            );
            syn::parse_quote!(())
        }
        Operation::SetReset { slot } => {
            let slot = slot_name(declarations, slot.declaration, path)?;
            steps.push(
                syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_reset(frame)?;),
            );
            syn::parse_quote!(())
        }
        Operation::SetIsEmpty { slot } => {
            let slot = slot_name(declarations, slot.declaration, path)?;
            steps.push(syn::parse_quote!(let (frame, #output): (_, #ty) = crate::ledger_slots::#slot.record_is_empty(frame)?;));
            return Ok(());
        }
        Operation::SetSize { slot } => {
            let slot = slot_name(declarations, slot.declaration, path)?;
            let raw = syn::Ident::new(
                &format!("__compact_effect_set_size_{}", binding.value.id),
                Span::call_site(),
            );
            steps.push(syn::parse_quote!(let (frame, #raw) = crate::ledger_slots::#slot.record_size(frame)?;));
            syn::parse_quote!(<#ty>::new(#raw as u128)?)
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
            return Ok(());
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
            return Ok(());
        }
        Operation::Assert { condition, message } => {
            let condition = value(condition);
            steps.push(syn::parse_quote!(if !#condition { return Err(runtime::CompactError::AssertionFailed(#message.to_owned())); }));
            syn::parse_quote!(())
        }
        Operation::PureCall {
            declaration,
            arguments,
        } => {
            let certified = declarations
                .certified_pure(*declaration)
                .ok_or_else(|| invalid(path, "missing checked pure declaration"))?;
            let method =
                ident(&certified.source().name).map_err(|e| invalid(path, e.to_string()))?;
            let arguments: Vec<_> = arguments.iter().map(value).collect();
            syn::parse_quote!(crate::pure_circuits::#method(#(#arguments),*)?)
        }
        Operation::Call { .. } | Operation::If { .. } => {
            unreachable!("control operations are rendered by block traversal")
        }
    };
    steps.push(syn::parse_quote!(let #output: #ty = #expression;));
    Ok(())
}
