// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Checked conversion plans and original Map/Fold runtime evaluation order.
use crate::{
    naming::checked_ident,
    pure_semantics::{coercion::CoercionPlan, identifier::CheckedName},
    value_lowering::coercion,
};
use proc_macro2::Span;

pub(super) fn map(
    source: syn::Expr,
    plan: &CoercionPlan,
    item_name: syn::Ident,
    body: syn::Expr,
    _length: usize,
) -> syn::Expr {
    let source = coercion::render(plan, source, 0);
    let mut expression: syn::ExprTry = syn::parse_quote!(
        (__compact_source_placeholder).try_map(
            |#item_name| -> Result<_, runtime::CompactError> {
                Ok(__compact_body_placeholder)
            }
        )?
    );
    // Replace fixed template slots without revisiting already checked children.
    // The source is evaluated once before the closure; body failures propagate
    // through try_map and then through the surrounding circuit's Result.
    let syn::Expr::MethodCall(call) = expression.expr.as_mut() else {
        unreachable!("Map method template");
    };
    let syn::Expr::Paren(receiver) = call.receiver.as_mut() else {
        unreachable!("Map receiver template");
    };
    *receiver.expr = source;
    let syn::Expr::Closure(callback) = call.args.first_mut().expect("Map callback slot") else {
        unreachable!("Map closure template");
    };
    let syn::Expr::Block(block) = callback.body.as_mut() else {
        unreachable!("Map closure body template");
    };
    block.block.stmts[0] = syn::Stmt::Expr(map_result(body), None);
    syn::Expr::Try(expression)
}
pub(super) fn fold(
    function: CheckedName,
    initial: syn::Expr,
    initial_plan: &CoercionPlan,
    source: syn::Expr,
    source_plan: &CoercionPlan,
) -> syn::Expr {
    let initial = coercion::render(initial_plan, initial, 0);
    let source = coercion::render(source_plan, source, 0);
    let function = checked_ident(&function);
    let acc = syn::Ident::new("__compact_fold_accumulator", Span::call_site());
    let source_name = syn::Ident::new("__compact_fold_source", Span::call_site());
    // Keep both initializations before iterator construction. In particular, an
    // initial-value failure must drop the original source before any iteration.
    syn::parse_quote!({
        let #source_name = #source;
        let #acc = #initial;
        #source_name.into_array().into_iter().try_fold(
            #acc, crate::pure_circuits::#function
        )?
    })
}

/// Checked native fallible children already return CompactError, as does this
/// explicit Map closure. Peel only its unannotated outer question mark; nested
/// calls, question marks, blocks and attributes remain in their original scope.
fn map_result(body: syn::Expr) -> syn::Expr {
    match body {
        syn::Expr::Try(fallible) if fallible.attrs.is_empty() => *fallible.expr,
        other => {
            let mut result: syn::ExprCall = syn::parse_quote!(Ok(__compact_body_placeholder));
            *result.args.first_mut().expect("Map result argument slot") = other;
            syn::Expr::Call(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::map_result;
    use quote::ToTokens;

    #[test]
    fn map_result_peels_only_unannotated_outer_try_and_keeps_nested_evaluation() {
        let inner: syn::Expr = syn::parse_quote!(runtime::cast_unsigned::<15, 255>(
            crate::pure_circuits::step(crate::pure_circuits::input()?)?
        ));
        let outer: syn::Expr = syn::parse_quote!(#inner?);
        assert_eq!(
            map_result(outer).to_token_stream().to_string(),
            inner.to_token_stream().to_string()
        );
        for body in [
            syn::parse_quote!(plain_value()),
            syn::parse_quote!((fallible()?)),
            syn::parse_quote!({
                let value = fallible()?;
                value
            }),
            syn::parse_quote!(
                #[allow(unused_parens)]
                fallible()?
            ),
        ] {
            let expected: syn::Expr = syn::parse_quote!(Ok(#body));
            assert_eq!(
                map_result(body).to_token_stream().to_string(),
                expected.to_token_stream().to_string()
            );
        }
    }
}
