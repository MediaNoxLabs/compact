// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Typed List effects reuse the existing runtime slot API and frame sequencing.
use super::*;

pub(super) fn render(
    binding: &Binding,
    slot: ListSlot,
    operation: &ListOperation,
    declarations: &Declarations,
    steps: &mut Vec<syn::Stmt>,
) -> Result<(), RecordingGap> {
    let slot = slot_name(declarations, slot.declaration, &binding.path)?;
    let output = name(&binding.value);
    let ty = rust_type(&binding.value.ty).map_err(|e| invalid(&binding.path, e.to_string()))?;
    match operation {
        ListOperation::Length => {
            let raw = syn::Ident::new(
                &format!("__compact_list_length_{}", binding.value.id),
                Span::call_site(),
            );
            steps.push(syn::parse_quote!(let (frame, #raw) = crate::ledger_slots::#slot.record_length(frame)?;));
            steps.push(syn::parse_quote!(let #output: #ty = <#ty>::new(#raw as u128)?;));
        }
        ListOperation::IsEmpty => {
            steps.push(syn::parse_quote!(let (frame, #output): (_, #ty) = crate::ledger_slots::#slot.record_is_empty(frame)?;));
        }
        ListOperation::Head => {
            steps.push(syn::parse_quote!(let (frame, #output): (_, #ty) = crate::ledger_slots::#slot.record_head::<#ty, _, _>(frame)?;));
        }
        ListOperation::PushFront(input) => {
            let input = value(input);
            steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_push_front(frame, #input)?;));
            steps.push(syn::parse_quote!(let #output: #ty = ();));
        }
        ListOperation::PopFront | ListOperation::Reset => {
            let method = if matches!(operation, ListOperation::PopFront) {
                "record_pop_front"
            } else {
                "record_reset"
            };
            let method = syn::Ident::new(method, Span::call_site());
            steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.#method(frame)?;));
            steps.push(syn::parse_quote!(let #output: #ty = ();));
        }
    }
    Ok(())
}
