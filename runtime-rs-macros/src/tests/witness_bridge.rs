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

//! Signature rejection and structural contracts of the witness bridge.
use super::expand;
use quote::ToTokens;
use syn::{Expr, ImplItem, Item, ItemTrait, Stmt, TraitItem};

fn rejects(source: &str, expected: &str) {
    let input: ItemTrait =
        syn::parse_str(source).expect("malformed-contract probe must parse as Rust syntax");
    let error = expand(input).unwrap_err();
    assert!(error.to_string().contains(expected), "{source}: {error}");
}

#[test]
fn rejects_trait_identity_bounds_and_non_type_private_parameters() {
    for source in [
        "trait Other<Private> {}",
        "trait Witnesses {}",
        "trait Witnesses<Private, Extra> {}",
        "trait Witnesses<Private>: Clone {}",
        "trait Witnesses<Private> where Private: Clone {}",
        "unsafe trait Witnesses<Private> {}",
        "auto trait Witnesses<Private> {}",
    ] {
        rejects(
            source,
            "expected generated trait Witnesses<Private> without bounds",
        );
    }
    for source in [
        "trait Witnesses<'a> {}",
        "trait Witnesses<const N: usize> {}",
    ] {
        rejects(source, "expected a Private type parameter");
    }
    rejects(
        "trait Witnesses<State> {}",
        "expected an unbounded Private type parameter",
    );
}

#[test]
fn rejects_associated_items_and_executable_or_modified_methods() {
    for source in [
        "trait Witnesses<Private> { type Value; }",
        "trait Witnesses<Private> { const VALUE: u64; }",
    ] {
        rejects(source, "expected witness method");
    }
    for method in [
        "fn read(&self, context: u64) -> (Private, u64) { loop {} }",
        "async fn read(&self, context: u64) -> (Private, u64);",
        "unsafe fn read(&self, context: u64) -> (Private, u64);",
        "fn read<T>(&self, context: T) -> (Private, u64);",
        "fn read<'a>(&self, context: &'a u64) -> (Private, u64);",
        "#[cfg(any())] fn read(&self, context: u64) -> (Private, u64);",
        "#[allow(dead_code)] fn read(&self, context: u64) -> (Private, u64);",
        "fn read(&self, context: u64, ...) -> (Private, u64);",
    ] {
        rejects(
            &format!("trait Witnesses<Private> {{ {method} }}"),
            "expected a generated witness method signature",
        );
    }
}

#[test]
fn rejects_missing_owned_and_mutable_receivers() {
    for method in [
        "fn read(context: u64) -> (Private, u64);",
        "fn read(self, context: u64) -> (Private, u64);",
        "fn read(&mut self, context: u64) -> (Private, u64);",
        "fn read() -> (Private, u64);",
    ] {
        rejects(
            &format!("trait Witnesses<Private> {{ {method} }}"),
            "expected a shared &self receiver",
        );
    }
}

#[test]
fn rejects_patterns_that_cannot_be_forwarded_as_plain_arguments() {
    for pattern in ["(left, right)", "_", "[first, second]"] {
        rejects(
            &format!(
                "trait Witnesses<Private> {{ fn read(&self, context: u64, {pattern}: (u64,u64)) -> (Private, ()); }}"
            ),
            "expected named witness argument",
        );
    }
    for pattern in ["ref value", "mut value", "value @ _"] {
        rejects(
            &format!(
                "trait Witnesses<Private> {{ fn read(&self, context: u64, {pattern}: u64) -> (Private, ()); }}"
            ),
            "expected a plain witness argument name",
        );
    }
    rejects(
        "trait Witnesses<Private> { fn read(&self) -> (Private, ()); }",
        "expected a named context argument",
    );
}

#[test]
fn rejects_absent_or_non_private_pair_results() {
    rejects(
        "trait Witnesses<Private> { fn read(&self, context: u64); }",
        "expected a witness output type",
    );
    for result in [
        "()",
        "(Private,)",
        "(Private, u64, bool)",
        "(u64, Private)",
        "(crate::Private, u64)",
        "u64",
    ] {
        rejects(
            &format!("trait Witnesses<Private> {{ fn read(&self, context: u64) -> {result}; }}"),
            "expected a (Private, Value) witness output",
        );
    }
}

#[test]
fn valid_bridge_preserves_visibility_signatures_and_argument_order() {
    let original: ItemTrait = syn::parse_quote! {
        pub(crate) trait Witnesses<Private> {
            /// First operation.
            fn r#type(&self, context: crate::Context<Private>, z: u16, a: crate::Value) -> (Private, crate::Value);
            fn observe(&self, context: crate::Context<Private>) -> (Private, ());
        }
    };
    let expected_original = original.to_token_stream().to_string();
    let file = syn::parse2::<syn::File>(expand(original.clone()).unwrap()).unwrap();
    assert_eq!(file.items.len(), 3);
    let Item::Trait(preserved) = &file.items[0] else {
        panic!("original trait")
    };
    assert_eq!(preserved.to_token_stream().to_string(), expected_original);
    let Item::Trait(fallible) = &file.items[1] else {
        panic!("fallible trait")
    };
    assert_eq!(fallible.ident, "TryWitnesses");
    assert_eq!(
        fallible.vis.to_token_stream().to_string(),
        original.vis.to_token_stream().to_string()
    );
    assert_eq!(fallible.items.len(), original.items.len());
    let Item::Impl(adapter) = &file.items[2] else {
        panic!("blanket adapter")
    };
    assert_eq!(adapter.items.len(), original.items.len());
    for ((source, output), forwarded) in original
        .items
        .iter()
        .zip(&fallible.items)
        .zip(&adapter.items)
    {
        let (TraitItem::Fn(source), TraitItem::Fn(output), ImplItem::Fn(forwarded)) =
            (source, output, forwarded)
        else {
            panic!("methods")
        };
        assert_eq!(source.sig.ident, output.sig.ident);
        assert_eq!(
            source.sig.inputs.to_token_stream().to_string(),
            output.sig.inputs.to_token_stream().to_string()
        );
        assert_eq!(
            output.sig.to_token_stream().to_string(),
            forwarded.sig.to_token_stream().to_string()
        );
        let syn::ReturnType::Type(_, output_type) = &output.sig.output else {
            panic!()
        };
        let syn::Type::Path(result) = output_type.as_ref() else {
            panic!()
        };
        let syn::PathArguments::AngleBracketed(arguments) =
            &result.path.segments.last().unwrap().arguments
        else {
            panic!()
        };
        assert_eq!(arguments.args.len(), 2);
        let syn::ReturnType::Type(_, original_output) = &source.sig.output else {
            panic!()
        };
        assert_eq!(
            arguments.args[0].to_token_stream().to_string(),
            original_output.to_token_stream().to_string()
        );
        assert_eq!(
            arguments.args[1].to_token_stream().to_string(),
            ":: midnight_compact_runtime :: CompactError"
        );
    }
    let ImplItem::Fn(first) = &adapter.items[0] else {
        panic!()
    };
    let Stmt::Expr(Expr::Call(ok), _) = &first.block.stmts[0] else {
        panic!("successful adapter result")
    };
    let Expr::Call(call) = &ok.args[0] else {
        panic!("forwarded method call")
    };
    let actual: Vec<_> = call
        .args
        .iter()
        .map(|arg| arg.to_token_stream().to_string())
        .collect();
    assert_eq!(actual, ["self", "context", "z", "a"]);
    assert!(call.func.to_token_stream().to_string().contains("r#type"));
}

#[test]
fn empty_private_trait_produces_empty_companion_and_blanket_impl() {
    let input: ItemTrait = syn::parse_quote!(
        trait Witnesses<Private> {}
    );
    let file = syn::parse2::<syn::File>(expand(input).unwrap()).unwrap();
    assert_eq!(file.items.len(), 3);
    let Item::Trait(companion) = &file.items[1] else {
        panic!()
    };
    assert!(matches!(companion.vis, syn::Visibility::Inherited));
    assert!(companion.items.is_empty());
    let Item::Impl(adapter) = &file.items[2] else {
        panic!()
    };
    assert!(adapter.items.is_empty());
    assert_eq!(adapter.generics.params.len(), 2);
    assert_eq!(
        adapter
            .trait_
            .as_ref()
            .unwrap()
            .1
            .segments
            .last()
            .unwrap()
            .ident,
        "TryWitnesses"
    );
}
