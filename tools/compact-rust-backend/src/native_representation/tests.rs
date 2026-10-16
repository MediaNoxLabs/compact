// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::ir::{Expr, StructField};

mod emission;

const OPERATIONS: [Operation; 8] = [
    Operation::Equal,
    Operation::NotEqual,
    Operation::TransientHash,
    Operation::TransientCommit,
    Operation::PersistentHash,
    Operation::PersistentCommit,
    Operation::Keccak256,
    Operation::HashToCurve,
];

fn tuple(arity: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::Field; arity],
    }
}

fn tuple_value(arity: usize) -> Expr {
    Expr::Tuple {
        elements: vec![Expr::FieldLiteral { value: "1".into() }; arity],
    }
}

fn expression(operation: Operation, value: Expr) -> Expr {
    let value = Box::new(value);
    match operation {
        Operation::Equal => Expr::Equal {
            left: value.clone(),
            right: value,
        },
        Operation::NotEqual => Expr::NotEqual {
            left: value.clone(),
            right: value,
        },
        Operation::TransientHash => Expr::TransientHash { value },
        Operation::TransientCommit => Expr::TransientCommit {
            value,
            opening: Box::new(Expr::FieldLiteral { value: "7".into() }),
        },
        Operation::PersistentHash => Expr::PersistentHash { value },
        Operation::PersistentCommit => Expr::PersistentCommit {
            value,
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
        Operation::Keccak256 => Expr::Keccak256 { value },
        Operation::HashToCurve => Expr::HashToCurve { value },
    }
}

fn assert_refusal(error: RenderError, operation: Operation, arity: usize, trait_name: &str) {
    assert_eq!(
        error,
        RenderError::UnsupportedTupleOperation {
            operation: operation.name(),
            arity,
            limit: operation.tuple_limit(),
            required_trait: operation.required_trait(trait_name == "HashInputValue"),
        }
    );
    assert!(error.to_string().contains(trait_name));
}

#[test]
fn all_operation_boundaries_are_specific_to_the_required_trait() {
    for operation in OPERATIONS {
        let limit = operation.tuple_limit();
        check_tuple_traits(operation, &tuple(limit)).unwrap();
        let error = check_tuple_traits(operation, &tuple(limit + 1)).unwrap_err();
        assert_refusal(error, operation, limit + 1, operation.required_trait(false));
    }
}

#[test]
fn nested_vector_and_named_product_tuples_do_not_escape_operation_checks() {
    for operation in OPERATIONS {
        let arity = operation.tuple_limit() + 1;
        for carrier in [
            Type::Vector {
                element: Box::new(tuple(arity)),
                length: 0,
            },
            Type::Struct {
                name: "Envelope".into(),
                fields: vec![StructField {
                    name: "payload".into(),
                    ty: tuple(arity),
                }],
            },
            Type::Tuple {
                elements: vec![Type::Boolean, tuple(arity)],
            },
        ] {
            assert_refusal(
                check_tuple_traits(operation, &carrier).unwrap_err(),
                operation,
                arity,
                operation.required_trait(false),
            );
        }
    }
}

#[test]
fn exact_hash_input_diagnostic_follows_the_whole_opaque_encoding_carrier() {
    // The failing nested tuple itself has no opaque leaves. Its sibling selects
    // ExactHashInput for the whole preimage, so HashInputValue is the obligation.
    let carrier = Type::Tuple {
        elements: vec![tuple(12), Type::OpaqueString],
    };
    for operation in [
        Operation::TransientHash,
        Operation::TransientCommit,
        Operation::HashToCurve,
    ] {
        assert_refusal(
            check_tuple_traits(operation, &carrier).unwrap_err(),
            operation,
            12,
            "HashInputValue",
        );
    }
    for operation in [
        Operation::PersistentHash,
        Operation::PersistentCommit,
        Operation::Keccak256,
    ] {
        assert_refusal(
            check_tuple_traits(operation, &carrier).unwrap_err(),
            operation,
            12,
            operation.required_trait(false),
        );
    }
}

#[test]
fn opaque_source_policy_and_ordinary_leaf_support_are_not_changed() {
    for operation in OPERATIONS {
        for carrier in [
            Type::OpaqueString,
            Type::OpaqueBytes,
            Type::JubjubPoint,
            Type::Unit,
        ] {
            check_tuple_traits(operation, &carrier).unwrap();
        }
    }
}

#[test]
fn borrowed_resource_refusal_precedes_tuple_trait_diagnostics() {
    let oversized = tuple(32_769);
    assert!(matches!(
        check_tuple_traits(Operation::Equal, &oversized),
        Err(RenderError::ResourceLimit { .. })
    ));
}
