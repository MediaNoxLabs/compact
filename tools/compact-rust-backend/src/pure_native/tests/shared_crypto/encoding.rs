// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tuple capability and selective exact encoding are separate preserved rules.
use super::*;

#[test]
fn six_hash_operations_preserve_recursive_tuple_boundaries_and_trait_labels() {
    for kind in [
        Kind::TransientHash,
        Kind::TransientCommit,
        Kind::PersistentHash,
        Kind::PersistentCommit,
        Kind::Keccak256,
        Kind::HashToCurve,
    ] {
        for opaque in [false, true] {
            for arity in [11, 12] {
                for payload in [
                    tuple(
                        if opaque {
                            Type::OpaqueString
                        } else {
                            Type::Field
                        },
                        arity,
                    ),
                    Type::Vector {
                        element: Box::new(tuple(
                            if opaque {
                                Type::OpaqueBytes
                            } else {
                                Type::Field
                            },
                            arity,
                        )),
                        length: 2,
                    },
                ] {
                    let opening = if matches!(kind, Kind::PersistentCommit) {
                        bytes(32)
                    } else {
                        Type::Field
                    };
                    let parameters = HashMap::from([
                        ("payload", (&payload, syn::parse_quote!(payload))),
                        ("opening", (&opening, syn::parse_quote!(opening))),
                    ]);
                    let result = compare(
                        &operation(kind, reference("payload"), reference("opening")),
                        &parameters,
                    );
                    if arity == 11 {
                        assert!(result.is_ok(), "{kind:?}");
                    } else {
                        let name = match kind {
                            Kind::TransientHash => "transientHash",
                            Kind::TransientCommit => "transientCommit",
                            Kind::PersistentHash => "persistentHash",
                            Kind::PersistentCommit => "persistentCommit",
                            Kind::Keccak256 => "keccak256",
                            Kind::HashToCurve => "hashToCurve",
                            _ => unreachable!(),
                        };
                        let required_trait = if matches!(kind, Kind::Keccak256) {
                            "Into<Value>"
                        } else if opaque
                            && matches!(
                                kind,
                                Kind::TransientHash | Kind::TransientCommit | Kind::HashToCurve
                            )
                        {
                            "HashInputValue"
                        } else {
                            "Into<Value> + Aligned"
                        };
                        assert_eq!(
                            result,
                            Err(RenderError::UnsupportedTupleOperation {
                                operation: name,
                                arity: 12,
                                limit: 11,
                                required_trait
                            })
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn exact_wrapper_tracks_zero_vector_elements_and_named_fields_but_not_ledger_maps() {
    for (payload, wrapped) in [
        (Type::Field, false),
        (bytes(32), false),
        (Type::OpaqueString, true),
        (
            Type::Vector {
                element: Box::new(Type::OpaqueBytes),
                length: 0,
            },
            true,
        ),
        (
            Type::Struct {
                name: "Bad$private".into(),
                fields: vec![StructField {
                    name: "entry".into(),
                    ty: Type::OpaqueString,
                }],
            },
            true,
        ),
        (map(Type::OpaqueString), false),
        (tuple(map(Type::OpaqueBytes), 1), false),
        (Type::Unsigned { max: "bad".into() }, false),
    ] {
        let field = Type::Field;
        let opening = bytes(32);
        for kind in [
            Kind::TransientHash,
            Kind::TransientCommit,
            Kind::PersistentHash,
            Kind::PersistentCommit,
            Kind::Keccak256,
            Kind::HashToCurve,
        ] {
            let parameters = HashMap::from([
                ("payload", (&payload, syn::parse_quote!(payload))),
                (
                    "opening",
                    (
                        if matches!(kind, Kind::PersistentCommit) {
                            &opening
                        } else {
                            &field
                        },
                        syn::parse_quote!(opening),
                    ),
                ),
            ]);
            let source = operation(kind, reference("payload"), reference("opening"));
            assert!(compare(&source, &parameters).is_ok());
            let (syntax, _) =
                crate::pure_crypto_expression(&source, &parameters, &HashMap::new()).unwrap();
            assert_eq!(
                syntax
                    .to_token_stream()
                    .to_string()
                    .contains("exact_hash_input"),
                wrapped
                    && matches!(
                        kind,
                        Kind::TransientHash | Kind::TransientCommit | Kind::HashToCurve
                    ),
                "{kind:?} {payload:?}"
            );
        }
    }
    // The broader capability scan sees a tuple behind LedgerMap; the encoding
    // predicate never descends there. These facts must not be conflated.
    let payload = map(tuple(Type::OpaqueString, 12));
    let parameters = HashMap::from([("payload", (&payload, syn::parse_quote!(payload)))]);
    assert_eq!(
        compare(
            &operation(Kind::TransientHash, reference("payload"), Expr::Unit),
            &parameters
        ),
        Err(RenderError::UnsupportedTupleOperation {
            operation: "transientHash",
            arity: 12,
            limit: 11,
            required_trait: "HashInputValue"
        })
    );
    assert_eq!(
        crate::pure_semantics::hash_encoding::DynamicInputEncoding::for_type(&payload),
        crate::pure_semantics::hash_encoding::DynamicInputEncoding::Direct
    );
}
