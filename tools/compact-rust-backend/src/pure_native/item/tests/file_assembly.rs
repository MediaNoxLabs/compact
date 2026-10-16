// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Three optional-module partitions pin exact prepared-file output/order.
use super::*;
use quote::quote;
#[allow(
    unused_mut,
    reason = "retains exact old mutable final-file assembly region"
)]
fn old(
    runtime_abi: syn::LitInt,
    types_module: Option<syn::Item>,
    alias_exports: Option<syn::Item>,
    items: Vec<syn::Item>,
    slots_module: Option<syn::Item>,
    ledger_module: Option<syn::Item>,
) -> syn::File {
    let mut file: syn::File = syn::parse2(quote! {
        /// The matching Midnight Compact Rust runtime used by this generated crate.
        pub use midnight_compact_runtime as runtime;
        #types_module
        #alias_exports
        #[allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables)]
        pub mod pure_circuits {
            use midnight_compact_runtime as runtime;
            const _: () = assert!(runtime::RUST_RUNTIME_ABI == #runtime_abi);
            #(#items)*
        }
        #slots_module
        #ledger_module
    })
    .expect("typed renderer constructed invalid Rust syntax");
    file
}

#[test]
fn prepared_file_preserves_module_order_guards_and_optional_modules() {
    for present in [
        [false, false, false, false],
        [true, true, true, true],
        [true, false, false, true],
    ] {
        let types: Option<syn::Item> = present[0].then(|| {
            syn::parse_quote!(
                pub mod types {
                    pub struct Marker;
                }
            )
        });
        let aliases: Option<syn::Item> = present[1].then(|| {
            syn::parse_quote!(
                pub use types::Marker as Alias;
            )
        });
        let slots: Option<syn::Item> = present[2].then(|| {
            syn::parse_quote!(
                pub mod ledger_slots {}
            )
        });
        let ledger: Option<syn::Item> = present[3].then(|| {
            syn::parse_quote!(
                pub mod ledger_contract {}
            )
        });
        let functions: Vec<syn::Item> = if present[0] {
            vec![
                pure(
                    syn::parse_quote!(first),
                    false,
                    vec![],
                    syn::parse_quote!(runtime::Field),
                    NativeFragment::Value(syn::parse_quote!(read_once()?)),
                    false,
                ),
                pure(
                    syn::parse_quote!(second),
                    true,
                    vec![],
                    syn::parse_quote!(()),
                    NativeFragment::unit(syn::parse_quote!(()), vec![]),
                    true,
                ),
            ]
        } else {
            vec![]
        };
        let abi: syn::LitInt = syn::parse_quote!(3);
        let previous = old(
            abi.clone(),
            types.clone(),
            aliases.clone(),
            functions.clone(),
            slots.clone(),
            ledger.clone(),
        );
        let current = file(abi, types, aliases, functions, slots, ledger);
        assert_eq!(
            current.to_token_stream().to_string(),
            previous.to_token_stream().to_string()
        );
        assert_eq!(
            prettyplease::unparse(&current),
            prettyplease::unparse(&previous)
        );
        let pure_module = current
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Mod(module) if module.ident == "pure_circuits" => Some(module),
                _ => None,
            })
            .unwrap();
        let contents = &pure_module.content.as_ref().unwrap().1;
        assert!(matches!(&contents[0], syn::Item::Use(_)));
        assert!(matches!(&contents[1], syn::Item::Const(_)));
        assert_eq!(contents.len(), if present[0] { 4 } else { 2 });
    }
}
