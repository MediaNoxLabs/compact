// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native leaf syntax from already checked literal, name and ownership facts.
use crate::{
    naming::checked_ident,
    pure_semantics::{
        numeric::UnsignedMaximum,
        ownership::Ownership,
        sink::{CheckedLeaf, FieldLiteralFacts, LeafSink},
    },
    value_lowering::{carrier::NativeCarrierSink, wide_uint_type},
};
use proc_macro2::Span;

pub(crate) struct NativeLeafSink {
    carriers: NativeCarrierSink,
}
impl NativeLeafSink {
    pub(crate) fn new() -> Self {
        Self {
            carriers: NativeCarrierSink,
        }
    }
}

impl LeafSink for NativeLeafSink {
    type Value = syn::Expr;
    type Binding = syn::Ident;
    type Carriers = NativeCarrierSink;

    fn carriers(&mut self) -> &mut NativeCarrierSink {
        &mut self.carriers
    }

    fn leaf(&mut self, facts: CheckedLeaf<'_, syn::Type, syn::Ident>) -> syn::Expr {
        match facts {
            CheckedLeaf::Unit => syn::parse_quote!(()),
            CheckedLeaf::Default { carrier } => syn::parse_quote!(<#carrier as Default>::default()),
            CheckedLeaf::Boolean(value) => syn::parse_quote!(#value),
            CheckedLeaf::Field(FieldLiteralFacts::Small(value)) => {
                let value = syn::LitInt::new(&format!("{value}u128"), Span::call_site());
                syn::parse_quote!(runtime::Field::from(#value))
            }
            CheckedLeaf::Field(FieldLiteralFacts::CanonicalBytes(bytes)) => {
                let bytes = byte_literals(&bytes);
                syn::parse_quote!(runtime::Field::from_le_bytes(&[#(#bytes),*]).expect("validated Compact Field literal"))
            }
            CheckedLeaf::Bytes(bytes) => {
                let values = byte_literals(bytes);
                syn::parse_quote!(runtime::FixedBytes::new([#(#values),*]))
            }
            CheckedLeaf::Unsigned(facts) => match facts.maximum() {
                UnsignedMaximum::Small(_) => {
                    let max = syn::LitInt::new(facts.max(), Span::call_site());
                    let value =
                        syn::LitInt::new(&format!("{}u128", facts.value()), Span::call_site());
                    syn::parse_quote!(runtime::BoundedUint::<#max>::new(#value).expect("Compact Uint literal fits its maximum"))
                }
                UnsignedMaximum::Wide { high, low } => {
                    let ty = wide_uint_type(high, low);
                    let bytes = byte_literals(&facts.bytes()[..31]);
                    syn::parse_quote!(<#ty>::from_le_bytes(&[#(#bytes),*]).expect("Compact Uint literal fits its maximum"))
                }
            },
            CheckedLeaf::Parameter {
                binding,
                ownership: Ownership::Copy,
            } => syn::parse_quote!(#binding),
            CheckedLeaf::Parameter {
                binding,
                ownership: Ownership::Owned,
            } => syn::parse_quote!(#binding.clone()),
            CheckedLeaf::EnumVariant { carrier, variant } => {
                let variant = checked_ident(&variant);
                syn::parse_quote!(#carrier::#variant)
            }
        }
    }
}

fn byte_literals(bytes: &[u8]) -> Vec<syn::LitInt> {
    bytes
        .iter()
        .map(|byte| syn::LitInt::new(&format!("{byte}u8"), Span::call_site()))
        .collect()
}
