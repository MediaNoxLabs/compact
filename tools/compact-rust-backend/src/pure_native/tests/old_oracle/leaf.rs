// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn pure_leaf_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    _circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    match expr {
        Expr::Unit => Ok((syn::parse_quote!(()), Type::Unit)),
        Expr::Default { ty } => {
            let rendered_type = rust_type(ty)?;
            Ok((
                syn::parse_quote!(<#rendered_type as Default>::default()),
                ty.clone(),
            ))
        }
        Expr::Boolean { value } => Ok((syn::parse_quote!(#value), Type::Boolean)),
        Expr::FieldLiteral { value } => {
            let bytes = field_literal_bytes(value)?;
            if let Ok(parsed) = value.parse::<u128>() {
                let value = syn::LitInt::new(&format!("{parsed}u128"), Span::call_site());
                return Ok((syn::parse_quote!(runtime::Field::from(#value)), Type::Field));
            }
            let bytes = bytes
                .iter()
                .map(|byte| syn::LitInt::new(&format!("{byte}u8"), Span::call_site()))
                .collect::<Vec<_>>();
            Ok((
                syn::parse_quote!(runtime::Field::from_le_bytes(&[#(#bytes),*]).expect("validated Compact Field literal")),
                Type::Field,
            ))
        }
        Expr::BytesLiteral { bytes } => {
            let values = bytes
                .iter()
                .map(|value| syn::LitInt::new(&format!("{value}u8"), Span::call_site()))
                .collect::<Vec<_>>();
            Ok((
                syn::parse_quote!(runtime::FixedBytes::new([#(#values),*])),
                Type::Bytes {
                    length: bytes.len(),
                },
            ))
        }
        Expr::UnsignedLiteral { value, max } => {
            let maximum = unsigned_maximum(max)?;
            let bytes =
                field_literal_bytes(value).map_err(|_| RenderError::InvalidUnsignedLiteral {
                    value: value.clone(),
                    max: max.clone(),
                })?;
            let low = u128::from_le_bytes(bytes[..16].try_into().expect("low limb"));
            let high = u128::from_le_bytes(bytes[16..].try_into().expect("high limb"));
            let maximum_limbs = match maximum {
                UnsignedMaximum::Small(maximum) => (0, maximum),
                UnsignedMaximum::Wide { high, low } => (high, low),
            };
            if (high, low) > maximum_limbs {
                return Err(RenderError::InvalidUnsignedLiteral {
                    value: value.clone(),
                    max: max.clone(),
                });
            }
            let rendered: syn::Expr = match maximum {
                UnsignedMaximum::Small(_) => {
                    let max_lit = syn::LitInt::new(max, Span::call_site());
                    let value_lit = syn::LitInt::new(&format!("{value}u128"), Span::call_site());
                    syn::parse_quote!(runtime::BoundedUint::<#max_lit>::new(#value_lit).expect("Compact Uint literal fits its maximum"))
                }
                UnsignedMaximum::Wide { high, low } => {
                    let ty = wide_uint_type(high, low);
                    let bytes = bytes[..31]
                        .iter()
                        .map(|byte| syn::LitInt::new(&format!("{byte}u8"), Span::call_site()))
                        .collect::<Vec<_>>();
                    syn::parse_quote!(<#ty>::from_le_bytes(&[#(#bytes),*]).expect("Compact Uint literal fits its maximum"))
                }
            };
            Ok((rendered, Type::Unsigned { max: max.clone() }))
        }
        Expr::Parameter { name } => {
            let (ty, rust_name) = parameters
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
            let value = if copy_type(ty) {
                syn::parse_quote!(#rust_name)
            } else {
                syn::parse_quote!(#rust_name.clone())
            };
            Ok((value, (*ty).clone()))
        }
        Expr::EnumVariant { ty, variant } => {
            let Type::Enum { variants, .. } = ty else {
                return Err(RenderError::InvalidEnumVariant(variant.clone()));
            };
            if !variants.contains(variant) {
                return Err(RenderError::InvalidEnumVariant(variant.clone()));
            }
            let rust_ty = rust_type(ty)?;
            let rust_variant = ident(variant)?;
            Ok((syn::parse_quote!(#rust_ty::#rust_variant), ty.clone()))
        }
        Expr::WitnessCall { .. }
        | Expr::KernelClaim { .. }
        | Expr::KernelMintShielded { .. }
        | Expr::CreateZswapInput { .. }
        | Expr::CreateZswapOutput { .. }
        | Expr::NativeWitnessCall { .. }
        | Expr::SetMember { .. }
        | Expr::MapMember { .. }
        | Expr::MapLookup { .. }
        | Expr::MerkleCheckRoot { .. }
        | Expr::HistoricMerkleCheckRoot { .. }
        | Expr::SetIsEmpty { .. }
        | Expr::SetSize { .. }
        | Expr::MapIsEmpty { .. }
        | Expr::ListLength { .. }
        | Expr::ListIsEmpty { .. }
        | Expr::ListHead { .. }
        | Expr::CellRead { .. }
        | Expr::CounterRead { .. }
        | Expr::CounterLessThan { .. }
        | Expr::KernelSelf { .. } => Err(RenderError::EffectfulExpression),
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    }
}
