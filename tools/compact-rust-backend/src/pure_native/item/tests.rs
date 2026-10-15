// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact old/new prepared item syntax; never replay the old parser at crash depth.
use super::*;
use crate::ir::Type;
use quote::ToTokens;
mod arguments;
mod file_assembly;
#[path = "old_reference.rs"]
mod old;
mod tails;

fn compare(
    name: syn::Ident,
    internal: bool,
    arguments: Vec<syn::FnArg>,
    result: syn::Type,
    body: NativeFragment,
    actual: Type,
) {
    let (expression, statements) = body.into_views();
    let previous = old::pure(
        name.clone(),
        internal,
        arguments.clone(),
        result.clone(),
        NativeFragment::unit(expression.clone(), statements.clone()),
        actual.clone(),
    );
    let current = pure(
        name,
        internal,
        arguments,
        result,
        NativeFragment::unit(expression, statements),
        actual == Type::Unit,
    );
    assert_eq!(
        current.to_token_stream().to_string(),
        previous.to_token_stream().to_string()
    );
    let current = syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![current],
    };
    let previous = syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![previous],
    };
    assert_eq!(
        prettyplease::unparse(&current),
        prettyplease::unparse(&previous)
    );
}
