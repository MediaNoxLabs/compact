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

use super::expand;
use quote::ToTokens;

#[test]
fn rejects_shapes_outside_concrete_named_structs() {
    for source in [
        "struct Generic<T> { value: T }",
        "struct Lifetime<'a> { value: &'a bool }",
        "struct Constant<const N: usize> { value: [bool; N] }",
        "struct Bound where bool: Copy { value: bool }",
        "struct Tuple(bool);",
        "struct Unit;",
        "enum Choice { First }",
        "union Overlay { value: u64 }",
    ] {
        let result = expand(syn::parse_str(source).unwrap());
        assert!(result.is_err(), "accepted {source}");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("CompactStructRepr")
        );
    }
}

#[test]
fn preserves_field_types_and_order_without_field_named_locals() {
    let file: syn::File = syn::parse2(expand(syn::parse_quote! {
        struct Record { __compact_input: crate::First, __compact_field_0: crate::Second, r#type: crate::Third }
    }).unwrap()).unwrap();
    assert_eq!(file.items.len(), 3);
    let source = file.to_token_stream().to_string();
    for mapping in [
        "__compact_input : __compact_field_0",
        "__compact_field_0 : __compact_field_1",
        "r#type : __compact_field_2",
    ] {
        assert!(source.contains(mapping), "missing {mapping}");
    }
    let first = source.find("< crate :: First as").unwrap();
    let second = source.find("< crate :: Second as").unwrap();
    let third = source.find("< crate :: Third as").unwrap();
    assert!(first < second && second < third);
}

#[test]
fn empty_named_struct_has_three_parseable_representation_impls() {
    let file: syn::File = syn::parse2(
        expand(syn::parse_quote!(
            struct Empty {}
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(file.items.len(), 3);
    let source = file.to_token_stream().to_string();
    assert!(source.contains("FIELD_SIZE : :: core :: primitive :: usize = 0"));
    assert!(!source.contains("let mut"));
}
