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
        }
        ListOperation::PopFront | ListOperation::Reset => {
            let method = if matches!(operation, ListOperation::PopFront) {
                "record_pop_front"
            } else {
                "record_reset"
            };
            let method = syn::Ident::new(method, Span::call_site());
            steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.#method(frame)?;));
        }
    }
    Ok(())
}
