// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Every operation's result/fallibility and old/native/neutral kind partitions.
use super::*;

#[test]
fn sixteen_native_and_no_syntax_successes_preserve_result_and_fallibility() {
    for (kind, left, right, result, fallible) in [
        (
            Kind::TransientHash,
            Type::Field,
            Type::Unit,
            Type::Field,
            false,
        ),
        (
            Kind::TransientCommit,
            Type::OpaqueString,
            Type::Field,
            Type::Field,
            false,
        ),
        (
            Kind::PersistentHash,
            Type::Field,
            Type::Unit,
            bytes(32),
            false,
        ),
        (
            Kind::PersistentCommit,
            Type::Field,
            bytes(32),
            bytes(32),
            false,
        ),
        (Kind::Keccak256, Type::Field, Type::Unit, bytes(32), false),
        (Kind::Degrade, bytes(32), Type::Unit, Type::Field, false),
        (Kind::Upgrade, Type::Field, Type::Unit, bytes(32), false),
        (
            Kind::HashToCurve,
            Type::OpaqueBytes,
            Type::Unit,
            Type::JubjubPoint,
            false,
        ),
        (Kind::X, Type::JubjubPoint, Type::Unit, Type::Field, false),
        (Kind::Y, Type::JubjubPoint, Type::Unit, Type::Field, false),
        (
            Kind::Construct,
            Type::Field,
            Type::Field,
            Type::JubjubPoint,
            true,
        ),
        (
            Kind::Add,
            Type::JubjubPoint,
            Type::JubjubPoint,
            Type::JubjubPoint,
            false,
        ),
        (
            Kind::Negate,
            Type::JubjubPoint,
            Type::Unit,
            Type::JubjubPoint,
            false,
        ),
        (
            Kind::Multiply,
            Type::JubjubPoint,
            Type::Field,
            Type::JubjubPoint,
            true,
        ),
        (
            Kind::Generator,
            Type::Field,
            Type::Unit,
            Type::JubjubPoint,
            true,
        ),
        (Kind::Reduce, Type::Field, Type::Unit, Type::Field, false),
    ] {
        let parameters = HashMap::from([
            ("source$left", (&left, syn::parse_quote!(left_argument))),
            ("source$right", (&right, syn::parse_quote!(right_argument))),
        ]);
        let source = operation(kind, reference("source$left"), reference("source$right"));
        assert_eq!(compare(&source, &parameters), Ok(result));
        let (output, _) =
            crate::pure_crypto_expression(&source, &parameters, &HashMap::new()).unwrap();
        assert_eq!(matches!(output, syn::Expr::Try(_)), fallible, "{kind:?}");
        let (result, visits) =
            visitation::observe(|| observe(&source, &parameters, &HashMap::new()));
        result.unwrap();
        assert_eq!(visits.len(), if kind.binary() { 3 } else { 2 });
        assert!(visits.values().all(|count| *count == 1));
    }
}

#[test]
fn private_bounded_kind_pairs_match_original_semantics_without_new_carrier_checks() {
    let types = [
        Type::Boolean,
        Type::Field,
        Type::JubjubPoint,
        bytes(31),
        bytes(32),
        Type::Unsigned {
            max: "malformed".into(),
        },
        Type::Struct {
            name: "Bad$private".into(),
            fields: vec![],
        },
    ];
    for kind in source::ALL {
        for left in &types {
            for right in &types {
                let parameters = HashMap::from([
                    ("left", (left, syn::parse_quote!(l))),
                    ("right", (right, syn::parse_quote!(r))),
                ]);
                let _ = compare(
                    &operation(kind, reference("left"), reference("right")),
                    &parameters,
                );
            }
        }
    }
}
