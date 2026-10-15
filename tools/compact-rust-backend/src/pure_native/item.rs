// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Pure function assembly without reparsing prepared carrier/body syntax.
//! All source names, Types and body rules are checked by the calling owner.
//! Only shallow fixed templates are parsed; prepared AST children move directly.
use super::fragment::NativeFragment;

pub(crate) fn argument(name: syn::Ident, carrier: syn::Type) -> syn::FnArg {
    syn::FnArg::Typed(syn::PatType {
        attrs: vec![],
        pat: Box::new(syn::Pat::Ident(syn::PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: None,
            ident: name,
            subpat: None,
        })),
        colon_token: syn::token::Colon::default(),
        ty: Box::new(carrier),
    })
}

pub(crate) fn pure(
    name: syn::Ident,
    internal: bool,
    arguments: Vec<syn::FnArg>,
    result: syn::Type,
    body: NativeFragment,
    unit_result: bool,
) -> syn::Item {
    let mut item: syn::ItemFn = syn::parse_quote! {
        pub fn __compact_function() -> Result<(), runtime::CompactError> {}
    };
    item.sig.ident = name;
    if internal {
        item.vis = syn::parse_quote!(pub(crate));
    }
    item.sig.inputs = arguments.into_iter().collect();
    insert_result(&mut item.sig.output, result);
    let (expression, mut statements) = body.into_views();
    item.block.stmts = if unit_result {
        statements.push(syn::Stmt::Expr(syn::parse_quote!(Ok(())), None));
        statements
    } else {
        vec![syn::Stmt::Expr(value_tail(expression), None)]
    };
    syn::Item::Fn(item)
}

fn insert_result(output: &mut syn::ReturnType, result: syn::Type) {
    // These shapes belong to the fixed local template, never to source input.
    let syn::ReturnType::Type(_, ty) = output else {
        unreachable!("pure function result template");
    };
    let syn::Type::Path(path) = ty.as_mut() else {
        unreachable!("pure function Result path template");
    };
    let syn::PathArguments::AngleBracketed(arguments) = &mut path.path.segments[0].arguments else {
        unreachable!("pure function Result arguments template");
    };
    *arguments
        .args
        .first_mut()
        .expect("pure function Result value slot") = syn::GenericArgument::Type(result);
}

fn value_tail(expression: syn::Expr) -> syn::Expr {
    match expression {
        // Return an already fallible result directly, retaining the old peeling
        // rule while moving its child instead of cloning/reparsing it.
        syn::Expr::Try(fallible) => *fallible.expr,
        other => {
            let mut call: syn::ExprCall = syn::parse_quote!(Ok(__compact_return_placeholder));
            *call.args.first_mut().expect("Ok value template slot") = other;
            syn::Expr::Call(call)
        }
    }
}

/// Assemble prepared modules without reparsing their items or type/body trees.
pub(crate) fn file(
    runtime_abi: syn::LitInt,
    types_module: Option<syn::Item>,
    alias_exports: Option<syn::Item>,
    pure_items: Vec<syn::Item>,
    slots_module: Option<syn::Item>,
    ledger_module: Option<syn::Item>,
) -> syn::File {
    let mut file: syn::File = syn::parse_quote! {
        /// The matching Midnight Compact Rust runtime used by this generated crate.
        pub use midnight_compact_runtime as runtime;
        #[allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables)]
        pub mod pure_circuits {
            use midnight_compact_runtime as runtime;
            const _: () = assert!(runtime::RUST_RUNTIME_ABI == #runtime_abi);
        }
    };
    let mut pure_module = file.items.pop().expect("pure module template slot");
    let syn::Item::Mod(module) = &mut pure_module else {
        unreachable!("pure module template");
    };
    module
        .content
        .as_mut()
        .expect("inline pure module template")
        .1
        .extend(pure_items);
    file.items.extend(types_module);
    file.items.extend(alias_exports);
    file.items.push(pure_module);
    file.items.extend(slots_module);
    file.items.extend(ledger_module);
    file
}

#[cfg(test)]
mod tests;
