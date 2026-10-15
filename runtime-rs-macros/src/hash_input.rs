// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Hash-only atom composition, independent of ledger storage conversion.
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

pub(crate) fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "CompactHashInput requires a concrete generated type without generic bounds",
        ));
    }
    let name = &input.ident;
    let body = match &input.data {
        Data::Struct(data) => {
            let Fields::Named(fields) = &data.fields else {
                return Err(syn::Error::new_spanned(
                    name,
                    "CompactHashInput requires named fields",
                ));
            };
            let names = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap());
            quote! {
                let mut __compact_atoms = ::std::vec::Vec::new();
                #(__compact_atoms.extend(
                    ::midnight_compact_runtime::HashInputValue::into_hash_value(self.#names).0
                );)*
                ::midnight_compact_runtime::fab::Value(__compact_atoms)
            }
        }
        Data::Enum(data)
            if !data.variants.is_empty()
                && data
                    .variants
                    .iter()
                    .all(|v| matches!(v.fields, Fields::Unit) && v.discriminant.is_none()) =>
        {
            // CompactCellValue owns ordinal encoding; never duplicate it here.
            quote!(::core::convert::Into::<
                ::midnight_compact_runtime::fab::Value,
            >::into(self))
        }
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "CompactHashInput requires a named struct or nonempty unit enum without discriminants",
            ));
        }
    };
    Ok(quote! {
        impl ::midnight_compact_runtime::HashInputValue for #name {
            fn into_hash_value(self) -> ::midnight_compact_runtime::fab::Value {
                #body
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_fields_preserve_order_and_do_not_shadow_generated_locals() {
        let output = expand(syn::parse_quote! { struct Payload {
            __compact_atoms: OpaqueString, r#type: Nested, tail: FixedVector<OpaqueBytes, 2>
        }})
        .unwrap();
        syn::parse2::<syn::ItemImpl>(output.clone()).unwrap();
        let text = output.to_string();
        assert_eq!(text.matches("into_hash_value (self .").count(), 3);
        assert!(text.find("self . __compact_atoms").unwrap() < text.find("self . r#type").unwrap());
        assert!(text.find("self . r#type").unwrap() < text.find("self . tail").unwrap());
        assert!(!text.contains("clone"));
    }

    #[test]
    fn empty_struct_and_enum_keep_authoritative_encoding_separate() {
        let empty = expand(syn::parse_quote! { struct Empty {} })
            .unwrap()
            .to_string();
        assert!(!empty.contains("extend"));
        let enumeration = expand(syn::parse_quote! { enum State { First, Second } })
            .unwrap()
            .to_string();
        assert!(enumeration.contains("Into"));
        assert!(!enumeration.contains("match"));
    }

    #[test]
    fn rejects_shapes_outside_concrete_generated_types() {
        for source in [
            "struct Generic<T> { value: T }",
            "struct Tuple(u8);",
            "struct Unit;",
            "enum Empty {}",
            "enum Data { Value(u8) }",
            "enum Explicit { Value = 1 }",
            "union Union { value: u8 }",
            "struct Bounded where u8: Copy { value: u8 }",
        ] {
            assert!(expand(syn::parse_str(source).unwrap()).is_err(), "{source}");
        }
    }
}
