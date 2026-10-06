// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Validation and type-preservation tests for generated Merkle adapters.
use super::{digest, entry, path};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{DeriveInput, ImplItem, Item, ReturnType};

type Expander = fn(DeriveInput) -> syn::Result<TokenStream>;

#[test]
fn every_merkle_adapter_rejects_non_named_or_generic_declarations() {
    for expand in [digest as Expander, entry, path] {
        for (source, message) in [
            ("enum NotStruct { Value }", "requires a struct"),
            ("union NotStruct { value: u8 }", "requires a struct"),
            ("struct Tuple(u8);", "requires named fields"),
            ("struct Unit;", "requires named fields"),
            ("struct Generic<T> { field: T }", "requires a concrete type"),
            (
                "struct Borrowed<'a> { field: &'a u8 }",
                "requires a concrete type",
            ),
            (
                "struct Sized<const N: usize> { field: [u8; N] }",
                "requires a concrete type",
            ),
        ] {
            let error = expand(syn::parse_str(source).unwrap()).unwrap_err();
            assert!(error.to_string().contains(message), "{source}: {error}");
        }
    }
}

#[test]
fn missing_merkle_members_report_the_specific_required_field() {
    for (expand, source, message) in [
        (
            digest as Expander,
            "struct Digest {}",
            "missing field field",
        ),
        (
            entry,
            "struct Entry { goes_left: bool }",
            "missing sibling field",
        ),
        (
            entry,
            "struct Entry { sibling: Digest }",
            "missing goes_left field",
        ),
        (
            path,
            "struct Path { path: FixedVector<Entry, 2> }",
            "missing leaf field",
        ),
        (path, "struct Path { leaf: Leaf }", "missing path field"),
    ] {
        let error = expand(syn::parse_str(source).unwrap()).unwrap_err();
        assert!(error.to_string().contains(message), "{source}: {error}");
    }
}

#[test]
fn path_rejects_nonvector_types_and_incomplete_or_misordered_arguments() {
    for (ty, message) in [
        ("[Entry; 2]", "path must be a FixedVector"),
        ("&FixedVector<Entry, 2>", "path must be a FixedVector"),
        ("OtherVector<Entry, 2>", "path must be a FixedVector"),
        ("FixedVector", "needs entry type and depth"),
        ("FixedVector<Entry>", "needs entry type and depth"),
        ("FixedVector<2, Entry>", "needs entry type and depth"),
        ("FixedVector<Entry, 2, 3>", "needs entry type and depth"),
        ("FixedVector<'static, 2>", "needs entry type and depth"),
    ] {
        let input = syn::parse_str(&format!("struct Path {{ leaf: Leaf, path: {ty} }}")).unwrap();
        let error = path(input).unwrap_err();
        assert!(error.to_string().contains(message), "{ty}: {error}");
    }
}

#[test]
fn digest_and_entry_emit_both_upstream_conversion_directions() {
    for (expand, source, upstream) in [
        (
            digest as Expander,
            "struct Digest { field: runtime::Field }",
            "MerkleTreeDigest",
        ),
        (
            entry,
            "struct Entry { sibling: Digest, goes_left: bool }",
            "MerklePathEntry",
        ),
    ] {
        let file =
            syn::parse2::<syn::File>(expand(syn::parse_str(source).unwrap()).unwrap()).unwrap();
        assert_eq!(file.items.len(), 2);
        for item in &file.items {
            let Item::Impl(item) = item else {
                panic!("conversion impl")
            };
            assert_eq!(
                item.trait_
                    .as_ref()
                    .unwrap()
                    .1
                    .segments
                    .last()
                    .unwrap()
                    .ident,
                "From"
            );
            assert_eq!(item.items.len(), 1);
            let ImplItem::Fn(method) = &item.items[0] else {
                panic!("conversion method")
            };
            assert_eq!(method.sig.ident, "from");
            assert_eq!(method.sig.inputs.len(), 1);
        }
        let Item::Impl(to_compact) = &file.items[0] else {
            panic!()
        };
        let Item::Impl(to_ledger) = &file.items[1] else {
            panic!()
        };
        assert!(
            to_compact
                .trait_
                .as_ref()
                .unwrap()
                .1
                .to_token_stream()
                .to_string()
                .contains(upstream)
        );
        assert!(
            to_ledger
                .self_ty
                .to_token_stream()
                .to_string()
                .ends_with(upstream)
        );
    }
}

#[test]
fn path_preserves_qualified_leaf_entry_and_const_depth() {
    for (leaf, element, depth) in [
        ("crate::types::Leaf", "crate::types::Entry", "2"),
        (
            "runtime::FixedBytes<32>",
            "crate::types::Entry",
            "{ 1 + 2 }",
        ),
    ] {
        let declaration = format!(
            "struct Path {{ leaf: {leaf}, path: runtime::FixedVector<{element}, {depth}> }}"
        );
        let file =
            syn::parse2::<syn::File>(path(syn::parse_str(&declaration).unwrap()).unwrap()).unwrap();
        assert_eq!(file.items.len(), 1);
        let Item::Impl(item) = &file.items[0] else {
            panic!("path impl")
        };
        assert!(item.trait_.is_none());
        assert_eq!(item.items.len(), 2);
        let ImplItem::Fn(from) = &item.items[0] else {
            panic!()
        };
        let ImplItem::Fn(into) = &item.items[1] else {
            panic!()
        };
        assert_eq!(from.sig.ident, "from_ledger_path");
        assert_eq!(into.sig.ident, "into_ledger_path");
        assert!(matches!(from.vis, syn::Visibility::Public(_)));
        assert!(matches!(into.vis, syn::Visibility::Public(_)));
        let leaf = syn::parse_str::<syn::Type>(leaf)
            .unwrap()
            .to_token_stream()
            .to_string();
        assert!(
            from.sig
                .inputs
                .to_token_stream()
                .to_string()
                .contains(&leaf)
        );
        let ReturnType::Type(_, result) = &into.sig.output else {
            panic!()
        };
        assert!(result.to_token_stream().to_string().contains(&leaf));
        let body = from.block.to_token_stream().to_string();
        let element = syn::parse_str::<syn::Type>(element)
            .unwrap()
            .to_token_stream()
            .to_string();
        let depth = syn::parse_str::<syn::Expr>(depth)
            .unwrap()
            .to_token_stream()
            .to_string();
        assert!(body.contains(&format!("[{element} ; {depth}]")), "{body}");
        assert!(body.contains("try_into"));
        assert!(body.contains("Merkle path depth differs from Compact type"));
        assert!(
            into.block
                .to_token_stream()
                .to_string()
                .contains("into_array")
        );
    }
}
