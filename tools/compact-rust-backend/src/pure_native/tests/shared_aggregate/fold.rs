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

//! Translate only the exact frozen loop template; all children remain compared.
use super::*;

pub(super) fn successor(tokens: &str) -> String {
    let mut expression: syn::Expr = syn::parse_str(tokens).unwrap();
    syn::visit_mut::VisitMut::visit_expr_mut(&mut LegacyFoldTemplate, &mut expression);
    expression.to_token_stream().to_string()
}
struct LegacyFoldTemplate;
impl syn::visit_mut::VisitMut for LegacyFoldTemplate {
    fn visit_expr_mut(&mut self, expression: &mut syn::Expr) {
        syn::visit_mut::visit_expr_mut(self, expression);
        let syn::Expr::Block(block) = expression else {
            return;
        };
        let [
            syn::Stmt::Local(source),
            syn::Stmt::Local(initial),
            syn::Stmt::Expr(syn::Expr::ForLoop(body), _),
            _,
        ] = block.block.stmts.as_slice()
        else {
            return;
        };
        let (Some(source), Some(initial)) = (&source.init, &initial.init) else {
            return;
        };
        let (source, initial) = (&source.expr, &initial.expr);
        let [syn::Stmt::Expr(syn::Expr::Assign(assign), _)] = body.body.stmts.as_slice() else {
            return;
        };
        let syn::Expr::Try(callback) = assign.right.as_ref() else {
            return;
        };
        let syn::Expr::Call(callback) = callback.expr.as_ref() else {
            return;
        };
        let function = &callback.func;
        let expected: syn::Expr = syn::parse_quote!({
            let __compact_fold_source = #source;
            let mut __compact_fold_accumulator = #initial;
            for __compact_fold_item in __compact_fold_source.into_array() {
                __compact_fold_accumulator = #function(__compact_fold_accumulator, __compact_fold_item)?;
            }
            __compact_fold_accumulator
        });
        if block.to_token_stream().to_string() != expected.to_token_stream().to_string() {
            return;
        }
        // A callback must be the generated pure function path, not a replacement expression.
        let syn::Expr::Path(path) = function.as_ref() else {
            return;
        };
        if path.qself.is_some()
            || path.path.leading_colon.is_some()
            || path.path.segments.len() != 3
            || path.path.segments[0].ident != "crate"
            || path.path.segments[1].ident != "pure_circuits"
            || path
                .path
                .segments
                .iter()
                .any(|part| !matches!(part.arguments, syn::PathArguments::None))
        {
            return;
        }
        *expression = syn::parse_quote!({
            let __compact_fold_source = #source;
            let __compact_fold_accumulator = #initial;
            __compact_fold_source.into_array().into_iter().try_fold(__compact_fold_accumulator, #function)?
        });
    }
}

#[test]
fn exact_fold_successor_preserves_children_and_refuses_changed_loop_semantics() {
    let original = quote::quote!({
        let __compact_fold_source = source()?;
        let mut __compact_fold_accumulator = initial()?;
        for __compact_fold_item in __compact_fold_source.into_array() {
            __compact_fold_accumulator =
                crate::pure_circuits::step(__compact_fold_accumulator, __compact_fold_item)?;
        }
        __compact_fold_accumulator
    })
    .to_string();
    let expected = quote::quote!({
        let __compact_fold_source = source()?;
        let __compact_fold_accumulator = initial()?;
        __compact_fold_source
            .into_array()
            .into_iter()
            .try_fold(__compact_fold_accumulator, crate::pure_circuits::step)?
    })
    .to_string();
    assert_eq!(successor(&original), expected);
    for changed in [
        original.replace(
            "__compact_fold_accumulator , __compact_fold_item",
            "__compact_fold_item , __compact_fold_accumulator",
        ),
        original.replace("into_array", "reversed"),
        original.replace(
            "crate :: pure_circuits :: step",
            "other :: pure_circuits :: step",
        ),
    ] {
        assert_ne!(changed, original);
        assert_eq!(successor(&changed), changed);
    }
}
