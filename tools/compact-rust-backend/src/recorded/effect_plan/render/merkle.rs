// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Checked tree effects delegate exclusively to the runtime's typed MerkleSlot.
use super::*;

pub(super) fn render(
    binding: &Binding,
    slot: MerkleSlot,
    operation: &MerkleOperation,
    declarations: &Declarations,
    steps: &mut Vec<syn::Stmt>,
) -> Result<(), RecordingGap> {
    let path = &binding.path;
    let slot = slot_name(declarations, slot.declaration, path)?;
    let output = name(&binding.value);
    let ty = rust_type(&binding.value.ty).map_err(|error| invalid(path, error.to_string()))?;
    let (method, arguments): (&str, Vec<syn::Expr>) = match operation {
        MerkleOperation::Insert {
            value: input,
            position: None,
        } => ("record_insert", vec![value(input)]),
        MerkleOperation::Insert {
            value: input,
            position: Some(position),
        } => ("record_insert_index", vec![value(input), value(position)]),
        MerkleOperation::InsertDefault { position } => {
            ("record_insert_index_default", vec![value(position)])
        }
        MerkleOperation::InsertHash {
            hash,
            position: None,
        } => ("record_insert_hash", vec![value(hash)]),
        MerkleOperation::InsertHash {
            hash,
            position: Some(position),
        } => (
            "record_insert_hash_index",
            vec![value(hash), value(position)],
        ),
        MerkleOperation::ResetDefault => ("record_reset_to_default", vec![]),
        MerkleOperation::ResetHistory => ("record_reset_history", vec![]),
        MerkleOperation::CheckRoot { root } => ("record_check_root", vec![value(root)]),
        MerkleOperation::IsFull => ("record_is_full", vec![]),
    };
    let method = ident(method).map_err(|error| invalid(path, error.to_string()))?;
    if matches!(
        operation,
        MerkleOperation::CheckRoot { .. } | MerkleOperation::IsFull
    ) {
        steps.push(syn::parse_quote!(let (frame, #output): (_, #ty) = crate::ledger_slots::#slot.#method(frame, #(#arguments),*)?;));
    } else {
        steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.#method(frame, #(#arguments),*)?;));
        steps.push(syn::parse_quote!(let #output: #ty = ();));
    }
    Ok(())
}
