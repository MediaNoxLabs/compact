// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Distinct nested source occurrences lower once with exact complete old views.
use super::*;

#[test]
fn nested_crypto_fallible_operands_and_coordinates_keep_source_occurrences_once() {
    let scalar = || Expr::FieldLiteral { value: "7".into() };
    let left = operation(
        Kind::Multiply,
        operation(Kind::HashToCurve, scalar(), Expr::Unit),
        operation(Kind::Reduce, scalar(), Expr::Unit),
    );
    let right = operation(Kind::Generator, scalar(), Expr::Unit);
    let source = operation(Kind::Y, operation(Kind::Add, left, right), Expr::Unit);
    assert_eq!(compare(&source, &HashMap::new()), Ok(Type::Field));
    let (result, visits) =
        visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
    result.unwrap();
    assert_eq!(visits.len(), 9);
    assert!(visits.values().all(|count| *count == 1));
    let source = operation(
        Kind::TransientCommit,
        operation(Kind::Upgrade, scalar(), Expr::Unit),
        operation(
            Kind::Degrade,
            Expr::BytesLiteral { bytes: vec![0; 32] },
            Expr::Unit,
        ),
    );
    assert_eq!(compare(&source, &HashMap::new()), Ok(Type::Field));
    let (result, visits) =
        visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
    result.unwrap();
    assert_eq!(visits.len(), 5);
    assert!(visits.values().all(|count| *count == 1));
}
