// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Child failures and opening-before-payload rules are paired, not isolated.
use super::*;

#[test]
fn binary_source_children_finish_before_kind_opening_and_tuple_refusals() {
    for kind in [
        Kind::TransientCommit,
        Kind::PersistentCommit,
        Kind::Construct,
        Kind::Add,
        Kind::Multiply,
    ] {
        for left in [Type::Boolean, tuple(Type::OpaqueString, 12)] {
            let parameters = HashMap::from([("left", (&left, syn::parse_quote!(left_value)))]);
            let source = operation(kind, reference("left"), reference("missing"));
            assert_eq!(
                compare(&source, &parameters),
                Err(RenderError::UnknownParameter("missing".into()))
            );
            let (_, visits) =
                visitation::observe(|| observe(&source, &parameters, &HashMap::new()));
            assert_eq!(visits.len(), 3);
            assert!(visits.values().all(|count| *count == 1));
            assert_eq!(
                compare(
                    &operation(
                        kind,
                        reference("first_missing"),
                        reference("second_missing")
                    ),
                    &parameters
                ),
                Err(RenderError::UnknownParameter("first_missing".into()))
            );
        }
    }
    for kind in source::ALL {
        assert_eq!(
            compare(
                &operation(kind, reference("missing"), reference("other")),
                &HashMap::new()
            ),
            Err(RenderError::UnknownParameter("missing".into()))
        );
    }
}

#[test]
fn wrong_opening_beats_overwide_payload_and_valid_opening_reveals_original_trait_refusal() {
    for (kind, opening_type, name, required_trait) in [
        (
            Kind::TransientCommit,
            Type::Field,
            "transientCommit",
            "HashInputValue",
        ),
        (
            Kind::PersistentCommit,
            bytes(32),
            "persistentCommit",
            "Into<Value> + Aligned",
        ),
    ] {
        let payload = tuple(Type::OpaqueString, 12);
        let boolean = Type::Boolean;
        let source = operation(kind, reference("payload"), reference("opening"));
        let parameters = HashMap::from([
            ("payload", (&payload, syn::parse_quote!(payload))),
            ("opening", (&boolean, syn::parse_quote!(opening))),
        ]);
        assert_eq!(
            compare(&source, &parameters),
            Err(RenderError::TypeMismatch {
                expected: opening_type.clone(),
                actual: Type::Boolean
            })
        );
        let parameters = HashMap::from([
            ("payload", (&payload, syn::parse_quote!(payload))),
            ("opening", (&opening_type, syn::parse_quote!(opening))),
        ]);
        assert_eq!(
            compare(&source, &parameters),
            Err(RenderError::UnsupportedTupleOperation {
                operation: name,
                arity: 12,
                limit: 11,
                required_trait
            })
        );
    }
}
