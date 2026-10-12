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
    let (slot, key, method) = match &binding.operation {
        Operation::MapMember { slot, key } => (slot, key, "record_member"),
        Operation::MapLookup { slot, key } => (slot, key, "record_lookup"),
        Operation::MapInsert { slot, key, .. } => (slot, key, "record_insert"),
        Operation::MapRemove { slot, key } => (slot, key, "record_remove"),
        _ => unreachable!("checked Map rendering"),
    };
    let slot = slot_name(declarations, slot.declaration, &binding.path)?;
    let method = ident(method).map_err(|e| invalid(&binding.path, e.to_string()))?;
    let key = value(key);
    let output = name(&binding.value);
    let ty = rust_type(&binding.value.ty).map_err(|e| invalid(&binding.path, e.to_string()))?;
    match &binding.operation {
        Operation::MapMember { .. } | Operation::MapLookup { .. } => {
            steps.push(syn::parse_quote!(let (frame, #output): (_, #ty) = crate::ledger_slots::#slot.#method(frame, #key)?;));
        }
        operation => {
            if let Operation::MapInsert { value: input, .. } = operation {
                let input = value(input);
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.#method(frame, #key, #input)?;));
            } else {
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.#method(frame, #key)?;));
            }
            steps.push(syn::parse_quote!(let #output: #ty = ();));
        }
    }
    Ok(())
}
