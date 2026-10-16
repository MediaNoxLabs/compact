// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Compile the actual independently rendered helper without changing raw output.

fn recording_items(file: &mut syn::File) -> &mut Vec<syn::Item> {
    let ledger = file
        .items
        .iter_mut()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "ledger_contract" => Some(module),
            _ => None,
        })
        .unwrap();
    let recorded = ledger
        .content
        .as_mut()
        .unwrap()
        .1
        .iter_mut()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "recorded" => Some(module),
            _ => None,
        })
        .unwrap();
    &mut recorded.content.as_mut().unwrap().1
}

pub(super) fn assemble(raw: &str, helper: &syn::Item) -> String {
    let syn::Item::Fn(function) = helper else {
        panic!("recorded helper must be a function")
    };
    let original = syn::parse_file(raw).unwrap();
    let mut assembled = original.clone();
    let items = recording_items(&mut assembled);
    assert!(!items.iter().any(|item| matches!(item,syn::Item::Fn(existing) if existing.sig.ident == function.sig.ident)),"direct helper must be distinct from normal raw exports");
    items.push(helper.clone());
    let source = prettyplease::unparse(&assembled);
    let mut restored = syn::parse_file(&source).unwrap();
    let removed = recording_items(&mut restored).pop().unwrap();
    // The formatter inserts optional trailing commas. Compare both parsed
    // items through that same formatter, without deleting punctuation or
    // relaxing any identifier, type, body or item comparison.
    let removed_file: syn::File = syn::parse_quote!(#removed);
    let helper_file: syn::File = syn::parse_quote!(#helper);
    assert_eq!(
        prettyplease::unparse(&removed_file),
        prettyplease::unparse(&helper_file)
    );
    assert_eq!(
        quote::quote!(#restored).to_string(),
        quote::quote!(#original).to_string(),
        "the qualification shell preserves all original generated AST items"
    );
    source
}
