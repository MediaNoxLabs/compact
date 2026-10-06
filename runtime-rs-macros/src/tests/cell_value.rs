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

//! Tests for the accepted declaration boundary and generated codec structure.
use quote::ToTokens;
use syn::{Expr, Fields, ImplItem, Item, ItemImpl, Stmt};

use super::{expand, expand_enum};

fn impls(source: &str, enum_repr: bool) -> Vec<ItemImpl> {
    let input = syn::parse_str(source).expect("test declaration parses");
    let tokens = if enum_repr {
        expand_enum(input)
    } else {
        expand(input)
    }
    .unwrap();
    syn::parse2::<syn::File>(tokens)
        .unwrap()
        .items
        .into_iter()
        .map(|item| {
            let Item::Impl(item) = item else {
                panic!("expected an impl")
            };
            item
        })
        .collect()
}

fn method<'a>(implementation: &'a ItemImpl, name: &str) -> &'a syn::ImplItemFn {
    implementation
        .items
        .iter()
        .find_map(|item| match item {
            ImplItem::Fn(method) if method.sig.ident == name => Some(method),
            _ => None,
        })
        .expect("expected generated method")
}

#[test]
fn cell_value_rejects_nonconcrete_and_nonrecord_shapes() {
    for (source, message) in [
        ("struct Generic<T> { value: T }", "concrete generated type"),
        (
            "struct Borrowed<'a> { value: &'a u8 }",
            "concrete generated type",
        ),
        (
            "struct Sized<const N: usize> { value: [u8; N] }",
            "concrete generated type",
        ),
        ("struct Tuple(u8);", "named struct fields"),
        ("struct Unit;", "named struct fields"),
        ("union Either { value: u8 }", "structs and enums only"),
        ("enum Empty {}", "requires enum variants"),
        ("enum Payload { Value(u8) }", "unit enum variants only"),
        (
            "enum Payload { Value { inner: u8 } }",
            "unit enum variants only",
        ),
    ] {
        let error = expand(syn::parse_str(source).unwrap()).unwrap_err();
        assert!(error.to_string().contains(message), "{source}: {error}");
    }
}

#[test]
fn both_enum_codecs_reject_rust_discriminants_instead_of_reinterpreting_them() {
    for source in [
        "enum Numbered { First = 0, Second }",
        "enum Numbered { First = 2, Second = 5 }",
        "enum Numbered { First = 1 + 2 }",
    ] {
        for derive in [expand, expand_enum] {
            let error = derive(syn::parse_str(source).unwrap()).unwrap_err();
            assert!(error.to_string().contains("without Rust discriminants"));
        }
    }
    for derive in [expand, expand_enum] {
        assert!(derive(syn::parse_str("enum Ordinals { First, Second }").unwrap()).is_ok());
    }
}

#[test]
fn enum_representation_rejects_non_enums_and_named_payloads() {
    for (source, message) in [
        ("struct Record { value: u8 }", "requires a unit enum"),
        ("union Either { value: u8 }", "requires a unit enum"),
        ("enum Payload { Value { inner: u8 } }", "unit variants"),
        (
            "enum Borrowed<'a> { Value(&'a u8) }",
            "concrete generated type",
        ),
        (
            "enum Sized<const N: usize> { Value }",
            "concrete generated type",
        ),
    ] {
        let error = expand_enum(syn::parse_str(source).unwrap()).unwrap_err();
        assert!(error.to_string().contains(message), "{source}: {error}");
    }
}

#[test]
fn named_struct_codecs_preserve_declared_member_order_types_and_raw_names() {
    let generated = impls(
        "struct Record { z: crate::Last, r#type: crate::First }",
        false,
    );
    assert_eq!(generated.len(), 3);
    let targets: Vec<_> = generated
        .iter()
        .map(|item| {
            item.trait_
                .as_ref()
                .unwrap()
                .1
                .segments
                .last()
                .unwrap()
                .ident
                .to_string()
        })
        .collect();
    assert_eq!(targets, ["Aligned", "From", "CellValue"]);
    for item in &generated {
        assert!(item.generics.params.is_empty());
    }

    // Encoding projects the actual source members, in declaration order, not
    // alphabetically or by the lexical order of their type names.
    let encode = method(&generated[1], "from");
    let Stmt::Local(parts) = &encode.block.stmts[0] else {
        panic!("parts binding")
    };
    let Expr::Macro(values) = parts.init.as_ref().unwrap().expr.as_ref() else {
        panic!("values vector")
    };
    let values = values.mac.tokens.to_string();
    assert!(values.find("value . z").unwrap() < values.find("value . r#type").unwrap());

    let decode = method(&generated[2], "decode_cell_value");
    let Stmt::Local(decoded) = &decode.block.stmts[1] else {
        panic!("decoded binding")
    };
    let Expr::Struct(decoded) = decoded.init.as_ref().unwrap().expr.as_ref() else {
        panic!("record construction")
    };
    let fields: Vec<_> = decoded
        .fields
        .iter()
        .map(|f| f.member.to_token_stream().to_string())
        .collect();
    assert_eq!(fields, ["z", "r#type"]);
    for (field, ty) in decoded
        .fields
        .iter()
        .zip(["crate :: Last", "crate :: First"])
    {
        let expression = field.expr.to_token_stream().to_string();
        assert!(expression.contains(ty));
        assert!(expression.contains("decode_cell_value"));
        assert!(expression.contains("struct atom count differs from declared type"));
    }
    assert!(
        decode
            .block
            .to_token_stream()
            .to_string()
            .contains("struct has trailing atoms")
    );
}

#[test]
fn empty_named_struct_expansion_still_has_a_checked_decoder() {
    let declaration: syn::DeriveInput = syn::parse_str("struct Empty {}").unwrap();
    let syn::Data::Struct(data) = &declaration.data else {
        panic!()
    };
    assert!(matches!(&data.fields, Fields::Named(fields) if fields.named.is_empty()));
    let generated = impls("struct Empty {}", false);
    assert_eq!(generated.len(), 3);
    let decoder = method(&generated[2], "decode_cell_value");
    assert!(
        decoder
            .block
            .to_token_stream()
            .to_string()
            .contains("struct has trailing atoms")
    );
}

#[test]
fn enum_cell_codec_assigns_ordinals_by_declaration_order() {
    let generated = impls("enum Choice { Zebra, Alpha, r#type }", false);
    let encode = method(&generated[1], "from");
    let Stmt::Local(ordinal) = &encode.block.stmts[0] else {
        panic!("ordinal")
    };
    let Expr::Match(ordinal) = ordinal.init.as_ref().unwrap().expr.as_ref() else {
        panic!("ordinal match")
    };
    let arms: Vec<_> = ordinal
        .arms
        .iter()
        .map(|arm| {
            (
                arm.pat.to_token_stream().to_string(),
                arm.body.to_token_stream().to_string(),
            )
        })
        .collect();
    assert_eq!(
        arms,
        [
            ("Choice :: Zebra".into(), "0".into()),
            ("Choice :: Alpha".into(), "1".into()),
            ("Choice :: r#type".into(), "2".into())
        ]
    );
    let decode = method(&generated[2], "decode_cell_value");
    let Some(Stmt::Expr(Expr::Match(decoded), _)) = decode.block.stmts.last() else {
        panic!("decode match")
    };
    assert_eq!(decoded.arms.len(), 4);
    assert!(matches!(&decoded.arms[3].pat, syn::Pat::Wild(_)));
    assert!(
        decoded.arms[3]
            .body
            .to_token_stream()
            .to_string()
            .contains("enum ordinal is outside declared variants")
    );
}

#[test]
fn enum_representation_has_default_field_binary_and_checked_decode_contracts() {
    let generated = impls("enum Choice { Zebra, Alpha }", true);
    let traits: Vec<_> = generated
        .iter()
        .map(|item| {
            item.trait_
                .as_ref()
                .unwrap()
                .1
                .segments
                .last()
                .unwrap()
                .ident
                .to_string()
        })
        .collect();
    assert_eq!(
        traits,
        ["Default", "FieldRepr", "BinaryHashRepr", "FromFieldRepr"]
    );
    assert_eq!(
        method(&generated[0], "default").block.stmts[0]
            .to_token_stream()
            .to_string(),
        "Self :: Zebra"
    );
    let decode = method(&generated[3], "from_field_repr");
    let Some(Stmt::Expr(Expr::Match(decoded), _)) = decode.block.stmts.last() else {
        panic!("checked ordinal match")
    };
    assert_eq!(decoded.arms.len(), 3);
    assert!(matches!(&decoded.arms[2].pat, syn::Pat::Wild(_)));
    assert_eq!(
        decoded.arms[2].body.to_token_stream().to_string(),
        ":: core :: option :: Option :: None"
    );
}
