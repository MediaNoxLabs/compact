// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Encoding classification is intentionally narrower than tuple traversal.
use super::*;
use crate::pure_semantics::hash_encoding::DynamicInputEncoding;

#[test]
fn opaque_vectors_and_structs_are_exact_but_ledger_map_payloads_are_not_descended() {
    let value = parameter("value");
    for (ty, expected) in [
        (Type::OpaqueString, DynamicInputEncoding::Exact),
        (
            Type::Vector {
                element: Box::new(Type::OpaqueBytes),
                length: 0,
            },
            DynamicInputEncoding::Exact,
        ),
        (
            Type::Struct {
                name: "Invalid$private".into(),
                fields: vec![crate::ir::StructField {
                    name: "field".into(),
                    ty: Type::OpaqueString,
                }],
            },
            DynamicInputEncoding::Exact,
        ),
        (
            Type::LedgerMap {
                key: Box::new(Type::Field),
                value: Box::new(Type::OpaqueString),
            },
            DynamicInputEncoding::Direct,
        ),
        (
            Type::Tuple {
                elements: vec![Type::LedgerMap {
                    key: Box::new(Type::OpaqueString),
                    value: Box::new(Type::OpaqueBytes),
                }],
            },
            DynamicInputEncoding::Direct,
        ),
        (
            Type::Unsigned {
                max: "malformed".into(),
            },
            DynamicInputEncoding::Direct,
        ),
    ] {
        let mut children = LeafChildren {
            bindings: HashMap::from([("value", (&ty, ()))]),
            visits: vec![],
        };
        let mut sink = Consumer::default();
        assert_eq!(
            fold_crypto(
                CryptoSource::TransientHash(&value),
                &mut children,
                &mut sink
            )
            .unwrap()
            .ty,
            Type::Field
        );
        assert!(
            matches!(sink.outputs.as_slice(), [CheckedCrypto::TransientHash { encoding, .. }] if *encoding == expected)
        );
        assert_eq!(children.visits.len(), 1);
    }
}
