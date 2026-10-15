// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn argument_ast_moves_checked_raw_names_and_nested_carriers_without_changing_output() {
    let names: [syn::Ident; 3] = [
        syn::parse_quote!(value),
        syn::parse_quote!(r#type),
        syn::parse_quote!(x_0),
    ];
    for name in names {
        let carriers: [syn::Type; 4] = [
            syn::parse_quote!(()),
            syn::parse_quote!(runtime::Field),
            syn::parse_quote!(runtime::FixedVector<runtime::FixedVector<runtime::Field, 1>, 3>),
            syn::parse_quote!((runtime::Field, runtime::FixedBytes<32>)),
        ];
        for carrier in carriers {
            let old: syn::FnArg = syn::parse_quote!(#name: #carrier);
            let current = argument(name.clone(), carrier.clone());
            assert_eq!(
                current.to_token_stream().to_string(),
                old.to_token_stream().to_string()
            );
            let syn::FnArg::Typed(current) = current else {
                panic!("typed argument");
            };
            assert_eq!(
                current.ty.to_token_stream().to_string(),
                carrier.to_token_stream().to_string()
            );
        }
    }
}

#[test]
fn zero_one_many_arguments_and_nested_result_carriers_keep_public_private_signatures() {
    for internal in [false, true] {
        for count in [0, 1, 3, 8] {
            let arguments = (0..count)
                .map(|index| {
                    argument(
                        syn::Ident::new(&format!("arg{index}"), proc_macro2::Span::call_site()),
                        syn::parse_quote!(runtime::Field),
                    )
                })
                .collect();
            compare(
                syn::parse_quote!(r#type),
                internal,
                arguments,
                syn::parse_quote!(runtime::FixedVector<runtime::FixedVector<runtime::Field, 1>, 3>),
                NativeFragment::Value(syn::parse_quote!(read_once())),
                Type::Vector {
                    element: Box::new(Type::Field),
                    length: 3,
                },
            );
        }
    }
}
