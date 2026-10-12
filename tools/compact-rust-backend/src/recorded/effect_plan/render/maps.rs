// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Checked Map operations call the existing typed runtime slots.
use super::*;

pub(super) fn render(
    binding: &Binding,
    declarations: &Declarations,
    steps: &mut Vec<syn::Stmt>,
) -> Result<(), RecordingGap> {
    let (slot, arguments, method): (_, Vec<syn::Expr>, _) = match &binding.operation {
        Operation::MapMember { slot, key } => (slot, vec![value(key)], "record_member"),
        Operation::MapLookup { slot, key } => (slot, vec![value(key)], "record_lookup"),
        Operation::MapInsert {
            slot,
            key,
            value: input,
        } => (slot, vec![value(key), value(input)], "record_insert"),
        Operation::MapRemove { slot, key } => (slot, vec![value(key)], "record_remove"),
        Operation::MapInsertDefault { slot, key } => {
            (slot, vec![value(key)], "record_insert_default")
        }
        Operation::MapReset { slot } => (slot, vec![], "record_reset"),
        Operation::MapSize { slot } => (slot, vec![], "record_size"),
        Operation::MapIsEmpty { slot } => (slot, vec![], "record_is_empty"),
        _ => unreachable!("checked Map rendering"),
    };
    let slot = slot_name(declarations, slot.declaration, &binding.path)?;
    let method = ident(method).map_err(|e| invalid(&binding.path, e.to_string()))?;
    let output = name(&binding.value);
    let ty = rust_type(&binding.value.ty).map_err(|e| invalid(&binding.path, e.to_string()))?;
    match &binding.operation {
        Operation::MapSize { .. } => {
            let raw = syn::Ident::new(
                &format!("__compact_map_size_{}", binding.value.id),
                Span::call_site(),
            );
            steps.push(syn::parse_quote!(let (frame, #raw) = crate::ledger_slots::#slot.record_size(frame)?;));
            steps.push(syn::parse_quote!(let #output: #ty = <#ty>::new(#raw as u128)?;));
        }
        Operation::MapMember { .. }
        | Operation::MapLookup { .. }
        | Operation::MapIsEmpty { .. } => {
            steps.push(syn::parse_quote!(let (frame, #output): (_, #ty) = crate::ledger_slots::#slot.#method(frame #(, #arguments)*)?;));
        }
        _ => {
            steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.#method(frame #(, #arguments)*)?;));
            steps.push(syn::parse_quote!(let #output: #ty = ();));
        }
    }
    Ok(())
}
