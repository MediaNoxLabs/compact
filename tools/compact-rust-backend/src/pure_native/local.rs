// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Output-only Let symbols from an accepted word and live native namespace.
//! Raw source keys retain their lookup identity. Same-key shadowing is allowed;
//! distinct keys with equal Rust spellings must keep separate output symbols.
use crate::pure_semantics::identifier::CheckedName;
use std::collections::HashSet;

pub(crate) fn allocate<'a>(
    checked_name: &CheckedName,
    raw_source_key: &str,
    occupied: impl IntoIterator<Item = (&'a str, &'a syn::Ident)>,
) -> syn::Ident {
    let occupied: HashSet<String> = occupied
        .into_iter()
        .filter(|(key, _)| *key != raw_source_key)
        .map(|(_, name)| {
            let spelling = name.to_string();
            spelling.strip_prefix("r#").unwrap_or(&spelling).to_owned()
        })
        .collect();
    let base = format!("__compact_local_{}", checked_name.text());
    let mut candidate = base.clone();
    let mut suffix = 1_usize;
    while occupied.contains(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    // Checked word text plus this fixed prefix is always an ordinary Rust
    // identifier, including source keywords/raw/trivia/dollar spellings.
    syn::Ident::new(&candidate, proc_macro2::Span::call_site())
}

#[cfg(test)]
mod tests;
