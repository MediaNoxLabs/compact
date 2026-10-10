// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Checked SSA values become official coin inputs to the declared storage slot.
use super::{Value, name};

pub(super) fn write(slot: &syn::Ident, coin: &Value, recipient: &Value) -> syn::Stmt {
    storage_operation(slot, &syn::parse_quote!(record_write_coin), coin, recipient)
}

pub(super) fn insert(slot: &syn::Ident, coin: &Value, recipient: &Value) -> syn::Stmt {
    storage_operation(
        slot,
        &syn::parse_quote!(record_insert_coin),
        coin,
        recipient,
    )
}

fn storage_operation(
    slot: &syn::Ident,
    method: &syn::Ident,
    coin: &Value,
    recipient: &Value,
) -> syn::Stmt {
    let (coin, recipient) = (name(coin), name(recipient));
    // Only Copy fields are read; both aggregates remain available for later uses.
    // The slot/frame resolves the recipient-bound commitment and allocated index.
    syn::parse_quote! {
        let frame = crate::ledger_slots::#slot.#method(
            frame,
            runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()),
            runtime::ledger::coin_recipient_from_compact(
                #recipient.is_left, #recipient.left.bytes, #recipient.right.bytes
            )
        )?;
    }
}
