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

//! Generate the fallible companion of a Compact witness trait.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, GenericParam, ItemTrait, Pat, ReturnType, TraitItem, Type};

pub(crate) fn expand(input: ItemTrait) -> syn::Result<TokenStream> {
    if input.ident != "Witnesses"
        || input.generics.params.len() != 1
        || input.generics.where_clause.is_some()
        || !input.supertraits.is_empty()
        || input.unsafety.is_some()
        || input.auto_token.is_some()
    {
        return Err(syn::Error::new_spanned(
            &input,
            "expected generated trait Witnesses<Private> without bounds",
        ));
    }
    let Some(GenericParam::Type(private)) = input.generics.params.first() else {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "expected a Private type parameter",
        ));
    };
    if private.ident != "Private" || !private.bounds.is_empty() || private.default.is_some() {
        return Err(syn::Error::new_spanned(
            private,
            "expected an unbounded Private type parameter without a default",
        ));
    }

    let visibility = &input.vis;
    let mut fallible_methods = Vec::new();
    let mut adapter_methods = Vec::new();
    for item in &input.items {
        let TraitItem::Fn(method) = item else {
            return Err(syn::Error::new_spanned(item, "expected witness method"));
        };
        if method.default.is_some()
            || !method.sig.generics.params.is_empty()
            || method.sig.generics.where_clause.is_some()
            || method.sig.constness.is_some()
            || method.sig.asyncness.is_some()
            || method.sig.unsafety.is_some()
            || method.sig.abi.is_some()
            || method.sig.variadic.is_some()
            || method.attrs.iter().any(|attr| !attr.path().is_ident("doc"))
        {
            return Err(syn::Error::new_spanned(
                method,
                "expected a generated witness method signature",
            ));
        }
        let mut inputs = method.sig.inputs.iter();
        if !matches!(inputs.next(), Some(FnArg::Receiver(receiver)) if receiver.reference.is_some() && receiver.mutability.is_none())
        {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "expected a shared &self receiver",
            ));
        }
        let mut argument_names = Vec::new();
        for input in inputs {
            let FnArg::Typed(parameter) = input else {
                return Err(syn::Error::new_spanned(input, "expected typed argument"));
            };
            let Pat::Ident(name) = parameter.pat.as_ref() else {
                return Err(syn::Error::new_spanned(
                    &parameter.pat,
                    "expected named witness argument",
                ));
            };
            if name.by_ref.is_some() || name.mutability.is_some() || name.subpat.is_some() {
                return Err(syn::Error::new_spanned(
                    name,
                    "expected a plain witness argument name",
                ));
            }
            argument_names.push(&name.ident);
        }
        if !argument_names
            .first()
            .is_some_and(|name| *name == "context")
        {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "expected a named context argument",
            ));
        }
        let ReturnType::Type(_, output) = &method.sig.output else {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "expected a witness output type",
            ));
        };
        let Type::Tuple(pair) = output.as_ref() else {
            return Err(syn::Error::new_spanned(
                output,
                "expected a (Private, Value) witness output",
            ));
        };
        if pair.elems.len() != 2
            || !matches!(pair.elems.first(), Some(Type::Path(path)) if path.qself.is_none() && path.path.is_ident("Private"))
        {
            return Err(syn::Error::new_spanned(
                output,
                "expected a (Private, Value) witness output",
            ));
        }
        let mut fallible_signature = method.sig.clone();
        fallible_signature.output = syn::parse_quote!(-> Result<#output, runtime::CompactError>);
        let name = &method.sig.ident;
        let attrs = &method.attrs;
        fallible_methods.push(quote! { #(#attrs)* #fallible_signature; });
        adapter_methods.push(quote! {
            #fallible_signature {
                Ok(<W as Witnesses<Private>>::#name(self, #(#argument_names),*))
            }
        });
    }

    Ok(quote! {
        #input
        /// Witness methods that can propagate ledger projection failures.
        #visibility trait TryWitnesses<Private> {
            #(#fallible_methods)*
        }
        impl<Private, W: Witnesses<Private>> TryWitnesses<Private> for W {
            #(#adapter_methods)*
        }
    })
}

#[cfg(test)]
mod tests {
    use super::expand;
    use syn::{ItemTrait, parse_quote};

    #[test]
    fn rejects_bounded_and_defaulted_private_state() {
        let bounded: ItemTrait = parse_quote!(
            pub trait Witnesses<Private: Clone> {}
        );
        assert!(expand(bounded).is_err());
        let defaulted: ItemTrait = parse_quote!(
            pub trait Witnesses<Private = ()> {}
        );
        assert!(expand(defaulted).is_err());
    }

    #[test]
    fn rejects_unsupported_witness_method_signatures() {
        for input in [
            "pub trait Witnesses<Private> { const fn secret(&self, context: u64) -> (Private, ()); }",
            "pub trait Witnesses<Private> { extern \"C\" fn secret(&self, context: u64) -> (Private, ()); }",
            "pub trait Witnesses<Private> { fn secret(&self, context: u64) -> (Private, ()) where Private: Clone; }",
            "pub trait Witnesses<Private> { fn secret(&self, seed: u64) -> (Private, ()); }",
            "pub trait Witnesses<Private> { fn secret(&self, context: u64) -> u64; }",
        ] {
            let trait_item: ItemTrait = syn::parse_str(input).unwrap();
            assert!(expand(trait_item).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn preserves_method_docs_on_both_public_traits() {
        let input: ItemTrait = parse_quote! {
            pub trait Witnesses<Private> {
                /// Reads the secret value.
                fn secret(&self, context: u64) -> (Private, u64);
            }
        };
        let source = expand(input).unwrap().to_string();
        assert_eq!(source.matches("Reads the secret value.").count(), 2);
    }
}
