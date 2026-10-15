// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen complete helper policies independently authenticate legacy adapters.
use super::*;
use crate::pure_semantics::hash_encoding::{DynamicInputEncoding, contains_opaque};

#[test]
fn dynamic_transcript_and_witness_helper_outputs_match_exact_frozen_source() {
    for ty in [
        Type::Field,
        Type::OpaqueBytes,
        Type::OpaqueString,
        tuple(Type::OpaqueString, 2),
        Type::Vector {
            element: Box::new(Type::OpaqueBytes),
            length: 0,
        },
        Type::Struct {
            name: "Invalid$private".into(),
            fields: vec![StructField {
                name: "field".into(),
                ty: Type::OpaqueString,
            }],
        },
        map(Type::OpaqueString),
        tuple(map(Type::OpaqueBytes), 1),
        Type::Unsigned { max: "bad".into() },
    ] {
        let value: syn::Expr = syn::parse_quote!(probe_once()?);
        let current = crate::hash_input::syntax(value.clone(), &ty);
        let previous = old_hash_input::syntax(value.clone(), &ty);
        assert_eq!(
            current.to_token_stream().to_string(),
            previous.to_token_stream().to_string()
        );
        let output =
            crate::hash_input::render_input(value.clone(), DynamicInputEncoding::for_type(&ty));
        assert_eq!(
            output.to_token_stream().to_string(),
            previous.to_token_stream().to_string()
        );
        assert_eq!(
            contains_opaque(&ty),
            previous
                .to_token_stream()
                .to_string()
                .contains("exact_hash_input")
        );
        let current = crate::hash_input::private_output(value.clone(), &ty);
        let previous = old_hash_input::private_output(value, &ty);
        assert_eq!(
            current.to_token_stream().to_string(),
            previous.to_token_stream().to_string()
        );
        assert_eq!(
            crate::hash_input::witness_method(&ty),
            old_hash_input::witness_method(&ty)
        );
    }
}
