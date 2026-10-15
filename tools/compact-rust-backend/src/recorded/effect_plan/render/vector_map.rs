// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Emit one loop; code size does not scale with the vector cardinality.
use super::*;

pub(super) fn render(
    binding: &Binding,
    declarations: &Declarations,
) -> Result<syn::Stmt, RecordingGap> {
    let Operation::VectorMap {
        source,
        parameter,
        body,
    } = &binding.operation
    else {
        return Err(invalid(&binding.path, "expected VectorMap operation"));
    };
    let crate::ir::Type::Vector { length, .. } = &binding.value.ty else {
        return Err(invalid(&binding.path, "expected VectorMap result vector"));
    };
    let output = name(&binding.value);
    let ty = rust_type(&binding.value.ty).map_err(|e| invalid(&binding.path, e.to_string()))?;
    let input = value(source);
    let formal = name(parameter);
    let body_steps = block(body, declarations)?;
    let body_value = value(&body.result);
    let length = syn::LitInt::new(&length.to_string(), Span::call_site());
    // Pure If syntax threads frame ownership. Carry it without observing or
    // mutating it; independent validation excludes every context effect here.
    Ok(syn::parse_quote! {
        let (frame, #output): (_, #ty) = {
            let __compact_map_source = #input;
            let mut __compact_mapped = Vec::with_capacity(#length);
            let mut __compact_map_frame = frame;
            for #formal in __compact_map_source.into_array() {
                let frame = __compact_map_frame;
                #(#body_steps)*
                __compact_mapped.push(#body_value);
                __compact_map_frame = frame;
            }
            (__compact_map_frame, runtime::FixedVector::new(
                <[_; #length]>::try_from(__compact_mapped).expect("Vector map preserves its length")
            ))
        };
    })
}
