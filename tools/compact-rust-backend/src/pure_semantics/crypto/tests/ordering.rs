// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Paired failures distinguish source visitation from operand admission order.
use super::*;

#[test]
fn second_child_error_beats_first_kind_or_tuple_error_for_all_binary_operations() {
    let left = parameter("left");
    let right = parameter("missing");
    for ty in [
        Type::Boolean,
        Type::Tuple {
            elements: vec![Type::Field; 12],
        },
    ] {
        for source in [
            CryptoSource::TransientCommit(&left, &right),
            CryptoSource::PersistentCommit(&left, &right),
            CryptoSource::ConstructJubjubPoint(&left, &right),
            CryptoSource::EcAdd(&left, &right),
            CryptoSource::EcMul(&left, &right),
        ] {
            let mut children = LeafChildren {
                bindings: HashMap::from([("left", (&ty, ()))]),
                visits: vec![],
            };
            let mut sink = Consumer::default();
            assert_eq!(
                fold_crypto(source, &mut children, &mut sink).err(),
                Some(RenderError::UnknownParameter("missing".into()))
            );
            assert_eq!(
                children.visits,
                [
                    std::ptr::from_ref(&left) as usize,
                    std::ptr::from_ref(&right) as usize
                ]
            );
            assert!(sink.outputs.is_empty());
        }
    }
    let mut children = LeafChildren {
        bindings: HashMap::new(),
        visits: vec![],
    };
    assert_eq!(
        fold_crypto(
            CryptoSource::EcAdd(&left, &right),
            &mut children,
            &mut Consumer::default()
        )
        .err(),
        Some(RenderError::UnknownParameter("left".into()))
    );
    assert_eq!(children.visits, [std::ptr::from_ref(&left) as usize]);
}

#[test]
fn wrong_opening_kind_precedes_tuple_guard_and_valid_opening_reveals_payload_refusal() {
    let value = parameter("payload");
    let opening = parameter("opening");
    let tuple = Type::Tuple {
        elements: vec![Type::OpaqueString; 12],
    };
    let boolean = Type::Boolean;
    for source in [
        CryptoSource::TransientCommit(&value, &opening),
        CryptoSource::PersistentCommit(&value, &opening),
    ] {
        let expected = if matches!(source, CryptoSource::TransientCommit(..)) {
            Type::Field
        } else {
            Type::Bytes { length: 32 }
        };
        let mut children = LeafChildren {
            bindings: HashMap::from([("payload", (&tuple, ())), ("opening", (&boolean, ()))]),
            visits: vec![],
        };
        assert_eq!(
            fold_crypto(source, &mut children, &mut Consumer::default()).err(),
            Some(RenderError::TypeMismatch {
                expected,
                actual: Type::Boolean
            })
        );
    }
    let field = Type::Field;
    let mut children = LeafChildren {
        bindings: HashMap::from([("payload", (&tuple, ())), ("opening", (&field, ()))]),
        visits: vec![],
    };
    assert_eq!(
        fold_crypto(
            CryptoSource::TransientCommit(&value, &opening),
            &mut children,
            &mut Consumer::default()
        )
        .err(),
        Some(RenderError::UnsupportedTupleOperation {
            operation: "transientCommit",
            arity: 12,
            limit: 11,
            required_trait: "HashInputValue"
        })
    );
}
