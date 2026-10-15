// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Complete old helper independently pins output-only ownership factoring.
use super::*;
use crate::pure_semantics::ownership::{Ownership, copy_type};
#[test]
fn checked_retention_and_legacy_entrypoint_match_exact_old_helper() {
    for ty in [
        Type::Unit,
        Type::Field,
        Type::JubjubPoint,
        Type::Bytes { length: 32 },
        uint("bad"),
        Type::OpaqueString,
        Type::OpaqueBytes,
        vector_type(Type::Field, 1),
        Type::Tuple {
            elements: vec![Type::Field; 13],
        },
        Type::Tuple {
            elements: vec![Type::OpaqueString],
        },
        structure("Bad$private", vec![]),
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::OpaqueString),
        },
    ] {
        let value: syn::Expr = syn::parse_quote!(probe_once()?.entry);
        let old = old_retained::retained_value(value.clone(), &ty)
            .to_token_stream()
            .to_string();
        assert_eq!(
            crate::value_lowering::retained_value(value.clone(), &ty)
                .to_token_stream()
                .to_string(),
            old
        );
        let ownership = if copy_type(&ty) {
            Ownership::Copy
        } else {
            Ownership::Owned
        };
        assert_eq!(
            crate::value_lowering::retained_value_with_ownership(value, ownership)
                .to_token_stream()
                .to_string(),
            old
        );
    }
}
