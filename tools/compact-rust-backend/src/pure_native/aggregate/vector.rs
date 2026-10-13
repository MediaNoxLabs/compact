// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

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
    length: usize,
) -> syn::Expr {
    let source = coercion::render(plan, source, 0);
    let mapped = syn::Ident::new("__compact_mapped", Span::call_site());
    let source_name = syn::Ident::new("__compact_map_source", Span::call_site());
    let length_lit = syn::LitInt::new(&length.to_string(), Span::call_site());
    let mut block: syn::ExprBlock = syn::parse_quote!({
        let #source_name = __compact_source_placeholder;
        let mut #mapped = Vec::with_capacity(#length_lit);
        for #item_name in #source_name.into_array() {
            #mapped.push(__compact_body_placeholder);
        }
        runtime::FixedVector::new(<[_; #length_lit]>::try_from(#mapped).expect("Vector map preserves its length"))
    });
    // Fixed local template slots; never walk/reparse already checked children.
    let syn::Stmt::Local(local) = &mut block.block.stmts[0] else {
        unreachable!("Map source initializer template");
    };
    *local
        .init
        .as_mut()
        .expect("Map source initializer slot")
        .expr = source;
    let syn::Stmt::Expr(syn::Expr::ForLoop(mapped_loop), _) = &mut block.block.stmts[2] else {
        unreachable!("Map loop template");
    };
    let syn::Stmt::Expr(syn::Expr::MethodCall(push), _) = &mut mapped_loop.body.stmts[0] else {
        unreachable!("Map push template");
    };
    *push.args.first_mut().expect("Map body argument slot") = body;
    syn::Expr::Block(block)
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
    let item = syn::Ident::new("__compact_fold_item", Span::call_site());
    let source_name = syn::Ident::new("__compact_fold_source", Span::call_site());
    syn::parse_quote!({
        let #source_name = #source;
        let mut #acc = #initial;
        for #item in #source_name.into_array() {
            #acc = crate::pure_circuits::#function(#acc, #item)?;
        }
        #acc
    })
}
