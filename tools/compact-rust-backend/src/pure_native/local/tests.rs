// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::pure_semantics::identifier::check_name;

#[test]
fn accepted_words_produce_safe_symbols_without_copying_source_trivia_or_raw_markers() {
    for (source, expected) in [
        ("x", "__compact_local_x"),
        ("x$0", "__compact_local_x_0"),
        ("r#value", "__compact_local_value"),
        ("type", "__compact_local_type"),
        ("gen", "__compact_local_gen"),
        ("x /* comment */", "__compact_local_x"),
    ] {
        let checked = check_name(source).unwrap();
        assert_eq!(allocate(&checked, source, []).to_string(), expected);
    }
}

#[test]
fn distinct_raw_keys_and_raw_native_spellings_block_but_same_key_shadow_does_not() {
    let checked = check_name("x").unwrap();
    let base: syn::Ident = syn::parse_quote!(__compact_local_x);
    let raw: syn::Ident = syn::parse_quote!(r#__compact_local_x);
    let first: syn::Ident = syn::parse_quote!(__compact_local_x_1);
    assert_eq!(
        allocate(&checked, "x", [("x", &base)]).to_string(),
        "__compact_local_x"
    );
    assert_eq!(
        allocate(&checked, "x", [("other", &raw)]).to_string(),
        "__compact_local_x_1"
    );
    let occupied = [("other", &base), ("another", &first), ("x", &base)];
    assert_eq!(
        allocate(&checked, "x", occupied).to_string(),
        "__compact_local_x_2"
    );
    assert_eq!(
        allocate(&checked, "x", occupied.into_iter().rev()).to_string(),
        "__compact_local_x_2"
    );
    // Lowest-free choice may change when the live namespace changes; previous
    // same-key suffix is not pinned as an allocation promise.
    assert_eq!(
        allocate(&checked, "x", [("other", &base), ("x", &first)]).to_string(),
        "__compact_local_x_1"
    );
    assert_eq!(
        allocate(&checked, "x", [("x", &first)]).to_string(),
        "__compact_local_x"
    );
}
